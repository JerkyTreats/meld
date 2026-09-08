//! Neutral Event envelope construction for intact owner publications.

use super::contracts::{OwnerPublicationOperation, OWNER_PUBLICATION_EVENT_TYPE};
use crate::error::StorageError;
use crate::events::EventEnvelope;

/// Build the neutral Event envelope for one retryable owner publication.
pub fn owner_publication_envelope(
    session_id: &str,
    operation: &OwnerPublicationOperation,
) -> Result<EventEnvelope, StorageError> {
    operation.validate()?;
    let mut objects = operation
        .batch
        .objects
        .iter()
        .map(|publication| publication.object_ref.clone())
        .collect::<Vec<_>>();
    for occurrence in &operation.batch.relations {
        objects.push(occurrence.src.clone());
        objects.push(occurrence.dst.clone());
    }
    objects.sort();
    objects.dedup();
    let mut relations = operation
        .batch
        .relations
        .iter()
        .map(|occurrence| occurrence.event_relation())
        .collect::<Result<Vec<_>, _>>()?;
    relations.sort_by(|left, right| {
        (&left.relation_type, &left.src, &left.dst).cmp(&(
            &right.relation_type,
            &right.src,
            &right.dst,
        ))
    });
    relations.dedup();
    let data = serde_json::to_value(operation).map_err(|error| {
        StorageError::InvalidPath(format!("cannot encode owner publication: {error}"))
    })?;
    Ok(EventEnvelope::with_now_domain(
        session_id,
        operation.batch.owner_id.clone(),
        operation.batch.scope.scope_id.clone(),
        OWNER_PUBLICATION_EVENT_TYPE,
        None,
        data,
    )
    .with_record_id(operation.event_record_id())
    .with_graph(objects, relations))
}
