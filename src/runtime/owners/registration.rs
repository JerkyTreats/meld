//! Inert publication and exact theory routing for an operator-selected owner.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;

use super::{
    NoOwnerCallbacks, OwnerCommandV1, OwnerConnection, OwnerConnectionLimitsV1, OwnerDescriptionV1,
    OwnerDiagnosticV1, OwnerExecutableV1, OWNER_PROTOCOL_VERSION,
};
use crate::theory::{OwnerRouteDiagnostic, PortBackedTheoryRouteHandler, TheoryRouteHandler};

/// Installation owns a separate inert connection. Operational participants must
/// prepare their assignment connection with the native supervisor's authority.
pub struct RegisteredOwner {
    description: OwnerDescriptionV1,
    connection: Arc<InstallationConnection>,
}

impl RegisteredOwner {
    pub fn open(
        expected_owner: &str,
        executable: &OwnerExecutableV1,
        implementation_root: &Path,
        revision_root: &Path,
        limits: OwnerConnectionLimitsV1,
    ) -> Result<Self, OwnerDiagnosticV1> {
        if expected_owner.trim().is_empty() || !revision_root.is_absolute() {
            return Err(OwnerDiagnosticV1::new(
                "owner_binding_invalid",
                "owner identity and absolute revision root are required",
            ));
        }
        let mut connection = OwnerConnection::start(executable, implementation_root, limits)?;
        let description: OwnerDescriptionV1 =
            connection.call(OwnerCommandV1::Describe, &mut NoOwnerCallbacks)?;
        validate_description(expected_owner, &description)?;
        Ok(Self {
            description,
            connection: Arc::new(InstallationConnection {
                connection: Mutex::new(connection),
                revision_root: revision_root.into(),
            }),
        })
    }

    pub fn description(&self) -> &OwnerDescriptionV1 {
        &self.description
    }

    pub fn prepare_bindings(
        &self,
        assignment_scope_id: String,
        subject: meld_events::DomainObjectRef,
        bindings: std::collections::BTreeMap<String, String>,
        installed_revisions: Vec<crate::theory::InstalledTheoryComponentRef>,
    ) -> Result<super::OwnerPreparedBindingsV1, OwnerDiagnosticV1> {
        self.connection.call(OwnerCommandV1::PrepareBindings {
            assignment_scope_id,
            subject,
            bindings,
            installed_revisions,
        })
    }

    pub fn curation_source(
        &self,
        template: meld_world_model::curation::CurationRuleTemplate,
        binding: meld_world_model::curation::CurationRuleBinding,
    ) -> Result<meld_world_model::curation::CurationSourceBinding, OwnerDiagnosticV1> {
        self.connection.call(OwnerCommandV1::ResolveCurationSource {
            template: Box::new(template),
            binding,
        })
    }

    /// Reuse the canonical Theory route adapter and installer. Owner bytes and
    /// exact revisions are opaque to this host.
    pub fn route_handlers(&self) -> Vec<Arc<dyn TheoryRouteHandler>> {
        self.description
            .routes
            .iter()
            .map(|contract| {
                let validate_connection = self.connection.clone();
                let install_connection = self.connection.clone();
                let verify_connection = self.connection.clone();
                let link_connection = self.connection.clone();
                let validate_route = contract.route.clone();
                let install_route = contract.route.clone();
                let verify_route = contract.route.clone();
                Arc::new(PortBackedTheoryRouteHandler::new(
                    contract.clone(),
                    Arc::new(move |owner_component_id, bytes| {
                        call(
                            &validate_connection,
                            OwnerCommandV1::ValidateTheory {
                                route: validate_route.clone(),
                                owner_component_id: owner_component_id.into(),
                                canonical_bytes: bytes.to_vec(),
                            },
                        )
                    }),
                    Arc::new(move |owner_component_id, bytes, installed_at_seq| {
                        call(
                            &install_connection,
                            OwnerCommandV1::InstallTheory {
                                route: install_route.clone(),
                                owner_component_id: owner_component_id.into(),
                                canonical_bytes: bytes.to_vec(),
                                installed_at_seq,
                            },
                        )
                    }),
                    Arc::new(move |reference| {
                        call(
                            &verify_connection,
                            OwnerCommandV1::VerifyTheory {
                                route: verify_route.clone(),
                                reference: reference.clone(),
                            },
                        )
                    }),
                    Arc::new(move |component, package| {
                        call(
                            &link_connection,
                            OwnerCommandV1::ValidateLinks {
                                component: component.clone(),
                                package: package.clone(),
                            },
                        )
                    }),
                )) as Arc<dyn TheoryRouteHandler>
            })
            .collect()
    }
}

struct InstallationConnection {
    connection: Mutex<OwnerConnection>,
    revision_root: std::path::PathBuf,
}

impl InstallationConnection {
    fn call<T: DeserializeOwned>(&self, command: OwnerCommandV1) -> Result<T, OwnerDiagnosticV1> {
        use fs2::FileExt;
        std::fs::create_dir_all(&self.revision_root).map_err(installation_error)?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.revision_root.join("installation.lock"))
            .map_err(installation_error)?;
        lock.try_lock_exclusive()
            .map_err(|error| OwnerDiagnosticV1::new("owner_revision_store_busy", error))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| installation_error("owner connection lock poisoned"))?;
        if let Err(error) = connection.call::<()>(
            OwnerCommandV1::OpenRevisionStore {
                state_root: self.revision_root.clone(),
            },
            &NoOwnerCallbacks,
        ) {
            connection.invalidate();
            return Err(error);
        }
        let result = connection.call(command, &NoOwnerCallbacks);
        let closed = connection.call::<()>(OwnerCommandV1::CloseRevisionStore, &NoOwnerCallbacks);
        if closed.is_err() {
            connection.invalidate();
        }
        // Closing the semantic store happens while the parent still holds its
        // physical lease, including when the routed owner operation failed.
        match (result, closed) {
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(value),
        }
    }
}

fn installation_error(error: impl ToString) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("owner_revision_store_unavailable", error)
}

fn call<T: DeserializeOwned>(
    connection: &InstallationConnection,
    command: OwnerCommandV1,
) -> Result<T, OwnerRouteDiagnostic> {
    connection
        .call(command)
        .map_err(|error| OwnerRouteDiagnostic::new(error.code, error.message))
}

pub(crate) fn validate_description(
    expected_owner: &str,
    description: &OwnerDescriptionV1,
) -> Result<(), OwnerDiagnosticV1> {
    let invalid = |message: &str| OwnerDiagnosticV1::new("owner_description_invalid", message);
    if description.protocol_version != OWNER_PROTOCOL_VERSION
        || description.owner_id != expected_owner
    {
        return Err(invalid(
            "selected owner or protocol differs from the executable description",
        ));
    }
    if description
        .observation_participant
        .as_ref()
        .is_some_and(|participant| participant.owner_domain != expected_owner)
    {
        return Err(invalid("owner published a foreign observation participant"));
    }
    let mut routes = BTreeSet::new();
    for contract in &description.routes {
        if contract.route.owner_domain != expected_owner
            || !routes.insert(contract.route.clone())
            || contract.route.route_version == 0
            || contract.handler_contract_version == 0
            || contract.accepted_component_schema.min == 0
            || contract.accepted_component_schema.min > contract.accepted_component_schema.max
        {
            return Err(invalid(
                "owner published foreign, duplicate, or invalid routes",
            ));
        }
    }
    let mut contracts = BTreeSet::new();
    for revision in &description.capabilities {
        revision
            .contract
            .validate()
            .map_err(|error| invalid(&error.to_string()))?;
        if revision.contract.owning_domain != expected_owner
            || revision.content_identity != revision.contract.content_identity()
            || !contracts.insert(revision.revision_ref())
        {
            return Err(invalid(
                "owner published foreign, duplicate, or altered capability contracts",
            ));
        }
    }
    let mut implementations = BTreeSet::new();
    for offer in &description.implementations {
        if !contracts.contains(&offer.contract_ref)
            || offer.implementation_ref.trim().is_empty()
            || !implementations.insert((&offer.contract_ref, &offer.implementation_ref))
            || offer
                .required_binding_ids
                .iter()
                .any(|id| id.trim().is_empty())
        {
            return Err(invalid(
                "owner implementation is ambiguous or names an unpublished contract",
            ));
        }
    }
    Ok(())
}
