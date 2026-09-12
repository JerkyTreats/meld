//! SQLite owns index transactions; Meld owns immutable revision and traversal meaning.

use super::contracts::*;
use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecordRef};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    connection: Arc<Mutex<Connection>>,
}
fn error(e: impl std::fmt::Display) -> StorageError {
    StorageError::Unavailable(format!("Graph SQLite index: {e}"))
}
fn encoded(v: &impl Serialize) -> Result<String, StorageError> {
    serde_json::to_string(v).map_err(error)
}
fn integer(v: u64) -> Result<i64, StorageError> {
    v.try_into().map_err(error)
}

impl ProjectionIndex {
    pub fn memory() -> Result<Self, StorageError> {
        Self::initialize(Connection::open_in_memory().map_err(error)?)
    }
    pub fn open(path: &std::path::Path) -> Result<Self, StorageError> {
        Self::initialize(Connection::open(path).map_err(error)?)
    }
    fn initialize(c: Connection) -> Result<Self, StorageError> {
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
            CREATE TABLE IF NOT EXISTS heads (seq INTEGER PRIMARY KEY, scope TEXT NOT NULL, source TEXT NOT NULL, route TEXT NOT NULL, body BLOB NOT NULL);
            CREATE INDEX IF NOT EXISTS heads_scope ON heads(scope, seq);
            CREATE TABLE IF NOT EXISTS revisions (key TEXT PRIMARY KEY, operation TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS objects (seq INTEGER NOT NULL, address TEXT NOT NULL, id TEXT NOT NULL, body BLOB NOT NULL, PRIMARY KEY(seq,address,id));
            CREATE TABLE IF NOT EXISTS relations (seq INTEGER NOT NULL, src TEXT NOT NULL, dst TEXT NOT NULL, id TEXT NOT NULL, body BLOB NOT NULL, PRIMARY KEY(seq,id));
            CREATE INDEX IF NOT EXISTS outgoing ON relations(seq,src);
            CREATE INDEX IF NOT EXISTS incoming ON relations(seq,dst);").map_err(error)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(c)),
        })
    }
    pub fn initialized(&self) -> Result<bool, StorageError> {
        self.connection
            .lock()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM meta WHERE key='ready')",
                [],
                |r| r.get(0),
            )
            .map_err(error)
    }
    pub fn finish_recovery(&self) -> Result<(), StorageError> {
        self.connection
            .lock()
            .execute("INSERT OR REPLACE INTO meta VALUES ('ready','1')", [])
            .map_err(error)?;
        Ok(())
    }
    pub fn check_revision(&self, p: &ProjectedOwnerPublication) -> Result<(), StorageError> {
        let b = &p.operation.batch;
        let old: Option<String> = self
            .connection
            .lock()
            .query_row(
                "SELECT operation FROM revisions WHERE key=?1",
                [encoded(&(&b.owner_id, &b.scope, &b.revision_id))?],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?;
        if old.is_some_and(|v| v != p.operation.operation_id) {
            return Err(StorageError::InvalidPath(
                "owner revision has divergent publication operations".into(),
            ));
        }
        Ok(())
    }
    #[tracing::instrument(target = "meld::trace", name = "graph.index_publish", skip_all)]
    pub fn publish(&self, p: &ProjectedOwnerPublication) -> Result<(), StorageError> {
        self.check_revision(p)?;
        let h = PublicationHead::from(p);
        let seq = integer(p.source_event.seq)?;
        let mut c = self.connection.lock();
        let tx = c.transaction().map_err(error)?;
        tx.execute(
            "INSERT OR REPLACE INTO heads VALUES (?1,?2,?3,?4,?5)",
            params![
                seq,
                encoded(&(&h.owner_id, &h.scope))?,
                encoded(&h.source_event)?,
                encoded(&h.source_route)?,
                serde_json::to_vec(&h).map_err(error)?
            ],
        )
        .map_err(error)?;
        tx.execute(
            "INSERT OR REPLACE INTO revisions VALUES (?1,?2)",
            params![
                encoded(&(&h.owner_id, &h.scope, &h.revision_id))?,
                h.operation_id
            ],
        )
        .map_err(error)?;
        {
            let mut objects = tx
                .prepare("INSERT OR REPLACE INTO objects VALUES (?1,?2,?3,?4)")
                .map_err(error)?;
            for o in &p.operation.batch.objects {
                objects
                    .execute(params![
                        seq,
                        encoded(&o.object_ref)?,
                        o.publication_id,
                        serde_json::to_vec(o).map_err(error)?
                    ])
                    .map_err(error)?;
            }
            let mut relations = tx
                .prepare("INSERT OR REPLACE INTO relations VALUES (?1,?2,?3,?4,?5)")
                .map_err(error)?;
            for r in &p.operation.batch.relations {
                relations
                    .execute(params![
                        seq,
                        encoded(&r.src)?,
                        encoded(&r.dst)?,
                        r.occurrence_id,
                        serde_json::to_vec(r).map_err(error)?
                    ])
                    .map_err(error)?;
            }
        }
        tx.commit().map_err(error)
    }
    #[tracing::instrument(target = "meld::trace", name = "graph.index_latest", skip_all)]
    pub fn latest(
        &self,
        r: &TraversalOwnerRequirement,
        seq: u64,
    ) -> Result<Option<PublicationHead>, StorageError> {
        let scope = encoded(&(&r.owner_id, &r.scope))?;
        let c = self.connection.lock();
        let raw: Option<Vec<u8>> = if r.event_source.is_some() {
            c.query_row("SELECT body FROM heads WHERE scope=?1 AND seq<=?2 AND route=?3 ORDER BY seq DESC LIMIT 1",params![scope,integer(seq)?,encoded(&r.event_source)?], |r| r.get(0)).optional().map_err(error)?
        } else {
            c.query_row(
                "SELECT body FROM heads WHERE scope=?1 AND seq<=?2 ORDER BY seq DESC LIMIT 1",
                params![scope, integer(seq)?],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?
        };
        raw.map(|v| serde_json::from_slice(&v).map_err(error))
            .transpose()
    }
    pub fn selected(&self, cut: &TraversalCut) -> Result<Vec<u64>, StorageError> {
        let c = self.connection.lock();
        let mut selected = Vec::new();
        for r in &cut.receipts {
            if let Some(source) = r
                .source_event
                .filter(|e| e.seq <= cut.graph_position.after_seq)
            {
                let stored: Option<String> = c
                    .query_row(
                        "SELECT source FROM heads WHERE seq=?1",
                        [integer(source.seq)?],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(error)?;
                if stored.as_deref() == Some(encoded(&source)?.as_str()) {
                    selected.push(source.seq);
                }
            }
        }
        selected.sort_unstable();
        selected.dedup();
        Ok(selected)
    }
    fn read<T: serde::de::DeserializeOwned>(
        &self,
        sql: &str,
        seqs: &[u64],
        object: &DomainObjectRef,
    ) -> Result<Vec<T>, StorageError> {
        let mut decoded_bytes = 0u64;
        let c = self.connection.lock();
        let mut stmt = c.prepare_cached(sql).map_err(error)?;
        let mut result = Vec::new();
        for seq in seqs {
            let mut rows = stmt
                .query(params![integer(*seq)?, encoded(object)?])
                .map_err(error)?;
            while let Some(row) = rows.next().map_err(error)? {
                let raw: Vec<u8> = row.get(0).map_err(error)?;
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
        self.read(
            "SELECT body FROM objects WHERE seq=?1 AND address=?2 ORDER BY id",
            seqs,
            object,
        )
    }
    #[tracing::instrument(target = "meld::trace", name = "graph.index_relations", skip_all, fields(decoded_bytes = tracing::field::Empty, decoded_rows = tracing::field::Empty))]
    pub fn relations(
        &self,
        seqs: &[u64],
        object: &DomainObjectRef,
        outgoing: bool,
    ) -> Result<Vec<OwnerRelationOccurrence>, StorageError> {
        self.read(
            if outgoing {
                "SELECT body FROM relations WHERE seq=?1 AND src=?2 ORDER BY id"
            } else {
                "SELECT body FROM relations WHERE seq=?1 AND dst=?2 ORDER BY id"
            },
            seqs,
            object,
        )
    }
}
