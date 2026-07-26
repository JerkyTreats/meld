//! Integration tests for the Merkle filesystem state management system

mod agent_authorization;
mod agent_cli;
mod belief_context;
mod blake3_verification;
mod branches_query;
mod branches_runtime;
mod capability_contracts;
mod capability_invocation;
mod config_integration;
mod context_api;
mod context_cli;
mod context_traversal;
pub(crate) mod docs_freshness_fixture;
mod docs_freshness_fixture_contract;
mod docs_freshness_reopen_contract;
mod docs_writer_task;
mod event_ledger;
mod execution_projection;
mod frame_queue;
mod generation_parity;
mod goal_acceptance;
mod harness_run;
mod hasher_verification;
mod init_command;
mod logging_default;
mod model_providers;
mod node_deletion;
mod outcome_evidence;
pub(crate) mod parity_fixture;
mod parity_fixture_contract;
mod product_event_authority_cutover;
mod product_storage_assembly;
mod progress_observability;
mod provider_cli;
mod runtime_cli;
mod store_integration;
mod task_artifact_repo;
mod task_bottom_up_compile_shape;
mod task_compiler;
mod task_executor;
mod test_utils;
mod tooling_integration;
mod traversal_graph;
mod tree_determinism;
mod tree_structure;
mod unified_status;
mod workflow_cli;
mod workflow_contracts_conformance;
mod workflow_task_compatibility;
mod workflow_traversal;
mod workspace_commands;
mod workspace_isolation;
mod workspace_scan_capability;
mod workspace_traversal;
mod world_init_pipeline;
mod world_state_graph;
mod xdg_config;

pub use test_utils::{
    create_test_agent, create_test_provider, register_docs_writer_capabilities,
    spawn_docs_writer_server, spawn_wrapped_docs_writer_server, with_env_lock, with_xdg_data_home,
    with_xdg_env,
};
