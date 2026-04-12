//! Workflow domain modules.

pub mod binding;
pub mod commands;
pub mod executor;
pub mod facade;
pub mod events;
pub mod gates;
pub mod normalization;
pub mod profile;
pub mod record_contracts;
pub mod registry;
pub mod resolver;
pub mod state_store;
pub mod summary;
pub mod tooling;

pub use facade::{
    build_target_execution_request, execute_registered_workflow_target,
    execute_registered_workflow_target_async, execute_workflow_target,
    execute_workflow_target_async,
};
