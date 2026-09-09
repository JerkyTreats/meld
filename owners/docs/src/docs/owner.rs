//! Local package server for Docs semantics over native Meld owner contracts.

use std::collections::BTreeSet;
use std::sync::Arc;

use meld_execution::capability::CapabilityInvoker;

use super::capability::*;
use super::claim_validation::{
    DocsClaimPolicyRegistryStore, DocsClaimPolicyRevision, DocsClaimPolicyRevisionRef,
};
use super::contribution::{configuration, selected_claim_policy, DocsCapabilityContributor};
use crate::capability::{OwnerBindingView, ProductCapabilityContributor};
use crate::runtime::lifecycle::{
    NativeObservationOwner, NativeObservationOwnerFactory, ParticipantLifecycleContextV1,
};
use crate::runtime::owners::events::CallbackEventAuthorityClient;
use crate::runtime::owners::execution::CallbackExecutionPorts;
use crate::runtime::owners::provider::CallbackProviderClient;
use crate::runtime::owners::server::{dispatch_observation, dispatch_theory, PackageOwner};
use crate::runtime::owners::*;

#[derive(Default)]
pub struct DocsPackageOwner {
    policies: Option<Arc<DocsClaimPolicyRegistryStore>>,
    runtime: Option<PreparedDocs>,
    observation: Option<Box<dyn NativeObservationOwner>>,
    active: Option<ParticipantLifecycleContextV1>,
    released: Option<(ParticipantLifecycleContextV1, serde_json::Value)>,
}

struct PreparedDocs {
    state_root: std::path::PathBuf,
    ledger_id: meld_events::LedgerIdentity,
    native: OwnerObservationPreparationV1,
    config: DocsCapabilityConfig,
    revision: DocsClaimPolicyRevision,
    route: meld_world_model::world_state::graph::admission::GraphOwnerEventRoute,
}

fn implementation(id: &str) -> String {
    format!("docs.serialized-local.v1::{id}")
}

impl DocsPackageOwner {
    pub fn description() -> OwnerDescriptionV1 {
        let contributor = DocsCapabilityContributor;
        OwnerDescriptionV1 {
            protocol_version: OWNER_PROTOCOL_VERSION,
            owner_id: "docs".into(),
            routes: vec![super::theory::route_contract()],
            capabilities: contributor.published_contracts(),
            implementations: contributor
                .implementation_offers()
                .into_iter()
                .map(|offer| OwnerImplementationOfferV1 {
                    implementation_ref: implementation(
                        &offer.contract_ref.selector.capability_type_id,
                    ),
                    contract_ref: offer.contract_ref,
                    required_binding_ids: offer.required_binding_ids,
                })
                .collect(),
            observation_participant: Some(crate::theory::ActivationParticipantSpec {
                participant_id: super::runtime::RUNTIME_ID.into(),
                owner_domain: "docs".into(),
                kind: crate::theory::ParticipantKind::BoundedActor,
                required: true,
                depends_on: BTreeSet::new(),
                readiness_contract_ref: "docs.readiness.v1".into(),
                wake_contract_ref: "docs.wake.v1".into(),
                safe_point_contract_ref: "docs.safe-point.v1".into(),
                stop_contract_ref: "docs.stop.v1".into(),
            }),
        }
    }

    fn prepare_bindings(
        &self,
        subject: meld_events::DomainObjectRef,
        bindings: std::collections::BTreeMap<String, String>,
        revisions: Vec<crate::theory::InstalledTheoryComponentRef>,
    ) -> OwnerResult {
        let selected: Vec<_> = revisions
            .iter()
            .filter(|component| component.route == super::theory::route_contract().route)
            .collect();
        let [selected] = selected.as_slice() else {
            return Err(failure("Docs requires one exact claim-policy revision"));
        };
        let reference = &selected.owner_revision;
        let policy = self
            .policies
            .as_ref()
            .ok_or_else(|| failure("Docs revision store is not open"))?
            .resolve(&DocsClaimPolicyRevisionRef {
                policy_id: reference.id.clone(),
                content_identity: reference.content_hash.clone(),
            })
            .map_err(failure)?
            .ok_or_else(|| failure("selected Docs policy is absent"))?;
        let bindings =
            super::contribution::bind_claim_policy(OwnerBindingView::new(bindings), &policy)
                .map_err(failure)?;
        let provider_observation = BTreeSet::from([
            "docs-source-claims".into(),
            "docs-claim-validation".into(),
            "docs-claim-correspondence".into(),
        ]);
        let capability_grants = Self::description()
            .implementations
            .into_iter()
            .map(|offer| {
                let id = offer.contract_ref.selector.capability_type_id.as_str();
                let publications = if id == PUBLISH_PATCH_SET {
                    BTreeSet::from([(
                        super::publication_return::EVENT_TYPE.into(),
                        subject.object_id.clone(),
                    )])
                } else {
                    BTreeSet::new()
                };
                let provider_frame_types = match id {
                    DRAFT_PATCH_SET => {
                        BTreeSet::from(["docs-readme".into(), "docs-readme-revision".into()])
                    }
                    VALIDATE_PATCH_SET => provider_observation.clone(),
                    _ => BTreeSet::new(),
                };
                OwnerCapabilityGrantV1 {
                    selection: OwnerInvocationSelectionV1 {
                        contract_ref: offer.contract_ref,
                        implementation_ref: offer.implementation_ref,
                    },
                    publications,
                    provider_frame_types,
                    continuation_publications: Default::default(),
                }
            })
            .collect();
        encode_owner_result(OwnerPreparedBindingsV1 {
            bindings: bindings.values().clone(),
            observation_publications: BTreeSet::from([(
                super::publication::OBSERVATION_EVENT.into(),
                subject.object_id,
            )]),
            observation_provider_frame_types: provider_observation,
            retained_observation_publications: Default::default(),
            capability_grants,
        })
    }

    fn prepare_runtime(&mut self, preparation: OwnerRuntimePreparationV1) -> OwnerResult {
        let native = preparation
            .observation
            .as_ref()
            .ok_or_else(|| failure("Docs observation preparation is absent"))?;
        let bindings = OwnerBindingView::new(preparation.bindings.clone());
        let config = configuration(&bindings).map_err(failure)?;
        let revision: DocsClaimPolicyRevision = serde_json::from_str(
            bindings
                .get("docs.claim-policy")
                .ok_or_else(|| failure("Docs policy binding is absent"))?,
        )
        .map_err(failure)?;
        selected_claim_policy(&bindings).map_err(failure)?;
        if !preparation.installed_revisions.iter().any(|component| {
            component.route == super::theory::route_contract().route
                && component.owner_revision.id == revision.policy.policy_id
                && component.owner_revision.content_hash == revision.content_identity
        }) {
            return Err(failure(
                "Docs runtime policy differs from its exact installed revision",
            ));
        }
        let route = native
            .event_routes
            .iter()
            .find(|route| {
                route.owner_id == "docs"
                    && route.event_type == super::publication::OBSERVATION_EVENT
            })
            .ok_or_else(|| failure("Docs Graph publication route is absent"))?
            .clone();
        // Preparation verifies immutable selection before the supervisor grants a lease.
        // A predecessor may still hold the assignment's mutable observation store.
        self.runtime = Some(PreparedDocs {
            native: native.clone(),
            config,
            revision,
            route,
            state_root: preparation.state_root,
            ledger_id: preparation.ledger_id,
        });
        encode_owner_result(())
    }

    fn open_observation(
        &mut self,
        callbacks: Arc<dyn OwnerCallbackPort>,
    ) -> Result<(), OwnerDiagnosticV1> {
        let preparation = self
            .runtime
            .as_ref()
            .ok_or_else(|| failure("Docs runtime is not prepared"))?;
        let native = &preparation.native;
        let config = preparation.config.clone();
        let revision = preparation.revision.clone();
        let route = preparation.route.clone();
        let events = meld_events::EventAppendCapability::from_remote(
            preparation.ledger_id,
            Arc::new(CallbackEventAuthorityClient::new(callbacks.clone())),
        );
        let db = sled::open(preparation.state_root.join("observations.sled")).map_err(failure)?;
        let binding = super::runtime::DocsObservationBinding {
            root: config.target_root.clone(),
            subject: native.subject.clone(),
            scope: native.scope.clone(),
            session_id: native.session_id.clone(),
            store: Arc::new(
                super::observation_store::DocsObservationStore::new(db).map_err(failure)?,
            ),
            events,
            claim_policy: Some(revision),
            claim_config: Some(config),
            claim_judge: Default::default(),
            route,
        };
        if !binding.bind_provider(Arc::new(CallbackProviderClient::new(callbacks))) {
            return Err(failure("Docs observation provider could not be bound"));
        }
        self.observation = Some(binding.build());
        self.active = None;
        self.released = None;
        Ok(())
    }

    fn invocation(
        &self,
        selection: OwnerInvocationSelectionV1,
        init: crate::capability::CapabilityRuntimeInit,
        payload: crate::capability::CapabilityInvocationPayload,
        context: Option<crate::execution::ExecutionEventContext>,
        callbacks: Arc<dyn OwnerCallbackPort>,
        recovery: bool,
    ) -> OwnerResult {
        if !Self::description().implementations.iter().any(|offer| {
            offer.contract_ref == selection.contract_ref
                && offer.implementation_ref == selection.implementation_ref
        }) || init.capability_type_id != selection.contract_ref.selector.capability_type_id
            || init.capability_version != selection.contract_ref.selector.capability_version
        {
            return Err(failure(
                "Docs invocation does not name a selected exact implementation",
            ));
        }
        let prepared = self
            .runtime
            .as_ref()
            .ok_or_else(|| failure("Docs runtime is not prepared"))?;
        let config = prepared.config.clone();
        let policy = prepared.revision.policy.clone();
        let events = meld_events::EventAppendCapability::from_remote(
            prepared.ledger_id,
            Arc::new(CallbackEventAuthorityClient::new(callbacks.clone())),
        );
        let ports = CallbackExecutionPorts::new(events.clone(), callbacks);
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(failure)?;
        if recovery {
            let result = if init.capability_type_id == PUBLISH_PATCH_SET {
                executor.block_on(PublishPatchSetCapability::new(config, policy).recover(
                    Some(&events.replay_capability()),
                    &init,
                    &payload,
                    context.as_ref(),
                ))
            } else {
                Ok(None)
            };
            return encode_capability_result(result);
        }
        let result = executor.block_on(async {
            match init.capability_type_id.as_str() {
                INSPECT_SCOPE => {
                    InspectScopeCapability::new(config, policy)
                        .invoke_owned(&ports, &init, &payload, context.as_ref())
                        .await
                }
                DRAFT_PATCH_SET => {
                    DraftPatchSetCapability::new(config, policy)
                        .invoke_owned(&ports, &init, &payload, context.as_ref())
                        .await
                }
                VALIDATE_PATCH_SET => {
                    ValidatePatchSetCapability::new(config, policy)
                        .invoke_owned(&ports, &init, &payload, context.as_ref())
                        .await
                }
                PUBLISH_PATCH_SET => {
                    PublishPatchSetCapability::new(config, policy)
                        .invoke_owned(&ports, &init, &payload, context.as_ref())
                        .await
                }
                _ => unreachable!("published Docs implementation was checked above"),
            }
        });
        encode_capability_result(result)
    }
}

impl PackageOwner for DocsPackageOwner {
    fn handle(
        &mut self,
        command: OwnerCommandV1,
        callbacks: Arc<dyn OwnerCallbackPort>,
    ) -> OwnerResult {
        use OwnerCommandV1::*;
        match command {
            Describe => encode_owner_result(Self::description()),
            OpenRevisionStore { state_root } => {
                let db = sled::open(state_root.join("revisions.sled")).map_err(failure)?;
                self.policies = Some(Arc::new(
                    DocsClaimPolicyRegistryStore::new(db).map_err(failure)?,
                ));
                encode_owner_result(())
            }
            command @ (ValidateTheory { .. }
            | InstallTheory { .. }
            | VerifyTheory { .. }
            | ValidateLinks { .. }) => {
                let policies = self
                    .policies
                    .as_ref()
                    .ok_or_else(|| failure("Docs revision store is not open"))?;
                dispatch_theory(
                    super::theory::route_handler(policies.clone()).as_ref(),
                    command,
                )
            }
            PrepareBindings {
                subject,
                bindings,
                installed_revisions,
            } => self.prepare_bindings(subject, bindings, installed_revisions),
            PrepareRuntime { preparation } => self.prepare_runtime(preparation),
            ResolveCurationSource {
                template: _,
                binding,
            } => {
                encode_owner_result(super::publication::curation_source(&binding).map_err(failure)?)
            }
            Invoke {
                selection,
                runtime_init,
                payload,
                event_context,
            } => self.invocation(
                selection,
                runtime_init,
                payload,
                event_context,
                callbacks,
                false,
            ),
            RecoverInvocation {
                selection,
                runtime_init,
                payload,
                event_context,
            } => self.invocation(
                selection,
                runtime_init,
                payload,
                event_context,
                callbacks,
                true,
            ),
            CloseRevisionStore => {
                self.policies = None;
                encode_owner_result(())
            }
            // Policy installation and observation transitions flush their own stores.
            Flush => encode_owner_result(()),
            command => {
                if let Release { context } = &command {
                    if let Some((released_context, receipt)) = &self.released {
                        if context == released_context {
                            return Ok(receipt.clone());
                        }
                    }
                }
                if matches!(command, Readiness { .. }) && self.observation.is_none() {
                    self.open_observation(callbacks)?;
                }
                let release_context = match &command {
                    Release { context } => Some(context.clone()),
                    _ => None,
                };
                let observation = self
                    .observation
                    .as_mut()
                    .ok_or_else(|| failure("Docs observation is not prepared"))?;
                let result = dispatch_observation(observation.as_mut(), &mut self.active, command)?;
                if let Some(context) = release_context {
                    // Keep the exact receipt while closing the released physical store.
                    self.released = Some((context, result.clone()));
                    self.observation = None;
                }
                Ok(result)
            }
        }
    }
}

fn failure(error: impl ToString) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("docs_owner_invalid", error)
}
