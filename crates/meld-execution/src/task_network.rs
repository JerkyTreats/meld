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

/// Serialized command boundary for task network writers.
pub mod command;
/// Shared identity and hashing helpers for task network records.
pub mod contracts;
/// Fenced dispatch claim and task outcome contracts.
pub mod dispatch;
/// Append-only task network journal records.
pub mod journal;
/// Append-only mutation and commit records.
pub mod mutation;
/// Outcome publication outbox contracts.
pub mod outcome;
/// Ready set computation over reduced task network state.
pub mod readiness;
/// Reduced task graph and lifecycle state contracts.
pub mod state;
/// In memory and sled-backed command stores.
pub mod store;

pub use command::{Command, Request as CommandRequest, Response};
pub use dispatch::{Claim, Outcome, OutcomeStatus, Request as DispatchRequest};
pub use journal::JournalRecord;
pub use mutation::{CommitRecord, CommitRequest, CommitResult, Inject, Mutation, Set};
pub use outcome::{Publication, PublicationState};
pub use readiness::compute_ready_set;
pub use state::{NetworkState, ReadySet, TaskNode, TaskStatus};
pub use store::{InMemoryTaskNetworkStore, SledTaskNetworkStore};
