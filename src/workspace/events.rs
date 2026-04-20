use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};
use crate::store::NodeRecord;
use crate::tree::path::{canonicalize_path, normalize_path_string};
use crate::types::NodeID;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSourceEventData {
    pub source_id: String,
    pub workspace_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceScanEventData {
    pub source_id: String,
    pub workspace_root: String,
    pub node_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSnapshotEventData {
    pub source_id: String,
    pub snapshot_id: String,
    pub root_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceNodeObservedEventData {
    pub source_id: String,
    pub snapshot_id: String,
    pub node_id: String,
    pub path: String,
    pub parent_node_id: Option<String>,
}

fn workspace_envelope(
    session_id: &str,
    stream_id: &str,
    event_type: &str,
    data: serde_json::Value,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
) -> EventEnvelope {
    EventEnvelope::with_now_domain(
        session_id.to_string(),
        "workspace_fs".to_string(),
        stream_id.to_string(),
        event_type.to_string(),
        None,
        data,
    )
    .with_graph(objects, relations)
}

pub fn source_attached_envelope(session_id: &str, workspace_root: &Path) -> EventEnvelope {
    let source = source_ref(workspace_root).expect("workspace source ref should be valid");
    let stream_id = source.object_id.clone();
    workspace_envelope(
        session_id,
        &stream_id,
        "workspace_fs.source_attached",
        json!(WorkspaceSourceEventData {
            source_id: source.object_id.clone(),
            workspace_root: source.object_id.clone(),
        }),
        vec![source],
        Vec::new(),
    )
}

pub fn scan_started_envelope(
    session_id: &str,
    workspace_root: &Path,
    node_count: usize,
) -> EventEnvelope {
    let source = source_ref(workspace_root).expect("workspace source ref should be valid");
    let stream_id = source.object_id.clone();
    workspace_envelope(
        session_id,
        &stream_id,
        "workspace_fs.scan_started",
        json!(WorkspaceScanEventData {
            source_id: source.object_id.clone(),
            workspace_root: source.object_id.clone(),
            node_count,
        }),
        vec![source],
        Vec::new(),
    )
}

pub fn scan_completed_envelope(
    session_id: &str,
    workspace_root: &Path,
    node_count: usize,
) -> EventEnvelope {
    let source = source_ref(workspace_root).expect("workspace source ref should be valid");
    let stream_id = source.object_id.clone();
    workspace_envelope(
        session_id,
        &stream_id,
        "workspace_fs.scan_completed",
        json!(WorkspaceScanEventData {
            source_id: source.object_id.clone(),
            workspace_root: source.object_id.clone(),
            node_count,
        }),
        vec![source],
        Vec::new(),
    )
}

pub fn snapshot_materialized_envelope(
    session_id: &str,
    workspace_root: &Path,
    root_node_id: NodeID,
) -> EventEnvelope {
    let source = source_ref(workspace_root).expect("workspace source ref should be valid");
    let snapshot = snapshot_ref(root_node_id).expect("workspace snapshot ref should be valid");
    let stream_id = source.object_id.clone();
    workspace_envelope(
        session_id,
        &stream_id,
        "workspace_fs.snapshot_materialized",
        json!(WorkspaceSnapshotEventData {
            source_id: source.object_id.clone(),
            snapshot_id: snapshot.object_id.clone(),
            root_node_id: hex::encode(root_node_id),
        }),
        vec![source.clone(), snapshot.clone()],
        vec![EventRelation::new("belongs_to", snapshot, source)
            .expect("workspace belongs_to relation should be valid")],
    )
}

pub fn snapshot_selected_envelope(
    session_id: &str,
    workspace_root: &Path,
    root_node_id: NodeID,
    previous_root_node_id: Option<NodeID>,
) -> EventEnvelope {
    let source = source_ref(workspace_root).expect("workspace source ref should be valid");
    let head =
        snapshot_head_ref(workspace_root).expect("workspace snapshot head ref should be valid");
    let snapshot = snapshot_ref(root_node_id).expect("workspace snapshot ref should be valid");
    let stream_id = source.object_id.clone();
    let mut relations = vec![
        EventRelation::new("attached_to", head.clone(), source.clone())
            .expect("workspace attached_to relation should be valid"),
        EventRelation::new("selected", head, snapshot.clone())
            .expect("workspace selected relation should be valid"),
    ];
    if let Some(previous_root_node_id) = previous_root_node_id {
        relations.push(
            EventRelation::new(
                "supersedes",
                snapshot.clone(),
                snapshot_ref(previous_root_node_id).expect("previous snapshot ref should be valid"),
            )
            .expect("workspace supersedes relation should be valid"),
        );
    }
    workspace_envelope(
        session_id,
        &stream_id,
        "workspace_fs.snapshot_selected",
        json!(WorkspaceSnapshotEventData {
            source_id: source.object_id.clone(),
            snapshot_id: snapshot.object_id.clone(),
            root_node_id: hex::encode(root_node_id),
        }),
        vec![
            source.clone(),
            snapshot_head_ref(workspace_root).expect("workspace snapshot head ref should be valid"),
            snapshot,
        ],
        relations,
    )
}

pub fn node_observed_envelope(
    session_id: &str,
    workspace_root: &Path,
    root_node_id: NodeID,
    record: &NodeRecord,
) -> EventEnvelope {
    let source = source_ref(workspace_root).expect("workspace source ref should be valid");
    let snapshot = snapshot_ref(root_node_id).expect("workspace snapshot ref should be valid");
    let node = node_ref(record.node_id).expect("workspace node ref should be valid");
    let stream_id = source.object_id.clone();
    let mut relations = vec![
        EventRelation::new("belongs_to", node.clone(), source.clone())
            .expect("workspace belongs_to relation should be valid"),
        EventRelation::new("observed_in", node.clone(), snapshot.clone())
            .expect("workspace observed_in relation should be valid"),
    ];
    let mut objects = vec![source.clone(), snapshot.clone(), node.clone()];
    if let Some(parent_node_id) = record.parent {
        let parent = node_ref(parent_node_id).expect("workspace parent ref should be valid");
        relations.push(
            EventRelation::new("contains", parent.clone(), node.clone())
                .expect("workspace contains relation should be valid"),
        );
        objects.push(parent);
    }
    workspace_envelope(
        session_id,
        &stream_id,
        "workspace_fs.node_observed",
        json!(WorkspaceNodeObservedEventData {
            source_id: source.object_id.clone(),
            snapshot_id: snapshot.object_id.clone(),
            node_id: node.object_id.clone(),
            path: record.path.to_string_lossy().to_string(),
            parent_node_id: record.parent.map(hex::encode),
        }),
        objects,
        relations,
    )
}

pub fn source_ref(workspace_root: &Path) -> Result<DomainObjectRef, crate::error::StorageError> {
    let canonical = canonicalize_path(workspace_root)?;
    DomainObjectRef::new(
        "workspace_fs",
        "source",
        normalize_path_string(&canonical.to_string_lossy()),
    )
}

pub fn snapshot_ref(root_node_id: NodeID) -> Result<DomainObjectRef, crate::error::StorageError> {
    DomainObjectRef::new("workspace_fs", "snapshot", hex::encode(root_node_id))
}

pub fn snapshot_head_ref(
    workspace_root: &Path,
) -> Result<DomainObjectRef, crate::error::StorageError> {
    let source = source_ref(workspace_root)?;
    DomainObjectRef::new("workspace_fs", "snapshot_head", source.object_id)
}

pub fn node_ref(node_id: NodeID) -> Result<DomainObjectRef, crate::error::StorageError> {
    DomainObjectRef::new("workspace_fs", "node", hex::encode(node_id))
}
