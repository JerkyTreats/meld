//! Security semantics over the same native owner protocol as other packages.

use super::capability::*;
use super::contribution::{bind_selected_policy, DependencySecurityCapabilityContributor};
use super::theory::DependencySecurityPolicyRegistry;
use crate::capability::{OwnerBindingView, ProductCapabilityContributor};
use crate::runtime::lifecycle::{
    NativeObservationOwner, NativeObservationOwnerFactory, ParticipantLifecycleContextV1,
};
use crate::runtime::owners::events::CallbackEventAuthorityClient;
use crate::runtime::owners::execution::CallbackExecutionPorts;
use crate::runtime::owners::server::{dispatch_observation, dispatch_theory, PackageOwner};
use crate::runtime::owners::*;
use meld_execution::capability::CapabilityInvoker;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Default)]
pub struct SecurityPackageOwner {
    policies: Option<DependencySecurityPolicyRegistry>,
    runtime: Option<OwnerRuntimePreparationV1>,
    observation: Option<Box<dyn NativeObservationOwner>>,
    active: Option<ParticipantLifecycleContextV1>,
    contributor: DependencySecurityCapabilityContributor,
}
fn implementation(id: &str) -> String {
    format!("dependency-security.serialized-local.v1::{id}")
}
impl SecurityPackageOwner {
    pub fn description() -> OwnerDescriptionV1 {
        let contributor = DependencySecurityCapabilityContributor::default();
        OwnerDescriptionV1 {
            protocol_version: OWNER_PROTOCOL_VERSION,
            owner_id: super::publication::OWNER.into(),
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
                owner_domain: super::publication::OWNER.into(),
                kind: crate::theory::ParticipantKind::BoundedActor,
                required: true,
                depends_on: Default::default(),
                readiness_contract_ref: "dependency-security.readiness.v1".into(),
                wake_contract_ref: "dependency-security.wake.v1".into(),
                safe_point_contract_ref: "dependency-security.safe-point.v1".into(),
                stop_contract_ref: "dependency-security.stop.v1".into(),
            }),
        }
    }
    fn prepare_bindings(
        &self,
        subject: meld_events::DomainObjectRef,
        bindings: std::collections::BTreeMap<String, String>,
        revisions: Vec<crate::theory::InstalledTheoryComponentRef>,
    ) -> OwnerResult {
        let bindings = bind_selected_policy(
            OwnerBindingView::new(bindings),
            self.policies
                .as_ref()
                .ok_or_else(|| failure("Security revision store is not open"))?,
            &revisions
                .iter()
                .map(|c| c.owner_revision.clone())
                .collect::<Vec<_>>(),
            &subject,
        )
        .map_err(failure)?;
        let capability = self
            .contributor
            .capability(ASSESS, &bindings)
            .map_err(failure)?;
        let condition_scope = super::condition::scope(
            &subject,
            &capability
                .policy
                .revision_ref()
                .map_err(failure)?
                .content_hash,
        )
        .map_err(failure)?;
        let mut retained = BTreeSet::from([
            (
                "dependency-security-observation".into(),
                super::observation::EVENT.into(),
                subject.object_id.clone(),
            ),
            (
                "dependency-security-observation".into(),
                super::observation::INVENTORY_EVENT.into(),
                subject.object_id.clone(),
            ),
            (
                "dependency-security-currency".into(),
                super::currency::EVENT.into(),
                subject.object_id.clone(),
            ),
            (
                "dependency-security-condition".into(),
                super::condition::EVENT.into(),
                condition_scope.scope_id,
            ),
        ]);
        for kind in [INVENTORY, ADVISORIES] {
            retained.insert((
                "dependency-security-observation".into(),
                super::publication::EVENT.into(),
                super::publication::product_scope(&subject, kind)
                    .map_err(failure)?
                    .scope_id,
            ));
        }
        let grants = Self::description()
            .implementations
            .into_iter()
            .map(|offer| {
                let kind = match offer.contract_ref.selector.capability_type_id.as_str() {
                    OBSERVE_INVENTORY => INVENTORY,
                    ACQUIRE_ADVISORIES => ADVISORIES,
                    ASSESS => ASSESSMENT,
                    VERIFY => VERIFICATION,
                    _ => unreachable!(),
                };
                Ok(OwnerCapabilityGrantV1 {
                    selection: OwnerInvocationSelectionV1 {
                        contract_ref: offer.contract_ref,
                        implementation_ref: offer.implementation_ref,
                    },
                    publications: BTreeSet::from([
                        (
                            super::publication::RECEIPT_EVENT.into(),
                            subject.object_id.clone(),
                        ),
                        (
                            super::publication::EVENT.into(),
                            super::publication::product_scope(&subject, kind)
                                .map_err(failure)?
                                .scope_id,
                        ),
                    ]),
                    provider_frame_types: Default::default(),
                    continuation_publications: retained.clone(),
                })
            })
            .collect::<Result<Vec<_>, OwnerDiagnosticV1>>()?;
        encode_owner_result(OwnerPreparedBindingsV1 {
            bindings: bindings.values().clone(),
            observation_publications: Default::default(),
            observation_provider_frame_types: Default::default(),
            retained_observation_publications: retained,
            capability_grants: grants,
        })
    }
    fn prepare_runtime(
        &mut self,
        preparation: OwnerRuntimePreparationV1,
        callbacks: Arc<dyn OwnerCallbackPort>,
    ) -> OwnerResult {
        let native = preparation
            .observation
            .as_ref()
            .ok_or_else(|| failure("Security native observation products are absent"))?;
        let bindings = OwnerBindingView::new(preparation.bindings.clone());
        let sources = self
            .contributor
            .observation_sources(&bindings, &published_contracts())
            .map_err(failure)?;
        if sources.iter().any(|source| {
            !preparation.installed_revisions.iter().any(|component| {
                source
                    .policy
                    .revision_ref()
                    .is_ok_and(|reference| reference == component.owner_revision)
            })
        }) {
            return Err(failure(
                "Security runtime policy differs from its installed revision",
            ));
        }
        let route = native
            .event_routes
            .iter()
            .find(|route| {
                route.owner_id == super::publication::OWNER
                    && route.event_type == super::condition::EVENT
            })
            .ok_or_else(|| failure("Security Graph route is absent"))?
            .clone();
        self.observation = Some(
            super::runtime::SecurityObservationBinding {
                sources,
                authority: native.authority.clone(),
                events: remote_events(preparation.ledger_id, callbacks),
                clock: Arc::new(super::observation::now),
                route,
            }
            .build(),
        );
        self.runtime = Some(preparation);
        encode_owner_result(())
    }
    fn invoke(
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
                "Security invocation does not name its exact selected implementation",
            ));
        }
        let prepared = self
            .runtime
            .as_ref()
            .ok_or_else(|| failure("Security runtime is not prepared"))?;
        let capability = self
            .contributor
            .capability(
                &init.capability_type_id,
                &OwnerBindingView::new(prepared.bindings.clone()),
            )
            .map_err(failure)?;
        let events = remote_events(prepared.ledger_id, callbacks.clone());
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(failure)?;
        if recovery {
            encode_capability_result(executor.block_on(capability.recover(
                Some(&events.replay_capability()),
                &init,
                &payload,
                context.as_ref(),
            )))
        } else {
            encode_capability_result(executor.block_on(capability.invoke_owned(
                &CallbackExecutionPorts::new(events, callbacks),
                &init,
                &payload,
                context.as_ref(),
            )))
        }
    }
}
impl PackageOwner for SecurityPackageOwner {
    fn handle(
        &mut self,
        command: OwnerCommandV1,
        callbacks: Arc<dyn OwnerCallbackPort>,
    ) -> OwnerResult {
        use OwnerCommandV1::*;
        match command {
            Describe => encode_owner_result(Self::description()),
            OpenRevisionStore { state_root } => {
                self.policies = Some(
                    DependencySecurityPolicyRegistry::new(
                        sled::open(state_root.join("revisions.sled")).map_err(failure)?,
                    )
                    .map_err(failure)?,
                );
                encode_owner_result(())
            }
            CloseRevisionStore => {
                self.policies = None;
                encode_owner_result(())
            }
            command @ (ValidateTheory { .. }
            | InstallTheory { .. }
            | VerifyTheory { .. }
            | ValidateLinks { .. }) => dispatch_theory(
                super::theory::route_handler(
                    self.policies
                        .as_ref()
                        .ok_or_else(|| failure("Security revision store is not open"))?
                        .clone(),
                )
                .as_ref(),
                command,
            ),
            PrepareBindings {
                subject,
                bindings,
                installed_revisions,
            } => self.prepare_bindings(subject, bindings, installed_revisions),
            PrepareRuntime { preparation } => self.prepare_runtime(preparation, callbacks),
            ResolveCurationSource { template, binding } => encode_owner_result(
                super::condition::curation_source(&template, &binding).map_err(failure)?,
            ),
            Invoke {
                selection,
                runtime_init,
                payload,
                event_context,
            } => self.invoke(
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
            } => self.invoke(
                selection,
                runtime_init,
                payload,
                event_context,
                callbacks,
                true,
            ),
            Flush => encode_owner_result(()),
            command => dispatch_observation(
                self.observation
                    .as_mut()
                    .ok_or_else(|| failure("Security observation is not prepared"))?
                    .as_mut(),
                &mut self.active,
                command,
            ),
        }
    }
}
fn remote_events(
    ledger: meld_events::LedgerIdentity,
    callbacks: Arc<dyn OwnerCallbackPort>,
) -> meld_events::EventAppendCapability {
    meld_events::EventAppendCapability::from_remote(
        ledger,
        Arc::new(CallbackEventAuthorityClient::new(callbacks)),
    )
}
fn failure(error: impl ToString) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("security_owner_invalid", error)
}
