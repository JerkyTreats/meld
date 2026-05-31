#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::{
    capability::CapabilityCatalog,
    planning::{MethodLibrary, PlanningRequest, PlanningRuntime},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct RuntimeInput {
    request: PlanningRequest,
    methods: Vec<meld_lang::Method>,
}

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = serde_json::from_slice::<RuntimeInput>(data) {
        let catalog = CapabilityCatalog::new();
        let library = MethodLibrary::from_methods(input.methods, &catalog);
        let runtime = PlanningRuntime::new(library, catalog);
        let _ = runtime.plan_goal(input.request);
    }
});
