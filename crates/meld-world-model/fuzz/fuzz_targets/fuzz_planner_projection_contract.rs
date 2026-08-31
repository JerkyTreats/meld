#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::planner::{PlannerAssemblyRequest, PlannerCut};

fuzz_target!(|data: &[u8]| {
    if let Ok(request) = serde_json::from_slice::<PlannerAssemblyRequest>(data) {
        let outcome = PlannerCut::assemble(request);
        let _ = serde_json::to_vec(&outcome);
    }
});
