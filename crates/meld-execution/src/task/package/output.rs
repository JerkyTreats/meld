//! Task package output policy contracts.

use serde::{Deserialize, Serialize};

/// Output policy for one authored turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnOutputPolicySpec {
    /// True when turn output should be persisted as a frame.
    pub persist_frame: bool,
}
