//! Event-ledger reducer for graph traversal state.
//!
//! The reducer consumes source events, stores graph-readable facts, derives
//! anchor intents, and updates current-anchor indexes. It emits traversal events
//! only when source events imply anchor changes.
//!
//! Runtime replay is driven by the identity-bearing graph replay port. Frozen
//! raw-ledger characterization lives under `graph::test_support`.

use crate::error::StorageError;
use crate::events::{EventEnvelope, EventRecord, EventRecordRef, LedgerIdentity};
use crate::world_state::graph::contracts::{
    AnchorEndInput, AnchorSelectionInput, AnchorSelectionRecord, OwnerPublicationOperation,
    ProjectedOwnerPublication, TraversalFactRecord, TraversalIntent, OWNER_PUBLICATION_EVENT_TYPE,
};
use crate::world_state::graph::events::{
    anchor_selected_envelope_from_record, anchor_superseded_envelope_from_record,
    AnchorSelectedEventData, AnchorSupersededEventData,
};
use crate::world_state::graph::projection::{AnchorLineageProjection, CurrentAnchorProjection};
use crate::world_state::graph::store::TraversalStore;

/// In-memory reducer state produced while replaying graph source events.
pub struct TraversalReducer {
    /// Current anchors observed during this replay pass.
    pub current_anchors: CurrentAnchorProjection,
    /// Anchor lineage observed during this replay pass.
    pub lineage: AnchorLineageProjection,
    /// Derived traversal events to append to the ledger after replay.
    pub emitted_envelopes: Vec<EventEnvelope>,
    /// Number of source events applied by this replay pass.
    pub applied_events: usize,
    /// Highest source sequence seen by this replay pass.
    pub last_seen_seq: u64,
}

impl TraversalReducer {
    /// Replay a caller-selected bounded event set into traversal storage.
    pub fn replay_records(
        store: &TraversalStore,
        ledger_id: LedgerIdentity,
        after_seq: u64,
        events: impl IntoIterator<Item = EventRecord>,
    ) -> Result<Self, StorageError> {
        let mut reducer = Self {
            current_anchors: CurrentAnchorProjection::default(),
            lineage: AnchorLineageProjection::default(),
            emitted_envelopes: Vec::new(),
            applied_events: 0,
            last_seen_seq: after_seq,
        };
        for event in events {
            reducer.last_seen_seq = event.seq;
            if reducer.apply_event(store, ledger_id, &event)? {
                reducer.applied_events += 1;
            }
        }
        store.flush()?;
        Ok(reducer)
    }

    fn apply_event(
        &mut self,
        store: &TraversalStore,
        ledger_id: LedgerIdentity,
        event: &EventRecord,
    ) -> Result<bool, StorageError> {
        if !is_traversal_source_event(event) {
            return Ok(false);
        }

        if event.event_type == OWNER_PUBLICATION_EVENT_TYPE {
            self.apply_owner_publication(store, ledger_id, event)?;
            return Ok(true);
        }

        // The spine:: prefix is a frozen stored-identifier format: existing
        // traversal facts reference it, so it survives the ledger renaming.
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
                    self.select_anchor(store, ledger_id, event, input)?;
                }
                TraversalIntent::EndAnchor(input) => {
                    self.end_anchor(store, input)?;
                }
            }
        }

        Ok(true)
    }

    fn apply_owner_publication(
        &mut self,
        store: &TraversalStore,
        ledger_id: LedgerIdentity,
        event: &EventRecord,
    ) -> Result<(), StorageError> {
        let operation: OwnerPublicationOperation = serde_json::from_value(event.data.clone())
            .map_err(|error| {
                StorageError::InvalidPath(format!("invalid owner publication payload: {error}"))
            })?;
        operation.validate()?;
        if event.domain_id != operation.batch.owner_id {
            return Err(StorageError::InvalidPath(
                "owner publication domain does not own its payload".to_string(),
            ));
        }
        if event.record_id.as_deref() != Some(operation.event_record_id().as_str()) {
            return Err(StorageError::InvalidPath(
                "owner publication record identity does not match its payload".to_string(),
            ));
        }
        let expected = crate::world_state::graph::events::owner_publication_envelope(
            &event.session,
            &operation,
        )?;
        if event.objects != expected.objects || event.relations != expected.relations {
            return Err(StorageError::InvalidPath(
                "owner publication Event hints do not match the typed payload".to_string(),
            ));
        }
        store.put_owner_publication(&ProjectedOwnerPublication {
            operation,
            source_event: EventRecordRef {
                ledger_id,
                seq: event.seq,
            },
        })
    }

    fn select_anchor(
        &mut self,
        store: &TraversalStore,
        ledger_id: LedgerIdentity,
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
            // Re-emit deterministic derived facts when replay resumes after a
            // crash that persisted projection writes before publication. The
            // authority sink is idempotent, so already published facts remain
            // duplicates while a missing fact is recovered before the cursor
            // advances.
            for predecessor in store.anchor_history(&anchor_ref)? {
                if predecessor.ended_at_seq == Some(event.seq)
                    && predecessor.ended_by_anchor_id.as_deref() == Some(anchor_id.as_str())
                {
                    self.emitted_envelopes
                        .push(anchor_superseded_envelope_from_record(
                            &event.session,
                            EventRecordRef {
                                ledger_id,
                                seq: event.seq,
                            },
                            AnchorSupersededEventData {
                                anchor: predecessor,
                            },
                        ));
                }
            }
            self.emitted_envelopes
                .push(anchor_selected_envelope_from_record(
                    &event.session,
                    EventRecordRef {
                        ledger_id,
                        seq: event.seq,
                    },
                    AnchorSelectedEventData {
                        anchor: existing.clone(),
                    },
                ));
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
            self.emitted_envelopes
                .push(anchor_superseded_envelope_from_record(
                    &event.session,
                    EventRecordRef {
                        ledger_id,
                        seq: event.seq,
                    },
                    AnchorSupersededEventData {
                        anchor: current.clone(),
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
        self.emitted_envelopes
            .push(anchor_selected_envelope_from_record(
                &event.session,
                EventRecordRef {
                    ledger_id,
                    seq: event.seq,
                },
                AnchorSelectedEventData {
                    anchor: record.clone(),
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
    event.event_type == OWNER_PUBLICATION_EVENT_TYPE
        || event.event_type == "workspace_fs.snapshot_selected"
        || matches!(event.domain_id.as_str(), "context" | "execution")
}

fn reducer_intents_for_event(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    crate::world_state::graph::source_intent::traversal_intents_for_event(event, source_fact_id)
}
