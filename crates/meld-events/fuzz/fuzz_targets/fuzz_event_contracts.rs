#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_events::{DomainObjectRef, EventRecord, EventRelation};

fuzz_target!(|data: &[u8]| {
    if let Ok(record) = serde_json::from_slice::<EventRecord>(data) {
        let normalized = record.normalize_legacy_defaults();
        for object in &normalized.objects {
            let _ = object.validate();
            let _ = object.index_key();
        }
        for relation in &normalized.relations {
            let _ = relation.validate();
        }
        let encoded = serde_json::to_vec(&normalized).expect("record encode");
        let decoded: EventRecord = serde_json::from_slice(&encoded).expect("record decode");
        assert_eq!(decoded, normalized);
    }

    if let Ok(object) = serde_json::from_slice::<DomainObjectRef>(data) {
        let _ = object.validate();
        let _ = object.index_key();
    }

    if let Ok(relation) = serde_json::from_slice::<EventRelation>(data) {
        let _ = relation.validate();
    }
});
