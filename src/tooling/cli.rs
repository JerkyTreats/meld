//! CLI Tooling
//!
//! Command-line interface for all Merkle operations. Provides workspace-scoped
//! operations with idempotent execution.

use crate::agent::AgentStorage;
use crate::api::{ContextApi, ContextView};
use crate::config::ConfigLoader;
use crate::error::ApiError;
use crate::workspace::{
    format_unified_status_text, format_workspace_status_text, IgnoreResult, ListDeletedResult,
    ValidateResult, WorkspaceCommandService, WorkspaceStatusRequest,
};
use crate::context::generation::{
    FailurePolicy, GenerationExecutor, GenerationItem, GenerationNodeType, GenerationPlan,
    PlanPriority,
};
use crate::context::queue::{FrameGenerationQueue, GenerationConfig, QueueEventContext};
use crate::heads::HeadIndex;
use crate::ignore;
use crate::store::persistence::SledNodeRecordStore;
use crate::store::{NodeRecord, NodeRecordStore};
use crate::telemetry::emission::{
    emit_command_summary as telemetry_emit_command_summary, truncate_for_summary,
    SummaryCommandDescriptor,
};
use crate::telemetry::{ProgressRuntime, ProviderLifecycleEventData, PrunePolicy};
use crate::tree::builder::TreeBuilder;
use crate::tree::walker::WalkerConfig;
use crate::types::{Hash, NodeID};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

use hex;

/// Merkle CLI - Deterministic filesystem state management
#[derive(Parser)]
#[command(name = "merkle")]
#[command(about = "Deterministic filesystem state management using Merkle trees")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Workspace root directory
    #[arg(long, default_value = ".")]
    pub workspace: PathBuf,

    /// Configuration file path (overrides default config loading)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Enable verbose logging (default: off)
    #[arg(long, default_value = "false")]
    pub verbose: bool,

    /// Log level (trace, debug, info, warn, error, off)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Log format (json, text)
    #[arg(long)]
    pub log_format: Option<String>,

    /// Log output (stdout, stderr, file, both)
    #[arg(long)]
    pub log_output: Option<String>,

    /// Log file path (if output includes "file")
    #[arg(long)]
    pub log_file: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan filesystem and rebuild tree
    Scan {
        /// Force rebuild even if tree exists
        #[arg(long)]
        force: bool,
    },
    /// Workspace commands (status, validate)
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommands,
    },
    /// Show unified status (workspace, agents, providers)
    Status {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
        /// Show only workspace section
        #[arg(long)]
        workspace_only: bool,
        /// Show only agents section
        #[arg(long)]
        agents_only: bool,
        /// Show only providers section
        #[arg(long)]
        providers_only: bool,
        /// Include top-level path breakdown in workspace section
        #[arg(long)]
        breakdown: bool,
        /// Test provider connectivity
        #[arg(long)]
        test_connectivity: bool,
    },
    /// Validate workspace integrity
    Validate,
    /// Start watch mode daemon
    Watch {
        /// Debounce window in milliseconds
        #[arg(long, default_value = "100")]
        debounce_ms: u64,
        /// Batch window in milliseconds
        #[arg(long, default_value = "50")]
        batch_window_ms: u64,
        /// Run in foreground (default: background daemon)
        #[arg(long)]
        foreground: bool,
    },
    /// Manage agents
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
    /// Manage providers
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },
    /// Initialize default agents and prompts
    Init {
        /// Force re-initialization (overwrite existing)
        #[arg(long)]
        force: bool,

        /// List what would be initialized without creating
        #[arg(long)]
        list: bool,
    },
    /// Context operations (generate and retrieve frames)
    Context {
        #[command(subcommand)]
        command: ContextCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    /// Show workspace status (tree, context coverage, top paths)
    Status {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
        /// Include top-level path breakdown
        #[arg(long)]
        breakdown: bool,
    },
    /// Validate workspace integrity
    Validate {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// List or add paths to the workspace ignore list
    Ignore {
        /// Path to add (omit to list current ignore list)
        path: Option<PathBuf>,
        /// When adding, report what would be added without writing
        #[arg(long)]
        dry_run: bool,
        /// Output format for list mode (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Tombstone a node and its descendants (logical delete; reversible with restore)
    Delete {
        /// Path to file or directory to delete
        path: Option<PathBuf>,
        /// Node ID (hex) instead of path
        #[arg(long)]
        node: Option<String>,
        /// Report counts without performing the operation
        #[arg(long)]
        dry_run: bool,
        /// Do not add the path to the workspace ignore list
        #[arg(long)]
        no_ignore: bool,
    },
    /// Restore a tombstoned node and its descendants
    Restore {
        /// Path to file or directory to restore
        path: Option<PathBuf>,
        /// Node ID (hex) instead of path
        #[arg(long)]
        node: Option<String>,
        /// Report counts without performing the operation
        #[arg(long)]
        dry_run: bool,
    },
    /// Purge tombstoned records older than TTL
    Compact {
        /// Tombstone age threshold in days (default: 90)
        #[arg(long)]
        ttl: Option<u64>,
        /// Purge all tombstoned records regardless of age
        #[arg(long)]
        all: bool,
        /// Do not purge frame blobs; only purge node and head index records
        #[arg(long)]
        keep_frames: bool,
        /// Report counts without performing compaction
        #[arg(long)]
        dry_run: bool,
    },
    /// List tombstoned (deleted) nodes
    ListDeleted {
        /// Show only nodes tombstoned longer than this many days
        #[arg(long)]
        older_than: Option<u64>,
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Show agent status (validation and prompt path)
    Status {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// List all agents
    List {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
        /// Filter by role (Reader or Writer)
        #[arg(long)]
        role: Option<String>,
    },
    /// Show agent details
    Show {
        /// Agent ID
        agent_id: String,
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
        /// Include prompt file content in output
        #[arg(long)]
        include_prompt: bool,
    },
    /// Validate agent configuration
    Validate {
        /// Agent ID (required unless --all is used)
        #[arg(required_unless_present = "all")]
        agent_id: Option<String>,
        /// Validate all agents
        #[arg(long, conflicts_with = "agent_id")]
        all: bool,
        /// Show detailed validation results
        #[arg(long)]
        verbose: bool,
    },
    /// Create new agent
    Create {
        /// Agent ID
        agent_id: String,
        /// Agent role (Reader or Writer)
        #[arg(long)]
        role: Option<String>,
        /// Path to prompt file (required for Writer)
        #[arg(long)]
        prompt_path: Option<String>,
        /// Use interactive mode (default)
        #[arg(long)]
        interactive: bool,
        /// Use non-interactive mode (use flags)
        #[arg(long)]
        non_interactive: bool,
    },
    /// Edit agent configuration
    Edit {
        /// Agent ID
        agent_id: String,
        /// Update prompt file path
        #[arg(long)]
        prompt_path: Option<String>,
        /// Update agent role
        #[arg(long)]
        role: Option<String>,
        /// Editor to use (default: $EDITOR)
        #[arg(long)]
        editor: Option<String>,
    },
    /// Remove agent
    Remove {
        /// Agent ID
        agent_id: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum ProviderCommands {
    /// Show provider status (optional connectivity)
    Status {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
        /// Test connectivity per provider (may be slow)
        #[arg(long)]
        test_connectivity: bool,
    },
    /// List all providers
    List {
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
        /// Filter by provider type (openai, anthropic, ollama, local)
        #[arg(long)]
        type_filter: Option<String>,
    },
    /// Show provider details
    Show {
        /// Provider name
        provider_name: String,
        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
        /// Show API key status
        #[arg(long)]
        include_credentials: bool,
    },
    /// Validate provider configuration
    Validate {
        /// Provider name
        provider_name: String,
        /// Test provider API connectivity
        #[arg(long)]
        test_connectivity: bool,
        /// Verify model is available
        #[arg(long)]
        check_model: bool,
        /// Show detailed validation results
        #[arg(long)]
        verbose: bool,
    },
    /// Test provider connectivity
    Test {
        /// Provider name
        provider_name: String,
        /// Test specific model (overrides config)
        #[arg(long)]
        model: Option<String>,
        /// Connection timeout in seconds (default: 10)
        #[arg(long, default_value = "10")]
        timeout: u64,
    },
    /// Create new provider
    Create {
        /// Provider name
        provider_name: String,
        /// Provider type (openai, anthropic, ollama, local)
        #[arg(long)]
        type_: Option<String>,
        /// Model name
        #[arg(long)]
        model: Option<String>,
        /// Endpoint URL
        #[arg(long)]
        endpoint: Option<String>,
        /// API key
        #[arg(long)]
        api_key: Option<String>,
        /// Use interactive mode (default)
        #[arg(long)]
        interactive: bool,
        /// Use non-interactive mode (use flags)
        #[arg(long)]
        non_interactive: bool,
    },
    /// Edit provider configuration
    Edit {
        /// Provider name
        provider_name: String,
        /// Update model name
        #[arg(long)]
        model: Option<String>,
        /// Update endpoint URL
        #[arg(long)]
        endpoint: Option<String>,
        /// Update API key
        #[arg(long)]
        api_key: Option<String>,
        /// Editor to use (default: $EDITOR)
        #[arg(long)]
        editor: Option<String>,
    },
    /// Remove provider
    Remove {
        /// Provider name
        provider_name: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum ContextCommands {
    /// Generate context frame for a node
    Generate {
        /// Target node by NodeID (hex string)
        #[arg(long, conflicts_with_all = ["path", "path_positional"])]
        node: Option<String>,

        /// Target node by workspace-relative or absolute path
        #[arg(long, value_name = "PATH", conflicts_with = "node")]
        path: Option<PathBuf>,

        /// Target path (positional; same as --path)
        #[arg(value_name = "PATH", index = 1, conflicts_with = "node")]
        path_positional: Option<PathBuf>,

        /// Agent to use for generation
        #[arg(long)]
        agent: Option<String>,

        /// Provider to use for generation (required)
        #[arg(long)]
        provider: Option<String>,

        /// Frame type (defaults to context-<agent_id>)
        #[arg(long)]
        frame_type: Option<String>,

        /// Generate even if head frame exists
        #[arg(long)]
        force: bool,
        /// Disable recursive generation for directory targets
        #[arg(long)]
        no_recursive: bool,
    },
    /// Retrieve context frames for a node
    Get {
        /// Target node by NodeID (hex string)
        #[arg(long, conflicts_with = "path")]
        node: Option<String>,

        /// Target node by workspace-relative or absolute path
        #[arg(long, conflicts_with = "node")]
        path: Option<PathBuf>,

        /// Filter by agent ID
        #[arg(long)]
        agent: Option<String>,

        /// Filter by frame type
        #[arg(long)]
        frame_type: Option<String>,

        /// Maximum frames to return
        #[arg(long, default_value = "10")]
        max_frames: usize,

        /// Ordering policy: recency or deterministic
        #[arg(long, default_value = "recency")]
        ordering: String,

        /// Concatenate frame contents with separator
        #[arg(long)]
        combine: bool,

        /// Separator used with --combine
        #[arg(long, default_value = "\n\n---\n\n")]
        separator: String,

        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,

        /// Include metadata fields in output
        #[arg(long)]
        include_metadata: bool,

        /// Include frames marked deleted (tombstones)
        #[arg(long)]
        include_deleted: bool,
    },
}

/// CLI context for managing workspace state
pub struct CliContext {
    api: Arc<ContextApi>,
    workspace_root: PathBuf,
    config_path: Option<PathBuf>,
    #[allow(dead_code)] // May be used for debugging or future features
    store_path: PathBuf,
    frame_storage_path: PathBuf,
    progress: Arc<ProgressRuntime>,
    /// Optional generation queue (initialized on demand for context generate commands)
    #[allow(dead_code)] // Queue is created on demand, not stored
    queue: Option<Arc<FrameGenerationQueue>>,
}

impl CliContext {
    /// Get a reference to the underlying API
    pub fn api(&self) -> &ContextApi {
        &self.api
    }

    /// Get a handle to the progress runtime.
    pub fn progress_runtime(&self) -> Arc<ProgressRuntime> {
        Arc::clone(&self.progress)
    }

    /// Create a new CLI context
    pub fn new(workspace_root: PathBuf, config_path: Option<PathBuf>) -> Result<Self, ApiError> {
        // Load config to get storage paths
        let config = if let Some(cfg_path) = &config_path {
            crate::config::ConfigLoader::load_from_file(cfg_path)?
        } else {
            crate::config::ConfigLoader::load(&workspace_root)?
        };

        // Resolve storage paths (will use XDG directories for default paths)
        let (store_path, frame_storage_path) =
            config.system.storage.resolve_paths(&workspace_root)?;

        // Initialize storage
        std::fs::create_dir_all(&store_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;

        let db = sled::open(&store_path).map_err(|e| {
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to open sled database: {}", e),
            )))
        })?;
        let node_store = Arc::new(SledNodeRecordStore::from_db(db.clone()));
        let progress = Arc::new(ProgressRuntime::new(db).map_err(ApiError::StorageError)?);

        std::fs::create_dir_all(&frame_storage_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;
        let frame_storage = Arc::new(
            crate::context::frame::storage::FrameStorage::new(&frame_storage_path)
                .map_err(|e| ApiError::StorageError(e))?,
        );
        // Load head index from disk, or create empty if not found
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

        // Load agents and providers from config.toml first, then XDG (XDG overrides)
        let mut agent_registry = crate::agent::AgentRegistry::new();
        agent_registry.load_from_config(&config)?;
        agent_registry.load_from_xdg()?; // XDG agents override config.toml agents

        let mut provider_registry = crate::provider::ProviderRegistry::new();
        provider_registry.load_from_config(&config)?;
        provider_registry.load_from_xdg()?; // XDG providers override config.toml providers

        let agent_registry = Arc::new(parking_lot::RwLock::new(agent_registry));
        let provider_registry = Arc::new(parking_lot::RwLock::new(provider_registry));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());

        let api = ContextApi::with_workspace_root(
            node_store,
            frame_storage,
            head_index,
            agent_registry,
            provider_registry,
            lock_manager,
            workspace_root.clone(),
        );

        // Store resolved storage paths for later use
        let (store_path, frame_storage_path) =
            config.system.storage.resolve_paths(&workspace_root)?;

        Ok(Self {
            api: Arc::new(api),
            workspace_root,
            config_path,
            store_path,
            frame_storage_path,
            progress,
            queue: None, // Initialize on demand for context generate commands
        })
    }

    /// Get or create the generation queue
    ///
    /// The queue is initialized lazily when needed for context generation commands.
    /// Creates a new queue each time (it's cheap to create, workers are started on first use).
    fn get_or_create_queue(
        &self,
        session_id: Option<&str>,
    ) -> Result<Arc<FrameGenerationQueue>, ApiError> {
        let gen_config = GenerationConfig::default();
        let event_context = session_id.map(|session| QueueEventContext {
            session_id: session.to_string(),
            progress: Arc::clone(&self.progress),
        });
        let queue = Arc::new(FrameGenerationQueue::with_event_context(
            Arc::clone(&self.api),
            gen_config,
            event_context,
        ));

        // Start the queue workers
        queue.start()?;

        Ok(queue)
    }

    /// Execute a CLI command
    pub fn execute(&self, command: &Commands) -> Result<String, ApiError> {
        let started = Instant::now();
        let session_id = self.progress.start_command_session(command_name(command))?;
        let result = self.execute_inner(command, &session_id);
        self.emit_command_summary(
            &session_id,
            command,
            result.as_ref(),
            started.elapsed().as_millis(),
        );
        let (ok, err) = match &result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        self.progress.finish_command_session(&session_id, ok, err)?;
        // Best-effort hygiene so stale completed sessions do not grow unbounded.
        let _ = self.progress.prune(PrunePolicy::default());
        result
    }

    fn execute_inner(&self, command: &Commands, session_id: &str) -> Result<String, ApiError> {
        match command {
            Commands::Scan { force } => {
                self.progress.emit_event_best_effort(
                    session_id,
                    "scan_started",
                    json!({ "force": force }),
                );
                let scan_started = Instant::now();
                // Load ignore patterns (built-in + .gitignore + ignore_list)
                let ignore_patterns = ignore::load_ignore_patterns(&self.workspace_root)
                    .unwrap_or_else(|_| WalkerConfig::default().ignore_patterns);
                let walker_config = WalkerConfig {
                    follow_symlinks: false,
                    ignore_patterns,
                    max_depth: None,
                };
                let builder =
                    TreeBuilder::new(self.workspace_root.clone()).with_walker_config(walker_config);
                let tree = builder.build().map_err(|e| ApiError::StorageError(e))?;
                let total_nodes = tree.nodes.len();

                // If force is false, check if root node already exists
                if !force {
                    if self
                        .api
                        .node_store()
                        .get(&tree.root_id)
                        .map_err(ApiError::from)?
                        .is_some()
                    {
                        self.progress.emit_event_best_effort(
                            session_id,
                            "scan_progress",
                            json!({
                                "node_count": total_nodes,
                                "total_nodes": total_nodes
                            }),
                        );
                        let root_hex = hex::encode(tree.root_id);
                        return Ok(format!(
                            "Tree already exists (root: {}). Use --force to rebuild.",
                            root_hex
                        ));
                    }
                }

                // Populate store with all nodes from tree
                let store = self.api.node_store().as_ref() as &dyn NodeRecordStore;
                const SCAN_PROGRESS_BATCH_NODES: usize = 128;
                let mut processed_nodes = 0usize;
                for (node_id, node) in &tree.nodes {
                    let record = NodeRecord::from_merkle_node(*node_id, node, &tree)
                        .map_err(ApiError::StorageError)?;
                    store.put(&record).map_err(ApiError::from)?;
                    processed_nodes += 1;
                    if processed_nodes % SCAN_PROGRESS_BATCH_NODES == 0
                        || processed_nodes == total_nodes
                    {
                        self.progress.emit_event_best_effort(
                            session_id,
                            "scan_progress",
                            json!({
                                "node_count": processed_nodes,
                                "total_nodes": total_nodes
                            }),
                        );
                    }
                }
                if total_nodes == 0 {
                    self.progress.emit_event_best_effort(
                        session_id,
                        "scan_progress",
                        json!({
                            "node_count": 0,
                            "total_nodes": 0
                        }),
                    );
                }
                store.flush().map_err(|e| ApiError::StorageError(e))?;

                // When .gitignore node hash changed, sync it into ignore_list
                let _ = ignore::maybe_sync_gitignore_after_tree(
                    &self.workspace_root,
                    tree.find_gitignore_node_id().as_ref(),
                );

                let root_hex = hex::encode(tree.root_id);
                self.progress.emit_event_best_effort(
                    session_id,
                    "scan_completed",
                    json!({
                        "force": force,
                        "node_count": total_nodes,
                        "duration_ms": scan_started.elapsed().as_millis(),
                    }),
                );
                Ok(format!(
                    "Scanned {} nodes (root: {})",
                    total_nodes, root_hex
                ))
            }
            Commands::Workspace { command } => self.handle_workspace_command(command),
            Commands::Status {
                format,
                workspace_only,
                agents_only,
                providers_only,
                breakdown,
                test_connectivity,
            } => {
                let include_all =
                    !*workspace_only && !*agents_only && !*providers_only;
                let include_workspace = include_all || *workspace_only;
                let include_agents = include_all || *agents_only;
                let include_providers = include_all || *providers_only;
                let registry_agent = self.api.agent_registry().read();
                let registry_provider = self.api.provider_registry().read();
                let unified = WorkspaceCommandService::unified_status(
                    self.api.as_ref(),
                    self.workspace_root.as_path(),
                    self.store_path.as_path(),
                    &registry_agent,
                    &registry_provider,
                    include_workspace,
                    include_agents,
                    include_providers,
                    *breakdown,
                    *test_connectivity,
                )?;
                if format == "json" {
                    serde_json::to_string_pretty(&unified).map_err(|e| {
                        ApiError::StorageError(crate::error::StorageError::InvalidPath(
                            e.to_string(),
                        ))
                    })
                } else {
                    Ok(format_unified_status_text(
                        &unified,
                        *breakdown,
                        *test_connectivity,
                    ))
                }
            }
            Commands::Validate => {
                let result = WorkspaceCommandService::validate(
                    self.api.as_ref(),
                    &self.workspace_root,
                    &self.frame_storage_path,
                )?;
                Ok(format_validate_result_text(&result))
            }
            Commands::Agent { command } => self.handle_agent_command(command),
            Commands::Provider { command } => self.handle_provider_command(command, session_id),
            Commands::Init { force, list } => self.handle_init(*force, *list),
            Commands::Context { command } => self.handle_context_command(command, session_id),
            Commands::Watch {
                debounce_ms,
                batch_window_ms,
                foreground: _,
            } => {
                use crate::workspace::{WatchConfig, WatchDaemon};

                // Load configuration to register agents
                let config = if let Some(ref config_path) = self.config_path {
                    // Load from specified config file
                    ConfigLoader::load_from_file(config_path).map_err(|e| {
                        ApiError::ConfigError(format!(
                            "Failed to load config from {}: {}",
                            config_path.display(),
                            e
                        ))
                    })?
                } else {
                    // Load from default locations
                    ConfigLoader::load(&self.workspace_root).map_err(|e| {
                        ApiError::ConfigError(format!("Failed to load config: {}", e))
                    })?
                };

                // Load agents from config into registry
                {
                    let mut registry = self.api.agent_registry().write();
                    registry.load_from_config(&config).map_err(|e| {
                        ApiError::ConfigError(format!("Failed to load agents from config: {}", e))
                    })?;
                }

                // Load ignore patterns (same sources as scan: built-in + .gitignore + ignore_list)
                let ignore_patterns = ignore::load_ignore_patterns(&self.workspace_root)
                    .unwrap_or_else(|_| WalkerConfig::default().ignore_patterns);

                // Build watch config
                let mut watch_config = WatchConfig::default();
                watch_config.workspace_root = self.workspace_root.clone();
                watch_config.debounce_ms = *debounce_ms;
                watch_config.batch_window_ms = *batch_window_ms;
                watch_config.ignore_patterns = ignore_patterns;
                watch_config.session_id = Some(session_id.to_string());
                watch_config.progress = Some(self.progress.clone());

                // Create watch daemon
                let daemon = WatchDaemon::new(self.api.clone(), watch_config)?;

                // Start daemon (this will block)
                info!("Starting watch mode daemon");
                daemon.start()?;

                Ok("Watch daemon stopped".to_string())
            }
        }
    }

    /// Handle workspace subcommands
    fn handle_workspace_command(
        &self,
        command: &WorkspaceCommands,
    ) -> Result<String, ApiError> {
        match command {
            WorkspaceCommands::Status { format, breakdown } => {
                let registry = self.api.agent_registry().read();
                let request = WorkspaceStatusRequest {
                    workspace_root: self.workspace_root.clone(),
                    store_path: self.store_path.clone(),
                    include_breakdown: *breakdown,
                };
                let status = WorkspaceCommandService::status(
                    self.api.as_ref(),
                    &request,
                    &registry,
                )?;
                if format == "json" {
                    serde_json::to_string_pretty(&status).map_err(|e| {
                        ApiError::StorageError(crate::error::StorageError::InvalidPath(
                            e.to_string(),
                        ))
                    })
                } else {
                    Ok(format_workspace_status_text(&status, request.include_breakdown))
                }
            }
            WorkspaceCommands::Validate { format } => {
                let result = WorkspaceCommandService::validate(
                    self.api.as_ref(),
                    &self.workspace_root,
                    &self.frame_storage_path,
                )?;
                if format == "json" {
                    serde_json::to_string_pretty(&result).map_err(|e| {
                        ApiError::StorageError(crate::error::StorageError::InvalidPath(
                            e.to_string(),
                        ))
                    })
                } else {
                    Ok(format_validate_result_text(&result))
                }
            }
            WorkspaceCommands::Ignore {
                path,
                dry_run,
                format,
            } => {
                let result = WorkspaceCommandService::ignore(
                    &self.workspace_root,
                    path.as_deref(),
                    *dry_run,
                )?;
                format_ignore_result(&result, format)
            }
            WorkspaceCommands::Delete {
                path,
                node,
                dry_run,
                no_ignore,
            } => WorkspaceCommandService::delete(
                self.api.as_ref(),
                &self.workspace_root,
                path.as_deref(),
                node.as_deref(),
                *dry_run,
                *no_ignore,
            ),
            WorkspaceCommands::Restore {
                path,
                node,
                dry_run,
            } => WorkspaceCommandService::restore(
                self.api.as_ref(),
                &self.workspace_root,
                path.as_deref(),
                node.as_deref(),
                *dry_run,
            ),
            WorkspaceCommands::Compact {
                ttl,
                all,
                keep_frames,
                dry_run,
            } => WorkspaceCommandService::compact(
                self.api.as_ref(),
                *ttl,
                *all,
                *keep_frames,
                *dry_run,
            ),
            WorkspaceCommands::ListDeleted { older_than, format } => {
                let result = WorkspaceCommandService::list_deleted(
                    self.api.as_ref(),
                    *older_than,
                )?;
                format_list_deleted_result(&result, format)
            }
        }
    }

    /// Handle agent management commands
    fn handle_agent_command(&self, command: &AgentCommands) -> Result<String, ApiError> {
        match command {
            AgentCommands::Status { format } => self.handle_agent_status(format.clone()),
            AgentCommands::List { format, role } => {
                self.handle_agent_list(format.clone(), role.as_deref())
            }
            AgentCommands::Show {
                agent_id,
                format,
                include_prompt,
            } => self.handle_agent_show(agent_id, format.clone(), *include_prompt),
            AgentCommands::Validate {
                agent_id,
                all,
                verbose,
            } => self.handle_agent_validate(agent_id.as_deref(), *all, *verbose),
            AgentCommands::Create {
                agent_id,
                role,
                prompt_path,
                interactive,
                non_interactive,
            } => self.handle_agent_create(
                agent_id,
                role.as_deref(),
                prompt_path.as_deref(),
                *interactive,
                *non_interactive,
            ),
            AgentCommands::Edit {
                agent_id,
                prompt_path,
                role,
                editor,
            } => self.handle_agent_edit(
                agent_id,
                prompt_path.as_deref(),
                role.as_deref(),
                editor.as_deref(),
            ),
            AgentCommands::Remove { agent_id, force } => self.handle_agent_remove(agent_id, *force),
        }
    }

    /// Handle agent list command
    fn handle_agent_list(
        &self,
        format: String,
        role_filter: Option<&str>,
    ) -> Result<String, ApiError> {
        let registry = self.api.agent_registry().read();
        let result = crate::agent::AgentCommandService::list(&registry, role_filter)?;
        match format.as_str() {
            "json" => Ok(format_agent_list_result_json(&result)),
            "text" | _ => Ok(format_agent_list_result_text(&result)),
        }
    }

    /// Handle agent show command
    fn handle_agent_show(
        &self,
        agent_id: &str,
        format: String,
        include_prompt: bool,
    ) -> Result<String, ApiError> {
        let registry = self.api.agent_registry().read();
        let result =
            crate::agent::AgentCommandService::show(&registry, agent_id, include_prompt)?;
        match format.as_str() {
            "json" => Ok(format_agent_show_result_json(&result)),
            "text" | _ => Ok(format_agent_show_result_text(&result)),
        }
    }

    /// Handle agent validate command
    fn handle_agent_validate(
        &self,
        agent_id: Option<&str>,
        all: bool,
        verbose: bool,
    ) -> Result<String, ApiError> {
        let registry = self.api.agent_registry().read();
        if all {
            let result = crate::agent::AgentCommandService::validate_all(&registry)?;
            if result.results.is_empty() {
                return Ok("No agents found to validate.".to_string());
            }
            Ok(format_validation_results_all(&result.results, verbose))
        } else {
            let id = agent_id.ok_or_else(|| {
                ApiError::ConfigError("Agent ID required unless --all is specified".to_string())
            })?;
            let result = crate::agent::AgentCommandService::validate_single(&registry, id)?;
            Ok(format_validation_result(&result.result, verbose))
        }
    }

    /// Handle agent create command
    fn handle_agent_create(
        &self,
        agent_id: &str,
        role: Option<&str>,
        prompt_path: Option<&str>,
        interactive: bool,
        non_interactive: bool,
    ) -> Result<String, ApiError> {
        let is_interactive = interactive || (!non_interactive && role.is_none());

        let (final_role, final_prompt_path) = if is_interactive {
            self.create_agent_interactive(agent_id)?
        } else {
            let role_str = role.ok_or_else(|| {
                ApiError::ConfigError(
                    "Role is required in non-interactive mode. Use --role <role>".to_string(),
                )
            })?;
            let parsed_role = crate::agent::AgentCommandService::parse_role(role_str)?;
            let prompt = if parsed_role != crate::agent::AgentRole::Reader {
                Some(
                    prompt_path
                        .ok_or_else(|| {
                            ApiError::ConfigError(
                                "Prompt path is required for Writer agents. Use --prompt-path <path>"
                                    .to_string(),
                            )
                        })?
                        .to_string(),
                )
            } else {
                None
            };
            (parsed_role, prompt)
        };

        let mut registry = self.api.agent_registry().write();
        let result =
            crate::agent::AgentCommandService::create(&mut registry, agent_id, final_role, final_prompt_path)?;
        Ok(format!(
            "Agent created: {}\nConfiguration file: {}",
            result.agent_id,
            result.config_path.display()
        ))
    }

    /// Interactive agent creation
    fn create_agent_interactive(
        &self,
        _agent_id: &str,
    ) -> Result<(crate::agent::AgentRole, Option<String>), ApiError> {
        use dialoguer::{Input, Select};

        // Prompt for role
        let role_selection = Select::new()
            .with_prompt("Agent role")
            .items(&["Reader", "Writer"])
            .default(1)
            .interact()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        let role = match role_selection {
            0 => crate::agent::AgentRole::Reader,
            1 => crate::agent::AgentRole::Writer,
            _ => unreachable!(),
        };

        // Prompt for prompt path if Writer
        let prompt_path = if role != crate::agent::AgentRole::Reader {
            let path: String = Input::new()
                .with_prompt("Prompt file path")
                .interact_text()
                .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;
            Some(path)
        } else {
            None
        };

        Ok((role, prompt_path))
    }

    /// Handle agent edit command
    fn handle_agent_edit(
        &self,
        agent_id: &str,
        prompt_path: Option<&str>,
        role: Option<&str>,
        editor: Option<&str>,
    ) -> Result<String, ApiError> {
        if prompt_path.is_some() || role.is_some() {
            let mut registry = self.api.agent_registry().write();
            let _ = crate::agent::AgentCommandService::update_flags(
                &mut registry,
                agent_id,
                prompt_path,
                role,
            )?;
        } else {
            self.edit_agent_with_editor(agent_id, editor)?;
        }
        Ok(format!("Agent updated: {}", agent_id))
    }

    /// Edit agent config with external editor
    fn edit_agent_with_editor(&self, agent_id: &str, editor: Option<&str>) -> Result<(), ApiError> {
        use std::process::Command;

        let config_path = self
            .api
            .agent_registry()
            .read()
            .agent_config_path(agent_id)?;

        // Load existing config
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ApiError::ConfigError(format!("Failed to read config: {}", e)))?;

        // Create temp file in system temp directory
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("merkle-agent-{}.toml", agent_id));

        std::fs::write(&temp_path, content.as_bytes())
            .map_err(|e| ApiError::ConfigError(format!("Failed to write temp file: {}", e)))?;

        // Determine editor
        let editor_cmd = if let Some(ed) = editor {
            ed.to_string()
        } else {
            std::env::var("EDITOR").map_err(|_| {
                ApiError::ConfigError(
                    "No editor specified and $EDITOR not set. Use --editor <editor>".to_string(),
                )
            })?
        };

        // Open editor
        let status = Command::new(&editor_cmd)
            .arg(&temp_path)
            .status()
            .map_err(|e| ApiError::ConfigError(format!("Failed to open editor: {}", e)))?;

        if !status.success() {
            return Err(ApiError::ConfigError(
                "Editor exited with non-zero status".to_string(),
            ));
        }

        // Read edited content
        let edited_content = std::fs::read_to_string(&temp_path)
            .map_err(|e| ApiError::ConfigError(format!("Failed to read edited file: {}", e)))?;

        let agent_config: crate::agent::AgentConfig = toml::from_str(&edited_content)
            .map_err(|e| ApiError::ConfigError(format!("Invalid config after editing: {}", e)))?;

        let mut registry = self.api.agent_registry().write();
        crate::agent::AgentCommandService::persist_edited_config(
            &mut registry,
            agent_id,
            agent_config,
        )?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        Ok(())
    }

    /// Handle agent remove command
    fn handle_agent_remove(&self, agent_id: &str, force: bool) -> Result<String, ApiError> {
        if !force {
            use dialoguer::Confirm;
            let confirmed = Confirm::new()
                .with_prompt(format!("Remove agent '{}'?", agent_id))
                .interact()
                .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

            if !confirmed {
                return Ok("Removal cancelled".to_string());
            }
        }

        let mut registry = self.api.agent_registry().write();
        let result = crate::agent::AgentCommandService::remove(&mut registry, agent_id)?;
        Ok(format!(
            "Removed agent: {}\nConfiguration file deleted: {}",
            result.agent_id,
            result.config_path.display()
        ))
    }

    /// Handle agent status command
    fn handle_agent_status(&self, format: String) -> Result<String, ApiError> {
        use crate::workspace::{
            format_agent_status_text, AgentStatusEntry, AgentStatusOutput,
        };

        let registry = self.api.agent_registry().read();
        let entries_result = crate::agent::AgentCommandService::status(&registry)?;
        let entries: Vec<AgentStatusEntry> = entries_result
            .into_iter()
            .map(|e| AgentStatusEntry {
                agent_id: e.agent_id,
                role: e.role,
                valid: e.valid,
                prompt_path_exists: e.prompt_path_exists,
            })
            .collect();
        let valid_count = entries.iter().filter(|e| e.valid).count();
        if format == "json" {
            Ok(serde_json::to_string_pretty(&AgentStatusOutput {
                agents: entries.clone(),
                total: entries.len(),
                valid_count,
            })
            .map_err(|e| {
                ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string()))
            })?)
        } else {
            Ok(format_agent_status_text(&entries))
        }
    }

    /// Handle provider management commands
    fn handle_provider_command(
        &self,
        command: &ProviderCommands,
        session_id: &str,
    ) -> Result<String, ApiError> {
        match command {
            ProviderCommands::Status {
                format,
                test_connectivity,
            } => self.handle_provider_status(format.clone(), *test_connectivity),
            ProviderCommands::List {
                format,
                type_filter,
            } => self.handle_provider_list(format.clone(), type_filter.as_deref()),
            ProviderCommands::Show {
                provider_name,
                format,
                include_credentials,
            } => self.handle_provider_show(provider_name, format.clone(), *include_credentials),
            ProviderCommands::Validate {
                provider_name,
                test_connectivity,
                check_model,
                verbose,
            } => self.handle_provider_validate(
                provider_name,
                *test_connectivity,
                *check_model,
                *verbose,
            ),
            ProviderCommands::Test {
                provider_name,
                model,
                timeout,
            } => self.handle_provider_test(provider_name, model.as_deref(), *timeout, session_id),
            ProviderCommands::Create {
                provider_name,
                type_,
                model,
                endpoint,
                api_key,
                interactive,
                non_interactive,
            } => self.handle_provider_create(
                provider_name,
                type_.as_deref(),
                model.as_deref(),
                endpoint.as_deref(),
                api_key.as_deref(),
                *interactive,
                *non_interactive,
            ),
            ProviderCommands::Edit {
                provider_name,
                model,
                endpoint,
                api_key,
                editor,
            } => self.handle_provider_edit(
                provider_name,
                model.as_deref(),
                endpoint.as_deref(),
                api_key.as_deref(),
                editor.as_deref(),
            ),
            ProviderCommands::Remove {
                provider_name,
                force,
            } => self.handle_provider_remove(provider_name, *force),
        }
    }

    /// Handle provider list command
    fn handle_provider_list(
        &self,
        format: String,
        type_filter: Option<&str>,
    ) -> Result<String, ApiError> {
        let registry = self.api.provider_registry().read();
        let result =
            crate::provider::commands::ProviderCommandService::run_list(&registry, type_filter)?;
        match format.as_str() {
            "json" => Ok(format_provider_list_result_json(&result)),
            "text" | _ => Ok(format_provider_list_result_text(&result)),
        }
    }

    /// Handle provider show command
    fn handle_provider_show(
        &self,
        provider_name: &str,
        format: String,
        include_credentials: bool,
    ) -> Result<String, ApiError> {
        let registry = self.api.provider_registry().read();
        let result = crate::provider::commands::ProviderCommandService::run_show(
            &registry,
            provider_name,
            include_credentials,
        )?;
        match format.as_str() {
            "json" => Ok(format_provider_show_result_json(&result)),
            "text" | _ => Ok(format_provider_show_result_text(&result)),
        }
    }

    /// Handle provider validate command
    fn handle_provider_validate(
        &self,
        provider_name: &str,
        test_connectivity: bool,
        check_model: bool,
        verbose: bool,
    ) -> Result<String, ApiError> {
        let registry = self.api.provider_registry().read();
        let result = crate::provider::commands::ProviderCommandService::run_validate(
            &registry,
            provider_name,
            test_connectivity,
            check_model,
        )?;
        Ok(format_provider_validation_result(&result, verbose))
    }

    /// Handle provider status command
    fn handle_provider_status(
        &self,
        format: String,
        test_connectivity: bool,
    ) -> Result<String, ApiError> {
        use crate::workspace::{
            format_provider_status_text, ProviderStatusEntry, ProviderStatusOutput,
        };

        let registry = self.api.provider_registry().read();
        let entries_result =
            crate::provider::commands::ProviderCommandService::run_status(&registry, test_connectivity)?;
        let entries: Vec<ProviderStatusEntry> = entries_result
            .into_iter()
            .map(|e| ProviderStatusEntry {
                provider_name: e.provider_name,
                provider_type: e.provider_type,
                model: e.model,
                connectivity: e.connectivity,
            })
            .collect();
        if format == "json" {
            Ok(serde_json::to_string_pretty(&ProviderStatusOutput {
                providers: entries.clone(),
                total: entries.len(),
            })
            .map_err(|e| {
                ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string()))
            })?)
        } else {
            Ok(format_provider_status_text(&entries, test_connectivity))
        }
    }

    /// Handle provider test command
    fn handle_provider_test(
        &self,
        provider_name: &str,
        model_override: Option<&str>,
        timeout: u64,
        session_id: &str,
    ) -> Result<String, ApiError> {
        let registry = self.api.provider_registry().read();
        let model_for_event = model_override.unwrap_or_else(|| {
            registry
                .get(provider_name)
                .map(|p| p.model.as_str())
                .unwrap_or("")
        });
        self.progress.emit_event_best_effort(
            session_id,
            "provider_request_sent",
            json!(ProviderLifecycleEventData {
                node_id: "provider_test".to_string(),
                agent_id: "provider_test".to_string(),
                provider_name: provider_name.to_string(),
                frame_type: model_for_event.to_string(),
                duration_ms: None,
                error: None,
                retry_count: Some(0),
            }),
        );
        let start = std::time::Instant::now();
        let result = crate::provider::commands::ProviderCommandService::run_test(
            &registry,
            provider_name,
            model_override,
            timeout,
        )?;
        let elapsed_ms = start.elapsed().as_millis();
        if result.connectivity_ok {
            self.progress.emit_event_best_effort(
                session_id,
                "provider_response_received",
                json!(ProviderLifecycleEventData {
                    node_id: "provider_test".to_string(),
                    agent_id: "provider_test".to_string(),
                    provider_name: result.provider_name.clone(),
                    frame_type: result.model_checked.clone(),
                    duration_ms: Some(elapsed_ms),
                    error: None,
                    retry_count: Some(0),
                }),
            );
        } else {
            self.progress.emit_event_best_effort(
                session_id,
                "provider_request_failed",
                json!(ProviderLifecycleEventData {
                    node_id: "provider_test".to_string(),
                    agent_id: "provider_test".to_string(),
                    provider_name: result.provider_name.clone(),
                    frame_type: result.model_checked.clone(),
                    duration_ms: Some(elapsed_ms),
                    error: result.error_message.clone(),
                    retry_count: Some(0),
                }),
            );
        }
        Ok(format_provider_test_result(&result, Some(elapsed_ms)))
    }

    /// Handle provider create command
    fn handle_provider_create(
        &self,
        provider_name: &str,
        type_: Option<&str>,
        model: Option<&str>,
        endpoint: Option<&str>,
        api_key: Option<&str>,
        interactive: bool,
        non_interactive: bool,
    ) -> Result<String, ApiError> {
        // Determine mode
        let is_interactive = interactive || (!non_interactive && type_.is_none());

        let (provider_type, final_model, final_endpoint, final_api_key, default_options) =
            if is_interactive {
                // Interactive mode
                self.create_provider_interactive()?
            } else {
                // Non-interactive mode
                let type_str = type_.ok_or_else(|| {
                    ApiError::ConfigError(
                        "Provider type is required in non-interactive mode. Use --type <type>"
                            .to_string(),
                    )
                })?;

                let parsed_type =
                    crate::provider::commands::ProviderCommandService::parse_provider_type(
                        type_str,
                    )?;

                let model_name = model.ok_or_else(|| {
                    ApiError::ConfigError(
                        "Model is required in non-interactive mode. Use --model <model>"
                            .to_string(),
                    )
                })?;

                (
                    parsed_type,
                    model_name.to_string(),
                    endpoint.map(|s| s.to_string()),
                    api_key.map(|s| s.to_string()),
                    crate::provider::CompletionOptions::default(),
                )
            };

        let mut registry = self.api.provider_registry().write();
        let result = crate::provider::commands::ProviderCommandService::run_create(
            &mut registry,
            provider_name,
            provider_type,
            final_model,
            final_endpoint,
            final_api_key,
            default_options,
        )?;
        Ok(format!(
            "Provider created: {}\nConfiguration file: {}",
            result.provider_name,
            result.config_path.display()
        ))
    }

    /// Interactive provider creation
    fn create_provider_interactive(
        &self,
    ) -> Result<
        (
            crate::config::ProviderType,
            String,
            Option<String>,
            Option<String>,
            crate::provider::CompletionOptions,
        ),
        ApiError,
    > {
        use dialoguer::{Input, Select};

        // Prompt for provider type
        let type_selection = Select::new()
            .with_prompt("Provider type")
            .items(&["openai", "anthropic", "ollama", "local"])
            .default(0)
            .interact()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        let provider_type = match type_selection {
            0 => crate::config::ProviderType::OpenAI,
            1 => crate::config::ProviderType::Anthropic,
            2 => crate::config::ProviderType::Ollama,
            3 => crate::config::ProviderType::LocalCustom,
            _ => unreachable!(),
        };

        // Prompt for model name
        let model: String = Input::new()
            .with_prompt("Model name")
            .interact_text()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        // Prompt for endpoint (with defaults)
        let default_endpoint =
            crate::provider::commands::ProviderCommandService::default_endpoint(provider_type);

        let endpoint = if provider_type == crate::config::ProviderType::LocalCustom {
            // Required for local
            Some(
                Input::new()
                    .with_prompt("Endpoint URL (required)")
                    .interact_text()
                    .map_err(|e| {
                        ApiError::ConfigError(format!("Failed to get user input: {}", e))
                    })?,
            )
        } else if let Some(default) = default_endpoint {
            // Optional with default
            let input: String = Input::new()
                .with_prompt(format!("Endpoint URL (optional, default: {})", default))
                .default(default)
                .interact_text()
                .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;
            Some(input)
        } else {
            None
        };

        // Prompt for API key (optional, suggest env var)
        let env_var = crate::provider::commands::ProviderCommandService::required_api_key_env_var(
            provider_type,
        )
        .unwrap_or("");

        let api_key = if provider_type == crate::config::ProviderType::Ollama
            || provider_type == crate::config::ProviderType::LocalCustom
        {
            None
        } else {
            let prompt = if !env_var.is_empty() {
                format!(
                    "API key (optional, will use {} env var if not set)",
                    env_var
                )
            } else {
                "API key (optional)".to_string()
            };

            let input: String = Input::new()
                .with_prompt(prompt)
                .allow_empty(true)
                .interact_text()
                .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

            if input.is_empty() {
                None
            } else {
                Some(input)
            }
        };

        // Prompt for default completion options
        let temperature: f32 = Input::new()
            .with_prompt("Default temperature (0.0-2.0, default: 1.0)")
            .default(1.0)
            .interact_text()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        let max_tokens: Option<u32> = {
            let input: String = Input::new()
                .with_prompt("Default max tokens (optional, press Enter to skip)")
                .allow_empty(true)
                .interact_text()
                .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

            if input.is_empty() {
                None
            } else {
                input.parse().ok()
            }
        };

        let default_options = crate::provider::CompletionOptions {
            temperature: Some(temperature),
            max_tokens,
            ..Default::default()
        };

        Ok((provider_type, model, endpoint, api_key, default_options))
    }

    /// Handle provider edit command
    fn handle_provider_edit(
        &self,
        provider_name: &str,
        model: Option<&str>,
        endpoint: Option<&str>,
        api_key: Option<&str>,
        editor: Option<&str>,
    ) -> Result<String, ApiError> {
        if model.is_some() || endpoint.is_some() || api_key.is_some() {
            let mut registry = self.api.provider_registry().write();
            crate::provider::commands::ProviderCommandService::run_update_flags(
                &mut registry,
                provider_name,
                model,
                endpoint,
                api_key,
            )?;
        } else {
            self.edit_provider_with_editor(provider_name, editor)?;
        }
        Ok(format!("Provider updated: {}", provider_name))
    }

    /// Edit provider config with external editor
    fn edit_provider_with_editor(
        &self,
        provider_name: &str,
        editor: Option<&str>,
    ) -> Result<(), ApiError> {
        use std::process::Command;

        let config_path = {
            let registry = self.api.provider_registry().read();
            crate::provider::commands::ProviderCommandService::provider_config_path(
                &registry,
                provider_name,
            )?
        };

        // Load existing config
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ApiError::ConfigError(format!("Failed to read config: {}", e)))?;

        // Create temp file in system temp directory
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("merkle-provider-{}.toml", provider_name));

        std::fs::write(&temp_path, content.as_bytes())
            .map_err(|e| ApiError::ConfigError(format!("Failed to write temp file: {}", e)))?;

        // Determine editor
        let editor_cmd = if let Some(ed) = editor {
            ed.to_string()
        } else {
            std::env::var("EDITOR").map_err(|_| {
                ApiError::ConfigError(
                    "No editor specified and $EDITOR not set. Use --editor <editor>".to_string(),
                )
            })?
        };

        // Open editor
        let status = Command::new(&editor_cmd)
            .arg(&temp_path)
            .status()
            .map_err(|e| ApiError::ConfigError(format!("Failed to open editor: {}", e)))?;

        if !status.success() {
            return Err(ApiError::ConfigError(
                "Editor exited with non-zero status".to_string(),
            ));
        }

        // Read edited content
        let edited_content = std::fs::read_to_string(&temp_path)
            .map_err(|e| ApiError::ConfigError(format!("Failed to read edited file: {}", e)))?;

        // Parse and validate
        let provider_config: crate::config::ProviderConfig = toml::from_str(&edited_content)
            .map_err(|e| ApiError::ConfigError(format!("Invalid config after editing: {}", e)))?;

        // Validate provider_name matches
        if let Some(ref config_name) = provider_config.provider_name {
            if config_name != provider_name {
                return Err(ApiError::ConfigError(format!(
                    "Provider name mismatch: config has '{}' but expected '{}'",
                    config_name, provider_name
                )));
            }
        }

        {
            let mut registry = self.api.provider_registry().write();
            crate::provider::commands::ProviderCommandService::persist_provider_config(
                &mut registry,
                provider_name,
                &provider_config,
            )?;
        }

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        Ok(())
    }

    /// Handle provider remove command
    fn handle_provider_remove(&self, provider_name: &str, force: bool) -> Result<String, ApiError> {
        {
            let registry = self.api.provider_registry().read();
            let provider = registry.get_or_error(provider_name)?;
            if provider.provider_type == crate::provider::ProviderType::OpenAI
                || provider.provider_type == crate::provider::ProviderType::Anthropic
            {
                eprintln!(
                    "Warning: Provider '{}' may be in use by agents.",
                    provider_name
                );
            }
        }

        if !force {
            use dialoguer::Confirm;
            let confirmed = Confirm::new()
                .with_prompt(format!("Remove provider '{}'?", provider_name))
                .interact()
                .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

            if !confirmed {
                return Ok("Removal cancelled".to_string());
            }
        }

        let mut registry = self.api.provider_registry().write();
        let result =
            crate::provider::commands::ProviderCommandService::run_remove(&mut registry, provider_name)?;
        Ok(format!(
            "Removed provider: {}\nConfiguration file deleted: {}",
            result.provider_name,
            result.config_path.display()
        ))
    }

    /// Handle init command
    fn handle_init(&self, force: bool, list: bool) -> Result<String, ApiError> {
        if list {
            let preview = crate::init::list_initialization()?;
            Ok(format_init_preview(&preview))
        } else {
            let summary = crate::init::initialize_all(force)?;
            Ok(format_init_summary(&summary, force))
        }
    }

    /// Handle context management commands
    fn handle_context_command(
        &self,
        command: &ContextCommands,
        session_id: &str,
    ) -> Result<String, ApiError> {
        match command {
            ContextCommands::Generate {
                node,
                path,
                path_positional,
                agent,
                provider,
                frame_type,
                force,
                no_recursive,
            } => {
                let path_merged = path.as_ref().or(path_positional.as_ref());
                self.handle_context_generate(
                    node.as_deref(),
                    path_merged,
                    agent.as_deref(),
                    provider.as_deref(),
                    frame_type.as_deref(),
                    *force,
                    *no_recursive,
                    session_id,
                )
            }
            ContextCommands::Get {
                node,
                path,
                agent,
                frame_type,
                max_frames,
                ordering,
                combine,
                separator,
                format,
                include_metadata,
                include_deleted,
            } => self.handle_context_get(
                node.as_deref(),
                path.as_ref(),
                agent.as_deref(),
                frame_type.as_deref(),
                *max_frames,
                ordering,
                *combine,
                separator,
                format,
                *include_metadata,
                *include_deleted,
                session_id,
            ),
        }
    }

    /// Resolve agent ID (default to single Writer agent if not specified)
    fn resolve_agent_id(&self, agent_id: Option<&str>) -> Result<String, ApiError> {
        if let Some(agent_id) = agent_id {
            // Verify agent exists
            self.api.get_agent(agent_id)?;
            return Ok(agent_id.to_string());
        }

        // Find Writer agents
        let (agent_count, agent_ids) = {
            let registry = self.api.agent_registry().read();
            let writer_agents = registry.list_by_role(Some(crate::agent::AgentRole::Writer));
            let agent_ids: Vec<String> = writer_agents.iter().map(|a| a.agent_id.clone()).collect();
            (agent_ids.len(), agent_ids)
        };

        match agent_count {
            0 => Err(ApiError::ConfigError(
                "No Writer agents found. Use `merkle agent list` to see available agents, or use `--agent <agent_id>` to specify an agent.".to_string()
            )),
            1 => Ok(agent_ids[0].clone()),
            _ => {
                Err(ApiError::ConfigError(format!(
                    "Multiple Writer agents found: {}. Use `--agent <agent_id>` to specify which agent to use.",
                    agent_ids.join(", ")
                )))
            }
        }
    }

    /// Resolve provider name (must be specified)
    fn resolve_provider_name(&self, provider_name: Option<&str>) -> Result<String, ApiError> {
        let provider_name = provider_name.ok_or_else(|| {
            ApiError::ProviderNotConfigured(
                "Provider is required. Use `--provider <provider_name>` to specify a provider. Use `merkle provider list` to see available providers.".to_string()
            )
        })?;

        // Verify provider exists
        let registry = self.api.provider_registry().read();
        registry.get_or_error(provider_name)?;
        drop(registry);

        Ok(provider_name.to_string())
    }

    /// Handle context generate command
    fn handle_context_generate(
        &self,
        node: Option<&str>,
        path: Option<&PathBuf>,
        agent: Option<&str>,
        provider: Option<&str>,
        frame_type: Option<&str>,
        force: bool,
        no_recursive: bool,
        session_id: &str,
    ) -> Result<String, ApiError> {
        // 1. Path/NodeID resolution (mutually exclusive)
        let node_id = match (node, path) {
            (Some(node_str), None) => {
                // Parse NodeID
                parse_node_id(node_str)?
            }
            (None, Some(path)) => {
                // Resolve path to NodeID
                crate::workspace::resolve_workspace_node_id(
                    self.api.as_ref(),
                    &self.workspace_root,
                    Some(path.as_path()),
                    None,
                    false,
                )?
            }
            (Some(_), Some(_)) => {
                return Err(ApiError::ConfigError(
                    "Cannot specify both --node and --path. Use one or the other.".to_string(),
                ));
            }
            (None, None) => {
                return Err(ApiError::ConfigError(
                    "Must specify either --node <node_id>, --path <path>, or a positional path (e.g. merkle context generate ./foo).".to_string()
                ));
            }
        };

        // 2. Agent resolution
        let agent_id = self.resolve_agent_id(agent)?;

        // 3. Provider resolution
        let provider_name = self.resolve_provider_name(provider)?;

        // 4. Frame type resolution
        let frame_type = frame_type
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("context-{}", agent_id));

        // 5. Agent validation
        let agent = self.api.get_agent(&agent_id)?;

        // Verify agent has Writer role
        if agent.role != crate::agent::AgentRole::Writer {
            return Err(ApiError::Unauthorized(format!(
                "Agent '{}' has role {:?}, but only Writer agents can generate frames.",
                agent_id, agent.role
            )));
        }

        // Verify node exists
        let node_record = self
            .api
            .node_store()
            .get(&node_id)
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NodeNotFound(node_id))?;
        let node_path = node_record.path.to_string_lossy().to_string();

        // Check if agent has system_prompt in metadata
        if !agent.metadata.contains_key("system_prompt") {
            return Err(ApiError::ConfigError(format!(
                "Agent '{}' is missing system_prompt. Use `merkle agent validate {}` to check agent configuration.",
                agent_id, agent_id
            )));
        }

        let is_directory_target =
            matches!(node_record.node_type, crate::store::NodeType::Directory);
        let recursive = is_directory_target && !no_recursive;
        let plan = self.build_generation_plan(
            node_id,
            &node_record.path,
            is_directory_target,
            recursive,
            force,
            session_id,
            &agent_id,
            &provider_name,
            &frame_type,
        )?;
        self.progress.emit_event_best_effort(
            session_id,
            "plan_constructed",
            json!({
                "plan_id": plan.plan_id,
                "node_id": hex::encode(node_id),
                "path": node_path,
                "agent_id": agent_id,
                "provider_name": provider_name,
                "frame_type": frame_type,
                "force": force,
                "recursive": recursive,
                "total_nodes": plan.total_nodes,
                "total_levels": plan.total_levels
            }),
        );
        if plan.total_nodes == 0 {
            return Ok(
                "Frame already exists for requested target.\nUse --force to generate a new frame."
                    .to_string(),
            );
        }

        // 7. Blocking generation through context executor
        // Create runtime before calling get_or_create_queue() which needs it for queue.start()
        // Check if we're already in a runtime (shouldn't happen in CLI, but can in tests)
        let rt = if let Ok(_handle) = tokio::runtime::Handle::try_current() {
            // We're in a runtime - can't create another one or use block_on
            // This should not happen in normal CLI usage, but can occur in tests
            // For now, return an error - the caller should handle this case
            return Err(ApiError::ProviderError(
                "Cannot generate context from within an async runtime context. This is a limitation when running from async tests.".to_string()
            ));
        } else {
            // No runtime exists, create one
            tokio::runtime::Runtime::new()
                .map_err(|e| ApiError::ProviderError(format!("Failed to create runtime: {}", e)))?
        };

        // Enter runtime context for queue.start() which needs tokio::spawn
        let _guard = rt.enter();
        let queue = self.get_or_create_queue(Some(session_id))?;
        // Drop guard before using block_on (can't block while in runtime context)
        drop(_guard);
        let executor = GenerationExecutor::new(Some(Arc::clone(&self.progress)));
        let result = rt.block_on(async { executor.execute(queue.as_ref(), plan).await })?;
        if result.total_failed > 0 {
            return Err(ApiError::GenerationFailed(format!(
                "Generation completed with failures. generated={}, failed={}",
                result.total_generated, result.total_failed
            )));
        }
        Ok(format!(
            "Generation completed: generated={}, failed={}",
            result.total_generated, result.total_failed
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn build_generation_plan(
        &self,
        target_node_id: NodeID,
        target_path: &std::path::Path,
        is_directory_target: bool,
        recursive: bool,
        force: bool,
        session_id: &str,
        agent_id: &str,
        provider_name: &str,
        frame_type: &str,
    ) -> Result<GenerationPlan, ApiError> {
        if !recursive && is_directory_target && !force {
            self.progress.emit_event_best_effort(
                session_id,
                "descendant_check_started",
                json!({
                    "node_id": hex::encode(target_node_id),
                    "path": target_path.to_string_lossy(),
                    "frame_type": frame_type,
                }),
            );
            let missing = self.find_missing_descendant_heads(target_node_id, frame_type)?;
            if !missing.is_empty() {
                self.progress.emit_event_best_effort(
                    session_id,
                    "descendant_check_failed",
                    json!({
                        "node_id": hex::encode(target_node_id),
                        "missing_count": missing.len(),
                        "missing_paths": missing,
                    }),
                );
                return Err(ApiError::GenerationFailed(
                    "Directory descendants are missing required heads; run recursive generation or use --force.".to_string(),
                ));
            }
            self.progress.emit_event_best_effort(
                session_id,
                "descendant_check_passed",
                json!({
                    "node_id": hex::encode(target_node_id),
                    "path": target_path.to_string_lossy(),
                    "frame_type": frame_type,
                }),
            );
        }

        let mut levels: Vec<Vec<GenerationItem>> = Vec::new();
        if recursive {
            let depth_levels = self.collect_subtree_levels(target_node_id)?;
            for level in depth_levels {
                let mut items = Vec::new();
                for node_id in level {
                    let record = self
                        .api
                        .node_store()
                        .get(&node_id)
                        .map_err(ApiError::from)?
                        .ok_or_else(|| ApiError::NodeNotFound(node_id))?;
                    if !force && self.api.get_head(&node_id, frame_type)?.is_some() {
                        self.progress.emit_event_best_effort(
                            session_id,
                            "node_skipped",
                            json!({
                                "node_id": hex::encode(node_id),
                                "path": record.path.to_string_lossy(),
                                "agent_id": agent_id,
                                "provider_name": provider_name,
                                "frame_type": frame_type,
                                "reason": "head_reuse",
                            }),
                        );
                        continue;
                    }
                    items.push(GenerationItem {
                        node_id,
                        path: record.path.to_string_lossy().to_string(),
                        node_type: match record.node_type {
                            crate::store::NodeType::File { .. } => GenerationNodeType::File,
                            crate::store::NodeType::Directory => GenerationNodeType::Directory,
                        },
                        agent_id: agent_id.to_string(),
                        provider_name: provider_name.to_string(),
                        frame_type: frame_type.to_string(),
                        force,
                    });
                }
                if !items.is_empty() {
                    levels.push(items);
                }
            }
        } else {
            if !force && self.api.get_head(&target_node_id, frame_type)?.is_some() {
                self.progress.emit_event_best_effort(
                    session_id,
                    "node_skipped",
                    json!({
                        "node_id": hex::encode(target_node_id),
                        "path": target_path.to_string_lossy(),
                        "agent_id": agent_id,
                        "provider_name": provider_name,
                        "frame_type": frame_type,
                        "reason": "head_reuse",
                    }),
                );
                return Ok(GenerationPlan {
                    plan_id: format!(
                        "plan-{}-{}",
                        crate::telemetry::now_millis(),
                        &hex::encode(target_node_id)[..8]
                    ),
                    source: format!("context generate {}", target_path.to_string_lossy()),
                    session_id: Some(session_id.to_string()),
                    levels: Vec::new(),
                    priority: PlanPriority::Urgent,
                    failure_policy: FailurePolicy::StopOnLevelFailure,
                    target_path: target_path.to_string_lossy().to_string(),
                    total_nodes: 0,
                    total_levels: 0,
                });
            }
            let target_record = self
                .api
                .node_store()
                .get(&target_node_id)
                .map_err(ApiError::from)?
                .ok_or_else(|| ApiError::NodeNotFound(target_node_id))?;
            levels.push(vec![GenerationItem {
                node_id: target_node_id,
                path: target_record.path.to_string_lossy().to_string(),
                node_type: match target_record.node_type {
                    crate::store::NodeType::File { .. } => GenerationNodeType::File,
                    crate::store::NodeType::Directory => GenerationNodeType::Directory,
                },
                agent_id: agent_id.to_string(),
                provider_name: provider_name.to_string(),
                frame_type: frame_type.to_string(),
                force,
            }]);
        }

        let total_nodes: usize = levels.iter().map(std::vec::Vec::len).sum();
        Ok(GenerationPlan {
            plan_id: format!(
                "plan-{}-{}",
                crate::telemetry::now_millis(),
                &hex::encode(target_node_id)[..8]
            ),
            source: format!("context generate {}", target_path.to_string_lossy()),
            session_id: Some(session_id.to_string()),
            total_levels: levels.len(),
            levels,
            priority: PlanPriority::Urgent,
            failure_policy: FailurePolicy::StopOnLevelFailure,
            target_path: target_path.to_string_lossy().to_string(),
            total_nodes,
        })
    }

    fn find_missing_descendant_heads(
        &self,
        target_node_id: NodeID,
        frame_type: &str,
    ) -> Result<Vec<String>, ApiError> {
        let mut missing = Vec::new();
        let mut visited: HashSet<NodeID> = HashSet::new();
        let mut queue = VecDeque::new();
        let target_record = self
            .api
            .node_store()
            .get(&target_node_id)
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NodeNotFound(target_node_id))?;
        for child in target_record.children {
            queue.push_back(child);
        }

        while let Some(node_id) = queue.pop_front() {
            if !visited.insert(node_id) {
                continue;
            }
            let record = self
                .api
                .node_store()
                .get(&node_id)
                .map_err(ApiError::from)?
                .ok_or_else(|| ApiError::NodeNotFound(node_id))?;
            if self.api.get_head(&node_id, frame_type)?.is_none() {
                missing.push(record.path.to_string_lossy().to_string());
            }
            for child in record.children {
                queue.push_back(child);
            }
        }
        Ok(missing)
    }

    fn collect_subtree_levels(&self, target_node_id: NodeID) -> Result<Vec<Vec<NodeID>>, ApiError> {
        let mut levels: HashMap<usize, Vec<NodeID>> = HashMap::new();
        let mut visited: HashSet<NodeID> = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((target_node_id, 0usize));

        while let Some((node_id, depth)) = queue.pop_front() {
            if !visited.insert(node_id) {
                continue;
            }
            levels.entry(depth).or_default().push(node_id);
            let record = self
                .api
                .node_store()
                .get(&node_id)
                .map_err(ApiError::from)?
                .ok_or_else(|| ApiError::NodeNotFound(node_id))?;
            for child in record.children {
                queue.push_back((child, depth + 1));
            }
        }

        let mut ordered_depths: Vec<_> = levels.into_iter().collect();
        ordered_depths.sort_by(|(a, _), (b, _)| b.cmp(a));
        Ok(ordered_depths.into_iter().map(|(_, nodes)| nodes).collect())
    }

    /// Handle context get command
    fn handle_context_get(
        &self,
        node: Option<&str>,
        path: Option<&PathBuf>,
        agent: Option<&str>,
        frame_type: Option<&str>,
        max_frames: usize,
        ordering: &str,
        combine: bool,
        separator: &str,
        format: &str,
        include_metadata: bool,
        include_deleted: bool,
        session_id: &str,
    ) -> Result<String, ApiError> {
        // 1. Path/NodeID resolution
        let node_id = match (node, path) {
            (Some(node_str), None) => parse_node_id(node_str)?,
            (None, Some(path)) => crate::workspace::resolve_workspace_node_id(
                self.api.as_ref(),
                &self.workspace_root,
                Some(path.as_path()),
                None,
                false,
            )?,
            (Some(_), Some(_)) => {
                return Err(ApiError::ConfigError(
                    "Cannot specify both --node and --path. Use one or the other.".to_string(),
                ));
            }
            (None, None) => {
                return Err(ApiError::ConfigError(
                    "Must specify either --node <node_id> or --path <path>.".to_string(),
                ));
            }
        };

        // 2. Build ContextView
        let ordering_policy = match ordering {
            "recency" => crate::views::OrderingPolicy::Recency,
            "deterministic" => crate::views::OrderingPolicy::Type, // Use type ordering for deterministic
            _ => {
                return Err(ApiError::ConfigError(format!(
                    "Invalid ordering: '{}'. Must be 'recency' or 'deterministic'.",
                    ordering
                )));
            }
        };

        let mut builder = ContextView::builder().max_frames(max_frames);

        // Set ordering
        match ordering_policy {
            crate::views::OrderingPolicy::Recency => {
                builder = builder.recent();
            }
            crate::views::OrderingPolicy::Type => {
                builder = builder.by_type_ordering(); // Deterministic ordering by type
            }
            _ => {
                builder = builder.recent(); // Default to recency
            }
        }

        // Add filters
        if let Some(agent_id) = agent {
            builder = builder.by_agent(agent_id);
        }
        if let Some(ft) = frame_type {
            builder = builder.by_type(ft);
        }
        if !include_deleted {
            // Exclude deleted frames by default
            // Note: FrameFilter::ExcludeDeleted would need to be added to views.rs
            // For now, we'll filter in the output formatting
        }

        let view = builder.build();

        // 3. Retrieve context
        let context = self.api.get_node(node_id, view)?;

        // 4. Format output
        let formatted = match format {
            "text" => format_context_text_output(
                &context,
                include_metadata,
                combine,
                separator,
                include_deleted,
            ),
            "json" => format_context_json_output(&context, include_metadata, include_deleted),
            _ => Err(ApiError::ConfigError(format!(
                "Invalid format: '{}'. Must be 'text' or 'json'.",
                format
            ))),
        }?;
        self.progress.emit_event_best_effort(
            session_id,
            "context_read_summary",
            json!({
                "node_id": hex::encode(node_id),
                "frame_count": context.frames.len(),
                "max_frames": max_frames,
                "ordering": ordering,
                "combine": combine,
                "format": format
            }),
        );
        Ok(formatted)
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
        let descriptor = summary_descriptor(command);
        telemetry_emit_command_summary(
            self.progress.as_ref(),
            session_id,
            &command_name(command),
            &descriptor,
            ok,
            duration_ms,
            error.as_deref(),
            message,
            output_chars,
            error_chars,
            truncated,
        );
    }
}

/// Format agent list result as text
fn format_agent_list_result_text(result: &crate::agent::AgentListResult) -> String {
    let agents = &result.agents;
    if agents.is_empty() {
        return "No agents found.\n\nNote: Agents are provider-agnostic. Providers are selected at runtime.".to_string();
    }
    let mut output = String::from("Available Agents:\n");
    for item in agents {
        let role_str = match item.role {
            crate::agent::AgentRole::Reader => "Reader",
            crate::agent::AgentRole::Writer => "Writer",
        };
        output.push_str(&format!("  {:<20} {:<10}\n", item.agent_id, role_str));
    }
    output.push_str(&format!("\nTotal: {} agent(s)\n\nNote: Agents are provider-agnostic. Providers are selected at runtime.", agents.len()));
    output
}

/// Format agent list result as JSON
fn format_agent_list_result_json(result: &crate::agent::AgentListResult) -> String {
    let agent_list: Vec<_> = result
        .agents
        .iter()
        .map(|item| {
            json!({
                "agent_id": item.agent_id,
                "role": match item.role {
                    crate::agent::AgentRole::Reader => "Reader",
                    crate::agent::AgentRole::Writer => "Writer",
                },
            })
        })
        .collect();
    let out = json!({ "agents": agent_list, "total": result.agents.len() });
    serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
}

/// Format agent show result as text
fn format_agent_show_result_text(result: &crate::agent::AgentShowResult) -> String {
    let role_str = match result.role {
        crate::agent::AgentRole::Reader => "Reader",
        crate::agent::AgentRole::Writer => "Writer",
    };
    let mut output = format!("Agent: {}\n", result.agent_id);
    output.push_str(&format!("Role: {}\n", role_str));
    output.push_str("Prompt: [see config]\n");
    if let Some(prompt) = &result.prompt_content {
        output.push_str("\nPrompt Content:\n");
        output.push_str(prompt);
    }
    output
}

/// Format agent show result as JSON
fn format_agent_show_result_json(result: &crate::agent::AgentShowResult) -> String {
    let role_str = match result.role {
        crate::agent::AgentRole::Reader => "Reader",
        crate::agent::AgentRole::Writer => "Writer",
    };
    let mut out = json!({
        "agent_id": result.agent_id,
        "role": role_str,
    });
    if let Some(p) = &result.prompt_content {
        out["prompt_content"] = json!(p);
    }
    serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
}

/// Format validation result
fn format_validation_result(result: &crate::agent::ValidationResult, verbose: bool) -> String {
    let mut output = format!("Validating agent: {}\n\n", result.agent_id);

    if result.errors.is_empty() && result.checks.iter().all(|(_, passed)| *passed) {
        output.push_str("✓ All validation checks passed\n\n");
    } else {
        // Show checks
        for (description, passed) in &result.checks {
            if *passed {
                output.push_str(&format!("✓ {}\n", description));
            } else {
                output.push_str(&format!("✗ {}\n", description));
            }
        }

        // Show errors
        if !result.errors.is_empty() {
            output.push_str("\n");
            for error in &result.errors {
                output.push_str(&format!("✗ {}\n", error));
            }
        }

        output.push_str("\n");
    }

    if verbose {
        output.push_str(&format!(
            "Validation summary: {}/{} checks passed\n",
            result.passed_checks(),
            result.total_checks()
        ));
        if !result.errors.is_empty() {
            output.push_str(&format!("Errors found: {}\n", result.errors.len()));
        }
    } else {
        if result.is_valid() {
            output.push_str(&format!(
                "Validation passed: {}/{} checks\n",
                result.passed_checks(),
                result.total_checks()
            ));
        } else {
            output.push_str(&format!(
                "Validation failed: {} error(s) found\n",
                result.errors.len()
            ));
        }
    }

    output
}

/// Format multiple validation results (for --all)
fn format_validation_results_all(
    results: &[(String, crate::agent::ValidationResult)],
    verbose: bool,
) -> String {
    let mut output = String::from("Validating all agents:\n\n");

    let mut valid_count = 0;
    let mut invalid_count = 0;

    for (agent_id, result) in results {
        if result.is_valid() {
            valid_count += 1;
            if verbose {
                output.push_str(&format!(
                    "✓ {}: All checks passed ({}/{} checks)\n",
                    agent_id,
                    result.passed_checks(),
                    result.total_checks()
                ));
            } else {
                output.push_str(&format!("✓ {}: Valid\n", agent_id));
            }
        } else {
            invalid_count += 1;
            output.push_str(&format!("✗ {}: Validation failed\n", agent_id));
            if verbose {
                // Show details for invalid agents
                for (description, passed) in &result.checks {
                    if !passed {
                        output.push_str(&format!("  ✗ {}\n", description));
                    }
                }
                for error in &result.errors {
                    output.push_str(&format!("  ✗ {}\n", error));
                }
            }
        }
    }

    output.push_str(&format!(
        "\nSummary: {} valid, {} invalid (out of {} total)\n",
        valid_count,
        invalid_count,
        results.len()
    ));

    output
}

/// Format provider list result as text
fn format_provider_list_result_text(
    result: &crate::provider::commands::ProviderListResult,
) -> String {
    let providers = &result.providers;
    if providers.is_empty() {
        return "No providers found.\n\nUse 'merkle provider create' to add a provider."
            .to_string();
    }
    let mut output = String::from("Available Providers:\n");
    for provider in providers {
        let type_str = crate::provider::profile::provider_type_slug(provider.provider_type);
        let endpoint_str = provider.endpoint.as_deref().unwrap_or("(default endpoint)");
        let provider_name = provider.provider_name.as_deref().unwrap_or("unknown");
        output.push_str(&format!(
            "  {:<20} {:<10} {:<20} {}\n",
            provider_name, type_str, provider.model, endpoint_str
        ));
    }
    output.push_str(&format!("\nTotal: {} provider(s)\n", providers.len()));
    output
}

/// Format provider list result as JSON
fn format_provider_list_result_json(
    result: &crate::provider::commands::ProviderListResult,
) -> String {
    let provider_list: Vec<_> = result
        .providers
        .iter()
        .map(|provider| {
            let type_str = crate::provider::profile::provider_type_slug(provider.provider_type);
            json!({
                "provider_name": provider.provider_name.as_deref().unwrap_or("unknown"),
                "provider_type": type_str,
                "model": provider.model,
                "endpoint": provider.endpoint,
            })
        })
        .collect();
    let out = json!({ "providers": provider_list, "total": result.providers.len() });
    serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
}

/// Format provider show result as text
fn format_provider_show_result_text(
    result: &crate::provider::commands::ProviderShowResult,
) -> String {
    let provider = &result.config;
    let mut output = format!(
        "Provider: {}\n",
        provider.provider_name.as_deref().unwrap_or("unknown")
    );
    let type_str = crate::provider::profile::provider_type_slug(provider.provider_type);
    output.push_str(&format!("Type: {}\n", type_str));
    output.push_str(&format!("Model: {}\n", provider.model));
    if let Some(endpoint) = &provider.endpoint {
        output.push_str(&format!("Endpoint: {}\n", endpoint));
    } else {
        output.push_str("Endpoint: (default endpoint)\n");
    }
    if let Some(status) = &result.api_key_status {
        output.push_str(&format!("API Key: {}\n", status));
    }
    output.push_str("\nDefault Completion Options:\n");
    if let Some(temp) = provider.default_options.temperature {
        output.push_str(&format!("  temperature: {}\n", temp));
    }
    if let Some(max_tokens) = provider.default_options.max_tokens {
        output.push_str(&format!("  max_tokens: {}\n", max_tokens));
    }
    if let Some(top_p) = provider.default_options.top_p {
        output.push_str(&format!("  top_p: {}\n", top_p));
    }
    if let Some(freq_penalty) = provider.default_options.frequency_penalty {
        output.push_str(&format!("  frequency_penalty: {}\n", freq_penalty));
    }
    if let Some(pres_penalty) = provider.default_options.presence_penalty {
        output.push_str(&format!("  presence_penalty: {}\n", pres_penalty));
    }
    if let Some(ref stop) = provider.default_options.stop {
        output.push_str(&format!("  stop: {:?}\n", stop));
    }

    output
}

/// Format provider show result as JSON
fn format_provider_show_result_json(
    result: &crate::provider::commands::ProviderShowResult,
) -> String {
    let provider = &result.config;
    let type_str = crate::provider::profile::provider_type_slug(provider.provider_type);
    let api_key_status_str = result.api_key_status.as_deref().map(|s| match s {
        s if s.contains("from config") => "set_from_config",
        s if s.contains("from environment") => "set_from_env",
        s if s.contains("Not set") => "not_set",
        s if s.contains("Not required") => "not_required",
        _ => "unknown",
    });
    let default_options = json!({
        "temperature": provider.default_options.temperature,
        "max_tokens": provider.default_options.max_tokens,
        "top_p": provider.default_options.top_p,
        "frequency_penalty": provider.default_options.frequency_penalty,
        "presence_penalty": provider.default_options.presence_penalty,
        "stop": provider.default_options.stop,
    });
    let out = json!({
        "provider_name": provider.provider_name.as_deref().unwrap_or("unknown"),
        "provider_type": type_str,
        "model": provider.model,
        "endpoint": provider.endpoint,
        "api_key_status": api_key_status_str,
        "default_options": default_options,
    });
    serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
}

/// Format provider validation result
fn format_provider_validation_result(
    result: &crate::provider::ValidationResult,
    verbose: bool,
) -> String {
    let mut output = format!("Validating provider: {}\n\n", result.provider_name);

    if result.errors.is_empty() && result.checks.iter().all(|(_, passed)| *passed) {
        output.push_str("✓ All validation checks passed\n\n");
    } else {
        // Show checks
        for (description, passed) in &result.checks {
            if *passed {
                output.push_str(&format!("✓ {}\n", description));
            } else {
                output.push_str(&format!("✗ {}\n", description));
            }
        }

        // Show errors
        if !result.errors.is_empty() {
            output.push_str("\nErrors:\n");
            for error in &result.errors {
                output.push_str(&format!("✗ {}\n", error));
            }
        }

        // Show warnings
        if !result.warnings.is_empty() {
            output.push_str("\nWarnings:\n");
            for warning in &result.warnings {
                output.push_str(&format!("⚠ {}\n", warning));
            }
        }

        output.push_str(&format!(
            "\nValidation {}: {}/{} checks passed, {} errors found\n",
            if result.is_valid() {
                "passed"
            } else {
                "failed"
            },
            result.passed_checks(),
            result.total_checks(),
            result.errors.len()
        ));
    }

    if verbose {
        output.push_str(&format!("\nTotal checks: {}\n", result.total_checks()));
        output.push_str(&format!("Passed: {}\n", result.passed_checks()));
        output.push_str(&format!("Errors: {}\n", result.errors.len()));
        output.push_str(&format!("Warnings: {}\n", result.warnings.len()));
    }

    output
}

/// Format provider test result as text
fn format_provider_test_result(
    result: &crate::provider::commands::ProviderTestResult,
    elapsed_ms: Option<u128>,
) -> String {
    let mut output = format!("Testing provider: {}\n\n", result.provider_name);
    output.push_str("✓ Provider client created\n");
    if result.connectivity_ok {
        output.push_str(&match elapsed_ms {
            Some(ms) => format!("✓ API connectivity: OK ({}ms)\n", ms),
            None => "✓ API connectivity: OK\n".to_string(),
        });
        if result.model_available {
            output.push_str(&format!("✓ Model '{}' is available\n", result.model_checked));
        } else {
            output.push_str(&format!("✗ Model '{}' not found\n", result.model_checked));
            output.push_str(&format!(
                "Available models: {}\n",
                result.available_models.join(", ")
            ));
            return output;
        }
    } else {
        if let Some(ref msg) = result.error_message {
            output.push_str(&format!("✗ API connectivity failed: {}\n", msg));
        }
        return output;
    }
    output.push_str("\nProvider is working correctly.\n");
    output
}

/// Format context output as text
fn format_context_text_output(
    context: &crate::api::NodeContext,
    include_metadata: bool,
    combine: bool,
    separator: &str,
    include_deleted: bool,
) -> Result<String, ApiError> {
    // Filter deleted frames if not including them
    let frames: Vec<&crate::context::frame::Frame> = if include_deleted {
        context.frames.iter().collect()
    } else {
        context
            .frames
            .iter()
            .filter(|f| {
                !f.metadata
                    .get("deleted")
                    .map(|v| v == "true")
                    .unwrap_or(false)
            })
            .collect()
    };

    if frames.is_empty() {
        return Ok(format!(
            "Node: {}\nPath: {}\nNo frames found.",
            hex::encode(context.node_id),
            context.node_record.path.display()
        ));
    }

    if combine {
        // Concatenate all frame contents
        let texts: Vec<String> = frames
            .iter()
            .filter_map(|f| f.text_content().ok())
            .collect();
        Ok(texts.join(separator))
    } else {
        // Show frames individually
        let mut output = format!(
            "Node: {}\nPath: {}\nFrames: {}/{}\n\n",
            hex::encode(context.node_id),
            context.node_record.path.display(),
            frames.len(),
            context.frame_count
        );

        for (i, frame) in frames.iter().enumerate() {
            output.push_str(&format!("--- Frame {} ---\n", i + 1));

            if include_metadata {
                output.push_str(&format!("Frame ID: {}\n", hex::encode(frame.frame_id)));
                output.push_str(&format!("Frame Type: {}\n", frame.frame_type));
                if let Some(agent_id) = frame.agent_id() {
                    output.push_str(&format!("Agent: {}\n", agent_id));
                }
                output.push_str(&format!("Timestamp: {:?}\n", frame.timestamp));
                if !frame.metadata.is_empty() {
                    output.push_str("Metadata:\n");
                    for (key, value) in &frame.metadata {
                        if key != "agent_id" && key != "deleted" {
                            output.push_str(&format!("  {}: {}\n", key, value));
                        }
                    }
                }
                output.push_str("\n");
            }

            if let Ok(text) = frame.text_content() {
                output.push_str(&format!("Content:\n{}\n", text));
            } else {
                output.push_str("Content: [Binary content - not UTF-8]\n");
            }
            output.push_str("\n");
        }

        Ok(output)
    }
}

/// Format context output as JSON
fn format_context_json_output(
    context: &crate::api::NodeContext,
    include_metadata: bool,
    include_deleted: bool,
) -> Result<String, ApiError> {
    use serde_json::json;

    // Filter deleted frames if not including them
    let frames: Vec<&crate::context::frame::Frame> = if include_deleted {
        context.frames.iter().collect()
    } else {
        context
            .frames
            .iter()
            .filter(|f| {
                !f.metadata
                    .get("deleted")
                    .map(|v| v == "true")
                    .unwrap_or(false)
            })
            .collect()
    };

    let frames_json: Vec<serde_json::Value> = frames
        .iter()
        .map(|frame| {
            let mut frame_obj = json!({
                "frame_id": hex::encode(frame.frame_id),
                "frame_type": frame.frame_type,
                "timestamp": frame.timestamp,
            });

            if include_metadata {
                if let Some(agent_id) = frame.agent_id() {
                    frame_obj["agent_id"] = json!(agent_id);
                }
                frame_obj["metadata"] = json!(frame.metadata);
            }

            if let Ok(text) = frame.text_content() {
                frame_obj["content"] = json!(text);
            } else {
                frame_obj["content"] = json!(null);
                frame_obj["content_binary"] = json!(true);
            }

            frame_obj
        })
        .collect();

    let result = json!({
        "node_id": hex::encode(context.node_id),
        "path": context.node_record.path.to_string_lossy(),
        "node_type": match context.node_record.node_type {
            crate::store::NodeType::File { size, .. } => format!("file:{}", size),
            crate::store::NodeType::Directory => "directory".to_string(),
        },
        "frames": frames_json,
        "frame_count": frames.len(),
        "total_frame_count": context.frame_count,
    });

    serde_json::to_string_pretty(&result)
        .map_err(|e| ApiError::ConfigError(format!("Failed to serialize JSON: {}", e)))
}

fn command_name(command: &Commands) -> String {
    match command {
        Commands::Scan { .. } => "scan".to_string(),
        Commands::Workspace { command } => format!("workspace.{}", workspace_command_name(command)),
        Commands::Status { .. } => "status".to_string(),
        Commands::Validate => "validate".to_string(),
        Commands::Watch { .. } => "watch".to_string(),
        Commands::Agent { command } => format!("agent.{}", agent_command_name(command)),
        Commands::Provider { command } => format!("provider.{}", provider_command_name(command)),
        Commands::Init { .. } => "init".to_string(),
        Commands::Context { command } => format!("context.{}", context_command_name(command)),
    }
}

fn workspace_command_name(command: &WorkspaceCommands) -> &'static str {
    match command {
        WorkspaceCommands::Status { .. } => "status",
        WorkspaceCommands::Validate { .. } => "validate",
        WorkspaceCommands::Ignore { .. } => "ignore",
        WorkspaceCommands::Delete { .. } => "delete",
        WorkspaceCommands::Restore { .. } => "restore",
        WorkspaceCommands::Compact { .. } => "compact",
        WorkspaceCommands::ListDeleted { .. } => "list_deleted",
    }
}

fn context_command_name(command: &ContextCommands) -> &'static str {
    match command {
        ContextCommands::Generate { .. } => "generate",
        ContextCommands::Get { .. } => "get",
    }
}

fn provider_command_name(command: &ProviderCommands) -> &'static str {
    match command {
        ProviderCommands::Status { .. } => "status",
        ProviderCommands::List { .. } => "list",
        ProviderCommands::Show { .. } => "show",
        ProviderCommands::Create { .. } => "create",
        ProviderCommands::Edit { .. } => "edit",
        ProviderCommands::Remove { .. } => "remove",
        ProviderCommands::Validate { .. } => "validate",
        ProviderCommands::Test { .. } => "test",
    }
}

fn agent_command_name(command: &AgentCommands) -> &'static str {
    match command {
        AgentCommands::Status { .. } => "status",
        AgentCommands::List { .. } => "list",
        AgentCommands::Show { .. } => "show",
        AgentCommands::Create { .. } => "create",
        AgentCommands::Edit { .. } => "edit",
        AgentCommands::Remove { .. } => "remove",
        AgentCommands::Validate { .. } => "validate",
    }
}

fn summary_descriptor(command: &Commands) -> SummaryCommandDescriptor {
    match command {
        Commands::Workspace { command } => match command {
            WorkspaceCommands::Status { format, breakdown } => {
                SummaryCommandDescriptor::WorkspaceStatus {
                    format: format.clone(),
                    breakdown: *breakdown,
                }
            }
            WorkspaceCommands::Validate { format } => SummaryCommandDescriptor::WorkspaceValidate {
                format: format.clone(),
            },
            WorkspaceCommands::Delete {
                path,
                node,
                dry_run,
                no_ignore,
            } => SummaryCommandDescriptor::WorkspaceDelete {
                target_path: path.is_some(),
                target_node: node.is_some(),
                dry_run: *dry_run,
                no_ignore: *no_ignore,
            },
            WorkspaceCommands::Restore {
                path,
                node,
                dry_run,
            } => SummaryCommandDescriptor::WorkspaceRestore {
                target_path: path.is_some(),
                target_node: node.is_some(),
                dry_run: *dry_run,
            },
            WorkspaceCommands::Compact {
                ttl,
                all,
                keep_frames,
                dry_run,
            } => SummaryCommandDescriptor::WorkspaceCompact {
                ttl_days: *ttl,
                all: *all,
                keep_frames: *keep_frames,
                dry_run: *dry_run,
            },
            WorkspaceCommands::ListDeleted { older_than, format } => {
                SummaryCommandDescriptor::WorkspaceListDeleted {
                    older_than_days: *older_than,
                    format: format.clone(),
                }
            }
            WorkspaceCommands::Ignore {
                path,
                dry_run,
                format,
            } => SummaryCommandDescriptor::WorkspaceIgnore {
                has_path: path.is_some(),
                dry_run: *dry_run,
                format: format.clone(),
            },
        },
        Commands::Status {
            format,
            workspace_only,
            agents_only,
            providers_only,
            breakdown,
            test_connectivity,
        } => {
            let include_all = !workspace_only && !agents_only && !providers_only;
            SummaryCommandDescriptor::StatusUnified {
                format: format.clone(),
                include_workspace: include_all || *workspace_only,
                include_agents: include_all || *agents_only,
                include_providers: include_all || *providers_only,
                breakdown: *breakdown,
                test_connectivity: *test_connectivity,
            }
        }
        Commands::Validate => SummaryCommandDescriptor::ValidateWorkspace,
        Commands::Agent { command } => SummaryCommandDescriptor::AgentAction {
            action: agent_command_name(command).to_string(),
            mutation: matches!(
                command,
                AgentCommands::Create { .. }
                    | AgentCommands::Edit { .. }
                    | AgentCommands::Remove { .. }
            ),
        },
        Commands::Provider { command } => SummaryCommandDescriptor::ProviderAction {
            action: provider_command_name(command).to_string(),
            mutation: matches!(
                command,
                ProviderCommands::Create { .. }
                    | ProviderCommands::Edit { .. }
                    | ProviderCommands::Remove { .. }
            ),
        },
        Commands::Init { force, list } => SummaryCommandDescriptor::Init {
            force: *force,
            list_only: *list,
        },
        _ => SummaryCommandDescriptor::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ContextApi;
    use crate::context::frame::storage::FrameStorage;
    use crate::heads::HeadIndex;
    use crate::store::persistence::SledNodeRecordStore;
    use crate::types::Hash;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_api() -> (ContextApi, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store");
        let node_store = Arc::new(SledNodeRecordStore::new(&store_path).unwrap());
        let frame_storage_path = temp_dir.path().join("frames");
        std::fs::create_dir_all(&frame_storage_path).unwrap();
        let frame_storage = Arc::new(FrameStorage::new(&frame_storage_path).unwrap());
        let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
        let agent_registry = Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new()));
        let provider_registry = Arc::new(parking_lot::RwLock::new(
            crate::provider::ProviderRegistry::new(),
        ));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());

        let api = ContextApi::new(
            node_store,
            frame_storage,
            head_index,
            agent_registry,
            provider_registry,
            lock_manager,
        );

        (api, temp_dir)
    }

    #[test]
    fn test_parse_node_id_valid() {
        let node_id = [1u8; 32];
        let hex_str = hex::encode(node_id);
        let parsed = parse_node_id(&hex_str).unwrap();
        assert_eq!(parsed, Hash::from(node_id));
    }

    #[test]
    fn test_parse_node_id_with_prefix() {
        let node_id = [1u8; 32];
        let hex_str = format!("0x{}", hex::encode(node_id));
        let parsed = parse_node_id(&hex_str).unwrap();
        assert_eq!(parsed, Hash::from(node_id));
    }

    #[test]
    fn test_parse_node_id_invalid() {
        // Invalid hex
        assert!(parse_node_id("not-hex").is_err());

        // Wrong length
        let short_hex = hex::encode([1u8; 16]);
        assert!(parse_node_id(&short_hex).is_err());
    }

    #[test]
    fn test_resolve_path_to_node_id() {
        let (api, temp_dir) = create_test_api();
        let workspace_root = temp_dir.path().to_path_buf();

        // Create a test node record
        let node_id: NodeID = [1u8; 32];
        let test_path = workspace_root.join("test.txt");
        std::fs::write(&test_path, "test content").unwrap();

        let canonical_path = crate::tree::path::canonicalize_path(&test_path).unwrap();

        let record = crate::store::NodeRecord {
            node_id,
            path: canonical_path.clone(),
            node_type: crate::store::NodeType::File {
                size: 12,
                content_hash: [0u8; 32],
            },
            children: vec![],
            parent: None,
            frame_set_root: None,
            metadata: std::collections::HashMap::new(),
            tombstoned_at: None,
        };

        api.node_store().put(&record).unwrap();

        // Test path resolution
        let resolved = crate::workspace::resolve_workspace_node_id(
            &api, &workspace_root, Some(test_path.as_path()), None, false,
        )
        .unwrap();
        assert_eq!(resolved, node_id);
    }

    #[test]
    fn test_resolve_path_to_node_id_not_found() {
        let (api, temp_dir) = create_test_api();
        let workspace_root = temp_dir.path().to_path_buf();

        // Create the file but don't add it to the store
        let test_path = workspace_root.join("nonexistent.txt");
        std::fs::write(&test_path, "test content").unwrap();

        let result = crate::workspace::resolve_workspace_node_id(
            &api, &workspace_root, Some(test_path.as_path()), None, false,
        );
        assert!(result.is_err());
        match result {
            Err(ApiError::PathNotInTree(_)) => {}
            _ => panic!("Expected PathNotInTree error, got: {:?}", result),
        }
    }

    #[test]
    fn test_resolve_path_to_node_id_fallback_for_relative_stored_path() {
        let (api, temp_dir) = create_test_api();
        let workspace_root = temp_dir.path().to_path_buf();

        let dir_path = workspace_root.join("src").join("generation");
        std::fs::create_dir_all(&dir_path).unwrap();

        let node_id: NodeID = [7u8; 32];
        let record = crate::store::NodeRecord {
            node_id,
            // Simulate legacy relative paths stored in the node index.
            path: std::path::PathBuf::from("./src/generation"),
            node_type: crate::store::NodeType::Directory,
            children: vec![],
            parent: None,
            frame_set_root: None,
            metadata: std::collections::HashMap::new(),
            tombstoned_at: None,
        };
        api.node_store().put(&record).unwrap();

        let resolved = crate::workspace::resolve_workspace_node_id(
            &api, &workspace_root, Some(dir_path.as_path()), None, false,
        )
        .unwrap();
        assert_eq!(resolved, node_id);
    }

    #[test]
    fn test_format_context_text_output_combine() {
        let node_id: NodeID = [1u8; 32];
        let node_record = crate::store::NodeRecord {
            node_id,
            path: std::path::PathBuf::from("/test/file.txt"),
            node_type: crate::store::NodeType::File {
                size: 100,
                content_hash: [0u8; 32],
            },
            children: vec![],
            parent: None,
            frame_set_root: None,
            metadata: std::collections::HashMap::new(),
            tombstoned_at: None,
        };

        let frame1 = crate::context::frame::Frame::new(
            crate::context::frame::Basis::Node(node_id),
            b"Frame 1 content".to_vec(),
            "type1".to_string(),
            "agent1".to_string(),
            std::collections::HashMap::new(),
        )
        .unwrap();

        let frame2 = crate::context::frame::Frame::new(
            crate::context::frame::Basis::Node(node_id),
            b"Frame 2 content".to_vec(),
            "type2".to_string(),
            "agent2".to_string(),
            std::collections::HashMap::new(),
        )
        .unwrap();

        let context = crate::api::NodeContext {
            node_id,
            node_record,
            frames: vec![frame1, frame2],
            frame_count: 2,
        };

        let output = format_context_text_output(&context, false, true, " | ", false).unwrap();
        assert!(output.contains("Frame 1 content"));
        assert!(output.contains("Frame 2 content"));
        assert!(output.contains(" | "));
    }

    #[test]
    fn test_format_context_json_output() {
        let node_id: NodeID = [1u8; 32];
        let node_record = crate::store::NodeRecord {
            node_id,
            path: std::path::PathBuf::from("/test/file.txt"),
            node_type: crate::store::NodeType::File {
                size: 100,
                content_hash: [0u8; 32],
            },
            children: vec![],
            parent: None,
            frame_set_root: None,
            metadata: std::collections::HashMap::new(),
            tombstoned_at: None,
        };

        let frame = crate::context::frame::Frame::new(
            crate::context::frame::Basis::Node(node_id),
            b"Test content".to_vec(),
            "test".to_string(),
            "agent1".to_string(),
            std::collections::HashMap::new(),
        )
        .unwrap();

        let context = crate::api::NodeContext {
            node_id,
            node_record,
            frames: vec![frame],
            frame_count: 1,
        };

        let output = format_context_json_output(&context, false, false).unwrap();
        assert!(output.contains("node_id"));
        assert!(output.contains("frames"));
        assert!(output.contains("Test content"));
    }
}

/// Parse a hex string to NodeID
fn parse_node_id(s: &str) -> Result<NodeID, ApiError> {
    // Remove 0x prefix if present
    let s = s.strip_prefix("0x").unwrap_or(s);

    // Parse hex string to bytes
    let bytes =
        hex::decode(s).map_err(|e| ApiError::InvalidFrame(format!("Invalid hex string: {}", e)))?;

    if bytes.len() != 32 {
        return Err(ApiError::InvalidFrame(format!(
            "NodeID must be 32 bytes, got {} bytes",
            bytes.len()
        )));
    }

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(Hash::from(hash))
}

fn format_validate_result_text(result: &ValidateResult) -> String {
    if result.errors.is_empty() && result.warnings.is_empty() {
        format!(
            "Validation passed:\n  Root hash: {}\n  Nodes: {}\n  Frames: {}\n  All checks passed",
            result.root_hash, result.node_count, result.frame_count
        )
    } else {
        let mut s = format!(
            "Validation completed with issues:\n  Root hash: {}\n  Nodes: {}\n  Frames: {}",
            result.root_hash, result.node_count, result.frame_count
        );
        if !result.errors.is_empty() {
            s.push_str(&format!("\n\nErrors ({}):", result.errors.len()));
            for e in &result.errors {
                s.push_str(&format!("\n  - {}", e));
            }
        }
        if !result.warnings.is_empty() {
            s.push_str(&format!("\n\nWarnings ({}):", result.warnings.len()));
            for w in &result.warnings {
                s.push_str(&format!("\n  - {}", w));
            }
        }
        s
    }
}

fn format_ignore_result(result: &IgnoreResult, format: &str) -> Result<String, ApiError> {
    match (result, format) {
        (IgnoreResult::List { entries }, "json") => {
            let out = serde_json::json!({ "ignored": entries });
            serde_json::to_string_pretty(&out).map_err(|e| {
                ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string()))
            })
        }
        (IgnoreResult::List { entries }, _) => {
            if entries.is_empty() {
                Ok("Ignore list is empty.".to_string())
            } else {
                let mut lines: Vec<String> = entries
                    .iter()
                    .enumerate()
                    .map(|(i, p)| format!("  {}. {}", i + 1, p))
                    .collect();
                lines.insert(0, "Ignore list:".to_string());
                Ok(lines.join("\n"))
            }
        }
        (IgnoreResult::Added { path }, _) => Ok(path.clone()),
    }
}

fn format_list_deleted_result(
    result: &ListDeletedResult,
    format: &str,
) -> Result<String, ApiError> {
    if format == "json" {
        let arr: Vec<serde_json::Value> = result
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "path": r.path,
                    "node_id": r.node_id_short,
                    "tombstoned_at": r.tombstoned_at,
                    "age": r.age
                })
            })
            .collect();
        return serde_json::to_string_pretty(&arr).map_err(|e| {
            ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string()))
        });
    }
    use comfy_table::Table;
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec!["Path", "Node ID", "Tombstoned At", "Age"]);
    for r in &result.rows {
        let ts_str = if r.tombstoned_at > 0 {
            format!("{}", r.tombstoned_at)
        } else {
            "-".to_string()
        };
        table.add_row(vec![&r.path, &r.node_id_short, &ts_str, &r.age]);
    }
    Ok(table.to_string())
}

/// Format initialization preview
fn format_init_preview(preview: &crate::init::InitPreview) -> String {
    let mut output = String::from("Initialization Preview:\n\n");

    if !preview.prompts.is_empty() {
        output.push_str("Would create prompts:\n");
        for prompt in &preview.prompts {
            output.push_str(&format!("  - {}\n", prompt));
        }
        output.push('\n');
    }

    if !preview.agents.is_empty() {
        output.push_str("Would create agents:\n");
        for agent in &preview.agents {
            output.push_str(&format!("  - {}.toml\n", agent));
        }
        output.push('\n');
    }

    if preview.prompts.is_empty() && preview.agents.is_empty() {
        output.push_str("All default agents and prompts already exist.\n");
    } else {
        output.push_str("Run 'merkle init' to perform initialization.\n");
    }

    output
}

/// Format initialization summary
fn format_init_summary(summary: &crate::init::InitSummary, force: bool) -> String {
    let mut output = String::from("Initializing Merkle configuration...\n\n");

    // Prompts section
    if !summary.prompts.created.is_empty() || !summary.prompts.skipped.is_empty() {
        let prompts_dir = crate::config::xdg::prompts_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.config/merkle/prompts/".to_string());
        output.push_str(&format!("Created prompts directory: {}\n", prompts_dir));

        for prompt in &summary.prompts.created {
            if force {
                output.push_str(&format!("  ✓ {} (overwritten)\n", prompt));
            } else {
                output.push_str(&format!("  ✓ {}\n", prompt));
            }
        }
        for prompt in &summary.prompts.skipped {
            output.push_str(&format!("  ⊘ {} (already exists, skipped)\n", prompt));
        }
        output.push('\n');
    }

    // Agents section
    if !summary.agents.created.is_empty() || !summary.agents.skipped.is_empty() {
        let agents_dir = crate::agent::XdgAgentStorage::new()
            .agents_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.config/merkle/agents/".to_string());
        output.push_str(&format!("Created agents directory: {}\n", agents_dir));

        for agent in &summary.agents.created {
            let role_str = match agent.as_str() {
                "reader" => "Reader",
                "code-analyzer" => "Writer",
                "docs-writer" => "Writer",
                _ => "Unknown",
            };
            if force {
                output.push_str(&format!(
                    "  ✓ {}.toml ({}) (overwritten)\n",
                    agent, role_str
                ));
            } else {
                output.push_str(&format!("  ✓ {}.toml ({})\n", agent, role_str));
            }
        }
        for agent in &summary.agents.skipped {
            let role_str = match agent.as_str() {
                "reader" => "Reader",
                "code-analyzer" => "Writer",
                "docs-writer" => "Writer",
                _ => "Unknown",
            };
            output.push_str(&format!(
                "  ⊘ {}.toml ({}) (already exists, skipped)\n",
                agent, role_str
            ));
        }
        output.push('\n');
    }

    // Errors section
    if !summary.prompts.errors.is_empty() || !summary.agents.errors.is_empty() {
        output.push_str("Errors:\n");
        for error in &summary.prompts.errors {
            output.push_str(&format!("  ✗ {}\n", error));
        }
        for error in &summary.agents.errors {
            output.push_str(&format!("  ✗ {}\n", error));
        }
        output.push('\n');
    }

    // Validation section
    let all_valid = summary
        .validation
        .results
        .iter()
        .all(|(_, is_valid, _)| *is_valid);
    if all_valid {
        output.push_str("Validation:\n");
        output.push_str("  ✓ All agents validated successfully\n\n");
    } else {
        output.push_str("Validation:\n");
        for (agent_id, is_valid, errors) in &summary.validation.results {
            if *is_valid {
                output.push_str(&format!("  ✓ {} validated\n", agent_id));
            } else {
                output.push_str(&format!("  ✗ {} validation failed:\n", agent_id));
                for error in errors {
                    output.push_str(&format!("    - {}\n", error));
                }
            }
        }
        output.push('\n');
    }

    if summary.prompts.created.is_empty() && summary.agents.created.is_empty() && !force {
        output.push_str("All default agents already exist. Use --force to re-initialize.\n");
    } else {
        output.push_str("Initialization complete! You can now use:\n");
        output.push_str("  - merkle agent list          # List all agents\n");
        output.push_str("  - merkle agent show <id>     # View agent details\n");
        output.push_str("  - merkle context generate    # Generate context frames\n");
    }

    output
}
