//! Traversal expansion DTOs used by workflow-backed task packages.

use crate::execution::contracts::ProviderExecutionBinding;
use crate::publish::FrameHeadPublishTemplate;
use crate::workflow::profile::WorkflowGate;
use serde::{Deserialize, Serialize};

/// Workspace node selected by traversal expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalExpansionNode {
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: String,
    /// Workspace path associated with this execution record.
    pub path: String,
}

/// Directed relation between two traversal expansion nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalExpansionRelation {
    /// Upstream workspace node identifier in a traversal relation.
    pub upstream_node_id: String,
    /// Downstream workspace node identifier in a traversal relation.
    pub downstream_node_id: String,
}

/// Workflow region template repeated across traversal batches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRegionTemplate {
    /// Workflow profile identifier that owns this execution record.
    pub workflow_id: String,
    /// Agent identifier responsible for this execution request.
    pub agent_id: String,
    /// Provider binding selected for execution.
    pub provider: ProviderExecutionBinding,
    /// Context frame type produced or consumed by this execution path.
    pub frame_type: String,
    /// True when execution should bypass cached or existing output.
    pub force: bool,
    /// Initialization slot identifier that carries the force flag.
    pub force_init_slot_id: String,
    /// Template used to derive node reference input slots.
    pub node_ref_slot_template: String,
    /// Template used to derive existing output input slots.
    pub existing_output_slot_template: String,
    /// Artifact type used for existing workflow output inputs.
    pub existing_output_artifact_type_id: String,
    /// Init slot carrying the belief context bundle for the trigger target.
    /// Present only when the workflow's `belief_context` flag was on at
    /// lowering time; absence keeps templates byte-identical to prior runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belief_init_slot_id: Option<String>,
    /// Ordered workflow turn templates in this repeated region.
    pub turns: Vec<WorkflowTurnTemplate>,
}

/// Workflow turn template materialized inside a repeated region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTurnTemplate {
    /// Workflow turn identifier within the owning workflow profile or thread.
    pub turn_id: String,
    /// Prompt text used by the workflow turn template.
    pub prompt_text: String,
    /// Declared output type produced by this workflow turn.
    pub output_type: String,
    /// Gate contract applied to this workflow turn template.
    pub gate: WorkflowGate,
    /// True when turn output should be persisted as a frame.
    pub persist_frame: bool,
    /// Maximum attempts allowed for this turn or runtime step.
    pub retry_limit: usize,
    /// True when turn output must parse as JSON.
    pub validate_json: bool,
}

/// Template that wires producer outputs to consumer inputs across traversal batches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteTemplate {
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

/// Authoring template for a traversal prerequisite expansion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteExpansionTemplate {
    /// Workflow region template expanded for each traversal batch.
    pub repeated_region: WorkflowRegionTemplate,
    /// Prerequisite wiring template applied across traversal batches.
    pub prerequisite_template: TraversalPrerequisiteTemplate,
    /// Optional frame head publish policy carried with the expansion.
    pub publish: Option<FrameHeadPublishTemplate>,
}

/// Compiled traversal prerequisite expansion content emitted by task expansion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteExpansionContent {
    /// Traversal strategy name used to produce node batches.
    pub traversal_strategy: String,
    /// Ordered traversal batches of workspace nodes.
    pub node_batches: Vec<Vec<TraversalExpansionNode>>,
    /// Traversal relations between expanded workspace nodes.
    pub relations: Vec<TraversalExpansionRelation>,
    /// Workflow region template expanded for each traversal batch.
    pub repeated_region: WorkflowRegionTemplate,
    /// Prerequisite wiring template applied across traversal batches.
    pub prerequisite_template: TraversalPrerequisiteTemplate,
    /// Optional frame head publish policy carried with the expansion.
    pub publish: Option<FrameHeadPublishTemplate>,
}
