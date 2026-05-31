#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::planning::{PlanningDiagnostic, PlanningRequest, PlanningResult};

fuzz_target!(|data: &[u8]| {
    if let Ok(request) = serde_json::from_slice::<PlanningRequest>(data) {
        let _ = serde_json::to_vec(&request);
    }
    if let Ok(result) = serde_json::from_slice::<PlanningResult>(data) {
        let _ = serde_json::to_vec(&result);
    }
    if let Ok(diagnostic) = serde_json::from_slice::<PlanningDiagnostic>(data) {
        let _ = serde_json::to_vec(&diagnostic);
    }
});
