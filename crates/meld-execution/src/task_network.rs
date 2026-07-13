//! Task network command, mutation, state, dispatch, and publication boundaries.
//!
//! This domain owns command acceptance, task graph mutation reduction, reduced
//! network state, ready task queries, dispatch claims, task outcome handoff, and
//! publication outbox state.
//!
//! Inputs are task network commands from planning, workers, publication
//! workers, and recovery code. Outputs are accepted journal records, reduced
//! state snapshots, ready sets, dispatch claims, and publication records.
//!
//! This domain does not own task-local compilation, capability invocation,
//! provider transport, goal lifecycle mutation, or world model belief revision.
//! Those domains adapt through explicit contracts.

/// Single-owner durable task network authority and bounded ports.
pub mod authority;
/// Serialized command boundary for task network writers.
pub mod command;
/// Shared identity and hashing helpers for task network records.
pub mod contracts;
/// Fenced dispatch claim and task outcome contracts.
pub mod dispatch;
/// Task initialization source validation and materialization.
pub mod initialization;
/// Append-only task network journal records.
pub mod journal;
/// Append-only mutation and commit records.
pub mod mutation;
/// Outcome publication outbox contracts.
pub mod outcome;
/// Publication bridge into the event ledger.
pub mod publication;
/// Ready set computation over reduced task network state.
pub mod readiness;
/// Bounded runtime actor facades for task network work.
pub mod runtime;
/// Reduced task graph and lifecycle state contracts.
pub mod state;
/// In memory and sled-backed command stores.
pub mod store;

pub use authority::{
    TaskNetworkAuthorities, TaskNetworkAuthority, TaskNetworkAuthorityError,
    TaskNetworkAuthorityLifecycle, TaskNetworkAuthorityLifecycleSnapshot,
    TaskNetworkAuthorityShutdownReceipt, TaskNetworkCommandPort, TaskNetworkQueryPort,
};
pub use command::{Command, Request as CommandRequest, Response};
pub use dispatch::{
    AttributedOutcome, Claim, Outcome, OutcomeSemanticLineage, OutcomeStatus,
    Request as DispatchRequest,
};
pub use initialization::{
    materialize_task_initialization, validate_task_init_graph_sources, validate_task_init_sources,
    MaterializedInitSource, MaterializedTaskInitialization, TaskInitializationDiagnostic,
    TaskInitializationDiagnosticCode, TaskInitializationMaterializationError,
};
pub use journal::JournalRecord;
pub use mutation::{CommitRecord, CommitRequest, CommitResult, Inject, Mutation, Set};
pub use outcome::{Publication, PublicationState};
pub use publication::{
    build_publication_envelope, publish_pending_publications, publish_publication, EventAppendSink,
    PublicationAppend, PublicationBridgeError, PublicationBridgeIssue, PublicationBridgeReport,
    PublicationBridgeScope, PublicationPublishResult, PublishPendingPublicationsRequest,
};
pub use readiness::compute_ready_set;
pub use runtime::{PublicationRuntime, PublicationRuntimeReport};
pub use state::{
    NetworkState, ReadySet, StaticSeedInitSource, TaskInitSource, TaskNode, TaskStatus,
    UpstreamArtifactInitSource,
};
pub use store::{InMemoryTaskNetworkStore, SledTaskNetworkStore};
