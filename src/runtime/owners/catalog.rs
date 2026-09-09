//! Operator-selected owner executables composed through the native inventories.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

use meld_events::events::remote::{EventAuthorityContract, LocalEventAuthorityClient};
use meld_events::EventAuthority;
use serde::{Deserialize, Serialize};

use super::registration::RegisteredOwner;
use super::runtime::{OwnerRuntimeGrants, PreparedOwnerRuntime};
use super::*;
use crate::capability::*;
use crate::config::{PhysicalBinding, PhysicalBindingRef};
use crate::runtime::lifecycle::NativeObservationOwnerFactory;
use crate::theory::{InstalledTheoryComponentRef, TheoryRouteHandler};

/// An explicit physical grant. Package descriptions cannot acquire bindings by
/// merely asking for their names. This value is retained in activation identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerSelectionV1 {
    pub executable: OwnerExecutableV1,
    pub limits: OwnerConnectionLimitsV1,
    pub binding_ids: BTreeSet<String>,
}

#[derive(Clone, Default)]
pub struct OwnerCatalog {
    owners: BTreeMap<String, Arc<SelectedOwner>>,
}

struct SelectedOwner {
    registration: RegisteredOwner,
    selection: OwnerSelectionV1,
    root: PathBuf,
    ledger_id: meld_events::LedgerIdentity,
    events: Arc<dyn EventAuthorityContract>,
    instances: Mutex<BTreeMap<(String, String), Weak<PreparedOwnerRuntime>>>,
}

/// Owner-issued immutable resources and native observation products participate
/// in capability preparation identity. The host does not decode resource bodies.
#[derive(Clone, Serialize, Deserialize)]
struct RuntimeSeed {
    resources: OwnerPreparedBindingsV1,
    installed_revisions: Vec<InstalledTheoryComponentRef>,
    observation: OwnerObservationPreparationV1,
    provider_id: Option<String>,
    agent_id: String,
}

fn seed_key(owner: &str) -> String {
    format!("owner-runtime::{owner}")
}

impl OwnerCatalog {
    pub fn open(
        binding: &PhysicalBinding,
        authority: &EventAuthority,
    ) -> Result<Self, OwnerDiagnosticV1> {
        let mut owners = BTreeMap::new();
        for (key, value) in &binding.bindings {
            let Some(owner_id) = key.strip_prefix("owner::") else {
                continue;
            };
            let PhysicalBindingRef::ConfigRef(json) = value else {
                return Err(failure(
                    "owner selection must be a config reference containing OwnerSelectionV1",
                ));
            };
            let selection: OwnerSelectionV1 = serde_json::from_str(json).map_err(failure)?;
            let root = binding
                .storage_root
                .join("owners")
                .join(blake3::hash(owner_id.as_bytes()).to_hex().as_str());
            let executable =
                retained_selection(&selection.executable, &root.join("implementations"))?;
            let registration = RegisteredOwner::open(
                owner_id,
                &executable,
                &root.join("implementations"),
                &root.join("revisions"),
                selection.limits.clone(),
            )?;
            owners.insert(
                owner_id.to_string(),
                Arc::new(SelectedOwner {
                    registration,
                    selection,
                    root,
                    ledger_id: authority.ledger_identity(),
                    events: Arc::new(LocalEventAuthorityClient::new(authority)),
                    instances: Mutex::new(BTreeMap::new()),
                }),
            );
        }
        Ok(Self { owners })
    }

    /// Refuse incompatible retained state before creating a replacement owner instance.
    pub fn check_legacy_state(
        &self,
        stores: &crate::runtime::storage::OpenProductStores,
    ) -> Result<(), OwnerDiagnosticV1> {
        let mut databases = Vec::new();
        if let Some(db) = stores.theory_db.opened() {
            databases.push(db);
        }
        if let Some(traversal) = stores.traversal_store.opened() {
            databases.push(traversal.db());
        }
        for description in self.descriptions() {
            for db in &databases {
                for name in db.tree_names() {
                    if description
                        .incompatible_legacy_trees
                        .iter()
                        .any(|legacy| legacy.as_bytes() == name.as_ref())
                        && !db.open_tree(&name).map_err(failure)?.is_empty()
                    {
                        return Err(OwnerDiagnosticV1::new("owner_history_incompatible", format!(
                            "owner '{}' cannot import retained legacy tree '{}'; preserve that history and use an explicit owner migration before activation", description.owner_id, String::from_utf8_lossy(&name))));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn placement_for(
        &self,
        selections: &BTreeMap<meld_execution::capability::CapabilityContractRevisionRef, String>,
        participants: &crate::theory::ActivationParticipantPlanV1,
    ) -> crate::config::AdapterPlacement {
        if self.descriptions().any(|owner| {
            owner
                .observation_participant
                .as_ref()
                .is_some_and(|spec| participants.participants.contains(spec))
                || owner.implementations.iter().any(|offer| {
                    selections.get(&offer.contract_ref) == Some(&offer.implementation_ref)
                })
        }) {
            crate::config::AdapterPlacement::SerializedLocal
        } else {
            crate::config::AdapterPlacement::InProcess
        }
    }

    pub fn is_empty(&self) -> bool {
        self.owners.is_empty()
    }

    pub fn descriptions(&self) -> impl Iterator<Item = &OwnerDescriptionV1> {
        self.owners
            .values()
            .map(|owner| owner.registration.description())
    }

    pub fn route_handlers(&self) -> Vec<Arc<dyn TheoryRouteHandler>> {
        self.owners
            .values()
            .flat_map(|owner| owner.registration.route_handlers())
            .collect()
    }

    pub fn contributors(&self) -> Vec<Arc<dyn ProductCapabilityContributor>> {
        self.owners
            .values()
            .cloned()
            .map(|owner| {
                Arc::new(ExternalContributor(owner)) as Arc<dyn ProductCapabilityContributor>
            })
            .collect()
    }

    pub fn prepare_bindings(
        &self,
        binding: &PhysicalBinding,
        revisions: &[InstalledTheoryComponentRef],
        observation: &OwnerObservationPreparationV1,
    ) -> Result<OwnerBindingView, OwnerDiagnosticV1> {
        let physical = binding.owner_binding_values();
        let mut prepared = OwnerBindingView::new(physical.clone());
        for (id, owner) in &self.owners {
            let description = owner.registration.description();
            let selected = revisions.iter().any(|component| {
                &component.route.owner_domain == id
                    || description.capabilities.iter().any(|contract| {
                        component.route.owner_domain == "execution"
                            && component.route.component_kind == "capability-contract"
                            && component.owner_revision.content_hash == contract.content_identity
                    })
            }) || observation
                .event_routes
                .iter()
                .any(|route| &route.owner_id == id);
            if !selected {
                continue;
            }
            let scoped = owner
                .selection
                .binding_ids
                .iter()
                .map(|key| {
                    physical
                        .get(key)
                        .cloned()
                        .map(|value| (key.clone(), value))
                        .ok_or_else(|| {
                            failure(format!(
                                "owner '{id}' has an unresolved granted binding '{key}'"
                            ))
                        })
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            let resources = owner.registration.prepare_bindings(
                binding.assignment_scope_id(),
                binding.subject.clone(),
                scoped,
                revisions.to_vec(),
            )?;
            let mut native = observation.clone();
            native.event_routes.retain(|route| &route.owner_id == id);
            let seed = RuntimeSeed {
                resources,
                installed_revisions: revisions.to_vec(),
                observation: native,
                provider_id: binding
                    .provider_id
                    .clone()
                    .filter(|_| owner.selection.binding_ids.contains("provider")),
                agent_id: binding.agent_id.clone(),
            };
            prepared =
                prepared.with_value(seed_key(id), serde_json::to_string(&seed).map_err(failure)?);
        }
        Ok(prepared)
    }

    pub fn observations(
        &self,
        assignment_id: &str,
        activation_id: &str,
        bindings: &OwnerBindingView,
        participants: &crate::theory::ActivationParticipantPlanV1,
    ) -> Result<BTreeMap<String, Arc<dyn NativeObservationOwnerFactory>>, OwnerDiagnosticV1> {
        self.owners
            .values()
            .filter_map(|owner| {
                owner
                    .registration
                    .description()
                    .observation_participant
                    .as_ref()
                    .filter(|participant| {
                        participants
                            .participants
                            .iter()
                            .any(|selected| selected == *participant)
                    })
                    .map(|participant| {
                        owner
                            .runtime(assignment_id, activation_id, bindings)
                            .map(|runtime| {
                                (
                                    participant.participant_id.clone(),
                                    Arc::new(runtime) as Arc<dyn NativeObservationOwnerFactory>,
                                )
                            })
                    })
            })
            .collect()
    }

    pub fn runtimes(
        &self,
        assignment_id: &str,
        activation_id: &str,
        bindings: &OwnerBindingView,
    ) -> Result<Vec<Arc<PreparedOwnerRuntime>>, OwnerDiagnosticV1> {
        self.owners
            .iter()
            .filter(|(id, _)| bindings.contains(&seed_key(id)))
            .map(|(_, owner)| owner.runtime(assignment_id, activation_id, bindings))
            .collect()
    }

    pub fn curation_source(
        &self,
        template: &meld_world_model::curation::CurationRuleTemplate,
        binding: &meld_world_model::curation::CurationRuleBinding,
    ) -> Result<Option<meld_world_model::curation::CurationSourceBinding>, String> {
        self.owners
            .get(&template.source_owner_id)
            .map(|owner| {
                owner
                    .registration
                    .curation_source(template.clone(), binding.clone())
                    .map_err(|error| error.to_string())
            })
            .transpose()
    }
}

impl SelectedOwner {
    fn runtime(
        &self,
        assignment_id: &str,
        activation_id: &str,
        bindings: &OwnerBindingView,
    ) -> Result<Arc<PreparedOwnerRuntime>, OwnerDiagnosticV1> {
        let id = &self.registration.description().owner_id;
        let seed: RuntimeSeed =
            serde_json::from_str(bindings.get(&seed_key(id)).ok_or_else(|| {
                failure(format!("owner '{id}' has no exact runtime preparation"))
            })?)
            .map_err(failure)?;
        let key = (assignment_id.to_string(), activation_id.to_string());
        let mut instances = self
            .instances
            .lock()
            .map_err(|_| failure("owner instance registry lock poisoned"))?;
        if let Some(existing) = instances.get(&key).and_then(Weak::upgrade) {
            return Ok(existing);
        }
        let executable = retained_selection(
            &self.selection.executable,
            &self.root.join("implementations"),
        )?;
        let runtime = PreparedOwnerRuntime::prepare(
            OwnerConnection::start(
                &executable,
                &self.root.join("implementations"),
                self.selection.limits.clone(),
            )?,
            self.registration.description().clone(),
            OwnerRuntimePreparationV1 {
                assignment_id: assignment_id.into(),
                activation_id: activation_id.into(),
                participant_id: self
                    .registration
                    .description()
                    .observation_participant
                    .as_ref()
                    .map(|spec| spec.participant_id.clone())
                    .unwrap_or_else(|| format!("owner::{id}")),
                state_root: self
                    .root
                    .join("assignments")
                    .join(seed.observation.scope.scope_id.as_str()),
                bindings: seed.resources.bindings.clone(),
                installed_revisions: seed.installed_revisions,
                ledger_id: self.ledger_id,
                observation: Some(seed.observation.clone()),
            },
            OwnerRuntimeGrants {
                events: self.events.clone(),
                observation_routes: seed.resources.observation_publications,
                invocation_routes: seed
                    .resources
                    .capability_grants
                    .iter()
                    .map(|grant| (grant.selection.clone(), grant.publications.clone()))
                    .collect(),
                observation_session: seed.observation.session_id,
                retained_observation_publications: seed.resources.retained_observation_publications,
                invocation_continuations: seed
                    .resources
                    .capability_grants
                    .iter()
                    .map(|grant| {
                        (
                            grant.selection.clone(),
                            grant.continuation_publications.clone(),
                        )
                    })
                    .collect(),
                provider_binding: seed
                    .provider_id
                    .as_ref()
                    .map(|id| {
                        crate::provider::ProviderExecutionBinding::new(id, Default::default())
                            .map_err(failure)
                    })
                    .transpose()?,
                observation_provider_frame_types: seed.resources.observation_provider_frame_types,
                invocation_provider_frame_types: seed
                    .resources
                    .capability_grants
                    .iter()
                    .map(|grant| (grant.selection.clone(), grant.provider_frame_types.clone()))
                    .collect(),
                agent_id: seed.agent_id,
            },
        )?;
        instances.insert(key, Arc::downgrade(&runtime));
        Ok(runtime)
    }
}

struct ExternalContributor(Arc<SelectedOwner>);
impl ProductCapabilityContributor for ExternalContributor {
    fn owner_domain(&self) -> &str {
        &self.0.registration.description().owner_id
    }
    fn published_contracts(&self) -> Vec<meld_execution::capability::CapabilityContractRevision> {
        self.0.registration.description().capabilities.clone()
    }
    fn implementation_offers(&self) -> Vec<CapabilityImplementationOffer> {
        let description = self.0.registration.description();
        description
            .implementations
            .iter()
            .map(|offer| CapabilityImplementationOffer {
                contract_ref: offer.contract_ref.clone(),
                implementation_ref: offer.implementation_ref.clone(),
                required_binding_ids: BTreeSet::from([seed_key(&description.owner_id)]),
                execution_class: description
                    .capabilities
                    .iter()
                    .find(|revision| revision.revision_ref() == offer.contract_ref)
                    .expect("registered exact offer")
                    .contract
                    .execution_contract
                    .execution_class,
                factory: Arc::new(ExternalFactory {
                    owner: self.0.clone(),
                    selection: OwnerInvocationSelectionV1 {
                        contract_ref: offer.contract_ref.clone(),
                        implementation_ref: offer.implementation_ref.clone(),
                    },
                }),
            })
            .collect()
    }
}
struct ExternalFactory {
    owner: Arc<SelectedOwner>,
    selection: OwnerInvocationSelectionV1,
}
impl CapabilityInvokerFactory for ExternalFactory {
    fn prepare(
        &self,
        request: &CapabilityFactoryRequest<'_>,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic> {
        let runtime = self
            .owner
            .runtime(request.assignment_id, request.activation_id, bindings)
            .map_err(contribution_failure)?;
        runtime
            .invoker(
                &self.selection.contract_ref,
                &self.selection.implementation_ref,
            )
            .map_err(contribution_failure)
    }
}

fn retained_selection(
    selected: &OwnerExecutableV1,
    root: &std::path::Path,
) -> Result<OwnerExecutableV1, OwnerDiagnosticV1> {
    // The exact retained artifact is the authority for predecessor reconstruction.
    // Validate the digest syntax before using it as a physical path component.
    if selected.content_hash.len() != 64
        || !selected
            .content_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(failure(
            "owner executable digest must be a BLAKE3 hex identity",
        ));
    }
    let retained = root.join(&selected.content_hash);
    Ok(OwnerExecutableV1 {
        path: if retained.is_file() {
            retained
        } else {
            selected.path.clone()
        },
        content_hash: selected.content_hash.clone(),
    })
}
fn failure(error: impl ToString) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("owner_configuration_invalid", error)
}
fn contribution_failure(error: OwnerDiagnosticV1) -> CapabilityContributionDiagnostic {
    CapabilityContributionDiagnostic::new(error.code, error.message)
}
