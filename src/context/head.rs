use crate::context::events::head_selected_envelope;
use crate::context::frame::FrameStorage;
use crate::error::{ApiError, StorageError};
use crate::events::DomainObjectRef;
use crate::heads::HeadIndex;
use crate::telemetry::ProgressRuntime;
use crate::types::{FrameID, NodeID};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentFrameHead {
    pub node_id: NodeID,
    pub frame_type: String,
    pub frame_id: FrameID,
}

pub trait CurrentFrameHeadRead {
    fn current_frame_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, ApiError>;

    fn current_frame_heads_for_node(&self, node_id: &NodeID) -> Result<Vec<FrameID>, ApiError>;

    fn count_nodes_for_frame_type(&self, frame_type: &str) -> Result<usize, ApiError>;
}

impl CurrentFrameHeadRead for HeadIndex {
    fn current_frame_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, ApiError> {
        self.get_head(node_id, frame_type).map_err(ApiError::from)
    }

    fn current_frame_heads_for_node(&self, node_id: &NodeID) -> Result<Vec<FrameID>, ApiError> {
        Ok(self.get_all_heads_for_node(node_id))
    }

    fn count_nodes_for_frame_type(&self, frame_type: &str) -> Result<usize, ApiError> {
        Ok(self.count_nodes_for_frame_type(frame_type))
    }
}

pub fn decode_frame_anchor_target(target: &DomainObjectRef) -> Result<FrameID, StorageError> {
    if target.domain_id != "context" || target.object_kind != "frame" {
        return Err(StorageError::InvalidPath(format!(
            "expected context frame anchor target, got '{}'",
            target.index_key()
        )));
    }
    let decoded = hex::decode(&target.object_id).map_err(|err| {
        StorageError::InvalidPath(format!(
            "invalid frame anchor target '{}': {}",
            target.object_id, err
        ))
    })?;
    if decoded.len() != 32 {
        return Err(StorageError::InvalidPath(format!(
            "invalid frame anchor target '{}' length {}",
            target.object_id,
            decoded.len()
        )));
    }
    let mut frame_id = [0u8; 32];
    frame_id.copy_from_slice(&decoded);
    Ok(frame_id)
}

pub fn head_ref(node_id: NodeID, frame_type: &str) -> DomainObjectRef {
    DomainObjectRef::new("context", "head", head_object_id(node_id, frame_type))
        .expect("head ref should be valid")
}

pub fn node_ref(node_id: NodeID) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", hex::encode(node_id))
        .expect("node ref should be valid")
}

pub fn frame_ref(frame_id: FrameID) -> DomainObjectRef {
    DomainObjectRef::new("context", "frame", hex::encode(frame_id))
        .expect("frame ref should be valid")
}

fn head_object_id(node_id: NodeID, frame_type: &str) -> String {
    format!("{}::{}", hex::encode(node_id), frame_type)
}

pub fn backfill_legacy_heads_into_spine(
    runtime: &ProgressRuntime,
    head_index: &HeadIndex,
    frame_storage: &FrameStorage,
    session_id: &str,
) -> Result<usize, ApiError> {
    let mut emitted = 0usize;
    for entry in head_index.active_entries() {
        if frame_storage
            .get(&entry.frame_id)
            .map_err(ApiError::from)?
            .is_none()
        {
            tracing::warn!(
                node_id = %hex::encode(entry.node_id),
                frame_type = %entry.frame_type,
                frame_id = %hex::encode(entry.frame_id),
                "skipping legacy head backfill because frame blob is missing"
            );
            continue;
        }
        let record_id = format!(
            "context::head_backfill::{}::{}::{}",
            hex::encode(entry.node_id),
            entry.frame_type,
            hex::encode(entry.frame_id)
        );
        runtime.emit_envelope_idempotent(
            head_selected_envelope(
                session_id,
                entry.node_id,
                &entry.frame_type,
                entry.frame_id,
                None,
            )
            .with_record_id(record_id),
        )?;
        emitted += 1;
    }
    Ok(emitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_head_ref_is_stable_for_node_and_frame_type() {
        let node_id = [7u8; 32];
        let head_ref = head_ref(node_id, "analysis");
        assert_eq!(head_ref.domain_id, "context");
        assert_eq!(head_ref.object_kind, "head");
        assert_eq!(
            head_ref.object_id,
            format!("{}::analysis", hex::encode(node_id))
        );
    }
}
