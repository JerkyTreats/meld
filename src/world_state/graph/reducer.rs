use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventEnvelope, EventRecord, EventStore};
use crate::world_state::graph::contracts::{
    AnchorSelectionRecord, PerspectiveKey, TraversalFactRecord,
};
use crate::world_state::graph::events::{
    anchor_selected_envelope, anchor_superseded_envelope, AnchorSelectedEventData,
    AnchorSupersededEventData,
};
use crate::world_state::graph::projection::{AnchorLineageProjection, CurrentAnchorProjection};
use crate::world_state::graph::store::TraversalStore;

pub struct TraversalReducer {
    pub current_anchors: CurrentAnchorProjection,
    pub lineage: AnchorLineageProjection,
    pub emitted_envelopes: Vec<EventEnvelope>,
    pub applied_events: usize,
}

impl TraversalReducer {
    pub fn replay_from_spine(
        spine: &EventStore,
        store: &TraversalStore,
        after_seq: u64,
    ) -> Result<Self, StorageError> {
        let mut reducer = Self {
            current_anchors: CurrentAnchorProjection::default(),
            lineage: AnchorLineageProjection::default(),
            emitted_envelopes: Vec::new(),
            applied_events: 0,
        };
        let mut last_reduced_seq = after_seq;
        for event in spine.read_all_events_after(after_seq)? {
            reducer.apply_event(store, &event)?;
            last_reduced_seq = event.seq;
            reducer.applied_events += 1;
        }
        store.set_last_reduced_seq(last_reduced_seq)?;
        store.flush()?;
        Ok(reducer)
    }

    fn apply_event(
        &mut self,
        store: &TraversalStore,
        event: &EventRecord,
    ) -> Result<(), StorageError> {
        if !is_traversal_relevant(event) {
            return Ok(());
        }

        let source_fact_id = format!("spine::{}", event.seq);
        let fact_id = format!("traversal::fact::{}", event.seq);
        store.put_fact(&TraversalFactRecord {
            fact_id: fact_id.clone(),
            source_spine_fact_id: source_fact_id.clone(),
            seq: event.seq,
            event_type: event.event_type.clone(),
            objects: event.objects.clone(),
            relations: event.relations.clone(),
        })?;

        match event.event_type.as_str() {
            "workspace_fs.snapshot_selected" => {
                let Some(source) = find_object_ref(&event.objects, "workspace_fs", "source") else {
                    return Ok(());
                };
                let Some(snapshot) = find_object_ref(&event.objects, "workspace_fs", "snapshot")
                else {
                    return Ok(());
                };
                let anchor_ref =
                    DomainObjectRef::new("workspace_fs", "snapshot_head", &source.object_id)?;
                let perspective = PerspectiveKey::new("snapshot", "current")?;
                self.select_anchor(
                    store,
                    event,
                    anchor_ref,
                    source,
                    perspective,
                    snapshot,
                    source_fact_id,
                )?;
            }
            "context.head_selected" => {
                let Some(head_ref) = find_object_ref(&event.objects, "context", "head") else {
                    return Ok(());
                };
                let Some(subject) = find_object_ref(&event.objects, "workspace_fs", "node") else {
                    return Ok(());
                };
                let Some(target) = find_object_ref(&event.objects, "context", "frame") else {
                    return Ok(());
                };
                let frame_type = head_ref
                    .object_id
                    .split_once("::")
                    .map(|(_, frame_type)| frame_type.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let perspective = PerspectiveKey::new("frame_type", frame_type)?;
                self.select_anchor(
                    store,
                    event,
                    head_ref,
                    subject,
                    perspective,
                    target,
                    source_fact_id,
                )?;
            }
            "context.head_tombstoned" => {
                let Some(head_ref) = find_object_ref(&event.objects, "context", "head") else {
                    return Ok(());
                };
                if let Some(current) = store.current_anchor(&head_ref)? {
                    let mut ended = current.clone();
                    ended.ended_at_seq = Some(event.seq);
                    store.put_anchor(&ended)?;
                    store.clear_current_anchor(
                        &ended.anchor_ref,
                        &ended.subject,
                        &ended.perspective,
                    )?;
                    self.current_anchors
                        .end(&ended.anchor_ref.index_key(), event.seq);
                }
            }
            "execution.task.artifact_emitted" => {
                let Some(task_run) = find_object_ref(&event.objects, "execution", "task_run")
                else {
                    return Ok(());
                };
                let Some(artifact) = find_object_ref(&event.objects, "execution", "artifact")
                else {
                    return Ok(());
                };
                let Some(slot_ref) = find_object_ref(&event.objects, "execution", "artifact_slot")
                else {
                    return Ok(());
                };
                let artifact_type_id = slot_ref
                    .object_id
                    .rsplit_once("::")
                    .map(|(_, artifact_type_id)| artifact_type_id.to_string())
                    .unwrap_or_else(|| "artifact".to_string());
                let perspective = PerspectiveKey::new("artifact_type", artifact_type_id)?;
                self.select_anchor(
                    store,
                    event,
                    slot_ref,
                    task_run,
                    perspective,
                    artifact,
                    source_fact_id,
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn select_anchor(
        &mut self,
        store: &TraversalStore,
        event: &EventRecord,
        anchor_ref: DomainObjectRef,
        subject: DomainObjectRef,
        perspective: PerspectiveKey,
        target: DomainObjectRef,
        source_fact_id: String,
    ) -> Result<(), StorageError> {
        let anchor_id = format!("anchor::{}::{}", anchor_ref.index_key(), event.seq);
        if let Some(existing) = store.get_anchor(&anchor_id)? {
            if let Some(current) = store.current_anchor(&anchor_ref)? {
                if current.anchor_id == anchor_id {
                    self.current_anchors.select(current);
                    return Ok(());
                }
                if current.selected_at_seq > event.seq {
                    return Ok(());
                }
            }
            if existing.ended_at_seq.is_none() {
                store.set_current_anchor(&existing)?;
                self.current_anchors.select(existing);
            } else if let Some(ended_at_seq) = existing.ended_at_seq {
                self.current_anchors
                    .end(&anchor_ref.index_key(), ended_at_seq);
            }
            return Ok(());
        }
        if let Some(mut current) = store.current_anchor(&anchor_ref)? {
            current.ended_at_seq = Some(event.seq);
            current.ended_by_anchor_id = Some(anchor_id.clone());
            store.put_anchor(&current)?;
            store.put_anchor_lineage(&current.anchor_id, &anchor_id)?;
            self.current_anchors.end(&anchor_ref.index_key(), event.seq);
            self.lineage
                .add_supersession(&current.anchor_id, anchor_id.clone(), event.seq);
            self.emitted_envelopes.push(anchor_superseded_envelope(
                &event.session,
                AnchorSupersededEventData {
                    fact_id: format!("world_state::anchor_superseded::{}", current.anchor_id),
                    anchor_id: current.anchor_id.clone(),
                    anchor_ref: anchor_ref.clone(),
                    superseded_by_anchor_id: anchor_id.clone(),
                    source_fact_id: source_fact_id.clone(),
                    seq: event.seq,
                },
            ));
        }

        let record = AnchorSelectionRecord {
            anchor_id: anchor_id.clone(),
            anchor_ref: anchor_ref.clone(),
            subject: subject.clone(),
            perspective: perspective.clone(),
            target: target.clone(),
            source_fact_ids: vec![source_fact_id.clone()],
            created_by_fact_id: format!("world_state::anchor_selected::{anchor_id}"),
            selected_at_seq: event.seq,
            ended_at_seq: None,
            ended_by_anchor_id: None,
        };
        store.put_anchor(&record)?;
        store.set_current_anchor(&record)?;
        self.current_anchors.select(record.clone());
        self.lineage
            .add_source_fact(&record.anchor_id, source_fact_id.clone(), event.seq);
        self.emitted_envelopes.push(anchor_selected_envelope(
            &event.session,
            AnchorSelectedEventData {
                fact_id: record.created_by_fact_id.clone(),
                anchor_id: record.anchor_id.clone(),
                anchor_ref,
                subject,
                perspective_kind: perspective.perspective_kind,
                perspective_id: perspective.perspective_id,
                target,
                source_fact_id,
                seq: event.seq,
            },
        ));
        Ok(())
    }
}

fn is_traversal_relevant(event: &EventRecord) -> bool {
    matches!(
        event.domain_id.as_str(),
        "workspace_fs" | "context" | "execution"
    )
}

fn find_object_ref(
    objects: &[DomainObjectRef],
    domain_id: &str,
    object_kind: &str,
) -> Option<DomainObjectRef> {
    objects
        .iter()
        .find(|object| object.domain_id == domain_id && object.object_kind == object_kind)
        .cloned()
}
