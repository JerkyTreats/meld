#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use meld_world_model::events::{
    DomainObjectRef, EventRecordRef, EventRelation, LedgerCursor, LedgerIdentity,
};
use meld_world_model::world_state::graph::contracts::{
    BoundedTraversalRequest, HydrationReference, OwnerCompletenessReceipt,
    OwnerCompletenessStatus, OwnerCurrentnessPolicy, OwnerObjectPublication,
    OwnerPublicationBatch, OwnerPublicationOperation, OwnerPublicationScope,
    OwnerPublicationState, OwnerRelationOccurrence, ProjectedOwnerPublication, TraversalBounds,
    TraversalCutRequest, TraversalOwnerRequirement,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{GraphWalkSpec, TraversalDirection, TraversalFactRecord, TraversalQuery};

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
    let max_depth = usize::from(data[1] % 4);
    let current_only = data[2] & 1 == 1;
    let include_facts = data[3] & 1 == 1;
    let relation_types = if data.get(4).copied().unwrap_or_default() & 1 == 1 {
        Some(vec!["next".to_string()])
    } else {
        None
    };

    let temp = tempfile::tempdir().expect("tempdir");
    let store = TraversalStore::new(sled::open(temp.path()).expect("sled")).expect("store");
    let a = DomainObjectRef::new("workspace_fs", "node", "a").expect("object");
    let b = DomainObjectRef::new("workspace_fs", "node", "b").expect("object");
    store
        .put_fact(&TraversalFactRecord {
            fact_id: "fact-a".to_string(),
            source_spine_fact_id: "spine::1".to_string(),
            seq: 1,
            event_type: "fuzz".to_string(),
            objects: vec![a.clone(), b.clone()],
            relations: vec![EventRelation::new("next", a.clone(), b).expect("relation")],
        })
        .expect("fact");

    let spec = GraphWalkSpec {
        direction,
        relation_types,
        max_depth,
        current_only,
        include_facts,
    };
    let result = TraversalQuery::new(&store).walk(&a, &spec);
    if max_depth == 0 {
        assert!(result.is_err());
    } else {
        assert!(result.is_ok());
    }

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
    let ledger_id = LedgerIdentity::new();
    store
        .put_owner_publication(&ProjectedOwnerPublication {
            operation,
            source_event: EventRecordRef { ledger_id, seq: 1 },
        })
        .expect("owner publication");
    store
        .db()
        .open_tree("traversal_runtime_meta")
        .expect("runtime meta")
        .insert(
            b"event_authority_cursor",
            serde_json::to_vec(&LedgerCursor {
                ledger_id,
                after_seq: 1,
            })
            .expect("cursor encoding"),
        )
        .expect("cursor write");
    store.flush().expect("flush");
    let query = TraversalQuery::new(&store);
    let cut = query
        .cut(&TraversalCutRequest {
            owners: vec![TraversalOwnerRequirement {
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
