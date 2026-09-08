#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use meld_world_model::events::{DomainObjectRef, LedgerCursor};
use meld_world_model::graph::events::owner_publication_envelope;
use meld_world_model::graph::test_support::GraphRuntimeTestFixture;
use meld_world_model::world_state::graph::contracts::{
    BoundedTraversalRequest, HydrationReference, OwnerCompletenessReceipt, OwnerCompletenessStatus,
    OwnerCurrentnessPolicy, OwnerObjectPublication, OwnerPublicationBatch,
    OwnerPublicationOperation, OwnerPublicationScope, OwnerPublicationState,
    OwnerRelationOccurrence, TraversalBounds, TraversalCutRequest, TraversalOwnerRequirement,
};
use meld_world_model::{TraversalDirection, TraversalQuery};

fuzz_target!(|data: &[u8]| {
    if let Ok(operation) = serde_json::from_slice::<OwnerPublicationOperation>(data) {
        let _ = operation.validate();
    }
    if data.len() < 4 {
        return;
    }

    let direction = match data[0] % 3 {
        0 => TraversalDirection::Outgoing,
        1 => TraversalDirection::Incoming,
        _ => TraversalDirection::Both,
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let fixture =
        GraphRuntimeTestFixture::open(sled::open(temp.path()).expect("sled")).expect("fixture");
    let runtime = fixture.runtime();
    let store = runtime.traversal_store();

    let owner = "fuzz_owner";
    let revision = "revision-a";
    let scope = OwnerPublicationScope {
        scope_id: "scope-a".to_string(),
        branch_id: None,
        perspective_id: None,
        valid_at: None,
    };
    let owner_a = DomainObjectRef::new(owner, "node", "a").expect("owner object");
    let owner_b = DomainObjectRef::new(owner, "node", "b").expect("owner object");
    let hydration = |product_id: &str| HydrationReference {
        owner_id: owner.to_string(),
        product_kind: "node".to_string(),
        product_id: product_id.to_string(),
        revision_id: revision.to_string(),
        role: "published_material".to_string(),
    };
    let operation = OwnerPublicationOperation::reconstruct(
        "fuzz-owner-rule-v1",
        OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: owner.to_string(),
            revision_id: revision.to_string(),
            scope: scope.clone(),
            objects: vec![
                OwnerObjectPublication {
                    publication_id: "object-a".to_string(),
                    object_ref: owner_a.clone(),
                    state: OwnerPublicationState::Observed,
                    source_product_ref: "a".to_string(),
                    hydration: hydration("a"),
                    provenance_refs: Vec::new(),
                    qualifications: BTreeMap::new(),
                },
                OwnerObjectPublication {
                    publication_id: "object-b".to_string(),
                    object_ref: owner_b.clone(),
                    state: OwnerPublicationState::Observed,
                    source_product_ref: "b".to_string(),
                    hydration: hydration("b"),
                    provenance_refs: Vec::new(),
                    qualifications: BTreeMap::new(),
                },
            ],
            relations: vec![OwnerRelationOccurrence {
                occurrence_id: "occurrence-a".to_string(),
                relation_type: "next".to_string(),
                src: owner_a.clone(),
                dst: owner_b,
                source_product_ref: "a-b".to_string(),
                hydration: hydration("a-b"),
                qualifications: BTreeMap::from([(
                    "fuzz_byte".to_string(),
                    data.get(5).copied().unwrap_or_default().to_string(),
                )]),
                provenance_refs: Vec::new(),
            }],
            completeness: OwnerCompletenessReceipt {
                receipt_id: "receipt-a".to_string(),
                scope: scope.clone(),
                included_ids: vec![
                    "object-a".to_string(),
                    "object-b".to_string(),
                    "occurrence-a".to_string(),
                ],
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .expect("owner operation");
    let receipt = fixture
        .append(owner_publication_envelope("fuzz", &operation).expect("envelope"))
        .expect("append");
    let ledger_id = receipt.ledger_id;
    runtime.catch_up().expect("projection");
    let query = TraversalQuery::new(&store);
    let cut = query
        .cut(&TraversalCutRequest {
            owners: vec![TraversalOwnerRequirement {
                event_source: None,
                owner_id: owner.to_string(),
                scope: scope.clone(),
                required: true,
            }],
            scope,
            currentness: OwnerCurrentnessPolicy::LatestComplete,
            event_position: LedgerCursor {
                ledger_id,
                after_seq: 1,
            },
        })
        .expect("cut");
    let bounded = query.traverse(
        &cut,
        &BoundedTraversalRequest {
            roots: vec![owner_a],
            direction,
            relation_types: Some(vec!["next".to_string()]),
            bounds: TraversalBounds {
                max_depth: usize::from(data[1] % 4) + 1,
                max_objects: usize::from(data[2] % 4) + 1,
                max_occurrences: usize::from(data[3] % 4) + 1,
                max_paths: usize::from(data.get(4).copied().unwrap_or_default() % 4) + 1,
            },
        },
    );
    assert!(bounded.is_ok());
});
