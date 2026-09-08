//! Workspace command service: single entry point per workspace CLI variant.
//!
//! Owns workspace workflow logic; CLI parses, calls one method per variant, and formats output.

use crate::agent::AgentRegistry;
use crate::api::ContextApi;
use crate::context::head::CurrentFrameHeadRead;
use crate::error::ApiError;
use crate::execution::ContextReadPort;
use crate::ignore;
use crate::store::{NodeRecord, NodeRecordStore};
use crate::telemetry::ProgressRuntime;
use crate::tree::builder::TreeBuilder;
use crate::types::NodeID;
use crate::workspace::scan::{
    build_owner_publication_candidate, build_publication_candidates,
    execute_workspace_scan_observed, stored_workspace_root_hash, workspace_walker_config,
    WorkspaceScanPolicy, WorkspaceScanRequest, WorkspaceScanStatus,
};
use crate::workspace::section;
use crate::workspace::types::{
    AgentStatusEntry, AgentStatusOutput, IgnoreResult, ListDeletedResult, ListDeletedRow,
    ProviderStatusEntry, ProviderStatusOutput, UnifiedStatusOutput, ValidateResult,
    WorkspaceScanInfo, WorkspaceScanState, WorkspaceStatusRequest, WorkspaceStatusResult,
};
use meld_world_model::world_state::graph::contracts::OWNER_PUBLICATION_EVENT_TYPE;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub(crate) fn current_workspace_root_hash(workspace_root: &Path) -> Result<NodeID, ApiError> {
    TreeBuilder::new(workspace_root.to_path_buf())
        .with_walker_config(workspace_walker_config(workspace_root))
        .compute_root()
        .map_err(ApiError::from)
}

fn workspace_lookup_path(workspace_root: &Path, path: &Path) -> PathBuf {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    PathBuf::from(crate::tree::path::normalize_path_string(
        &resolved.to_string_lossy(),
    ))
}

pub(crate) fn assess_workspace_scan_state(
    node_store: &dyn NodeRecordStore,
    workspace_root: &Path,
) -> Result<WorkspaceScanInfo, ApiError> {
    let current_root_hash_id = current_workspace_root_hash(workspace_root)?;
    let current_root_hash = hex::encode(current_root_hash_id);
    let active_node_count = node_store.list_active().map_err(ApiError::from)?.len();
    let stored_root_hash =
        stored_workspace_root_hash(node_store, workspace_root, &current_root_hash_id)?;
    let scan_state = if active_node_count == 0 {
        WorkspaceScanState::Missing
    } else if stored_root_hash.as_deref() == Some(current_root_hash.as_str()) {
        WorkspaceScanState::Current
    } else {
        WorkspaceScanState::Stale
    };

    Ok(WorkspaceScanInfo {
        scan_state,
        current_root_hash,
        stored_root_hash,
        active_node_count,
    })
}

pub fn read_workspace_scan_state(
    api: &ContextApi,
    workspace_root: &Path,
) -> Result<WorkspaceScanInfo, ApiError> {
    assess_workspace_scan_state(api.node_store().as_ref(), workspace_root)
}

/// Resolve path or --node to NodeID. If include_tombstoned is true, use get_by_path (for restore).
pub fn resolve_workspace_node_id(
    api: &(impl ContextReadPort + ?Sized),
    workspace_root: &Path,
    path: Option<&Path>,
    node: Option<&str>,
    include_tombstoned: bool,
) -> Result<NodeID, ApiError> {
    match (path, node) {
        (Some(p), None) => {
            let resolved_path = workspace_lookup_path(workspace_root, p);
            let record = if include_tombstoned {
                api.read_node_record_by_path(&resolved_path, true)?
            } else {
                api.read_node_record_by_path(&resolved_path, false)?
            };
            if let Some(record) = record {
                return Ok(record.node_id);
            }
            if let Ok(canonical_path) = crate::tree::path::canonicalize_path(&resolved_path) {
                let record = if include_tombstoned {
                    api.read_node_record_by_path(&canonical_path, true)?
                } else {
                    api.read_node_record_by_path(&canonical_path, false)?
                };
                if let Some(record) = record {
                    return Ok(record.node_id);
                }
                if let Some(node_id) = resolve_node_id_by_canonical_fallback(
                    api,
                    workspace_root,
                    &canonical_path,
                    include_tombstoned,
                )? {
                    return Ok(node_id);
                }
            }
            Err(ApiError::PathNotInTree(resolved_path))
        }
        (None, Some(hex_str)) => {
            let bytes = hex::decode(hex_str.trim_start_matches("0x"))
                .map_err(|_| ApiError::ConfigError(format!("Invalid node ID hex: {}", hex_str)))?;
            if bytes.len() != 32 {
                return Err(ApiError::ConfigError(
                    "Node ID must be 32 bytes (64 hex chars).".to_string(),
                ));
            }
            let mut node_id = [0u8; 32];
            node_id.copy_from_slice(&bytes);
            if api.read_node_record(&node_id)?.is_none() {
                return Err(ApiError::NodeNotFound(node_id));
            }
            Ok(node_id)
        }
        (Some(_), Some(_)) => Err(ApiError::ConfigError(
            "Cannot specify both path and --node. Use one or the other.".to_string(),
        )),
        (None, None) => Err(ApiError::ConfigError(
            "Must specify either path or --node <node_id>.".to_string(),
        )),
    }
}

/// Fallback: match by canonical path when direct path lookup misses.
pub fn resolve_node_id_by_canonical_fallback(
    store: &(impl ContextReadPort + ?Sized),
    workspace_root: &Path,
    canonical_target: &Path,
    include_tombstoned: bool,
) -> Result<Option<NodeID>, ApiError> {
    let records = if include_tombstoned {
        store.list_node_records(true)?
    } else {
        store.list_node_records(false)?
    };

    for record in records {
        let candidate = if record.path.is_absolute() {
            record.path.clone()
        } else {
            workspace_root.join(&record.path)
        };
        let canonical_candidate = match crate::tree::path::canonicalize_path(&candidate) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if canonical_candidate == canonical_target {
            return Ok(Some(record.node_id));
        }
    }
    Ok(None)
}

fn count_frame_files(path: &PathBuf) -> Result<usize, ApiError> {
    let mut count = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?
        {
            let entry = entry
                .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;
            let path = entry.path();
            if path.is_dir() {
                count += count_frame_files(&path)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("frame") {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Stateless workspace command service.
pub struct WorkspaceCommandService;

impl WorkspaceCommandService {
    /// Workspace section only: tree, context coverage, top paths.
    /// Aligns with AgentCommandService::status and ProviderCommandService::run_status pattern.
    pub fn status(
        api: &ContextApi,
        request: &WorkspaceStatusRequest,
        agent_registry: &AgentRegistry,
    ) -> Result<WorkspaceStatusResult, ApiError> {
        let node_store = api.node_store().as_ref() as &dyn NodeRecordStore;
        section::build_workspace_status(
            node_store,
            api,
            agent_registry,
            &request.workspace_root,
            &request.store_path,
            request.include_breakdown,
        )
    }

    /// Validate store, head index, and root consistency.
    pub fn validate(
        api: &ContextApi,
        workspace_root: &Path,
        frame_storage_path: &PathBuf,
    ) -> Result<ValidateResult, ApiError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let root_hash = match current_workspace_root_hash(workspace_root) {
            Ok(hash) => hash,
            Err(e) => {
                errors.push(format!("Failed to compute workspace root: {}", e));
                return Ok(ValidateResult {
                    valid: false,
                    root_hash: String::new(),
                    node_count: 0,
                    frame_count: 0,
                    errors,
                    warnings,
                });
            }
        };

        let node_count = match api.node_store().get(&root_hash).map_err(ApiError::from)? {
            Some(record) => {
                if record.node_id != root_hash {
                    errors.push(format!(
                        "Root node record has mismatched node_id: {} vs {}",
                        hex::encode(record.node_id),
                        hex::encode(root_hash)
                    ));
                }
                api.node_store().list_all().map_err(ApiError::from)?.len()
            }
            None => {
                warnings.push(
                    "Root node not found in store - workspace may not be scanned".to_string(),
                );
                0
            }
        };

        for record in api.node_store().list_active().map_err(ApiError::from)? {
            let node_id = record.node_id;
            let frame_ids = api.current_frame_heads_for_node(&node_id)?;
            for frame_id in frame_ids {
                if api
                    .frame_storage()
                    .get(&frame_id)
                    .map_err(ApiError::from)?
                    .is_none()
                {
                    warnings.push(format!(
                        "Head frame {} for node {} not found in storage",
                        hex::encode(frame_id),
                        hex::encode(node_id)
                    ));
                }
            }
        }

        let frame_count = if frame_storage_path.exists() {
            count_frame_files(frame_storage_path)?
        } else {
            0
        };

        let root_hex = hex::encode(root_hash);
        let valid = errors.is_empty();

        Ok(ValidateResult {
            valid,
            root_hash: root_hex,
            node_count,
            frame_count,
            errors,
            warnings,
        })
    }

    /// List ignore list or add a path.
    pub fn ignore(
        workspace_root: &Path,
        path: Option<&Path>,
        dry_run: bool,
    ) -> Result<IgnoreResult, ApiError> {
        match path {
            None => {
                let entries = ignore::read_ignore_list(workspace_root)?;
                Ok(IgnoreResult::List { entries })
            }
            Some(p) => {
                let normalized = ignore::normalize_workspace_relative(workspace_root, p)?;
                if dry_run {
                    return Ok(IgnoreResult::Added {
                        path: format!("Would add {} to ignore list.", normalized),
                    });
                }
                ignore::append_to_ignore_list(workspace_root, &normalized)?;
                Ok(IgnoreResult::Added {
                    path: format!("Added {} to ignore list.", normalized),
                })
            }
        }
    }

    /// Tombstone node/subtree; optionally add path to ignore list.
    pub fn delete(
        api: &ContextApi,
        workspace_root: &Path,
        path: Option<&Path>,
        node: Option<&str>,
        dry_run: bool,
        no_ignore: bool,
    ) -> Result<String, ApiError> {
        let node_id = resolve_workspace_node_id(api, workspace_root, path, node, false)?;
        let store = api.node_store();
        let record = store
            .get(&node_id)
            .map_err(ApiError::from)?
            .ok_or(ApiError::NodeNotFound(node_id))?;
        if record.tombstoned_at.is_some() {
            return Ok("Already deleted.".to_string());
        }
        if dry_run {
            let set = api.collect_subtree_node_ids(node_id)?;
            let n = set.len() as u64;
            let mut total_heads = 0u64;
            for nid in &set {
                total_heads += api.current_frame_heads_for_node(nid)?.len() as u64;
            }
            return Ok(format!(
                "Would delete {} nodes, {} head entries.",
                n, total_heads
            ));
        }
        let result = api.tombstone_node(node_id)?;
        let path_for_ignore = if !no_ignore {
            let norm = ignore::normalize_workspace_relative(workspace_root, &record.path)?;
            ignore::append_to_ignore_list(workspace_root, &norm)?;
            Some(norm)
        } else {
            None
        };
        let mut msg = format!(
            "Deleted {} nodes, {} head entries.",
            result.nodes_tombstoned, result.head_entries_tombstoned
        );
        if let Some(p) = path_for_ignore {
            msg.push_str(&format!(" Added {} to ignore list.", p));
        }
        Ok(msg)
    }

    /// Restore tombstoned node/subtree and remove from ignore list.
    pub fn restore(
        api: &ContextApi,
        workspace_root: &Path,
        path: Option<&Path>,
        node: Option<&str>,
        dry_run: bool,
    ) -> Result<String, ApiError> {
        let node_id = resolve_workspace_node_id(api, workspace_root, path, node, true)?;
        let store = api.node_store();
        let record = store
            .get(&node_id)
            .map_err(ApiError::from)?
            .ok_or(ApiError::NodeNotFound(node_id))?;
        if record.tombstoned_at.is_none() {
            return Ok("Not deleted.".to_string());
        }
        if dry_run {
            let set = api.collect_subtree_node_ids(node_id)?;
            let n = set.len() as u64;
            let mut total_heads = 0u64;
            for nid in &set {
                total_heads += api.current_frame_heads_for_node(nid)?.len() as u64;
            }
            return Ok(format!(
                "Would restore {} nodes, {} head entries.",
                n, total_heads
            ));
        }
        let result = api.restore_node(node_id)?;
        let norm = ignore::normalize_workspace_relative(workspace_root, &record.path)?;
        let _ = ignore::remove_from_ignore_list(workspace_root, &record.path);
        Ok(format!(
            "Restored {} nodes, {} head entries. Removed {} from ignore list.",
            result.nodes_restored, result.head_entries_restored, norm
        ))
    }

    /// Purge old tombstones; optionally purge frame blobs.
    pub fn compact(
        api: &ContextApi,
        ttl: Option<u64>,
        all: bool,
        keep_frames: bool,
        dry_run: bool,
    ) -> Result<String, ApiError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ttl_seconds = if all {
            0
        } else {
            let ttl_days = ttl.unwrap_or(90);
            ttl_days * 24 * 60 * 60
        };
        let cutoff = now.saturating_sub(ttl_seconds);
        let node_ids = api
            .node_store()
            .list_tombstoned(Some(cutoff))
            .map_err(ApiError::from)?;
        if dry_run {
            let mut frames = 0u64;
            let mut artifacts = 0u64;
            if !keep_frames {
                for nid in &node_ids {
                    frames += api.heads().get_all_heads_for_node(nid).len() as u64;
                }
                artifacts = api.prompt_context_storage().count_older_than(cutoff)?;
            }
            let head_count: usize = api
                .heads()
                .heads
                .iter()
                .filter(|(_, e)| e.tombstoned_at.is_some_and(|ts| ts <= cutoff))
                .count();
            return Ok(format!(
                "Would compact {} nodes, {} head entries, {} frames, {} artifacts.",
                node_ids.len(),
                head_count,
                frames,
                artifacts
            ));
        }
        let result = api.compact(ttl_seconds, !keep_frames)?;
        Ok(format!(
            "Compacted {} nodes, {} head entries, {} frames, {} artifacts.",
            result.nodes_purged,
            result.head_entries_purged,
            result.frames_purged,
            result.artifacts_purged
        ))
    }

    /// List tombstoned nodes with optional age filter.
    pub fn list_deleted(
        api: &ContextApi,
        older_than: Option<u64>,
    ) -> Result<ListDeletedResult, ApiError> {
        let cutoff = older_than.map(|days| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now.saturating_sub(days * 24 * 60 * 60)
        });
        let node_ids = api
            .node_store()
            .list_tombstoned(cutoff)
            .map_err(ApiError::from)?;
        let store = api.node_store();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut rows = Vec::new();
        for nid in &node_ids {
            if let Some(record) = store.get(nid).map_err(ApiError::from)? {
                let ts = record.tombstoned_at.unwrap_or(0);
                let age_secs = now.saturating_sub(ts);
                let age_str = if age_secs < 60 {
                    format!("{}s", age_secs)
                } else if age_secs < 3600 {
                    format!("{}m", age_secs / 60)
                } else if age_secs < 86400 {
                    format!("{}h", age_secs / 3600)
                } else {
                    format!("{}d", age_secs / 86400)
                };
                let node_hex = hex::encode(nid);
                let short_id = if node_hex.len() > 12 {
                    format!("{}...", &node_hex[..12])
                } else {
                    node_hex
                };
                rows.push(ListDeletedRow {
                    path: record.path.to_string_lossy().to_string(),
                    node_id_short: short_id,
                    tombstoned_at: ts,
                    age: age_str,
                });
            }
        }
        Ok(ListDeletedResult { rows })
    }

    /// Scan filesystem and rebuild tree through the workspace scan core.
    /// Returns a summary string. Progress/session_id optional for telemetry events.
    pub fn scan(
        api: &ContextApi,
        workspace_root: &Path,
        force: bool,
        progress: Option<&Arc<ProgressRuntime>>,
        session_id: Option<&str>,
    ) -> Result<String, ApiError> {
        let scan_started = Instant::now();
        let telemetry = match (progress, session_id) {
            (Some(progress), Some(session_id)) => Some((progress, session_id)),
            _ => None,
        };
        let request = WorkspaceScanRequest {
            workspace_root: workspace_root.to_path_buf(),
            policy: WorkspaceScanPolicy { force },
            session_id: telemetry.map(|(_, session_id)| session_id.to_string()),
            // The CLI reports summary facts and forwards publication
            // candidates; it never reads per-node observed refs.
            collect_observed: false,
        };
        let mut emit_scan_progress = |processed: usize, total: usize| {
            if let Some((progress, session_id)) = telemetry {
                progress.emit_event_best_effort(
                    session_id,
                    "scan_progress",
                    json!({
                        "node_count": processed,
                        "total_nodes": total
                    }),
                );
            }
        };
        let outcome =
            execute_workspace_scan_observed(api, &request, Some(&mut emit_scan_progress))?;

        if let Some((progress, _)) = telemetry {
            for envelope in outcome.publication_candidates.iter().cloned() {
                if envelope.event_type == OWNER_PUBLICATION_EVENT_TYPE {
                    progress.emit_envelope_idempotent(envelope)?;
                } else {
                    progress.emit_envelope_best_effort(envelope);
                }
            }
        }

        match outcome.summary.status {
            WorkspaceScanStatus::UpToDate => Ok(format!(
                "Tree already exists (root: {}). Use --force to rebuild.",
                outcome.summary.root_node_id
            )),
            WorkspaceScanStatus::Scanned => {
                if let Some((progress, session_id)) = telemetry {
                    progress.emit_event_best_effort(
                        session_id,
                        "scan_completed",
                        json!({
                            "force": force,
                            "node_count": outcome.summary.node_count,
                            "duration_ms": scan_started.elapsed().as_millis(),
                        }),
                    );
                }
                Ok(format!(
                    "Scanned {} nodes (root: {})",
                    outcome.summary.node_count, outcome.summary.root_node_id
                ))
            }
        }
    }

    /// Fan-in workspace + agent + provider status for `meld status`.
    #[allow(clippy::too_many_arguments)]
    pub fn unified_status(
        api: &ContextApi,
        workspace_root: &Path,
        store_path: &Path,
        agent_registry: &AgentRegistry,
        provider_registry: &crate::provider::ProviderRegistry,
        include_workspace: bool,
        include_agents: bool,
        include_providers: bool,
        include_breakdown: bool,
        test_connectivity: bool,
    ) -> Result<UnifiedStatusOutput, ApiError> {
        let workspace = if include_workspace {
            let request = WorkspaceStatusRequest {
                workspace_root: workspace_root.to_path_buf(),
                store_path: store_path.to_path_buf(),
                include_breakdown,
            };
            Some(Self::status(api, &request, agent_registry)?)
        } else {
            None
        };

        let agents = if include_agents {
            let entries = crate::agent::AgentCommandService::status(agent_registry)?;
            let total = entries.len();
            let valid_count = entries.iter().filter(|e| e.valid).count();
            let agents_vec: Vec<AgentStatusEntry> = entries
                .into_iter()
                .map(|e| AgentStatusEntry {
                    agent_id: e.agent_id,
                    role: e.role,
                    valid: e.valid,
                    prompt_path_exists: e.prompt_path_exists,
                })
                .collect();
            Some(AgentStatusOutput {
                agents: agents_vec,
                total,
                valid_count,
            })
        } else {
            None
        };

        let providers = if include_providers {
            let entries = crate::provider::commands::ProviderCommandService::run_status(
                provider_registry,
                test_connectivity,
            )?;
            let total = entries.len();
            let providers_vec: Vec<ProviderStatusEntry> = entries
                .into_iter()
                .map(|e| ProviderStatusEntry {
                    provider_name: e.provider_name,
                    provider_type: e.provider_type,
                    model: e.model,
                    connectivity: e.connectivity,
                })
                .collect();
            Some(ProviderStatusOutput {
                providers: providers_vec,
                total,
            })
        } else {
            None
        };

        Ok(UnifiedStatusOutput {
            workspace,
            agents,
            providers,
        })
    }
}

/// Durably emits the exact owner publication and preserves legacy telemetry.
pub(crate) fn emit_workspace_snapshot_facts(
    progress: &Arc<ProgressRuntime>,
    session_id: &str,
    workspace_root: &Path,
    tree: &crate::tree::builder::Tree,
    previous_root_hash: Option<&str>,
    _observed_node_ids: &[NodeID],
) -> Result<(), ApiError> {
    let mut all_observed = tree
        .nodes
        .iter()
        .map(|(node_id, node)| {
            NodeRecord::from_merkle_node(*node_id, node, tree).map_err(ApiError::StorageError)
        })
        .collect::<Result<Vec<_>, _>>()?;
    all_observed.sort_by(|left, right| left.path.cmp(&right.path));
    let mut observed = _observed_node_ids
        .iter()
        .filter_map(|node_id| {
            let node = tree.nodes.get(node_id)?;
            NodeRecord::from_merkle_node(*node_id, node, tree).ok()
        })
        .collect::<Vec<_>>();
    observed.sort_by(|left, right| left.path.cmp(&right.path));
    let mut candidates = build_publication_candidates(
        session_id,
        workspace_root,
        tree,
        previous_root_hash,
        &observed,
    )?;
    candidates.retain(|envelope| envelope.event_type != OWNER_PUBLICATION_EVENT_TYPE);
    candidates.push(build_owner_publication_candidate(
        session_id,
        workspace_root,
        tree,
        &all_observed,
    )?);
    for envelope in candidates {
        if envelope.event_type == OWNER_PUBLICATION_EVENT_TYPE {
            progress.emit_envelope_idempotent(envelope)?;
        } else {
            progress.emit_envelope_best_effort(envelope);
        }
    }
    Ok(())
}
