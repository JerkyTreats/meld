//! Portable assignment-local lifecycle beneath one stable service role.

use meld_execution::capability::CapabilityContractRevisionRef;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::{Db, Tree};
use std::collections::BTreeSet;
use thiserror::Error;

pub const STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID: &str = "runtime.pds_activation_lifecycle";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerPrepareRequest {
    pub generation_id: String,
    pub participant_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerPrepareReceipt {
    pub receipt_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerReadinessRequest {
    pub generation_id: String,
    pub incarnation_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerReadinessReceipt {
    pub receipt_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerSafePointRequest {
    pub generation_id: String,
    pub incarnation_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerSafePointReceipt {
    pub receipt_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStopRequest {
    pub generation_id: String,
    pub incarnation_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStopReceipt {
    pub receipt_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOwnerWait {
    pub wait_ref: String,
    pub resolved: bool,
}
pub trait OwnerPreparationPort: Send + Sync {
    fn prepare(&self, request: OwnerPrepareRequest) -> Result<OwnerPrepareReceipt, LifecycleError>;
}
pub trait OwnerReadinessPort: Send + Sync {
    fn readiness(
        &self,
        request: OwnerReadinessRequest,
    ) -> Result<OwnerReadinessReceipt, LifecycleError>;
}
pub trait OwnerWakePort: Send + Sync {
    fn resolve_wait(&self, wait_ref: &str) -> Result<ResolvedOwnerWait, LifecycleError>;
}
pub trait OwnerSafePointPort: Send + Sync {
    fn safe_point(
        &self,
        request: OwnerSafePointRequest,
    ) -> Result<OwnerSafePointReceipt, LifecycleError>;
}
pub trait OwnerStopPort: Send + Sync {
    fn stop(&self, request: OwnerStopRequest) -> Result<OwnerStopReceipt, LifecycleError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Activate,
    Replace,
    Deactivate,
    Recover,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationLifecycleIntentV1 {
    pub intent_id: String,
    pub request_key: String,
    pub assignment_id: String,
    pub desired_activation_id: String,
    pub expected_prior_generation: Option<String>,
    pub action: LifecycleAction,
}
impl ActivationLifecycleIntentV1 {
    pub fn new(
        request_key: String,
        assignment_id: String,
        desired_activation_id: String,
        expected_prior_generation: Option<String>,
        action: LifecycleAction,
    ) -> Result<Self, LifecycleError> {
        if [&request_key, &assignment_id, &desired_activation_id]
            .iter()
            .any(|v| v.trim().is_empty())
        {
            return Err(LifecycleError::Invalid(
                "lifecycle intent identity is incomplete".into(),
            ));
        }
        let intent_id = hash(&(
            &request_key,
            &assignment_id,
            &desired_activation_id,
            &expected_prior_generation,
            &action,
        ))?;
        Ok(Self {
            intent_id,
            request_key,
            assignment_id,
            desired_activation_id,
            expected_prior_generation,
            action,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantIncarnationStatus {
    Starting,
    Ready,
    Active,
    Stopping,
    Stopped,
    Interrupted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantIncarnationV1 {
    pub incarnation_id: String,
    pub generation_id: String,
    pub participant_id: String,
    pub adapter_implementation_ref: String,
    pub lease_ref: String,
    pub incarnation_number: u64,
    pub status: ParticipantIncarnationStatus,
}
impl ParticipantIncarnationV1 {
    pub fn new(
        generation_id: String,
        participant_id: String,
        adapter_implementation_ref: String,
        lease_ref: String,
        incarnation_number: u64,
    ) -> Result<Self, LifecycleError> {
        let incarnation_id = hash(&(
            &generation_id,
            &participant_id,
            &adapter_implementation_ref,
            incarnation_number,
        ))?;
        Ok(Self {
            incarnation_id,
            generation_id,
            participant_id,
            adapter_implementation_ref,
            lease_ref,
            incarnation_number,
            status: ParticipantIncarnationStatus::Starting,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableOperationStatus {
    Pending,
    Dispatched,
    Ambiguous,
    Retryable,
    Completed,
    RejectedLate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableOperationV1 {
    pub operation_key: String,
    pub assignment_id: String,
    pub generation_id: String,
    pub capability_contract_ref: CapabilityContractRevisionRef,
    pub semantic_request_hash: String,
    pub status: DurableOperationStatus,
    pub admitted_product_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationAttemptStatus {
    Claimed,
    Dispatched,
    Ambiguous,
    Completed,
    ProvenNoEffect,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationAttemptV1 {
    pub attempt_id: String,
    pub operation_key: String,
    pub incarnation_id: String,
    pub dispatch_claim_ref: String,
    pub adapter_dispatch_ref: Option<String>,
    pub attempt_number: u64,
    pub status: OperationAttemptStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralWakeRef {
    EventPosition(String),
    OwnerRevision(String),
    DurableDeadline(String),
    PassiveSubscription(String),
    OperatorAction(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerWaitReceiptV1 {
    pub wait_ref: String,
    pub generation_id: String,
    pub incarnation_id: String,
    pub owner_checkpoint_ref: String,
    pub reason_code: String,
    pub wake_refs: Vec<StructuralWakeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedLifecycleState {
    ActiveIdle,
    Quiescent,
    Stalled,
    Current,
    AdmissionClosed,
    Interrupted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentLifecycleProjectionV1 {
    pub assignment_id: String,
    pub generation_id: Option<String>,
    pub state: ProjectedLifecycleState,
    pub participant_statuses: Vec<String>,
    pub unresolved_operation_count: u64,
    pub waiting: Vec<String>,
    pub broken_wake_refs: Vec<StructuralWakeRef>,
    pub last_transition_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FencedQuiescenceReceiptV1 {
    pub receipt_id: String,
    pub assignment_id: String,
    pub generation_id: String,
    pub admission_fence_ref: String,
    pub owner_safe_point_refs: Vec<String>,
    pub unresolved_operation_summary_ref: String,
    pub passive_subscription_fence_refs: Vec<String>,
    pub participant_drain_refs: Vec<String>,
}
impl FencedQuiescenceReceiptV1 {
    pub fn new(
        assignment_id: String,
        generation_id: String,
        admission_fence_ref: String,
        mut owner_safe_point_refs: Vec<String>,
        unresolved_operation_summary_ref: String,
        mut passive_subscription_fence_refs: Vec<String>,
        mut participant_drain_refs: Vec<String>,
    ) -> Result<Self, LifecycleError> {
        owner_safe_point_refs.sort();
        passive_subscription_fence_refs.sort();
        participant_drain_refs.sort();
        let receipt_id = hash(&(
            &assignment_id,
            &generation_id,
            &admission_fence_ref,
            &owner_safe_point_refs,
            &unresolved_operation_summary_ref,
            &passive_subscription_fence_refs,
            &participant_drain_refs,
        ))?;
        Ok(Self {
            receipt_id,
            assignment_id,
            generation_id,
            admission_fence_ref,
            owner_safe_point_refs,
            unresolved_operation_summary_ref,
            passive_subscription_fence_refs,
            participant_drain_refs,
        })
    }
}

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("lifecycle_invalid: {0}")]
    Invalid(String),
    #[error("lifecycle_conflict: {0}")]
    Conflict(String),
    #[error("lifecycle_storage: {0}")]
    Storage(String),
}

#[derive(Clone)]
pub struct ActivationLifecycleStore {
    db: Db,
    intents: Tree,
    intent_keys: Tree,
    intent_phases: Tree,
    incarnations: Tree,
    operations: Tree,
    attempts: Tree,
    admission: Tree,
    heads: Tree,
    quiescence: Tree,
}
impl ActivationLifecycleStore {
    pub fn new(db: Db) -> Result<Self, LifecycleError> {
        Ok(Self {
            intents: db.open_tree("pds_activation_intents_v1").map_err(storage)?,
            intent_keys: db
                .open_tree("pds_activation_intent_keys_v1")
                .map_err(storage)?,
            intent_phases: db
                .open_tree("pds_activation_intent_phases_v1")
                .map_err(storage)?,
            incarnations: db
                .open_tree("pds_participant_incarnations_v1")
                .map_err(storage)?,
            operations: db.open_tree("pds_durable_operations_v1").map_err(storage)?,
            attempts: db.open_tree("pds_operation_attempts_v1").map_err(storage)?,
            admission: db.open_tree("pds_admission_fences_v1").map_err(storage)?,
            heads: db.open_tree("pds_assignment_heads_v1").map_err(storage)?,
            quiescence: db
                .open_tree("pds_quiescence_receipts_v1")
                .map_err(storage)?,
            db,
        })
    }
    pub fn submit_intent(
        &self,
        intent: ActivationLifecycleIntentV1,
    ) -> Result<ActivationLifecycleIntentV1, LifecycleError> {
        if let Some(existing_id) = self
            .intent_keys
            .get(intent.request_key.as_bytes())
            .map_err(storage)?
        {
            let existing: ActivationLifecycleIntentV1 =
                get_required(&self.intents, &String::from_utf8_lossy(&existing_id))?;
            if existing != intent {
                return Err(LifecycleError::Conflict(
                    "request key already names another lifecycle intent".into(),
                ));
            }
            return Ok(existing);
        }
        put_immutable(&self.intents, &intent.intent_id, &intent)?;
        self.intent_keys
            .insert(intent.request_key.as_bytes(), intent.intent_id.as_bytes())
            .map_err(storage)?;
        self.db.flush().map_err(storage)?;
        Ok(intent)
    }
    pub fn publish_head(
        &self,
        assignment_id: &str,
        expected: Option<&str>,
        generation_id: &str,
    ) -> Result<(), LifecycleError> {
        let result = self
            .heads
            .compare_and_swap(
                assignment_id.as_bytes(),
                expected.map(str::as_bytes),
                Some(generation_id.as_bytes()),
            )
            .map_err(storage)?;
        if result.is_err() {
            return Err(LifecycleError::Conflict(
                "assignment generation fence changed".into(),
            ));
        }
        self.admission
            .insert(generation_id.as_bytes(), b"open".as_slice())
            .map_err(storage)?;
        self.db.flush().map_err(storage)?;
        Ok(())
    }
    pub fn replace_head(
        &self,
        assignment_id: &str,
        old_generation_id: &str,
        new_generation_id: &str,
    ) -> Result<(), LifecycleError> {
        self.close_admission(old_generation_id)?;
        let result = self
            .heads
            .compare_and_swap(
                assignment_id.as_bytes(),
                Some(old_generation_id.as_bytes()),
                Some(new_generation_id.as_bytes()),
            )
            .map_err(storage)?;
        if result.is_err() {
            return Err(LifecycleError::Conflict(
                "replacement generation fence changed".into(),
            ));
        }
        self.admission
            .insert(new_generation_id.as_bytes(), b"open".as_slice())
            .map_err(storage)?;
        self.db.flush().map_err(storage)?;
        Ok(())
    }
    pub fn close_admission(&self, generation_id: &str) -> Result<String, LifecycleError> {
        let fence = hash(&(generation_id, "closed"))?;
        self.admission
            .insert(generation_id.as_bytes(), fence.as_bytes())
            .map_err(storage)?;
        self.db.flush().map_err(storage)?;
        Ok(fence)
    }
    pub fn admission_open(&self, generation_id: &str) -> Result<bool, LifecycleError> {
        Ok(self
            .admission
            .get(generation_id.as_bytes())
            .map_err(storage)?
            .as_deref()
            == Some(b"open"))
    }
    pub fn persist_operation(&self, operation: DurableOperationV1) -> Result<(), LifecycleError> {
        if operation.operation_key.trim().is_empty()
            || operation.semantic_request_hash.trim().is_empty()
        {
            return Err(LifecycleError::Invalid(
                "operation identity is incomplete".into(),
            ));
        }
        put_immutable(&self.operations, &operation.operation_key, &operation)?;
        self.db.flush().map_err(storage)?;
        Ok(())
    }
    pub fn begin_attempt(
        &self,
        operation_key: &str,
        incarnation_id: String,
        claim: String,
        attempt_number: u64,
    ) -> Result<OperationAttemptV1, LifecycleError> {
        let mut operation: DurableOperationV1 = get_required(&self.operations, operation_key)?;
        if !matches!(
            operation.status,
            DurableOperationStatus::Pending | DurableOperationStatus::Retryable
        ) {
            return Err(LifecycleError::Conflict(
                "operation is not dispatchable".into(),
            ));
        }
        let attempt_id = hash(&(operation_key, &incarnation_id, attempt_number))?;
        let attempt = OperationAttemptV1 {
            attempt_id: attempt_id.clone(),
            operation_key: operation_key.into(),
            incarnation_id,
            dispatch_claim_ref: claim,
            adapter_dispatch_ref: None,
            attempt_number,
            status: OperationAttemptStatus::Claimed,
        };
        put_immutable(&self.attempts, &attempt_id, &attempt)?;
        operation.status = DurableOperationStatus::Dispatched;
        put_replace(&self.operations, operation_key, &operation)?;
        self.db.flush().map_err(storage)?;
        Ok(attempt)
    }
    pub fn mark_ambiguous(
        &self,
        operation_key: &str,
        attempt_id: &str,
    ) -> Result<(), LifecycleError> {
        let mut operation: DurableOperationV1 = get_required(&self.operations, operation_key)?;
        let mut attempt: OperationAttemptV1 = get_required(&self.attempts, attempt_id)?;
        attempt.status = OperationAttemptStatus::Ambiguous;
        operation.status = DurableOperationStatus::Ambiguous;
        put_replace(&self.attempts, attempt_id, &attempt)?;
        put_replace(&self.operations, operation_key, &operation)?;
        Ok(())
    }
    pub fn reconcile_no_effect(
        &self,
        operation_key: &str,
        attempt_id: &str,
    ) -> Result<(), LifecycleError> {
        let mut operation: DurableOperationV1 = get_required(&self.operations, operation_key)?;
        let mut attempt: OperationAttemptV1 = get_required(&self.attempts, attempt_id)?;
        if !matches!(attempt.status, OperationAttemptStatus::Ambiguous) {
            return Err(LifecycleError::Conflict("attempt is not ambiguous".into()));
        }
        attempt.status = OperationAttemptStatus::ProvenNoEffect;
        operation.status = DurableOperationStatus::Retryable;
        put_replace(&self.attempts, attempt_id, &attempt)?;
        put_replace(&self.operations, operation_key, &operation)?;
        Ok(())
    }
    pub fn complete_operation(
        &self,
        operation_key: &str,
        product_ref: String,
        current_generation_id: &str,
        late: crate::runtime::delivery::LateResultDisposition,
    ) -> Result<bool, LifecycleError> {
        let mut operation: DurableOperationV1 = get_required(&self.operations, operation_key)?;
        if operation.generation_id != current_generation_id
            || !self.admission_open(&operation.generation_id)?
        {
            match late {
                crate::runtime::delivery::LateResultDisposition::Reject => {
                    operation.status = DurableOperationStatus::RejectedLate;
                    put_replace(&self.operations, operation_key, &operation)?;
                    return Err(LifecycleError::Conflict(
                        "late operation result rejected".into(),
                    ));
                }
                crate::runtime::delivery::LateResultDisposition::HistoricalOnly => {
                    operation.status = DurableOperationStatus::RejectedLate;
                    operation.admitted_product_refs.push(product_ref);
                    put_replace(&self.operations, operation_key, &operation)?;
                    return Ok(false);
                }
            }
        }
        operation.status = DurableOperationStatus::Completed;
        operation.admitted_product_refs.push(product_ref);
        put_replace(&self.operations, operation_key, &operation)?;
        Ok(true)
    }
    pub fn create_incarnation(
        &self,
        generation_id: String,
        participant_id: String,
        implementation: String,
        lease: String,
        number: u64,
    ) -> Result<ParticipantIncarnationV1, LifecycleError> {
        let item = ParticipantIncarnationV1::new(
            generation_id,
            participant_id,
            implementation,
            lease,
            number,
        )?;
        put_immutable(&self.incarnations, &item.incarnation_id, &item)?;
        Ok(item)
    }
    pub fn recover_incarnation(
        &self,
        generation_id: String,
        participant_id: String,
        implementation: String,
        lease: String,
        number: u64,
    ) -> Result<ParticipantIncarnationV1, LifecycleError> {
        self.close_admission(&generation_id)?;
        self.create_incarnation(generation_id, participant_id, implementation, lease, number)
    }
    pub fn project_liveness(
        &self,
        assignment_id: String,
        generation_id: String,
        required_participants: &BTreeSet<String>,
        waits: Vec<OwnerWaitReceiptV1>,
        resolvable_wakes: &BTreeSet<StructuralWakeRef>,
        unresolved_operation_count: u64,
    ) -> AssignmentLifecycleProjectionV1 {
        let waiting_participants: BTreeSet<_> = waits
            .iter()
            .map(|wait| wait.incarnation_id.clone())
            .collect();
        let mut broken = Vec::new();
        for wait in &waits {
            for wake in &wait.wake_refs {
                if !resolvable_wakes.contains(wake) {
                    broken.push(wake.clone())
                }
            }
        }
        broken.sort();
        broken.dedup();
        let represented = required_participants.is_subset(&waiting_participants);
        let state = if !represented || !broken.is_empty() {
            ProjectedLifecycleState::Stalled
        } else {
            ProjectedLifecycleState::Quiescent
        };
        AssignmentLifecycleProjectionV1 {
            assignment_id,
            generation_id: Some(generation_id),
            state,
            participant_statuses: waiting_participants.into_iter().collect(),
            unresolved_operation_count,
            waiting: waits.into_iter().map(|wait| wait.wait_ref).collect(),
            broken_wake_refs: broken,
            last_transition_ref: "bounded-liveness-projection".into(),
        }
    }
    pub fn commit_quiescence(
        &self,
        receipt: FencedQuiescenceReceiptV1,
    ) -> Result<(), LifecycleError> {
        if self.admission_open(&receipt.generation_id)? {
            return Err(LifecycleError::Conflict(
                "admission must close before quiescence".into(),
            ));
        }
        put_immutable(&self.quiescence, &receipt.receipt_id, &receipt)?;
        self.db.flush().map_err(storage)?;
        Ok(())
    }
    pub fn bounded_step(&self) -> Result<LifecycleStepOutcome, LifecycleError> {
        for item in self.intents.iter() {
            let (key, _) = item.map_err(storage)?;
            if self.intent_phases.get(&key).map_err(storage)?.is_none() {
                let transition = hash(&(&key.as_ref(), "exact_inputs_pending"))?;
                self.intent_phases
                    .insert(&key, transition.as_bytes())
                    .map_err(storage)?;
                self.db.flush().map_err(storage)?;
                return Ok(LifecycleStepOutcome::Advanced {
                    transition_ref: transition,
                });
            }
        }
        Ok(LifecycleStepOutcome::NoWork)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleStepOutcome {
    Advanced { transition_ref: String },
    NoWork,
}
#[derive(Clone)]
pub struct ActivationLifecycleService {
    store: ActivationLifecycleStore,
}
impl ActivationLifecycleService {
    pub fn new(store: ActivationLifecycleStore) -> Self {
        Self { store }
    }
    pub fn runtime_id(&self) -> &'static str {
        STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID
    }
    pub fn bounded_step(&self) -> Result<LifecycleStepOutcome, LifecycleError> {
        self.store.bounded_step()
    }
}

fn put_immutable<T: Serialize>(tree: &Tree, key: &str, value: &T) -> Result<(), LifecycleError> {
    let bytes = bincode::serialize(value).map_err(|e| LifecycleError::Storage(e.to_string()))?;
    if let Some(existing) = tree.get(key.as_bytes()).map_err(storage)? {
        if existing.as_ref() != bytes.as_slice() {
            return Err(LifecycleError::Conflict(format!(
                "immutable record '{key}' differs"
            )));
        }
    } else {
        tree.insert(key.as_bytes(), bytes).map_err(storage)?;
    }
    Ok(())
}
fn put_replace<T: Serialize>(tree: &Tree, key: &str, value: &T) -> Result<(), LifecycleError> {
    tree.insert(
        key.as_bytes(),
        bincode::serialize(value).map_err(|e| LifecycleError::Storage(e.to_string()))?,
    )
    .map_err(storage)?;
    Ok(())
}
fn get_required<T: DeserializeOwned>(tree: &Tree, key: &str) -> Result<T, LifecycleError> {
    let bytes = tree
        .get(key.as_bytes())
        .map_err(storage)?
        .ok_or_else(|| LifecycleError::Invalid(format!("record '{key}' is absent")))?;
    bincode::deserialize(&bytes).map_err(|e| LifecycleError::Storage(e.to_string()))
}
fn hash(value: &impl Serialize) -> Result<String, LifecycleError> {
    bincode::serialize(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|e| LifecycleError::Storage(e.to_string()))
}
fn storage(error: sled::Error) -> LifecycleError {
    LifecycleError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_lang::CapabilityRef;
    fn store() -> ActivationLifecycleStore {
        ActivationLifecycleStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap()
    }
    #[test]
    fn intent_retry_is_idempotent() {
        let store = store();
        let intent = ActivationLifecycleIntentV1::new(
            "request".into(),
            "assignment".into(),
            "activation".into(),
            None,
            LifecycleAction::Activate,
        )
        .unwrap();
        assert_eq!(
            store.submit_intent(intent.clone()).unwrap(),
            store.submit_intent(intent).unwrap()
        );
    }
    #[test]
    fn expected_prior_fences_concurrent_replacement() {
        let store = store();
        store.publish_head("a", None, "g1").unwrap();
        assert!(store.publish_head("a", None, "g2").is_err());
        store.publish_head("a", Some("g1"), "g2").unwrap();
    }
    fn operation() -> DurableOperationV1 {
        DurableOperationV1 {
            operation_key: "op".into(),
            assignment_id: "a".into(),
            generation_id: "g".into(),
            capability_contract_ref: CapabilityContractRevisionRef {
                selector: CapabilityRef {
                    capability_type_id: "dependency_security.assess".into(),
                    capability_version: 1,
                },
                content_identity: "contract".into(),
            },
            semantic_request_hash: "request".into(),
            status: DurableOperationStatus::Pending,
            admitted_product_refs: vec![],
        }
    }
    #[test]
    fn operation_persists_before_dispatch() {
        let store = store();
        assert!(store
            .begin_attempt("op", "i".into(), "claim".into(), 1)
            .is_err());
        store.persist_operation(operation()).unwrap();
        assert!(matches!(
            store
                .begin_attempt("op", "i".into(), "claim".into(), 1)
                .unwrap()
                .status,
            OperationAttemptStatus::Claimed
        ));
    }
    #[test]
    fn ambiguous_attempt_reconciles_before_retry() {
        let store = store();
        store.persist_operation(operation()).unwrap();
        let attempt = store
            .begin_attempt("op", "i".into(), "claim".into(), 1)
            .unwrap();
        store.mark_ambiguous("op", &attempt.attempt_id).unwrap();
        assert!(store
            .begin_attempt("op", "i".into(), "claim2".into(), 2)
            .is_err());
        store
            .reconcile_no_effect("op", &attempt.attempt_id)
            .unwrap();
        assert!(store
            .begin_attempt("op", "i".into(), "claim2".into(), 2)
            .is_ok());
    }
    #[test]
    fn zero_work_without_complete_waits_is_stalled() {
        let store = store();
        let projection = store.project_liveness(
            "a".into(),
            "g".into(),
            &BTreeSet::from(["incarnation".into()]),
            vec![],
            &BTreeSet::new(),
            0,
        );
        assert_eq!(projection.state, ProjectedLifecycleState::Stalled);
    }
    #[test]
    fn complete_wait_and_wake_closure_is_quiescent() {
        let store = store();
        let wake = StructuralWakeRef::OperatorAction("resume".into());
        let projection = store.project_liveness(
            "a".into(),
            "g".into(),
            &BTreeSet::from(["incarnation".into()]),
            vec![OwnerWaitReceiptV1 {
                wait_ref: "wait".into(),
                generation_id: "g".into(),
                incarnation_id: "incarnation".into(),
                owner_checkpoint_ref: "checkpoint".into(),
                reason_code: "no_fixture_advance".into(),
                wake_refs: vec![wake.clone()],
            }],
            &BTreeSet::from([wake]),
            0,
        );
        assert_eq!(projection.state, ProjectedLifecycleState::Quiescent);
    }
    #[test]
    fn participant_restart_changes_incarnation_not_generation() {
        let store = store();
        let first = store
            .create_incarnation("g".into(), "p".into(), "adapter".into(), "lease1".into(), 1)
            .unwrap();
        let second = store
            .create_incarnation("g".into(), "p".into(), "adapter".into(), "lease2".into(), 2)
            .unwrap();
        assert_ne!(first.incarnation_id, second.incarnation_id);
        assert_eq!(first.generation_id, second.generation_id);
    }
    #[test]
    fn replacement_fences_old_before_new_publication() {
        let store = store();
        store.publish_head("a", None, "g1").unwrap();
        store.replace_head("a", "g1", "g2").unwrap();
        assert!(!store.admission_open("g1").unwrap());
        assert!(store.admission_open("g2").unwrap());
    }
    #[test]
    fn admission_stays_closed_until_reconciliation() {
        let store = store();
        store.publish_head("a", None, "g").unwrap();
        store
            .recover_incarnation("g".into(), "p".into(), "adapter".into(), "lease".into(), 2)
            .unwrap();
        assert!(!store.admission_open("g").unwrap());
    }
    #[test]
    fn reopen_resumes_incomplete_lifecycle_phase() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("lifecycle.sled");
        let db = sled::open(&path).unwrap();
        let store = ActivationLifecycleStore::new(db.clone()).unwrap();
        let intent = ActivationLifecycleIntentV1::new(
            "recover".into(),
            "a".into(),
            "activation".into(),
            Some("g".into()),
            LifecycleAction::Recover,
        )
        .unwrap();
        let id = store.submit_intent(intent).unwrap().intent_id;
        drop(store);
        drop(db);
        let reopened = ActivationLifecycleStore::new(sled::open(path).unwrap()).unwrap();
        let retry = ActivationLifecycleIntentV1::new(
            "recover".into(),
            "a".into(),
            "activation".into(),
            Some("g".into()),
            LifecycleAction::Recover,
        )
        .unwrap();
        assert_eq!(reopened.submit_intent(retry).unwrap().intent_id, id);
    }
    #[test]
    fn one_stable_service_advances_one_bounded_transition() {
        let store = store();
        store
            .submit_intent(
                ActivationLifecycleIntentV1::new(
                    "request".into(),
                    "a".into(),
                    "activation".into(),
                    None,
                    LifecycleAction::Activate,
                )
                .unwrap(),
            )
            .unwrap();
        let service = ActivationLifecycleService::new(store);
        assert_eq!(service.runtime_id(), STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID);
        assert!(matches!(
            service.bounded_step().unwrap(),
            LifecycleStepOutcome::Advanced { .. }
        ));
        assert_eq!(
            service.bounded_step().unwrap(),
            LifecycleStepOutcome::NoWork
        );
    }

    #[test]
    fn fenced_quiescence_accepts_durable_unresolved_effect() {
        let store = store();
        store.publish_head("a", None, "g").unwrap();
        let fence = store.close_admission("g").unwrap();
        let receipt = FencedQuiescenceReceiptV1::new(
            "a".into(),
            "g".into(),
            fence,
            vec!["owner-safe".into()],
            "one-ambiguous-operation".into(),
            vec!["subscription-fenced".into()],
            vec!["participant-drained".into()],
        )
        .unwrap();
        store.commit_quiescence(receipt).unwrap();
    }
}
