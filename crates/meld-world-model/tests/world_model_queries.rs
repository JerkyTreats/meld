//! Event-to-Graph-to-Planner cutover acceptance through public runtime contracts.
use meld_world_model::belief::{BeliefKey, BeliefQuery, BeliefStore, BranchScope};
use meld_world_model::events::{DomainObjectRef, EventEnvelope, LedgerCursor};
use meld_world_model::planner::*;
use meld_world_model::world_state::graph::contracts::*;
use meld_world_model::world_state::graph::events::owner_publication_envelope;
use meld_world_model::{PerspectiveKey, TraversalQuery, WorldModelQueries};
use std::collections::BTreeMap;
mod support;
use support::GraphRuntimeTestFixture;

fn publication(revision: &str, state: OwnerPublicationState) -> OwnerPublicationOperation {
    let scope = scope();
    OwnerPublicationOperation::reconstruct(
        "owner-rule-v1",
        OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: "sample".into(),
            revision_id: revision.into(),
            scope: scope.clone(),
            objects: vec![OwnerObjectPublication {
                publication_id: "object-a".into(),
                object_ref: subject(),
                state,
                source_product_ref: format!("sample::{revision}"),
                hydration: HydrationReference {
                    owner_id: "sample".into(),
                    product_kind: "observation".into(),
                    product_id: "object-a".into(),
                    revision_id: revision.into(),
                    role: "source".into(),
                },
                provenance_refs: vec![format!("sample::{revision}")],
                qualifications: BTreeMap::new(),
            }],
            relations: Vec::new(),
            completeness: OwnerCompletenessReceipt {
                receipt_id: revision.into(),
                scope,
                included_ids: vec!["object-a".into()],
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .unwrap()
}
fn subject() -> DomainObjectRef {
    DomainObjectRef::new("sample", "object", "a").unwrap()
}
fn scope() -> OwnerPublicationScope {
    OwnerPublicationScope {
        scope_id: "scope-a".into(),
        branch_id: Some("main".into()),
        perspective_id: Some("default".into()),
        valid_at: None,
    }
}
fn cut_request(position: LedgerCursor) -> TraversalCutRequest {
    TraversalCutRequest {
        owners: vec![TraversalOwnerRequirement {
            event_source: None,
            owner_id: "sample".into(),
            scope: scope(),
            required: true,
        }],
        scope: scope(),
        currentness: OwnerCurrentnessPolicy::LatestComplete,
        event_position: position,
    }
}
fn walk() -> BoundedTraversalRequest {
    BoundedTraversalRequest {
        roots: vec![subject()],
        direction: TraversalDirection::Both,
        relation_types: None,
        bounds: TraversalBounds {
            max_depth: 1,
            max_objects: 8,
            max_occurrences: 8,
            max_paths: 8,
        },
    }
}
fn planner_request(position: LedgerCursor) -> PlannerCurrentAssemblyRequest {
    PlannerCurrentAssemblyRequest {
        required_derived_evidence: None,
        required_graph_evidence: Vec::new(),
        additional_beliefs: Vec::new(),
        context: PlannerDecisionContext {
            context_id: "context-a".into(),
            agent_id: "agent-a".into(),
            goal_id: "goal-a".into(),
            subject: subject(),
            observation_subject: None,
            scope_id: "scope-a".into(),
            branch_id: "main".into(),
            perspective_id: "default".into(),
            authority_scope_id: "authority-a".into(),
            activation_generation: "generation-a".into(),
            admission_epoch: Some("epoch-a".into()),
        },
        policy: PlannerAssemblyPolicy {
            acquisition_question: None,
            policy_revision_id: "policy-a".into(),
            required_sources: vec![PlannerSourceKind::Graph],
            explicitly_not_required: vec![PlannerSourceKind::Causation, PlannerSourceKind::Regime],
        },
        traversal_cut_request: cut_request(position),
        traversal_request: walk(),
        belief_key: BeliefKey {
            subject: subject(),
            dimension_id: "quality".into(),
            predicate_id: "confidence".into(),
            perspective: PerspectiveKey::new("default", "default").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "evidence-a".into(),
        },
        source_positions: Vec::new(),
    }
}

#[test]
fn facade_and_native_planner_share_owner_publication_currentness() {
    let fixture =
        GraphRuntimeTestFixture::open(sled::Config::new().temporary(true).open().unwrap()).unwrap();
    let runtime = fixture.runtime();
    let queries = WorldModelQueries::new(runtime.clone());
    let first = fixture
        .append(
            owner_publication_envelope("test", &publication("r1", OwnerPublicationState::Observed))
                .unwrap(),
        )
        .unwrap();
    let first_position = LedgerCursor {
        ledger_id: first.ledger_id,
        after_seq: first.seq,
    };
    let first_cut = queries.cut(&cut_request(first_position)).unwrap();
    assert_eq!(first_cut.status, TraversalCutStatus::Complete);
    let result = queries.traverse(&first_cut, &walk()).unwrap();
    let store = runtime.traversal_store();
    let belief = BeliefStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
    let planner = PlannerQuery::new(BeliefQuery::new(&belief), TraversalQuery::new(&store));
    let PlannerAssemblyOutcome::Complete(first_plan) =
        planner.assemble_current(planner_request(first_position))
    else {
        panic!("owner observation must assemble")
    };
    assert_eq!(first_plan.traversal_result, result);
    assert!(first_plan
        .world_model_view
        .world_state
        .propositions()
        .iter()
        .any(|value| matches!(value, meld_lang::Proposition::Accessible { .. })));
    assert!(first_plan
        .world_model_view
        .hydration_refs
        .graph_anchor_ids
        .is_empty());

    let withdrawn = fixture
        .append(
            owner_publication_envelope(
                "test",
                &publication("r2", OwnerPublicationState::Withdrawn),
            )
            .unwrap(),
        )
        .unwrap();
    let next_position = LedgerCursor {
        ledger_id: withdrawn.ledger_id,
        after_seq: withdrawn.seq,
    };
    let next_cut = queries.cut(&cut_request(next_position)).unwrap();
    let next = queries.traverse(&next_cut, &walk()).unwrap();
    assert!(next
        .objects
        .iter()
        .all(|object| object.state != OwnerPublicationState::Observed));
    let PlannerAssemblyOutcome::Complete(next_plan) =
        planner.assemble_current(planner_request(next_position))
    else {
        panic!("withdrawal still has a complete owner account")
    };
    assert_eq!(next_plan.traversal_result, next);
    assert!(!next_plan
        .world_model_view
        .world_state
        .propositions()
        .iter()
        .any(|value| matches!(value, meld_lang::Proposition::Accessible { .. })));
    assert_eq!(queries.traverse(&first_cut, &walk()).unwrap(), result);
    assert_eq!(
        fixture.records().unwrap().len(),
        2,
        "Graph must not append semantic events"
    );
}

#[test]
fn raw_foreign_event_hints_cannot_create_owner_knowledge() {
    let fixture =
        GraphRuntimeTestFixture::open(sled::Config::new().temporary(true).open().unwrap()).unwrap();
    let record = fixture
        .append(
            EventEnvelope::with_now_domain(
                "test",
                "sample",
                "scope-a",
                "context.head_selected",
                None,
                serde_json::json!({}),
            )
            .with_graph(vec![subject()], Vec::new()),
        )
        .unwrap();
    let queries = WorldModelQueries::new(fixture.runtime());
    let cut = queries
        .cut(&cut_request(LedgerCursor {
            ledger_id: record.ledger_id,
            after_seq: record.seq,
        }))
        .unwrap();
    assert_eq!(cut.status, TraversalCutStatus::Incomplete);
    assert!(cut.receipts.is_empty());
    assert_eq!(fixture.records().unwrap().len(), 1);
}
