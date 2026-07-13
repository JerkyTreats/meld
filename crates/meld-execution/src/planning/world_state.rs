//! Execution side contracts for goal scoped world state projection.

use meld_events::DomainObjectRef;
use meld_lang::{Goal, Proposition, Term};
use serde::{Deserialize, Serialize};

const WORLD_STATE_HASH_DOMAIN: &[u8] = b"meld.planner-world-state.v1";
const PROJECTION_REQUEST_HASH_DOMAIN: &[u8] = b"meld.planner-projection-request.v1";
const PROJECTION_FRAME_HASH_DOMAIN: &[u8] = b"meld.planner-projection-frame.v1";
const MAX_PROJECTION_IDENTITY_COMPONENT_BYTES: usize = 256;
const MAX_CANONICAL_PROJECTION_REQUEST_BYTES: usize = 256 * 1024;
const MAX_PROJECTION_SOURCE_REF_BYTES: usize = 1024;
const MAX_PROJECTION_SOURCE_REFS: usize = 256;

/// Canonical world-model frame identity retained for planning replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PlanningProjectionIdentityInputsWire")]
pub struct PlanningProjectionIdentityInputs {
    /// Execution goal that scoped the authoritative projection request.
    goal_id: String,
    /// Exact durable goal sequence carried by the projection request.
    goal_updated_at_seq: u64,
    /// Canonical complete execution request whose hash crossed the authority boundary.
    canonical_request: String,
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
    goal_id: String,
    goal_updated_at_seq: u64,
    canonical_request: String,
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
        let canonical_request = canonical_projection_request(request)?;
        let inputs = Self {
            goal_id: request.goal_id.clone(),
            goal_updated_at_seq: request.source_seq,
            canonical_request,
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
        validate_projection_text("projection goal id", &self.goal_id)?;
        if self.goal_updated_at_seq == 0 {
            return Err("projection goal update sequence must be positive".to_string());
        }
        let canonical_request = self.canonical_request()?;
        if canonical_request.goal_id != self.goal_id
            || canonical_request.source_seq != self.goal_updated_at_seq
            || canonical_request.perspective.perspective_kind != self.perspective_kind
            || canonical_request.perspective.perspective_id != self.perspective_id
            || canonical_request.branch_id != self.branch_id
            || canonical_request.canonical_hash()? != self.source_request_hash
        {
            return Err(
                "projection identity does not match its canonical execution request".to_string(),
            );
        }
        for (label, value) in [
            ("projection frame id", self.frame_id.as_str()),
            ("projection request id", self.request_id.as_str()),
            ("projection version", self.projection_version.as_str()),
            (
                "projection perspective kind",
                self.perspective_kind.as_str(),
            ),
            ("projection perspective id", self.perspective_id.as_str()),
            ("projection branch id", self.branch_id.as_str()),
        ] {
            validate_projection_text(label, value)?;
        }
        for (label, digest) in [
            (
                "projection source request",
                self.source_request_hash.as_str(),
            ),
            ("projection output", self.projection_hash.as_str()),
            ("projection world state", self.world_state_hash.as_str()),
        ] {
            validate_projection_digest(label, digest)?;
        }
        if self.source_refs.len() > MAX_PROJECTION_SOURCE_REFS {
            return Err(format!(
                "projection identity source refs may contain at most {MAX_PROJECTION_SOURCE_REFS} items"
            ));
        }
        for source_ref in &self.source_refs {
            if source_ref.trim().is_empty() || source_ref.len() > MAX_PROJECTION_SOURCE_REF_BYTES {
                return Err(format!(
                    "projection identity source ref must contain at most {MAX_PROJECTION_SOURCE_REF_BYTES} bytes"
                ));
            }
        }
        let mut canonical_sources = self.source_refs.clone();
        canonical_sources.sort();
        canonical_sources.dedup();
        if canonical_sources != self.source_refs {
            return Err("projection identity source refs must be sorted and unique".to_string());
        }
        if self.request_id != canonical_planner_request_id(&canonical_request)? {
            return Err(
                "projection request id does not match the canonical execution request".to_string(),
            );
        }
        if self.frame_id != canonical_planner_frame_id(self)? {
            return Err("projection frame id does not match its authority identity".to_string());
        }
        Ok(())
    }

    /// Borrow the execution goal id bound into the authoritative projection.
    pub fn goal_id(&self) -> &str {
        &self.goal_id
    }

    /// Return the durable execution goal sequence bound into the projection.
    pub fn goal_updated_at_seq(&self) -> u64 {
        self.goal_updated_at_seq
    }

    /// Borrow the exact durable world-model frame identifier.
    pub fn frame_id(&self) -> &str {
        &self.frame_id
    }

    fn canonical_request(&self) -> Result<PlanningWorldStateRequest, String> {
        if self.canonical_request.len() > MAX_CANONICAL_PROJECTION_REQUEST_BYTES {
            return Err(format!(
                "canonical projection request may contain at most {MAX_CANONICAL_PROJECTION_REQUEST_BYTES} bytes"
            ));
        }
        let request: PlanningWorldStateRequest =
            serde_json::from_str(&self.canonical_request).map_err(|error| error.to_string())?;
        if canonical_projection_request(&request)? != self.canonical_request {
            return Err("projection request identity is not canonically encoded".to_string());
        }
        Ok(request)
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
        if self.canonical_request != canonical_projection_request(request)?
            || self.goal_id != request.goal_id
            || self.goal_updated_at_seq != request.source_seq
            || self.source_request_hash != request.canonical_hash()?
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
            goal_id: value.goal_id,
            goal_updated_at_seq: value.goal_updated_at_seq,
            canonical_request: value.canonical_request,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum CanonicalPlannerSourceRef {
    BeliefRevision { revision_id: String },
    Evidence { evidence_id: String },
    SourceFact { source_fact_id: String },
    GraphAnchor { anchor_id: String },
    ProjectionRule { rule_id: String },
}

fn canonical_projection_request(request: &PlanningWorldStateRequest) -> Result<String, String> {
    request.validate()?;
    let mut canonical = request.clone();
    canonical.requested_dimensions.sort();
    canonical.requested_dimensions.dedup();
    canonicalize_propositions(&mut canonical.required_preconditions)?;
    let encoded = serde_json::to_string(&canonical).map_err(|error| error.to_string())?;
    if encoded.len() > MAX_CANONICAL_PROJECTION_REQUEST_BYTES {
        return Err(format!(
            "canonical projection request may contain at most {MAX_CANONICAL_PROJECTION_REQUEST_BYTES} bytes"
        ));
    }
    Ok(encoded)
}

fn canonical_planner_request_id(request: &PlanningWorldStateRequest) -> Result<String, String> {
    #[derive(Serialize)]
    struct BranchScope<'a> {
        branch_id: &'a str,
    }

    #[derive(Serialize)]
    struct Identity<'a> {
        source_request_hash: &'a str,
        agent_id: &'a str,
        subject: &'a DomainObjectRef,
        perspective: &'a PlanningPerspectiveRef,
        branch_scope: BranchScope<'a>,
        requested_dimensions: &'a [String],
        required_preconditions: &'a [Proposition],
    }

    let canonical: PlanningWorldStateRequest =
        serde_json::from_str(&canonical_projection_request(request)?)
            .map_err(|error| error.to_string())?;
    hash_serializable(
        PROJECTION_REQUEST_HASH_DOMAIN,
        &Identity {
            source_request_hash: &canonical.canonical_hash()?,
            agent_id: &canonical.agent_id,
            subject: &canonical.subject,
            perspective: &canonical.perspective,
            branch_scope: BranchScope {
                branch_id: &canonical.branch_id,
            },
            requested_dimensions: &canonical.requested_dimensions,
            required_preconditions: &canonical.required_preconditions,
        },
    )
}

fn canonical_planner_frame_id(
    identity: &PlanningProjectionIdentityInputs,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Identity<'a> {
        request_id: &'a str,
        source_request_hash: &'a str,
        projection_version: &'a str,
        projection_hash: &'a str,
        world_state_hash: &'a str,
        source_refs: &'a [CanonicalPlannerSourceRef],
    }

    let mut source_refs = identity
        .source_refs
        .iter()
        .map(|source_ref| {
            let parsed =
                serde_json::from_str::<CanonicalPlannerSourceRef>(source_ref).map_err(|error| {
                    format!("projection source ref is not authoritative JSON: {error}")
                })?;
            if serde_json::to_string(&parsed).map_err(|error| error.to_string())? != *source_ref {
                return Err("projection source ref is not canonically encoded".to_string());
            }
            Ok(parsed)
        })
        .collect::<Result<Vec<_>, _>>()?;
    source_refs.sort();
    source_refs.dedup();
    hash_serializable(
        PROJECTION_FRAME_HASH_DOMAIN,
        &Identity {
            request_id: &identity.request_id,
            source_request_hash: &identity.source_request_hash,
            projection_version: &identity.projection_version,
            projection_hash: &identity.projection_hash,
            world_state_hash: &identity.world_state_hash,
            source_refs: &source_refs,
        },
    )
}

fn hash_serializable(domain: &[u8], value: &impl Serialize) -> Result<String, String> {
    let encoded = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

fn validate_projection_text(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > MAX_PROJECTION_IDENTITY_COMPONENT_BYTES {
        return Err(format!(
            "{label} must contain at most {MAX_PROJECTION_IDENTITY_COMPONENT_BYTES} bytes"
        ));
    }
    Ok(())
}

fn validate_projection_digest(label: &str, digest: &str) -> Result<(), String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!("{label} digest must be lowercase BLAKE3 hex"));
    }
    Ok(())
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
    /// Explicit world-model subject selected from the ground goal target.
    pub subject: DomainObjectRef,
    /// Durable execution goal sequence that orders request persistence.
    pub source_seq: u64,
    /// Target proposition that planning will evaluate.
    pub target: Proposition,
    /// Complete world-model perspective identity.
    pub perspective: PlanningPerspectiveRef,
    /// World model branch identifier.
    pub branch_id: String,
    /// Dimensions execution expects to evaluate.
    pub requested_dimensions: Vec<String>,
    /// Method preconditions that can be projected with the goal target.
    pub required_preconditions: Vec<Proposition>,
}

impl PlanningWorldStateRequest {
    /// Build one durable projection request from an execution-owned goal record.
    ///
    /// The goal target must be ground and identify exactly one subject so root
    /// integration never has to infer world-model scope. The source sequence is
    /// the durable goal-record sequence and becomes part of request identity.
    pub fn for_goal(
        goal: &Goal,
        source_seq: u64,
        perspective: PlanningPerspectiveRef,
        branch_id: impl Into<String>,
        requested_dimensions: Vec<String>,
        required_preconditions: Vec<Proposition>,
    ) -> Result<Self, String> {
        let subject = unique_projection_subject(&goal.target)?;
        let request = Self {
            goal_id: goal.goal_id.clone(),
            agent_id: goal.agent_id.clone(),
            subject,
            source_seq,
            target: goal.target.clone(),
            perspective,
            branch_id: branch_id.into(),
            requested_dimensions,
            required_preconditions,
        };
        request.validate()?;
        Ok(request)
    }

    /// Validate durable identity, scope agreement, and target grounding.
    pub fn validate(&self) -> Result<(), String> {
        if self.goal_id.trim().is_empty() || self.agent_id.trim().is_empty() {
            return Err("projection request goal and agent ids must be non-empty".to_string());
        }
        self.subject.validate().map_err(|error| error.to_string())?;
        if self.source_seq == 0 {
            return Err("projection request source sequence must be greater than zero".to_string());
        }
        if let Some(issue) = self.target.grounding_issue() {
            return Err(format!(
                "projection request target must be ground before persistence: {issue}"
            ));
        }
        if !proposition_references_subject(&self.target, &self.subject) {
            return Err(
                "projection request subject must be referenced by the ground goal target"
                    .to_string(),
            );
        }
        self.perspective.validate()?;
        if self.branch_id.trim().is_empty() {
            return Err("projection request branch id must be non-empty".to_string());
        }
        Ok(())
    }

    /// Derive a canonical digest from every projection request field.
    pub fn canonical_hash(&self) -> Result<String, String> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.requested_dimensions.sort();
        canonical.requested_dimensions.dedup();
        canonicalize_propositions(&mut canonical.required_preconditions)?;
        let encoded = serde_json::to_vec(&canonical).map_err(|error| error.to_string())?;
        Ok(blake3::hash(&encoded).to_hex().to_string())
    }
}

fn unique_projection_subject(target: &Proposition) -> Result<DomainObjectRef, String> {
    if let Some(issue) = target.grounding_issue() {
        return Err(format!(
            "projection request target must be ground before persistence: {issue}"
        ));
    }
    let mut subjects = Vec::new();
    collect_projection_subjects(target, &mut subjects);
    subjects.sort();
    subjects.dedup();
    match subjects.as_slice() {
        [subject] => Ok(subject.clone()),
        [] => Err("projection request target must reference one domain object subject".to_string()),
        _ => Err(
            "projection request target is ambiguous across multiple domain object subjects"
                .to_string(),
        ),
    }
}

fn proposition_references_subject(target: &Proposition, subject: &DomainObjectRef) -> bool {
    let mut subjects = Vec::new();
    collect_projection_subjects(target, &mut subjects);
    subjects.iter().any(|candidate| candidate == subject)
}

fn collect_projection_subjects(target: &Proposition, subjects: &mut Vec<DomainObjectRef>) {
    match target {
        Proposition::Holds { subject, .. } => collect_object_term(subject, subjects),
        Proposition::Exists { scope, .. } | Proposition::Accessible { scope } => {
            collect_object_term(scope, subjects);
        }
        Proposition::Related { src, dst, .. } => {
            collect_object_term(src, subjects);
            collect_object_term(dst, subjects);
        }
        Proposition::All(children) | Proposition::Any(children) => {
            for child in children {
                collect_projection_subjects(child, subjects);
            }
        }
        Proposition::Not(child) => collect_projection_subjects(child, subjects),
    }
}

fn collect_object_term(term: &Term, subjects: &mut Vec<DomainObjectRef>) {
    if let Term::Object(subject) = term {
        subjects.push(subject.clone());
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
    /// Independently derive the authority request and frame identities from exact output.
    #[allow(clippy::too_many_arguments)]
    pub fn identified_from_authority(
        projection_version: impl Into<String>,
        projection_hash: impl Into<String>,
        world_state_hash: impl Into<String>,
        request: &PlanningWorldStateRequest,
        world_state: &meld_lang::WorldState,
        source_refs: Vec<String>,
        warnings: Vec<String>,
    ) -> Result<Self, String> {
        let projection_version = projection_version.into();
        let projection_hash = projection_hash.into();
        let world_state_hash = world_state_hash.into();
        let source_request_hash = request.canonical_hash()?;
        let request_id = canonical_planner_request_id(request)?;
        let mut identity = PlanningProjectionIdentityInputs {
            goal_id: request.goal_id.clone(),
            goal_updated_at_seq: request.source_seq,
            canonical_request: canonical_projection_request(request)?,
            frame_id: String::new(),
            request_id: request_id.clone(),
            source_request_hash: source_request_hash.clone(),
            projection_version: projection_version.clone(),
            projection_hash: projection_hash.clone(),
            world_state_hash: world_state_hash.clone(),
            perspective_kind: request.perspective.perspective_kind.clone(),
            perspective_id: request.perspective.perspective_id.clone(),
            branch_id: request.branch_id.clone(),
            source_refs: source_refs.clone(),
        };
        identity.frame_id = canonical_planner_frame_id(&identity)?;
        let frame = Self {
            frame_id: identity.frame_id,
            request_id,
            source_request_hash,
            projection_version,
            projection_hash,
            world_state_hash,
            perspective_kind: request.perspective.perspective_kind.clone(),
            perspective_id: request.perspective.perspective_id.clone(),
            branch_id: request.branch_id.clone(),
            source_refs,
            warnings,
        };
        PlanningProjectionIdentityInputs::from_projection(request, world_state, &frame)?;
        Ok(frame)
    }

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
        let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let request = PlanningWorldStateRequest {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            subject: subject.clone(),
            source_seq: 7,
            target: Proposition::Not(Box::new(Proposition::Accessible {
                scope: Term::Object(subject),
            })),
            perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["docs_freshness".to_string()],
            required_preconditions: Vec::new(),
        };
        let world_state = meld_lang::WorldState::empty();
        let frame = PlanningWorldStateFrameRef::identified_from_authority(
            "projection-v1",
            blake3::hash(b"projection-a").to_hex().to_string(),
            canonical_world_state_hash(&world_state).unwrap(),
            &request,
            &world_state,
            vec![
                serde_json::json!({"ProjectionRule": {"rule_id": "source-a"}}).to_string(),
                serde_json::json!({"ProjectionRule": {"rule_id": "source-b"}}).to_string(),
            ],
            Vec::new(),
        )
        .unwrap();
        let inputs =
            PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame)
                .unwrap();

        assert_eq!(inputs.source_refs.len(), 2);
        assert!(inputs
            .validate_for_projection(&request, &world_state)
            .is_ok());
        let first = inputs.frame_ref(Vec::new()).unwrap();
        let second = inputs.frame_ref(Vec::new()).unwrap();
        assert_eq!(first.frame_id, second.frame_id);
        assert_eq!(first.source_refs, inputs.source_refs);
        let encoded = serde_json::to_vec(&inputs).unwrap();
        let decoded: PlanningProjectionIdentityInputs = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, inputs);

        let mut rebound_request = request.clone();
        rebound_request.goal_id = "goal-b".to_string();
        rebound_request.source_seq += 1;
        assert!(PlanningProjectionIdentityInputs::from_projection(
            &rebound_request,
            &world_state,
            &frame,
        )
        .is_err());

        let mut retained_source: serde_json::Value = serde_json::to_value(&inputs).unwrap();
        retained_source["goal_id"] = serde_json::json!(rebound_request.goal_id);
        retained_source["goal_updated_at_seq"] = serde_json::json!(rebound_request.source_seq);
        retained_source["canonical_request"] =
            serde_json::json!(canonical_projection_request(&rebound_request).unwrap());
        assert!(
            serde_json::from_value::<PlanningProjectionIdentityInputs>(retained_source).is_err()
        );

        let rebound_frame = PlanningWorldStateFrameRef::identified_from_authority(
            frame.projection_version.clone(),
            frame.projection_hash.clone(),
            frame.world_state_hash.clone(),
            &rebound_request,
            &world_state,
            frame.source_refs.clone(),
            Vec::new(),
        )
        .unwrap();
        let rebound_identity = PlanningProjectionIdentityInputs::from_projection(
            &rebound_request,
            &world_state,
            &rebound_frame,
        )
        .unwrap();
        let mut retained_frame = serde_json::to_value(rebound_identity).unwrap();
        retained_frame["frame_id"] = serde_json::json!(frame.frame_id);
        assert!(
            serde_json::from_value::<PlanningProjectionIdentityInputs>(retained_frame).is_err()
        );

        assert!(PlanningProjectionIdentityInputs::from_projection(
            &request,
            &meld_lang::WorldState::new(vec![meld_lang::Proposition::All(Vec::new())]).unwrap(),
            &frame,
        )
        .is_err());
    }

    #[test]
    fn projection_identity_rejects_malformed_or_unbounded_products_on_build_and_decode() {
        let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let request = PlanningWorldStateRequest {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            subject: subject.clone(),
            source_seq: 7,
            target: Proposition::Accessible {
                scope: Term::Object(subject),
            },
            perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: Vec::new(),
            required_preconditions: Vec::new(),
        };
        let world_state = meld_lang::WorldState::empty();
        let mut frame = PlanningWorldStateFrameRef::identified_from_authority(
            "projection-v1",
            blake3::hash(b"projection-a").to_hex().to_string(),
            canonical_world_state_hash(&world_state).unwrap(),
            &request,
            &world_state,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let identity =
            PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame)
                .unwrap();

        frame.projection_hash = "not-a-digest".to_string();
        assert!(
            PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame)
                .is_err()
        );
        frame.projection_hash = blake3::hash(b"projection-a").to_hex().to_string();
        frame.frame_id = "x".repeat(MAX_PROJECTION_IDENTITY_COMPONENT_BYTES + 1);
        assert!(
            PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame)
                .is_err()
        );

        let mut encoded = serde_json::to_value(identity).unwrap();
        encoded["world_state_hash"] = serde_json::json!("not-a-digest");
        assert!(serde_json::from_value::<PlanningProjectionIdentityInputs>(encoded).is_err());
    }

    #[test]
    fn source_request_hash_canonicalizes_semantic_set_order() {
        let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let mut first = PlanningWorldStateRequest {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            subject: subject.clone(),
            source_seq: 7,
            target: Proposition::Accessible {
                scope: Term::Object(subject),
            },
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

    #[test]
    fn durable_request_identity_binds_subject_and_source_sequence() {
        let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let goal = Goal {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: Proposition::Accessible {
                scope: Term::Object(subject.clone()),
            },
            priority: meld_lang::GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: meld_lang::GoalSource::UserDirected {
                directive: "refresh docs".to_string(),
            },
            lifecycle: meld_lang::GoalLifecycle::Active,
        };
        let request = PlanningWorldStateRequest::for_goal(
            &goal,
            7,
            PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            "main",
            vec!["docs_freshness".to_string()],
            Vec::new(),
        )
        .unwrap();
        let mut different_subject = request.clone();
        different_subject.subject = DomainObjectRef::new("workspace", "node", "other").unwrap();
        let mut different_sequence = request.clone();
        different_sequence.source_seq += 1;

        assert_eq!(request.subject, subject);
        assert!(different_subject.validate().is_err());
        assert_ne!(
            request.canonical_hash().unwrap(),
            different_sequence.canonical_hash().unwrap()
        );
    }

    #[test]
    fn durable_request_rejects_zero_sequence_and_ambiguous_goal_subject() {
        let first = DomainObjectRef::new("workspace", "node", "first").unwrap();
        let second = DomainObjectRef::new("workspace", "node", "second").unwrap();
        let goal = Goal {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: Proposition::Related {
                src: Term::Object(first),
                relation: Term::Literal(meld_lang::Literal::Text("depends_on".to_string())),
                dst: Term::Object(second),
            },
            priority: meld_lang::GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: meld_lang::GoalSource::UserDirected {
                directive: "test".to_string(),
            },
            lifecycle: meld_lang::GoalLifecycle::Active,
        };

        let error = PlanningWorldStateRequest::for_goal(
            &goal,
            1,
            PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            "main",
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err();
        assert!(error.contains("ambiguous"));

        let mut single = goal;
        single.target = Proposition::Accessible {
            scope: Term::Object(DomainObjectRef::new("workspace", "node", "single").unwrap()),
        };
        let error = PlanningWorldStateRequest::for_goal(
            &single,
            0,
            PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            "main",
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err();
        assert!(error.contains("greater than zero"));
    }
}
