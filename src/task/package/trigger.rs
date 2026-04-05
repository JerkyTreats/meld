//! Task package trigger authoring contracts.

use serde::{Deserialize, Serialize};

/// Authored target selector kinds accepted by one package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelectorKind {
    NodeId,
    Path,
}

/// Declarative trigger contract for one task package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTriggerSpec {
    pub accepted_targets: Vec<TargetSelectorKind>,
    pub required_runtime_fields: Vec<String>,
}
