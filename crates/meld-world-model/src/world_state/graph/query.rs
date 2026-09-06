//! Read-only graph traversal query facade.
//!
//! This module exposes current anchors, anchor history, provenance, object
//! facts, and bounded walks without exposing sled tree layout. Belief
//! normalization consumes this facade rather than reaching into graph storage.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::world_state::graph::store::TraversalStore;
//! use meld_world_model::TraversalQuery;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let store = TraversalStore::new(sled::open(temp.path()).unwrap()).unwrap();
//! let query = TraversalQuery::new(&store);
//! let object = meld_world_model::events::DomainObjectRef::new(
//!     "workspace_fs",
//!     "node",
//!     "node-a",
//! )
//! .unwrap();
//! assert!(query.current_anchors_for_subject(&object).unwrap().is_empty());
//! ```

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use crate::error::StorageError;
use crate::events::{DomainObjectRef, LedgerCursor};
use crate::world_state::graph::contracts::{
    traversal_cut_identity, traversal_result_identity, AnchorProvenanceRecord,
    AnchorSelectionRecord, BoundedTraversalRequest, GraphWalkResult, GraphWalkSpec,
    OwnerCompletenessStatus, OwnerGraphRevisionReceipt, OwnerObjectPublication,
    OwnerRelationOccurrence, ProjectedOwnerPublication, TraversalCut, TraversalCutIssue,
    TraversalCutRequest, TraversalCutStatus, TraversalDirection, TraversalFactRecord,
    TraversalFrontierEntry, TraversalFrontierReason, TraversalPath, TraversalResult,
    TraversalTruncation,
};
use crate::world_state::graph::store::TraversalStore;

/// Read facade over graph traversal storage.
pub struct TraversalQuery<'a> {
    pub(super) store: &'a TraversalStore,
}

impl<'a> TraversalQuery<'a> {
    /// Create a traversal query facade over an existing store.
    pub fn new(store: &'a TraversalStore) -> Self {
        Self { store }
    }

    /// Select the newest revision at the requested Event position for each owner scope.
    /// A newer incomplete publication prevents using earlier evidence as current.
    pub fn cut(&self, request: &TraversalCutRequest) -> Result<TraversalCut, StorageError> {
        request.validate()?;
        let request = request.normalized();
        let graph_position = self
            .store
            .authority_cursor(request.event_position.ledger_id)?;
        let reduced_seq = graph_position.after_seq;
        let visible_seq = reduced_seq.min(request.event_position.after_seq);
        let publications = self.store.owner_publications_through_seq(visible_seq)?;
        let mut receipts = Vec::new();
        let mut issues = Vec::new();
        if request.event_position.after_seq > reduced_seq {
            issues.push(TraversalCutIssue::ProjectionLag {
                event_seq: request.event_position.after_seq,
                graph_seq: reduced_seq,
            });
        }
        for requirement in &request.owners {
            let event_coverage = if let Some(source) = &requirement.event_source {
                if !self.store.covers_event_source(
                    request.event_position.ledger_id,
                    &requirement.owner_id,
                    source,
                )? {
                    if requirement.required {
                        issues.push(TraversalCutIssue::MissingRequiredOwner {
                            owner_id: requirement.owner_id.clone(),
                            scope_id: requirement.scope.scope_id.clone(),
                        });
                    }
                    continue;
                }
                Some(super::contracts::OwnerEventCoverageReceipt {
                    source: source.clone(),
                    through: request.event_position,
                })
            } else {
                None
            };
            let selected = publications
                .iter()
                .filter(|publication| {
                    let batch = &publication.operation.batch;
                    batch.owner_id == requirement.owner_id
                        && batch.scope == requirement.scope
                        && requirement
                            .event_source
                            .as_ref()
                            .is_none_or(|source| publication.source_route.as_ref() == Some(source))
                })
                .max_by(|left, right| {
                    (left.source_event.seq, &left.operation.operation_id)
                        .cmp(&(right.source_event.seq, &right.operation.operation_id))
                });
            if let Some(publication) = selected {
                let batch = &publication.operation.batch;
                if batch.completeness.status != OwnerCompletenessStatus::Complete {
                    if requirement.required {
                        issues.push(TraversalCutIssue::MissingRequiredOwner {
                            owner_id: requirement.owner_id.clone(),
                            scope_id: requirement.scope.scope_id.clone(),
                        });
                    }
                    continue;
                }
                receipts.push(OwnerGraphRevisionReceipt {
                    event_coverage: event_coverage.clone(),
                    owner_id: batch.owner_id.clone(),
                    revision_id: batch.revision_id.clone(),
                    scope: batch.scope.clone(),
                    completeness: batch.completeness.clone(),
                    source_event: Some(publication.source_event),
                    projection_position: LedgerCursor {
                        ledger_id: publication.source_event.ledger_id,
                        after_seq: publication.source_event.seq,
                    },
                });
            } else if let Some(coverage) = event_coverage {
                let bytes = serde_json::to_vec(&(
                    &requirement.owner_id,
                    &requirement.scope,
                    &coverage.source,
                ))
                .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
                let revision_id = format!("empty-event-source::{}", blake3::hash(&bytes).to_hex());
                receipts.push(OwnerGraphRevisionReceipt {
                    event_coverage: Some(coverage),
                    source_event: None,
                    owner_id: requirement.owner_id.clone(),
                    revision_id: revision_id.clone(),
                    scope: requirement.scope.clone(),
                    completeness: super::contracts::OwnerCompletenessReceipt {
                        receipt_id: revision_id,
                        scope: requirement.scope.clone(),
                        included_ids: Vec::new(),
                        exclusions: Vec::new(),
                        failures: Vec::new(),
                        status: OwnerCompletenessStatus::Complete,
                    },
                    projection_position: request.event_position,
                });
            } else if requirement.required {
                issues.push(TraversalCutIssue::MissingRequiredOwner {
                    owner_id: requirement.owner_id.clone(),
                    scope_id: requirement.scope.scope_id.clone(),
                });
            }
        }
        receipts.sort_by(|left, right| {
            (&left.owner_id, &left.scope, &left.revision_id).cmp(&(
                &right.owner_id,
                &right.scope,
                &right.revision_id,
            ))
        });
        issues.sort();
        issues.dedup();
        let status = if issues.is_empty() {
            TraversalCutStatus::Complete
        } else {
            TraversalCutStatus::Incomplete
        };
        let mut cut = TraversalCut {
            cut_id: String::new(),
            owners: request.owners,
            receipts,
            scope: request.scope,
            currentness: request.currentness,
            event_position: request.event_position,
            graph_position,
            status,
            issues,
        };
        cut.cut_id = traversal_cut_identity(&cut)?;
        Ok(cut)
    }

    /// Execute a deterministic resource-bounded traversal against one cut.
    pub fn traverse(
        &self,
        cut: &TraversalCut,
        request: &BoundedTraversalRequest,
    ) -> Result<TraversalResult, StorageError> {
        cut.validate_identity()?;
        request.validate()?;
        let request = request.normalized();
        let publications = selected_publications(
            self.store
                .owner_publications_through_seq(cut.graph_position.after_seq)?,
            cut,
        );
        let graph = PublicationGraph::new(publications);
        let mut absent_roots = Vec::new();
        let mut selected_objects = BTreeMap::new();
        let mut selected_occurrences = BTreeMap::new();
        let mut paths = Vec::new();
        let mut frontier = Vec::new();
        let mut truncation = TraversalTruncation::default();
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();

        for root in &request.roots {
            let root_observed = graph.objects_by_address.contains_key(&root.index_key());
            if !root_observed {
                if cut.status == TraversalCutStatus::Complete
                    && cut.receipts.iter().any(|receipt| {
                        receipt.owner_id == root.domain_id
                            && receipt.event_coverage.is_some()
                            && receipt.completeness.status == OwnerCompletenessStatus::Complete
                    })
                {
                    absent_roots.push(root.clone());
                } else {
                    frontier.push(frontier_entry(
                        root,
                        0,
                        TraversalFrontierReason::UnresolvedObject,
                    ));
                    continue;
                }
            }
            if !visited.insert(root.index_key()) {
                continue;
            }
            let path = TraversalPath {
                objects: vec![root.clone()],
                occurrence_ids: Vec::new(),
            };
            if paths.len() >= request.bounds.max_paths {
                truncation.paths = true;
                frontier.push(frontier_entry(root, 0, TraversalFrontierReason::PathBound));
                continue;
            }
            paths.push(path.clone());
            if root_observed
                && !select_objects(
                    &graph,
                    root,
                    request.bounds.max_objects,
                    &mut selected_objects,
                )
            {
                truncation.objects = true;
                frontier.push(frontier_entry(
                    root,
                    0,
                    TraversalFrontierReason::ObjectBound,
                ));
                continue;
            }
            queue.push_back((root.clone(), 0usize, path));
        }

        while let Some((current, depth, path)) = queue.pop_front() {
            let candidates = graph.candidates(&current, &request);
            if depth >= request.bounds.max_depth {
                if !candidates.is_empty() {
                    truncation.depth = true;
                    frontier.push(frontier_entry(
                        &current,
                        depth,
                        TraversalFrontierReason::DepthBound,
                    ));
                }
                continue;
            }
            for candidate in candidates {
                let occurrence_key = candidate.occurrence_key();
                if !selected_occurrences.contains_key(&occurrence_key)
                    && selected_occurrences.len() >= request.bounds.max_occurrences
                {
                    truncation.occurrences = true;
                    frontier.push(frontier_entry(
                        &current,
                        depth,
                        TraversalFrontierReason::OccurrenceBound,
                    ));
                    break;
                }
                selected_occurrences
                    .entry(occurrence_key)
                    .or_insert_with(|| candidate.occurrence.clone());
                if !graph
                    .objects_by_address
                    .contains_key(&candidate.neighbor.index_key())
                {
                    frontier.push(frontier_entry(
                        &candidate.neighbor,
                        depth + 1,
                        TraversalFrontierReason::UnresolvedObject,
                    ));
                    continue;
                }
                let mut next_path = path.clone();
                next_path.objects.push(candidate.neighbor.clone());
                next_path
                    .occurrence_ids
                    .push(candidate.occurrence.occurrence_id.clone());
                if visited.contains(&candidate.neighbor.index_key()) {
                    if !paths.contains(&next_path) {
                        if paths.len() >= request.bounds.max_paths {
                            truncation.paths = true;
                            frontier.push(frontier_entry(
                                &candidate.neighbor,
                                depth + 1,
                                TraversalFrontierReason::PathBound,
                            ));
                        } else {
                            paths.push(next_path);
                        }
                    }
                    continue;
                }
                if graph.object_count(&candidate.neighbor) + selected_objects.len()
                    > request.bounds.max_objects
                {
                    truncation.objects = true;
                    frontier.push(frontier_entry(
                        &candidate.neighbor,
                        depth + 1,
                        TraversalFrontierReason::ObjectBound,
                    ));
                    continue;
                }
                if paths.len() >= request.bounds.max_paths {
                    truncation.paths = true;
                    frontier.push(frontier_entry(
                        &candidate.neighbor,
                        depth + 1,
                        TraversalFrontierReason::PathBound,
                    ));
                    continue;
                }
                visited.insert(candidate.neighbor.index_key());
                paths.push(next_path.clone());
                select_objects(
                    &graph,
                    &candidate.neighbor,
                    request.bounds.max_objects,
                    &mut selected_objects,
                );
                queue.push_back((candidate.neighbor, depth + 1, next_path));
            }
        }

        let mut objects = selected_objects.into_values().collect::<Vec<_>>();
        objects.sort_by(|left, right| {
            (&left.hydration.owner_id, &left.publication_id)
                .cmp(&(&right.hydration.owner_id, &right.publication_id))
        });
        let mut occurrences = selected_occurrences.into_values().collect::<Vec<_>>();
        occurrences.sort_by(|left, right| {
            (&left.hydration.owner_id, &left.occurrence_id)
                .cmp(&(&right.hydration.owner_id, &right.occurrence_id))
        });
        paths.sort();
        paths.dedup();
        frontier.sort();
        frontier.dedup();
        Ok(TraversalResult {
            absent_roots,
            result_id: traversal_result_identity(&cut.cut_id, &request)?,
            cut_id: cut.cut_id.clone(),
            objects,
            occurrences,
            paths,
            receipts: cut.receipts.clone(),
            frontier,
            truncation,
        })
    }

    /// Read the current anchor for a logical anchor reference.
    pub fn current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.store.current_anchor(anchor_ref)
    }

    /// Read the current anchor for one subject and perspective.
    pub fn current_anchor_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.store
            .current_anchor_for_subject(subject, perspective_kind, perspective_id)
    }

    /// Read every current anchor for one subject.
    pub fn current_anchors_for_subject(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.store.current_anchors_for_subject(subject)
    }

    /// Read anchor history for one logical anchor reference.
    pub fn anchor_history(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.store.anchor_history(anchor_ref)
    }

    /// Read adjacent objects through relation indexes.
    pub fn neighbors(
        &self,
        object: &DomainObjectRef,
        direction: TraversalDirection,
        relation_types: Option<&[String]>,
        current_only: bool,
    ) -> Result<Vec<DomainObjectRef>, StorageError> {
        self.store
            .neighbors(object, direction, relation_types, current_only)
    }

    /// Run a bounded graph walk from one object.
    pub fn walk(
        &self,
        start: &DomainObjectRef,
        spec: &GraphWalkSpec,
    ) -> Result<GraphWalkResult, StorageError> {
        self.store.walk(start, spec)
    }

    /// Read graph-readable facts for one object after a sequence cursor.
    pub fn facts_for_object(
        &self,
        object: &DomainObjectRef,
        after_seq: u64,
    ) -> Result<Vec<TraversalFactRecord>, StorageError> {
        self.store.facts_for_object(object, after_seq)
    }

    /// Read compact provenance for one anchor.
    pub fn provenance_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError> {
        self.store.anchor_provenance(anchor_id)
    }

    /// Read an anchor record when callers need generic graph lifecycle state.
    pub fn supersession_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        Ok(self.store.get_anchor(anchor_id)?.filter(|anchor| {
            anchor.ended_at_seq.is_some()
                || anchor.ended_by_anchor_id.is_some()
                || anchor.ended_by_fact_id.is_some()
        }))
    }

    /// Read the current workspace snapshot anchor for a source object.
    pub fn current_snapshot_for_source(
        &self,
        source: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(source, "snapshot", "current")
    }

    /// Read the current frame head anchor for a node and frame type.
    pub fn current_frame_head(
        &self,
        node: &DomainObjectRef,
        frame_type: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(node, "frame_type", frame_type)
    }

    /// Read current frame head anchors for a node.
    pub fn current_frame_heads_for_node(
        &self,
        node: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        Ok(self
            .current_anchors_for_subject(node)?
            .into_iter()
            .filter(|anchor| anchor.perspective.perspective_kind == "frame_type")
            .collect())
    }

    /// Count current frame head anchors for a frame type.
    pub fn current_frame_head_count_by_type(
        &self,
        frame_type: &str,
    ) -> Result<usize, StorageError> {
        self.store
            .current_anchor_count_by_perspective("frame_type", frame_type)
    }

    /// Read the current artifact anchor for a task run and artifact type.
    pub fn current_artifact_for_task_run(
        &self,
        task_run: &DomainObjectRef,
        artifact_type_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(task_run, "artifact_type", artifact_type_id)
    }
}

fn selected_publications(
    publications: Vec<ProjectedOwnerPublication>,
    cut: &TraversalCut,
) -> Vec<ProjectedOwnerPublication> {
    let events = cut
        .receipts
        .iter()
        .filter_map(|receipt| receipt.source_event)
        .collect::<HashSet<_>>();
    publications
        .into_iter()
        .filter(|publication| events.contains(&publication.source_event))
        .collect()
}

struct PublicationGraph {
    objects_by_address: BTreeMap<String, Vec<OwnerObjectPublication>>,
    outgoing: BTreeMap<String, Vec<TraversalCandidate>>,
    incoming: BTreeMap<String, Vec<TraversalCandidate>>,
}

impl PublicationGraph {
    fn new(publications: Vec<ProjectedOwnerPublication>) -> Self {
        let mut graph = Self {
            objects_by_address: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
        };
        for publication in publications {
            for object in publication.operation.batch.objects {
                graph
                    .objects_by_address
                    .entry(object.object_ref.index_key())
                    .or_default()
                    .push(object);
            }
            for occurrence in publication.operation.batch.relations {
                graph
                    .outgoing
                    .entry(occurrence.src.index_key())
                    .or_default()
                    .push(TraversalCandidate {
                        neighbor: occurrence.dst.clone(),
                        occurrence: occurrence.clone(),
                    });
                graph
                    .incoming
                    .entry(occurrence.dst.index_key())
                    .or_default()
                    .push(TraversalCandidate {
                        neighbor: occurrence.src.clone(),
                        occurrence,
                    });
            }
        }
        for objects in graph.objects_by_address.values_mut() {
            objects.sort_by(|left, right| left.publication_id.cmp(&right.publication_id));
        }
        for candidates in graph
            .outgoing
            .values_mut()
            .chain(graph.incoming.values_mut())
        {
            candidates.sort_by_key(TraversalCandidate::occurrence_key);
        }
        graph
    }

    fn object_count(&self, object: &DomainObjectRef) -> usize {
        self.objects_by_address
            .get(&object.index_key())
            .map_or(0, Vec::len)
    }

    fn candidates(
        &self,
        object: &DomainObjectRef,
        request: &BoundedTraversalRequest,
    ) -> Vec<TraversalCandidate> {
        let key = object.index_key();
        let mut candidates = Vec::new();
        if matches!(
            request.direction,
            TraversalDirection::Outgoing | TraversalDirection::Both
        ) {
            candidates.extend(self.outgoing.get(&key).cloned().unwrap_or_default());
        }
        if matches!(
            request.direction,
            TraversalDirection::Incoming | TraversalDirection::Both
        ) {
            candidates.extend(self.incoming.get(&key).cloned().unwrap_or_default());
        }
        if let Some(types) = &request.relation_types {
            candidates.retain(|candidate| types.contains(&candidate.occurrence.relation_type));
        }
        candidates.sort_by_key(TraversalCandidate::occurrence_key);
        candidates.dedup_by(|left, right| left.occurrence_key() == right.occurrence_key());
        candidates
    }
}

#[derive(Clone)]
struct TraversalCandidate {
    neighbor: DomainObjectRef,
    occurrence: OwnerRelationOccurrence,
}

impl TraversalCandidate {
    fn occurrence_key(&self) -> String {
        format!(
            "{}::{}",
            self.occurrence.hydration.owner_id, self.occurrence.occurrence_id
        )
    }
}

fn select_objects(
    graph: &PublicationGraph,
    object: &DomainObjectRef,
    max_objects: usize,
    selected: &mut BTreeMap<String, OwnerObjectPublication>,
) -> bool {
    let Some(publications) = graph.objects_by_address.get(&object.index_key()) else {
        return false;
    };
    let additional = publications
        .iter()
        .filter(|publication| {
            !selected.contains_key(&format!(
                "{}::{}",
                publication.hydration.owner_id, publication.publication_id
            ))
        })
        .count();
    if selected.len() + additional > max_objects {
        return false;
    }
    for publication in publications {
        selected.insert(
            format!(
                "{}::{}",
                publication.hydration.owner_id, publication.publication_id
            ),
            publication.clone(),
        );
    }
    true
}

fn frontier_entry(
    object_ref: &DomainObjectRef,
    depth: usize,
    reason: TraversalFrontierReason,
) -> TraversalFrontierEntry {
    TraversalFrontierEntry {
        object_ref: object_ref.clone(),
        depth,
        reason,
    }
}

#[cfg(test)]
mod owner_publication_tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::events::{DomainObjectRef, EventEnvelope};
    use crate::world_state::graph::contracts::{
        HydrationReference, OwnerCompletenessReceipt, OwnerCompletenessStatus,
        OwnerCurrentnessPolicy, OwnerObjectPublication, OwnerPublicationBatch,
        OwnerPublicationFailure, OwnerPublicationOperation, OwnerPublicationScope,
        OwnerPublicationState, OwnerRelationOccurrence, TraversalBounds, TraversalOwnerRequirement,
        OWNER_PUBLICATION_EVENT_TYPE,
    };
    use crate::world_state::graph::events::owner_publication_envelope;
    use crate::world_state::graph::test_support::GraphRuntimeTestFixture;

    #[test]
    fn cut_tracks_lag_preserves_occurrences_and_excludes_typed_structural_facts() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path()).unwrap();
        let fixture = GraphRuntimeTestFixture::open(db.clone()).unwrap();
        let operation = operation("docs", "scope-a", "revision-a", true);
        let receipt = fixture
            .append(owner_publication_envelope("session-a", &operation).unwrap())
            .unwrap();
        let runtime = fixture.runtime();
        let event_position = LedgerCursor {
            ledger_id: fixture.ledger_identity(),
            after_seq: receipt.seq,
        };
        let cut_request = cut_request("docs", "scope-a", event_position);
        let store = runtime.traversal_store();
        let query = TraversalQuery::new(store.as_ref());
        let lagging = query.cut(&cut_request).unwrap();
        assert_eq!(lagging.status, TraversalCutStatus::Incomplete);
        assert!(lagging
            .issues
            .iter()
            .any(|issue| matches!(issue, TraversalCutIssue::ProjectionLag { .. })));
        assert!(lagging
            .issues
            .iter()
            .any(|issue| matches!(issue, TraversalCutIssue::MissingRequiredOwner { .. })));

        let expected =
            super::super::contracts::OwnerPublicationExpectation::from_operation(&operation)
                .unwrap();
        assert!(query
            .publication_visibility(&lagging, &expected)
            .unwrap()
            .is_none());
        runtime.catch_up().unwrap();
        let cut = query.cut(&cut_request).unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Complete);
        assert_eq!(cut.receipts[0].revision_id, "revision-a");
        assert_eq!(query.cut(&cut_request).unwrap().cut_id, cut.cut_id);
        let visibility = query
            .publication_visibility(&cut, &expected)
            .unwrap()
            .unwrap();
        assert_eq!(visibility.cut_id(), cut.cut_id);
        assert_eq!(visibility.receipt(), &cut.receipts[0]);
        let mut foreign = expected.clone();
        foreign.event_record_id = "foreign-event-record".into();
        assert!(query
            .publication_visibility(&cut, &foreign)
            .unwrap()
            .is_none());
        foreign = expected.clone();
        foreign.scope.scope_id = "foreign-scope".into();
        assert!(query
            .publication_visibility(&cut, &foreign)
            .unwrap()
            .is_none());
        let mut fabricated = cut.clone();
        fabricated.receipts[0].revision_id = "fabricated-revision".into();
        fabricated.cut_id = traversal_cut_identity(&fabricated).unwrap();
        foreign = expected.clone();
        foreign.revision_id = "fabricated-revision".into();
        assert!(query
            .publication_visibility(&fabricated, &foreign)
            .unwrap()
            .is_none());
        let request = traversal_request("docs", 16);
        let result = query.traverse(&cut, &request).unwrap();
        assert_eq!(result.occurrences.len(), 2);
        assert_eq!(result.occurrences[0].src, result.occurrences[1].src);
        assert_eq!(result.occurrences[0].dst, result.occurrences[1].dst);
        assert_ne!(
            result.occurrences[0].occurrence_id,
            result.occurrences[1].occurrence_id
        );
        assert_eq!(
            query.traverse(&cut, &request).unwrap().result_id,
            result.result_id
        );
        assert!(query
            .facts_for_object(&object("docs", "a"), 0)
            .unwrap()
            .is_empty());

        let bounded = query.traverse(&cut, &traversal_request("docs", 1)).unwrap();
        assert!(bounded.truncation.occurrences);
        assert!(bounded
            .frontier
            .iter()
            .any(|entry| entry.reason == TraversalFrontierReason::OccurrenceBound));
        drop(store);
        drop(runtime);
        drop(fixture);
        drop(db);

        let reopened_db = sled::open(temp.path()).unwrap();
        let reopened = GraphRuntimeTestFixture::open(reopened_db).unwrap();
        let store = reopened.runtime().traversal_store();
        let reopened_cut = TraversalQuery::new(store.as_ref())
            .cut(&cut_request)
            .unwrap();
        assert_eq!(reopened_cut.cut_id, cut.cut_id);
        assert_eq!(
            TraversalQuery::new(store.as_ref())
                .publication_visibility(&cut, &expected)
                .unwrap(),
            Some(visibility)
        );
    }

    #[test]
    fn newer_incomplete_owner_blocks_current_cut_but_preserves_historical_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let fixture = GraphRuntimeTestFixture::open(sled::open(temp.path()).unwrap()).unwrap();
        let complete = operation("dependency_security", "repo-a", "revision-1", true);
        let incomplete = operation("dependency_security", "repo-a", "revision-2", false);
        let first = fixture
            .append(owner_publication_envelope("session-a", &complete).unwrap())
            .unwrap();
        let latest = fixture
            .append(owner_publication_envelope("session-a", &incomplete).unwrap())
            .unwrap();
        let raw_object = object("workspace_fs", "raw-only");
        fixture
            .append(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "workspace_fs",
                    "scope-a",
                    "workspace_fs.node_observed",
                    None,
                    serde_json::json!({"legacy": true}),
                )
                .with_graph(vec![raw_object], Vec::new()),
            )
            .unwrap();
        let runtime = fixture.runtime();
        runtime.catch_up().unwrap();
        let store = runtime.traversal_store();
        let query = TraversalQuery::new(store.as_ref());
        let position = LedgerCursor {
            ledger_id: fixture.ledger_identity(),
            after_seq: latest.seq + 1,
        };
        let cut = query
            .cut(&cut_request("dependency_security", "repo-a", position))
            .unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Incomplete);
        assert!(cut.receipts.is_empty());
        let historical = query
            .cut(&cut_request(
                "dependency_security",
                "repo-a",
                LedgerCursor {
                    ledger_id: fixture.ledger_identity(),
                    after_seq: first.seq,
                },
            ))
            .unwrap();
        assert_eq!(historical.status, TraversalCutStatus::Complete);
        assert_eq!(historical.receipts[0].revision_id, "revision-1");
        let missing = query
            .cut(&cut_request("workspace_fs", "scope-a", position))
            .unwrap();
        assert_eq!(missing.status, TraversalCutStatus::Incomplete);
        assert!(missing.receipts.is_empty());
        assert!(query
            .facts_for_object(&object("workspace_fs", "raw-only"), 0)
            .unwrap()
            .is_empty());
        assert_eq!(
            complete.batch.objects[0].hydration.owner_id,
            "dependency_security"
        );
        assert_eq!(
            complete.batch.objects[0].hydration.role,
            "published_material"
        );
        assert_eq!(
            OWNER_PUBLICATION_EVENT_TYPE,
            "world_state.owner_publication.v1"
        );
    }

    #[test]
    fn divergent_operation_for_one_owner_revision_is_rejected_before_cursor_advance() {
        let temp = tempfile::tempdir().unwrap();
        let fixture = GraphRuntimeTestFixture::open(sled::open(temp.path()).unwrap()).unwrap();
        let first = operation("docs", "scope-a", "revision-a", true);
        fixture
            .append(owner_publication_envelope("session-a", &first).unwrap())
            .unwrap();
        let mut divergent_batch = first.batch.clone();
        divergent_batch.objects[0]
            .qualifications
            .insert("variant".to_string(), "divergent".to_string());
        let divergent =
            OwnerPublicationOperation::reconstruct("owner-rule-v1", divergent_batch).unwrap();
        let divergent_seq = fixture
            .append(owner_publication_envelope("session-a", &divergent).unwrap())
            .unwrap()
            .seq;
        let runtime = fixture.runtime();
        let error = runtime.catch_up().unwrap_err();
        assert!(error
            .to_string()
            .contains("divergent publication operations"));
        assert!(runtime.durable_event_cursor().unwrap().after_seq < divergent_seq);
    }

    fn operation(
        owner_id: &str,
        scope_id: &str,
        revision_id: &str,
        complete: bool,
    ) -> OwnerPublicationOperation {
        let left = object(owner_id, "a");
        let right = object(owner_id, "b");
        let scope = scope(scope_id);
        let objects = vec![
            publication(owner_id, revision_id, "object-a", left.clone()),
            publication(owner_id, revision_id, "object-b", right.clone()),
        ];
        let relations = vec![
            occurrence(owner_id, revision_id, "occurrence-a", &left, &right),
            occurrence(owner_id, revision_id, "occurrence-b", &left, &right),
        ];
        let included_ids = objects
            .iter()
            .map(|item| item.publication_id.clone())
            .chain(relations.iter().map(|item| item.occurrence_id.clone()))
            .collect();
        OwnerPublicationOperation::reconstruct(
            "owner-rule-v1",
            OwnerPublicationBatch {
                owner_id: owner_id.to_string(),
                revision_id: revision_id.to_string(),
                scope: scope.clone(),
                objects,
                relations,
                completeness: OwnerCompletenessReceipt {
                    receipt_id: format!("receipt::{owner_id}::{revision_id}"),
                    scope,
                    included_ids,
                    exclusions: Vec::new(),
                    failures: if complete {
                        Vec::new()
                    } else {
                        vec![OwnerPublicationFailure {
                            source_ref: "source-a".to_string(),
                            reason: "unreadable".to_string(),
                        }]
                    },
                    status: if complete {
                        OwnerCompletenessStatus::Complete
                    } else {
                        OwnerCompletenessStatus::Incomplete
                    },
                },
            },
        )
        .unwrap()
    }

    fn publication(
        owner_id: &str,
        revision_id: &str,
        publication_id: &str,
        object_ref: DomainObjectRef,
    ) -> OwnerObjectPublication {
        OwnerObjectPublication {
            publication_id: publication_id.to_string(),
            object_ref,
            state: OwnerPublicationState::Observed,
            source_product_ref: publication_id.to_string(),
            hydration: hydration(owner_id, revision_id, publication_id, "published_material"),
            provenance_refs: vec![format!("source::{revision_id}")],
            qualifications: BTreeMap::new(),
        }
    }

    fn occurrence(
        owner_id: &str,
        revision_id: &str,
        occurrence_id: &str,
        src: &DomainObjectRef,
        dst: &DomainObjectRef,
    ) -> OwnerRelationOccurrence {
        OwnerRelationOccurrence {
            occurrence_id: occurrence_id.to_string(),
            relation_type: "related_to".to_string(),
            src: src.clone(),
            dst: dst.clone(),
            source_product_ref: occurrence_id.to_string(),
            hydration: hydration(owner_id, revision_id, occurrence_id, "qualified_relation"),
            qualifications: BTreeMap::new(),
            provenance_refs: vec![format!("source::{revision_id}")],
        }
    }

    fn hydration(
        owner_id: &str,
        revision_id: &str,
        product_id: &str,
        role: &str,
    ) -> HydrationReference {
        HydrationReference {
            owner_id: owner_id.to_string(),
            product_kind: "specimen".to_string(),
            product_id: product_id.to_string(),
            revision_id: revision_id.to_string(),
            role: role.to_string(),
        }
    }

    fn object(owner_id: &str, object_id: &str) -> DomainObjectRef {
        DomainObjectRef::new(owner_id, "specimen", object_id).unwrap()
    }

    fn scope(scope_id: &str) -> OwnerPublicationScope {
        OwnerPublicationScope {
            scope_id: scope_id.to_string(),
            branch_id: None,
            perspective_id: None,
            valid_at: None,
        }
    }

    fn cut_request(
        owner_id: &str,
        scope_id: &str,
        event_position: LedgerCursor,
    ) -> TraversalCutRequest {
        let scope = scope(scope_id);
        TraversalCutRequest {
            owners: vec![TraversalOwnerRequirement {
                event_source: None,
                owner_id: owner_id.to_string(),
                scope: scope.clone(),
                required: true,
            }],
            scope,
            currentness: OwnerCurrentnessPolicy::LatestComplete,
            event_position,
        }
    }

    fn traversal_request(owner_id: &str, max_occurrences: usize) -> BoundedTraversalRequest {
        BoundedTraversalRequest {
            roots: vec![object(owner_id, "a")],
            direction: TraversalDirection::Outgoing,
            relation_types: None,
            bounds: TraversalBounds {
                max_depth: 2,
                max_objects: 8,
                max_occurrences,
                max_paths: 8,
            },
        }
    }
}
