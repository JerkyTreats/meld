//! Root runtime supervisor domain.

/// Supervisor lifecycle contracts.
pub mod contracts;
/// Explicit supervisor lifecycle entrypoint.
pub mod entrypoint;
/// Durable supervisor lifecycle store.
pub mod store;

pub use contracts::{
    RestartCause, RestartPolicy, RuntimeDesiredState, RuntimeDiagnosticSummary, RuntimeHealth,
    RuntimeHealthSnapshot, RuntimeHealthStatus, RuntimeHeartbeat, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLease, RuntimeLeaseOwner, RuntimeLeaseStatus,
    RuntimeReplacementCheckpoint, RuntimeReplacementStage, RuntimeRestartRecord,
    RuntimeRestartSchedule, RuntimeShutdownState, RuntimeShutdownStatus, SupervisorContractError,
    SupervisorLifecycleEvent, SupervisorLifecycleEventType,
};
pub use entrypoint::{
    RuntimeSupervisor, SupervisorRestartEvaluation, SupervisorRuntimeError,
    SupervisorRuntimeStatus, SupervisorShutdownReport, SupervisorStartCommand,
    SupervisorStatusSnapshot, SupervisorTickReport,
};
pub use store::{SupervisorStore, SupervisorStoreError};
