//! Task package repeated region authoring contracts.

use crate::task::package::TurnOutputPolicySpec;
use serde::{Deserialize, Serialize};

/// Declarative repeated region authored by a task package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatedRegionSpec {
    pub region_id: String,
    pub force_init_slot_id: String,
    pub node_ref_slot_template: String,
    pub existing_output_slot_template: String,
    pub existing_output_artifact_type_id: String,
    pub stage_chain: StageChainSpec,
    pub turns: Vec<TurnSpec>,
}

/// Declarative shared stage chain for one repeated region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageChainSpec {
    pub stages: Vec<StageSpec>,
}

/// Declarative stage inside a repeated region stage chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSpec {
    pub stage_id: String,
    pub capability_type_id: String,
    pub capability_version: u32,
}

/// Declarative turn authored inside a repeated region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnSpec {
    pub turn_id: String,
    pub prompt_ref: String,
    pub output_type: String,
    pub gate_id: String,
    pub output_policy: TurnOutputPolicySpec,
}
