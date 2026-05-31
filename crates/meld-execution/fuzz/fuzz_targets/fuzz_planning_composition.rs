#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_lang::{substitute, validate, Bindings, Composition};

fuzz_target!(|data: &[u8]| {
    if let Ok(composition) = serde_json::from_slice::<Composition>(data) {
        let _ = validate(&composition);
        let _ = substitute(&composition, &Bindings::empty());
    }
});
