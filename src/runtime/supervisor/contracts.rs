//! Supervisor lifecycle contracts for root runtime ownership.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Error returned by supervisor lifecycle contract validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SupervisorContractError {
    /// Runtime id is empty or outside the supervisor id grammar.
    #[error("invalid supervisor runtime id: {0}")]
    InvalidRuntimeId(String),
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
pub struct RuntimeRestartSchedule {
    /// Canonical restart audit product.
    pub restart: RuntimeRestartRecord,
    /// Earliest supervisor time when replacement may begin.
    pub next_eligible_at_ms: u64,
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
pub struct RuntimeReplacementCheckpoint {
    /// Enforced schedule and canonical restart audit product.
    pub schedule: RuntimeRestartSchedule,
    /// Last durable replacement stage.
    pub stage: RuntimeReplacementStage,
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
        let schedule = RuntimeRestartSchedule {
            restart: RuntimeRestartRecord {
                restart_id: "restart-a".to_string(),
                runtime_id: RuntimeId::new("world_model.graph_replay").unwrap(),
                instance_id: "instance-a".to_string(),
                previous_lease_id: Some("lease-a".to_string()),
                cause: RestartCause::HeartbeatExpired,
                attempt: 1,
                requested_at_ms: 100,
                backoff_ms: 50,
            },
            next_eligible_at_ms: 150,
        };
        let checkpoint = RuntimeReplacementCheckpoint {
            schedule,
            stage: RuntimeReplacementStage::AwaitOldSafePoint,
        };

        let encoded = serde_json::to_vec(&checkpoint).unwrap();
        let decoded: RuntimeReplacementCheckpoint = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, checkpoint);
        assert_eq!(decoded.schedule.next_eligible_at_ms, 150);
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
