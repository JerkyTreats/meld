//! CLI route: shared runtime context and top-level command dispatch only.

use crate::branches::{BranchHandle, BranchRuntime};
use crate::cli::parse::Commands;
use crate::cli::progress::LiveProgressHandle;
use crate::cli::runtime_assembly::CliRuntimeAssembly;
use crate::cli::session::{finish_command_session, start_command_session};
use crate::cli::{command_name, typed_summary_event};
use crate::config::ConfigLoader;
use crate::error::ApiError;
use crate::session::PrunePolicy;
use crate::telemetry::emission::{emit_command_summary, truncate_for_summary};
use crate::telemetry::ProgressRuntime;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::warn;

/// Runtime context for CLI execution: workspace, config paths, and domain facades.
/// Built from workspace path and optional config path using ConfigLoader only.
pub struct RunContext {
    assembly: CliRuntimeAssembly,
    workspace_root: PathBuf,
    config_path: Option<PathBuf>,
    #[allow(dead_code)]
    store_path: PathBuf,
    frame_storage_path: PathBuf,
    #[allow(dead_code)]
    artifact_storage_path: PathBuf,
    branch_runtime: BranchRuntime,
    active_branch: BranchHandle,
}

impl RunContext {
    /// Reference to the underlying context API.
    pub fn api(&self) -> &crate::api::ContextApi {
        self.assembly.api().as_ref()
    }

    /// Progress runtime for session and event emission.
    pub fn progress_runtime(&self) -> Arc<ProgressRuntime> {
        Arc::clone(self.assembly.progress())
    }

    /// Workflow profile registry.
    pub fn workflow_registry(&self) -> Arc<parking_lot::RwLock<crate::workflow::WorkflowRegistry>> {
        Arc::clone(self.assembly.workflow_registry())
    }

    /// Create run context from workspace root and optional config path. Uses ConfigLoader only.
    pub fn new(workspace_root: PathBuf, config_path: Option<PathBuf>) -> Result<Self, ApiError> {
        let config = if let Some(ref cfg_path) = config_path {
            ConfigLoader::load_from_file(cfg_path)?
        } else {
            ConfigLoader::load(&workspace_root)?
        };
        let branch_runtime = BranchRuntime::new();
        let active_branch = branch_runtime.resolve_active_branch(&workspace_root)?;
        if let Err(err) = branch_runtime.ensure_active_branch_registered(&active_branch) {
            warn!(error = %err, "failed to register active branch during startup");
        }

        let (store_path, frame_storage_path, artifact_storage_path) =
            config.system.storage.resolve_paths(&workspace_root)?;
        let assembly = CliRuntimeAssembly::load(&workspace_root, &config)?;

        match assembly.graph_runtime().catch_up() {
            Ok(applied_events) => {
                let last_reduced_seq = match assembly
                    .graph_runtime()
                    .traversal_store()
                    .last_reduced_seq()
                {
                    Ok(seq) => seq,
                    Err(err) => {
                        warn!(error = %err, "failed to read last reduced seq during startup");
                        0
                    }
                };
                if let Err(err) = branch_runtime.record_branch_graph_catch_up_success(
                    &active_branch,
                    last_reduced_seq,
                    applied_events,
                ) {
                    warn!(error = %err, "failed to record branch graph migration during startup");
                }
            }
            Err(err) => {
                warn!(error = %err, "failed to catch up graph runtime during startup");
                if let Err(record_err) = branch_runtime
                    .record_branch_graph_catch_up_failure(&active_branch, &err.to_string())
                {
                    warn!(
                        error = %record_err,
                        "failed to record branch graph migration failure during startup"
                    );
                }
            }
        }

        Ok(Self {
            assembly,
            workspace_root,
            config_path,
            store_path,
            frame_storage_path,
            artifact_storage_path,
            branch_runtime,
            active_branch,
        })
    }

    /// Execute a CLI command via the single route table.
    pub fn execute(&self, command: &Commands) -> Result<String, ApiError> {
        let started = Instant::now();
        let command_name = command_name(command);
        let session_id = start_command_session(self.assembly.progress().as_ref(), &command_name)?;
        self.assembly
            .api()
            .set_progress_context(Arc::clone(self.assembly.progress()), session_id.clone());
        let mut live_progress = LiveProgressHandle::start_if_supported(
            Arc::clone(self.assembly.progress()),
            &session_id,
            command,
        );
        let result = self.execute_inner(command, &session_id);
        match self.assembly.graph_runtime().catch_up() {
            Ok(applied_events) => {
                let last_reduced_seq = match self
                    .assembly
                    .graph_runtime()
                    .traversal_store()
                    .last_reduced_seq()
                {
                    Ok(seq) => seq,
                    Err(err) => {
                        warn!(error = %err, "failed to read last reduced seq after command execution");
                        0
                    }
                };
                if applied_events > 0 {
                    if let Err(err) = self.branch_runtime.record_branch_graph_catch_up_success(
                        &self.active_branch,
                        last_reduced_seq,
                        applied_events,
                    ) {
                        warn!(error = %err, "failed to record branch graph migration after command execution");
                    }
                } else if let Err(err) =
                    self.branch_runtime.touch_active_branch(&self.active_branch)
                {
                    warn!(error = %err, "failed to update active branch last seen after command execution");
                }
            }
            Err(err) => {
                warn!(error = %err, "failed to catch up graph runtime after command execution");
                if let Err(record_err) = self
                    .branch_runtime
                    .record_branch_graph_catch_up_failure(&self.active_branch, &err.to_string())
                {
                    warn!(
                        error = %record_err,
                        "failed to record branch graph migration failure after command execution"
                    );
                }
            }
        }
        self.emit_command_summary(
            &session_id,
            command,
            result.as_ref(),
            started.elapsed().as_millis(),
        );
        let ok = result.is_ok();
        let err = result.as_ref().err().map(|e| e.to_string());
        finish_command_session(self.assembly.progress().as_ref(), &session_id, ok, err)?;
        self.assembly.api().clear_progress_context();
        if let Some(handle) = live_progress.as_mut() {
            handle.stop();
        }
        let _ = self.assembly.progress().prune(PrunePolicy::default());
        result
    }

    fn execute_inner(&self, command: &Commands, session_id: &str) -> Result<String, ApiError> {
        match command {
            Commands::Scan { force } => crate::workspace::tooling::handle_scan_command(
                self.assembly.api().as_ref(),
                &self.workspace_root,
                self.assembly.progress(),
                *force,
                session_id,
            ),
            Commands::Workspace { command } => crate::workspace::tooling::handle_cli_command(
                self.assembly.api().as_ref(),
                &self.workspace_root,
                &self.store_path,
                &self.frame_storage_path,
                command,
            ),
            Commands::Status {
                format,
                workspace_only,
                agents_only,
                providers_only,
                breakdown,
                test_connectivity,
            } => crate::workspace::tooling::handle_status_command(
                self.assembly.api().as_ref(),
                &self.workspace_root,
                &self.store_path,
                format,
                *workspace_only,
                *agents_only,
                *providers_only,
                *breakdown,
                *test_connectivity,
            ),
            Commands::Validate => crate::workspace::tooling::handle_validate_command(
                self.assembly.api().as_ref(),
                &self.workspace_root,
                &self.frame_storage_path,
            ),
            Commands::Agent { command } => {
                crate::agent::tooling::handle_cli_command(self.assembly.api().as_ref(), command)
            }
            Commands::Provider { command } => crate::provider::tooling::handle_cli_command(
                self.assembly.api().as_ref(),
                self.assembly.progress(),
                command,
                session_id,
            ),
            Commands::Init { force, list } => {
                crate::init::tooling::handle_cli_command(*force, *list)
            }
            Commands::Context { command } => crate::context::tooling::handle_cli_command(
                Arc::clone(self.assembly.api()),
                &self.workspace_root,
                &self.assembly.workflow_registry().read(),
                self.assembly.progress(),
                command,
                session_id,
            ),
            Commands::Workflow { command } => crate::workflow::tooling::handle_cli_command(
                self.assembly.api().as_ref(),
                &self.workspace_root,
                self.config_path.as_deref(),
                self.assembly.workflow_registry(),
                self.assembly.progress(),
                command,
                session_id,
            ),
            Commands::Branches { command } => crate::branches::tooling::handle_cli_command(command),
            Commands::Danger { .. } => Err(ApiError::ConfigError(
                "Danger commands must run from the CLI entry point".to_string(),
            )),
            Commands::Watch {
                debounce_ms,
                batch_window_ms,
                foreground: _,
            } => crate::workspace::tooling::handle_watch_command(
                Arc::clone(self.assembly.api()),
                &self.workspace_root,
                self.config_path.as_deref(),
                self.assembly.workflow_registry(),
                self.assembly.progress(),
                *debounce_ms,
                *batch_window_ms,
                session_id,
            ),
        }
    }

    fn emit_command_summary(
        &self,
        session_id: &str,
        command: &Commands,
        result: Result<&String, &ApiError>,
        duration_ms: u128,
    ) {
        let ok = result.is_ok();
        let error = result.as_ref().err().map(|err| err.to_string());
        let (message, output_chars, error_chars, truncated) = match result {
            Ok(output) => (None, Some(output.chars().count()), None, None),
            Err(_) => {
                let error_text = error
                    .clone()
                    .unwrap_or_else(|| "command failed".to_string());
                let error_chars = error_text.chars().count();
                let (preview, was_truncated) = truncate_for_summary(&error_text);
                (Some(preview), None, Some(error_chars), Some(was_truncated))
            }
        };
        let typed_summary = typed_summary_event(command, ok, duration_ms, error.as_deref());
        emit_command_summary(
            self.assembly.progress().as_ref(),
            session_id,
            &command_name(command),
            typed_summary,
            ok,
            duration_ms,
            message,
            output_chars,
            error_chars,
            truncated,
        );
    }
}
