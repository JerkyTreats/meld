//! Workspace command service: single entry point per workspace CLI variant.
//!
//! Owns workspace workflow logic; CLI parses, calls one method per variant, and formats output.

use crate::agent::AgentRegistry;
use crate::api::ContextApi;
use crate::error::ApiError;
use crate::ignore;
use crate::store::{NodeRecord, NodeRecordStore};
use crate::telemetry::ProgressRuntime;
use crate::tree::builder::TreeBuilder;
use crate::tree::walker::WalkerConfig;
use crate::types::NodeID;
use crate::workspace::events::{
    node_observed_envelope, scan_completed_envelope, snapshot_materialized_envelope,
    snapshot_selected_envelope, source_attached_envelope,
};
use crate::workspace::section;
use crate::workspace::types::{
    AgentStatusEntry, AgentStatusOutput, IgnoreResult, ListDeletedResult, ListDeletedRow,
    ProviderStatusEntry, ProviderStatusOutput, UnifiedStatusOutput, ValidateResult,
    WorkspaceScanInfo, WorkspaceScanState, WorkspaceStatusRequest, WorkspaceStatusResult,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub(crate) fn workspace_walker_config(workspace_root: &Path) -> WalkerConfig {
    let ignore_patterns = ignore::load_ignore_patterns(workspace_root)
        .unwrap_or_else(|_| WalkerConfig::default().ignore_patterns);
    WalkerConfig {
        follow_symlinks: false,
        ignore_patterns,
        max_depth: None,
    }
}

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

pub(crate) fn stored_workspace_root_hash(
    node_store: &dyn NodeRecordStore,
    workspace_root: &Path,
    current_root_hash: &NodeID,
) -> Result<Option<String>, ApiError> {
    if node_store
        .get(current_root_hash)
        .map_err(ApiError::from)?
        .is_some()
    {
        return Ok(Some(hex::encode(current_root_hash)));
    }

    let canonical_root =
        crate::tree::path::canonicalize_path(workspace_root).map_err(ApiError::StorageError)?;
    let mapped = node_store
        .find_by_path(&canonical_root)
        .map_err(ApiError::from)?
        .map(|record| hex::encode(record.node_id));
    if mapped.is_some() {
        return Ok(mapped);
    }

    let active = node_store.list_active().map_err(ApiError::from)?;
    if let Some(record) = active
        .iter()
        .find(|record| record.parent.is_none() && record.path == Path::new("."))
    {
        return Ok(Some(hex::encode(record.node_id)));
    }

    let parentless: Vec<_> = active
        .iter()
        .filter(|record| record.parent.is_none())
        .collect();
    if parentless.len() == 1 {
        return Ok(Some(hex::encode(parentless[0].node_id)));
    }

    Ok(None)
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
    api: &ContextApi,
    workspace_root: &Path,
    path: Option<&Path>,
    node: Option<&str>,
    include_tombstoned: bool,
) -> Result<NodeID, ApiError> {
    match (path, node) {
        (Some(p), None) => {
            let resolved_path = workspace_lookup_path(workspace_root, p);
            let store = api.node_store();
            let record = if include_tombstoned {
                store.get_by_path(&resolved_path).map_err(ApiError::from)?
            } else {
                store.find_by_path(&resolved_path).map_err(ApiError::from)?
            };
            if let Some(record) = record {
                return Ok(record.node_id);
            }
            if let Ok(canonical_path) = crate::tree::path::canonicalize_path(&resolved_path) {
                let record = if include_tombstoned {
                    store.get_by_path(&canonical_path).map_err(ApiError::from)?
                } else {
                    store
                        .find_by_path(&canonical_path)
                        .map_err(ApiError::from)?
                };
                if let Some(record) = record {
                    return Ok(record.node_id);
                }
                if let Some(node_id) = resolve_node_id_by_canonical_fallback(
                    store.as_ref(),
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
            if api
                .node_store()
                .get(&node_id)
                .map_err(ApiError::from)?
                .is_none()
            {
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
    store: &dyn NodeRecordStore,
    workspace_root: &Path,
    canonical_target: &Path,
    include_tombstoned: bool,
) -> Result<Option<NodeID>, ApiError> {
    let records = if include_tombstoned {
        store.list_all().map_err(ApiError::from)?
    } else {
        store.list_active().map_err(ApiError::from)?
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
        let head_index = api.head_index().read();
        section::build_workspace_status(
            node_store,
            &head_index,
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

        let head_index = api.head_index().read();
        for node_id in head_index.get_all_node_ids() {
            let frame_ids = head_index.get_all_heads_for_node(&node_id);
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
        drop(head_index);

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
                total_heads += api.head_index().read().get_all_heads_for_node(nid).len() as u64;
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
                total_heads += api.head_index().read().get_all_heads_for_node(nid).len() as u64;
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
                    frames += api.head_index().read().get_all_heads_for_node(nid).len() as u64;
                }
                artifacts = api.prompt_context_storage().count_older_than(cutoff)?;
            }
            let head_count: usize = api
                .head_index()
                .read()
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

    /// Scan filesystem and rebuild tree: ignore load, TreeBuilder, store population, flush, ignore sync.
    /// Returns a summary string. Progress/session_id optional for telemetry events.
    pub fn scan(
        api: &ContextApi,
        workspace_root: &Path,
        force: bool,
        progress: Option<&Arc<ProgressRuntime>>,
        session_id: Option<&str>,
    ) -> Result<String, ApiError> {
        let scan_started = Instant::now();
        let ignore_patterns = ignore::load_ignore_patterns(workspace_root)
            .unwrap_or_else(|_| WalkerConfig::default().ignore_patterns);
        let walker_config = WalkerConfig {
            follow_symlinks: false,
            ignore_patterns,
            max_depth: None,
        };
        let builder =
            TreeBuilder::new(workspace_root.to_path_buf()).with_walker_config(walker_config);
        let tree = builder.build().map_err(ApiError::StorageError)?;
        let total_nodes = tree.nodes.len();
        let previous_root_hash =
            stored_workspace_root_hash(api.node_store().as_ref(), workspace_root, &tree.root_id)?;

        if !force
            && api
                .node_store()
                .get(&tree.root_id)
                .map_err(ApiError::from)?
                .is_some()
        {
            if let (Some(prog), Some(sid)) = (progress, session_id) {
                prog.emit_event_best_effort(
                    sid,
                    "scan_progress",
                    json!({
                        "node_count": total_nodes,
                        "total_nodes": total_nodes
                    }),
                );
            }
            let root_hex = hex::encode(tree.root_id);
            return Ok(format!(
                "Tree already exists (root: {}). Use --force to rebuild.",
                root_hex
            ));
        }

        let store = api.node_store().as_ref() as &dyn NodeRecordStore;
        const SCAN_PROGRESS_BATCH_NODES: usize = 128;
        let mut processed_nodes = 0usize;
        for (node_id, node) in &tree.nodes {
            let record = NodeRecord::from_merkle_node(*node_id, node, &tree)
                .map_err(ApiError::StorageError)?;
            store.put(&record).map_err(ApiError::from)?;
            processed_nodes += 1;
            if let (Some(prog), Some(sid)) = (progress, session_id) {
                if processed_nodes.is_multiple_of(SCAN_PROGRESS_BATCH_NODES)
                    || processed_nodes == total_nodes
                {
                    prog.emit_event_best_effort(
                        sid,
                        "scan_progress",
                        json!({
                            "node_count": processed_nodes,
                            "total_nodes": total_nodes
                        }),
                    );
                }
            }
        }
        if total_nodes == 0 {
            if let (Some(prog), Some(sid)) = (progress, session_id) {
                prog.emit_event_best_effort(
                    sid,
                    "scan_progress",
                    json!({
                        "node_count": 0,
                        "total_nodes": 0
                    }),
                );
            }
        }
        store.flush().map_err(ApiError::StorageError)?;

        let _ = ignore::maybe_sync_gitignore_after_tree(
            workspace_root,
            tree.find_gitignore_node_id().as_ref(),
        );

        let root_hex = hex::encode(tree.root_id);
        if let (Some(prog), Some(sid)) = (progress, session_id) {
            emit_workspace_scan_facts(
                prog,
                sid,
                workspace_root,
                &tree,
                previous_root_hash.as_deref(),
            );
            prog.emit_event_best_effort(
                sid,
                "scan_completed",
                json!({
                    "force": force,
                    "node_count": total_nodes,
                    "duration_ms": scan_started.elapsed().as_millis(),
                }),
            );
        }
        Ok(format!(
            "Scanned {} nodes (root: {})",
            total_nodes, root_hex
        ))
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

pub(crate) fn emit_workspace_snapshot_facts(
    progress: &Arc<ProgressRuntime>,
    session_id: &str,
    workspace_root: &Path,
    tree: &crate::tree::builder::Tree,
    previous_root_hash: Option<&str>,
    observed_node_ids: &[NodeID],
) {
    let current_root_hex = hex::encode(tree.root_id);
    if previous_root_hash.is_none() {
        progress.emit_envelope_best_effort(source_attached_envelope(session_id, workspace_root));
    }
    progress.emit_envelope_best_effort(snapshot_materialized_envelope(
        session_id,
        workspace_root,
        tree.root_id,
    ));
    let previous_root_node_id = previous_root_hash.and_then(decode_node_id_hex);
    if previous_root_hash != Some(current_root_hex.as_str()) {
        progress.emit_envelope_best_effort(snapshot_selected_envelope(
            session_id,
            workspace_root,
            tree.root_id,
            previous_root_node_id,
        ));
    }
    for node_id in observed_node_ids {
        let Some(node) = tree.nodes.get(node_id) else {
            continue;
        };
        let record = match NodeRecord::from_merkle_node(*node_id, node, tree) {
            Ok(record) => record,
            Err(_) => continue,
        };
        progress.emit_envelope_best_effort(node_observed_envelope(
            session_id,
            workspace_root,
            tree.root_id,
            &record,
        ));
    }
}

fn emit_workspace_scan_facts(
    progress: &Arc<ProgressRuntime>,
    session_id: &str,
    workspace_root: &Path,
    tree: &crate::tree::builder::Tree,
    previous_root_hash: Option<&str>,
) {
    let observed_node_ids: Vec<_> = tree.nodes.keys().copied().collect();
    emit_workspace_snapshot_facts(
        progress,
        session_id,
        workspace_root,
        tree,
        previous_root_hash,
        &observed_node_ids,
    );
    progress.emit_envelope_best_effort(scan_completed_envelope(
        session_id,
        workspace_root,
        tree.nodes.len(),
    ));
}

fn decode_node_id_hex(value: &str) -> Option<NodeID> {
    let bytes = hex::decode(value).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut node_id = [0u8; 32];
    node_id.copy_from_slice(&bytes);
    Some(node_id)
}
