//! Workspace scan contract and scan execution core.
//!
//! Owns the typed request, outcome, and publication-candidate shapes for one
//! workspace scan. Both the CLI scan command and the `workspace_scan`
//! capability run scans through this module so workspace tree updates happen
//! in one place. The core never appends canonical events: it reports
//! publication candidates for the owning publication runtime to append.

use crate::api::ContextApi;
use crate::error::ApiError;
use crate::events::{DomainObjectRef, EventEnvelope};
use crate::ignore;
use crate::store::{NodeRecord, NodeRecordStore};
use crate::tree::builder::{Tree, TreeBuilder};
use crate::tree::walker::WalkerConfig;
use crate::types::NodeID;
use crate::workspace::events::{
    node_observed_envelope, owner_publication_operation, scan_completed_envelope,
    snapshot_materialized_envelope, snapshot_ref, snapshot_selected_envelope,
    source_attached_envelope,
};
use meld_world_model::world_state::graph::events::owner_publication_envelope;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Emission cadence for scan progress observers, in nodes.
const SCAN_PROGRESS_BATCH_NODES: usize = 128;

/// Scan policy accepted by the workspace scan contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceScanPolicy {
    /// True when the tree must be rebuilt even if the current root already exists.
    #[serde(default)]
    pub force: bool,
}

fn default_collect_observed() -> bool {
    true
}

/// Typed scan request accepted through the workspace scan port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceScanRequest {
    /// Workspace root scanned by this request.
    pub workspace_root: PathBuf,
    /// Scan policy applied by this request.
    pub policy: WorkspaceScanPolicy,
    /// Session shaping publication candidates; no candidates without it.
    pub session_id: Option<String>,
    /// True when the outcome must carry per-node observed refs, the default.
    /// Callers that only read scan facts, like the CLI scan command, opt out
    /// so the up-to-date path skips building a record per tree node.
    #[serde(default = "default_collect_observed")]
    pub collect_observed: bool,
}

/// Terminal disposition of one scan execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceScanStatus {
    /// Tree was rebuilt and written to the workspace store.
    Scanned,
    /// Stored tree already matches the filesystem; nothing was written.
    UpToDate,
}

/// Structured scan facts reported by one scan execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceScanSummary {
    /// Terminal disposition of this scan.
    pub status: WorkspaceScanStatus,
    /// Node count observed in the built tree.
    pub node_count: usize,
    /// Hex root node identifier of the built tree.
    pub root_node_id: String,
    /// Hex root node identifier stored before this scan, when one existed.
    pub previous_root_node_id: Option<String>,
    /// True when the request forced a rebuild.
    pub force: bool,
}

/// One node reference observed by a scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceObservedNodeRef {
    /// Hex node identifier.
    pub node_id: String,
    /// Canonical node path.
    pub path: String,
    /// Hex parent node identifier, absent for the workspace root.
    pub parent_node_id: Option<String>,
}

/// Typed scan outcome returned through the workspace scan port.
#[derive(Debug, Clone)]
pub struct WorkspaceScanOutcome {
    /// Structured scan facts for this execution.
    pub summary: WorkspaceScanSummary,
    /// Durable snapshot object reference for the built tree.
    pub snapshot_ref: DomainObjectRef,
    /// Workspace root node observed by this scan.
    pub root_node_ref: WorkspaceObservedNodeRef,
    /// All node references observed by this scan, ordered by path. Empty
    /// when the request opted out of observed-ref collection.
    pub observed_nodes: Vec<WorkspaceObservedNodeRef>,
    /// Event envelopes for the publication runtime to append; never appended here.
    pub publication_candidates: Vec<EventEnvelope>,
}

/// Observer invoked with `(processed_nodes, total_nodes)` at scan batch points.
pub(crate) type ScanProgressObserver<'a> = &'a mut dyn FnMut(usize, usize);

/// Builds the walker configuration shared by every workspace tree build.
pub(crate) fn workspace_walker_config(workspace_root: &Path) -> WalkerConfig {
    let ignore_patterns = ignore::load_ignore_patterns(workspace_root)
        .unwrap_or_else(|_| WalkerConfig::default().ignore_patterns);
    WalkerConfig {
        follow_symlinks: false,
        ignore_patterns,
        max_depth: None,
    }
}

/// Resolves the workspace root hash recorded by a previous scan, falling back
/// through canonical-path and parentless-root lookups for legacy stores.
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

/// Decodes a 32-byte hex node id, returning `None` for malformed values.
pub(crate) fn decode_node_id_hex(value: &str) -> Option<NodeID> {
    let bytes = hex::decode(value).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut node_id = [0u8; 32];
    node_id.copy_from_slice(&bytes);
    Some(node_id)
}

/// Executes one workspace scan and returns its typed outcome.
pub fn execute_workspace_scan(
    api: &ContextApi,
    request: &WorkspaceScanRequest,
) -> Result<WorkspaceScanOutcome, ApiError> {
    execute_workspace_scan_observed(api, request, None)
}

/// Executes one workspace scan, reporting progress at CLI-compatible batch points.
pub(crate) fn execute_workspace_scan_observed(
    api: &ContextApi,
    request: &WorkspaceScanRequest,
    mut progress: Option<ScanProgressObserver<'_>>,
) -> Result<WorkspaceScanOutcome, ApiError> {
    let workspace_root = request.workspace_root.as_path();
    let tree = TreeBuilder::new(workspace_root.to_path_buf())
        .with_walker_config(workspace_walker_config(workspace_root))
        .build()
        .map_err(ApiError::StorageError)?;
    let total_nodes = tree.nodes.len();
    let previous_root_hash =
        stored_workspace_root_hash(api.node_store().as_ref(), workspace_root, &tree.root_id)?;

    if !request.policy.force
        && api
            .node_store()
            .get(&tree.root_id)
            .map_err(ApiError::from)?
            .is_some()
    {
        if let Some(observer) = progress.as_mut() {
            observer(total_nodes, total_nodes);
        }
        let observed = if request.collect_observed || request.session_id.is_some() {
            collect_records(&tree)?
        } else {
            // Root-only record: build_outcome still resolves root_node_ref
            // from it, and opted-out callers never read observed_nodes.
            vec![root_record(&tree)?]
        };
        let publication_candidates = match request.session_id.as_deref() {
            Some(session_id) => vec![build_owner_publication_candidate(
                session_id,
                workspace_root,
                &tree,
                &observed,
            )?],
            None => Vec::new(),
        };
        return build_outcome(
            WorkspaceScanStatus::UpToDate,
            request,
            &tree,
            previous_root_hash,
            observed,
            publication_candidates,
        );
    }

    let store = api.node_store().as_ref() as &dyn NodeRecordStore;
    let mut processed_nodes = 0usize;
    let mut observed = Vec::with_capacity(total_nodes);
    for (node_id, node) in &tree.nodes {
        let record =
            NodeRecord::from_merkle_node(*node_id, node, &tree).map_err(ApiError::StorageError)?;
        store.put(&record).map_err(ApiError::from)?;
        observed.push(record);
        processed_nodes += 1;
        if let Some(observer) = progress.as_mut() {
            if processed_nodes.is_multiple_of(SCAN_PROGRESS_BATCH_NODES)
                || processed_nodes == total_nodes
            {
                observer(processed_nodes, total_nodes);
            }
        }
    }
    if total_nodes == 0 {
        if let Some(observer) = progress.as_mut() {
            observer(0, 0);
        }
    }
    // Tree nodes iterate in hash-map order; sorting here makes publication
    // candidates and outcome ordering a pure function of the tree.
    sort_records_by_path(&mut observed);
    store.flush().map_err(ApiError::StorageError)?;

    let _ = ignore::maybe_sync_gitignore_after_tree(
        workspace_root,
        tree.find_gitignore_node_id().as_ref(),
    );

    let publication_candidates = match request.session_id.as_deref() {
        Some(session_id) => {
            let mut candidates = build_publication_candidates(
                session_id,
                workspace_root,
                &tree,
                previous_root_hash.as_deref(),
                &observed,
            )?;
            // Only full scans terminate with a scan_completed fact; the
            // shared builder is also reused by the watch path, which does not.
            candidates.push(scan_completed_envelope(
                session_id,
                workspace_root,
                tree.nodes.len(),
            ));
            candidates
        }
        None => Vec::new(),
    };
    build_outcome(
        WorkspaceScanStatus::Scanned,
        request,
        &tree,
        previous_root_hash,
        observed,
        publication_candidates,
    )
}

fn root_record(tree: &Tree) -> Result<NodeRecord, ApiError> {
    let root_node = tree.nodes.get(&tree.root_id).ok_or_else(|| {
        ApiError::ConfigError("Workspace tree is missing its root node".to_string())
    })?;
    NodeRecord::from_merkle_node(tree.root_id, root_node, tree).map_err(ApiError::StorageError)
}

fn collect_records(tree: &Tree) -> Result<Vec<NodeRecord>, ApiError> {
    let mut records: Vec<NodeRecord> = tree
        .nodes
        .iter()
        .map(|(node_id, node)| {
            NodeRecord::from_merkle_node(*node_id, node, tree).map_err(ApiError::StorageError)
        })
        .collect::<Result<_, _>>()?;
    sort_records_by_path(&mut records);
    Ok(records)
}

fn sort_records_by_path(records: &mut [NodeRecord]) {
    records.sort_by(|left, right| left.path.cmp(&right.path));
}

/// Builds the workspace snapshot fact sequence for one tree observation:
/// source attachment on first sight, snapshot materialization, snapshot
/// selection on root change, then one node_observed per record. Shared by
/// the scan path (which appends its own scan_completed fact) and the watch
/// path so the sequence cannot drift between them.
pub(crate) fn build_publication_candidates(
    session_id: &str,
    workspace_root: &Path,
    tree: &Tree,
    previous_root_hash: Option<&str>,
    observed: &[NodeRecord],
) -> Result<Vec<EventEnvelope>, ApiError> {
    let current_root_hex = hex::encode(tree.root_id);
    let mut candidates = Vec::with_capacity(observed.len() + 3);
    if previous_root_hash.is_none() {
        candidates.push(source_attached_envelope(session_id, workspace_root));
    }
    candidates.push(snapshot_materialized_envelope(
        session_id,
        workspace_root,
        tree.root_id,
    ));
    if previous_root_hash != Some(current_root_hex.as_str()) {
        candidates.push(snapshot_selected_envelope(
            session_id,
            workspace_root,
            tree.root_id,
            previous_root_hash.and_then(decode_node_id_hex),
        ));
    }
    for record in observed {
        candidates.push(node_observed_envelope(
            session_id,
            workspace_root,
            tree.root_id,
            record,
        ));
    }
    candidates.push(build_owner_publication_candidate(
        session_id,
        workspace_root,
        tree,
        observed,
    )?);
    Ok(candidates)
}

pub(crate) fn build_owner_publication_candidate(
    session_id: &str,
    workspace_root: &Path,
    tree: &Tree,
    observed: &[NodeRecord],
) -> Result<EventEnvelope, ApiError> {
    let operation = owner_publication_operation(workspace_root, tree.root_id, observed)
        .map_err(ApiError::StorageError)?;
    owner_publication_envelope(session_id, &operation)
        .map_err(|error| ApiError::ConfigError(error.to_string()))
}

// Callers supply `observed` already sorted by path.
fn build_outcome(
    status: WorkspaceScanStatus,
    request: &WorkspaceScanRequest,
    tree: &Tree,
    previous_root_hash: Option<String>,
    observed: Vec<NodeRecord>,
    publication_candidates: Vec<EventEnvelope>,
) -> Result<WorkspaceScanOutcome, ApiError> {
    let root_record = observed
        .iter()
        .find(|record| record.node_id == tree.root_id)
        .ok_or_else(|| {
            ApiError::ConfigError("Workspace scan did not observe the workspace root".to_string())
        })?;
    let snapshot = snapshot_ref(tree.root_id).map_err(ApiError::StorageError)?;

    Ok(WorkspaceScanOutcome {
        summary: WorkspaceScanSummary {
            status,
            node_count: tree.nodes.len(),
            root_node_id: hex::encode(tree.root_id),
            previous_root_node_id: previous_root_hash,
            force: request.policy.force,
        },
        snapshot_ref: snapshot,
        root_node_ref: observed_node_ref(root_record),
        observed_nodes: if request.collect_observed {
            observed.iter().map(observed_node_ref).collect()
        } else {
            Vec::new()
        },
        publication_candidates,
    })
}

fn observed_node_ref(record: &NodeRecord) -> WorkspaceObservedNodeRef {
    WorkspaceObservedNodeRef {
        node_id: hex::encode(record.node_id),
        path: record.path.to_string_lossy().to_string(),
        parent_node_id: record.parent.map(hex::encode),
    }
}
