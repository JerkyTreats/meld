//! CLI route: shared runtime context and top-level command dispatch only.

use crate::api::ContextApi;
use crate::branches::{BranchHandle, BranchRuntime};
use crate::cli::parse::Commands;
use crate::cli::progress::LiveProgressHandle;
use crate::cli::session::{finish_command_session, start_command_session};
use crate::cli::{command_name, typed_summary_event};
use crate::config::ConfigLoader;
use crate::error::ApiError;
use crate::heads::HeadIndex;
use crate::session::PrunePolicy;
use crate::store::persistence::SledNodeRecordStore;
use crate::telemetry::emission::{emit_command_summary, truncate_for_summary};
use crate::telemetry::ProgressRuntime;
use crate::world_state::GraphRuntime;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::warn;

/// Runtime context for CLI execution: workspace, config paths, and domain facades.
/// Built from workspace path and optional config path using ConfigLoader only.
pub struct RunContext {
    api: Arc<ContextApi>,
    workspace_root: PathBuf,
    config_path: Option<PathBuf>,
    #[allow(dead_code)]
    store_path: PathBuf,
    frame_storage_path: PathBuf,
    #[allow(dead_code)]
    artifact_storage_path: PathBuf,
    workflow_registry: Arc<parking_lot::RwLock<crate::workflow::registry::WorkflowRegistry>>,
    progress: Arc<ProgressRuntime>,
    graph_runtime: Arc<GraphRuntime>,
    branch_runtime: BranchRuntime,
    active_branch: BranchHandle,
}

impl RunContext {
    /// Reference to the underlying context API.
    pub fn api(&self) -> &ContextApi {
        &self.api
    }

    /// Progress runtime for session and event emission.
    pub fn progress_runtime(&self) -> Arc<ProgressRuntime> {
        Arc::clone(&self.progress)
    }

    /// Workflow profile registry.
    pub fn workflow_registry(
        &self,
    ) -> Arc<parking_lot::RwLock<crate::workflow::registry::WorkflowRegistry>> {
        Arc::clone(&self.workflow_registry)
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
        let workflow_registry =
            crate::workflow::registry::WorkflowRegistry::load(&config.workflows)?;

        std::fs::create_dir_all(&store_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;

        let db = sled::open(&store_path).map_err(|e| {
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                format!("Failed to open sled database: {}", e),
            )))
        })?;
        let node_store = Arc::new(SledNodeRecordStore::from_db(db.clone()));
        let progress = Arc::new(ProgressRuntime::new(db.clone()).map_err(ApiError::StorageError)?);
        let graph_runtime = Arc::new(GraphRuntime::new(db).map_err(ApiError::StorageError)?);

        std::fs::create_dir_all(&frame_storage_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;
        std::fs::create_dir_all(&artifact_storage_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;
        let frame_storage = Arc::new(
            crate::context::frame::open_storage(&frame_storage_path)
                .map_err(ApiError::StorageError)?,
        );
        let prompt_context_storage = Arc::new(
            crate::prompt_context::PromptContextArtifactStorage::new(&artifact_storage_path)
                .map_err(ApiError::StorageError)?,
        );
        let head_index_path = HeadIndex::persistence_path(&workspace_root);
        let head_index = Arc::new(parking_lot::RwLock::new(
            HeadIndex::load_from_disk(&head_index_path).unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to load head index from disk: {}, starting with empty index",
                    e
                );
                HeadIndex::new()
            }),
        ));

        let mut agent_registry = crate::agent::AgentRegistry::new();
        agent_registry.load_from_config(&config)?;
        agent_registry.load_from_xdg()?;

        let mut provider_registry = crate::provider::ProviderRegistry::new();
        provider_registry.load_from_config(&config)?;
        provider_registry.load_from_xdg()?;

        for agent in agent_registry.list_all() {
            crate::workflow::binding::validate_agent_binding(agent, &workflow_registry)?;
        }

        let agent_registry = Arc::new(parking_lot::RwLock::new(agent_registry));
        let provider_registry = Arc::new(parking_lot::RwLock::new(provider_registry));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());
        let workflow_registry = Arc::new(parking_lot::RwLock::new(workflow_registry));

        let api = ContextApi::with_workspace_root(
            node_store,
            frame_storage,
            head_index,
            prompt_context_storage,
            agent_registry,
            provider_registry,
            lock_manager,
            workspace_root.clone(),
        );
        api.set_graph_runtime(Arc::clone(&graph_runtime));

        let (store_path, frame_storage_path, artifact_storage_path) =
            config.system.storage.resolve_paths(&workspace_root)?;

        match graph_runtime.catch_up() {
            Ok(applied_events) => {
                let last_reduced_seq = match graph_runtime.traversal_store().last_reduced_seq() {
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
            api: Arc::new(api),
            workspace_root,
            config_path,
            store_path,
            frame_storage_path,
            artifact_storage_path,
            workflow_registry,
            progress,
            graph_runtime,
            branch_runtime,
            active_branch,
        })
    }

    /// Execute a CLI command via the single route table.
    pub fn execute(&self, command: &Commands) -> Result<String, ApiError> {
        let started = Instant::now();
        let command_name = command_name(command);
        let session_id = start_command_session(&self.progress, &command_name)?;
        self.api
            .set_progress_context(Arc::clone(&self.progress), session_id.clone());
        let mut live_progress = LiveProgressHandle::start_if_supported(
            Arc::clone(&self.progress),
            &session_id,
            command,
        );
        let result = self.execute_inner(command, &session_id);
        match self.graph_runtime.catch_up() {
            Ok(applied_events) => {
                let last_reduced_seq = match self.graph_runtime.traversal_store().last_reduced_seq()
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
        finish_command_session(&self.progress, &session_id, ok, err)?;
        self.api.clear_progress_context();
        if let Some(handle) = live_progress.as_mut() {
            handle.stop();
        }
        let _ = self.progress.prune(PrunePolicy::default());
        result
    }

    fn execute_inner(&self, command: &Commands, session_id: &str) -> Result<String, ApiError> {
        match command {
            Commands::Scan { force } => crate::workspace::tooling::handle_scan_command(
                self.api.as_ref(),
                &self.workspace_root,
                &self.progress,
                *force,
                session_id,
            ),
            Commands::Workspace { command } => crate::workspace::tooling::handle_cli_command(
                self.api.as_ref(),
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
                self.api.as_ref(),
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
                self.api.as_ref(),
                &self.workspace_root,
                &self.frame_storage_path,
            ),
            Commands::Agent { command } => {
                crate::agent::tooling::handle_cli_command(self.api.as_ref(), command)
            }
            Commands::Provider { command } => crate::provider::tooling::handle_cli_command(
                self.api.as_ref(),
                &self.progress,
                command,
                session_id,
            ),
            Commands::Init { force, list } => {
                crate::init::tooling::handle_cli_command(*force, *list)
            }
            Commands::Context { command } => crate::context::tooling::handle_cli_command(
                Arc::clone(&self.api),
                &self.workspace_root,
                &self.workflow_registry.read(),
                &self.progress,
                command,
                session_id,
            ),
            Commands::Workflow { command } => crate::workflow::tooling::handle_cli_command(
                self.api.as_ref(),
                &self.workspace_root,
                self.config_path.as_deref(),
                &self.workflow_registry,
                &self.progress,
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
                Arc::clone(&self.api),
                &self.workspace_root,
                self.config_path.as_deref(),
                &self.workflow_registry,
                &self.progress,
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
            self.progress.as_ref(),
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
