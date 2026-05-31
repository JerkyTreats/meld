//! Task package trigger authoring contracts.

use serde::{Deserialize, Serialize};

/// Authored target selector kinds accepted by one package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelectorKind {
    /// Node identifier variant for this execution contract.
    NodeId,
    /// Path variant for this execution contract.
    Path,
}

/// Declarative trigger contract for one task package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTriggerSpec {
    /// Accepted targets owned by this execution contract.
    pub accepted_targets: Vec<TargetSelectorKind>,
    /// Required runtime fields owned by this execution contract.
    pub required_runtime_fields: Vec<String>,
}
