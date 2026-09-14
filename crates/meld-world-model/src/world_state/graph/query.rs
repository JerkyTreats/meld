//! Read-only traversal over owner publications selected by a canonical Event cut.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::error::StorageError;
use crate::events::{DomainObjectRef, LedgerCursor};
use crate::world_state::graph::contracts::{
    traversal_cut_identity, traversal_result_identity, BoundedTraversalRequest,
    OwnerCompletenessStatus, OwnerGraphRevisionReceipt, OwnerObjectPublication,
    OwnerRelationOccurrence, TraversalCut, TraversalCutIssue, TraversalCutRequest,
    TraversalCutStatus, TraversalDirection, TraversalFrontierEntry, TraversalFrontierReason,
    TraversalPath, TraversalResult, TraversalTruncation,
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
    #[tracing::instrument(target = "meld::trace", name = "graph.cut", skip_all)]
    pub fn cut(&self, request: &TraversalCutRequest) -> Result<TraversalCut, StorageError> {
        request.validate()?;
        let request = request.normalized();
        let graph_position = self
            .store
            .authority_cursor(request.event_position.ledger_id)?;
        let reduced_seq = graph_position.after_seq;
        let visible_seq = reduced_seq.min(request.event_position.after_seq);
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
            let selected = self.store.publications.select(requirement, visible_seq)?;
            if let Some(publication) = selected {
                let batch = &publication;
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
                    work_input_basis_id: batch.work_input_basis_id.clone(),
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
                    work_input_basis_id: None,
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
    #[tracing::instrument(target = "meld::trace", name = "graph.traverse", skip_all)]
    pub fn traverse(
        &self,
        cut: &TraversalCut,
        request: &BoundedTraversalRequest,
    ) -> Result<TraversalResult, StorageError> {
        cut.validate_identity()?;
        request.validate()?;
        let request = request.normalized();
        self.store.authority_cursor(cut.event_position.ledger_id)?;
        for receipt in &cut.receipts {
            if let Some(event) = receipt.source_event {
                let publication = self
                    .store
                    .publications
                    .header_for_event(event.seq)?
                    .ok_or_else(|| {
                        StorageError::InvalidPath("cut publication is unavailable".into())
                    })?;
                let batch = &publication;
                if event != publication.source_event
                    || event.ledger_id != cut.event_position.ledger_id
                    || event.seq
                        > cut
                            .event_position
                            .after_seq
                            .min(cut.graph_position.after_seq)
                    || batch.owner_id != receipt.owner_id
                    || batch.scope != receipt.scope
                    || batch.revision_id != receipt.revision_id
                    || batch.completeness != receipt.completeness
                    || batch.work_input_basis_id != receipt.work_input_basis_id
                {
                    return Err(StorageError::InvalidPath(
                        "cut receipt disagrees with admitted publication".into(),
                    ));
                }
            }
        }
        let graph = PublicationGraph {
            store: self.store,
            cut,
            objects: Default::default(),
        };
        let mut absent_roots = Vec::new();
        let mut selected_objects = BTreeMap::new();
        let mut selected_occurrences = BTreeMap::new();
        let mut paths = Vec::new();
        let mut frontier = Vec::new();
        let mut truncation = TraversalTruncation::default();
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();

        for root in &request.roots {
            let root_observed = !graph.objects(root)?.is_empty();
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
            if !visited.insert(
                serde_json::to_string(root)
                    .map_err(|e| StorageError::InvalidPath(e.to_string()))?,
            ) {
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
                )?
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
            let candidates = graph.candidates(&current, &request)?;
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
                if graph.objects(&candidate.neighbor)?.is_empty() {
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
                if visited.contains(
                    &serde_json::to_string(&candidate.neighbor)
                        .map_err(|e| StorageError::InvalidPath(e.to_string()))?,
                ) {
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
                if graph.objects(&candidate.neighbor)?.len() + selected_objects.len()
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
                visited.insert(
                    serde_json::to_string(&candidate.neighbor)
                        .map_err(|e| StorageError::InvalidPath(e.to_string()))?,
                );
                paths.push(next_path.clone());
                select_objects(
                    &graph,
                    &candidate.neighbor,
                    request.bounds.max_objects,
                    &mut selected_objects,
                )?;
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
}

struct PublicationGraph<'a> {
    store: &'a TraversalStore,
    cut: &'a TraversalCut,
    objects: std::cell::RefCell<BTreeMap<String, Vec<OwnerObjectPublication>>>,
}

impl PublicationGraph<'_> {
    fn objects(
        &self,
        object: &DomainObjectRef,
    ) -> Result<Vec<OwnerObjectPublication>, StorageError> {
        let key =
            serde_json::to_string(object).map_err(|e| StorageError::InvalidPath(e.to_string()))?;
        if let Some(objects) = self.objects.borrow().get(&key) {
            return Ok(objects.clone());
        }
        let mut objects = Vec::new();
        for event in self.cut.receipts.iter().filter_map(|r| r.source_event) {
            objects.extend(self.store.publications.objects(event.seq, object)?);
        }
        objects.sort_by(|left, right| left.publication_id.cmp(&right.publication_id));
        self.objects.borrow_mut().insert(key, objects.clone());
        Ok(objects)
    }

    fn candidates(
        &self,
        object: &DomainObjectRef,
        request: &BoundedTraversalRequest,
    ) -> Result<Vec<TraversalCandidate>, StorageError> {
        let mut candidates = Vec::new();
        for event in self.cut.receipts.iter().filter_map(|r| r.source_event) {
            for incoming in [false, true] {
                if incoming && request.direction == TraversalDirection::Outgoing
                    || !incoming && request.direction == TraversalDirection::Incoming
                {
                    continue;
                }
                for occurrence in self
                    .store
                    .publications
                    .relations(event.seq, object, incoming)?
                {
                    if request
                        .relation_types
                        .as_ref()
                        .is_some_and(|types| !types.contains(&occurrence.relation_type))
                    {
                        continue;
                    }
                    let neighbor = if incoming {
                        occurrence.src.clone()
                    } else {
                        occurrence.dst.clone()
                    };
                    candidates.push(TraversalCandidate {
                        neighbor,
                        occurrence,
                    });
                }
            }
        }
        candidates.sort_by_key(TraversalCandidate::occurrence_key);
        candidates.dedup_by(|left, right| left.occurrence_key() == right.occurrence_key());
        Ok(candidates)
    }
}

#[derive(Clone)]
struct TraversalCandidate {
    neighbor: DomainObjectRef,
    occurrence: OwnerRelationOccurrence,
}

impl TraversalCandidate {
    fn occurrence_key(&self) -> (String, String) {
        (
            self.occurrence.hydration.owner_id.clone(),
            self.occurrence.occurrence_id.clone(),
        )
    }
}

fn select_objects(
    graph: &PublicationGraph,
    object: &DomainObjectRef,
    max_objects: usize,
    selected: &mut BTreeMap<(String, String), OwnerObjectPublication>,
) -> Result<bool, StorageError> {
    let publications = graph.objects(object)?;
    if publications.is_empty() {
        return Ok(false);
    }
    let additional = publications
        .iter()
        .filter(|publication| {
            !selected.contains_key(&(
                publication.hydration.owner_id.clone(),
                publication.publication_id.clone(),
            ))
        })
        .count();
    if selected.len() + additional > max_objects {
        return Ok(false);
    }
    for publication in publications {
        selected.insert(
            (
                publication.hydration.owner_id.clone(),
                publication.publication_id.clone(),
            ),
            publication.clone(),
        );
    }
    Ok(true)
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
        let graph_directory = tempfile::tempdir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path()).unwrap();
        let fixture =
            GraphRuntimeTestFixture::open(db.clone(), graph_directory.path().join("graph.agdb"))
                .unwrap();
        let mut batch = operation("docs", "scope-a", "revision-a", true).batch;
        batch.work_input_basis_id = Some("opaque-owner-input".into());
        let operation = OwnerPublicationOperation::reconstruct("owner-rule-v1", batch).unwrap();
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
        assert_eq!(
            cut.receipts[0].work_input_basis_id.as_deref(),
            Some("opaque-owner-input")
        );
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

        let reopened_db = crate::lifecycle::test_support::reopen_sled_after_close(temp.path());
        let reopened =
            GraphRuntimeTestFixture::open(reopened_db, graph_directory.path().join("graph.agdb"))
                .unwrap();
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
        let graph_directory = tempfile::tempdir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let fixture = GraphRuntimeTestFixture::open(
            sled::open(temp.path()).unwrap(),
            graph_directory.path().join("graph.agdb"),
        )
        .unwrap();
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
        let graph_directory = tempfile::tempdir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let fixture = GraphRuntimeTestFixture::open(
            sled::open(temp.path()).unwrap(),
            graph_directory.path().join("graph.agdb"),
        )
        .unwrap();
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

    #[test]
    fn retained_sled_publications_migrate_exactly_and_new_writes_do_not_reach_sled() {
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path().join("world")).unwrap();
        let legacy = db.open_tree("traversal_owner_publications").unwrap();
        let first = super::super::contracts::ProjectedOwnerPublication {
            source_route: None,
            operation: operation("docs", "scope-a", "old", true),
            source_event: crate::events::EventRecordRef {
                ledger_id: crate::events::LedgerIdentity::new(),
                seq: u64::MAX - 1,
            },
        };
        legacy
            .insert(
                format!(
                    "{:020}::{}",
                    first.source_event.seq, first.operation.operation_id
                ),
                serde_json::to_vec(&first).unwrap(),
            )
            .unwrap();
        db.flush().unwrap();
        let path = root.path().join("graph.agdb");
        let store = TraversalStore::new(db.clone(), &path).unwrap();
        assert_eq!(
            store
                .owner_publication_for_event(first.source_event.seq)
                .unwrap(),
            Some(first.clone())
        );
        let next = super::super::contracts::ProjectedOwnerPublication {
            operation: operation("docs", "scope-a", "new", true),
            source_event: crate::events::EventRecordRef {
                seq: u64::MAX,
                ..first.source_event
            },
            ..first.clone()
        };
        store.put_owner_publication(&next).unwrap();
        store.put_owner_publication(&next).unwrap();
        store.flush().unwrap();
        assert_eq!(legacy.len(), 1);
        drop(store);
        let reopened = TraversalStore::new(db, &path).unwrap();
        assert_eq!(
            reopened.owner_publications_through_seq(u64::MAX).unwrap(),
            vec![first.clone(), next.clone()]
        );
        let requirement = cut_request(
            "docs",
            "scope-a",
            crate::events::LedgerCursor {
                ledger_id: first.source_event.ledger_id,
                after_seq: u64::MAX,
            },
        )
        .owners
        .remove(0);
        assert_eq!(
            reopened
                .publications
                .select(&requirement, u64::MAX - 1)
                .unwrap()
                .unwrap()
                .source_event,
            first.source_event
        );
        assert_eq!(
            reopened
                .publications
                .select(&requirement, u64::MAX)
                .unwrap()
                .unwrap()
                .source_event,
            next.source_event
        );
        drop(reopened);
        drop(legacy);
        std::fs::remove_file(&path).unwrap();
        let db =
            crate::lifecycle::test_support::reopen_sled_after_close(&root.path().join("world"));
        assert!(TraversalStore::new(db, &path)
            .err()
            .unwrap()
            .to_string()
            .contains("explicit rebuild"));
    }

    #[test]
    fn graph_aliases_distinguish_structured_addresses_with_delimiters() {
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path().join("world")).unwrap();
        let store = TraversalStore::new(db, root.path().join("graph.agdb")).unwrap();
        let mut batch = operation("docs", "scope-a", "r1", true).batch;
        let mut left = batch.objects[0].object_ref.clone();
        let mut right = left.clone();
        left.object_kind = "a::b".into();
        left.object_id = "c".into();
        right.object_kind = "a".into();
        right.object_id = "b::c".into();
        batch.objects[0].object_ref = left.clone();
        batch.objects[1].object_ref = right.clone();
        for relation in &mut batch.relations {
            relation.src = left.clone();
            relation.dst = right.clone();
        }
        let publication = super::super::contracts::ProjectedOwnerPublication {
            source_route: None,
            operation: OwnerPublicationOperation::reconstruct("owner-rule-v1", batch).unwrap(),
            source_event: crate::events::EventRecordRef {
                ledger_id: crate::events::LedgerIdentity::new(),
                seq: 1,
            },
        };
        store.put_owner_publication(&publication).unwrap();
        assert_eq!(store.publications.objects(1, &left).unwrap().len(), 1);
        assert_eq!(store.publications.objects(1, &right).unwrap().len(), 1);
        assert_eq!(
            store.publications.relations(1, &left, false).unwrap().len(),
            publication.operation.batch.relations.len()
        );
        assert!(store
            .publications
            .relations(1, &right, false)
            .unwrap()
            .is_empty());
        assert_eq!(
            store.owner_publication_for_event(1).unwrap(),
            Some(publication)
        );
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
                work_input_basis_id: None,
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
