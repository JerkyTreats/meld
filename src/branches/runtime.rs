use chrono::{SecondsFormat, Utc};
use std::path::{Path, PathBuf};

use crate::branches::contracts::{
    BranchAttachmentStatus, BranchCatalogEntry, BranchHandle, BranchInspectionStatus,
    BranchMigrationLane, BranchMigrationLedgerEntry, BranchMigrationStatus,
    BranchMigrationStepStatus, BranchStatusRow, BranchesStatusOutput, ResolvedBranch,
};
use crate::branches::{catalog, ledger, locator, manifest};
use crate::error::ApiError;
use crate::world_state::GraphRuntime;

#[derive(Debug, Clone, Default)]
pub struct BranchRuntime;

impl BranchRuntime {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_active_branch(&self, workspace_root: &Path) -> Result<BranchHandle, ApiError> {
        locator::resolve_active_branch(workspace_root).map(BranchHandle::from)
    }

    pub fn ensure_active_branch_registered(&self, branch: &BranchHandle) -> Result<(), ApiError> {
        self.ensure_branch_registered(
            branch.resolved(),
            BranchAttachmentStatus::Active,
            Some(locator::branch_store_path(
                &branch.resolved().data_home_path,
            )),
        )
    }

    pub fn attach_branch(&self, workspace_path: &Path) -> Result<BranchesStatusOutput, ApiError> {
        let branch = self.resolve_active_branch(workspace_path)?;
        self.ensure_branch_registered(
            branch.resolved(),
            BranchAttachmentStatus::Dormant,
            Some(locator::branch_store_path(
                &branch.resolved().data_home_path,
            )),
        )?;
        self.status()
    }

    pub fn discover_branches(&self) -> Result<BranchesStatusOutput, ApiError> {
        for data_home_path in locator::discover_branch_data_homes()? {
            let recovered_workspace_path = locator::recover_workspace_path(&data_home_path)?;
            let resolved =
                resolved_branch_from_data_home(&data_home_path, &recovered_workspace_path);
            let attachment_status = if recovered_workspace_path.exists() {
                BranchAttachmentStatus::Dormant
            } else {
                BranchAttachmentStatus::MissingWorkspacePath
            };
            self.ensure_branch_registered(
                &resolved,
                attachment_status,
                Some(locator::branch_store_path(&data_home_path)),
            )?;
        }
        self.status()
    }

    pub fn migrate_branches(&self) -> Result<BranchesStatusOutput, ApiError> {
        let catalog_path = locator::global_catalog_path()?;
        let branch_catalog = catalog::load(&catalog_path)?;
        for entry in branch_catalog.branches {
            let resolved = resolved_branch_from_catalog_entry(&entry);
            let store_path = entry
                .store_path
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| locator::branch_store_path(&resolved.data_home_path));
            let db = sled::open(&store_path).map_err(to_api_storage_error)?;
            let graph_runtime = GraphRuntime::new(db).map_err(ApiError::StorageError)?;
            match graph_runtime.catch_up() {
                Ok(applied_events) => {
                    let last_reduced_seq = graph_runtime
                        .traversal_store()
                        .last_reduced_seq()
                        .map_err(ApiError::StorageError)?;
                    self.record_graph_success(
                        &resolved,
                        last_reduced_seq,
                        applied_events,
                        Some(store_path.clone()),
                    )?;
                }
                Err(err) => {
                    self.record_graph_failure(
                        &resolved,
                        &err.to_string(),
                        Some(store_path.clone()),
                    )?;
                }
            }
        }
        self.status()
    }

    pub fn touch_active_branch(&self, branch: &BranchHandle) -> Result<(), ApiError> {
        self.touch_branch(
            branch.resolved(),
            BranchAttachmentStatus::Active,
            Some(locator::branch_store_path(
                &branch.resolved().data_home_path,
            )),
        )
    }

    pub fn record_branch_graph_catch_up_success(
        &self,
        branch: &BranchHandle,
        last_reduced_seq: u64,
        applied_events: usize,
    ) -> Result<(), ApiError> {
        self.record_graph_success(
            branch.resolved(),
            last_reduced_seq,
            applied_events,
            Some(locator::branch_store_path(
                &branch.resolved().data_home_path,
            )),
        )
    }

    pub fn record_branch_graph_catch_up_failure(
        &self,
        branch: &BranchHandle,
        error: &str,
    ) -> Result<(), ApiError> {
        self.record_graph_failure(
            branch.resolved(),
            error,
            Some(locator::branch_store_path(
                &branch.resolved().data_home_path,
            )),
        )
    }

    pub fn status(&self) -> Result<BranchesStatusOutput, ApiError> {
        let catalog_path = locator::global_catalog_path()?;
        let branch_catalog = catalog::load(&catalog_path)?;
        let branches = branch_catalog
            .branches
            .into_iter()
            .map(|branch| BranchStatusRow {
                branch_id: branch.branch_id,
                canonical_locator: branch.canonical_locator,
                data_home_path: branch.data_home_path,
                store_path: branch.store_path,
                attachment_status: branch.attachment_status.as_str().to_string(),
                inspection_status: branch.inspection_status.as_str().to_string(),
                migration_status: branch.migration_status.as_str().to_string(),
                last_seen_at: branch.last_seen_at,
                last_migration_at: branch.last_migration_at,
            })
            .collect();
        Ok(BranchesStatusOutput { branches })
    }

    fn ensure_branch_registered(
        &self,
        resolved: &ResolvedBranch,
        attachment_status: BranchAttachmentStatus,
        store_path: Option<PathBuf>,
    ) -> Result<(), ApiError> {
        let now = timestamp();
        let plan_id = plan_id(&resolved.branch_id, "register");

        append_started(
            resolved,
            &plan_id,
            "write_branch_manifest",
            BranchMigrationLane::Metadata,
            vec![format!(
                "workspace_path={}",
                resolved.canonical_locator.display()
            )],
        )?;

        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_branch_manifest(resolved, &now));
        current_manifest.canonical_locator =
            resolved.canonical_locator.to_string_lossy().to_string();
        current_manifest.last_seen_at = now.clone();
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "write_branch_manifest",
            BranchMigrationLane::Metadata,
            vec!["branch manifest persisted".to_string()],
        )?;

        append_started(
            resolved,
            &plan_id,
            "refresh_catalog_entry",
            BranchMigrationLane::Metadata,
            vec![format!(
                "data_home_path={}",
                resolved.data_home_path.display()
            )],
        )?;

        let catalog_path = locator::global_catalog_path()?;
        let mut branch_catalog = catalog::load(&catalog_path)?;
        let existing = branch_catalog
            .branches
            .iter()
            .find(|branch| branch.branch_id == current_manifest.branch_id)
            .cloned();
        let last_migration_at = existing
            .as_ref()
            .and_then(|branch| branch.last_migration_at.clone());
        catalog::upsert_branch(
            &mut branch_catalog,
            BranchCatalogEntry {
                branch_id: current_manifest.branch_id.clone(),
                branch_kind: current_manifest.branch_kind.clone(),
                canonical_locator: current_manifest.canonical_locator.clone(),
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                store_path: store_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .or_else(|| {
                        existing
                            .as_ref()
                            .and_then(|branch| branch.store_path.clone())
                    }),
                attachment_status,
                inspection_status: BranchInspectionStatus::Registered,
                migration_status: existing
                    .as_ref()
                    .map(|branch| branch.migration_status.clone())
                    .unwrap_or(BranchMigrationStatus::Unknown),
                last_seen_at: Some(now.clone()),
                last_inspected_at: Some(now.clone()),
                last_migration_at,
            },
        );
        catalog::save(&catalog_path, &branch_catalog)?;

        current_manifest.last_successful_plan_id = Some(plan_id.clone());
        current_manifest.last_successful_step_id = Some("refresh_catalog_entry".to_string());
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "refresh_catalog_entry",
            BranchMigrationLane::Metadata,
            vec!["branch catalog updated".to_string()],
        )?;

        Ok(())
    }

    fn touch_branch(
        &self,
        resolved: &ResolvedBranch,
        attachment_status: BranchAttachmentStatus,
        store_path: Option<PathBuf>,
    ) -> Result<(), ApiError> {
        let now = timestamp();
        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_branch_manifest(resolved, &now));
        current_manifest.last_seen_at = now.clone();
        current_manifest.canonical_locator =
            resolved.canonical_locator.to_string_lossy().to_string();
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        let catalog_path = locator::global_catalog_path()?;
        let mut branch_catalog = catalog::load(&catalog_path)?;
        let existing = branch_catalog
            .branches
            .iter()
            .find(|branch| branch.branch_id == current_manifest.branch_id)
            .cloned();
        catalog::upsert_branch(
            &mut branch_catalog,
            BranchCatalogEntry {
                branch_id: current_manifest.branch_id.clone(),
                branch_kind: current_manifest.branch_kind.clone(),
                canonical_locator: current_manifest.canonical_locator.clone(),
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                store_path: store_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .or_else(|| {
                        existing
                            .as_ref()
                            .and_then(|branch| branch.store_path.clone())
                    }),
                attachment_status,
                inspection_status: BranchInspectionStatus::Registered,
                migration_status: existing
                    .as_ref()
                    .map(|branch| branch.migration_status.clone())
                    .unwrap_or(BranchMigrationStatus::Unknown),
                last_seen_at: Some(now.clone()),
                last_inspected_at: existing
                    .as_ref()
                    .and_then(|branch| branch.last_inspected_at.clone()),
                last_migration_at: existing.and_then(|branch| branch.last_migration_at),
            },
        );
        catalog::save(&catalog_path, &branch_catalog)?;
        Ok(())
    }

    fn record_graph_success(
        &self,
        resolved: &ResolvedBranch,
        last_reduced_seq: u64,
        applied_events: usize,
        store_path: Option<PathBuf>,
    ) -> Result<(), ApiError> {
        let now = timestamp();
        let plan_id = plan_id(&resolved.branch_id, "graph");

        append_started(
            resolved,
            &plan_id,
            "replay_spine_into_traversal",
            BranchMigrationLane::Derived,
            vec![
                format!("applied_events={}", applied_events),
                format!("last_reduced_seq={}", last_reduced_seq),
            ],
        )?;

        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_branch_manifest(resolved, &now));
        current_manifest.last_seen_at = now.clone();
        current_manifest.last_reduced_seq = last_reduced_seq;
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "replay_spine_into_traversal",
            BranchMigrationLane::Derived,
            vec!["traversal replay completed".to_string()],
        )?;

        append_started(
            resolved,
            &plan_id,
            "mark_derived_version",
            BranchMigrationLane::Derived,
            vec![format!(
                "derived_state_version={}",
                current_manifest.derived_state_version
            )],
        )?;

        let catalog_path = locator::global_catalog_path()?;
        let mut branch_catalog = catalog::load(&catalog_path)?;
        let existing = branch_catalog
            .branches
            .iter()
            .find(|branch| branch.branch_id == current_manifest.branch_id)
            .cloned();
        catalog::upsert_branch(
            &mut branch_catalog,
            BranchCatalogEntry {
                branch_id: current_manifest.branch_id.clone(),
                branch_kind: current_manifest.branch_kind.clone(),
                canonical_locator: current_manifest.canonical_locator.clone(),
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                store_path: store_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .or_else(|| {
                        existing
                            .as_ref()
                            .and_then(|branch| branch.store_path.clone())
                    }),
                attachment_status: existing
                    .as_ref()
                    .map(|branch| branch.attachment_status.clone())
                    .unwrap_or(BranchAttachmentStatus::Active),
                inspection_status: BranchInspectionStatus::Registered,
                migration_status: if applied_events == 0 {
                    BranchMigrationStatus::NotNeeded
                } else {
                    BranchMigrationStatus::Succeeded
                },
                last_seen_at: Some(now.clone()),
                last_inspected_at: Some(now.clone()),
                last_migration_at: Some(now.clone()),
            },
        );
        catalog::save(&catalog_path, &branch_catalog)?;

        current_manifest.last_successful_plan_id = Some(plan_id.clone());
        current_manifest.last_successful_step_id = Some("mark_derived_version".to_string());
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "mark_derived_version",
            BranchMigrationLane::Derived,
            vec!["derived migration status persisted".to_string()],
        )?;

        Ok(())
    }

    fn record_graph_failure(
        &self,
        resolved: &ResolvedBranch,
        error: &str,
        store_path: Option<PathBuf>,
    ) -> Result<(), ApiError> {
        let now = timestamp();
        let plan_id = plan_id(&resolved.branch_id, "graph_failure");
        append_failed(
            resolved,
            &plan_id,
            "replay_spine_into_traversal",
            BranchMigrationLane::Derived,
            error.to_string(),
        )?;

        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_branch_manifest(resolved, &now));
        current_manifest.last_seen_at = now.clone();
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        let catalog_path = locator::global_catalog_path()?;
        let mut branch_catalog = catalog::load(&catalog_path)?;
        let existing = branch_catalog
            .branches
            .iter()
            .find(|branch| branch.branch_id == current_manifest.branch_id)
            .cloned();
        catalog::upsert_branch(
            &mut branch_catalog,
            BranchCatalogEntry {
                branch_id: current_manifest.branch_id,
                branch_kind: current_manifest.branch_kind,
                canonical_locator: current_manifest.canonical_locator,
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                store_path: store_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .or_else(|| {
                        existing
                            .as_ref()
                            .and_then(|branch| branch.store_path.clone())
                    }),
                attachment_status: existing
                    .as_ref()
                    .map(|branch| branch.attachment_status.clone())
                    .unwrap_or(BranchAttachmentStatus::Active),
                inspection_status: BranchInspectionStatus::Registered,
                migration_status: BranchMigrationStatus::Failed,
                last_seen_at: Some(now.clone()),
                last_inspected_at: Some(now.clone()),
                last_migration_at: Some(now),
            },
        );
        catalog::save(&catalog_path, &branch_catalog)?;
        Ok(())
    }
}

fn append_started(
    resolved: &ResolvedBranch,
    plan_id: &str,
    step_id: &str,
    lane: BranchMigrationLane,
    observed_inputs: Vec<String>,
) -> Result<(), ApiError> {
    ledger::append(
        &resolved.ledger_path,
        &BranchMigrationLedgerEntry {
            plan_id: plan_id.to_string(),
            step_id: step_id.to_string(),
            lane,
            status: BranchMigrationStepStatus::Started,
            started_at: timestamp(),
            finished_at: None,
            error_summary: None,
            observed_inputs,
            verification_summary: Vec::new(),
        },
    )
}

fn append_verified(
    resolved: &ResolvedBranch,
    plan_id: &str,
    step_id: &str,
    lane: BranchMigrationLane,
    verification_summary: Vec<String>,
) -> Result<(), ApiError> {
    let now = timestamp();
    ledger::append(
        &resolved.ledger_path,
        &BranchMigrationLedgerEntry {
            plan_id: plan_id.to_string(),
            step_id: step_id.to_string(),
            lane,
            status: BranchMigrationStepStatus::Verified,
            started_at: now.clone(),
            finished_at: Some(now),
            error_summary: None,
            observed_inputs: Vec::new(),
            verification_summary,
        },
    )
}

fn append_failed(
    resolved: &ResolvedBranch,
    plan_id: &str,
    step_id: &str,
    lane: BranchMigrationLane,
    error_summary: String,
) -> Result<(), ApiError> {
    let now = timestamp();
    ledger::append(
        &resolved.ledger_path,
        &BranchMigrationLedgerEntry {
            plan_id: plan_id.to_string(),
            step_id: step_id.to_string(),
            lane,
            status: BranchMigrationStepStatus::Failed,
            started_at: now.clone(),
            finished_at: Some(now),
            error_summary: Some(error_summary),
            observed_inputs: Vec::new(),
            verification_summary: Vec::new(),
        },
    )
}

fn resolved_branch_from_data_home(data_home_path: &Path, workspace_path: &Path) -> ResolvedBranch {
    let normalized_path =
        crate::tree::path::normalize_path_string(&workspace_path.to_string_lossy());
    let branch_id = blake3::hash(normalized_path.as_bytes())
        .to_hex()
        .to_string();
    ResolvedBranch {
        branch_id,
        branch_kind: crate::branches::BranchKind::WorkspaceFs,
        canonical_locator: workspace_path.to_path_buf(),
        data_home_path: data_home_path.to_path_buf(),
        manifest_path: data_home_path.join("branch_manifest.json"),
        ledger_path: data_home_path.join("branch_migration_ledger.jsonl"),
    }
}

fn resolved_branch_from_catalog_entry(entry: &BranchCatalogEntry) -> ResolvedBranch {
    let data_home_path = PathBuf::from(&entry.data_home_path);
    ResolvedBranch {
        branch_id: entry.branch_id.clone(),
        branch_kind: entry.branch_kind.clone(),
        canonical_locator: PathBuf::from(&entry.canonical_locator),
        data_home_path: data_home_path.clone(),
        manifest_path: data_home_path.join("branch_manifest.json"),
        ledger_path: data_home_path.join("branch_migration_ledger.jsonl"),
    }
}

fn to_api_storage_error(error: sled::Error) -> ApiError {
    ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
        format!("Failed to open sled database: {}", error),
    )))
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn plan_id(branch_id: &str, phase: &str) -> String {
    format!("{}::{}::{}", phase, branch_id, timestamp())
}
