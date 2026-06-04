//! Task package trigger authoring contracts.

use serde::{Deserialize, Serialize};

/// Authored target selector kinds accepted by one package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelectorKind {
    /// Target selected by workspace node identifier.
    NodeId,
    /// Target selected by workspace path.
    Path,
}

/// Declarative trigger contract for one task package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTriggerSpec {
    /// Target selector kinds accepted by this package.
    pub accepted_targets: Vec<TargetSelectorKind>,
    /// Runtime field names required before the trigger can lower.
    pub required_runtime_fields: Vec<String>,
}
