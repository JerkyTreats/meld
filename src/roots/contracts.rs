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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub last_reduced_seq: u64,
}

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

impl From<RootManifest> for BranchManifest {
    fn from(value: RootManifest) -> Self {
        Self {
            branch_id: value.root_id,
            branch_kind: BranchKind::WorkspaceFs,
            canonical_locator: value.workspace_path,
            locator_version: value.workspace_locator_version,
            authoritative_state_version: value.authoritative_state_version,
            derived_state_version: value.derived_state_version,
            migration_runtime_version: value.migration_runtime_version,
            last_seen_at: value.last_seen_at,
            last_successful_plan_id: value.last_successful_plan_id,
            last_successful_step_id: value.last_successful_step_id,
            last_reduced_seq: value.last_reduced_seq,
        }
    }
}

impl From<BranchManifest> for RootManifest {
    fn from(value: BranchManifest) -> Self {
        Self {
            root_id: value.branch_id,
            workspace_path: value.canonical_locator,
            workspace_locator_version: value.locator_version,
            authoritative_state_version: value.authoritative_state_version,
            derived_state_version: value.derived_state_version,
            migration_runtime_version: value.migration_runtime_version,
            last_seen_at: value.last_seen_at,
            last_successful_plan_id: value.last_successful_plan_id,
            last_successful_step_id: value.last_successful_step_id,
            last_reduced_seq: value.last_reduced_seq,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchCatalog {
    pub branches: Vec<BranchCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchCatalogEntry {
    pub branch_id: String,
    pub branch_kind: BranchKind,
    pub canonical_locator: String,
    pub data_home_path: String,
    pub attachment_status: BranchAttachmentStatus,
    pub inspection_status: BranchInspectionStatus,
    pub migration_status: BranchMigrationStatus,
    pub last_seen_at: Option<String>,
    pub last_inspected_at: Option<String>,
    pub last_migration_at: Option<String>,
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

impl From<RootCatalog> for BranchCatalog {
    fn from(value: RootCatalog) -> Self {
        Self {
            branches: value.roots.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<BranchCatalog> for RootCatalog {
    fn from(value: BranchCatalog) -> Self {
        Self {
            roots: value.branches.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RootCatalogEntry> for BranchCatalogEntry {
    fn from(value: RootCatalogEntry) -> Self {
        Self {
            branch_id: value.root_id,
            branch_kind: BranchKind::WorkspaceFs,
            canonical_locator: value.workspace_path,
            data_home_path: value.data_home_path,
            attachment_status: value.attachment_status.into(),
            inspection_status: value.inspection_status.into(),
            migration_status: value.migration_status.into(),
            last_seen_at: value.last_seen_at,
            last_inspected_at: value.last_inspected_at,
            last_migration_at: value.last_migration_at,
        }
    }
}

impl From<BranchCatalogEntry> for RootCatalogEntry {
    fn from(value: BranchCatalogEntry) -> Self {
        Self {
            root_id: value.branch_id,
            workspace_path: value.canonical_locator,
            data_home_path: value.data_home_path,
            attachment_status: value.attachment_status.into(),
            inspection_status: value.inspection_status.into(),
            migration_status: value.migration_status.into(),
            last_seen_at: value.last_seen_at,
            last_inspected_at: value.last_inspected_at,
            last_migration_at: value.last_migration_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl From<RootAttachmentStatus> for BranchAttachmentStatus {
    fn from(value: RootAttachmentStatus) -> Self {
        match value {
            RootAttachmentStatus::Active => Self::Active,
            RootAttachmentStatus::Dormant => Self::Dormant,
            RootAttachmentStatus::MissingWorkspacePath => Self::MissingWorkspacePath,
            RootAttachmentStatus::Ambiguous => Self::Ambiguous,
        }
    }
}

impl From<BranchAttachmentStatus> for RootAttachmentStatus {
    fn from(value: BranchAttachmentStatus) -> Self {
        match value {
            BranchAttachmentStatus::Active => Self::Active,
            BranchAttachmentStatus::Dormant => Self::Dormant,
            BranchAttachmentStatus::MissingWorkspacePath => Self::MissingWorkspacePath,
            BranchAttachmentStatus::Ambiguous => Self::Ambiguous,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl From<RootInspectionStatus> for BranchInspectionStatus {
    fn from(value: RootInspectionStatus) -> Self {
        match value {
            RootInspectionStatus::Unknown => Self::Unknown,
            RootInspectionStatus::Registered => Self::Registered,
            RootInspectionStatus::InspectionRequired => Self::InspectionRequired,
            RootInspectionStatus::InvalidCandidate => Self::InvalidCandidate,
        }
    }
}

impl From<BranchInspectionStatus> for RootInspectionStatus {
    fn from(value: BranchInspectionStatus) -> Self {
        match value {
            BranchInspectionStatus::Unknown => Self::Unknown,
            BranchInspectionStatus::Registered => Self::Registered,
            BranchInspectionStatus::InspectionRequired => Self::InspectionRequired,
            BranchInspectionStatus::InvalidCandidate => Self::InvalidCandidate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl From<RootMigrationStatus> for BranchMigrationStatus {
    fn from(value: RootMigrationStatus) -> Self {
        match value {
            RootMigrationStatus::Unknown => Self::Unknown,
            RootMigrationStatus::NotNeeded => Self::NotNeeded,
            RootMigrationStatus::ReplayNeeded => Self::ReplayNeeded,
            RootMigrationStatus::InProgress => Self::InProgress,
            RootMigrationStatus::Failed => Self::Failed,
            RootMigrationStatus::Succeeded => Self::Succeeded,
        }
    }
}

impl From<BranchMigrationStatus> for RootMigrationStatus {
    fn from(value: BranchMigrationStatus) -> Self {
        match value {
            BranchMigrationStatus::Unknown => Self::Unknown,
            BranchMigrationStatus::NotNeeded => Self::NotNeeded,
            BranchMigrationStatus::ReplayNeeded => Self::ReplayNeeded,
            BranchMigrationStatus::InProgress => Self::InProgress,
            BranchMigrationStatus::Failed => Self::Failed,
            BranchMigrationStatus::Succeeded => Self::Succeeded,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBranch {
    pub branch_id: String,
    pub branch_kind: BranchKind,
    pub canonical_locator: PathBuf,
    pub data_home_path: PathBuf,
    pub manifest_path: PathBuf,
    pub ledger_path: PathBuf,
}

impl From<ResolvedRoot> for ResolvedBranch {
    fn from(value: ResolvedRoot) -> Self {
        Self {
            branch_id: value.root_id,
            branch_kind: BranchKind::WorkspaceFs,
            canonical_locator: value.workspace_path,
            data_home_path: value.data_home_path,
            manifest_path: value.manifest_path,
            ledger_path: value.ledger_path,
        }
    }
}

impl From<ResolvedBranch> for ResolvedRoot {
    fn from(value: ResolvedBranch) -> Self {
        Self {
            root_id: value.branch_id,
            workspace_path: value.canonical_locator,
            data_home_path: value.data_home_path,
            manifest_path: value.manifest_path,
            ledger_path: value.ledger_path,
        }
    }
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

    pub fn as_root(&self) -> ResolvedRoot {
        self.resolved.clone().into()
    }
}

impl From<ResolvedRoot> for BranchHandle {
    fn from(value: ResolvedRoot) -> Self {
        Self::new(value.into())
    }
}

impl From<ResolvedBranch> for BranchHandle {
    fn from(value: ResolvedBranch) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BranchAttachmentStatus, BranchHandle, BranchInspectionStatus, BranchKind, BranchManifest,
        BranchMigrationStatus, ResolvedBranch, ResolvedRoot, RootCatalog, RootCatalogEntry,
        RootInspectionStatus, RootManifest, RootMigrationStatus,
    };
    use std::path::PathBuf;

    #[test]
    fn root_manifest_converts_to_branch_manifest() {
        let root_manifest = RootManifest {
            root_id: "root-1".to_string(),
            workspace_path: "/tmp/workspace".to_string(),
            workspace_locator_version: 1,
            authoritative_state_version: 2,
            derived_state_version: 3,
            migration_runtime_version: 4,
            last_seen_at: "2026-04-15T00:00:00Z".to_string(),
            last_successful_plan_id: Some("plan-1".to_string()),
            last_successful_step_id: Some("step-1".to_string()),
            last_reduced_seq: 9,
        };

        let branch_manifest = BranchManifest::from(root_manifest.clone());
        assert_eq!(branch_manifest.branch_id, "root-1");
        assert_eq!(branch_manifest.branch_kind, BranchKind::WorkspaceFs);
        assert_eq!(branch_manifest.canonical_locator, "/tmp/workspace");
        assert_eq!(RootManifest::from(branch_manifest), root_manifest);
    }

    #[test]
    fn root_catalog_converts_to_branch_catalog() {
        let root_catalog = RootCatalog {
            roots: vec![RootCatalogEntry {
                root_id: "root-1".to_string(),
                workspace_path: "/tmp/workspace".to_string(),
                data_home_path: "/tmp/data".to_string(),
                attachment_status: super::RootAttachmentStatus::Active,
                inspection_status: RootInspectionStatus::Registered,
                migration_status: RootMigrationStatus::Succeeded,
                last_seen_at: Some("2026-04-15T00:00:00Z".to_string()),
                last_inspected_at: Some("2026-04-15T00:00:00Z".to_string()),
                last_migration_at: Some("2026-04-15T00:00:00Z".to_string()),
            }],
        };

        let branch_catalog = super::BranchCatalog::from(root_catalog.clone());
        assert_eq!(branch_catalog.branches.len(), 1);
        let branch = &branch_catalog.branches[0];
        assert_eq!(branch.branch_kind, BranchKind::WorkspaceFs);
        assert_eq!(branch.attachment_status, BranchAttachmentStatus::Active);
        assert_eq!(branch.inspection_status, BranchInspectionStatus::Registered);
        assert_eq!(branch.migration_status, BranchMigrationStatus::Succeeded);
        assert_eq!(RootCatalog::from(branch_catalog), root_catalog);
    }

    #[test]
    fn resolved_root_converts_to_branch_handle() {
        let resolved_root = ResolvedRoot {
            root_id: "root-1".to_string(),
            workspace_path: PathBuf::from("/tmp/workspace"),
            data_home_path: PathBuf::from("/tmp/data"),
            manifest_path: PathBuf::from("/tmp/data/root_manifest.json"),
            ledger_path: PathBuf::from("/tmp/data/migration_ledger.jsonl"),
        };

        let branch_handle = BranchHandle::from(resolved_root.clone());
        assert_eq!(branch_handle.branch_id(), "root-1");
        assert_eq!(branch_handle.branch_kind(), &BranchKind::WorkspaceFs);
        assert_eq!(
            branch_handle.canonical_locator(),
            &PathBuf::from("/tmp/workspace")
        );
        assert_eq!(branch_handle.as_root(), resolved_root);
    }

    #[test]
    fn resolved_branch_converts_to_root() {
        let resolved_branch = ResolvedBranch {
            branch_id: "branch-1".to_string(),
            branch_kind: BranchKind::WorkspaceFs,
            canonical_locator: PathBuf::from("/tmp/workspace"),
            data_home_path: PathBuf::from("/tmp/data"),
            manifest_path: PathBuf::from("/tmp/data/root_manifest.json"),
            ledger_path: PathBuf::from("/tmp/data/migration_ledger.jsonl"),
        };

        let resolved_root: ResolvedRoot = resolved_branch.into();
        assert_eq!(resolved_root.root_id, "branch-1");
        assert_eq!(
            resolved_root.workspace_path,
            PathBuf::from("/tmp/workspace")
        );
    }
}
