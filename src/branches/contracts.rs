use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchKind {
    WorkspaceFs,
}

impl BranchKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WorkspaceFs => "workspace_fs",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchManifest {
    pub branch_id: String,
    pub branch_kind: BranchKind,
    pub canonical_locator: String,
    pub locator_version: u32,
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
pub struct BranchCatalog {
    #[serde(default)]
    pub branches: Vec<BranchCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchCatalogEntry {
    pub branch_id: String,
    pub branch_kind: BranchKind,
    pub canonical_locator: String,
    pub data_home_path: String,
    #[serde(default)]
    pub store_path: Option<String>,
    pub attachment_status: BranchAttachmentStatus,
    pub inspection_status: BranchInspectionStatus,
    pub migration_status: BranchMigrationStatus,
    pub last_seen_at: Option<String>,
    pub last_inspected_at: Option<String>,
    pub last_migration_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchAttachmentStatus {
    Active,
    Dormant,
    MissingWorkspacePath,
    Ambiguous,
}

impl BranchAttachmentStatus {
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
pub enum BranchInspectionStatus {
    Unknown,
    Registered,
    InspectionRequired,
    InvalidCandidate,
}

impl BranchInspectionStatus {
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
pub enum BranchMigrationStatus {
    Unknown,
    NotNeeded,
    ReplayNeeded,
    InProgress,
    Failed,
    Succeeded,
}

impl BranchMigrationStatus {
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
pub struct BranchMigrationLedgerEntry {
    pub plan_id: String,
    pub step_id: String,
    pub lane: BranchMigrationLane,
    pub status: BranchMigrationStepStatus,
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
pub enum BranchMigrationLane {
    Metadata,
    Derived,
    AuthoritativeCompatibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchMigrationStepStatus {
    Started,
    Verified,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchesStatusOutput {
    pub branches: Vec<BranchStatusRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchStatusRow {
    pub branch_id: String,
    pub canonical_locator: String,
    pub data_home_path: String,
    pub store_path: Option<String>,
    pub attachment_status: String,
    pub inspection_status: String,
    pub migration_status: String,
    pub last_seen_at: Option<String>,
    pub last_migration_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBranch {
    pub branch_id: String,
    pub branch_kind: BranchKind,
    pub canonical_locator: PathBuf,
    pub data_home_path: PathBuf,
    pub manifest_path: PathBuf,
    pub ledger_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchHandle {
    resolved: ResolvedBranch,
}

impl BranchHandle {
    pub fn new(resolved: ResolvedBranch) -> Self {
        Self { resolved }
    }

    pub fn resolved(&self) -> &ResolvedBranch {
        &self.resolved
    }

    pub fn branch_id(&self) -> &str {
        &self.resolved.branch_id
    }

    pub fn branch_kind(&self) -> &BranchKind {
        &self.resolved.branch_kind
    }

    pub fn canonical_locator(&self) -> &PathBuf {
        &self.resolved.canonical_locator
    }
}

impl From<ResolvedBranch> for BranchHandle {
    fn from(value: ResolvedBranch) -> Self {
        Self::new(value)
    }
}
