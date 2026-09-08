//! Admission of intact owner publications from the canonical Event history.

use super::contracts::{
    OwnerPublicationOperation, ProjectedOwnerPublication, OWNER_PUBLICATION_EVENT_TYPE,
};
use super::store::TraversalStore;
use crate::error::StorageError;
use crate::events::{EventRecord, EventRecordRef, LedgerIdentity};

/// Result of reducing a bounded Event page into the owner-publication projection.
pub struct TraversalReducer {
    pub applied_events: usize,
    pub last_seen_seq: u64,
}

impl TraversalReducer {
    pub fn replay_records(
        store: &TraversalStore,
        ledger_id: LedgerIdentity,
        after_seq: u64,
        events: impl IntoIterator<Item = EventRecord>,
    ) -> Result<Self, StorageError> {
        let mut reducer = Self {
            applied_events: 0,
            last_seen_seq: after_seq,
        };
        for event in events {
            reducer.last_seen_seq = event.seq;
            let route = store.owner_event_route(&event.domain_id, &event.event_type)?;
            if event.event_type == OWNER_PUBLICATION_EVENT_TYPE || route.is_some() {
                reducer.apply_owner_publication(store, ledger_id, &event, route.as_ref())?;
                reducer.applied_events += 1;
            }
        }
        store.flush()?;
        Ok(reducer)
    }

    fn apply_owner_publication(
        &mut self,
        store: &TraversalStore,
        ledger_id: LedgerIdentity,
        event: &EventRecord,
        route: Option<&super::admission::GraphOwnerEventRoute>,
    ) -> Result<(), StorageError> {
        let operation: OwnerPublicationOperation = serde_json::from_value(event.data.clone())
            .map_err(|error| {
                StorageError::InvalidPath(format!("invalid owner publication payload: {error}"))
            })?;
        operation.validate()?;
        if route.is_some_and(|route| {
            operation.enumeration_rule_revision != route.enumeration_rule_revision
        }) {
            return Err(StorageError::InvalidPath(
                "owner publication does not match its installed Graph enumeration contract".into(),
            ));
        }

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
            source_route: route.map(|route| route.source_ref()).transpose()?,
            operation,
            source_event: EventRecordRef {
                ledger_id,
                seq: event.seq,
            },
        })
    }
}
