//! Shared task network identity and hash helpers.
//!
//! Owner: task network.
//! Inputs: serializable task network records.
//! Outputs: stable ids and content hashes.
//! Does not own: this module does not validate graph semantics or run tasks.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::contracts::stable_id;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Identity<'a> {
//!     name: &'a str,
//! }
//!
//! let id = stable_id("record", &Identity { name: "alpha" });
//! assert!(id.starts_with("record-"));
//! ```

use serde::Serialize;

/// Current schema version for first-slice task network records.
pub const TASK_NETWORK_SCHEMA_VERSION: u32 = 1;

/// Builds a stable prefixed id from canonical JSON bytes.
pub fn stable_id(prefix: &str, value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("task network identity input is serializable");
    format!("{}-{}", prefix, blake3::hash(&bytes).to_hex())
}

/// Builds a stable content hash from canonical JSON bytes.
pub fn stable_hash(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("task network hash input is serializable");
    blake3::hash(&bytes).to_hex().to_string()
}

/// Returns true when a command boundary id is not empty after trimming.
pub fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_text_requires_non_whitespace_content() {
        assert!(has_text("alpha"));
        assert!(has_text("  alpha  "));
        assert!(!has_text(""));
        assert!(!has_text(" \n\t "));
    }
}
