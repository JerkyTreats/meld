//! Supervisor lifecycle contracts for root runtime ownership.

use std::fmt;
use std::path::PathBuf;

use meld_events::EventFinalBarrier;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Error returned by supervisor lifecycle contract validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SupervisorContractError {
    /// Runtime id is empty or outside the supervisor id grammar.
    #[error("invalid supervisor runtime id: {0}")]
    InvalidRuntimeId(String),
    /// Restart schedule does not enforce requested backoff.
    #[error("invalid restart schedule: {0}")]
    InvalidRestartSchedule(String),
    /// Replacement checkpoint skipped or reordered a required stage.
    #[error("invalid replacement checkpoint: {0}")]
    InvalidReplacementCheckpoint(String),
    /// Shutdown completion is not bound to completed lifecycle state.
    #[error("invalid shutdown completion: {0}")]
    InvalidShutdownCompletion(String),
}

/// Stable id for one supervised runtime role.
///
/// Runtime ids identify process roles only. Domain stores own their aggregate
/// ids, replay positions, task state, publication state, and source positions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeId(String);

/// Desired lifecycle state for one runtime id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDesiredState {
    /// Stable supervised runtime id.
    pub runtime_id: RuntimeId,
    /// Whether this runtime should be started by the supervisor.
    pub enabled: bool,
    /// Restart behavior requested for this runtime.
    pub restart_policy: RestartPolicy,
}

/// Registered root supervisor process instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeInstance {
    /// Generated process instance id.
    pub instance_id: String,
    /// Product storage root this supervisor owns operationally.
    pub product_root: PathBuf,
    /// Instance start time in milliseconds.
    pub started_at_ms: u64,
    /// Instance stop time in milliseconds when known.
    pub stopped_at_ms: Option<u64>,
    /// Current operational instance status.
    pub status: RuntimeInstanceStatus,
}

/// Operational status for one supervisor process instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeInstanceStatus {
    /// Instance has been created but startup is not yet complete.
    Starting,
    /// Instance is running and may own runtime leases.
    Running,
    /// Instance is coordinating graceful shutdown.
    Stopping,
    /// Instance completed graceful shutdown.
    Stopped,
    /// Instance did not stop cleanly before its leases expired.
    Stale,
    /// Instance failed before clean shutdown.
    Failed,
}

/// Runtime lease owner tuple supplied by a running handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeLeaseOwner {
    /// Stable runtime id.
    pub runtime_id: RuntimeId,
    /// Lease id generated for one handle start.
    pub lease_id: String,
    /// Supervisor instance id that acquired the lease.
    pub instance_id: String,
}

/// Duplicate runtime ownership guard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeLease {
    /// Stable runtime id.
    pub runtime_id: RuntimeId,
    /// Lease id generated for one handle start.
    pub lease_id: String,
    /// Supervisor instance id that owns this lease.
    pub instance_id: String,
    /// Lease acquisition time in milliseconds.
    pub acquired_at_ms: u64,
    /// Last renewal time in milliseconds.
    pub renewed_at_ms: Option<u64>,
    /// Release time in milliseconds when shutdown completed.
    pub released_at_ms: Option<u64>,
    /// Expiry time in milliseconds.
    pub expires_at_ms: u64,
    /// Current lease lifecycle status.
    pub status: RuntimeLeaseStatus,
}

/// Operational lease lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLeaseStatus {
    /// Lease acquisition is being committed.
    Acquiring,
    /// Lease is the current active owner for its runtime id.
    Active,
    /// Lease renewal is being committed.
    Renewing,
    /// Lease was released during graceful shutdown.
    Released,
    /// Lease expired and is no longer trusted as owner.
    Expired,
    /// Lease was replaced by another owner.
    Superseded,
    /// Lease owner was abandoned after an unsafe stop.
    Abandoned,
}

/// Latest runtime heartbeat accepted by the supervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHeartbeat {
    /// Stable runtime id.
    pub runtime_id: RuntimeId,
    /// Active lease id for the heartbeat owner.
    pub lease_id: String,
    /// Active supervisor instance id for the heartbeat owner.
    pub instance_id: String,
    /// Observation time in milliseconds.
    pub observed_at_ms: u64,
    /// Current health reported by the runtime handle.
    pub health: RuntimeHealth,
    /// Bounded operational diagnostic summary.
    pub diagnostic: Option<RuntimeDiagnosticSummary>,
}

/// Runtime health signal copied from a lifecycle handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHealth {
    /// Coarse operational health status.
    pub status: RuntimeHealthStatus,
    /// Retryable diagnostic count reported by the runtime.
    pub retryable_error_count: u64,
    /// Fatal diagnostic count reported by the runtime.
    pub fatal_error_count: u64,
    /// True when the last worker stopped on budget.
    pub budget_exhausted: bool,
    /// Last successful tick time in milliseconds when reported.
    pub last_successful_tick_at_ms: Option<u64>,
    /// Last stable operational error code when reported.
    pub last_error_code: Option<String>,
}

/// Coarse runtime health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHealthStatus {
    /// Runtime has not yet reported a clear state.
    Unknown,
    /// Runtime is starting.
    Starting,
    /// Runtime is healthy.
    Healthy,
    /// Runtime is running with retryable issues.
    Degraded,
    /// Runtime is unhealthy and needs supervisor or operator action.
    Unhealthy,
    /// Runtime has stopped.
    Stopped,
}

/// Bounded diagnostic summary for operator visibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDiagnosticSummary {
    /// Runtime actor that produced the diagnostic.
    pub actor_id: String,
    /// Retryable diagnostic issue count.
    pub retryable_issue_count: u64,
    /// Fatal diagnostic issue count.
    pub fatal_issue_count: u64,
    /// True when the diagnostic stopped due to budget.
    pub budget_exhausted: bool,
    /// Last stable operational error code when known.
    pub last_error_code: Option<String>,
}

/// Supervisor health snapshot derived from lifecycle records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHealthSnapshot {
    /// Stable runtime id.
    pub runtime_id: RuntimeId,
    /// Active lease id when one is present.
    pub lease_id: Option<String>,
    /// Snapshot observation time in milliseconds.
    pub observed_at_ms: u64,
    /// Derived operational health status.
    pub status: RuntimeHealthStatus,
    /// Last heartbeat time in milliseconds when present.
    pub last_heartbeat_at_ms: Option<u64>,
    /// Retryable diagnostic count.
    pub retryable_error_count: u64,
    /// Fatal diagnostic count.
    pub fatal_error_count: u64,
    /// True when the latest report stopped on budget.
    pub budget_exhausted: bool,
    /// Restart attempts observed by the supervisor.
    pub restart_count: u64,
    /// Last restart cause when one has occurred.
    pub last_restart_cause: Option<RestartCause>,
}

/// Conservative restart policy for supervised runtime handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Never restart without an explicit operator or caller request.
    Never,
    /// Restart after a retryable runtime failure.
    OnRetryableFailure,
    /// Restart after heartbeat expiry and lease recovery.
    OnHeartbeatExpiry,
}

/// Operational restart cause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartCause {
    /// Runtime reported a retryable failure.
    RetryableFailure,
    /// Runtime heartbeat or lease expired.
    HeartbeatExpired,
    /// Operator requested restart.
    OperatorRequested,
}

/// Restart audit record for one runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRestartRecord {
    /// Stable restart id.
    pub restart_id: String,
    /// Stable runtime id.
    pub runtime_id: RuntimeId,
    /// Supervisor instance id that scheduled the restart.
    pub instance_id: String,
    /// Prior lease id when known.
    pub previous_lease_id: Option<String>,
    /// Restart cause recorded for operators.
    pub cause: RestartCause,
    /// Attempt number for this runtime.
    pub attempt: u64,
    /// Restart scheduling time in milliseconds.
    pub requested_at_ms: u64,
    /// Bounded backoff chosen by the supervisor.
    pub backoff_ms: u64,
}

/// Enforced restart schedule retaining the canonical audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeRestartScheduleWire")]
pub struct RuntimeRestartSchedule {
    /// Canonical restart audit product.
    restart: RuntimeRestartRecord,
    /// Earliest supervisor time when replacement may begin.
    next_eligible_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RuntimeRestartScheduleWire {
    restart: RuntimeRestartRecord,
    next_eligible_at_ms: u64,
}

impl RuntimeRestartSchedule {
    /// Build an enforced schedule from the canonical restart record.
    pub fn try_new(restart: RuntimeRestartRecord) -> Result<Self, SupervisorContractError> {
        let next_eligible_at_ms = restart
            .requested_at_ms
            .checked_add(restart.backoff_ms)
            .ok_or_else(|| {
                SupervisorContractError::InvalidRestartSchedule(
                    "restart eligibility overflowed u64".to_string(),
                )
            })?;
        Ok(Self {
            restart,
            next_eligible_at_ms,
        })
    }

    /// Borrow the canonical restart audit product.
    pub fn restart(&self) -> &RuntimeRestartRecord {
        &self.restart
    }

    /// Return the enforced replacement eligibility time.
    pub fn next_eligible_at_ms(&self) -> u64 {
        self.next_eligible_at_ms
    }
}

impl TryFrom<RuntimeRestartScheduleWire> for RuntimeRestartSchedule {
    type Error = SupervisorContractError;

    fn try_from(value: RuntimeRestartScheduleWire) -> Result<Self, Self::Error> {
        let schedule = Self::try_new(value.restart)?;
        if schedule.next_eligible_at_ms != value.next_eligible_at_ms {
            return Err(SupervisorContractError::InvalidRestartSchedule(
                "persisted restart eligibility does not equal request time plus backoff"
                    .to_string(),
            ));
        }
        Ok(schedule)
    }
}

/// Ordered replacement stage for one expired or failed runtime handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeReplacementStage {
    /// Signal the old handle to stop.
    StopOldHandle,
    /// Wait for the old handle safe point.
    AwaitOldSafePoint,
    /// Flush resources owned by the old handle.
    FlushOldHandle,
    /// Release the prior lease after the old handle is safe and durable.
    ReleaseOldLease,
    /// Acquire the replacement lease after backoff.
    AcquireReplacementLease,
    /// Start the replacement handle under the new lease.
    StartReplacementHandle,
    /// Replacement completed.
    Completed,
}

/// Persistable replacement checkpoint used for restart recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeReplacementCheckpointWire")]
pub struct RuntimeReplacementCheckpoint {
    /// Enforced schedule and canonical restart audit product.
    schedule: RuntimeRestartSchedule,
    /// Exact ordered prefix of completed replacement stages.
    completed_stages: Vec<RuntimeReplacementStage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RuntimeReplacementCheckpointWire {
    schedule: RuntimeRestartSchedule,
    completed_stages: Vec<RuntimeReplacementStage>,
}

impl RuntimeReplacementCheckpoint {
    /// Start a replacement checkpoint before any destructive step.
    pub fn new(schedule: RuntimeRestartSchedule) -> Self {
        Self {
            schedule,
            completed_stages: Vec::new(),
        }
    }

    /// Append exactly the next replacement stage.
    pub fn advance(
        mut self,
        stage: RuntimeReplacementStage,
    ) -> Result<Self, SupervisorContractError> {
        let expected = replacement_stage_order()
            .get(self.completed_stages.len())
            .copied();
        if expected != Some(stage) {
            return Err(SupervisorContractError::InvalidReplacementCheckpoint(
                "replacement stage does not follow the required stop and flush order".to_string(),
            ));
        }
        self.completed_stages.push(stage);
        Ok(self)
    }

    /// Borrow the enforced restart schedule.
    pub fn schedule(&self) -> &RuntimeRestartSchedule {
        &self.schedule
    }

    /// Borrow the exact ordered prefix of completed stages.
    pub fn completed_stages(&self) -> &[RuntimeReplacementStage] {
        &self.completed_stages
    }

    fn validate(&self) -> Result<(), SupervisorContractError> {
        let required = replacement_stage_order();
        if self.completed_stages.len() > required.len()
            || self.completed_stages.as_slice() != &required[..self.completed_stages.len()]
        {
            return Err(SupervisorContractError::InvalidReplacementCheckpoint(
                "persisted replacement stages are not an ordered prefix".to_string(),
            ));
        }
        Ok(())
    }
}

impl TryFrom<RuntimeReplacementCheckpointWire> for RuntimeReplacementCheckpoint {
    type Error = SupervisorContractError;

    fn try_from(value: RuntimeReplacementCheckpointWire) -> Result<Self, Self::Error> {
        let checkpoint = Self {
            schedule: value.schedule,
            completed_stages: value.completed_stages,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }
}

fn replacement_stage_order() -> &'static [RuntimeReplacementStage] {
    &[
        RuntimeReplacementStage::StopOldHandle,
        RuntimeReplacementStage::AwaitOldSafePoint,
        RuntimeReplacementStage::FlushOldHandle,
        RuntimeReplacementStage::ReleaseOldLease,
        RuntimeReplacementStage::AcquireReplacementLease,
        RuntimeReplacementStage::StartReplacementHandle,
        RuntimeReplacementStage::Completed,
    ]
}

/// Graceful shutdown state for one supervisor instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeShutdownState {
    /// Stable shutdown id.
    pub shutdown_id: String,
    /// Supervisor instance id coordinating shutdown.
    pub instance_id: String,
    /// Shutdown request time in milliseconds.
    pub requested_at_ms: u64,
    /// Shutdown completion time in milliseconds when known.
    pub completed_at_ms: Option<u64>,
    /// Current shutdown status.
    pub status: RuntimeShutdownStatus,
}

/// Completed shutdown bound to the final closed event barrier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeShutdownCompletionWire")]
pub struct RuntimeShutdownCompletion {
    /// Canonical completed shutdown state.
    shutdown: RuntimeShutdownState,
    /// Final closed and fully durable event barrier.
    final_event_barrier: EventFinalBarrier,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RuntimeShutdownCompletionWire {
    shutdown: RuntimeShutdownState,
    final_event_barrier: EventFinalBarrier,
}

impl RuntimeShutdownCompletion {
    /// Bind completed shutdown state to its final event barrier.
    pub fn try_new(
        shutdown: RuntimeShutdownState,
        final_event_barrier: EventFinalBarrier,
    ) -> Result<Self, SupervisorContractError> {
        if shutdown.status != RuntimeShutdownStatus::Completed || shutdown.completed_at_ms.is_none()
        {
            return Err(SupervisorContractError::InvalidShutdownCompletion(
                "final event barrier requires completed shutdown state".to_string(),
            ));
        }
        Ok(Self {
            shutdown,
            final_event_barrier,
        })
    }

    /// Borrow canonical shutdown state.
    pub fn shutdown(&self) -> &RuntimeShutdownState {
        &self.shutdown
    }

    /// Return the final closed event barrier.
    pub fn final_event_barrier(&self) -> EventFinalBarrier {
        self.final_event_barrier
    }
}

impl TryFrom<RuntimeShutdownCompletionWire> for RuntimeShutdownCompletion {
    type Error = SupervisorContractError;

    fn try_from(value: RuntimeShutdownCompletionWire) -> Result<Self, Self::Error> {
        Self::try_new(value.shutdown, value.final_event_barrier)
    }
}

/// Graceful shutdown lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeShutdownStatus {
    /// Shutdown has been requested.
    Requested,
    /// Runtime handles are being signaled.
    Signaling,
    /// Runtime safe points are being awaited.
    WaitingForSafePoint,
    /// Product stores are being flushed.
    FlushingStores,
    /// Shutdown completed cleanly.
    Completed,
    /// Shutdown exceeded its grace window.
    TimedOut,
    /// Shutdown failed.
    Failed,
}

/// Operational lifecycle event owned by the supervisor store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorLifecycleEvent {
    /// Stable event id.
    pub event_id: String,
    /// Supervisor instance id associated with the event.
    pub instance_id: String,
    /// Runtime id when the event is runtime scoped.
    pub runtime_id: Option<RuntimeId>,
    /// Lease id when the event is lease scoped.
    pub lease_id: Option<String>,
    /// Event time in milliseconds.
    pub occurred_at_ms: u64,
    /// Operational event kind.
    pub event_type: SupervisorLifecycleEventType,
    /// Bounded human-readable operational detail.
    pub message: Option<String>,
}

/// Supervisor operational event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorLifecycleEventType {
    /// A supervisor instance was registered.
    InstanceRegistered,
    /// A runtime start was requested.
    RuntimeStartRequested,
    /// A lease was acquired.
    LeaseAcquired,
    /// A heartbeat was accepted.
    HeartbeatAccepted,
    /// A heartbeat was detected as stale.
    HeartbeatStale,
    /// A restart was scheduled.
    RestartScheduled,
    /// Shutdown was requested.
    ShutdownRequested,
    /// A lease was released.
    LeaseReleased,
    /// A lease was expired during recovery.
    LeaseExpired,
    /// A supervisor instance stopped.
    InstanceStopped,
}

impl RuntimeId {
    /// Validate and construct a runtime id.
    pub fn new(value: impl Into<String>) -> Result<Self, SupervisorContractError> {
        let value = value.into();
        validate_runtime_id(&value)?;
        Ok(Self(canonical_runtime_id(&value).to_string()))
    }

    /// Borrow the runtime id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the runtime id as a string.
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Translate requirement-era runtime ids at ingress.
pub fn canonical_runtime_id(value: &str) -> &str {
    match value {
        "events.ledger" => "event.append",
        "world_model.graph.replay" => "world_model.graph_replay",
        "world_model.belief.assessment" => "world_model.belief_assessment",
        "world_model.agent.goal_curation" => "world_model.agent_goal_curation",
        "world_model.belief.event_evidence_ingestion" => "world_model.evidence_ingestion",
        "world_model.agent.satisfaction_curation" => "world_model.satisfaction_curation",
        "execution.goal.set" => "execution.goal_set",
        "execution.task_network.command" => "execution.task_network_command",
        "execution.task.dispatch" => "execution.task_dispatch",
        "execution.task_network.publication" => "execution.publication",
        _ => value,
    }
}

impl RuntimeLease {
    /// Return the owner tuple for this lease.
    pub fn owner(&self) -> RuntimeLeaseOwner {
        RuntimeLeaseOwner {
            runtime_id: self.runtime_id.clone(),
            lease_id: self.lease_id.clone(),
            instance_id: self.instance_id.clone(),
        }
    }

    /// Return true when the lease can still be treated as active.
    pub fn is_active_at(&self, now_ms: u64) -> bool {
        self.has_active_status() && self.expires_at_ms > now_ms
    }

    /// Return true when the lease status belongs to the active owner set.
    pub fn has_active_status(&self) -> bool {
        matches!(
            self.status,
            RuntimeLeaseStatus::Acquiring
                | RuntimeLeaseStatus::Active
                | RuntimeLeaseStatus::Renewing
        )
    }

    /// Return true when the supplied owner matches this lease.
    pub fn is_owned_by(&self, owner: &RuntimeLeaseOwner) -> bool {
        self.runtime_id == owner.runtime_id
            && self.lease_id == owner.lease_id
            && self.instance_id == owner.instance_id
    }
}

impl fmt::Display for RuntimeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for RuntimeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RuntimeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        RuntimeId::new(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for RuntimeId {
    type Error = SupervisorContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        RuntimeId::new(value)
    }
}

impl TryFrom<&str> for RuntimeId {
    type Error = SupervisorContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        RuntimeId::new(value)
    }
}

/// Validate a supervisor runtime id.
pub fn validate_runtime_id(value: &str) -> Result<(), SupervisorContractError> {
    let valid = !value.is_empty()
        && value.contains('.')
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '.' || ch == '_' || ch == '-'
        });

    if valid {
        Ok(())
    } else {
        Err(SupervisorContractError::InvalidRuntimeId(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_validation_accepts_domain_scoped_ids() {
        for runtime_id in [
            "events.ledger",
            "world_model.graph.reducer",
            "world_model.belief.docs_freshness",
            "execution.task_network.network-docs",
            "execution.publication.network_docs",
        ] {
            assert_eq!(
                RuntimeId::new(runtime_id).unwrap().as_str(),
                canonical_runtime_id(runtime_id)
            );
        }
    }

    #[test]
    fn requirement_era_runtime_ids_normalize_at_ingress() {
        assert_eq!(
            RuntimeId::new("events.ledger").unwrap().as_str(),
            "event.append"
        );
        assert_eq!(
            RuntimeId::new("world_model.agent.goal_curation")
                .unwrap()
                .as_str(),
            "world_model.agent_goal_curation"
        );
    }

    #[test]
    fn restart_schedule_retains_backoff_eligibility_and_order() {
        let schedule = RuntimeRestartSchedule::try_new(RuntimeRestartRecord {
            restart_id: "restart-a".to_string(),
            runtime_id: RuntimeId::new("world_model.graph_replay").unwrap(),
            instance_id: "instance-a".to_string(),
            previous_lease_id: Some("lease-a".to_string()),
            cause: RestartCause::HeartbeatExpired,
            attempt: 1,
            requested_at_ms: 100,
            backoff_ms: 50,
        })
        .unwrap();
        let checkpoint = RuntimeReplacementCheckpoint::new(schedule)
            .advance(RuntimeReplacementStage::StopOldHandle)
            .unwrap()
            .advance(RuntimeReplacementStage::AwaitOldSafePoint)
            .unwrap();

        let encoded = serde_json::to_vec(&checkpoint).unwrap();
        let decoded: RuntimeReplacementCheckpoint = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, checkpoint);
        assert_eq!(decoded.schedule().next_eligible_at_ms(), 150);
        assert!(
            RuntimeReplacementCheckpoint::new(decoded.schedule().clone())
                .advance(RuntimeReplacementStage::AcquireReplacementLease)
                .is_err()
        );
        let mut invalid_schedule = serde_json::to_value(decoded.schedule()).unwrap();
        invalid_schedule["next_eligible_at_ms"] = serde_json::json!(149);
        assert!(serde_json::from_value::<RuntimeRestartSchedule>(invalid_schedule).is_err());
        let invalid_checkpoint = serde_json::json!({
            "schedule": decoded.schedule(),
            "completed_stages": ["acquire_replacement_lease"]
        });
        assert!(
            serde_json::from_value::<RuntimeReplacementCheckpoint>(invalid_checkpoint).is_err()
        );
    }

    #[test]
    fn runtime_id_validation_rejects_invalid_ids() {
        for runtime_id in [
            "",
            "events",
            "Events.ledger",
            "events ledger",
            " events.ledger",
            "events.ledger ",
            ".events",
            "events.",
            "events..ledger",
            "../events.ledger",
            "events/ledger",
            "events.ledger$",
        ] {
            assert!(RuntimeId::new(runtime_id).is_err(), "{runtime_id}");
        }
    }
}
