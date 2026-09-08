//! Federated readers traverse the same owner publications while preserving each ledger cut.
use crate::integration::test_utils::open_authority_progress;
use crate::integration::with_xdg_data_home;
use meld::branches::{BranchQueryRuntime, BranchQueryScope, BranchRuntime};
use meld_events::{DomainObjectRef, LedgerCursor};
use meld_world_model::graph::contracts::*;
use meld_world_model::graph::events::owner_publication_envelope;
use std::collections::BTreeMap;
use tempfile::TempDir;

fn scope() -> OwnerPublicationScope {
    OwnerPublicationScope {
        scope_id: "sample-scope".into(),
        branch_id: None,
        perspective_id: None,
        valid_at: None,
    }
}
fn object(id: &str) -> DomainObjectRef {
    DomainObjectRef::new("sample", "object", id).unwrap()
}
fn request() -> BoundedTraversalRequest {
    BoundedTraversalRequest {
        roots: vec![object("a")],
        direction: TraversalDirection::Outgoing,
        relation_types: None,
        bounds: TraversalBounds {
            max_depth: 1,
            max_objects: 8,
            max_occurrences: 8,
            max_paths: 8,
        },
    }
}
fn load_graph(path: &std::path::Path, revision: &str) -> LedgerCursor {
    let db = sled::open(path).unwrap();
    let fixture = open_authority_progress(db.clone());
    let graph = fixture.graph_runtime(db);
    let hydration = HydrationReference {
        owner_id: "sample".into(),
        product_kind: "sample".into(),
        product_id: "source".into(),
        revision_id: revision.into(),
        role: "source".into(),
    };
    let operation = OwnerPublicationOperation::reconstruct(
        "sample-rule",
        OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: "sample".into(),
            revision_id: revision.into(),
            scope: scope(),
            objects: ["a", "b"]
                .into_iter()
                .map(|id| OwnerObjectPublication {
                    publication_id: id.into(),
                    object_ref: object(id),
                    state: OwnerPublicationState::Observed,
                    source_product_ref: revision.into(),
                    hydration: hydration.clone(),
                    provenance_refs: vec![revision.into()],
                    qualifications: BTreeMap::new(),
                })
                .collect(),
            relations: vec![OwnerRelationOccurrence {
                occurrence_id: "a-to-b".into(),
                relation_type: "related".into(),
                src: object("a"),
                dst: object("b"),
                source_product_ref: revision.into(),
                hydration,
                provenance_refs: vec![revision.into()],
                qualifications: BTreeMap::new(),
            }],
            completeness: OwnerCompletenessReceipt {
                receipt_id: revision.into(),
                scope: scope(),
                included_ids: vec!["a".into(), "b".into(), "a-to-b".into()],
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .unwrap();
    fixture
        .progress
        .emit_envelope_idempotent(owner_publication_envelope("test", &operation).unwrap())
        .unwrap();
    graph.catch_up().unwrap();
    graph.durable_event_cursor().unwrap()
}
fn attach(workspace: &TempDir) -> meld::branches::ResolvedBranch {
    let runtime = BranchRuntime::new();
    let resolved = runtime.resolve_active_branch(workspace.path()).unwrap();
    runtime.attach_branch(workspace.path()).unwrap();
    resolved.resolved().clone()
}

#[test]
fn federated_owner_walk_retains_distinct_branches_and_authority_positions() {
    let env = TempDir::new().unwrap();
    let first_workspace = TempDir::new().unwrap();
    let second_workspace = TempDir::new().unwrap();
    with_xdg_data_home(&env, || {
        let first = attach(&first_workspace);
        let second = attach(&second_workspace);
        let first_position = load_graph(
            &meld::branches::locator::branch_store_path(&first.data_home_path),
            "first-revision",
        );
        let second_position = load_graph(
            &meld::branches::locator::branch_store_path(&second.data_home_path),
            "second-revision",
        );
        assert_ne!(first_position.ledger_id, second_position.ledger_id);
        let runtime = BranchQueryRuntime::new();
        let result = runtime
            .owner_walk(
                BranchQueryScope::All,
                None,
                first_position,
                "sample",
                scope(),
                &request(),
            )
            .unwrap();
        assert_eq!(result.branches.len(), 2);
        for (branch, position, revision) in [
            (&first, first_position, "first-revision"),
            (&second, second_position, "second-revision"),
        ] {
            let read = result
                .branches
                .iter()
                .find(|read| read.branch_id == branch.branch_id)
                .unwrap();
            assert_eq!(read.cut.event_position, position);
            assert_eq!(read.cut.receipts[0].revision_id, revision);
            assert_eq!(read.result.objects.len(), 2);
            assert_eq!(read.result.occurrences.len(), 1);
            let single = runtime
                .owner_walk(
                    BranchQueryScope::BranchIds(vec![branch.branch_id.clone()]),
                    None,
                    position,
                    "sample",
                    scope(),
                    &request(),
                )
                .unwrap();
            assert_eq!(&single.branches[0], read);
        }
        assert_eq!(
            runtime
                .owner_walk(
                    BranchQueryScope::All,
                    None,
                    first_position,
                    "sample",
                    scope(),
                    &request()
                )
                .unwrap(),
            result
        );
    });
}

#[test]
fn federated_owner_walk_reports_unreadable_branches_and_strict_reads_fail() {
    let env = TempDir::new().unwrap();
    let healthy_workspace = TempDir::new().unwrap();
    let missing_workspace = TempDir::new().unwrap();
    with_xdg_data_home(&env, || {
        let healthy = attach(&healthy_workspace);
        let missing = attach(&missing_workspace);
        let position = load_graph(
            &meld::branches::locator::branch_store_path(&healthy.data_home_path),
            "healthy-revision",
        );
        let runtime = BranchQueryRuntime::new();
        let status = runtime.graph_status(BranchQueryScope::All, None).unwrap();
        let healthy_status = status
            .branches
            .iter()
            .find(|row| row.branch_id == healthy.branch_id)
            .unwrap();
        assert_eq!(healthy_status.last_reduced_seq, Some(position.after_seq));
        let result = runtime
            .owner_walk(
                BranchQueryScope::All,
                None,
                position,
                "sample",
                scope(),
                &request(),
            )
            .unwrap();
        assert_eq!(result.metadata.readable_branch_ids, vec![healthy.branch_id]);
        assert_eq!(
            result.metadata.skipped_branches[0].branch_id,
            missing.branch_id
        );
        assert!(runtime
            .owner_walk(
                BranchQueryScope::BranchIds(vec![missing.branch_id]),
                None,
                position,
                "sample",
                scope(),
                &request()
            )
            .is_err());
    });
}
