#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::planner::{project_world_state, PlannerProjectionInput};

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = serde_json::from_slice::<PlannerProjectionInput>(data) {
        let _ = project_world_state(input);
    }
});
