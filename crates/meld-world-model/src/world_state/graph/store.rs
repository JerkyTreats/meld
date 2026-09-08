//! Canonical owner-publication storage and projection positions.

use std::io;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use super::contracts::ProjectedOwnerPublication;
use crate::error::StorageError;
use crate::events::{LedgerCursor, LedgerIdentity};

const TREE_OWNER_PUBLICATIONS: &str = "traversal_owner_publications";
const TREE_RUNTIME_META: &str = "traversal_runtime_meta";
const KEY_LAST_REDUCED_SEQ: &str = "last_reduced_seq";
const KEY_AUTHORITY_CURSOR: &[u8] = b"event_authority_cursor";
const KEY_PENDING_DERIVED_EVENTS: &[u8] = b"pending_derived_events";
fn owner_event_route_key(owner: &str, event_type: &str) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(&(owner, event_type)).map_err(to_storage_data)?;
    Ok(format!(
        "owner-event-route::{}",
        blake3::hash(&bytes).to_hex()
    ))
}

const KEY_PAD: usize = 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PersistedGraphCursor {
    ledger_id: LedgerIdentity,
    after_seq: u64,
}

/// Sled-backed graph traversal store.
#[derive(Clone)]
pub struct TraversalStore {
    resource_id: String,
    db: Db,
    owner_publications: Tree,
    runtime_meta: Tree,
}

impl TraversalStore {
    /// Exact durable world-model resource bound to this store instance.
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Open all traversal trees against a shared sled database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            resource_id: crate::waiting::resource_identity(&db)?,
            owner_publications: db
                .open_tree(TREE_OWNER_PUBLICATIONS)
                .map_err(to_storage_io)?,
            runtime_meta: db.open_tree(TREE_RUNTIME_META).map_err(to_storage_io)?,
            db,
        })
    }

    /// Install one immutable owner Event admission contract in the existing Graph metadata.
    pub fn install_owner_event_route(
        &self,
        route: &super::admission::GraphOwnerEventRoute,
    ) -> Result<crate::belief::TheoryRevisionRef, StorageError> {
        let reference = route.revision_ref()?;
        let key = owner_event_route_key(&route.owner_id, &route.event_type)?;
        let bytes = serde_json::to_vec(route).map_err(to_storage_data)?;
        match self
            .runtime_meta
            .compare_and_swap(key, None as Option<&[u8]>, Some(bytes.as_slice()))
            .map_err(to_storage_io)?
        {
            Ok(()) => {
                self.db.flush().map_err(to_storage_io)?;
            }
            Err(conflict) if conflict.current.as_deref() == Some(bytes.as_slice()) => {}
            Err(_) => {
                return Err(StorageError::InvalidPath(
                    "owner Event kind already has a different Graph admission contract".into(),
                ))
            }
        }
        Ok(reference)
    }

    pub fn owner_event_route(
        &self,
        owner: &str,
        event_type: &str,
    ) -> Result<Option<super::admission::GraphOwnerEventRoute>, StorageError> {
        self.runtime_meta
            .get(owner_event_route_key(owner, event_type)?)
            .map_err(to_storage_io)?
            .map(|raw| {
                let route: super::admission::GraphOwnerEventRoute =
                    serde_json::from_slice(&raw).map_err(to_storage_data)?;
                route.validate()?;
                if route.owner_id != owner || route.event_type != event_type {
                    return Err(StorageError::InvalidPath(
                        "Graph owner Event route key disagrees with its body".into(),
                    ));
                }
                Ok(route)
            })
            .transpose()
    }

    pub fn owner_event_routes(
        &self,
    ) -> Result<Vec<super::admission::GraphOwnerEventRoute>, StorageError> {
        self.runtime_meta
            .scan_prefix(b"owner-event-route::")
            .map(|row| {
                let (_, raw) = row.map_err(to_storage_io)?;
                let route: super::admission::GraphOwnerEventRoute =
                    serde_json::from_slice(&raw).map_err(to_storage_data)?;
                route.validate()?;
                Ok(route)
            })
            .collect()
    }

    /// Record the immutable route set present before a verified replay from genesis.
    pub(crate) fn record_genesis_event_sources(
        &self,
        ledger_id: LedgerIdentity,
        routes: &[super::admission::OwnerEventSourceRef],
    ) -> Result<(), StorageError> {
        if self.authority_cursor(ledger_id)?.after_seq != 0 {
            return Err(StorageError::InvalidPath(
                "Event-source genesis coverage requires a zero Graph cursor".into(),
            ));
        }
        self.runtime_meta
            .insert(
                format!("event-source-genesis::{ledger_id}").as_bytes(),
                serde_json::to_vec(routes).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    /// Check exact installed owner authority and replay coverage, never just its namespace.
    pub fn covers_event_source(
        &self,
        ledger_id: LedgerIdentity,
        owner_id: &str,
        source: &super::admission::OwnerEventSourceRef,
    ) -> Result<bool, StorageError> {
        let installed = self.owner_event_routes()?.into_iter().any(|route| {
            route.complete_event_source
                && route.owner_id == owner_id
                && route
                    .source_ref()
                    .is_ok_and(|reference| &reference == source)
        });
        if !installed {
            return Ok(false);
        }
        let Some(raw) = self
            .runtime_meta
            .get(format!("event-source-genesis::{ledger_id}").as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(false);
        };
        let covered: Vec<super::admission::OwnerEventSourceRef> =
            serde_json::from_slice(&raw).map_err(to_storage_data)?;
        Ok(covered.contains(source))
    }

    /// Inspect each exhaustive route's native replay obligation without advancing it.
    pub fn owner_event_replay_states(
        &self,
        ledger_id: LedgerIdentity,
    ) -> Result<Vec<super::admission::OwnerEventReplayState>, StorageError> {
        let mut states = Vec::new();
        for route in self
            .owner_event_routes()?
            .into_iter()
            .filter(|route| route.complete_event_source)
        {
            let source = route.source_ref()?;
            let covered = self.covers_event_source(ledger_id, &route.owner_id, &source)?;
            states.push(super::admission::OwnerEventReplayState {
                covered,
                position: if covered {
                    self.authority_cursor(ledger_id)?
                } else {
                    self.owner_event_replay_cursor(ledger_id, &source)?
                },
                source,
            });
        }
        states.sort_by(|left, right| left.source.cmp(&right.source));
        Ok(states)
    }

    pub(crate) fn owner_event_replay_cursor(
        &self,
        ledger_id: LedgerIdentity,
        source: &super::admission::OwnerEventSourceRef,
    ) -> Result<LedgerCursor, StorageError> {
        let key = format!("owner-event-replay::{ledger_id}::{}", source.consumer_id());
        let Some(raw) = self
            .runtime_meta
            .get(key.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(LedgerCursor {
                ledger_id,
                after_seq: 0,
            });
        };
        let cursor: LedgerCursor = serde_json::from_slice(&raw).map_err(to_storage_data)?;
        if cursor.ledger_id != ledger_id {
            return Err(StorageError::IdentityMismatch {
                expected: ledger_id,
                actual: cursor.ledger_id,
            });
        }
        Ok(cursor)
    }

    pub(crate) fn advance_owner_event_replay(
        &self,
        source: &super::admission::OwnerEventSourceRef,
        prior: LedgerCursor,
        next: u64,
    ) -> Result<LedgerCursor, StorageError> {
        if self.owner_event_replay_cursor(prior.ledger_id, source)? != prior
            || next < prior.after_seq
            || next > self.authority_cursor(prior.ledger_id)?.after_seq
        {
            return Err(StorageError::InvalidPath(
                "owner Event replay cannot regress or pass the canonical projection".into(),
            ));
        }
        // Projection writes are durable before their separate resume position advances.
        self.flush()?;
        let cursor = LedgerCursor {
            ledger_id: prior.ledger_id,
            after_seq: next,
        };
        self.runtime_meta
            .insert(
                format!(
                    "owner-event-replay::{}::{}",
                    prior.ledger_id,
                    source.consumer_id()
                )
                .as_bytes(),
                serde_json::to_vec(&cursor).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(cursor)
    }

    pub(crate) fn complete_owner_event_replay(
        &self,
        source: &super::admission::OwnerEventSourceRef,
        cursor: LedgerCursor,
    ) -> Result<(), StorageError> {
        if self.owner_event_replay_cursor(cursor.ledger_id, source)? != cursor
            || cursor != self.authority_cursor(cursor.ledger_id)?
        {
            return Err(StorageError::InvalidPath(
                "owner Event coverage requires completed replay through the canonical projection"
                    .into(),
            ));
        }
        let key = format!("event-source-genesis::{}", cursor.ledger_id);
        loop {
            let prior = self
                .runtime_meta
                .get(key.as_bytes())
                .map_err(to_storage_io)?;
            let mut sources: Vec<super::admission::OwnerEventSourceRef> = prior
                .as_ref()
                .map(|raw| serde_json::from_slice(raw).map_err(to_storage_data))
                .transpose()?
                .unwrap_or_default();
            if sources.contains(source) {
                return Ok(());
            }
            sources.push(source.clone());
            sources.sort();
            let bytes = serde_json::to_vec(&sources).map_err(to_storage_data)?;
            if self
                .runtime_meta
                .compare_and_swap(key.as_bytes(), prior, Some(bytes))
                .map_err(to_storage_io)?
                .is_ok()
            {
                break;
            }
        }
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    /// Open the store behind an `Arc` for runtime assembly.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Return the shared sled database handle.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Persist one Graph-owned projection of an intact owner operation.
    pub fn put_owner_publication(
        &self,
        publication: &ProjectedOwnerPublication,
    ) -> Result<(), StorageError> {
        publication.operation.validate()?;
        let candidate = &publication.operation;
        for existing in self.owner_publications_through_seq(u64::MAX)? {
            let existing = existing.operation;
            if existing.batch.owner_id == candidate.batch.owner_id
                && existing.batch.scope == candidate.batch.scope
                && existing.batch.revision_id == candidate.batch.revision_id
                && existing.operation_id != candidate.operation_id
            {
                return Err(StorageError::InvalidPath(format!(
                    "owner revision '{}' has divergent publication operations",
                    candidate.batch.revision_id
                )));
            }
        }
        let key = encode_seq_index_key(
            publication.source_event.seq,
            &publication.operation.operation_id,
        );
        self.owner_publications
            .insert(
                key.as_bytes(),
                serde_json::to_vec(publication).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read the intact owner publication admitted from one Event record.
    pub fn owner_publication_for_event(
        &self,
        seq: u64,
    ) -> Result<Option<ProjectedOwnerPublication>, StorageError> {
        let prefix = format!("{seq:0KEY_PAD$}::");
        self.owner_publications
            .scan_prefix(prefix.as_bytes())
            .next()
            .map(|entry| {
                let (_, bytes) = entry.map_err(to_storage_io)?;
                serde_json::from_slice(&bytes).map_err(to_storage_data)
            })
            .transpose()
    }

    /// Read owner publications visible through one durable Graph position.
    pub fn owner_publications_through_seq(
        &self,
        through_seq: u64,
    ) -> Result<Vec<ProjectedOwnerPublication>, StorageError> {
        let mut publications = Vec::new();
        for item in self.owner_publications.iter() {
            let (key, value) = item.map_err(to_storage_io)?;
            let key = std::str::from_utf8(key.as_ref()).map_err(|error| {
                StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
            })?;
            let Some((seq, _)) = key.split_once("::") else {
                return Err(StorageError::InvalidPath(
                    "invalid owner publication key".to_string(),
                ));
            };
            let seq = seq.parse::<u64>().map_err(|error| {
                StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
            })?;
            if seq > through_seq {
                break;
            }
            let publication: ProjectedOwnerPublication =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            publication.operation.validate()?;
            if publication.source_event.seq != seq {
                return Err(StorageError::InvalidPath(
                    "owner publication key and source event disagree".to_string(),
                ));
            }
            publications.push(publication);
        }
        publications.sort_by(|left, right| {
            (left.source_event.seq, &left.operation.operation_id)
                .cmp(&(right.source_event.seq, &right.operation.operation_id))
        });
        Ok(publications)
    }

    /// Read the durable projection position without inventing an authority identity.
    pub fn projection_position(&self) -> Result<Option<LedgerCursor>, StorageError> {
        self.runtime_meta
            .get(KEY_AUTHORITY_CURSOR)
            .map_err(to_storage_io)?
            .map(|raw| {
                let persisted: PersistedGraphCursor =
                    serde_json::from_slice(&raw).map_err(to_storage_data)?;
                Ok(LedgerCursor {
                    ledger_id: persisted.ledger_id,
                    after_seq: persisted.after_seq,
                })
            })
            .transpose()
    }

    /// Read the canonical identity-bearing Graph projection position.
    pub fn authority_cursor(
        &self,
        expected_ledger_id: LedgerIdentity,
    ) -> Result<LedgerCursor, StorageError> {
        let Some(raw) = self
            .runtime_meta
            .get(KEY_AUTHORITY_CURSOR)
            .map_err(to_storage_io)?
        else {
            return Ok(LedgerCursor {
                ledger_id: expected_ledger_id,
                after_seq: 0,
            });
        };
        let persisted: PersistedGraphCursor =
            serde_json::from_slice(&raw).map_err(to_storage_data)?;
        if persisted.ledger_id != expected_ledger_id {
            return Err(StorageError::IdentityMismatch {
                expected: expected_ledger_id,
                actual: persisted.ledger_id,
            });
        }
        Ok(LedgerCursor {
            ledger_id: persisted.ledger_id,
            after_seq: persisted.after_seq,
        })
    }

    /// Clear projection data whose identifiers were derived from a legacy
    /// ledger sequence space before replaying a bound event authority.
    ///
    /// Cursor migration evidence and its recovery marker live in runtime
    /// metadata and are deliberately preserved. The legacy cursor, any
    /// identity-less derived-event retry state, and all sequence-derived
    /// projection indexes are rebuilt from authority sequence zero.
    pub(super) fn reset_for_event_authority_migration(&self) -> Result<(), StorageError> {
        self.owner_publications.clear().map_err(to_storage_io)?;
        self.runtime_meta
            .remove(KEY_LAST_REDUCED_SEQ.as_bytes())
            .map_err(to_storage_io)?;
        self.runtime_meta
            .remove(KEY_AUTHORITY_CURSOR)
            .map_err(to_storage_io)?;
        self.runtime_meta
            .remove(KEY_PENDING_DERIVED_EVENTS)
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    /// Flush the shared sled database.
    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }
}

fn encode_seq_index_key(seq: u64, operation_id: &str) -> String {
    format!("{seq:0KEY_PAD$}::{operation_id}")
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}
