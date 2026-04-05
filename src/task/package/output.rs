//! Task package output policy contracts.

use serde::{Deserialize, Serialize};

/// Output policy for one authored turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnOutputPolicySpec {
    pub persist_frame: bool,
}
