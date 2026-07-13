//! Execution side contracts for goal scoped world state projection.

use serde::{Deserialize, Serialize};

const WORLD_STATE_HASH_DOMAIN: &[u8] = b"meld.planner-world-state.v1";

/// Canonical world-model frame identity retained for planning replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PlanningProjectionIdentityInputsWire")]
pub struct PlanningProjectionIdentityInputs {
    /// Durable frame id assigned by the world-model projection authority.
    frame_id: String,
    /// Durable world-model request completed by the frame.
    request_id: String,
    /// Digest of the complete execution projection request.
    source_request_hash: String,
    /// Projection schema or algorithm version.
    projection_version: String,
    /// World-model digest of the complete projection output.
    projection_hash: String,
    /// Digest of the exact world state supplied to execution.
    world_state_hash: String,
    /// Perspective family used for the projection.
    perspective_kind: String,
    /// Perspective member used for the projection.
    perspective_id: String,
    /// Branch used for the projection.
    branch_id: String,
    /// Sorted unique source references used by the projection.
    source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PlanningProjectionIdentityInputsWire {
    frame_id: String,
    request_id: String,
    source_request_hash: String,
    projection_version: String,
    projection_hash: String,
    world_state_hash: String,
    perspective_kind: String,
    perspective_id: String,
    branch_id: String,
    source_refs: Vec<String>,
}

impl PlanningProjectionIdentityInputs {
    /// Bind the exact world-model frame identity to its execution request and state.
    pub fn from_projection(
        request: &PlanningWorldStateRequest,
        world_state: &meld_lang::WorldState,
        frame: &PlanningWorldStateFrameRef,
    ) -> Result<Self, String> {
        let inputs = Self {
            frame_id: frame.frame_id.clone(),
            request_id: frame.request_id.clone(),
            source_request_hash: frame.source_request_hash.clone(),
            projection_version: frame.projection_version.clone(),
            projection_hash: frame.projection_hash.clone(),
            world_state_hash: frame.world_state_hash.clone(),
            perspective_kind: frame.perspective_kind.clone(),
            perspective_id: frame.perspective_id.clone(),
            branch_id: frame.branch_id.clone(),
            source_refs: frame.source_refs.clone(),
        };
        inputs.validate()?;
        inputs.validate_for_projection(request, world_state)?;
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
        if self.frame_id.trim().is_empty()
            || self.request_id.trim().is_empty()
            || self.source_request_hash.trim().is_empty()
            || self.projection_version.trim().is_empty()
            || self.projection_hash.trim().is_empty()
            || self.world_state_hash.trim().is_empty()
            || self.perspective_kind.trim().is_empty()
            || self.perspective_id.trim().is_empty()
            || self.branch_id.trim().is_empty()
        {
            return Err("projection identity components must be non-empty".to_string());
        }
        Ok(())
    }

    /// Rebuild the exact authoritative frame reference retained by these inputs.
    pub fn frame_ref(&self, warnings: Vec<String>) -> Result<PlanningWorldStateFrameRef, String> {
        self.validate()?;
        Ok(PlanningWorldStateFrameRef {
            frame_id: self.frame_id.clone(),
            request_id: self.request_id.clone(),
            source_request_hash: self.source_request_hash.clone(),
            projection_version: self.projection_version.clone(),
            projection_hash: self.projection_hash.clone(),
            world_state_hash: self.world_state_hash.clone(),
            perspective_kind: self.perspective_kind.clone(),
            perspective_id: self.perspective_id.clone(),
            branch_id: self.branch_id.clone(),
            source_refs: self.source_refs.clone(),
            warnings,
        })
    }

    /// Validate that identity inputs name this exact request and projected state.
    pub fn validate_for_projection(
        &self,
        request: &PlanningWorldStateRequest,
        world_state: &meld_lang::WorldState,
    ) -> Result<(), String> {
        self.validate()?;
        if self.source_request_hash != request.canonical_hash()?
            || self.world_state_hash != canonical_world_state_hash(world_state)?
            || self.perspective_kind != request.perspective.perspective_kind
            || self.perspective_id != request.perspective.perspective_id
            || self.branch_id != request.branch_id
        {
            return Err(
                "world-model frame does not match the projection request and state".to_string(),
            );
        }
        Ok(())
    }
}

impl TryFrom<PlanningProjectionIdentityInputsWire> for PlanningProjectionIdentityInputs {
    type Error = String;

    fn try_from(value: PlanningProjectionIdentityInputsWire) -> Result<Self, Self::Error> {
        let inputs = Self {
            frame_id: value.frame_id,
            request_id: value.request_id,
            source_request_hash: value.source_request_hash,
            projection_version: value.projection_version,
            projection_hash: value.projection_hash,
            world_state_hash: value.world_state_hash,
            perspective_kind: value.perspective_kind,
            perspective_id: value.perspective_id,
            branch_id: value.branch_id,
            source_refs: value.source_refs,
        };
        inputs.validate()?;
        Ok(inputs)
    }
}

/// Hash one projected world state under the shared world-model boundary domain.
pub fn canonical_world_state_hash(world_state: &meld_lang::WorldState) -> Result<String, String> {
    let encoded = serde_json::to_vec(world_state).map_err(|error| error.to_string())?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(WORLD_STATE_HASH_DOMAIN);
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

/// Complete world-model perspective identity supplied by execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningPerspectiveRef {
    /// Perspective family.
    pub perspective_kind: String,
    /// Perspective member.
    pub perspective_id: String,
}

impl PlanningPerspectiveRef {
    /// Build and validate a complete perspective reference.
    pub fn new(
        perspective_kind: impl Into<String>,
        perspective_id: impl Into<String>,
    ) -> Result<Self, String> {
        let perspective = Self {
            perspective_kind: perspective_kind.into(),
            perspective_id: perspective_id.into(),
        };
        perspective.validate()?;
        Ok(perspective)
    }

    /// Validate both perspective identity components.
    pub fn validate(&self) -> Result<(), String> {
        if self.perspective_kind.trim().is_empty() || self.perspective_id.trim().is_empty() {
            return Err("planning perspective components must be non-empty".to_string());
        }
        Ok(())
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
    /// Complete world-model perspective identity.
    pub perspective: PlanningPerspectiveRef,
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
        let mut canonical = self.clone();
        canonical.requested_dimensions.sort();
        canonical.requested_dimensions.dedup();
        canonicalize_propositions(&mut canonical.required_preconditions)?;
        let encoded = serde_json::to_vec(&canonical).map_err(|error| error.to_string())?;
        Ok(blake3::hash(&encoded).to_hex().to_string())
    }
}

fn canonicalize_propositions(propositions: &mut Vec<meld_lang::Proposition>) -> Result<(), String> {
    let mut keyed = propositions
        .drain(..)
        .map(|proposition| serde_json::to_string(&proposition).map(|key| (key, proposition)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|left, right| left.0 == right.0);
    propositions.extend(keyed.into_iter().map(|(_, proposition)| proposition));
    Ok(())
}

/// Provenance for the projected world state consumed by planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningWorldStateFrameRef {
    /// Durable frame id assigned by the world-model projection authority.
    pub frame_id: String,
    /// Durable world-model request completed by this frame.
    pub request_id: String,
    /// Digest of the complete execution projection request.
    pub source_request_hash: String,
    /// Projection schema or algorithm version.
    pub projection_version: String,
    /// World-model digest of the complete projection output.
    pub projection_hash: String,
    /// Digest of the exact projected world state supplied to execution.
    pub world_state_hash: String,
    /// Perspective family used for the projection.
    pub perspective_kind: String,
    /// Perspective member used for the projection.
    pub perspective_id: String,
    /// Branch used for the projection.
    pub branch_id: String,
    /// Source belief or frame references used to build the projection.
    pub source_refs: Vec<String>,
    /// Non-fatal projection warnings that should travel with planning output.
    pub warnings: Vec<String>,
}

impl PlanningWorldStateFrameRef {
    /// Map one world-model-owned frame identity into the execution boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn from_authority(
        frame_id: impl Into<String>,
        request_id: impl Into<String>,
        source_request_hash: impl Into<String>,
        projection_version: impl Into<String>,
        projection_hash: impl Into<String>,
        world_state_hash: impl Into<String>,
        request: &PlanningWorldStateRequest,
        world_state: &meld_lang::WorldState,
        source_refs: Vec<String>,
        warnings: Vec<String>,
    ) -> Result<Self, String> {
        let frame = Self {
            frame_id: frame_id.into(),
            request_id: request_id.into(),
            source_request_hash: source_request_hash.into(),
            projection_version: projection_version.into(),
            projection_hash: projection_hash.into(),
            world_state_hash: world_state_hash.into(),
            perspective_kind: request.perspective.perspective_kind.clone(),
            perspective_id: request.perspective.perspective_id.clone(),
            branch_id: request.branch_id.clone(),
            source_refs,
            warnings,
        };
        PlanningProjectionIdentityInputs::from_projection(request, world_state, &frame)?;
        Ok(frame)
    }
}

#[cfg(test)]
mod contract_freeze_tests {
    use super::*;

    #[test]
    fn projection_identity_preserves_authoritative_frame_and_binds_world_state() {
        let request = PlanningWorldStateRequest {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: meld_lang::Proposition::Not(Box::new(meld_lang::Proposition::All(vec![]))),
            perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["docs_freshness".to_string()],
            required_preconditions: Vec::new(),
        };
        let world_state = meld_lang::WorldState::empty();
        let frame = PlanningWorldStateFrameRef::from_authority(
            "world-model-frame-a",
            "world-model-request-a",
            request.canonical_hash().unwrap(),
            "projection-v1",
            "projection-hash-a",
            canonical_world_state_hash(&world_state).unwrap(),
            &request,
            &world_state,
            vec!["source-a".to_string(), "source-b".to_string()],
            Vec::new(),
        )
        .unwrap();
        let inputs =
            PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame)
                .unwrap();

        assert_eq!(inputs.source_refs, vec!["source-a", "source-b"]);
        assert!(inputs
            .validate_for_projection(&request, &world_state)
            .is_ok());
        let first = inputs.frame_ref(Vec::new()).unwrap();
        let second = inputs.frame_ref(Vec::new()).unwrap();
        assert_eq!(first.frame_id, "world-model-frame-a");
        assert_eq!(first.frame_id, second.frame_id);
        assert_eq!(first.source_refs, inputs.source_refs);
        let encoded = serde_json::to_vec(&inputs).unwrap();
        let decoded: PlanningProjectionIdentityInputs = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, inputs);
        assert!(PlanningProjectionIdentityInputs::from_projection(
            &request,
            &meld_lang::WorldState::new(vec![meld_lang::Proposition::All(Vec::new())]).unwrap(),
            &frame,
        )
        .is_err());
    }

    #[test]
    fn source_request_hash_canonicalizes_semantic_set_order() {
        let mut first = PlanningWorldStateRequest {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: meld_lang::Proposition::All(Vec::new()),
            perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["freshness".to_string(), "docs".to_string()],
            required_preconditions: vec![
                meld_lang::Proposition::All(Vec::new()),
                meld_lang::Proposition::Not(Box::new(meld_lang::Proposition::All(Vec::new()))),
            ],
        };
        let mut second = first.clone();
        second.requested_dimensions.reverse();
        second.required_preconditions.reverse();
        first.requested_dimensions.push("docs".to_string());

        assert_eq!(
            first.canonical_hash().unwrap(),
            second.canonical_hash().unwrap()
        );
    }
}
