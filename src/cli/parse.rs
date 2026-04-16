//! CLI parse: clap types for Merkle. No behavior; definitions only.

use clap::{Parser, Subcommand};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Merkle CLI - Deterministic filesystem state management
#[derive(Parser)]
#[command(name = "meld")]
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

    /// Enable verbose logging and mirror logs to stderr unless output is explicitly set
    #[arg(long, default_value = "false")]
    pub verbose: bool,

    /// Disable logging (overrides config and default)
    #[arg(long, default_value = "false")]
    pub quiet: bool,

    /// Log level (trace, debug, info, warn, error, off)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Log format (json, text)
    #[arg(long)]
    pub log_format: Option<String>,

    /// Log output (stdout, stderr, file, file+stderr, both)
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
    /// Workflow operations
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    /// Branch discovery and migration status
    #[command(alias = "roots")]
    Branches {
        #[command(subcommand)]
        command: BranchesCommands,
    },
    /// Dangerous destructive operations for workspace runtime state
    Danger {
        #[command(subcommand)]
        command: DangerCommands,
    },
}

#[derive(Subcommand)]
pub enum BranchesCommands {
    /// Show known branches and migration status
    Status {
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Discover dormant branches from the global data home
    Discover {
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Migrate all registered branches safely
    Migrate {
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Attach one explicit branch path
    Attach {
        /// Workspace path to attach
        path: PathBuf,
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Show federated graph readiness for selected branches
    GraphStatus {
        /// Branch scope to query
        #[arg(long, default_value = "all")]
        scope: String,
        /// One or more explicit branch ids when scope is branch
        #[arg(long = "branch-id")]
        branch_ids: Vec<String>,
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Query federated neighbors across selected branches
    GraphNeighbors {
        /// Branch scope to query
        #[arg(long, default_value = "all")]
        scope: String,
        /// One or more explicit branch ids when scope is branch
        #[arg(long = "branch-id")]
        branch_ids: Vec<String>,
        /// Domain id for the start object
        #[arg(long)]
        domain: String,
        /// Object kind for the start object
        #[arg(long = "object-kind")]
        object_kind: String,
        /// Object id for the start object
        #[arg(long = "object-id")]
        object_id: String,
        /// Traversal direction
        #[arg(long, default_value = "both")]
        direction: String,
        /// Relation type filters
        #[arg(long = "relation-type")]
        relation_types: Vec<String>,
        /// Limit to current relations only
        #[arg(long)]
        current_only: bool,
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Query federated graph walks across selected branches
    GraphWalk {
        /// Branch scope to query
        #[arg(long, default_value = "all")]
        scope: String,
        /// One or more explicit branch ids when scope is branch
        #[arg(long = "branch-id")]
        branch_ids: Vec<String>,
        /// Domain id for the start object
        #[arg(long)]
        domain: String,
        /// Object kind for the start object
        #[arg(long = "object-kind")]
        object_kind: String,
        /// Object id for the start object
        #[arg(long = "object-id")]
        object_id: String,
        /// Traversal direction
        #[arg(long, default_value = "both")]
        direction: String,
        /// Relation type filters
        #[arg(long = "relation-type")]
        relation_types: Vec<String>,
        /// Maximum traversal depth
        #[arg(long, default_value_t = 1)]
        max_depth: usize,
        /// Limit to current relations only
        #[arg(long)]
        current_only: bool,
        /// Include facts in the walk response
        #[arg(long)]
        include_facts: bool,
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
}

pub type RootsCommands = BranchesCommands;

#[derive(Subcommand)]
pub enum DangerCommands {
    /// Remove all workspace runtime state except logs
    Flush {
        /// Target workspace path
        #[arg(long, value_name = "PATH", conflicts_with = "path_positional")]
        path: Option<PathBuf>,

        /// Target workspace path
        #[arg(value_name = "PATH", index = 1, conflicts_with = "path")]
        path_positional: Option<PathBuf>,

        /// Show what would be removed without deleting
        #[arg(long)]
        dry_run: bool,

        /// Confirm destructive deletion of runtime state
        #[arg(long)]
        yes: bool,
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
        /// Report counts without compaction
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
    /// Edit agent prompt files
    Prompt {
        #[command(subcommand)]
        command: AgentPromptCommands,
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
pub enum AgentPromptCommands {
    /// Show the prompt file for an agent
    Show {
        /// Agent ID
        agent_id: String,
    },
    /// Edit the prompt file for an agent
    Edit {
        /// Agent ID
        agent_id: String,
        /// Editor to use (default: $EDITOR)
        #[arg(long)]
        editor: Option<String>,
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
        #[arg(long, name = "type")]
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

        /// Override workflow id for this run (bypasses agent workflow binding)
        #[arg(long)]
        workflow_id: Option<String>,

        /// Override provider model for this run only
        #[arg(long)]
        provider_model: Option<String>,

        /// Path to JSON object merged into provider additional_json for this run only
        #[arg(long, value_name = "JSON_PATH")]
        provider_additional_json_file: Option<PathBuf>,

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
    /// Re generate a context frame for a node and prefer directory only reroll
    Regenerate {
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

        /// Override workflow id for this run (bypasses agent workflow binding)
        #[arg(long)]
        workflow_id: Option<String>,

        /// Override provider model for this run only
        #[arg(long)]
        provider_model: Option<String>,

        /// Path to JSON object merged into provider additional_json for this run only
        #[arg(long, value_name = "JSON_PATH")]
        provider_additional_json_file: Option<PathBuf>,

        /// Frame type (defaults to context-<agent_id>)
        #[arg(long)]
        frame_type: Option<String>,

        /// Regenerate directory target recursively instead of only rerolling the directory frame
        #[arg(long)]
        recursive: bool,
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

pub fn parse_provider_additional_json_file(
    path: Option<&PathBuf>,
) -> Result<Option<BTreeMap<String, Value>>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let content = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "Failed to read provider additional json file {}: {}",
            path.display(),
            e
        )
    })?;
    let value: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))?;
    let obj = value.as_object().ok_or_else(|| {
        format!(
            "Provider additional json file {} must contain a top-level JSON object",
            path.display()
        )
    })?;
    Ok(Some(
        obj.iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    ))
}

#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// List resolved workflows with source metadata
    List {
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Validate workflow profile registry loading and schema
    Validate {
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Inspect one workflow profile
    Inspect {
        /// Workflow ID
        workflow_id: String,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Execute one workflow profile for a target node
    Execute {
        /// Workflow ID
        workflow_id: String,

        /// Target node by NodeID hex string
        #[arg(long, conflicts_with_all = ["path", "path_positional"])]
        node: Option<String>,

        /// Target node by workspace relative or absolute path
        #[arg(long, value_name = "PATH", conflicts_with = "node")]
        path: Option<PathBuf>,

        /// Target path positional form same as --path
        #[arg(value_name = "PATH", index = 1, conflicts_with = "node")]
        path_positional: Option<PathBuf>,

        /// Agent ID used for workflow execution
        #[arg(long)]
        agent: String,

        /// Provider name used for workflow execution
        #[arg(long)]
        provider: String,

        /// Frame type defaults to context-agent
        #[arg(long)]
        frame_type: Option<String>,

        /// Generate even if thread state can reuse existing head
        #[arg(long)]
        force: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::{BranchesCommands, Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_roots_status_command_alias() {
        let cli = Cli::try_parse_from(["meld", "roots", "status", "--format", "json"]).unwrap();
        match cli.command {
            Commands::Branches {
                command: BranchesCommands::Status { format },
            } => assert_eq!(format, "json"),
            _ => panic!("expected branches status command"),
        }
    }

    #[test]
    fn parses_branches_status_command() {
        let cli = Cli::try_parse_from(["meld", "branches", "status", "--format", "json"]).unwrap();
        match cli.command {
            Commands::Branches {
                command: BranchesCommands::Status { format },
            } => assert_eq!(format, "json"),
            _ => panic!("expected branches status command"),
        }
    }

    #[test]
    fn parses_branches_attach_command() {
        let cli =
            Cli::try_parse_from(["meld", "branches", "attach", "/tmp/ws", "--format", "json"])
                .unwrap();
        match cli.command {
            Commands::Branches {
                command: BranchesCommands::Attach { path, format },
            } => {
                assert_eq!(path, PathBuf::from("/tmp/ws"));
                assert_eq!(format, "json");
            }
            _ => panic!("expected branches attach command"),
        }
    }

    #[test]
    fn parses_branches_graph_neighbors_command() {
        let cli = Cli::try_parse_from([
            "meld",
            "branches",
            "graph-neighbors",
            "--scope",
            "branch",
            "--branch-id",
            "branch-a",
            "--domain",
            "workspace_fs",
            "--object-kind",
            "node",
            "--object-id",
            "node-a",
            "--direction",
            "both",
            "--relation-type",
            "points_to",
            "--current-only",
            "--format",
            "json",
        ])
        .unwrap();
        match cli.command {
            Commands::Branches {
                command:
                    BranchesCommands::GraphNeighbors {
                        scope,
                        branch_ids,
                        domain,
                        object_kind,
                        object_id,
                        direction,
                        relation_types,
                        current_only,
                        format,
                    },
            } => {
                assert_eq!(scope, "branch");
                assert_eq!(branch_ids, vec!["branch-a".to_string()]);
                assert_eq!(domain, "workspace_fs");
                assert_eq!(object_kind, "node");
                assert_eq!(object_id, "node-a");
                assert_eq!(direction, "both");
                assert_eq!(relation_types, vec!["points_to".to_string()]);
                assert!(current_only);
                assert_eq!(format, "json");
            }
            _ => panic!("expected branches graph-neighbors command"),
        }
    }
}
