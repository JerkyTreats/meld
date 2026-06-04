#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::task_network::{
    dispatch::Claim,
    journal::JournalRecord,
    mutation::{CommitRecord, Set},
    outcome::Publication,
    state::NetworkState,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(record) = serde_json::from_slice::<Set>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<CommitRecord>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<NetworkState>(data) {
        let _ = record.recompute_state_hash();
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<Claim>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<Publication>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<JournalRecord>(data) {
        let _ = serde_json::to_vec(&record);
    }
});
