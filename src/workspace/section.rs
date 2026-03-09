//! Internal workspace-section build used by status and unified_status.

use crate::agent::{AgentRegistry, AgentRole};
use crate::error::ApiError;
use crate::heads::HeadIndex;
use crate::store::NodeRecord;
use crate::store::NodeRecordStore;
use crate::types::NodeID;
use crate::workspace::commands::{assess_workspace_scan_state, current_workspace_root_hash};
use crate::workspace::types::{
    ContextCoverageEntry, PathCount, TreeStatus, WorkspaceScanState, WorkspaceStatus,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Build workspace status from store, head index, agent registry, and workspace root.
///
/// When `include_breakdown` is true, the tree section includes top-level path breakdown.
pub fn build_workspace_status(
    node_store: &dyn NodeRecordStore,
    head_index: &HeadIndex,
    agent_registry: &AgentRegistry,
    workspace_root: &Path,
    store_path: &Path,
    include_breakdown: bool,
) -> Result<WorkspaceStatus, ApiError> {
    fn collect_reachable_records(
        node_store: &dyn NodeRecordStore,
        root_id: NodeID,
    ) -> Result<Vec<NodeRecord>, ApiError> {
        let mut visited: HashSet<NodeID> = HashSet::new();
        let mut queue = VecDeque::from([root_id]);
        let mut records = Vec::new();

        while let Some(node_id) = queue.pop_front() {
            if !visited.insert(node_id) {
                continue;
            }
            let Some(record) = node_store.get(&node_id).map_err(ApiError::from)? else {
                continue;
            };
            if record.tombstoned_at.is_some() {
                continue;
            }
            for child in &record.children {
                queue.push_back(*child);
            }
            records.push(record);
        }

        Ok(records)
    }

    let scan_info = assess_workspace_scan_state(node_store, workspace_root)?;
    if matches!(scan_info.scan_state, WorkspaceScanState::Missing) {
        return Ok(WorkspaceStatus {
            scanned: false,
            scan_state: scan_info.scan_state,
            store_path: normalize_display_path(store_path),
            message: Some("Run meld scan to build the tree.".to_string()),
            current_root_hash: Some(scan_info.current_root_hash),
            stored_root_hash: scan_info.stored_root_hash,
            tree: None,
            context_coverage: None,
            top_paths_by_node_count: None,
        });
    }

    let records = if matches!(scan_info.scan_state, WorkspaceScanState::Current) {
        let root_id = current_workspace_root_hash(workspace_root)?;
        collect_reachable_records(node_store, root_id)?
    } else {
        node_store.list_active().map_err(ApiError::from)?
    };
    let total_nodes = records.len() as u64;

    let workspace_root_buf = workspace_root.to_path_buf();
    let mut prefix_counts: HashMap<String, u64> = HashMap::new();
    for record in &records {
        let rel = record
            .path
            .strip_prefix(&workspace_root_buf)
            .unwrap_or(record.path.as_path());
        let first = rel
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        let key = if first.is_empty() {
            ".".to_string()
        } else {
            first
        };
        *prefix_counts.entry(key).or_insert(0) += 1;
    }

    let mut top_paths: Vec<PathCount> = vec![PathCount {
        path: ".".to_string(),
        nodes: total_nodes,
    }];
    let mut rest: Vec<(String, u64)> = prefix_counts
        .iter()
        .filter(|(k, _)| *k != ".")
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    rest.sort_by(|a, b| b.1.cmp(&a.1));
    for (path, nodes) in rest.into_iter().take(4) {
        top_paths.push(PathCount {
            path: path + "/",
            nodes,
        });
    }

    let breakdown = if include_breakdown {
        let mut by_count: Vec<(String, u64)> = prefix_counts
            .iter()
            .map(|(k, v)| {
                let path = if *k == "." {
                    ".".to_string()
                } else {
                    k.clone() + "/"
                };
                (path, *v)
            })
            .collect();
        by_count.sort_by(|a, b| b.1.cmp(&a.1));
        Some(
            by_count
                .into_iter()
                .map(|(path, nodes)| PathCount { path, nodes })
                .collect(),
        )
    } else {
        None
    };

    let writers = agent_registry.list_by_role(Some(AgentRole::Writer));
    let mut agent_ids: std::collections::HashSet<String> =
        writers.iter().map(|a| a.agent_id.clone()).collect();
    let mut context_coverage: Vec<ContextCoverageEntry> = Vec::new();
    for agent_id in agent_ids.drain() {
        let frame_type = format!("context-{}", agent_id);
        let nodes_with_frame = head_index.count_nodes_for_frame_type(&frame_type) as u64;
        let nodes_without_frame = total_nodes.saturating_sub(nodes_with_frame);
        let coverage_pct = if total_nodes > 0 {
            Some((nodes_with_frame * 100) / total_nodes)
        } else {
            Some(0)
        };
        context_coverage.push(ContextCoverageEntry {
            agent_id,
            nodes_with_frame,
            nodes_without_frame,
            coverage_pct,
        });
    }
    context_coverage.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));

    Ok(WorkspaceStatus {
        scanned: true,
        scan_state: scan_info.scan_state,
        store_path: normalize_display_path(store_path),
        message: if matches!(scan_info.scan_state, WorkspaceScanState::Stale) {
            Some("Run meld scan to refresh tree to current workspace state.".to_string())
        } else {
            None
        },
        current_root_hash: Some(scan_info.current_root_hash.clone()),
        stored_root_hash: scan_info.stored_root_hash.clone(),
        tree: Some(TreeStatus {
            root_hash: scan_info.current_root_hash,
            total_nodes,
            breakdown,
        }),
        context_coverage: Some(context_coverage),
        top_paths_by_node_count: Some(top_paths),
    })
}

fn normalize_display_path(path: &Path) -> String {
    let buf: PathBuf = path.to_path_buf();
    buf.display().to_string()
}
