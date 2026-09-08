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
    connection: Arc<Mutex<OwnerConnection>>,
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
        connection.call::<()>(
            OwnerCommandV1::OpenRevisionStore {
                state_root: revision_root.into(),
            },
            &mut NoOwnerCallbacks,
        )?;
        Ok(Self {
            description,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn description(&self) -> &OwnerDescriptionV1 {
        &self.description
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

fn call<T: DeserializeOwned>(
    connection: &Mutex<OwnerConnection>,
    command: OwnerCommandV1,
) -> Result<T, OwnerRouteDiagnostic> {
    connection
        .lock()
        .map_err(|_| {
            OwnerRouteDiagnostic::new(
                "owner_connection_unavailable",
                "owner installation connection lock poisoned",
            )
        })?
        .call(command, &mut NoOwnerCallbacks)
        .map_err(|error| OwnerRouteDiagnostic::new(error.code, error.message))
}

fn validate_description(
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
