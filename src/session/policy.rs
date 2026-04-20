//! Session policy: status and prune policy.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Interrupted,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PrunePolicy {
    pub max_completed: usize,
    pub max_age_ms: u64,
}

impl Default for PrunePolicy {
    fn default() -> Self {
        Self {
            max_completed: 500,
            max_age_ms: 1000 * 60 * 60 * 24 * 14,
        }
    }
}
