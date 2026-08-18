//! Durable startup-only PDS activation publication.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::{Db, Tree};
use thiserror::Error;

use crate::capability::{CapabilityCatalog, CapabilityExecutorRegistry};
use crate::theory::activation::PreparedActivationClosureV1;

const TREE_ASSIGNMENTS: &str = "pds_assignments_v1";
const TREE_ACTIVATIONS: &str = "pds_activations_v1";
const TREE_PREPARED: &str = "pds_prepared_activations_v1";
const TREE_GENERATIONS: &str = "pds_activation_generations_v1";
const TREE_HEADS: &str = "pds_assignment_heads_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupGenerationStatus {
    Starting,
    Ready,
    Current,
    FailedBeforeCurrent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerReadinessReceiptRef {
    pub participant_id: String,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationGenerationV1 {
    pub generation_id: String,
    pub generation_number: u64,
    pub assignment_id: String,
    pub activation_id: String,
    pub prepared_id: String,
    pub expected_prior_generation: Option<String>,
    pub status: StartupGenerationStatus,
    pub readiness_receipts: Vec<OwnerReadinessReceiptRef>,
}

#[derive(Serialize)]
struct GenerationIdentity<'a> {
    assignment_id: &'a str,
    activation_id: &'a str,
    prepared_id: &'a str,
    generation_number: u64,
}

impl ActivationGenerationV1 {
    pub fn startup(prepared: &PreparedActivationClosureV1) -> Result<Self, ActivationError> {
        let identity = GenerationIdentity {
            assignment_id: &prepared.assignment.assignment_id,
            activation_id: &prepared.activation.activation_id,
            prepared_id: &prepared.prepared_id,
            generation_number: 1,
        };
        Ok(Self {
            generation_id: hash(&identity)?,
            generation_number: 1,
            assignment_id: prepared.assignment.assignment_id.clone(),
            activation_id: prepared.activation.activation_id.clone(),
            prepared_id: prepared.prepared_id.clone(),
            expected_prior_generation: None,
            status: StartupGenerationStatus::Starting,
            readiness_receipts: vec![],
        })
    }
}

pub struct CurrentAssignmentRuntime {
    pub assignment_id: String,
    pub generation_id: String,
    pub package_receipt_id: String,
    pub agent_runtime_ref: String,
    pub belief_scope_ref: String,
    pub capability_catalog: Arc<CapabilityCatalog>,
    pub executor_registry: Arc<CapabilityExecutorRegistry>,
    pub execution_scope_ref: String,
    pub admission_ref: String,
}

#[derive(Debug, Error)]
pub enum ActivationError {
    #[error("activation_invalid: {0}")]
    Invalid(String),
    #[error("activation_conflict: {0}")]
    Conflict(String),
    #[error("activation_storage: {0}")]
    Storage(String),
}

#[derive(Clone)]
pub struct StartupActivationStore {
    db: Db,
    assignments: Tree,
    activations: Tree,
    prepared: Tree,
    generations: Tree,
    heads: Tree,
}

impl StartupActivationStore {
    pub fn new(db: Db) -> Result<Self, ActivationError> {
        Ok(Self {
            assignments: db.open_tree(TREE_ASSIGNMENTS).map_err(storage)?,
            activations: db.open_tree(TREE_ACTIVATIONS).map_err(storage)?,
            prepared: db.open_tree(TREE_PREPARED).map_err(storage)?,
            generations: db.open_tree(TREE_GENERATIONS).map_err(storage)?,
            heads: db.open_tree(TREE_HEADS).map_err(storage)?,
            db,
        })
    }

    pub fn publish_startup(
        &self,
        prepared: PreparedActivationClosureV1,
        mut readiness: Vec<OwnerReadinessReceiptRef>,
        capability_catalog: Arc<CapabilityCatalog>,
        executor_registry: Arc<CapabilityExecutorRegistry>,
    ) -> Result<CurrentAssignmentRuntime, ActivationError> {
        prepared
            .assignment
            .verify_identity()
            .map_err(|failure| ActivationError::Invalid(failure.to_string()))?;
        prepared
            .activation
            .verify_identity()
            .map_err(|failure| ActivationError::Invalid(failure.to_string()))?;
        let required: BTreeSet<_> = prepared
            .participant_plan
            .participants
            .iter()
            .filter(|participant| participant.required)
            .map(|participant| participant.participant_id.as_str())
            .collect();
        readiness.sort_by(|left, right| left.participant_id.cmp(&right.participant_id));
        let ready: BTreeSet<_> = readiness
            .iter()
            .map(|receipt| receipt.participant_id.as_str())
            .collect();
        if !required.is_subset(&ready)
            || readiness
                .iter()
                .any(|receipt| receipt.receipt_ref.trim().is_empty())
        {
            return Err(ActivationError::Invalid(
                "required participant readiness is incomplete".to_string(),
            ));
        }
        put_immutable(
            &self.assignments,
            &prepared.assignment.assignment_id,
            &prepared.assignment,
        )?;
        put_immutable(
            &self.activations,
            &prepared.activation.activation_id,
            &prepared.activation,
        )?;
        put_immutable(&self.prepared, &prepared.prepared_id, &prepared)?;

        let mut generation = ActivationGenerationV1::startup(&prepared)?;
        put_immutable(
            &self.generations,
            &format!("{}:starting", generation.generation_id),
            &generation,
        )?;
        generation.status = StartupGenerationStatus::Ready;
        generation.readiness_receipts = readiness;
        put_immutable(
            &self.generations,
            &format!("{}:ready", generation.generation_id),
            &generation,
        )?;
        let head_result = self
            .heads
            .compare_and_swap(
                prepared.assignment.assignment_id.as_bytes(),
                None as Option<&[u8]>,
                Some(generation.generation_id.as_bytes()),
            )
            .map_err(storage)?;
        if let Err(conflict) = head_result {
            let current = conflict
                .current
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string());
            if current.as_deref() != Some(generation.generation_id.as_str()) {
                return Err(ActivationError::Conflict(
                    "assignment head was not absent".to_string(),
                ));
            }
        }
        generation.status = StartupGenerationStatus::Current;
        put_immutable(
            &self.generations,
            &format!("{}:current", generation.generation_id),
            &generation,
        )?;
        self.db.flush().map_err(storage)?;
        Ok(CurrentAssignmentRuntime {
            assignment_id: prepared.assignment.assignment_id,
            generation_id: generation.generation_id.clone(),
            package_receipt_id: prepared.package_receipt_id,
            agent_runtime_ref: format!("agent:{}", prepared.assignment.agent_id),
            belief_scope_ref: format!("belief:{}", generation.generation_id),
            capability_catalog,
            executor_registry,
            execution_scope_ref: format!("execution:{}", generation.generation_id),
            admission_ref: format!("admission:{}:open", generation.generation_id),
        })
    }

    pub fn current_generation(
        &self,
        assignment_id: &str,
    ) -> Result<Option<ActivationGenerationV1>, ActivationError> {
        let Some(generation_id) = self.heads.get(assignment_id.as_bytes()).map_err(storage)? else {
            return Ok(None);
        };
        get(
            &self.generations,
            &format!("{}:current", String::from_utf8_lossy(&generation_id)),
        )
    }
}

fn put_immutable<T: Serialize>(tree: &Tree, key: &str, value: &T) -> Result<(), ActivationError> {
    let bytes = bincode::serialize(value)
        .map_err(|failure| ActivationError::Storage(failure.to_string()))?;
    if let Some(existing) = tree.get(key.as_bytes()).map_err(storage)? {
        if existing.as_ref() != bytes.as_slice() {
            return Err(ActivationError::Conflict(format!(
                "immutable record '{key}' differs"
            )));
        }
    } else {
        tree.insert(key.as_bytes(), bytes).map_err(storage)?;
    }
    Ok(())
}

fn get<T: DeserializeOwned>(tree: &Tree, key: &str) -> Result<Option<T>, ActivationError> {
    tree.get(key.as_bytes())
        .map_err(storage)?
        .map(|bytes| {
            bincode::deserialize(&bytes)
                .map_err(|failure| ActivationError::Storage(failure.to_string()))
        })
        .transpose()
}

fn hash(value: &impl Serialize) -> Result<String, ActivationError> {
    bincode::serialize(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| ActivationError::Storage(failure.to_string()))
}

fn storage(failure: sled::Error) -> ActivationError {
    ActivationError::Storage(failure.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AdapterPlacement, OperationalLimits, RuntimeIsolationRequirements};
    use crate::config::{StewardshipActivationV1, StewardshipAssignmentV1};
    use crate::theory::activation::{
        ActivationParticipantPlanV1, ActivationParticipantSpec, EffectiveAuthorityInputRefs,
        ParticipantKind, PreparedDomainActivationRef,
    };
    use meld_events::DomainObjectRef;
    use std::collections::BTreeMap;

    fn prepared(id: &str) -> PreparedActivationClosureV1 {
        let assignment = StewardshipAssignmentV1::new(
            "package".into(),
            "principal".into(),
            id.into(),
            DomainObjectRef::new("workspace_fs", "node", id).unwrap(),
            "perspective".into(),
            "main".into(),
            "authority".into(),
            "grant".into(),
        )
        .unwrap();
        let activation = StewardshipActivationV1::new(
            assignment.assignment_id.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            AdapterPlacement::InProcess,
            RuntimeIsolationRequirements::default(),
            OperationalLimits::default(),
        )
        .unwrap();
        let plan = ActivationParticipantPlanV1::new(vec![ActivationParticipantSpec {
            participant_id: id.into(),
            owner_domain: "docs".into(),
            kind: ParticipantKind::BoundedActor,
            required: true,
            depends_on: BTreeSet::new(),
            readiness_contract_ref: "ready".into(),
            wake_contract_ref: "wake".into(),
            safe_point_contract_ref: "safe".into(),
            stop_contract_ref: "stop".into(),
        }])
        .unwrap();
        PreparedActivationClosureV1::new(
            assignment,
            activation,
            "content".into(),
            vec![PreparedDomainActivationRef {
                owner_domain: "docs".into(),
                receipt_ref: "owner".into(),
            }],
            "capability".into(),
            plan,
            vec![],
            EffectiveAuthorityInputRefs {
                requested_authority_ref: "authority".into(),
                principal_grant_ref: "grant".into(),
                current_judgment_ref: "judgment".into(),
            },
        )
        .unwrap()
    }

    fn publish(
        store: &StartupActivationStore,
        prepared: PreparedActivationClosureV1,
    ) -> Result<CurrentAssignmentRuntime, ActivationError> {
        let participant = prepared.participant_plan.participants[0]
            .participant_id
            .clone();
        store.publish_startup(
            prepared,
            vec![OwnerReadinessReceiptRef {
                participant_id: participant,
                receipt_ref: "ready".into(),
            }],
            Arc::new(CapabilityCatalog::new()),
            Arc::new(CapabilityExecutorRegistry::new()),
        )
    }

    #[test]
    fn startup_publishes_only_after_required_readiness() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = StartupActivationStore::new(db).unwrap();
        let prepared = prepared("docs");
        let assignment_id = prepared.assignment.assignment_id.clone();
        assert!(store
            .publish_startup(
                prepared,
                vec![],
                Arc::new(CapabilityCatalog::new()),
                Arc::new(CapabilityExecutorRegistry::new())
            )
            .is_err());
        assert!(store.current_generation(&assignment_id).unwrap().is_none());
    }

    #[test]
    fn current_head_uses_expected_absent_fence() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = StartupActivationStore::new(db).unwrap();
        let first = prepared("docs");
        let assignment_id = first.assignment.assignment_id.clone();
        publish(&store, first.clone()).unwrap();
        publish(&store, first).unwrap();
        assert_eq!(
            store
                .current_generation(&assignment_id)
                .unwrap()
                .unwrap()
                .status,
            StartupGenerationStatus::Current
        );
    }

    #[test]
    fn overlapping_assignments_have_distinct_runtime_views() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = StartupActivationStore::new(db).unwrap();
        let left = publish(&store, prepared("left")).unwrap();
        let right = publish(&store, prepared("right")).unwrap();
        assert_ne!(left.assignment_id, right.assignment_id);
        assert_ne!(left.generation_id, right.generation_id);
        assert!(!Arc::ptr_eq(
            &left.executor_registry,
            &right.executor_registry
        ));
    }

    #[test]
    fn assignments_and_current_heads_survive_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path().join("activation.sled")).unwrap();
        let store = StartupActivationStore::new(db.clone()).unwrap();
        let prepared = prepared("docs");
        let assignment_id = prepared.assignment.assignment_id.clone();
        let generation_id = publish(&store, prepared).unwrap().generation_id;
        drop(store);
        drop(db);
        let reopened =
            StartupActivationStore::new(sled::open(temp.path().join("activation.sled")).unwrap())
                .unwrap();
        assert_eq!(
            reopened
                .current_generation(&assignment_id)
                .unwrap()
                .unwrap()
                .generation_id,
            generation_id
        );
    }
}
