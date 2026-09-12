//! Durable PDS product declarations and complete compilation receipts.
//!
//! Package routing proves one package at a time. This module owns the
//! separate product boundary that selects an exact package set and makes it
//! selectable only after every package has one complete installation receipt.

mod selection;
pub use selection::position_packages;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::capability::CapabilityPreparationReceiptV1;
use crate::config::StewardshipAssignmentV1;
use crate::theory::activation::{ActivationParticipantPlanV1, PreparedActivationClosureV1};
use meld_world_model::agent::{AgentGenesisReceiptV1, AgentStore};

use super::contracts::{InstalledTheoryComponentRef, TheoryRouteId};
use super::error::{error, TheoryRouterError};
use super::receipt::PdsPackageInstallationReceiptV1;
use super::registry::PdsPackageStore;

const TREE_DECLARATIONS: &str = "pds_product_declarations_v1";
const TREE_COMPILATIONS: &str = "pds_product_compilations_v1";
const TREE_HEADS: &str = "pds_product_compilation_heads_v1";
const TREE_ASSIGNMENTS: &str = "pds_product_assignments_v1";
const TREE_TOPOLOGY_RECEIPTS: &str = "pds_product_agent_topology_receipts_v1";
const TREE_CAPABILITY_PREPARATIONS: &str = "pds_product_capability_preparations_v1";
const TREE_PREPARED_CLOSURES: &str = "pds_product_prepared_closures_v1";
const TREE_PREPARED_HEADS: &str = "pds_product_prepared_heads_v1";

/// One exact package revision selected by a product declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductPackageSelectionV1 {
    pub package_id: String,
    pub package_version: String,
    pub package_content_hash: String,
}

/// One source relationship required by a declared Agent position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductAgentSubscriptionV1 {
    pub source_owner: String,
    pub source_contract_component_id: String,
    pub initial_cursor_policy: String,
}

/// One finite Agent position declared by a reusable product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductAgentPositionV1 {
    /// Exact WAD package identity in the composition's retained receipt closure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    pub position_id: String,
    pub directive: String,
    pub required_owner_routes: Vec<TheoryRouteId>,
    pub observation_scope_component_id: String,
    pub required_subscriptions: Vec<ProductAgentSubscriptionV1>,
    pub participant_ref: String,
}

/// Exact factory selection for one declared runtime participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductParticipantBindingV1 {
    pub factory_id: String,
    pub agent_position_id: String,
}

/// One immutable principal-facing product composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductDeclarationV1 {
    /// Reusable factory and Agent context for explicitly realized participants.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub participant_bindings: BTreeMap<String, ProductParticipantBindingV1>,
    pub product_revision_id: String,
    pub product_id: String,
    pub principal_id: String,
    pub selected_packages: Vec<ProductPackageSelectionV1>,
    pub agent_topology: Vec<ProductAgentPositionV1>,
    pub participant_plan: ActivationParticipantPlanV1,
    pub requested_authority_ref: String,
    pub principal_grant_ref: String,
    pub compilation_policy_revision: String,
}

#[derive(Serialize)]
struct ProductDeclarationIdentity<'a> {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    participant_bindings: &'a BTreeMap<String, ProductParticipantBindingV1>,
    product_id: &'a str,
    principal_id: &'a str,
    selected_packages: &'a [ProductPackageSelectionV1],
    agent_topology: &'a [ProductAgentPositionV1],
    participant_plan: &'a ActivationParticipantPlanV1,
    requested_authority_ref: &'a str,
    principal_grant_ref: &'a str,
    compilation_policy_revision: &'a str,
}

pub(super) fn valid_agent_topology(
    positions: &[ProductAgentPositionV1],
    plan: &ActivationParticipantPlanV1,
) -> bool {
    !positions.is_empty()
        && positions
            .iter()
            .map(|position| &position.position_id)
            .collect::<BTreeSet<_>>()
            .len()
            == positions.len()
        && positions.iter().all(|position| {
            !position.position_id.trim().is_empty()
                && position
                    .package_id
                    .as_ref()
                    .is_none_or(|id| !id.trim().is_empty())
                && !position.directive.trim().is_empty()
                && !position.required_owner_routes.is_empty()
                && !position.observation_scope_component_id.trim().is_empty()
                && !position.required_subscriptions.is_empty()
                && position.required_subscriptions.iter().all(|subscription| {
                    !subscription.source_owner.trim().is_empty()
                        && !subscription.source_contract_component_id.trim().is_empty()
                        && !subscription.initial_cursor_policy.trim().is_empty()
                })
                && plan
                    .participants
                    .iter()
                    .any(|participant| participant.participant_id == position.participant_ref)
        })
}

impl ProductDeclarationV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        product_id: String,
        principal_id: String,
        mut selected_packages: Vec<ProductPackageSelectionV1>,
        mut agent_topology: Vec<ProductAgentPositionV1>,
        participant_plan: ActivationParticipantPlanV1,
        requested_authority_ref: String,
        principal_grant_ref: String,
        compilation_policy_revision: String,
        participant_bindings: BTreeMap<String, ProductParticipantBindingV1>,
    ) -> Result<Self, TheoryRouterError> {
        participant_plan.verify_identity()?;
        selected_packages.sort();
        agent_topology.sort_by(|left, right| left.position_id.cmp(&right.position_id));
        for position in &mut agent_topology {
            position.required_owner_routes.sort();
            position.required_owner_routes.dedup();
            position.required_subscriptions.sort();
            position.required_subscriptions.dedup();
        }
        let required = [
            product_id.as_str(),
            principal_id.as_str(),
            requested_authority_ref.as_str(),
            principal_grant_ref.as_str(),
            compilation_policy_revision.as_str(),
        ];
        if required.iter().any(|value| value.trim().is_empty())
            || selected_packages.is_empty()
            || agent_topology.is_empty()
            || selected_packages.iter().any(|package| {
                package.package_id.trim().is_empty()
                    || package.package_version.trim().is_empty()
                    || package.package_content_hash.trim().is_empty()
            })
            || !selected_packages
                .windows(2)
                .all(|pair| pair[0].package_id != pair[1].package_id)
            || !valid_agent_topology(&agent_topology, &participant_plan)
            || !valid_participant_bindings(
                &participant_bindings,
                &agent_topology,
                &participant_plan,
            )
        {
            return Err(error(
                "product_declaration_invalid",
                "product declaration identities, package set, and Agent topology must be complete",
            ));
        }
        let identity = ProductDeclarationIdentity {
            participant_bindings: &participant_bindings,
            product_id: &product_id,
            principal_id: &principal_id,
            selected_packages: &selected_packages,
            agent_topology: &agent_topology,
            participant_plan: &participant_plan,
            requested_authority_ref: &requested_authority_ref,
            principal_grant_ref: &principal_grant_ref,
            compilation_policy_revision: &compilation_policy_revision,
        };
        let product_revision_id = identity_hash(&identity)?;
        Ok(Self {
            participant_bindings,
            product_revision_id,
            product_id,
            principal_id,
            selected_packages,
            agent_topology,
            participant_plan,
            requested_authority_ref,
            principal_grant_ref,
            compilation_policy_revision,
        })
    }

    pub fn verify_identity(&self) -> Result<(), TheoryRouterError> {
        let rebuilt = Self::new(
            self.product_id.clone(),
            self.principal_id.clone(),
            self.selected_packages.clone(),
            self.agent_topology.clone(),
            self.participant_plan.clone(),
            self.requested_authority_ref.clone(),
            self.principal_grant_ref.clone(),
            self.compilation_policy_revision.clone(),
            self.participant_bindings.clone(),
        )?;
        if rebuilt != *self {
            return Err(error(
                "product_declaration_corrupt",
                "product declaration identity or canonical ordering differs",
            ));
        }
        Ok(())
    }
}

pub(super) fn valid_participant_bindings(
    bindings: &BTreeMap<String, ProductParticipantBindingV1>,
    positions: &[ProductAgentPositionV1],
    plan: &ActivationParticipantPlanV1,
) -> bool {
    bindings.iter().all(|(id, binding)| {
        !binding.factory_id.trim().is_empty()
            && plan.participants.iter().any(|p| &p.participant_id == id)
            && positions
                .iter()
                .any(|p| p.position_id == binding.agent_position_id)
            && positions
                .iter()
                .filter(|p| &p.participant_ref == id)
                .all(|p| p.position_id == binding.agent_position_id)
    })
}

pub fn product_topology_id(
    positions: &[ProductAgentPositionV1],
) -> Result<String, TheoryRouterError> {
    identity_hash(&positions)
}

/// Complete proof that every package selected by one product was installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductCompilationReceiptV1 {
    pub compilation_receipt_id: String,
    pub product_revision_id: String,
    pub package_receipt_ids: Vec<String>,
    pub installed_owner_revisions: Vec<InstalledTheoryComponentRef>,
    pub compilation_policy_revision: String,
    pub compiled_at_seq: u64,
}

#[derive(Serialize)]
struct CompilationIdentity<'a> {
    product_revision_id: &'a str,
    package_receipt_ids: &'a [String],
    installed_owner_revisions: &'a [InstalledTheoryComponentRef],
    compilation_policy_revision: &'a str,
}

impl ProductCompilationReceiptV1 {
    pub fn compile(
        declaration: &ProductDeclarationV1,
        mut receipts: Vec<PdsPackageInstallationReceiptV1>,
        package_store: &PdsPackageStore,
        compiled_at_seq: u64,
    ) -> Result<Self, TheoryRouterError> {
        declaration.verify_identity()?;
        receipts.sort_by(|left, right| left.package_id.cmp(&right.package_id));
        if receipts.len() != declaration.selected_packages.len() {
            return Err(error(
                "product_compilation_incomplete",
                "the installed package receipt set does not match the selected package set",
            ));
        }
        for receipt in &receipts {
            receipt.verify_identity()?;
        }
        for (selected, receipt) in declaration.selected_packages.iter().zip(&receipts) {
            if selected.package_id != receipt.package_id
                || selected.package_version != receipt.package_version
                || selected.package_content_hash != receipt.package_content_hash
            {
                return Err(error(
                    "product_compilation_incomplete",
                    "an installed package receipt does not match its exact product selection",
                ));
            }
        }
        let receipts = package_store.resolve_closure(
            &receipts
                .iter()
                .map(|receipt| receipt.receipt_id.clone())
                .collect::<Vec<_>>(),
        )?;
        for position in &declaration.agent_topology {
            let selected = position_packages(position, &receipts, package_store)?;
            let available_routes = selected
                .iter()
                .flat_map(|receipt| receipt.components.iter().map(|c| &c.route))
                .collect::<BTreeSet<_>>();
            if position
                .required_owner_routes
                .iter()
                .any(|route| !available_routes.contains(route))
            {
                return Err(error(
                    "product_compilation_incomplete",
                    "Agent WAD requires an owner route absent from its selected package closure",
                ));
            }
            let required_components =
                std::iter::once(position.observation_scope_component_id.as_str()).chain(
                    position
                        .required_subscriptions
                        .iter()
                        .map(|subscription| subscription.source_contract_component_id.as_str()),
                );
            for component_id in required_components {
                let matches = selected
                    .iter()
                    .flat_map(|receipt| receipt.components.iter())
                    .filter(|component| component.component_id == component_id)
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return Err(error(
                        "product_compilation_incomplete",
                        "Agent topology requires one exact component absent or ambiguous in the selected package set",
                    ));
                }
            }
        }
        let package_receipt_ids = receipts
            .iter()
            .map(|receipt| receipt.receipt_id.clone())
            .collect::<Vec<_>>();
        let mut installed_owner_revisions = receipts
            .into_iter()
            .flat_map(|receipt| receipt.components)
            .collect::<Vec<_>>();
        installed_owner_revisions.sort_by(|left, right| {
            left.route
                .cmp(&right.route)
                .then(left.component_id.cmp(&right.component_id))
                .then(
                    left.owner_revision
                        .content_hash
                        .cmp(&right.owner_revision.content_hash),
                )
        });
        let identity = CompilationIdentity {
            product_revision_id: &declaration.product_revision_id,
            package_receipt_ids: &package_receipt_ids,
            installed_owner_revisions: &installed_owner_revisions,
            compilation_policy_revision: &declaration.compilation_policy_revision,
        };
        let compilation_receipt_id = identity_hash(&identity)?;
        Ok(Self {
            compilation_receipt_id,
            product_revision_id: declaration.product_revision_id.clone(),
            package_receipt_ids,
            installed_owner_revisions,
            compilation_policy_revision: declaration.compilation_policy_revision.clone(),
            compiled_at_seq,
        })
    }

    pub fn verify_identity(&self) -> Result<(), TheoryRouterError> {
        let identity = CompilationIdentity {
            product_revision_id: &self.product_revision_id,
            package_receipt_ids: &self.package_receipt_ids,
            installed_owner_revisions: &self.installed_owner_revisions,
            compilation_policy_revision: &self.compilation_policy_revision,
        };
        if identity_hash(&identity)? != self.compilation_receipt_id {
            return Err(error(
                "product_compilation_corrupt",
                "product compilation receipt identity differs",
            ));
        }
        Ok(())
    }
}

/// Current exact product compilation selected for one product identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductCompilationHeadV1 {
    pub product_id: String,
    pub product_revision_id: String,
    pub compilation_receipt_id: String,
    pub head_revision: u64,
}

/// Complete PDS aggregate over every Agent-owned genesis receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductAgentTopologyReceiptV1 {
    pub topology_receipt_id: String,
    pub assignment_id: String,
    pub product_compilation_receipt_id: String,
    pub topology_id: String,
    pub agent_genesis_receipt_ids: Vec<String>,
}

impl ProductAgentTopologyReceiptV1 {
    pub fn new(
        assignment: &StewardshipAssignmentV1,
        declaration: &ProductDeclarationV1,
        mut receipts: Vec<AgentGenesisReceiptV1>,
    ) -> Result<Self, TheoryRouterError> {
        assignment
            .verify_identity()
            .map_err(|failure| error("product_topology_invalid", failure.to_string()))?;
        declaration.verify_identity()?;
        if assignment.product_revision_id != declaration.product_revision_id {
            return Err(error(
                "product_topology_invalid",
                "assignment product revision differs from the product declaration",
            ));
        }
        receipts.sort_by(|left, right| left.topology_position_id.cmp(&right.topology_position_id));
        for receipt in &receipts {
            receipt
                .verify_identity()
                .map_err(|failure| error("product_topology_invalid", failure.to_string()))?;
        }
        let declared = declaration
            .agent_topology
            .iter()
            .map(|position| position.position_id.as_str())
            .collect::<Vec<_>>();
        let accepted = receipts
            .iter()
            .map(|receipt| receipt.topology_position_id.as_str())
            .collect::<Vec<_>>();
        let assigned = assignment
            .agent_positions
            .iter()
            .map(|position| position.position_id.as_str())
            .collect::<Vec<_>>();
        if declared != accepted
            || declared != assigned
            || receipts.iter().any(|receipt| {
                receipt.assignment_id != assignment.assignment_id
                    || receipt.product_compilation_receipt_id
                        != assignment.product_compilation_receipt_id
                    || !assignment.agent_positions.iter().any(|position| {
                        position.position_id == receipt.topology_position_id
                            && position.agent_id == receipt.agent_id
                    })
            })
        {
            return Err(error(
                "product_topology_incomplete",
                "Agent genesis receipts do not exactly cover the assigned product topology",
            ));
        }
        let topology_id = product_topology_id(&declaration.agent_topology)?;
        if assignment.topology_id != topology_id {
            return Err(error(
                "product_topology_invalid",
                "assignment topology identity differs from the product declaration",
            ));
        }
        let agent_genesis_receipt_ids = receipts
            .into_iter()
            .map(|receipt| receipt.genesis_receipt_id)
            .collect::<Vec<_>>();
        let topology_receipt_id = identity_hash(&(
            assignment.assignment_id.as_str(),
            assignment.product_compilation_receipt_id.as_str(),
            topology_id.as_str(),
            &agent_genesis_receipt_ids,
        ))?;
        Ok(Self {
            topology_receipt_id,
            assignment_id: assignment.assignment_id.clone(),
            product_compilation_receipt_id: assignment.product_compilation_receipt_id.clone(),
            topology_id,
            agent_genesis_receipt_ids,
        })
    }

    pub fn verify_identity(&self) -> Result<(), TheoryRouterError> {
        if self.assignment_id.trim().is_empty()
            || self.product_compilation_receipt_id.trim().is_empty()
            || self.topology_id.trim().is_empty()
            || self.agent_genesis_receipt_ids.is_empty()
            || self
                .agent_genesis_receipt_ids
                .iter()
                .any(|receipt_id| receipt_id.trim().is_empty())
        {
            return Err(error(
                "product_topology_invalid",
                "Agent topology receipt is incomplete",
            ));
        }
        let expected = identity_hash(&(
            self.assignment_id.as_str(),
            self.product_compilation_receipt_id.as_str(),
            self.topology_id.as_str(),
            &self.agent_genesis_receipt_ids,
        ))?;
        if expected != self.topology_receipt_id {
            return Err(error(
                "product_topology_invalid",
                "Agent topology receipt identity differs",
            ));
        }
        Ok(())
    }
}

/// Current inert preparation selected for one product identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedProductHeadV1 {
    pub product_id: String,
    pub assignment_id: String,
    pub prepared_id: String,
    pub head_revision: u64,
}

/// Append-only declarations and compilations with one compare-and-swap head.
#[derive(Clone)]
pub struct PdsProductStore {
    db: Db,
    declarations: Tree,
    topologies: Tree,
    compilations: Tree,
    heads: Tree,
    assignments: Tree,
    topology_receipts: Tree,
    capability_preparations: Tree,
    prepared_closures: Tree,
    prepared_heads: Tree,
}

impl PdsProductStore {
    pub fn new(db: Db) -> Result<Self, TheoryRouterError> {
        Ok(Self {
            topologies: db
                .open_tree("pds_product_topologies_v1")
                .map_err(|e| error("product_storage", e.to_string()))?,
            declarations: db
                .open_tree(TREE_DECLARATIONS)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            compilations: db
                .open_tree(TREE_COMPILATIONS)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            heads: db
                .open_tree(TREE_HEADS)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            assignments: db
                .open_tree(TREE_ASSIGNMENTS)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            topology_receipts: db
                .open_tree(TREE_TOPOLOGY_RECEIPTS)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            capability_preparations: db
                .open_tree(TREE_CAPABILITY_PREPARATIONS)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            prepared_closures: db
                .open_tree(TREE_PREPARED_CLOSURES)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            prepared_heads: db
                .open_tree(TREE_PREPARED_HEADS)
                .map_err(|failure| error("product_storage", failure.to_string()))?,
            db,
        })
    }

    pub fn install_topology(
        &self,
        topology: &super::ProductTopologyV1,
    ) -> Result<super::TheoryRevisionRef, TheoryRouterError> {
        let reference = topology.revision_ref()?;
        put_immutable(&self.topologies, &reference.content_hash, topology)?;
        self.db
            .flush()
            .map_err(|e| error("product_storage", e.to_string()))?;
        Ok(reference)
    }

    pub fn topology(
        &self,
        reference: &super::TheoryRevisionRef,
    ) -> Result<Option<super::ProductTopologyV1>, TheoryRouterError> {
        let topology: Option<super::ProductTopologyV1> =
            get_immutable(&self.topologies, &reference.content_hash)?;
        if topology.as_ref().is_some_and(|value| {
            !value
                .revision_ref()
                .is_ok_and(|actual| &actual == reference)
        }) {
            return Err(error(
                "product_storage_corrupt",
                "installed topology differs from its exact revision",
            ));
        }
        Ok(topology)
    }

    pub fn install(
        &self,
        declaration: &ProductDeclarationV1,
        receipt: &ProductCompilationReceiptV1,
        expected_prior: Option<&ProductCompilationHeadV1>,
    ) -> Result<ProductCompilationHeadV1, TheoryRouterError> {
        declaration.verify_identity()?;
        receipt.verify_identity()?;
        if receipt.product_revision_id != declaration.product_revision_id {
            return Err(error(
                "product_compilation_invalid",
                "compilation receipt cites another product revision",
            ));
        }
        let current = self.head(&declaration.product_id)?;
        if current
            .as_ref()
            .map(|head| head.compilation_receipt_id.as_str())
            == Some(receipt.compilation_receipt_id.as_str())
        {
            let stored_compilation = self
                .compilation(&receipt.compilation_receipt_id)?
                .ok_or_else(|| {
                    error(
                        "product_storage_corrupt",
                        "current product head does not resolve its compilation",
                    )
                })?;
            stored_compilation.verify_identity()?;
            if self.declaration(&declaration.product_revision_id)?.as_ref() != Some(declaration)
                || stored_compilation.product_revision_id != declaration.product_revision_id
            {
                return Err(error(
                    "product_storage_corrupt",
                    "current product head does not resolve its immutable declaration and compilation",
                ));
            }
            return Ok(current.expect("checked as present"));
        }
        put_immutable(
            &self.declarations,
            &declaration.product_revision_id,
            declaration,
        )?;
        put_immutable(&self.compilations, &receipt.compilation_receipt_id, receipt)?;
        if current.as_ref() != expected_prior {
            return Err(error(
                "product_compilation_head_conflict",
                "product compilation head differs from expected prior",
            ));
        }
        let next = ProductCompilationHeadV1 {
            product_id: declaration.product_id.clone(),
            product_revision_id: declaration.product_revision_id.clone(),
            compilation_receipt_id: receipt.compilation_receipt_id.clone(),
            head_revision: expected_prior.map_or(1, |head| head.head_revision + 1),
        };
        let current_bytes = expected_prior
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|failure| error("product_storage", failure.to_string()))?;
        let next_bytes = serde_json::to_vec(&next)
            .map_err(|failure| error("product_storage", failure.to_string()))?;
        self.heads
            .compare_and_swap(
                declaration.product_id.as_bytes(),
                current_bytes.as_deref(),
                Some(next_bytes.as_slice()),
            )
            .map_err(|failure| error("product_storage", failure.to_string()))?
            .map_err(|_| {
                error(
                    "product_compilation_head_conflict",
                    "product compilation head changed concurrently",
                )
            })?;
        self.db
            .flush()
            .map_err(|failure| error("product_storage", failure.to_string()))?;
        Ok(next)
    }

    pub fn declaration(
        &self,
        product_revision_id: &str,
    ) -> Result<Option<ProductDeclarationV1>, TheoryRouterError> {
        let declaration: Option<ProductDeclarationV1> =
            get_immutable(&self.declarations, product_revision_id)?;
        if let Some(declaration) = &declaration {
            declaration.verify_identity()?;
        }
        Ok(declaration)
    }

    pub fn compilation(
        &self,
        compilation_receipt_id: &str,
    ) -> Result<Option<ProductCompilationReceiptV1>, TheoryRouterError> {
        let compilation: Option<ProductCompilationReceiptV1> =
            get_immutable(&self.compilations, compilation_receipt_id)?;
        if let Some(compilation) = &compilation {
            compilation.verify_identity()?;
        }
        Ok(compilation)
    }

    pub fn head(
        &self,
        product_id: &str,
    ) -> Result<Option<ProductCompilationHeadV1>, TheoryRouterError> {
        let Some(raw) = self
            .heads
            .get(product_id.as_bytes())
            .map_err(|failure| error("product_storage", failure.to_string()))?
        else {
            return Ok(None);
        };
        let head: ProductCompilationHeadV1 = serde_json::from_slice(&raw)
            .map_err(|failure| error("product_storage", failure.to_string()))?;
        if head.product_id != product_id
            || head.product_revision_id.trim().is_empty()
            || head.compilation_receipt_id.trim().is_empty()
            || head.head_revision == 0
        {
            return Err(error(
                "product_storage_corrupt",
                "product compilation head is incomplete or stored under another product",
            ));
        }
        Ok(Some(head))
    }

    pub fn install_prepared(
        &self,
        assignment: &StewardshipAssignmentV1,
        agent_store: &AgentStore,
        capability: &CapabilityPreparationReceiptV1,
        closure: &PreparedActivationClosureV1,
        expected_prior: Option<&PreparedProductHeadV1>,
    ) -> Result<PreparedProductHeadV1, TheoryRouterError> {
        assignment
            .verify_identity()
            .map_err(|failure| error("product_preparation_invalid", failure.to_string()))?;
        capability
            .verify_identity()
            .map_err(|failure| error("product_preparation_invalid", failure.to_string()))?;
        closure.verify_identity()?;
        let compilation = self
            .compilation(&assignment.product_compilation_receipt_id)?
            .ok_or_else(|| {
                error(
                    "product_preparation_invalid",
                    "assignment cites an absent product compilation receipt",
                )
            })?;
        let declaration = self
            .declaration(&assignment.product_revision_id)?
            .ok_or_else(|| {
                error(
                    "product_preparation_invalid",
                    "assignment cites an absent product declaration",
                )
            })?;
        let compilation_head = self.head(&declaration.product_id)?.ok_or_else(|| {
            error(
                "product_preparation_invalid",
                "product declaration has no current compilation head",
            )
        })?;
        if compilation.product_revision_id != assignment.product_revision_id
            || compilation_head.product_revision_id != declaration.product_revision_id
            || compilation_head.compilation_receipt_id != compilation.compilation_receipt_id
        {
            return Err(error(
                "product_preparation_invalid",
                "assignment does not cite the current product compilation",
            ));
        }
        let agent_receipts = agent_store
            .genesis_receipts_for_assignment(&assignment.assignment_id)
            .map_err(|failure| error("product_preparation_invalid", failure.to_string()))?;
        let topology =
            ProductAgentTopologyReceiptV1::new(assignment, &declaration, agent_receipts)?;
        validate_prepared_coherence(
            &declaration,
            &compilation,
            assignment,
            &topology,
            capability,
            closure,
        )?;
        if closure.expected_prior_prepared_id.as_deref()
            != expected_prior.map(|head| head.prepared_id.as_str())
            || expected_prior.is_some_and(|head| head.product_id != declaration.product_id)
        {
            return Err(error(
                "product_preparation_invalid",
                "prepared closure prior does not belong to this product",
            ));
        }
        put_immutable(&self.assignments, &assignment.assignment_id, assignment)?;
        put_immutable(
            &self.topology_receipts,
            &topology.topology_receipt_id,
            &topology,
        )?;
        put_immutable(
            &self.capability_preparations,
            &capability.preparation_receipt_id,
            capability,
        )?;
        put_immutable(&self.prepared_closures, &closure.prepared_id, closure)?;
        let product_id = declaration.product_id;
        let scope_id = assignment.scope_id(&product_id);
        let current = self.prepared_head(&product_id, &scope_id)?;
        if current.as_ref().map(|head| head.prepared_id.as_str())
            == Some(closure.prepared_id.as_str())
        {
            return Ok(current.expect("checked as present"));
        }
        if current.as_ref() != expected_prior {
            return Err(error(
                "product_prepared_head_conflict",
                "prepared product head differs from expected prior",
            ));
        }
        let next = PreparedProductHeadV1 {
            product_id: product_id.to_string(),
            assignment_id: assignment.assignment_id.clone(),
            prepared_id: closure.prepared_id.clone(),
            head_revision: expected_prior.map_or(1, |head| head.head_revision + 1),
        };
        let current_bytes = expected_prior
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|failure| error("product_storage", failure.to_string()))?;
        let next_bytes = serde_json::to_vec(&next)
            .map_err(|failure| error("product_storage", failure.to_string()))?;
        self.prepared_heads
            .compare_and_swap(
                scope_id.as_bytes(),
                current_bytes.as_deref(),
                Some(next_bytes.as_slice()),
            )
            .map_err(|failure| error("product_storage", failure.to_string()))?
            .map_err(|_| {
                error(
                    "product_prepared_head_conflict",
                    "prepared product head changed concurrently",
                )
            })?;
        self.db
            .flush()
            .map_err(|failure| error("product_storage", failure.to_string()))?;
        Ok(next)
    }

    pub fn prepared_head(
        &self,
        product_id: &str,
        scope_id: &str,
    ) -> Result<Option<PreparedProductHeadV1>, TheoryRouterError> {
        let head: Option<PreparedProductHeadV1> = get_immutable(&self.prepared_heads, scope_id)?;
        if head.is_none()
            && self
                .prepared_heads
                .contains_key(product_id)
                .map_err(|failure| error("product_storage", failure.to_string()))?
        {
            return Err(error("legacy_assignment_history_incompatible", "unscoped prepared history requires an explicit owner migration; refusing to reset assignment state"));
        }

        if let Some(head) = &head {
            if head.product_id != product_id
                || head.assignment_id.trim().is_empty()
                || head.prepared_id.trim().is_empty()
                || head.head_revision == 0
            {
                return Err(error(
                    "product_storage_corrupt",
                    "prepared product head is incomplete or stored under another product",
                ));
            }
            self.validate_stored_prepared_head(head)?;
            let assignment = self
                .assignment(&head.assignment_id)?
                .ok_or_else(|| error("product_storage_corrupt", "prepared assignment absent"))?;
            if assignment.scope_id(product_id) != scope_id {
                return Err(error(
                    "product_storage_corrupt",
                    "prepared head belongs to another assignment scope",
                ));
            }
        }
        Ok(head)
    }

    fn validate_stored_prepared_head(
        &self,
        head: &PreparedProductHeadV1,
    ) -> Result<(), TheoryRouterError> {
        let closure: PreparedActivationClosureV1 =
            get_immutable(&self.prepared_closures, &head.prepared_id)?.ok_or_else(|| {
                error(
                    "product_storage_corrupt",
                    "prepared product head cites a missing closure",
                )
            })?;
        closure.verify_identity()?;
        let assignment: StewardshipAssignmentV1 =
            get_immutable(&self.assignments, &head.assignment_id)?.ok_or_else(|| {
                error(
                    "product_storage_corrupt",
                    "prepared product head cites a missing assignment",
                )
            })?;
        assignment
            .verify_identity()
            .map_err(|failure| error("product_storage_corrupt", failure.to_string()))?;
        let declaration = self
            .declaration(&assignment.product_revision_id)?
            .ok_or_else(|| {
                error(
                    "product_storage_corrupt",
                    "prepared assignment cites a missing product declaration",
                )
            })?;
        let compilation = self
            .compilation(&assignment.product_compilation_receipt_id)?
            .ok_or_else(|| {
                error(
                    "product_storage_corrupt",
                    "prepared assignment cites a missing product compilation",
                )
            })?;
        if declaration.product_id != head.product_id {
            return Err(error(
                "product_storage_corrupt",
                "prepared declaration belongs to another product",
            ));
        }
        let topology: ProductAgentTopologyReceiptV1 =
            get_immutable(&self.topology_receipts, &closure.agent_topology_receipt_id)?
                .ok_or_else(|| {
                    error(
                        "product_storage_corrupt",
                        "prepared closure cites a missing Agent topology receipt",
                    )
                })?;
        topology.verify_identity()?;
        let capability: CapabilityPreparationReceiptV1 = get_immutable(
            &self.capability_preparations,
            &closure.capability_preparation_receipt_id,
        )?
        .ok_or_else(|| {
            error(
                "product_storage_corrupt",
                "prepared closure cites a missing Capability preparation receipt",
            )
        })?;
        capability
            .verify_identity()
            .map_err(|failure| error("product_storage_corrupt", failure.to_string()))?;
        validate_prepared_coherence(
            &declaration,
            &compilation,
            &assignment,
            &topology,
            &capability,
            &closure,
        )
    }

    pub fn prepared_closure(
        &self,
        prepared_id: &str,
    ) -> Result<Option<PreparedActivationClosureV1>, TheoryRouterError> {
        let closure: Option<PreparedActivationClosureV1> =
            get_immutable(&self.prepared_closures, prepared_id)?;
        if let Some(closure) = &closure {
            closure.verify_identity()?;
        }
        Ok(closure)
    }

    pub fn assignment(
        &self,
        assignment_id: &str,
    ) -> Result<Option<StewardshipAssignmentV1>, TheoryRouterError> {
        let assignment: Option<StewardshipAssignmentV1> =
            get_immutable(&self.assignments, assignment_id)?;
        if let Some(assignment) = &assignment {
            assignment
                .verify_identity()
                .map_err(|failure| error("product_storage_corrupt", failure.to_string()))?;
        }
        Ok(assignment)
    }

    pub fn topology_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<ProductAgentTopologyReceiptV1>, TheoryRouterError> {
        let receipt: Option<ProductAgentTopologyReceiptV1> =
            get_immutable(&self.topology_receipts, receipt_id)?;
        if let Some(receipt) = &receipt {
            receipt.verify_identity()?;
        }
        Ok(receipt)
    }

    pub fn capability_preparation(
        &self,
        receipt_id: &str,
    ) -> Result<Option<CapabilityPreparationReceiptV1>, TheoryRouterError> {
        let receipt: Option<CapabilityPreparationReceiptV1> =
            get_immutable(&self.capability_preparations, receipt_id)?;
        if let Some(receipt) = &receipt {
            receipt
                .verify_identity()
                .map_err(|failure| error("product_storage_corrupt", failure.to_string()))?;
        }
        Ok(receipt)
    }
}

/// Derives exact per-owner preparation references from one compilation.
pub fn product_owner_preparation_refs(
    compilation: &ProductCompilationReceiptV1,
) -> Result<Vec<crate::theory::activation::PreparedDomainActivationRef>, TheoryRouterError> {
    compilation.verify_identity()?;
    let mut by_owner = BTreeMap::<String, Vec<_>>::new();
    for component in &compilation.installed_owner_revisions {
        by_owner
            .entry(component.route.owner_domain.clone())
            .or_default()
            .push(component);
    }
    by_owner
        .into_iter()
        .map(|(owner_domain, components)| {
            serde_json::to_vec(&components)
                .map(
                    |bytes| crate::theory::activation::PreparedDomainActivationRef {
                        owner_domain,
                        receipt_ref: blake3::hash(&bytes).to_hex().to_string(),
                    },
                )
                .map_err(|failure| error("product_preparation_invalid", failure.to_string()))
        })
        .collect()
}

fn validate_prepared_coherence(
    declaration: &ProductDeclarationV1,
    compilation: &ProductCompilationReceiptV1,
    assignment: &StewardshipAssignmentV1,
    topology: &ProductAgentTopologyReceiptV1,
    capability: &CapabilityPreparationReceiptV1,
    closure: &PreparedActivationClosureV1,
) -> Result<(), TheoryRouterError> {
    let declared_positions = declaration
        .agent_topology
        .iter()
        .map(|position| position.position_id.as_str())
        .collect::<Vec<_>>();
    let assigned_positions = assignment
        .agent_positions
        .iter()
        .map(|position| position.position_id.as_str())
        .collect::<Vec<_>>();
    let selected_contracts = closure
        .activation
        .selected_implementations
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let capability_covers_selection = capability.selected_contracts == selected_contracts
        && capability.contribution_receipts.len()
            == closure.activation.selected_implementations.len()
        && closure.activation.selected_implementations.iter().all(
            |(contract_ref, implementation_ref)| {
                capability.contribution_receipts.iter().any(|receipt| {
                    &receipt.contract_ref == contract_ref
                        && &receipt.implementation_ref == implementation_ref
                })
            },
        );
    let capability_binding_refs = capability
        .binding_revision_refs
        .iter()
        .map(
            |(binding_id, revision_ref)| crate::theory::activation::OwnerBindingRevisionRef {
                binding_id: binding_id.clone(),
                revision_ref: revision_ref.clone(),
            },
        )
        .collect::<Vec<_>>();
    if compilation.product_revision_id != declaration.product_revision_id
        || assignment.product_revision_id != declaration.product_revision_id
        || assignment.product_compilation_receipt_id != compilation.compilation_receipt_id
        || assignment.principal_id != declaration.principal_id
        || assignment.requested_authority_ref != declaration.requested_authority_ref
        || assignment.principal_grant_ref != declaration.principal_grant_ref
        || declared_positions != assigned_positions
        || topology.assignment_id != assignment.assignment_id
        || topology.product_compilation_receipt_id != compilation.compilation_receipt_id
        || topology.topology_id != assignment.topology_id
        || capability.assignment_id != assignment.assignment_id
        || capability.activation_id != closure.activation.activation_id
        || !capability_covers_selection
        || closure.assignment != *assignment
        || closure.activation.assignment_id != assignment.assignment_id
        || closure.product_compilation_receipt_id != compilation.compilation_receipt_id
        || closure.product_revision_id != declaration.product_revision_id
        || closure.agent_topology_receipt_id != topology.topology_receipt_id
        || closure.capability_preparation_receipt_id != capability.preparation_receipt_id
        || closure.participant_plan != declaration.participant_plan
        || closure.owner_receipts != product_owner_preparation_refs(compilation)?
        || closure.binding_revision_refs != capability_binding_refs
        || closure.effective_authority_inputs.requested_authority_ref
            != assignment.requested_authority_ref
        || closure.effective_authority_inputs.principal_grant_ref != assignment.principal_grant_ref
    {
        return Err(error(
            "product_preparation_invalid",
            "prepared records do not form one coherent current product aggregate",
        ));
    }
    Ok(())
}

fn put_immutable<T: Serialize>(tree: &Tree, key: &str, value: &T) -> Result<(), TheoryRouterError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|failure| error("product_storage", failure.to_string()))?;
    match tree
        .compare_and_swap(
            key.as_bytes(),
            None as Option<&[u8]>,
            Some(bytes.as_slice()),
        )
        .map_err(|failure| error("product_storage", failure.to_string()))?
    {
        Ok(()) => Ok(()),
        Err(conflict) if conflict.current.as_deref() == Some(bytes.as_slice()) => Ok(()),
        Err(_) => Err(error(
            "product_storage_conflict",
            "immutable product record differs",
        )),
    }
}

fn get_immutable<T: serde::de::DeserializeOwned>(
    tree: &Tree,
    key: &str,
) -> Result<Option<T>, TheoryRouterError> {
    tree.get(key.as_bytes())
        .map_err(|failure| error("product_storage", failure.to_string()))?
        .map(|raw| {
            serde_json::from_slice(&raw)
                .map_err(|failure| error("product_storage", failure.to_string()))
        })
        .transpose()
}

fn identity_hash(value: &impl Serialize) -> Result<String, TheoryRouterError> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| error("product_identity_failed", failure.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};
    use meld_execution::capability::CapabilityContractRevisionRef;
    use meld_lang::CapabilityRef;
    use meld_world_model::agent::{
        AgentGenesis, AgentGenesisIntentV1, AgentSubscriptionRequestV1, SeedAgentRegistration,
    };
    use meld_world_model::belief::{
        BeliefFamilyConfig, BeliefFamilyRegistry, BeliefFamilyRegistryStore, BeliefKey,
        BeliefStore, BeliefSubscriptionAuthority, BranchScope, TheoryRevisionRef as AgentTheoryRef,
    };
    use meld_world_model::world_state::graph::PerspectiveKey;

    use super::*;
    use crate::capability::{
        ExactCapabilityActivationRequest, OwnerBindingView, ProductCapabilityInventory,
    };
    use crate::config::{
        AdapterPlacement, AssignedAgentPositionV1, OperationalLimits, RuntimeIsolationRequirements,
        StewardshipActivationV1,
    };
    use crate::theory::activation::{
        ActivationParticipantSpec, EffectiveAuthorityInputRefs, ParticipantKind,
    };
    use crate::theory::{TheoryRevisionRef, TheoryRouteId};

    struct PreparedCandidate {
        declaration: ProductDeclarationV1,
        compilation: ProductCompilationReceiptV1,
        assignment: StewardshipAssignmentV1,
        activation: StewardshipActivationV1,
        topology: ProductAgentTopologyReceiptV1,
        capability: CapabilityPreparationReceiptV1,
        closure: PreparedActivationClosureV1,
        agent_store: AgentStore,
    }

    fn participant_plan() -> ActivationParticipantPlanV1 {
        ActivationParticipantPlanV1::new(vec![ActivationParticipantSpec {
            participant_id: "steward".to_string(),
            owner_domain: "agent".to_string(),
            kind: ParticipantKind::BoundedActor,
            required: true,
            depends_on: BTreeSet::new(),
            readiness_contract_ref: "readiness.v1".to_string(),
            wake_contract_ref: "wake.v1".to_string(),
            safe_point_contract_ref: "safe-point.v1".to_string(),
            stop_contract_ref: "stop.v1".to_string(),
        }])
        .unwrap()
    }

    fn product_records(
        product_id: &str,
        belief_revision: AgentTheoryRef,
    ) -> (ProductDeclarationV1, ProductCompilationReceiptV1) {
        let package_id = format!("package-{product_id}");
        let component_id = format!("belief-{product_id}");
        let route = TheoryRouteId::new("belief", "belief-family", 1);
        let component = InstalledTheoryComponentRef {
            component_id: component_id.clone(),
            route: route.clone(),
            component_schema_version: 1,
            source_content_hash: format!("source-{product_id}"),
            owner_revision: TheoryRevisionRef {
                registry: belief_revision.registry,
                id: belief_revision.id,
                content_hash: belief_revision.content_hash,
            },
        };
        let package = PdsPackageInstallationReceiptV1::new(
            package_id.clone(),
            "1".to_string(),
            format!("package-hash-{product_id}"),
            1,
            Vec::new(),
            vec![component],
            1,
        )
        .unwrap();
        let declaration = ProductDeclarationV1::new(
            product_id.to_string(),
            format!("principal-{product_id}"),
            vec![ProductPackageSelectionV1 {
                package_id,
                package_version: "1".to_string(),
                package_content_hash: format!("package-hash-{product_id}"),
            }],
            vec![ProductAgentPositionV1 {
                package_id: None,
                position_id: "steward".to_string(),
                directive: format!("steward {product_id}"),
                required_owner_routes: vec![route],
                observation_scope_component_id: component_id.clone(),
                required_subscriptions: vec![ProductAgentSubscriptionV1 {
                    source_owner: "belief".to_string(),
                    source_contract_component_id: component_id,
                    initial_cursor_policy: "from_genesis".to_string(),
                }],
                participant_ref: "steward".to_string(),
            }],
            participant_plan(),
            format!("authority-{product_id}"),
            format!("grant-{product_id}"),
            "compilation-policy.v1".to_string(),
            BTreeMap::new(),
        )
        .unwrap();
        let package_store =
            PdsPackageStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        package_store.install_receipt(&package).unwrap();
        let compilation =
            ProductCompilationReceiptV1::compile(&declaration, vec![package], &package_store, 1)
                .unwrap();
        (declaration, compilation)
    }

    #[test]
    fn positions_select_exact_wads_without_renaming_local_components() {
        let (base, baseline) = product_records(
            "selection",
            AgentTheoryRef {
                registry: "belief_family".into(),
                id: "family".into(),
                content_hash: "revision".into(),
            },
        );
        let store =
            PdsPackageStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let mut packages = Vec::new();
        let mut positions = Vec::new();
        for name in ["first", "second", "third"] {
            let mut components = baseline.installed_owner_revisions.clone();
            components[0].owner_revision.id = name.into();
            let package = PdsPackageInstallationReceiptV1::new(
                name.into(),
                "1".into(),
                format!("hash-{name}"),
                1,
                vec![],
                components,
                1,
            )
            .unwrap();
            store.install_receipt(&package).unwrap();
            let mut position = base.agent_topology[0].clone();
            position.position_id = name.into();
            position.package_id = Some(name.into());
            positions.push(position);
            packages.push(package);
        }
        let declare = |positions| {
            ProductDeclarationV1::new(
                base.product_id.clone(),
                base.principal_id.clone(),
                packages
                    .iter()
                    .map(|p| ProductPackageSelectionV1 {
                        package_id: p.package_id.clone(),
                        package_version: p.package_version.clone(),
                        package_content_hash: p.package_content_hash.clone(),
                    })
                    .collect(),
                positions,
                base.participant_plan.clone(),
                base.requested_authority_ref.clone(),
                base.principal_grant_ref.clone(),
                base.compilation_policy_revision.clone(),
                BTreeMap::new(),
            )
            .unwrap()
        };
        let declaration = declare(positions.clone());
        let compiled =
            ProductCompilationReceiptV1::compile(&declaration, packages.clone(), &store, 1)
                .unwrap();
        for position in &positions {
            let selected = compiled.position_packages(position, &store).unwrap();
            assert_eq!(selected.len(), 1);
            assert_eq!(
                selected[0].components[0].owner_revision.id,
                position.position_id
            );
        }
        positions[0].package_id = None;
        assert!(ProductCompilationReceiptV1::compile(
            &declare(positions.clone()),
            packages.clone(),
            &store,
            1
        )
        .is_err());
        positions[0].package_id = Some("absent".into());
        assert!(
            ProductCompilationReceiptV1::compile(&declare(positions), packages, &store, 1).is_err()
        );
        let historical = &base.agent_topology[0];
        let encoded = serde_json::to_value(historical).unwrap();
        assert!(encoded.get("package_id").is_none());
        assert_eq!(
            serde_json::from_value::<ProductAgentPositionV1>(encoded).unwrap(),
            *historical
        );
    }

    #[test]
    fn compilation_includes_shared_imported_components_once() {
        let (declaration, baseline) = product_records(
            "imports",
            AgentTheoryRef {
                registry: "belief_family".into(),
                id: "family".into(),
                content_hash: "revision".into(),
            },
        );
        let store =
            PdsPackageStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let selected = &declaration.selected_packages[0];
        let leaf = PdsPackageInstallationReceiptV1::new(
            selected.package_id.clone(),
            selected.package_version.clone(),
            selected.package_content_hash.clone(),
            1,
            Vec::new(),
            baseline.installed_owner_revisions.clone(),
            1,
        )
        .unwrap();
        store.install_receipt(&leaf).unwrap();
        let import = |package: &PdsPackageInstallationReceiptV1| {
            crate::theory::InstalledExactPackageImport {
                package_id: package.package_id.clone(),
                receipt_id: package.receipt_id.clone(),
            }
        };
        let middle = PdsPackageInstallationReceiptV1::new(
            "middle".into(),
            "1".into(),
            "middle-hash".into(),
            1,
            vec![import(&leaf)],
            Vec::new(),
            1,
        )
        .unwrap();
        store.install_receipt(&middle).unwrap();
        let root = PdsPackageInstallationReceiptV1::new(
            "root".into(),
            "1".into(),
            "root-hash".into(),
            1,
            vec![import(&leaf), import(&middle)],
            Vec::new(),
            1,
        )
        .unwrap();
        store.install_receipt(&root).unwrap();
        let imported = ProductDeclarationV1::new(
            declaration.product_id,
            declaration.principal_id,
            vec![ProductPackageSelectionV1 {
                package_id: root.package_id.clone(),
                package_version: root.package_version.clone(),
                package_content_hash: root.package_content_hash.clone(),
            }],
            declaration.agent_topology,
            declaration.participant_plan,
            declaration.requested_authority_ref,
            declaration.principal_grant_ref,
            declaration.compilation_policy_revision,
            declaration.participant_bindings,
        )
        .unwrap();
        let compiled =
            ProductCompilationReceiptV1::compile(&imported, vec![root], &store, 1).unwrap();
        assert_eq!(compiled.package_receipt_ids.len(), 3);
        assert_eq!(
            compiled.installed_owner_revisions,
            baseline.installed_owner_revisions
        );
        assert_ne!(
            compiled.compilation_receipt_id,
            baseline.compilation_receipt_id
        );
    }

    fn agent_receipt(
        store: &AgentStore,
        belief_store: &BeliefStore,
        belief_revision: &meld_world_model::belief::BeliefFamilyRevision,
        assignment: &StewardshipAssignmentV1,
        compilation_receipt_id: &str,
        product_id: &str,
    ) -> AgentGenesisReceiptV1 {
        let agent_id = assignment.agent_positions[0].agent_id.clone();
        let revision = belief_revision.revision_ref();
        let subject = DomainObjectRef::new("workspace_fs", "node", product_id).unwrap();
        let perspective = PerspectiveKey::new("default", "default").unwrap();
        let request = AgentSubscriptionRequestV1::new(
            agent_id.clone(),
            "belief".to_string(),
            revision.clone(),
            BeliefKey {
                subject: subject.clone(),
                dimension_id: revision.id.clone(),
                predicate_id: belief_revision.config.predicate_id.clone(),
                perspective: perspective.clone(),
                branch_scope: BranchScope::main(),
                evidence_policy_id: belief_revision.config.evidence_policy_id.clone(),
            },
            "from_genesis".to_string(),
        )
        .unwrap();
        let intent = AgentGenesisIntentV1::new(
            assignment.assignment_id.clone(),
            compilation_receipt_id.to_string(),
            "steward".to_string(),
            vec![revision],
            SeedAgentRegistration {
                agent_id,
                perspective_key: perspective,
                subject,
                branch_scope: BranchScope::main(),
                observation_scope: product_id.to_string(),
                directive: format!("steward {product_id}"),
                seed_provenance: "product coherence test".to_string(),
                curation_rule: None,
                curation_rule_revision: None,
                maintained_condition: None,
                maintained_condition_revision: None,
                created_at_seq: 1,
            },
            vec![request.clone()],
        )
        .unwrap();
        let authority = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let append = authority.append_capability();
        let genesis = AgentGenesis::new(store, &append);
        let pending = genesis.prepare(intent).unwrap();
        let acceptance = BeliefSubscriptionAuthority::new(belief_store)
            .accept(&request, belief_revision)
            .unwrap();
        genesis
            .complete(pending, vec![acceptance], "product-test")
            .unwrap()
            .0
    }

    fn candidate(store: &PdsProductStore, product_id: &str) -> PreparedCandidate {
        let world_db = sled::Config::new().temporary(true).open().unwrap();
        let mut families = BeliefFamilyRegistryStore::new(world_db.clone()).unwrap();
        let mut family: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/belief_family.docs_freshness.json"
        ))
        .unwrap();
        family.family_id = format!("belief-{product_id}");
        family.dimension_id = format!("belief-{product_id}");
        let belief_revision = families.install(family, 1).unwrap().1;
        let (declaration, compilation) =
            product_records(product_id, belief_revision.revision_ref());
        store.install(&declaration, &compilation, None).unwrap();
        let assignment = StewardshipAssignmentV1::new(
            compilation.compilation_receipt_id.clone(),
            declaration.product_revision_id.clone(),
            declaration.principal_id.clone(),
            DomainObjectRef::new("workspace_fs", "node", product_id).unwrap(),
            "default::default".to_string(),
            "main".to_string(),
            product_topology_id(&declaration.agent_topology).unwrap(),
            vec![AssignedAgentPositionV1 {
                position_id: "steward".to_string(),
                agent_id: format!("agent-{product_id}"),
            }],
            declaration.requested_authority_ref.clone(),
            declaration.principal_grant_ref.clone(),
        )
        .unwrap();
        let agent_store = AgentStore::new(world_db.clone()).unwrap();
        let belief_store = BeliefStore::new(world_db).unwrap();
        let receipt = agent_receipt(
            &agent_store,
            &belief_store,
            &belief_revision,
            &assignment,
            &compilation.compilation_receipt_id,
            product_id,
        );
        let topology =
            ProductAgentTopologyReceiptV1::new(&assignment, &declaration, vec![receipt]).unwrap();
        let activation = StewardshipActivationV1::new(
            assignment.assignment_id.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            AdapterPlacement::InProcess,
            RuntimeIsolationRequirements::default(),
            OperationalLimits::default(),
        )
        .unwrap();
        let capability = ProductCapabilityInventory::assemble(Vec::new())
            .unwrap()
            .prepare(
                ExactCapabilityActivationRequest {
                    assignment_id: assignment.assignment_id.clone(),
                    activation_id: activation.activation_id.clone(),
                    selected_contracts: Vec::new(),
                    selected_implementations: BTreeMap::new(),
                    compatibility_policy_revision: "capability-compatibility.v1".to_string(),
                },
                &OwnerBindingView::default(),
            )
            .unwrap()
            .preparation_receipt;
        let closure = PreparedActivationClosureV1::new(
            assignment.clone(),
            activation.clone(),
            topology.topology_receipt_id.clone(),
            product_owner_preparation_refs(&compilation).unwrap(),
            capability.preparation_receipt_id.clone(),
            declaration.participant_plan.clone(),
            Vec::new(),
            EffectiveAuthorityInputRefs {
                requested_authority_ref: assignment.requested_authority_ref.clone(),
                principal_grant_ref: assignment.principal_grant_ref.clone(),
                current_judgment_ref: "judgment.v1".to_string(),
            },
            None,
        )
        .unwrap();
        PreparedCandidate {
            declaration,
            compilation,
            assignment,
            activation,
            topology,
            capability,
            closure,
            agent_store,
        }
    }

    #[test]
    fn topology_rejects_agent_receipt_from_another_compilation() {
        let store =
            PdsProductStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let prepared = candidate(&store, "product-a");
        let foreign_db = sled::Config::new().temporary(true).open().unwrap();
        let foreign_agent_store = AgentStore::new(foreign_db.clone()).unwrap();
        let foreign_belief_store = BeliefStore::new(foreign_db.clone()).unwrap();
        let mut families = BeliefFamilyRegistryStore::new(foreign_db).unwrap();
        let mut family: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/belief_family.docs_freshness.json"
        ))
        .unwrap();
        family.family_id = "belief-product-a".to_string();
        family.dimension_id = "belief-product-a".to_string();
        let belief_revision = families.install(family, 1).unwrap().1;
        let receipt = agent_receipt(
            &foreign_agent_store,
            &foreign_belief_store,
            &belief_revision,
            &prepared.assignment,
            "another-compilation",
            "product-a",
        );
        assert!(ProductAgentTopologyReceiptV1::new(
            &prepared.assignment,
            &prepared.declaration,
            vec![receipt],
        )
        .is_err());
    }

    #[test]
    fn prepared_head_cannot_alias_another_product() {
        let store =
            PdsProductStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let product_b = candidate(&store, "product-b");
        let head_b = store
            .install_prepared(
                &product_b.assignment,
                &product_b.agent_store,
                &product_b.capability,
                &product_b.closure,
                None,
            )
            .unwrap();
        let product_a = candidate(&store, "product-a");
        let closure = PreparedActivationClosureV1::new(
            product_a.assignment.clone(),
            product_a.activation.clone(),
            product_a.topology.topology_receipt_id.clone(),
            product_owner_preparation_refs(&product_a.compilation).unwrap(),
            product_a.capability.preparation_receipt_id.clone(),
            product_a.declaration.participant_plan.clone(),
            Vec::new(),
            EffectiveAuthorityInputRefs {
                requested_authority_ref: product_a.assignment.requested_authority_ref.clone(),
                principal_grant_ref: product_a.assignment.principal_grant_ref.clone(),
                current_judgment_ref: "judgment.v1".to_string(),
            },
            Some(head_b.prepared_id.clone()),
        )
        .unwrap();
        assert!(store
            .install_prepared(
                &product_a.assignment,
                &product_a.agent_store,
                &product_a.capability,
                &closure,
                Some(&head_b),
            )
            .is_err());
        assert!(store
            .prepared_head("product-a", &product_a.assignment.scope_id("product-a"))
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .prepared_head("product-b", &product_b.assignment.scope_id("product-b"))
                .unwrap(),
            Some(head_b)
        );
    }

    #[test]
    fn capability_receipt_must_cover_exact_activation_selections() {
        let store =
            PdsProductStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let prepared = candidate(&store, "product-a");
        let exact = CapabilityContractRevisionRef {
            selector: CapabilityRef {
                capability_type_id: "different".to_string(),
                capability_version: 1,
            },
            content_identity: "different-revision".to_string(),
        };
        let activation = StewardshipActivationV1::new(
            prepared.assignment.assignment_id.clone(),
            BTreeMap::new(),
            BTreeMap::from([(exact, "different-implementation".to_string())]),
            AdapterPlacement::InProcess,
            RuntimeIsolationRequirements::default(),
            OperationalLimits::default(),
        )
        .unwrap();
        let closure = PreparedActivationClosureV1::new(
            prepared.assignment.clone(),
            activation,
            prepared.topology.topology_receipt_id.clone(),
            product_owner_preparation_refs(&prepared.compilation).unwrap(),
            prepared.capability.preparation_receipt_id.clone(),
            prepared.declaration.participant_plan.clone(),
            Vec::new(),
            EffectiveAuthorityInputRefs {
                requested_authority_ref: prepared.assignment.requested_authority_ref.clone(),
                principal_grant_ref: prepared.assignment.principal_grant_ref.clone(),
                current_judgment_ref: "judgment.v1".to_string(),
            },
            None,
        )
        .unwrap();
        assert!(store
            .install_prepared(
                &prepared.assignment,
                &prepared.agent_store,
                &prepared.capability,
                &closure,
                None,
            )
            .is_err());
    }
}
