//! Execution authority contracts for Meld capability, task, workflow,
//! generation, Task admission, traversal, and publish flows.
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
//! - [`authority`] owns exact policy revisions and execution revalidation.
//! - [`execution`] defines the adapter ports used by workflow, task, and
//!   workspace runtimes.
//! - [`generation`] carries prompt assembly, provider completion, prompt
//!   lineage, and generated metadata DTOs.
//! - [`task_admission`] validates and directly lowers Agent-authorized Tasks.
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

/// Effective-authority policy durability and execution enforcement helpers.
pub mod authority;
/// Capability publication, binding, invocation, and runtime contracts.
pub mod capability;
/// Execution-domain error types shared by public contracts.
pub mod error;
/// Runtime port traits and provider execution binding contracts.
pub mod execution;
/// Prompt assembly, provider completion, lineage, and metadata DTOs.
pub mod generation;
/// Native lifecycle transitions owned by Execution runtime actors.
pub mod lifecycle;
/// Frame head publish templates used by expansion paths.
pub mod publish;
/// Task definition, compilation, artifact, invocation, and runtime contracts.
pub mod task;
/// Durable admission and direct lowering of Agent-authorized Tasks.
pub mod task_admission;
/// Task network command, mutation, state, dispatch, and publication contracts.
pub mod task_network;
pub mod waiting;

pub use waiting::{StructuralWakeAddress, WaitingOnDeclaration};
/// Legacy Workflow profile and historical Event contracts.
pub mod workflow;

pub use execution::*;
pub use generation::*;
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
