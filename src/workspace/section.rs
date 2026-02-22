//! Internal workspace-section build used by status and unified_status.

use crate::agent::{AgentRegistry, AgentRole};
use crate::error::ApiError;
use crate::heads::HeadIndex;
use crate::ignore;
use crate::store::NodeRecordStore;
use crate::tree::builder::TreeBuilder;
use crate::tree::walker::WalkerConfig;
use crate::types::NodeID;
use crate::workspace::types::{
    ContextCoverageEntry, PathCount, TreeStatus, WorkspaceStatus,
};
use std::collections::HashMap;
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
    let ignore_patterns = ignore::load_ignore_patterns(workspace_root)
        .unwrap_or_else(|_| WalkerConfig::default().ignore_patterns);
    let walker_config = WalkerConfig {
        follow_symlinks: false,
        ignore_patterns,
        max_depth: None,
    };
    let root_hash: NodeID = TreeBuilder::new(workspace_root.to_path_buf())
        .with_walker_config(walker_config)
        .compute_root()
        .map_err(ApiError::from)?;

    let root_in_store = node_store
        .get(&root_hash)
        .map_err(ApiError::from)?
        .is_some();
    if !root_in_store {
        return Ok(WorkspaceStatus {
            scanned: false,
            store_path: normalize_display_path(store_path),
            message: Some("Run merkle scan to build the tree.".to_string()),
            tree: None,
            context_coverage: None,
            top_paths_by_node_count: None,
        });
    }

    let records = node_store.list_active().map_err(ApiError::from)?;
    let total_nodes = records.len() as u64;
    let root_hash_hex = hex::encode(root_hash);

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
        store_path: normalize_display_path(store_path),
        message: None,
        tree: Some(TreeStatus {
            root_hash: root_hash_hex,
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
