use chrono::{SecondsFormat, Utc};
use std::path::Path;

use crate::error::ApiError;
use crate::roots::contracts::{
    ResolvedRoot, RootAttachmentStatus, RootCatalogEntry, RootInspectionStatus, RootMigrationLane,
    RootMigrationLedgerEntry, RootMigrationStatus, RootMigrationStepStatus, RootStatusRow,
    RootsStatusOutput,
};
use crate::roots::{catalog, ledger, locator, manifest};

#[derive(Debug, Clone, Default)]
pub struct RootRuntime;

impl RootRuntime {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_active_root(&self, workspace_root: &Path) -> Result<ResolvedRoot, ApiError> {
        locator::resolve_active_root(workspace_root)
    }

    pub fn ensure_active_root_registered(&self, resolved: &ResolvedRoot) -> Result<(), ApiError> {
        let now = timestamp();
        let plan_id = plan_id(&resolved.root_id, "register");

        append_started(
            resolved,
            &plan_id,
            "write_root_manifest",
            RootMigrationLane::Metadata,
            vec![format!(
                "workspace_path={}",
                resolved.workspace_path.display()
            )],
        )?;

        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_manifest(resolved, &now));
        current_manifest.workspace_path = resolved.workspace_path.to_string_lossy().to_string();
        current_manifest.last_seen_at = now.clone();
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "write_root_manifest",
            RootMigrationLane::Metadata,
            vec!["root manifest persisted".to_string()],
        )?;

        append_started(
            resolved,
            &plan_id,
            "refresh_catalog_entry",
            RootMigrationLane::Metadata,
            vec![format!(
                "data_home_path={}",
                resolved.data_home_path.display()
            )],
        )?;

        let catalog_path = locator::global_catalog_path()?;
        let mut root_catalog = catalog::load(&catalog_path)?;
        let existing = root_catalog
            .roots
            .iter()
            .find(|root| root.root_id == current_manifest.root_id)
            .cloned();
        let last_migration_at = existing.and_then(|root| root.last_migration_at);
        catalog::upsert(
            &mut root_catalog,
            RootCatalogEntry {
                root_id: current_manifest.root_id.clone(),
                workspace_path: current_manifest.workspace_path.clone(),
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                attachment_status: RootAttachmentStatus::Active,
                inspection_status: RootInspectionStatus::Registered,
                migration_status: RootMigrationStatus::Unknown,
                last_seen_at: Some(now.clone()),
                last_inspected_at: Some(now.clone()),
                last_migration_at,
            },
        );
        catalog::save(&catalog_path, &root_catalog)?;

        current_manifest.last_successful_plan_id = Some(plan_id.clone());
        current_manifest.last_successful_step_id = Some("refresh_catalog_entry".to_string());
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "refresh_catalog_entry",
            RootMigrationLane::Metadata,
            vec!["root catalog updated".to_string()],
        )?;

        Ok(())
    }

    pub fn touch_active_root(&self, resolved: &ResolvedRoot) -> Result<(), ApiError> {
        let now = timestamp();
        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_manifest(resolved, &now));
        current_manifest.last_seen_at = now.clone();
        current_manifest.workspace_path = resolved.workspace_path.to_string_lossy().to_string();
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        let catalog_path = locator::global_catalog_path()?;
        let mut root_catalog = catalog::load(&catalog_path)?;
        let existing = root_catalog
            .roots
            .iter()
            .find(|root| root.root_id == current_manifest.root_id)
            .cloned();
        catalog::upsert(
            &mut root_catalog,
            RootCatalogEntry {
                root_id: current_manifest.root_id.clone(),
                workspace_path: current_manifest.workspace_path.clone(),
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                attachment_status: RootAttachmentStatus::Active,
                inspection_status: RootInspectionStatus::Registered,
                migration_status: existing
                    .as_ref()
                    .map(|root| root.migration_status.clone())
                    .unwrap_or(RootMigrationStatus::Unknown),
                last_seen_at: Some(now.clone()),
                last_inspected_at: existing
                    .as_ref()
                    .and_then(|root| root.last_inspected_at.clone()),
                last_migration_at: existing.and_then(|root| root.last_migration_at),
            },
        );
        catalog::save(&catalog_path, &root_catalog)?;
        Ok(())
    }

    pub fn record_graph_catch_up_success(
        &self,
        resolved: &ResolvedRoot,
        last_reduced_seq: u64,
        applied_events: usize,
    ) -> Result<(), ApiError> {
        let now = timestamp();
        let plan_id = plan_id(&resolved.root_id, "graph");

        append_started(
            resolved,
            &plan_id,
            "replay_spine_into_traversal",
            RootMigrationLane::Derived,
            vec![
                format!("applied_events={}", applied_events),
                format!("last_reduced_seq={}", last_reduced_seq),
            ],
        )?;

        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_manifest(resolved, &now));
        current_manifest.last_seen_at = now.clone();
        current_manifest.last_reduced_seq = last_reduced_seq;
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "replay_spine_into_traversal",
            RootMigrationLane::Derived,
            vec!["traversal replay completed".to_string()],
        )?;

        append_started(
            resolved,
            &plan_id,
            "mark_derived_version",
            RootMigrationLane::Derived,
            vec![format!(
                "derived_state_version={}",
                current_manifest.derived_state_version
            )],
        )?;

        let catalog_path = locator::global_catalog_path()?;
        let mut root_catalog = catalog::load(&catalog_path)?;
        catalog::upsert(
            &mut root_catalog,
            RootCatalogEntry {
                root_id: current_manifest.root_id.clone(),
                workspace_path: current_manifest.workspace_path.clone(),
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                attachment_status: RootAttachmentStatus::Active,
                inspection_status: RootInspectionStatus::Registered,
                migration_status: if applied_events == 0 {
                    RootMigrationStatus::NotNeeded
                } else {
                    RootMigrationStatus::Succeeded
                },
                last_seen_at: Some(now.clone()),
                last_inspected_at: Some(now.clone()),
                last_migration_at: Some(now.clone()),
            },
        );
        catalog::save(&catalog_path, &root_catalog)?;

        current_manifest.last_successful_plan_id = Some(plan_id.clone());
        current_manifest.last_successful_step_id = Some("mark_derived_version".to_string());
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        append_verified(
            resolved,
            &plan_id,
            "mark_derived_version",
            RootMigrationLane::Derived,
            vec!["derived migration status persisted".to_string()],
        )?;

        Ok(())
    }

    pub fn record_graph_catch_up_failure(
        &self,
        resolved: &ResolvedRoot,
        error: &str,
    ) -> Result<(), ApiError> {
        let now = timestamp();
        let plan_id = plan_id(&resolved.root_id, "graph_failure");
        append_failed(
            resolved,
            &plan_id,
            "replay_spine_into_traversal",
            RootMigrationLane::Derived,
            error.to_string(),
        )?;

        let mut current_manifest = manifest::load(&resolved.manifest_path)?
            .unwrap_or_else(|| manifest::new_manifest(resolved, &now));
        current_manifest.last_seen_at = now.clone();
        manifest::save(&resolved.manifest_path, &current_manifest)?;

        let catalog_path = locator::global_catalog_path()?;
        let mut root_catalog = catalog::load(&catalog_path)?;
        catalog::upsert(
            &mut root_catalog,
            RootCatalogEntry {
                root_id: current_manifest.root_id,
                workspace_path: current_manifest.workspace_path,
                data_home_path: resolved.data_home_path.to_string_lossy().to_string(),
                attachment_status: RootAttachmentStatus::Active,
                inspection_status: RootInspectionStatus::Registered,
                migration_status: RootMigrationStatus::Failed,
                last_seen_at: Some(now.clone()),
                last_inspected_at: Some(now.clone()),
                last_migration_at: Some(now),
            },
        );
        catalog::save(&catalog_path, &root_catalog)?;
        Ok(())
    }

    pub fn status(&self) -> Result<RootsStatusOutput, ApiError> {
        let catalog_path = locator::global_catalog_path()?;
        let root_catalog = catalog::load(&catalog_path)?;
        let roots = root_catalog
            .roots
            .into_iter()
            .map(|root| RootStatusRow {
                root_id: root.root_id,
                workspace_path: root.workspace_path,
                data_home_path: root.data_home_path,
                attachment_status: root.attachment_status.as_str().to_string(),
                inspection_status: root.inspection_status.as_str().to_string(),
                migration_status: root.migration_status.as_str().to_string(),
                last_seen_at: root.last_seen_at,
                last_migration_at: root.last_migration_at,
            })
            .collect();
        Ok(RootsStatusOutput { roots })
    }
}

fn append_started(
    resolved: &ResolvedRoot,
    plan_id: &str,
    step_id: &str,
    lane: RootMigrationLane,
    observed_inputs: Vec<String>,
) -> Result<(), ApiError> {
    ledger::append(
        &resolved.ledger_path,
        &RootMigrationLedgerEntry {
            plan_id: plan_id.to_string(),
            step_id: step_id.to_string(),
            lane,
            status: RootMigrationStepStatus::Started,
            started_at: timestamp(),
            finished_at: None,
            error_summary: None,
            observed_inputs,
            verification_summary: Vec::new(),
        },
    )
}

fn append_verified(
    resolved: &ResolvedRoot,
    plan_id: &str,
    step_id: &str,
    lane: RootMigrationLane,
    verification_summary: Vec<String>,
) -> Result<(), ApiError> {
    let now = timestamp();
    ledger::append(
        &resolved.ledger_path,
        &RootMigrationLedgerEntry {
            plan_id: plan_id.to_string(),
            step_id: step_id.to_string(),
            lane,
            status: RootMigrationStepStatus::Verified,
            started_at: now.clone(),
            finished_at: Some(now),
            error_summary: None,
            observed_inputs: Vec::new(),
            verification_summary,
        },
    )
}

fn append_failed(
    resolved: &ResolvedRoot,
    plan_id: &str,
    step_id: &str,
    lane: RootMigrationLane,
    error_summary: String,
) -> Result<(), ApiError> {
    let now = timestamp();
    ledger::append(
        &resolved.ledger_path,
        &RootMigrationLedgerEntry {
            plan_id: plan_id.to_string(),
            step_id: step_id.to_string(),
            lane,
            status: RootMigrationStepStatus::Failed,
            started_at: now.clone(),
            finished_at: Some(now),
            error_summary: Some(error_summary),
            observed_inputs: Vec::new(),
            verification_summary: Vec::new(),
        },
    )
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn plan_id(root_id: &str, phase: &str) -> String {
    format!("{}::{}::{}", phase, root_id, timestamp())
}
