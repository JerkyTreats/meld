use std::collections::BTreeMap;
use std::path::Path;

use meld_world_model::world_state::graph::contracts::{
    HydrationReference, OwnerCompletenessReceipt, OwnerCompletenessStatus, OwnerObjectPublication,
    OwnerPublicationBatch, OwnerPublicationOperation, OwnerPublicationScope, OwnerPublicationState,
    OwnerRelationOccurrence,
};
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
    Ok(DomainObjectRef::new(
        "workspace_fs",
        "source",
        normalize_path_string(&canonical.to_string_lossy()),
    )?)
}

pub fn snapshot_ref(root_node_id: NodeID) -> Result<DomainObjectRef, crate::error::StorageError> {
    Ok(DomainObjectRef::new(
        "workspace_fs",
        "snapshot",
        hex::encode(root_node_id),
    )?)
}

pub fn snapshot_head_ref(
    workspace_root: &Path,
) -> Result<DomainObjectRef, crate::error::StorageError> {
    let source = source_ref(workspace_root)?;
    Ok(DomainObjectRef::new(
        "workspace_fs",
        "snapshot_head",
        source.object_id,
    )?)
}

pub fn node_ref(node_id: NodeID) -> Result<DomainObjectRef, crate::error::StorageError> {
    Ok(DomainObjectRef::new(
        "workspace_fs",
        "node",
        hex::encode(node_id),
    )?)
}

/// Reconstruct the exact owner operation for one workspace tree revision.
pub fn owner_publication_operation(
    workspace_root: &Path,
    root_node_id: NodeID,
    records: &[NodeRecord],
) -> Result<OwnerPublicationOperation, crate::error::StorageError> {
    const OWNER_ID: &str = "workspace_fs";
    const RULE_REVISION: &str = "workspace-tree-owner-publication-v1";

    let source = source_ref(workspace_root)?;
    let snapshot = snapshot_ref(root_node_id)?;
    let head = snapshot_head_ref(workspace_root)?;
    let revision_id = hex::encode(root_node_id);
    let scope = OwnerPublicationScope {
        scope_id: source.object_id.clone(),
        branch_id: None,
        perspective_id: None,
        valid_at: None,
    };
    let mut objects = vec![
        owner_object(
            &source,
            "source",
            &source.object_id,
            &revision_id,
            BTreeMap::new(),
        ),
        owner_object(
            &snapshot,
            "snapshot",
            &revision_id,
            &revision_id,
            BTreeMap::new(),
        ),
        owner_object(
            &head,
            "snapshot_head",
            &source.object_id,
            &revision_id,
            BTreeMap::new(),
        ),
    ];
    let mut relations = vec![
        owner_relation(
            "snapshot-belongs-to-source",
            "belongs_to",
            &snapshot,
            &source,
            &revision_id,
        ),
        owner_relation(
            "head-attached-to-source",
            "attached_to",
            &head,
            &source,
            &revision_id,
        ),
        owner_relation(
            "head-selects-snapshot",
            "selected",
            &head,
            &snapshot,
            &revision_id,
        ),
    ];
    for record in records {
        let node = node_ref(record.node_id)?;
        let node_id = hex::encode(record.node_id);
        let mut qualifications = BTreeMap::new();
        qualifications.insert(
            "path".to_string(),
            record.path.to_string_lossy().to_string(),
        );
        qualifications.insert(
            "node_kind".to_string(),
            match record.node_type {
                crate::store::NodeType::Directory => "directory",
                crate::store::NodeType::File { .. } => "file",
            }
            .to_string(),
        );
        objects.push(owner_object(
            &node,
            "node",
            &node_id,
            &revision_id,
            qualifications,
        ));
        relations.push(owner_relation(
            &format!("node-belongs-to-source::{node_id}"),
            "belongs_to",
            &node,
            &source,
            &revision_id,
        ));
        relations.push(owner_relation(
            &format!("node-observed-in-snapshot::{node_id}"),
            "observed_in",
            &node,
            &snapshot,
            &revision_id,
        ));
        if let Some(parent_id) = record.parent {
            let parent = node_ref(parent_id)?;
            relations.push(owner_relation(
                &format!(
                    "parent-contains-node::{}::{node_id}",
                    hex::encode(parent_id)
                ),
                "contains",
                &parent,
                &node,
                &revision_id,
            ));
        }
    }
    let mut included_ids = objects
        .iter()
        .map(|object| object.publication_id.clone())
        .chain(
            relations
                .iter()
                .map(|relation| relation.occurrence_id.clone()),
        )
        .collect::<Vec<_>>();
    included_ids.sort();
    OwnerPublicationOperation::reconstruct(
        RULE_REVISION,
        OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: OWNER_ID.to_string(),
            revision_id: revision_id.clone(),
            scope: scope.clone(),
            objects,
            relations,
            completeness: OwnerCompletenessReceipt {
                receipt_id: format!("workspace-completeness::{revision_id}"),
                scope,
                included_ids,
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .map_err(|error| crate::error::StorageError::InvalidPath(error.to_string()))
}

fn owner_object(
    object_ref: &DomainObjectRef,
    product_kind: &str,
    product_id: &str,
    revision_id: &str,
    qualifications: BTreeMap<String, String>,
) -> OwnerObjectPublication {
    OwnerObjectPublication {
        publication_id: format!("object::{}", object_ref.index_key()),
        object_ref: object_ref.clone(),
        state: OwnerPublicationState::Observed,
        source_product_ref: product_id.to_string(),
        hydration: HydrationReference {
            owner_id: "workspace_fs".to_string(),
            product_kind: product_kind.to_string(),
            product_id: product_id.to_string(),
            revision_id: revision_id.to_string(),
            role: "published_material".to_string(),
        },
        provenance_refs: vec![format!("workspace-revision::{revision_id}")],
        qualifications,
    }
}

fn owner_relation(
    occurrence_id: &str,
    relation_type: &str,
    src: &DomainObjectRef,
    dst: &DomainObjectRef,
    revision_id: &str,
) -> OwnerRelationOccurrence {
    OwnerRelationOccurrence {
        occurrence_id: occurrence_id.to_string(),
        relation_type: relation_type.to_string(),
        src: src.clone(),
        dst: dst.clone(),
        source_product_ref: occurrence_id.to_string(),
        hydration: HydrationReference {
            owner_id: "workspace_fs".to_string(),
            product_kind: "workspace_relation".to_string(),
            product_id: occurrence_id.to_string(),
            revision_id: revision_id.to_string(),
            role: "qualified_relation".to_string(),
        },
        qualifications: BTreeMap::new(),
        provenance_refs: vec![format!("workspace-revision::{revision_id}")],
    }
}
