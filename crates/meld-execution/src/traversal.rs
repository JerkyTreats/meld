use crate::execution::contracts::ProviderExecutionBinding;
use crate::publish::FrameHeadPublishTemplate;
use crate::workflow::profile::WorkflowGate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalExpansionNode {
    pub node_id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalExpansionRelation {
    pub upstream_node_id: String,
    pub downstream_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRegionTemplate {
    pub workflow_id: String,
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub force_init_slot_id: String,
    pub node_ref_slot_template: String,
    pub existing_output_slot_template: String,
    pub existing_output_artifact_type_id: String,
    pub turns: Vec<WorkflowTurnTemplate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTurnTemplate {
    pub turn_id: String,
    pub prompt_text: String,
    pub output_type: String,
    pub gate: WorkflowGate,
    pub persist_frame: bool,
    pub retry_limit: usize,
    pub validate_json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteTemplate {
    pub producer_turn_id: String,
    pub producer_stage_id: String,
    pub producer_output_slot_id: String,
    pub producer_artifact_type_id: String,
    pub consumer_turn_id: String,
    pub consumer_stage_id: String,
    pub consumer_input_slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteExpansionTemplate {
    pub repeated_region: WorkflowRegionTemplate,
    pub prerequisite_template: TraversalPrerequisiteTemplate,
    pub publish: Option<FrameHeadPublishTemplate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteExpansionContent {
    pub traversal_strategy: String,
    pub node_batches: Vec<Vec<TraversalExpansionNode>>,
    pub relations: Vec<TraversalExpansionRelation>,
    pub repeated_region: WorkflowRegionTemplate,
    pub prerequisite_template: TraversalPrerequisiteTemplate,
    pub publish: Option<FrameHeadPublishTemplate>,
}
