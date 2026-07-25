//! Root runtime supervisor domain.

/// Supervisor lifecycle contracts.
pub mod contracts;
/// Explicit supervisor lifecycle entrypoint.
pub mod entrypoint;
/// Durable per-tick report preservation.
pub mod reports;
/// Bounded-step adaptation for supervised handles.
pub mod stepping;
/// Durable supervisor lifecycle store.
pub mod store;

pub use contracts::{
    RestartCause, RestartPolicy, RuntimeDesiredState, RuntimeDiagnosticSummary, RuntimeHealth,
    RuntimeHealthSnapshot, RuntimeHealthStatus, RuntimeHeartbeat, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLease, RuntimeLeaseOwner, RuntimeLeaseStatus,
    RuntimeRestartRecord, RuntimeShutdownState, RuntimeShutdownStatus, SupervisorContractError,
    SupervisorLifecycleEvent, SupervisorLifecycleEventType,
};
pub use entrypoint::{
    RuntimeSupervisor, SupervisorRestartEvaluation, SupervisorRuntimeError,
    SupervisorRuntimeStatus, SupervisorShutdownReport, SupervisorStartCommand,
    SupervisorStatusSnapshot, SupervisorTickReport,
};
pub use reports::{SupervisorReportStore, DEFAULT_MAX_TICK_ACTION_RECORDS};
pub use stepping::BoundedActorHandle;
pub use store::{SupervisorStore, SupervisorStoreError};
