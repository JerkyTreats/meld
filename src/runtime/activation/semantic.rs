//! Versioned post-bootstrap runtime selection.

use std::collections::BTreeSet;

use meld_execution::activation::ExecutionActivationValidationReceipt;
use meld_world_model::activation::AgentBootstrapReceipt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::packages::RuntimeActivationInput;

/// Schema for the first recurring docs freshness runtime selection.
pub const SEMANTIC_RUNTIME_SELECTION_SCHEMA_VERSION: u32 = 1;

const BOOTSTRAP_RUNTIME_ID: &str = "world_model.agent.bootstrap.docs_freshness";
const DOCS_FRESHNESS_SELECTION_ID: &str = "docs_freshness.semantic.v1";
const GRAPH_REPLAY_RUNTIME_ID: &str = "world_model.graph_replay";
const SELECTION_HASH_DOMAIN: &[u8] = b"meld.semantic-runtime-selection.v1";

const DOCS_FRESHNESS_ACTORS: &[&str] = &[
    "execution.planning",
    "execution.publication",
    "world_model.agent_goal_curation",
    "world_model.agent_hydration",
    "world_model.belief_assessment",
    "world_model.evidence_ingestion",
    "world_model.graph_replay",
    "world_model.planner_projection",
    "world_model.satisfaction_curation",
];

const DOCS_FRESHNESS_SERVICES: &[&str] = &["execution.task_network_command"];

/// Immutable selection for recurring product runtimes after bootstrap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticRuntimeSelection {
    /// Version of this derived runtime-selection contract.
    pub schema_version: u32,
    /// Stable logical identity for this post-bootstrap runtime family.
    pub selection_id: String,
    /// Logical activation whose immutable products this selection consumes.
    pub source_activation_id: String,
    /// Canonical Wave 2 activation hash retained without reinterpretation.
    pub source_activation_hash: String,
    /// Durable world-model bootstrap receipt accepted for this selection.
    pub world_model_bootstrap_receipt_id: String,
    /// Canonical hash of the accepted world-model activation input.
    pub world_model_input_hash: String,
    /// Execution validation receipt accepted for this selection.
    pub execution_receipt_id: String,
    /// Canonical hash of the accepted execution activation input.
    pub execution_input_hash: String,
    /// Configured task network hosted by the execution authority.
    pub task_network_id: String,
    /// Canonically sorted concrete actor ids that own bounded ticks.
    pub actor_runtime_ids: Vec<String>,
    /// Canonically sorted passive services hosted without worker ticks.
    pub service_runtime_ids: Vec<String>,
    /// Deterministic identity of every preceding field.
    pub selection_hash: String,
}

impl SemanticRuntimeSelection {
    /// Derive schema one recurring selection from accepted bootstrap products.
    pub fn docs_freshness_v1(
        bootstrap: &RuntimeActivationInput,
        world_model: &AgentBootstrapReceipt,
        execution: &ExecutionActivationValidationReceipt,
    ) -> Result<Self, SemanticRuntimeSelectionError> {
        validate_bootstrap_selection(bootstrap)?;
        if bootstrap.activation_id != execution.activation_id
            || bootstrap.activation_hash != execution.activation_hash
            || bootstrap.activation_id != world_model.activation_id
            || bootstrap.activation_hash != world_model.activation_hash
            || bootstrap.bootstrap_runtime_id != world_model.bootstrap_id
            || world_model.belief.activation_id != bootstrap.activation_id
            || world_model.belief.activation_hash != bootstrap.activation_hash
        {
            return Err(SemanticRuntimeSelectionError::SourceIdentityMismatch);
        }
        if world_model.receipt_id.trim().is_empty()
            || world_model.input_hash.trim().is_empty()
            || execution.receipt_id.trim().is_empty()
            || execution.input_hash.trim().is_empty()
        {
            return Err(SemanticRuntimeSelectionError::InvalidReceiptIdentity);
        }
        if execution.task_network_id.trim().is_empty() {
            return Err(SemanticRuntimeSelectionError::InvalidTaskNetworkId);
        }

        let actor_runtime_ids = DOCS_FRESHNESS_ACTORS
            .iter()
            .map(|runtime_id| (*runtime_id).to_string())
            .collect();
        let service_runtime_ids = DOCS_FRESHNESS_SERVICES
            .iter()
            .map(|runtime_id| (*runtime_id).to_string())
            .collect();
        let mut selection = Self {
            schema_version: SEMANTIC_RUNTIME_SELECTION_SCHEMA_VERSION,
            selection_id: DOCS_FRESHNESS_SELECTION_ID.to_string(),
            source_activation_id: bootstrap.activation_id.clone(),
            source_activation_hash: bootstrap.activation_hash.clone(),
            world_model_bootstrap_receipt_id: world_model.receipt_id.clone(),
            world_model_input_hash: world_model.input_hash.clone(),
            execution_receipt_id: execution.receipt_id.clone(),
            execution_input_hash: execution.input_hash.clone(),
            task_network_id: execution.task_network_id.clone(),
            actor_runtime_ids,
            service_runtime_ids,
            selection_hash: String::new(),
        };
        selection.selection_hash = selection.derive_hash()?;
        selection.validate()?;
        Ok(selection)
    }

    /// Validate version, role separation, canonical ordering, and identity.
    pub fn validate(&self) -> Result<(), SemanticRuntimeSelectionError> {
        if self.schema_version != SEMANTIC_RUNTIME_SELECTION_SCHEMA_VERSION {
            return Err(SemanticRuntimeSelectionError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.source_activation_id.trim().is_empty()
            || self.source_activation_hash.trim().is_empty()
        {
            return Err(SemanticRuntimeSelectionError::InvalidSourceIdentity);
        }
        if self.world_model_bootstrap_receipt_id.trim().is_empty()
            || self.world_model_input_hash.trim().is_empty()
            || self.execution_receipt_id.trim().is_empty()
            || self.execution_input_hash.trim().is_empty()
        {
            return Err(SemanticRuntimeSelectionError::InvalidReceiptIdentity);
        }
        if self.task_network_id.trim().is_empty() {
            return Err(SemanticRuntimeSelectionError::InvalidTaskNetworkId);
        }
        if self.selection_id != DOCS_FRESHNESS_SELECTION_ID {
            return Err(SemanticRuntimeSelectionError::UnsupportedSelectionId);
        }
        validate_runtime_ids(&self.actor_runtime_ids)?;
        validate_runtime_ids(&self.service_runtime_ids)?;
        let actors = self.actor_runtime_ids.iter().collect::<BTreeSet<_>>();
        let services = self.service_runtime_ids.iter().collect::<BTreeSet<_>>();
        if actors.intersection(&services).next().is_some() {
            return Err(SemanticRuntimeSelectionError::RoleOverlap);
        }
        if self.actor_runtime_ids
            != DOCS_FRESHNESS_ACTORS
                .iter()
                .map(|runtime_id| (*runtime_id).to_string())
                .collect::<Vec<_>>()
            || self.service_runtime_ids
                != DOCS_FRESHNESS_SERVICES
                    .iter()
                    .map(|runtime_id| (*runtime_id).to_string())
                    .collect::<Vec<_>>()
        {
            return Err(SemanticRuntimeSelectionError::UnsupportedRuntimeSet);
        }
        if self.selection_hash != self.derive_hash()? {
            return Err(SemanticRuntimeSelectionError::SelectionHashMismatch);
        }
        Ok(())
    }

    fn derive_hash(&self) -> Result<String, SemanticRuntimeSelectionError> {
        #[derive(Serialize)]
        struct HashProjection<'a> {
            schema_version: u32,
            selection_id: &'a str,
            source_activation_id: &'a str,
            source_activation_hash: &'a str,
            world_model_bootstrap_receipt_id: &'a str,
            world_model_input_hash: &'a str,
            execution_receipt_id: &'a str,
            execution_input_hash: &'a str,
            task_network_id: &'a str,
            actor_runtime_ids: &'a [String],
            service_runtime_ids: &'a [String],
        }

        let projection = HashProjection {
            schema_version: self.schema_version,
            selection_id: &self.selection_id,
            source_activation_id: &self.source_activation_id,
            source_activation_hash: &self.source_activation_hash,
            world_model_bootstrap_receipt_id: &self.world_model_bootstrap_receipt_id,
            world_model_input_hash: &self.world_model_input_hash,
            execution_receipt_id: &self.execution_receipt_id,
            execution_input_hash: &self.execution_input_hash,
            task_network_id: &self.task_network_id,
            actor_runtime_ids: &self.actor_runtime_ids,
            service_runtime_ids: &self.service_runtime_ids,
        };
        let encoded = serde_json::to_vec(&projection)
            .map_err(|error| SemanticRuntimeSelectionError::Hash(error.to_string()))?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(SELECTION_HASH_DOMAIN);
        hasher.update(&encoded);
        Ok(hasher.finalize().to_hex().to_string())
    }
}

/// Rejected post-bootstrap runtime selection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SemanticRuntimeSelectionError {
    /// Schema one bootstrap inputs were widened or otherwise changed.
    #[error("semantic runtime selection requires the exact schema one bootstrap runtime set")]
    InvalidBootstrapRuntimeSet,
    /// Root and execution products do not describe one activation.
    #[error("semantic runtime selection source identities disagree")]
    SourceIdentityMismatch,
    /// Source activation identity is missing.
    #[error("semantic runtime selection source identity must be non-empty")]
    InvalidSourceIdentity,
    /// Durable owner receipt identity or input hash is missing.
    #[error("semantic runtime selection owner receipts must be non-empty")]
    InvalidReceiptIdentity,
    /// Configured task-network identity is missing.
    #[error("semantic runtime selection task network id must be non-empty")]
    InvalidTaskNetworkId,
    /// Logical selection id is not the supported docs freshness family.
    #[error("semantic runtime selection id is unsupported")]
    UnsupportedSelectionId,
    /// Runtime ids are empty, duplicated, unsorted, or structurally invalid.
    #[error("semantic runtime selection runtime ids are not canonical")]
    InvalidRuntimeIds,
    /// One role was classified as both an actor and a service.
    #[error("semantic runtime selection actor and service ids overlap")]
    RoleOverlap,
    /// The first version does not recognize this exact runtime set.
    #[error("semantic runtime selection contains an unsupported runtime set")]
    UnsupportedRuntimeSet,
    /// Persisted selection identity does not match the selected fields.
    #[error("semantic runtime selection hash does not match its fields")]
    SelectionHashMismatch,
    /// Selection schema is newer or older than this implementation.
    #[error("unsupported semantic runtime selection schema version {0}")]
    UnsupportedSchemaVersion(u32),
    /// Selection identity could not be encoded.
    #[error("semantic runtime selection hashing failed: {0}")]
    Hash(String),
}

fn validate_bootstrap_selection(
    bootstrap: &RuntimeActivationInput,
) -> Result<(), SemanticRuntimeSelectionError> {
    let mut runtime_ids = bootstrap
        .enabled_runtime_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    runtime_ids.sort_unstable();
    if bootstrap.bootstrap_runtime_id != BOOTSTRAP_RUNTIME_ID
        || runtime_ids != [BOOTSTRAP_RUNTIME_ID, GRAPH_REPLAY_RUNTIME_ID]
    {
        return Err(SemanticRuntimeSelectionError::InvalidBootstrapRuntimeSet);
    }
    Ok(())
}

fn validate_runtime_ids(runtime_ids: &[String]) -> Result<(), SemanticRuntimeSelectionError> {
    let mut canonical = runtime_ids.to_vec();
    canonical.sort();
    canonical.dedup();
    if canonical != runtime_ids
        || runtime_ids.is_empty()
        || runtime_ids.iter().any(|runtime_id| {
            crate::runtime::supervisor::contracts::validate_runtime_id(runtime_id).is_err()
        })
    {
        return Err(SemanticRuntimeSelectionError::InvalidRuntimeIds);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_world_model::activation::BeliefActivationReceipt;

    fn bootstrap() -> RuntimeActivationInput {
        RuntimeActivationInput {
            activation_id: "docs_freshness".to_string(),
            activation_hash: "activation-hash".to_string(),
            bootstrap_runtime_id: BOOTSTRAP_RUNTIME_ID.to_string(),
            enabled_runtime_ids: vec![
                BOOTSTRAP_RUNTIME_ID.to_string(),
                GRAPH_REPLAY_RUNTIME_ID.to_string(),
            ],
        }
    }

    fn execution() -> ExecutionActivationValidationReceipt {
        ExecutionActivationValidationReceipt {
            receipt_id: "receipt".to_string(),
            activation_hash: "activation-hash".to_string(),
            activation_id: "docs_freshness".to_string(),
            input_hash: "input".to_string(),
            method_binding_id: "binding".to_string(),
            method_binding_digest: "binding-digest".to_string(),
            method_id: "refresh_docs_v1".to_string(),
            method_library_digest: "library".to_string(),
            task_package_id: "docs_writer".to_string(),
            task_package_digest: "package".to_string(),
            task_network_id: "docs-freshness".to_string(),
            task_network_storage_key: "docs-freshness".to_string(),
            task_network_identity_digest: "network".to_string(),
            artifact_contract_digest: "artifact".to_string(),
            workspace_scan_contract_digest: "scan".to_string(),
            execution_coordinates_digest: "coordinates".to_string(),
            publication_mapping_id: "publication".to_string(),
            publication_mapping_digest: "publication-digest".to_string(),
        }
    }

    fn world_model() -> AgentBootstrapReceipt {
        AgentBootstrapReceipt {
            receipt_id: "world-model-receipt".to_string(),
            bootstrap_id: BOOTSTRAP_RUNTIME_ID.to_string(),
            activation_hash: "activation-hash".to_string(),
            activation_id: "docs_freshness".to_string(),
            input_hash: "world-model-input".to_string(),
            belief: BeliefActivationReceipt {
                family_id: "docs_freshness".to_string(),
                config_snapshot_hash: "belief-config".to_string(),
                activation_hash: "activation-hash".to_string(),
                activation_id: "docs_freshness".to_string(),
            },
            directive_id: "directive.docs_freshness".to_string(),
            agent_id: "seed.docs_freshness".to_string(),
            rule_id: "rule.docs_freshness".to_string(),
            subscription_id: "subscription.docs_freshness".to_string(),
            completed_at_seq: 12,
        }
    }

    #[test]
    fn post_bootstrap_selection_is_versioned_deterministic_and_role_separated() {
        let first =
            SemanticRuntimeSelection::docs_freshness_v1(&bootstrap(), &world_model(), &execution())
                .unwrap();
        let second =
            SemanticRuntimeSelection::docs_freshness_v1(&bootstrap(), &world_model(), &execution())
                .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.schema_version, 1);
        assert!(!first
            .actor_runtime_ids
            .contains(&BOOTSTRAP_RUNTIME_ID.to_string()));
        assert_eq!(first.service_runtime_ids, DOCS_FRESHNESS_SERVICES);
        assert!(first.validate().is_ok());
    }

    #[test]
    fn schema_one_bootstrap_set_cannot_be_silently_widened() {
        let mut widened = bootstrap();
        widened
            .enabled_runtime_ids
            .push("world_model.belief_assessment".to_string());

        assert!(matches!(
            SemanticRuntimeSelection::docs_freshness_v1(&widened, &world_model(), &execution(),),
            Err(SemanticRuntimeSelectionError::InvalidBootstrapRuntimeSet)
        ));

        widened.enabled_runtime_ids = vec![
            BOOTSTRAP_RUNTIME_ID.to_string(),
            GRAPH_REPLAY_RUNTIME_ID.to_string(),
            GRAPH_REPLAY_RUNTIME_ID.to_string(),
        ];
        assert!(matches!(
            SemanticRuntimeSelection::docs_freshness_v1(&widened, &world_model(), &execution(),),
            Err(SemanticRuntimeSelectionError::InvalidBootstrapRuntimeSet)
        ));
    }

    #[test]
    fn source_identity_and_runtime_set_are_bound_by_selection_hash() {
        let selection =
            SemanticRuntimeSelection::docs_freshness_v1(&bootstrap(), &world_model(), &execution())
                .unwrap();
        let mut changed_source = selection.clone();
        changed_source.source_activation_hash = "different".to_string();
        assert!(matches!(
            changed_source.validate(),
            Err(SemanticRuntimeSelectionError::SelectionHashMismatch)
        ));

        let mut changed_roles = selection;
        changed_roles.actor_runtime_ids.pop();
        assert!(matches!(
            changed_roles.validate(),
            Err(SemanticRuntimeSelectionError::UnsupportedRuntimeSet)
        ));
    }

    #[test]
    fn selection_rejects_mismatched_or_unbound_owner_receipts() {
        let mut mismatched_world_model = world_model();
        mismatched_world_model.bootstrap_id = "different-bootstrap".to_string();
        assert!(matches!(
            SemanticRuntimeSelection::docs_freshness_v1(
                &bootstrap(),
                &mismatched_world_model,
                &execution(),
            ),
            Err(SemanticRuntimeSelectionError::SourceIdentityMismatch)
        ));

        let mut missing_execution_identity = execution();
        missing_execution_identity.input_hash.clear();
        assert!(matches!(
            SemanticRuntimeSelection::docs_freshness_v1(
                &bootstrap(),
                &world_model(),
                &missing_execution_identity,
            ),
            Err(SemanticRuntimeSelectionError::InvalidReceiptIdentity)
        ));

        let selection =
            SemanticRuntimeSelection::docs_freshness_v1(&bootstrap(), &world_model(), &execution())
                .unwrap();
        let mut changed_receipt = selection;
        changed_receipt.world_model_input_hash = "different-input".to_string();
        assert!(matches!(
            changed_receipt.validate(),
            Err(SemanticRuntimeSelectionError::SelectionHashMismatch)
        ));
    }
}
