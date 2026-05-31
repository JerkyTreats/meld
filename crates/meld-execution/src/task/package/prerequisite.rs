//! Task package prerequisite authoring contracts.

use serde::{Deserialize, Serialize};

/// Declarative prerequisite rule applied over related repeated regions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrerequisiteTemplateSpec {
    /// Producing turn identifier in a traversal prerequisite.
    pub producer_turn_id: String,
    /// Producing stage identifier in a traversal prerequisite.
    pub producer_stage_id: String,
    /// Producing output slot identifier in a traversal prerequisite.
    pub producer_output_slot_id: String,
    /// Producing artifact type identifier in a traversal prerequisite.
    pub producer_artifact_type_id: String,
    /// Consuming turn identifier in a traversal prerequisite.
    pub consumer_turn_id: String,
    /// Consuming stage identifier in a traversal prerequisite.
    pub consumer_stage_id: String,
    /// Consuming input slot identifier in a traversal prerequisite.
    pub consumer_input_slot_id: String,
}
