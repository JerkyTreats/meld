//! Execution side contracts for goal scoped world state projection.

use serde::{Deserialize, Serialize};

/// Canonical inputs used to derive a durable projection frame id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PlanningProjectionIdentityInputsWire")]
pub struct PlanningProjectionIdentityInputs {
    /// Digest of the complete projection request.
    request_hash: String,
    /// Projection schema or algorithm version.
    projection_version: String,
    /// Perspective used for the projection.
    perspective_id: String,
    /// Branch used for the projection.
    branch_id: String,
    /// Sorted unique source references used by the projection.
    source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PlanningProjectionIdentityInputsWire {
    request_hash: String,
    projection_version: String,
    perspective_id: String,
    branch_id: String,
    source_refs: Vec<String>,
}

impl PlanningProjectionIdentityInputs {
    /// Build identity inputs from the complete canonical projection request.
    pub fn for_request(
        request: &PlanningWorldStateRequest,
        projection_version: impl Into<String>,
        mut source_refs: Vec<String>,
    ) -> Result<Self, String> {
        source_refs.sort();
        source_refs.dedup();
        let inputs = Self {
            request_hash: request.canonical_hash()?,
            projection_version: projection_version.into(),
            perspective_id: request.perspective_id.clone(),
            branch_id: request.branch_id.clone(),
            source_refs,
        };
        inputs.validate()?;
        Ok(inputs)
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

    /// Validate that identity inputs were derived from this exact request.
    pub fn validate_for_request(&self, request: &PlanningWorldStateRequest) -> Result<(), String> {
        self.validate()?;
        if self.request_hash != request.canonical_hash()?
            || self.perspective_id != request.perspective_id
            || self.branch_id != request.branch_id
        {
            return Err(
                "projection identity does not match the canonical projection request".to_string(),
            );
        }
        Ok(())
    }
}

impl TryFrom<PlanningProjectionIdentityInputsWire> for PlanningProjectionIdentityInputs {
    type Error = String;

    fn try_from(value: PlanningProjectionIdentityInputsWire) -> Result<Self, Self::Error> {
        let inputs = Self {
            request_hash: value.request_hash,
            projection_version: value.projection_version,
            perspective_id: value.perspective_id,
            branch_id: value.branch_id,
            source_refs: value.source_refs,
        };
        inputs.validate()?;
        Ok(inputs)
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

impl PlanningWorldStateRequest {
    /// Derive a canonical digest from every projection request field.
    pub fn canonical_hash(&self) -> Result<String, String> {
        let encoded = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        Ok(blake3::hash(&encoded).to_hex().to_string())
    }
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
        let request = PlanningWorldStateRequest {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: meld_lang::Proposition::Not(Box::new(meld_lang::Proposition::All(vec![]))),
            perspective_id: "agent-a".to_string(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["docs_freshness".to_string()],
            required_preconditions: Vec::new(),
        };
        let inputs = PlanningProjectionIdentityInputs::for_request(
            &request,
            "projection-v1",
            vec![
                "source-b".to_string(),
                "source-a".to_string(),
                "source-b".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(inputs.source_refs, vec!["source-a", "source-b"]);
        assert!(inputs.validate_for_request(&request).is_ok());
        let first = inputs.frame_ref(Vec::new()).unwrap();
        let second = inputs.frame_ref(Vec::new()).unwrap();
        assert_eq!(first.frame_id, second.frame_id);
        assert_eq!(first.source_refs, inputs.source_refs);
        let encoded = serde_json::to_vec(&inputs).unwrap();
        let decoded: PlanningProjectionIdentityInputs = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, inputs);
    }
}
