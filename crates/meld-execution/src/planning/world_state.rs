//! Execution side contracts for goal scoped world state projection.

use serde::{Deserialize, Serialize};

/// Canonical inputs used to derive a durable projection frame id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningProjectionIdentityInputs {
    /// Digest of the complete projection request.
    pub request_hash: String,
    /// Projection schema or algorithm version.
    pub projection_version: String,
    /// Perspective used for the projection.
    pub perspective_id: String,
    /// Branch used for the projection.
    pub branch_id: String,
    /// Sorted unique source references used by the projection.
    pub source_refs: Vec<String>,
}

impl PlanningProjectionIdentityInputs {
    /// Build identity inputs with canonical source ordering.
    pub fn new(
        request_hash: impl Into<String>,
        projection_version: impl Into<String>,
        perspective_id: impl Into<String>,
        branch_id: impl Into<String>,
        mut source_refs: Vec<String>,
    ) -> Self {
        source_refs.sort();
        source_refs.dedup();
        Self {
            request_hash: request_hash.into(),
            projection_version: projection_version.into(),
            perspective_id: perspective_id.into(),
            branch_id: branch_id.into(),
            source_refs,
        }
    }

    /// Validate canonical ordering and required identity components.
    pub fn validate(&self) -> Result<(), String> {
        let mut canonical_sources = self.source_refs.clone();
        canonical_sources.sort();
        canonical_sources.dedup();
        if canonical_sources != self.source_refs {
            return Err("projection identity source refs must be sorted and unique".to_string());
        }
        if self.request_hash.trim().is_empty()
            || self.projection_version.trim().is_empty()
            || self.perspective_id.trim().is_empty()
            || self.branch_id.trim().is_empty()
        {
            return Err("projection identity components must be non-empty".to_string());
        }
        Ok(())
    }

    /// Derive the durable projection frame id from canonical inputs.
    pub fn derive_frame_id(&self) -> Result<String, String> {
        self.validate()?;
        let encoded = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        Ok(blake3::hash(&encoded).to_hex().to_string())
    }

    /// Build the only frame reference valid for these identity inputs.
    pub fn frame_ref(&self, warnings: Vec<String>) -> Result<PlanningWorldStateFrameRef, String> {
        Ok(PlanningWorldStateFrameRef {
            frame_id: self.derive_frame_id()?,
            projection_version: self.projection_version.clone(),
            perspective_id: self.perspective_id.clone(),
            branch_id: self.branch_id.clone(),
            source_refs: self.source_refs.clone(),
            warnings,
        })
    }
}

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

#[cfg(test)]
mod contract_freeze_tests {
    use super::*;

    #[test]
    fn projection_identity_inputs_canonicalize_source_order() {
        let inputs = PlanningProjectionIdentityInputs::new(
            "request-a",
            "projection-v1",
            "agent-a",
            "main",
            vec![
                "source-b".to_string(),
                "source-a".to_string(),
                "source-b".to_string(),
            ],
        );

        assert_eq!(inputs.source_refs, vec!["source-a", "source-b"]);
        let first = inputs.frame_ref(Vec::new()).unwrap();
        let second = inputs.frame_ref(Vec::new()).unwrap();
        assert_eq!(first.frame_id, second.frame_id);
        assert_eq!(first.source_refs, inputs.source_refs);
        let encoded = serde_json::to_vec(&inputs).unwrap();
        let decoded: PlanningProjectionIdentityInputs = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, inputs);
    }
}
