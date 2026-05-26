#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::events::{DomainObjectRef, EventRelation};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{GraphWalkSpec, TraversalDirection, TraversalFactRecord, TraversalQuery};

fuzz_target!(|data: &[u8]| {
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
});
