//! Execution side contracts for goal scoped world state projection.

use serde::{Deserialize, Serialize};

/// Request shape execution sends to a world-model projection boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningWorldStateRequest {
    /// Goal that scopes this projection request.
    pub goal_id: String,
    /// Agent perspective requesting the projection.
    pub agent_id: String,
    /// Target proposition that planning will evaluate.
    pub target: meld_lang::Proposition,
    /// World model perspective identifier.
    pub perspective_id: String,
    /// World model branch identifier.
    pub branch_id: String,
    /// Dimensions execution expects to evaluate.
    pub requested_dimensions: Vec<String>,
    /// Method preconditions that can be projected with the goal target.
    pub required_preconditions: Vec<meld_lang::Proposition>,
}

/// Provenance for the projected world state consumed by planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningWorldStateFrameRef {
    /// Durable frame id or equivalent projection record id.
    pub frame_id: String,
    /// Projection schema or algorithm version.
    pub projection_version: String,
    /// Perspective used for the projection.
    pub perspective_id: String,
    /// Branch used for the projection.
    pub branch_id: String,
    /// Source belief or frame references used to build the projection.
    pub source_refs: Vec<String>,
    /// Non-fatal projection warnings that should travel with planning output.
    pub warnings: Vec<String>,
}
