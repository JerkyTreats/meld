use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootManifest {
    pub root_id: String,
    pub workspace_path: String,
    pub workspace_locator_version: u32,
    pub authoritative_state_version: u32,
    pub derived_state_version: u32,
    pub migration_runtime_version: u32,
    pub last_seen_at: String,
    pub last_successful_plan_id: Option<String>,
    pub last_successful_step_id: Option<String>,
    #[serde(default)]
    pub last_reduced_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RootCatalog {
    #[serde(default)]
    pub roots: Vec<RootCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootCatalogEntry {
    pub root_id: String,
    pub workspace_path: String,
    pub data_home_path: String,
    pub attachment_status: RootAttachmentStatus,
    pub inspection_status: RootInspectionStatus,
    pub migration_status: RootMigrationStatus,
    pub last_seen_at: Option<String>,
    pub last_inspected_at: Option<String>,
    pub last_migration_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RootAttachmentStatus {
    Active,
    Dormant,
    MissingWorkspacePath,
    Ambiguous,
}

impl RootAttachmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Dormant => "dormant",
            Self::MissingWorkspacePath => "missing_workspace_path",
            Self::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RootInspectionStatus {
    Unknown,
    Registered,
    InspectionRequired,
    InvalidCandidate,
}

impl RootInspectionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Registered => "registered",
            Self::InspectionRequired => "inspection_required",
            Self::InvalidCandidate => "invalid_candidate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RootMigrationStatus {
    Unknown,
    NotNeeded,
    ReplayNeeded,
    InProgress,
    Failed,
    Succeeded,
}

impl RootMigrationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::NotNeeded => "not_needed",
            Self::ReplayNeeded => "replay_needed",
            Self::InProgress => "in_progress",
            Self::Failed => "failed",
            Self::Succeeded => "succeeded",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootMigrationLedgerEntry {
    pub plan_id: String,
    pub step_id: String,
    pub lane: RootMigrationLane,
    pub status: RootMigrationStepStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error_summary: Option<String>,
    #[serde(default)]
    pub observed_inputs: Vec<String>,
    #[serde(default)]
    pub verification_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RootMigrationLane {
    Metadata,
    Derived,
    AuthoritativeCompatibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RootMigrationStepStatus {
    Started,
    Verified,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootsStatusOutput {
    pub roots: Vec<RootStatusRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootStatusRow {
    pub root_id: String,
    pub workspace_path: String,
    pub data_home_path: String,
    pub attachment_status: String,
    pub inspection_status: String,
    pub migration_status: String,
    pub last_seen_at: Option<String>,
    pub last_migration_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub root_id: String,
    pub workspace_path: PathBuf,
    pub data_home_path: PathBuf,
    pub manifest_path: PathBuf,
    pub ledger_path: PathBuf,
}
