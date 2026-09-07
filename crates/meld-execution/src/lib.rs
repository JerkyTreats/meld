//! Execution authority for Agent-authorized Tasks and shared operational work.
//!
//! This crate owns complete Task admission, capability invocation, Task Network
//! realization, independent discharge accounts and native Execution lifecycle.
//! Provider and workspace adapters enter through explicit execution ports.
//! Agent and Strategy retain Goal judgment and Plan construction.
//!
//! # Module Map
//!
//! - [`capability`] publishes contracts, invocation payloads and runtime bindings.
//! - [`authority`] owns policy revisions and execution revalidation.
//! - [`execution`] defines provider and workspace adapter contracts.
//! - [`generation`] carries prompt, provider completion and lineage data.
//! - [`task_admission`] validates and lowers independently authorized Tasks.
//! - [`task`] compiles and executes task-local capability graphs.
//! - [`task_network`] shares compatible work and retains each admission's returns.
//! - [`lifecycle`] supplies native Execution transition evidence.
//! - [`workflow`] retains historical profile and Event contracts and reusable
//!   output validators. It provides no Workflow coordinator or state writer.
//!
//! Start with [`task_admission`] for the Agent boundary, [`task_network`] for
//! operational coherence, and [`capability`] for executable owner contracts.

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
