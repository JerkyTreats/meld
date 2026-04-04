//! Integration tests for the Merkle filesystem state management system

mod agent_authorization;
mod agent_cli;
mod blake3_verification;
mod capability_contracts;
mod capability_invocation;
mod config_integration;
mod context_api;
mod context_cli;
mod frame_queue;
mod generation_parity;
mod hasher_verification;
mod init_command;
mod logging_default;
mod model_providers;
mod node_deletion;
mod progress_observability;
mod provider_cli;
mod store_integration;
mod task_artifact_repo;
mod task_compiler;
mod task_executor;
mod test_utils;
mod tooling_integration;
mod tree_determinism;
mod tree_structure;
mod unified_status;
mod workflow_cli;
mod workflow_contracts_conformance;
mod workspace_commands;
mod workspace_isolation;
mod xdg_config;

pub use test_utils::{with_env_lock, with_xdg_data_home, with_xdg_env};
