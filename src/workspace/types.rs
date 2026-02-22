//! Shared types for workspace commands and status.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parameters for workspace status command; aligns with agent/provider status pattern.
#[derive(Debug, Clone)]
pub struct WorkspaceStatusRequest {
    pub workspace_root: PathBuf,
    pub store_path: PathBuf,
    pub include_breakdown: bool,
}

/// Workspace status: not-scanned or scanned with tree, coverage, top paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    pub scanned: bool,
    pub store_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<TreeStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_coverage: Option<Vec<ContextCoverageEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_paths_by_node_count: Option<Vec<PathCount>>,
}

/// Result type for workspace status command; aligns with AgentStatusEntryResult / ProviderStatusEntryResult naming.
pub type WorkspaceStatusResult = WorkspaceStatus;

/// Tree section when scanned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeStatus {
    pub root_hash: String,
    pub total_nodes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<Vec<PathCount>>,
}

/// Path prefix and node count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCount {
    pub path: String,
    pub nodes: u64,
}

/// Per-agent context coverage when scanned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCoverageEntry {
    pub agent_id: String,
    pub nodes_with_frame: u64,
    pub nodes_without_frame: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_pct: Option<u64>,
}

// --- Agent status (for unified status) ---

/// One row for agent status table / JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusEntry {
    pub agent_id: String,
    pub role: String,
    pub valid: bool,
    pub prompt_path_exists: bool,
}

/// Agent status output for JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusOutput {
    pub agents: Vec<AgentStatusEntry>,
    pub total: usize,
    pub valid_count: usize,
}

// --- Provider status (for unified status) ---

/// One row for provider status table / JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusEntry {
    pub provider_name: String,
    pub provider_type: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connectivity: Option<String>,
}

/// Provider status output for JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusOutput {
    pub providers: Vec<ProviderStatusEntry>,
    pub total: usize,
}

// --- Unified status ---

/// Unified status output combining workspace, agents, and providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedStatusOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<AgentStatusOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<ProviderStatusOutput>,
}

// --- Command result DTOs (for CLI formatting) ---

/// Result of workspace validate command.
#[derive(Debug, Clone, Serialize)]
pub struct ValidateResult {
    pub valid: bool,
    pub root_hash: String,
    pub node_count: usize,
    pub frame_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Result of workspace ignore command: list entries or single added path.
#[derive(Debug, Clone, Serialize)]
pub enum IgnoreResult {
    List { entries: Vec<String> },
    Added { path: String },
}

/// One row for list_deleted output.
#[derive(Debug, Clone, Serialize)]
pub struct ListDeletedRow {
    pub path: String,
    pub node_id_short: String,
    pub tombstoned_at: u64,
    pub age: String,
}

/// Result of workspace list_deleted command.
#[derive(Debug, Clone, Serialize)]
pub struct ListDeletedResult {
    pub rows: Vec<ListDeletedRow>,
}
