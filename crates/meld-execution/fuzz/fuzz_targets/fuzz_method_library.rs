#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::{capability::CapabilityCatalog, planning::MethodLibrary};

fuzz_target!(|data: &[u8]| {
    if let Ok(method) = serde_json::from_slice::<meld_lang::Method>(data) {
        let catalog = CapabilityCatalog::new();
        let _ = MethodLibrary::from_methods(vec![method], &catalog);
    }
    if let Ok(methods) = serde_json::from_slice::<Vec<meld_lang::Method>>(data) {
        let catalog = CapabilityCatalog::new();
        let _ = MethodLibrary::from_methods(methods, &catalog);
    }
});
