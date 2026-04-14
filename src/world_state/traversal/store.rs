use std::collections::{BTreeSet, VecDeque};
use std::io;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::error::StorageError;
use crate::telemetry::{DomainObjectRef, EventRelation};
use crate::world_state::traversal::contracts::{
    AnchorProvenanceRecord, AnchorSelectionRecord, GraphWalkResult, GraphWalkSpec,
    TraversalDirection, TraversalFactRecord,
};

const TREE_FACTS: &str = "traversal_facts";
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
const KEY_PAD: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationRecord {
    pub relation: EventRelation,
    pub fact_id: String,
    pub seq: u64,
}

#[derive(Clone)]
pub struct TraversalStore {
    db: Db,
    facts: Tree,
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
}

impl TraversalStore {
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            facts: db.open_tree(TREE_FACTS).map_err(to_storage_io)?,
            fact_objects: db.open_tree(TREE_FACT_OBJECTS).map_err(to_storage_io)?,
            object_facts: db.open_tree(TREE_OBJECT_FACTS).map_err(to_storage_io)?,
            outgoing_relations: db.open_tree(TREE_OUTGOING_RELATIONS).map_err(to_storage_io)?,
            incoming_relations: db.open_tree(TREE_INCOMING_RELATIONS).map_err(to_storage_io)?,
            anchors: db.open_tree(TREE_ANCHORS).map_err(to_storage_io)?,
            current_anchor: db.open_tree(TREE_CURRENT_ANCHOR).map_err(to_storage_io)?,
            anchor_history: db.open_tree(TREE_ANCHOR_HISTORY).map_err(to_storage_io)?,
            anchor_lineage: db.open_tree(TREE_ANCHOR_LINEAGE).map_err(to_storage_io)?,
            source_fact_index: db.open_tree(TREE_SOURCE_FACT_INDEX).map_err(to_storage_io)?,
            seq_index: db.open_tree(TREE_SEQ_INDEX).map_err(to_storage_io)?,
            subject_perspective_index: db.open_tree(TREE_SUBJECT_PERSPECTIVE).map_err(to_storage_io)?,
            db,
        })
    }

    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn put_fact(&self, fact: &TraversalFactRecord) -> Result<(), StorageError> {
        self.facts
            .insert(fact.fact_id.as_bytes(), serde_json::to_vec(fact).map_err(to_storage_data)?)
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

    pub fn get_fact(&self, fact_id: &str) -> Result<Option<TraversalFactRecord>, StorageError> {
        let Some(raw) = self.facts.get(fact_id.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
    }

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

    pub fn get_anchor(&self, anchor_id: &str) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        let Some(raw) = self.anchors.get(anchor_id.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
    }

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

    pub fn clear_current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
        subject: &DomainObjectRef,
        perspective: &crate::world_state::traversal::contracts::PerspectiveKey,
    ) -> Result<(), StorageError> {
        self.current_anchor
            .remove(anchor_ref.index_key().as_bytes())
            .map_err(to_storage_io)?;
        self.subject_perspective_index
            .remove(encode_subject_perspective_key(subject, perspective).as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

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
        let Some(raw) = self.subject_perspective_index.get(key.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        let anchor_id = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        self.get_anchor(&anchor_id)
    }

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

    pub fn put_anchor_lineage(&self, anchor_id: &str, superseded_by_anchor_id: &str) -> Result<(), StorageError> {
        self.anchor_lineage
            .insert(
                encode_membership_key(anchor_id, superseded_by_anchor_id).as_bytes(),
                superseded_by_anchor_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

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
        Ok(AnchorProvenanceRecord {
            anchor_id: anchor_id.to_string(),
            source_fact_ids: anchor.source_fact_ids,
            objects: objects.into_iter().collect(),
            relations,
        })
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

    pub fn neighbors(
        &self,
        object: &DomainObjectRef,
        direction: TraversalDirection,
        relation_types: Option<&[String]>,
        current_only: bool,
    ) -> Result<Vec<DomainObjectRef>, StorageError> {
        let mut neighbors = BTreeSet::new();
        if matches!(direction, TraversalDirection::Outgoing | TraversalDirection::Both) {
            self.collect_neighbors(
                &self.outgoing_relations,
                object,
                relation_types,
                current_only,
                true,
                &mut neighbors,
            )?;
        }
        if matches!(direction, TraversalDirection::Incoming | TraversalDirection::Both) {
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
            if let Some(types) = relation_types {
                if !types.iter().any(|value| value == &record.relation.relation_type) {
                    continue;
                }
            }
            if current_only && record.relation.relation_type == "selected" {
                let anchor_ref = if outgoing {
                    &record.relation.src
                } else {
                    &record.relation.dst
                };
                if self.current_anchor(anchor_ref)?.is_none() {
                    continue;
                }
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
                    let record: RelationRecord = serde_json::from_slice(&value).map_err(to_storage_data)?;
                    visited_relations.push(record.relation);
                }
            } else {
                for tree in [&self.outgoing_relations, &self.incoming_relations] {
                    for item in tree.scan_prefix(relation_prefix.as_bytes()) {
                        let (_, value) = item.map_err(to_storage_io)?;
                        let record: RelationRecord = serde_json::from_slice(&value).map_err(to_storage_data)?;
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
    perspective: &crate::world_state::traversal::contracts::PerspectiveKey,
) -> String {
    format!("{}::{}", subject.index_key(), perspective.index_key())
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
