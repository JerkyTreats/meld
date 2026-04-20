use crate::error::StorageError;
use crate::events::{EventEnvelope, EventRecord, EventStore};
use crate::world_state::graph::contracts::{
    AnchorEndInput, AnchorSelectionInput, AnchorSelectionRecord, TraversalFactRecord,
    TraversalIntent,
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
    pub last_seen_seq: u64,
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
            last_seen_seq: after_seq,
        };
        for event in spine.read_all_events_after(after_seq)? {
            reducer.last_seen_seq = event.seq;
            if reducer.apply_event(store, &event)? {
                reducer.applied_events += 1;
            }
        }
        store.flush()?;
        Ok(reducer)
    }

    fn apply_event(
        &mut self,
        store: &TraversalStore,
        event: &EventRecord,
    ) -> Result<bool, StorageError> {
        if !is_traversal_source_event(event) {
            return Ok(false);
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

        for intent in reducer_intents_for_event(event, &source_fact_id)? {
            match intent {
                TraversalIntent::SelectAnchor(input) => {
                    self.select_anchor(store, event, input)?;
                }
                TraversalIntent::EndAnchor(input) => {
                    self.end_anchor(store, input)?;
                }
            }
        }

        Ok(true)
    }

    fn select_anchor(
        &mut self,
        store: &TraversalStore,
        event: &EventRecord,
        input: AnchorSelectionInput,
    ) -> Result<(), StorageError> {
        let AnchorSelectionInput {
            anchor_ref,
            subject,
            perspective,
            target,
            source_fact_id,
        } = input;
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
            let superseded_fact_id =
                format!("world_state::anchor_superseded::{}", current.anchor_id);
            current.ended_at_seq = Some(event.seq);
            current.ended_by_anchor_id = Some(anchor_id.clone());
            current.ended_by_fact_id = Some(superseded_fact_id.clone());
            store.put_anchor(&current)?;
            store.put_anchor_lineage(&current.anchor_id, &anchor_id)?;
            self.current_anchors.end(&anchor_ref.index_key(), event.seq);
            self.lineage
                .add_supersession(&current.anchor_id, anchor_id.clone(), event.seq);
            self.emitted_envelopes.push(anchor_superseded_envelope(
                &event.session,
                AnchorSupersededEventData {
                    fact_id: superseded_fact_id,
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
            ended_by_fact_id: None,
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

    fn end_anchor(
        &mut self,
        store: &TraversalStore,
        input: AnchorEndInput,
    ) -> Result<(), StorageError> {
        if let Some(current) = store.current_anchor(&input.anchor_ref)? {
            let mut ended = current.clone();
            ended.ended_at_seq = Some(input.ended_at_seq);
            store.put_anchor(&ended)?;
            store.clear_current_anchor(&ended.anchor_ref, &ended.subject, &ended.perspective)?;
            self.current_anchors
                .end(&ended.anchor_ref.index_key(), input.ended_at_seq);
        }
        Ok(())
    }
}

fn is_traversal_source_event(event: &EventRecord) -> bool {
    matches!(
        event.domain_id.as_str(),
        "workspace_fs" | "context" | "execution"
    )
}

fn reducer_intents_for_event(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    let mut intents = Vec::new();
    intents.extend(crate::workspace::reducer::graph_reducer_intents_for_event(
        event,
        source_fact_id,
    )?);
    intents.extend(crate::context::reducer::graph_reducer_intents_for_event(
        event,
        source_fact_id,
    )?);
    intents.extend(crate::task::reducer::graph_reducer_intents_for_event(
        event,
        source_fact_id,
    )?);
    Ok(intents)
}
