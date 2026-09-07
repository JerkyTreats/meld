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
mod context_cold_fallback;
mod context_traversal;
pub(crate) mod docs_freshness_fixture;
mod docs_freshness_fixture_contract;
pub(crate) mod docs_provider;
mod event_ledger;
mod execution_projection;
mod frame_queue;
mod generation_parity;
mod harness_run;
mod harness_served_surface;
mod harness_stall_specimen;
pub(crate) mod harness_survey_fixture;
mod harness_three_altitudes;
mod hasher_verification;
mod init_command;
mod logging_default;
mod model_providers;
mod node_deletion;
mod outcome_evidence;
pub(crate) mod outcome_evidence_support;
mod pds_docs_gmail_operator_verification;
mod product_event_authority_cutover;
mod product_storage_assembly;
mod progress_observability;
mod provider_cli;
pub(crate) mod readme_parity_assertions;
mod runtime_cli;
mod runtime_status_live;
mod store_integration;
mod task_artifact_repo;
mod task_compiler;
mod task_executor;
mod test_utils;
mod theory_source;
mod tooling_integration;
mod traversal_graph;
mod tree_determinism;
mod tree_structure;
mod unified_status;
mod workflow_cli;
mod workflow_contracts_conformance;
mod workflow_traversal;
mod workspace_commands;
mod workspace_isolation;
mod workspace_scan_capability;
mod workspace_traversal;
mod world_model_reconciliation;
mod world_state_graph;
mod xdg_config;

pub use test_utils::{create_test_agent, with_env_lock, with_xdg_data_home, with_xdg_env};

#[path = "fixtures/workflow_assets.rs"]
mod workflow_assets;

pub(crate) fn install_legacy_workflow_fixture() -> Result<(), meld::error::ApiError> {
    let root = meld::config::WorkflowConfig::default().resolve_user_profile_dir()?;
    for (relative, content) in workflow_assets::FILES {
        let path = root.join(relative);
        if path.exists() {
            continue;
        }
        std::fs::create_dir_all(path.parent().unwrap())
            .and_then(|_| std::fs::write(path, content))
            .map_err(|error| meld::error::ApiError::ConfigError(error.to_string()))?;
    }
    Ok(())
}
