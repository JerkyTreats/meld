//! Task package repeated region authoring contracts.

use super::output::TurnOutputPolicySpec;
use serde::{Deserialize, Serialize};

/// Declarative repeated region authored by a task package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatedRegionSpec {
    /// Region identifier carried across the execution boundary.
    pub region_id: String,
    /// Initialization slot identifier that carries the force flag.
    pub force_init_slot_id: String,
    /// Template used to derive node reference input slots.
    pub node_ref_slot_template: String,
    /// Template used to derive existing output input slots.
    pub existing_output_slot_template: String,
    /// Artifact type used for existing workflow output inputs.
    pub existing_output_artifact_type_id: String,
    /// Stage chain owned by this execution contract.
    pub stage_chain: StageChainSpec,
    /// Ordered workflow turn templates in this repeated region.
    pub turns: Vec<TurnSpec>,
}

/// Declarative shared stage chain for one repeated region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageChainSpec {
    /// Stages owned by this execution contract.
    pub stages: Vec<StageSpec>,
}

/// Declarative stage inside a repeated region stage chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSpec {
    /// Stage identifier carried across the execution boundary.
    pub stage_id: String,
    /// Stable capability type identifier published by the owning domain.
    pub capability_type_id: String,
    /// Version of the published capability contract.
    pub capability_version: u32,
}

/// Declarative turn authored inside a repeated region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnSpec {
    /// Workflow turn identifier within the owning workflow profile or thread.
    pub turn_id: String,
    /// Prompt reference owned by this execution contract.
    pub prompt_ref: String,
    /// Declared output type produced by this workflow turn.
    pub output_type: String,
    /// Gate identifier carried across the execution boundary.
    pub gate_id: String,
    /// Output policy owned by this execution contract.
    pub output_policy: TurnOutputPolicySpec,
    /// Maximum attempts allowed for this turn or runtime step.
    #[serde(default = "default_retry_limit")]
    pub retry_limit: usize,
    /// True when turn output must parse as JSON.
    #[serde(default)]
    pub validate_json: bool,
}

fn default_retry_limit() -> usize {
    1
}
