#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::belief::BeliefConfigLoader;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = BeliefConfigLoader::load_json(input);
    }
});
