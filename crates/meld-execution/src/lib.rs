//! Execution authority contracts for Meld capability, task, workflow,
//! generation, planning, goal, traversal, and publish flows.
//!
//! This crate owns execution-facing contracts, orchestration records, runtime
//! ports, task artifacts, workflow state, capability invocation shapes, and the
//! DTOs passed between those execution boundaries.
//!
//! This crate does not own CLI parsing, workspace storage internals, provider
//! transport implementations, world model storage, or language semantics. Those
//! domains adapt into this crate through explicit ports and shared contracts.
//!
//! # Module Map
//!
//! - [`capability`] publishes capability contracts, catalogs, invocation
//!   payloads, and bound runtime records.
//! - [`execution`] defines the adapter ports used by workflow, task, and
//!   workspace runtimes.
//! - [`generation`] carries prompt assembly, provider completion, prompt
//!   lineage, and generated metadata DTOs.
//! - [`goals`] stores execution-owned goal sets.
//! - [`planning`] evaluates goals into execution compositions.
//! - [`publish`] carries frame head publish templates.
//! - [`task`] compiles and runs task-local capability graphs.
//! - [`task_network`] accepts task network commands and reduces graph state.
//! - [`traversal`] carries traversal expansion templates for workflow-backed
//!   task packages.
//! - [`workflow`] defines workflow profiles, runtime state, gates, events, and
//!   execution entry points.
//!
//! Start with [`execution`] when implementing an adapter boundary, [`workflow`]
//! when running profile-driven orchestration, [`task`] when working with
//! compiled capability graphs, and [`capability`] when publishing or binding
//! executable capability contracts.

#![deny(missing_docs)]

/// Capability publication, binding, invocation, and runtime contracts.
pub mod capability;
/// Execution-domain error types shared by public contracts.
pub mod error;
/// Runtime port traits and provider execution binding contracts.
pub mod execution;
/// Prompt assembly, provider completion, lineage, and metadata DTOs.
pub mod generation;
/// Execution-owned goal set commands, records, stores, and queries.
pub mod goals;
/// Goal planning contracts, method libraries, and planning runtime facade.
pub mod planning;
/// Frame head publish templates used by expansion paths.
pub mod publish;
/// Task definition, compilation, artifact, invocation, and runtime contracts.
pub mod task;
/// Task network command, mutation, state, dispatch, and publication contracts.
pub mod task_network;
/// Traversal expansion DTOs for workflow-backed task packages.
pub mod traversal;
/// Workflow profiles, state, gates, events, and execution runtimes.
pub mod workflow;

pub use execution::*;
pub use generation::*;
pub use planning::lowering::{
    Diagnostic as CompositionLoweringDiagnostic, Lowerer as ExecutionCompositionLowerer,
    Plan as CompositionLoweringPlan, Request as CompositionLoweringRequest,
};
pub use task_network::command::{
    Command as TaskNetworkCommand, Request as TaskNetworkCommandRequest,
    Response as TaskNetworkCommandResponse,
};
pub use task_network::mutation::{
    CommitRecord as TaskNetworkCommitRecord, CommitRequest as TaskNetworkCommitRequest,
    CommitResult as TaskNetworkCommitResult, Inject as TaskNetworkInjectMutation,
    Mutation as TaskNetworkMutation, Set as TaskNetworkMutationSet,
};
pub use workflow::*;
