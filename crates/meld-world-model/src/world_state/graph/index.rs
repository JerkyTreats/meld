//! Derived immutable revision and adjacency rows. The Event projection owns writes.

use super::contracts::*;
use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecordRef};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct PublicationHead {
    pub operation_id: String,
    pub owner_id: String,
    pub revision_id: String,
    pub scope: OwnerPublicationScope,
    pub completeness: OwnerCompletenessReceipt,
    pub work_input_basis_id: Option<String>,
    pub source_event: EventRecordRef,
    pub source_route: Option<super::admission::OwnerEventSourceRef>,
}

impl From<&ProjectedOwnerPublication> for PublicationHead {
    fn from(p: &ProjectedOwnerPublication) -> Self {
        let b = &p.operation.batch;
        Self {
            operation_id: p.operation.operation_id.clone(),
            owner_id: b.owner_id.clone(),
            revision_id: b.revision_id.clone(),
            scope: b.scope.clone(),
            completeness: b.completeness.clone(),
            work_input_basis_id: b.work_input_basis_id.clone(),
            source_event: p.source_event,
            source_route: p.source_route.clone(),
        }
    }
}

#[derive(Clone)]
pub(super) struct ProjectionIndex {
    rows: sled::Tree,
}

pub(super) fn error(e: impl std::fmt::Display) -> StorageError {
    StorageError::Unavailable(format!("Graph projection index: {e}"))
}

fn identity(v: &impl Serialize) -> Result<String, StorageError> {
    Ok(blake3::hash(&serde_json::to_vec(v).map_err(error)?)
        .to_hex()
        .to_string())
}

impl ProjectionIndex {
    pub fn new(db: &sled::Db) -> Result<Self, StorageError> {
        Ok(Self {
            rows: db.open_tree("graph_read_index_spike_v1").map_err(error)?,
        })
    }
    pub fn initialized(&self) -> Result<bool, StorageError> {
        self.rows.contains_key("ready").map_err(error)
    }
    pub fn finish_recovery(&self) -> Result<(), StorageError> {
        self.rows.insert("ready", b"1").map_err(error)?;
        Ok(())
    }

    pub fn check_revision(&self, p: &ProjectedOwnerPublication) -> Result<(), StorageError> {
        let b = &p.operation.batch;
        let key = format!("r/{}", identity(&(&b.owner_id, &b.scope, &b.revision_id))?);
        if let Some(old) = self.rows.get(key).map_err(error)? {
            if old.as_ref() != p.operation.operation_id.as_bytes() {
                return Err(StorageError::InvalidPath(
                    "owner revision has divergent publication operations".into(),
                ));
            }
        }
        Ok(())
    }

    #[tracing::instrument(target = "meld::trace", name = "graph.index_publish", skip_all)]
    pub fn publish(&self, p: &ProjectedOwnerPublication) -> Result<(), StorageError> {
        self.check_revision(p)?;
        let h = PublicationHead::from(p);
        let seq = p.source_event.seq;
        let scope = identity(&(&h.owner_id, &h.scope))?;
        let mut batch = sled::Batch::default();
        batch.insert(
            format!("h/{scope}/{seq:020}").as_bytes(),
            serde_json::to_vec(&h).map_err(error)?,
        );
        batch.insert(
            format!("e/{seq:020}").as_bytes(),
            serde_json::to_vec(&p.source_event).map_err(error)?,
        );
        batch.insert(
            format!("r/{}", identity(&(&h.owner_id, &h.scope, &h.revision_id))?).as_bytes(),
            h.operation_id.as_bytes(),
        );
        for o in &p.operation.batch.objects {
            batch.insert(
                format!(
                    "o/{seq:020}/{}/{}/",
                    identity(&o.object_ref)?,
                    o.publication_id
                )
                .as_bytes(),
                serde_json::to_vec(o).map_err(error)?,
            );
        }
        for r in &p.operation.batch.relations {
            let raw = serde_json::to_vec(r).map_err(error)?;
            batch.insert(
                format!("f/{seq:020}/{}/{}/", identity(&r.src)?, r.occurrence_id).as_bytes(),
                raw.clone(),
            );
            batch.insert(
                format!("b/{seq:020}/{}/{}/", identity(&r.dst)?, r.occurrence_id).as_bytes(),
                raw,
            );
        }
        self.rows.apply_batch(batch).map_err(error)
    }

    #[tracing::instrument(target = "meld::trace", name = "graph.index_latest", skip_all)]
    pub fn latest(
        &self,
        r: &TraversalOwnerRequirement,
        seq: u64,
    ) -> Result<Option<PublicationHead>, StorageError> {
        let prefix = format!("h/{}/", identity(&(&r.owner_id, &r.scope))?);
        let end = format!("{prefix}{seq:020}");
        for item in self.rows.range(prefix.as_bytes()..=end.as_bytes()).rev() {
            let (_, raw) = item.map_err(error)?;
            let h: PublicationHead = serde_json::from_slice(&raw).map_err(error)?;
            if r.event_source
                .as_ref()
                .is_none_or(|s| h.source_route.as_ref() == Some(s))
            {
                return Ok(Some(h));
            }
        }
        Ok(None)
    }

    pub fn selected(&self, cut: &TraversalCut) -> Result<Vec<u64>, StorageError> {
        let mut selected = Vec::new();
        for r in &cut.receipts {
            if let Some(source) = r
                .source_event
                .filter(|e| e.seq <= cut.graph_position.after_seq)
            {
                if let Some(raw) = self
                    .rows
                    .get(format!("e/{:020}", source.seq))
                    .map_err(error)?
                {
                    let stored: EventRecordRef = serde_json::from_slice(&raw).map_err(error)?;
                    if stored == source {
                        selected.push(source.seq);
                    }
                }
            }
        }
        selected.sort_unstable();
        selected.dedup();
        Ok(selected)
    }

    fn read<T: serde::de::DeserializeOwned>(
        &self,
        kind: &str,
        seqs: &[u64],
        object: &DomainObjectRef,
    ) -> Result<Vec<T>, StorageError> {
        let mut decoded_bytes = 0u64;
        let address = identity(object)?;
        let mut result = Vec::new();
        for seq in seqs {
            for item in self
                .rows
                .scan_prefix(format!("{kind}/{seq:020}/{address}/"))
            {
                let (_, raw) = item.map_err(error)?;
                decoded_bytes += raw.len() as u64;
                result.push(serde_json::from_slice(&raw).map_err(error)?);
            }
        }
        tracing::Span::current().record("decoded_bytes", decoded_bytes);
        tracing::Span::current().record("decoded_rows", result.len() as u64);
        Ok(result)
    }
    #[tracing::instrument(target = "meld::trace", name = "graph.index_objects", skip_all, fields(decoded_bytes = tracing::field::Empty, decoded_rows = tracing::field::Empty))]
    pub fn objects(
        &self,
        seqs: &[u64],
        object: &DomainObjectRef,
    ) -> Result<Vec<OwnerObjectPublication>, StorageError> {
        self.read("o", seqs, object)
    }
    #[tracing::instrument(target = "meld::trace", name = "graph.index_relations", skip_all, fields(decoded_bytes = tracing::field::Empty, decoded_rows = tracing::field::Empty))]
    pub fn relations(
        &self,
        seqs: &[u64],
        object: &DomainObjectRef,
        outgoing: bool,
    ) -> Result<Vec<OwnerRelationOccurrence>, StorageError> {
        self.read(if outgoing { "f" } else { "b" }, seqs, object)
    }
}
