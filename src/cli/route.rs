//! CLI route: shared runtime context and top-level command dispatch only.

use crate::branches::{BranchHandle, BranchRuntime};
use crate::cli::parse::{Commands, WorldCommands};
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

    /// Durable identity-bearing cursor of the shared graph runtime.
    pub fn graph_event_cursor(&self) -> Result<meld_events::LedgerCursor, ApiError> {
        self.assembly
            .graph_runtime()
            .durable_event_cursor()
            .map_err(ApiError::from)
    }

    /// Bounded replay capability for the product-bound event authority.
    pub fn event_replay_capability(&self) -> meld_events::EventReplayCapability {
        self.assembly
            .product_runtime()
            .event_authority()
            .replay_capability()
    }

    /// Durable watermark capability for the product-bound event authority.
    pub fn event_watermark_capability(&self) -> meld_events::EventWatermarkCapability {
        self.assembly
            .product_runtime()
            .event_authority()
            .watermark_capability()
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
        // Reject invalid product storage before branch registration or any
        // other startup metadata write.
        config
            .system
            .storage
            .resolve_product_root(&workspace_root)?;
        let branch_runtime = BranchRuntime::new();
        let active_branch = branch_runtime.resolve_active_branch(&workspace_root)?;
        if let Err(err) = branch_runtime.ensure_active_branch_registered(&active_branch) {
            warn!(error = %err, "failed to register active branch during startup");
        }

        let assembly =
            CliRuntimeAssembly::load(&workspace_root, &config, active_branch.resolved())?;
        let store_path = assembly.legacy_store_path().to_path_buf();
        let frame_storage_path = assembly.frame_storage_path().to_path_buf();
        let artifact_storage_path = assembly.artifact_storage_path().to_path_buf();

        // No hidden graph catch-up here: catch-up is supervised actor work
        // (Runtime Initialization stage 5), not a command-startup side
        // effect. Callers that need the projection advanced use the
        // explicit catch_up_graph_projection path.
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

    /// Explicitly advance the shared graph projection to the ledger tip.
    ///
    /// This is the explicit path replacing the retired hidden catch-up
    /// around command routing: catch-up is supervised actor work under the
    /// runtime, and interactive callers that need the projection current
    /// without booting the supervisor invoke it deliberately. Branch graph
    /// state is recorded exactly as the supervised path records it.
    pub fn catch_up_graph_projection(&self) -> Result<usize, ApiError> {
        // Queued best-effort emissions must be durable before the reducer
        // reads, so the caller's own events are visible to this pass.
        if let Err(err) = self.assembly.progress().barrier() {
            warn!(error = %err, "failed to drain ledger writer before graph catch-up");
        }
        match self.assembly.graph_runtime().catch_up() {
            Ok(applied_events) => {
                let last_reduced_seq = match self.assembly.graph_runtime().durable_event_cursor() {
                    Ok(cursor) => cursor.after_seq,
                    Err(err) => {
                        warn!(error = %err, "failed to read last reduced seq after graph catch-up");
                        0
                    }
                };
                if let Err(err) = self.branch_runtime.record_branch_graph_catch_up_success(
                    &self.active_branch,
                    &self.assembly.product_runtime().layout().world_model_db,
                    last_reduced_seq,
                    applied_events,
                ) {
                    warn!(error = %err, "failed to record branch graph migration after catch-up");
                }
                Ok(applied_events)
            }
            Err(err) => {
                if let Err(record_err) = self.branch_runtime.record_branch_graph_catch_up_failure(
                    &self.active_branch,
                    &self.assembly.product_runtime().layout().world_model_db,
                    &err.to_string(),
                ) {
                    warn!(
                        error = %record_err,
                        "failed to record branch graph migration failure after catch-up"
                    );
                }
                Err(ApiError::from(err))
            }
        }
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
            self.event_replay_capability(),
            &session_id,
            command,
        );
        let result = self.execute_inner(command, &session_id);
        // No hidden graph catch-up after the command: catch-up is
        // supervised actor work, never a command-routing side effect. Only
        // the branch last-seen touch survives from the removed pass.
        if let Err(err) = self.branch_runtime.touch_active_branch(&self.active_branch) {
            warn!(error = %err, "failed to update active branch last seen after command execution");
        }
        self.emit_command_summary(
            &session_id,
            command,
            result.as_ref(),
            started.elapsed().as_millis(),
        );
        let ok = result.is_ok();
        let err = result.as_ref().err().map(|e| e.to_string());
        // The durable session_ended emit must stay the last emission of the
        // command: queue order means its ack proves every earlier
        // best-effort event survived, even when exit paths skip Drop.
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
            Commands::Runtime { command } => crate::runtime::tooling::handle_cli_command(
                self.assembly.product_runtime().as_ref(),
                command,
            ),
            Commands::World {
                command:
                    WorldCommands::Init {
                        path,
                        stages,
                        format,
                    },
            } => {
                // Format problems must surface before any stage runs, even
                // though the stages are idempotent.
                crate::cli::presentation::validate_world_init_format(format)?;
                let report = crate::init::world::tooling::run_world_init(
                    self.assembly.product_runtime().as_ref(),
                    path,
                    stages,
                    session_id,
                )?;
                crate::cli::presentation::format_world_init_report(&report, format)
            }
            Commands::Event { command } => {
                let authority = self.assembly.product_runtime().event_authority();
                crate::events::tooling::handle_cli_command(
                    &authority.observability_capability(),
                    &authority.replay_capability(),
                    &authority.subscription_capability(),
                    self.assembly.progress(),
                    command,
                )
            }
            Commands::Branches { command } => {
                let graph_runtime = self.assembly.graph_runtime();
                crate::branches::tooling::handle_cli_command_with_active_store(
                    command,
                    Some(&self.workspace_root),
                    Some((
                        self.active_branch.branch_id(),
                        graph_runtime.traversal_store().clone(),
                    )),
                )
            }
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
