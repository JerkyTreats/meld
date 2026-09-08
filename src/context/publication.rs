//! Context publishes its durable selections without delegating their meaning to Graph.

use std::collections::BTreeMap;

use crate::context::frame::FrameStorage;
use crate::context::head::{frame_ref, head_ref, node_ref};
use crate::error::{ApiError, StorageError};
use crate::heads::HeadIndex;
use crate::world_state::graph::contracts::*;
use crate::world_state::graph::events::owner_publication_envelope;
use meld_events::{AppendMode, EventAppendCapability};

/// Reconstruct one retryable publication from the exact persisted Context revision.
pub fn head_publication(
    heads: &HeadIndex,
    frames: &FrameStorage,
) -> Result<OwnerPublicationOperation, StorageError> {
    let revision = heads.revision_id().to_string();
    let scope = OwnerPublicationScope {
        scope_id: format!("context-frame-heads::{}", heads.source_id()),
        branch_id: None,
        perspective_id: None,
        valid_at: None,
    };
    let mut objects = BTreeMap::new();
    let mut relations = Vec::new();
    for entry in heads.entries() {
        let mut head = head_ref(entry.node_id, &entry.frame_type);
        head.object_id = format!("{}::{}", heads.source_id(), head.object_id);
        let frame = frame_ref(entry.frame_id);
        let active = entry.tombstoned_at.is_none();
        if active && frames.get(&entry.frame_id)?.is_none() {
            return Err(StorageError::InvalidPath(format!(
                "Context head {} references missing frame {}",
                head.index_key(),
                frame.index_key()
            )));
        }
        let hydration = HydrationReference {
            owner_id: "context".into(),
            product_kind: "frame_head".into(),
            product_id: head.object_id.clone(),
            revision_id: revision.clone(),
            role: "current_selection".into(),
        };
        let source = format!("context-head-revision::{revision}");
        objects.insert(
            head.index_key(),
            OwnerObjectPublication {
                publication_id: head.index_key(),
                object_ref: head.clone(),
                state: if active {
                    OwnerPublicationState::Observed
                } else {
                    OwnerPublicationState::Withdrawn
                },
                source_product_ref: source.clone(),
                hydration: hydration.clone(),
                provenance_refs: vec![source.clone()],
                qualifications: BTreeMap::from([("frame_type".into(), entry.frame_type)]),
            },
        );
        if active {
            objects
                .entry(frame.index_key())
                .or_insert_with(|| OwnerObjectPublication {
                    publication_id: format!("{}::{}", heads.source_id(), frame.index_key()),
                    object_ref: frame.clone(),
                    state: OwnerPublicationState::Observed,
                    source_product_ref: source.clone(),
                    hydration: HydrationReference {
                        product_kind: "frame".into(),
                        product_id: frame.object_id.clone(),
                        role: "selected_frame".into(),
                        ..hydration.clone()
                    },
                    provenance_refs: vec![source.clone()],
                    qualifications: BTreeMap::new(),
                });
            for (kind, dst) in [
                ("selected", frame),
                ("attached_to", node_ref(entry.node_id)),
            ] {
                relations.push(OwnerRelationOccurrence {
                    occurrence_id: format!("{}::{kind}", head.index_key()),
                    relation_type: kind.into(),
                    src: head.clone(),
                    dst,
                    source_product_ref: source.clone(),
                    hydration: hydration.clone(),
                    qualifications: BTreeMap::new(),
                    provenance_refs: vec![source.clone()],
                });
            }
        }
    }
    let objects = objects.into_values().collect::<Vec<_>>();
    let included_ids = objects
        .iter()
        .map(|object| object.publication_id.clone())
        .chain(
            relations
                .iter()
                .map(|relation| relation.occurrence_id.clone()),
        )
        .collect();
    OwnerPublicationOperation::reconstruct(
        "context-head-enumeration::v1",
        OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: "context".into(),
            revision_id: revision.clone(),
            scope: scope.clone(),
            objects,
            relations,
            completeness: OwnerCompletenessReceipt {
                receipt_id: format!("context-head-coverage::{revision}"),
                scope,
                included_ids,
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .map_err(StorageError::from)
}

/// Repeating publication after restart preserves the original operation identity.
pub fn publish_heads(
    append: &EventAppendCapability,
    heads: &HeadIndex,
    frames: &FrameStorage,
    session_id: &str,
) -> Result<(), ApiError> {
    let operation = head_publication(heads, frames).map_err(ApiError::from)?;
    append
        .append_durable(
            owner_publication_envelope(session_id, &operation).map_err(ApiError::from)?,
            AppendMode::Idempotent,
        )
        .map(|_| ())
        .map_err(|error| ApiError::from(StorageError::EventAuthorityUnavailable(error.to_string())))
}
