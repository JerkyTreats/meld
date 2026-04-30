//! Task package prerequisite authoring contracts.

use serde::{Deserialize, Serialize};

/// Declarative prerequisite rule applied over related repeated regions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrerequisiteTemplateSpec {
    pub producer_turn_id: String,
    pub producer_stage_id: String,
    pub producer_output_slot_id: String,
    pub producer_artifact_type_id: String,
    pub consumer_turn_id: String,
    pub consumer_stage_id: String,
    pub consumer_input_slot_id: String,
}
