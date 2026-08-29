//! Durable graph traversal storage.
//!
//! The traversal store keeps graph facts, object membership, relation indexes,
//! anchor history, current-anchor indexes, and reducer cursors. Writes are
//! index-building operations over durable facts; query modules provide the
//! public read surface.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::world_state::graph::store::TraversalStore;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let store = TraversalStore::new(sled::open(temp.path()).unwrap()).unwrap();
//! assert_eq!(store.last_reduced_seq().unwrap(), 0);
//! ```

use std::collections::{BTreeSet, VecDeque};
use std::io;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRelation, LedgerCursor, LedgerIdentity};
use crate::world_state::graph::contracts::{
    AnchorProvenanceRecord, AnchorSelectionRecord, GraphWalkResult, GraphWalkSpec,
    ProjectedOwnerPublication, TraversalDirection, TraversalFactRecord,
};

const TREE_FACTS: &str = "traversal_facts";
const TREE_OWNER_PUBLICATIONS: &str = "traversal_owner_publications";
const TREE_FACT_OBJECTS: &str = "traversal_fact_objects";
const TREE_OBJECT_FACTS: &str = "traversal_object_facts";
const TREE_OUTGOING_RELATIONS: &str = "traversal_outgoing_relations";
const TREE_INCOMING_RELATIONS: &str = "traversal_incoming_relations";
const TREE_ANCHORS: &str = "traversal_anchor_records";
const TREE_CURRENT_ANCHOR: &str = "traversal_current_anchor";
const TREE_ANCHOR_HISTORY: &str = "traversal_anchor_history";
const TREE_ANCHOR_LINEAGE: &str = "traversal_anchor_lineage";
const TREE_SOURCE_FACT_INDEX: &str = "traversal_source_fact_index";
const TREE_SEQ_INDEX: &str = "traversal_seq_index";
const TREE_SUBJECT_PERSPECTIVE: &str = "traversal_subject_perspective_index";
const TREE_RUNTIME_META: &str = "traversal_runtime_meta";
const KEY_LAST_REDUCED_SEQ: &str = "last_reduced_seq";
const KEY_AUTHORITY_CURSOR: &[u8] = b"event_authority_cursor";
const KEY_PENDING_DERIVED_EVENTS: &[u8] = b"pending_derived_events";
const KEY_PAD: usize = 20;

/// Stored relation edge plus the fact that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationRecord {
    pub relation: EventRelation,
    pub fact_id: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PersistedGraphCursor {
    ledger_id: LedgerIdentity,
    after_seq: u64,
}

/// Sled-backed graph traversal store.
#[derive(Clone)]
pub struct TraversalStore {
    db: Db,
    facts: Tree,
    owner_publications: Tree,
    fact_objects: Tree,
    object_facts: Tree,
    outgoing_relations: Tree,
    incoming_relations: Tree,
    anchors: Tree,
    current_anchor: Tree,
    anchor_history: Tree,
    anchor_lineage: Tree,
    source_fact_index: Tree,
    seq_index: Tree,
    subject_perspective_index: Tree,
    runtime_meta: Tree,
}

impl TraversalStore {
    /// Open all traversal trees against a shared sled database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            facts: db.open_tree(TREE_FACTS).map_err(to_storage_io)?,
            owner_publications: db
                .open_tree(TREE_OWNER_PUBLICATIONS)
                .map_err(to_storage_io)?,
            fact_objects: db.open_tree(TREE_FACT_OBJECTS).map_err(to_storage_io)?,
            object_facts: db.open_tree(TREE_OBJECT_FACTS).map_err(to_storage_io)?,
            outgoing_relations: db
                .open_tree(TREE_OUTGOING_RELATIONS)
                .map_err(to_storage_io)?,
            incoming_relations: db
                .open_tree(TREE_INCOMING_RELATIONS)
                .map_err(to_storage_io)?,
            anchors: db.open_tree(TREE_ANCHORS).map_err(to_storage_io)?,
            current_anchor: db.open_tree(TREE_CURRENT_ANCHOR).map_err(to_storage_io)?,
            anchor_history: db.open_tree(TREE_ANCHOR_HISTORY).map_err(to_storage_io)?,
            anchor_lineage: db.open_tree(TREE_ANCHOR_LINEAGE).map_err(to_storage_io)?,
            source_fact_index: db
                .open_tree(TREE_SOURCE_FACT_INDEX)
                .map_err(to_storage_io)?,
            seq_index: db.open_tree(TREE_SEQ_INDEX).map_err(to_storage_io)?,
            subject_perspective_index: db
                .open_tree(TREE_SUBJECT_PERSPECTIVE)
                .map_err(to_storage_io)?,
            runtime_meta: db.open_tree(TREE_RUNTIME_META).map_err(to_storage_io)?,
            db,
        })
    }

    /// Open the store behind an `Arc` for runtime assembly.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Return the shared sled database handle.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Store a graph-readable fact and update object and relation indexes.
    pub fn put_fact(&self, fact: &TraversalFactRecord) -> Result<(), StorageError> {
        self.facts
            .insert(
                fact.fact_id.as_bytes(),
                serde_json::to_vec(fact).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.seq_index
            .insert(
                encode_seq_index_key(fact.seq, &fact.fact_id).as_bytes(),
                fact.fact_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        self.source_fact_index
            .insert(
                encode_membership_key(&fact.source_spine_fact_id, &fact.fact_id).as_bytes(),
                fact.fact_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        for object in &fact.objects {
            let object_key = object.index_key();
            self.fact_objects
                .insert(
                    encode_membership_key(&fact.fact_id, &object_key).as_bytes(),
                    object_key.as_bytes(),
                )
                .map_err(to_storage_io)?;
            self.object_facts
                .insert(
                    encode_membership_key(&object_key, &fact.fact_id).as_bytes(),
                    fact.fact_id.as_bytes(),
                )
                .map_err(to_storage_io)?;
        }
        for relation in &fact.relations {
            let record = RelationRecord {
                relation: relation.clone(),
                fact_id: fact.fact_id.clone(),
                seq: fact.seq,
            };
            let outgoing_key = encode_relation_key(
                &relation.src.index_key(),
                &relation.relation_type,
                &relation.dst.index_key(),
                fact.seq,
                &fact.fact_id,
            );
            let incoming_key = encode_relation_key(
                &relation.dst.index_key(),
                &relation.relation_type,
                &relation.src.index_key(),
                fact.seq,
                &fact.fact_id,
            );
            let encoded = serde_json::to_vec(&record).map_err(to_storage_data)?;
            self.outgoing_relations
                .insert(outgoing_key.as_bytes(), encoded.clone())
                .map_err(to_storage_io)?;
            self.incoming_relations
                .insert(incoming_key.as_bytes(), encoded)
                .map_err(to_storage_io)?;
        }
        Ok(())
    }

    /// Read one graph-readable fact by id.
    pub fn get_fact(&self, fact_id: &str) -> Result<Option<TraversalFactRecord>, StorageError> {
        let Some(raw) = self.facts.get(fact_id.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
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
            let seq = seq.parse::<u64>().map_err(to_storage_parse)?;
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

    /// Read facts mentioning an object after a source sequence cursor.
    pub fn facts_for_object(
        &self,
        object: &DomainObjectRef,
        after_seq: u64,
    ) -> Result<Vec<TraversalFactRecord>, StorageError> {
        let mut out = Vec::new();
        let prefix = format!("{}::", object.index_key());
        for item in self.object_facts.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let fact_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(fact) = self.get_fact(&fact_id)? {
                if fact.seq > after_seq {
                    out.push(fact);
                }
            }
        }
        out.sort_by_key(|fact| fact.seq);
        Ok(out)
    }

    /// Store an anchor record and update history indexes.
    pub fn put_anchor(&self, anchor: &AnchorSelectionRecord) -> Result<(), StorageError> {
        self.anchors
            .insert(
                anchor.anchor_id.as_bytes(),
                serde_json::to_vec(anchor).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        let anchor_ref_key = anchor.anchor_ref.index_key();
        self.anchor_history
            .insert(
                encode_membership_key(&anchor_ref_key, &anchor.anchor_id).as_bytes(),
                anchor.anchor_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        self.subject_perspective_index
            .insert(
                encode_subject_perspective_key(&anchor.subject, &anchor.perspective).as_bytes(),
                anchor.anchor_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read one anchor record by id.
    pub fn get_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        let Some(raw) = self
            .anchors
            .get(anchor_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
    }

    /// Move the current-anchor index to an existing anchor record.
    pub fn set_current_anchor(&self, anchor: &AnchorSelectionRecord) -> Result<(), StorageError> {
        self.current_anchor
            .insert(
                anchor.anchor_ref.index_key().as_bytes(),
                anchor.anchor_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        self.subject_perspective_index
            .insert(
                encode_subject_perspective_key(&anchor.subject, &anchor.perspective).as_bytes(),
                anchor.anchor_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Clear the current-anchor index for a subject and perspective.
    pub fn clear_current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
        subject: &DomainObjectRef,
        perspective: &crate::world_state::graph::contracts::PerspectiveKey,
    ) -> Result<(), StorageError> {
        self.current_anchor
            .remove(anchor_ref.index_key().as_bytes())
            .map_err(to_storage_io)?;
        self.subject_perspective_index
            .remove(encode_subject_perspective_key(subject, perspective).as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read the current anchor for a logical anchor reference.
    pub fn current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        let Some(raw) = self
            .current_anchor
            .get(anchor_ref.index_key().as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let anchor_id = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        self.get_anchor(&anchor_id)
    }

    /// Read the current anchor for one subject and perspective.
    pub fn current_anchor_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        let key = format!(
            "{}::{}::{}",
            subject.index_key(),
            perspective_kind,
            perspective_id
        );
        let Some(raw) = self
            .subject_perspective_index
            .get(key.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let anchor_id = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        self.get_anchor(&anchor_id)
    }

    /// Read current anchors for every perspective on one subject.
    pub fn current_anchors_for_subject(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        let mut out = Vec::new();
        let prefix = format!("{}::", subject.index_key());
        for item in self
            .subject_perspective_index
            .scan_prefix(prefix.as_bytes())
        {
            let (_, value) = item.map_err(to_storage_io)?;
            let anchor_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(record) = self.get_anchor(&anchor_id)? {
                out.push(record);
            }
        }
        out.sort_by_key(|record| record.selected_at_seq);
        Ok(out)
    }

    /// Read current anchors for one perspective across subjects.
    pub fn current_anchors_by_perspective(
        &self,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        // Demotion-phase scan. A perspective-first index can replace this if
        // coverage and status reads become hot.
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for item in self.subject_perspective_index.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let anchor_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if !seen.insert(anchor_id.clone()) {
                continue;
            }
            if let Some(record) = self.get_anchor(&anchor_id)? {
                if record.perspective.perspective_kind == perspective_kind
                    && record.perspective.perspective_id == perspective_id
                    && record.ended_at_seq.is_none()
                {
                    out.push(record);
                }
            }
        }
        out.sort_by_key(|record| record.selected_at_seq);
        Ok(out)
    }

    /// Count current anchors for one perspective.
    pub fn current_anchor_count_by_perspective(
        &self,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<usize, StorageError> {
        Ok(self
            .current_anchors_by_perspective(perspective_kind, perspective_id)?
            .len())
    }

    /// Read all anchors ever selected for one logical anchor reference.
    pub fn anchor_history(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        let mut out = Vec::new();
        let prefix = format!("{}::", anchor_ref.index_key());
        for item in self.anchor_history.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let anchor_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(record) = self.get_anchor(&anchor_id)? {
                out.push(record);
            }
        }
        out.sort_by_key(|record| record.selected_at_seq);
        Ok(out)
    }

    /// Record that one anchor was superseded by another anchor.
    pub fn put_anchor_lineage(
        &self,
        anchor_id: &str,
        superseded_by_anchor_id: &str,
    ) -> Result<(), StorageError> {
        self.anchor_lineage
            .insert(
                encode_membership_key(anchor_id, superseded_by_anchor_id).as_bytes(),
                superseded_by_anchor_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Build compact provenance for one anchor from its source facts.
    pub fn anchor_provenance(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError> {
        let anchor = self
            .get_anchor(anchor_id)?
            .ok_or_else(|| StorageError::InvalidPath(format!("unknown anchor '{}'", anchor_id)))?;
        let mut objects = BTreeSet::new();
        let mut relations = Vec::new();
        for source_fact_id in &anchor.source_fact_ids {
            if let Some(fact) = self.fact_for_source_spine_fact(source_fact_id)? {
                for object in fact.objects {
                    objects.insert(object);
                }
                relations.extend(fact.relations);
            }
        }
        let derived_fact_ids = derived_fact_ids_for_anchor(&anchor);
        Ok(AnchorProvenanceRecord {
            anchor_id: anchor_id.to_string(),
            source_fact_ids: anchor.source_fact_ids,
            derived_fact_ids,
            objects: objects.into_iter().collect(),
            relations,
        })
    }

    /// Read the last event-ledger sequence reduced into this store.
    pub fn last_reduced_seq(&self) -> Result<u64, StorageError> {
        let Some(raw) = self
            .runtime_meta
            .get(KEY_LAST_REDUCED_SEQ.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(0);
        };
        let value = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        value.parse::<u64>().map_err(to_storage_parse)
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

    /// Persist the last event-ledger sequence reduced into this store.
    pub fn set_last_reduced_seq(&self, seq: u64) -> Result<(), StorageError> {
        self.runtime_meta
            .insert(KEY_LAST_REDUCED_SEQ.as_bytes(), seq.to_string().as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Clear projection data whose identifiers were derived from a legacy
    /// ledger sequence space before replaying a bound event authority.
    ///
    /// Cursor migration evidence and its recovery marker live in runtime
    /// metadata and are deliberately preserved. The legacy cursor, any
    /// identity-less derived-event retry state, and all sequence-derived
    /// projection indexes are rebuilt from authority sequence zero.
    pub(super) fn reset_for_event_authority_migration(&self) -> Result<(), StorageError> {
        for tree in [
            &self.facts,
            &self.owner_publications,
            &self.fact_objects,
            &self.object_facts,
            &self.outgoing_relations,
            &self.incoming_relations,
            &self.anchors,
            &self.current_anchor,
            &self.anchor_history,
            &self.anchor_lineage,
            &self.source_fact_index,
            &self.seq_index,
            &self.subject_perspective_index,
        ] {
            tree.clear().map_err(to_storage_io)?;
        }
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

    fn fact_for_source_spine_fact(
        &self,
        source_fact_id: &str,
    ) -> Result<Option<TraversalFactRecord>, StorageError> {
        let prefix = format!("{}::", source_fact_id);
        for item in self.source_fact_index.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let fact_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(record) = self.get_fact(&fact_id)? {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    /// Read neighboring objects through relation indexes.
    pub fn neighbors(
        &self,
        object: &DomainObjectRef,
        direction: TraversalDirection,
        relation_types: Option<&[String]>,
        current_only: bool,
    ) -> Result<Vec<DomainObjectRef>, StorageError> {
        let mut neighbors = BTreeSet::new();
        if matches!(
            direction,
            TraversalDirection::Outgoing | TraversalDirection::Both
        ) {
            self.collect_neighbors(
                &self.outgoing_relations,
                object,
                relation_types,
                current_only,
                true,
                &mut neighbors,
            )?;
        }
        if matches!(
            direction,
            TraversalDirection::Incoming | TraversalDirection::Both
        ) {
            self.collect_neighbors(
                &self.incoming_relations,
                object,
                relation_types,
                current_only,
                false,
                &mut neighbors,
            )?;
        }
        Ok(neighbors.into_iter().collect())
    }

    fn collect_neighbors(
        &self,
        tree: &Tree,
        object: &DomainObjectRef,
        relation_types: Option<&[String]>,
        current_only: bool,
        outgoing: bool,
        neighbors: &mut BTreeSet<DomainObjectRef>,
    ) -> Result<(), StorageError> {
        let prefix = format!("{}::", object.index_key());
        for item in tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let record: RelationRecord = serde_json::from_slice(&value).map_err(to_storage_data)?;
            if !self.relation_visible(&record, relation_types, current_only)? {
                continue;
            }
            let neighbor = if outgoing {
                record.relation.dst
            } else {
                record.relation.src
            };
            neighbors.insert(neighbor);
        }
        Ok(())
    }

    /// Run a bounded breadth-first graph walk.
    pub fn walk(
        &self,
        start: &DomainObjectRef,
        spec: &GraphWalkSpec,
    ) -> Result<GraphWalkResult, StorageError> {
        spec.validate()?;
        let mut visited = BTreeSet::new();
        let mut visited_relations = Vec::new();
        let mut visited_facts = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((start.clone(), 0usize));
        visited.insert(start.clone());

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= spec.max_depth {
                continue;
            }
            let neighbors = self.neighbors(
                &current,
                spec.direction,
                spec.relation_types.as_deref(),
                spec.current_only,
            )?;
            for neighbor in neighbors {
                if visited.insert(neighbor.clone()) {
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
            if spec.include_facts {
                for fact in self.facts_for_object(&current, 0)? {
                    visited_facts.insert(fact.fact_id.clone());
                }
            }
            let relation_prefix = format!("{}::", current.index_key());
            let tree = match spec.direction {
                TraversalDirection::Outgoing => Some(&self.outgoing_relations),
                TraversalDirection::Incoming => Some(&self.incoming_relations),
                TraversalDirection::Both => None,
            };
            if let Some(tree) = tree {
                for item in tree.scan_prefix(relation_prefix.as_bytes()) {
                    let (_, value) = item.map_err(to_storage_io)?;
                    let record: RelationRecord =
                        serde_json::from_slice(&value).map_err(to_storage_data)?;
                    if !self.relation_visible(
                        &record,
                        spec.relation_types.as_deref(),
                        spec.current_only,
                    )? {
                        continue;
                    }
                    visited_relations.push(record.relation);
                }
            } else {
                for tree in [&self.outgoing_relations, &self.incoming_relations] {
                    for item in tree.scan_prefix(relation_prefix.as_bytes()) {
                        let (_, value) = item.map_err(to_storage_io)?;
                        let record: RelationRecord =
                            serde_json::from_slice(&value).map_err(to_storage_data)?;
                        if !self.relation_visible(
                            &record,
                            spec.relation_types.as_deref(),
                            spec.current_only,
                        )? {
                            continue;
                        }
                        visited_relations.push(record.relation);
                    }
                }
            }
        }

        let facts = if spec.include_facts {
            let mut out = Vec::new();
            for fact_id in visited_facts {
                if let Some(fact) = self.get_fact(&fact_id)? {
                    out.push(fact);
                }
            }
            out.sort_by_key(|fact| fact.seq);
            out
        } else {
            Vec::new()
        };

        Ok(GraphWalkResult {
            visited_objects: visited.into_iter().collect(),
            visited_facts: facts,
            traversed_relations: visited_relations,
        })
    }
}

impl TraversalStore {
    fn relation_visible(
        &self,
        record: &RelationRecord,
        relation_types: Option<&[String]>,
        current_only: bool,
    ) -> Result<bool, StorageError> {
        if let Some(types) = relation_types {
            if !types
                .iter()
                .any(|value| value == &record.relation.relation_type)
            {
                return Ok(false);
            }
        }
        if current_only && record.relation.relation_type == "selected" {
            let Some(current_anchor) = self.current_anchor(&record.relation.src)? else {
                return Ok(false);
            };
            if current_anchor.target.index_key() != record.relation.dst.index_key() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn encode_membership_key(prefix: &str, item_id: &str) -> String {
    format!("{prefix}::{item_id}")
}

fn encode_relation_key(
    object_key: &str,
    relation_type: &str,
    neighbor_key: &str,
    seq: u64,
    fact_id: &str,
) -> String {
    format!(
        "{}::{}::{}::{:0KEY_PAD$}::{}",
        object_key, relation_type, neighbor_key, seq, fact_id
    )
}

fn encode_seq_index_key(seq: u64, fact_id: &str) -> String {
    format!("{seq:0KEY_PAD$}::{fact_id}")
}

fn encode_subject_perspective_key(
    subject: &DomainObjectRef,
    perspective: &crate::world_state::graph::contracts::PerspectiveKey,
) -> String {
    format!("{}::{}", subject.index_key(), perspective.index_key())
}

fn derived_fact_ids_for_anchor(anchor: &AnchorSelectionRecord) -> Vec<String> {
    let mut derived = vec![anchor.created_by_fact_id.clone()];
    if let Some(ended_by_fact_id) = &anchor.ended_by_fact_id {
        derived.push(ended_by_fact_id.clone());
    }
    derived
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn to_storage_utf8(err: std::string::FromUtf8Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn to_storage_parse(err: std::num::ParseIntError) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}
