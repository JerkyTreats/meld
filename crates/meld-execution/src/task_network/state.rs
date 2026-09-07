//! Reduced task network state contracts.
//!
//! Owner: task network.
//! Inputs: committed mutations, dispatch claims, task outcomes, artifact
//! availability records, and publication marks.
//! Outputs: deterministic reduced state and ready set query records.
//! Does not own: this module does not execute capabilities or publish events.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::state::NetworkState;
//!
//! let state = NetworkState::empty("network-a");
//! assert_eq!(state.revision, 0);
//! assert!(!state.state_hash.is_empty());
//! ```

use crate::task::{CompiledTaskRecord, TaskRunContext};
use crate::task_network::{contracts::stable_hash, dispatch, outcome};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Complete reduced task network state at one journal revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkState {
    /// Intact lowered steps and compatibility decisions attached to shared nodes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shared_steps: BTreeMap<String, super::mutation::Inject>,
    /// Stable task network identifier.
    pub network_id: String,
    /// Monotonic journal revision applied to this state.
    pub revision: u64,
    /// Stable hash over reduced state content excluding this field.
    pub state_hash: String,
    /// Agent-authorized Task admission decisions keyed by admission identity.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub admissions: BTreeMap<String, crate::task_admission::TaskAdmissionRecord>,
    /// Canonical lowered node hashes keyed by admission and step identity.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub admitted_region_node_hashes: BTreeMap<String, BTreeMap<String, String>>,
    /// Task nodes keyed by task instance id.
    pub tasks: BTreeMap<String, TaskNode>,
    /// Dependency edges between task nodes.
    pub edges: Vec<DependencyEdge>,
    /// Lifecycle status keyed by task instance id.
    pub statuses: BTreeMap<String, TaskStatus>,
    /// Artifact availability records emitted by completed tasks.
    pub artifact_availability: Vec<ArtifactAvailability>,
    /// Dispatch claims keyed by claim id.
    pub claims: BTreeMap<String, dispatch::Claim>,
    /// Task outcomes keyed by outcome id.
    pub outcomes: BTreeMap<String, dispatch::Outcome>,
    /// Publication outbox records keyed by publication id.
    pub publications: BTreeMap<String, outcome::Publication>,
}

impl NetworkState {
    /// Creates an empty state snapshot with a stable initial hash.
    pub fn empty(network_id: impl Into<String>) -> Self {
        let mut state = Self {
            shared_steps: BTreeMap::new(),
            network_id: network_id.into(),
            revision: 0,
            state_hash: String::new(),
            admissions: BTreeMap::new(),
            admitted_region_node_hashes: BTreeMap::new(),
            tasks: BTreeMap::new(),
            edges: Vec::new(),
            statuses: BTreeMap::new(),
            artifact_availability: Vec::new(),
            claims: BTreeMap::new(),
            outcomes: BTreeMap::new(),
            publications: BTreeMap::new(),
        };
        state.state_hash = state.recompute_state_hash();
        state
    }

    /// Recomputes the deterministic state hash for this snapshot.
    pub fn recompute_state_hash(&self) -> String {
        #[derive(Serialize)]
        struct HashProjection<'a> {
            #[serde(skip_serializing_if = "BTreeMap::is_empty")]
            shared_steps: &'a BTreeMap<String, super::mutation::Inject>,
            network_id: &'a str,
            revision: u64,
            #[serde(skip_serializing_if = "BTreeMap::is_empty")]
            admissions: &'a BTreeMap<String, crate::task_admission::TaskAdmissionRecord>,
            #[serde(skip_serializing_if = "BTreeMap::is_empty")]
            admitted_region_node_hashes: &'a BTreeMap<String, BTreeMap<String, String>>,
            tasks: &'a BTreeMap<String, TaskNode>,
            edges: Vec<DependencyEdge>,
            statuses: &'a BTreeMap<String, TaskStatus>,
            artifact_availability: Vec<ArtifactAvailability>,
            claims: &'a BTreeMap<String, dispatch::Claim>,
            outcomes: &'a BTreeMap<String, dispatch::Outcome>,
            publications: &'a BTreeMap<String, outcome::Publication>,
        }

        let mut edges = self.edges.clone();
        edges.sort();
        let mut artifact_availability = self.artifact_availability.clone();
        artifact_availability.sort();

        stable_hash(&HashProjection {
            shared_steps: &self.shared_steps,
            network_id: &self.network_id,
            revision: self.revision,
            admissions: &self.admissions,
            admitted_region_node_hashes: &self.admitted_region_node_hashes,
            tasks: &self.tasks,
            edges,
            statuses: &self.statuses,
            artifact_availability,
            claims: &self.claims,
            outcomes: &self.outcomes,
            publications: &self.publications,
        })
    }

    /// Sets the revision and updates the hash after a journal record is applied.
    pub fn set_revision_and_hash(&mut self, revision: u64) {
        self.revision = revision;
        self.state_hash = self.recompute_state_hash();
    }
}

/// Return the exact terminal outcome for one admitted operational region.
///
/// A failed branch does not make the region terminal while an independent
/// branch can still run. Pending descendants blocked by failed or cancelled
/// ancestors are treated as unreachable. Successful regions must converge on
/// exactly one sink so this query never invents an aggregate outcome.
pub fn admission_region_terminal_outcome_id<'a>(
    state: &'a NetworkState,
    admission_id: &str,
) -> Option<&'a str> {
    let region = state
        .tasks
        .values()
        .filter(|node| {
            node.lineage
                .admission
                .as_ref()
                .is_some_and(|attribution| attribution.admission_id == admission_id)
        })
        .map(|node| node.task_instance_id.as_str())
        .chain(state.shared_steps.values().filter_map(|step| {
            (step.task_node.lineage.admission.as_ref()?.admission_id == admission_id)
                .then_some(step.sharing.as_ref()?.shared_node_id.as_str())
        }))
        .collect::<BTreeSet<_>>();
    if region.is_empty() {
        return None;
    }

    let mut unreachable = BTreeSet::new();
    loop {
        let mut changed = false;
        for task_id in &region {
            if unreachable.contains(task_id)
                || !matches!(state.statuses.get(*task_id), Some(TaskStatus::Pending))
            {
                continue;
            }
            let blocked = state.edges.iter().any(|edge| {
                edge.to == **task_id
                    && region.contains(edge.from.as_str())
                    && (unreachable.contains(edge.from.as_str())
                        || matches!(
                            state.statuses.get(&edge.from),
                            Some(TaskStatus::Failed { .. } | TaskStatus::Cancelled { .. })
                        ))
            });
            if blocked {
                unreachable.insert(*task_id);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    if region.iter().any(|task_id| {
        matches!(
            state.statuses.get(*task_id),
            None | Some(TaskStatus::Running { .. } | TaskStatus::Pending)
                if !unreachable.contains(task_id)
        )
    }) {
        return None;
    }

    if let Some((_, outcome_id)) =
        region
            .iter()
            .find_map(|task_id| match state.statuses.get(*task_id) {
                Some(TaskStatus::Failed { outcome_id, .. }) => {
                    Some((*task_id, outcome_id.as_str()))
                }
                _ => None,
            })
    {
        return state
            .outcomes
            .contains_key(outcome_id)
            .then_some(outcome_id);
    }

    let sinks = region
        .iter()
        .filter(|task_id| {
            !state
                .edges
                .iter()
                .any(|edge| edge.from == ***task_id && region.contains(edge.to.as_str()))
        })
        .copied()
        .collect::<Vec<_>>();
    let [sink] = sinks.as_slice() else {
        return None;
    };
    let Some(TaskStatus::Succeeded { outcome_id }) = state.statuses.get(*sink) else {
        return None;
    };
    state
        .outcomes
        .contains_key(outcome_id)
        .then_some(outcome_id)
}

/// Executable task node in the durable operational graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskNode {
    /// Stable task instance identifier within the task network.
    pub task_instance_id: String,
    /// Lifecycle epoch used to fence stale claims and outcomes.
    pub lifecycle_epoch: u64,
    /// Compiled task-local capability graph.
    pub compiled_task: CompiledTaskRecord,
    /// Source records used to materialize the dispatch initialization payload.
    pub init_sources: Vec<TaskInitSource>,
    /// Runtime context assigned to this task run.
    pub task_run_context: TaskRunContext,
    /// Exact operational lineage preserved across lowering and dispatch.
    pub lineage: TaskLineage,
}

/// Planned source for one task initialization artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskInitSource {
    /// Structured seed content owned by the task node.
    StaticSeed(StaticSeedInitSource),
    /// Structured content selected from an upstream task outcome.
    UpstreamArtifact(UpstreamArtifactInitSource),
}

impl TaskInitSource {
    /// Returns the target init slot satisfied by this source.
    pub fn init_slot_id(&self) -> &str {
        match self {
            Self::StaticSeed(source) => &source.init_slot_id,
            Self::UpstreamArtifact(source) => &source.init_slot_id,
        }
    }

    /// Returns the artifact type materialized for the target init slot.
    pub fn artifact_type_id(&self) -> &str {
        match self {
            Self::StaticSeed(source) => &source.artifact_type_id,
            Self::UpstreamArtifact(source) => &source.artifact_type_id,
        }
    }

    /// Returns the schema version materialized for the target init slot.
    pub fn schema_version(&self) -> u32 {
        match self {
            Self::StaticSeed(source) => source.schema_version,
            Self::UpstreamArtifact(source) => source.schema_version,
        }
    }
}

/// Static seed for one task initialization slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticSeedInitSource {
    /// Task initialization slot that receives this artifact.
    pub init_slot_id: String,
    /// Artifact type materialized for the init slot.
    pub artifact_type_id: String,
    /// Schema version for the materialized init artifact.
    pub schema_version: u32,
    /// Structured seed content copied into the dispatch payload.
    pub content: Value,
}

/// Upstream artifact source for one task initialization slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpstreamArtifactInitSource {
    /// Task initialization slot that receives this artifact.
    pub init_slot_id: String,
    /// Artifact type materialized for the init slot.
    pub artifact_type_id: String,
    /// Schema version for the materialized init artifact.
    pub schema_version: u32,
    /// Task instance that must emit the upstream artifact.
    pub upstream_task_instance_id: String,
    /// Artifact type selected from the upstream task outcome.
    pub upstream_artifact_type_id: String,
}

/// Operational lineage for a task node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskLineage {
    /// Historical Execution composition identity, empty for current writes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) composition_id: String,
    /// Historical planner Goal identity, empty for current writes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) goal_id: String,
    /// Historical Method identity, empty for current writes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) method_id: String,
    /// Composition step lowered into this task.
    pub step_id: String,
    /// Operator identifier resolved for this task.
    pub operator_id: String,
    /// Historical planning frame identity, empty for current writes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) world_state_frame_id: String,
    /// Capability type selected for the operator.
    pub capability_type_id: String,
    /// Capability version selected for the operator.
    pub capability_version: u32,
    /// Effective authority retained for independent dispatch enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_decision: Option<meld_lang::AuthorityDecision>,
    /// Complete Agent Task admission attribution for canonical WMR nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission: Option<TaskAdmissionAttribution>,
}

impl TaskLineage {
    /// Build current operational lineage for one admitted Agent Task step.
    pub fn admitted(
        step_id: String,
        operator_id: String,
        capability_type_id: String,
        capability_version: u32,
        authority_decision: Option<meld_lang::AuthorityDecision>,
        admission: TaskAdmissionAttribution,
    ) -> Self {
        Self {
            composition_id: String::new(),
            goal_id: String::new(),
            method_id: String::new(),
            step_id,
            operator_id,
            world_state_frame_id: String::new(),
            capability_type_id,
            capability_version,
            authority_decision,
            admission: Some(admission),
        }
    }

    /// Build current lineage for a Task outside Agent admission authority.
    pub fn unattributed(
        step_id: String,
        operator_id: String,
        capability_type_id: String,
        capability_version: u32,
    ) -> Self {
        Self {
            composition_id: String::new(),
            goal_id: String::new(),
            method_id: String::new(),
            step_id,
            operator_id,
            world_state_frame_id: String::new(),
            capability_type_id,
            capability_version,
            authority_decision: None,
            admission: None,
        }
    }

    /// True when this lineage was decoded from the retired Goal-planning format.
    pub(crate) fn is_legacy_planning(&self) -> bool {
        !self.composition_id.is_empty()
            || !self.goal_id.is_empty()
            || !self.method_id.is_empty()
            || !self.world_state_frame_id.is_empty()
    }
}

impl NetworkState {
    pub(crate) fn contains_legacy_planning(&self) -> bool {
        self.tasks
            .values()
            .any(|node| node.lineage.is_legacy_planning())
    }
}

/// Exact producer and consumer lineage retained on every admitted Task node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskAdmissionAttribution {
    /// Agent that granted Task authority.
    pub agent_id: String,
    /// Goal to which operational work remains attributed.
    pub goal_id: String,
    /// Immutable Plan revision containing the Task.
    pub plan_revision_id: String,
    /// Complete Task identity.
    pub task_id: String,
    /// Fresh Agent product authorization identity.
    pub authorization_id: String,
    /// Durable Execution admission identity.
    pub admission_id: String,
    /// Agent authority scope identity.
    pub authority_scope_id: String,
    /// Exact authority policy content identity.
    pub authority_policy_content_hash: String,
    /// Activation generation fencing realization.
    pub activation_generation: String,
    /// Exact admission epoch retained from the accepted offer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_epoch: Option<String>,
}

impl TaskAdmissionAttribution {
    /// Project complete attribution from one durable admission record.
    pub fn from_record(record: &crate::task_admission::TaskAdmissionRecord) -> Self {
        Self {
            agent_id: record.request.lineage.agent_id.clone(),
            goal_id: record.request.lineage.goal_id.clone(),
            plan_revision_id: record.request.lineage.plan_revision_id.clone(),
            task_id: record.request.task.task_id.clone(),
            authorization_id: record.request.lineage.authorization_id.clone(),
            admission_id: record.admission_id.clone(),
            authority_scope_id: record.request.lineage.authority_scope_id.clone(),
            authority_policy_content_hash: record
                .request
                .lineage
                .authority_policy_content_hash
                .clone(),
            activation_generation: record.request.lineage.activation_generation.clone(),
            admission_epoch: record.request.lineage.admission_epoch.clone(),
        }
    }
}

/// Verify that one attributed node names the exact durable admitted Task.
pub(crate) fn validate_task_admission_attribution(
    state: &NetworkState,
    node: &TaskNode,
) -> Result<(), String> {
    validate_task_admission_attribution_for_lowering(state, node)?;
    let Some(attribution) = &node.lineage.admission else {
        return Ok(());
    };
    let expected_hash = state
        .admitted_region_node_hashes
        .get(&attribution.admission_id)
        .and_then(|hashes| hashes.get(&node.lineage.step_id))
        .ok_or_else(|| {
            format!(
                "Task admission attribution '{}' has no canonical lowered node",
                attribution.admission_id
            )
        })?;
    if expected_hash != &stable_hash(node) {
        return Err(format!(
            "Task admission attribution '{}' carries executable content outside canonical lowering",
            attribution.admission_id
        ));
    }
    Ok(())
}

/// Verify admission identity while the canonical lowered node is being sealed.
pub(crate) fn validate_task_admission_attribution_for_lowering(
    state: &NetworkState,
    node: &TaskNode,
) -> Result<(), String> {
    let Some(attribution) = &node.lineage.admission else {
        return if node.lineage.authority_decision.is_some() {
            Err("Agent Task authority has no durable Task admission attribution".to_string())
        } else {
            Ok(())
        };
    };
    let record = state
        .admissions
        .get(&attribution.admission_id)
        .ok_or_else(|| {
            format!(
                "Task admission attribution '{}' has no durable record",
                attribution.admission_id
            )
        })?;
    if record.decision != crate::task_admission::TaskAdmissionDecision::Admitted {
        return Err(format!(
            "Task admission attribution '{}' does not name an admitted decision",
            attribution.admission_id
        ));
    }
    if attribution != &TaskAdmissionAttribution::from_record(record) {
        return Err(format!(
            "Task admission attribution '{}' differs from its durable record",
            attribution.admission_id
        ));
    }
    if node.lineage.authority_decision != record.request.lineage.authority_decision {
        return Err(format!(
            "Task admission attribution '{}' carries a different authority decision",
            attribution.admission_id
        ));
    }
    let step = record
        .request
        .task
        .composition
        .steps
        .iter()
        .find(|step| step.step_id == node.lineage.step_id)
        .ok_or_else(|| {
            format!(
                "Task admission attribution '{}' names a step absent from the admitted Task",
                attribution.admission_id
            )
        })?;
    let meld_lang::StepKind::Op(operator) = &step.kind else {
        return Err(format!(
            "Task admission attribution '{}' names a non-operational step",
            attribution.admission_id
        ));
    };
    let specific = operator.resolution.specific.as_ref().ok_or_else(|| {
        format!(
            "Task admission attribution '{}' names an unresolved Capability",
            attribution.admission_id
        )
    })?;
    if node.lineage.operator_id != operator.operator_id
        || node.lineage.capability_type_id != specific.capability_type_id
        || node.lineage.capability_version != specific.capability_version
    {
        return Err(format!(
            "Task admission attribution '{}' carries operational lineage outside the admitted Task",
            attribution.admission_id
        ));
    }
    Ok(())
}

/// Dependency edge between task nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Upstream task instance id.
    pub from: String,
    /// Downstream task instance id.
    pub to: String,
    /// Semantics required to satisfy this dependency.
    pub kind: DependencyKind,
    /// Why the edge exists. Committed edges record their origin so the
    /// later semantic-versus-scheduling separation needs no migration.
    /// An unrecorded origin is skipped during serialization so pre-origin
    /// durable snapshots keep their byte form and state hash.
    #[serde(default, skip_serializing_if = "DependencyEdgeOrigin::is_unrecorded")]
    pub origin: DependencyEdgeOrigin,
}

/// Recorded origin of a committed dependency edge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DependencyEdgeOrigin {
    /// The edge encodes domain meaning, such as child artifacts feeding a
    /// parent.
    Semantic,
    /// The edge encodes an execution-policy ordering with no domain
    /// meaning.
    Scheduling,
    /// The edge predates origin recording. New commits must not use this.
    #[default]
    Unrecorded,
}

impl DependencyEdgeOrigin {
    /// True when the origin predates recording, used to keep legacy
    /// snapshot serialization byte-identical.
    pub fn is_unrecorded(&self) -> bool {
        matches!(self, Self::Unrecorded)
    }
}

/// Dependency semantics between two task nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DependencyKind {
    /// Upstream task must succeed before downstream dispatch.
    Ordering,
    /// Upstream task must emit the named artifact type before downstream dispatch.
    DataFlow {
        /// Required artifact type id.
        artifact_type_id: String,
    },
    /// Guarded activation deferred in the first slice.
    Conditional {
        /// Output field path evaluated by a later slice.
        field_path: String,
        /// Stable serialized guard expression.
        guard_json: String,
    },
}

/// Task lifecycle status in the task network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is eligible for readiness once dependencies are satisfied.
    Pending,
    /// Task has a current dispatch claim.
    Running {
        /// Current claim id.
        claim_id: String,
    },
    /// Task completed successfully.
    Succeeded {
        /// Outcome id that completed the task.
        outcome_id: String,
    },
    /// Task completed with a failure.
    Failed {
        /// Outcome id that completed the task.
        outcome_id: String,
        /// Failure summary.
        error: String,
    },
    /// Task was cancelled by a lifecycle mutation.
    Cancelled {
        /// Cancellation reason.
        reason: String,
    },
}

/// Artifact availability record emitted by a completed task.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactAvailability {
    /// Task instance that produced or exposed the artifact.
    pub task_instance_id: String,
    /// Artifact type id available for downstream data flow edges.
    pub artifact_type_id: String,
    /// Stable artifact id.
    pub artifact_id: String,
    /// Artifact schema version.
    pub schema_version: u32,
}

/// Deterministic ready task query result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadySet {
    /// Stable task network identifier.
    pub network_id: String,
    /// Revision evaluated by this query.
    pub revision: u64,
    /// State hash evaluated by this query.
    pub state_hash: String,
    /// Ready task instance ids in deterministic order.
    pub task_instance_ids: Vec<String>,
    /// Diagnostics for blocked or invalid graph state.
    pub diagnostics: Vec<ReadinessDiagnostic>,
}

/// Readiness diagnostic for graph or dependency state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessDiagnostic {
    /// Machine-readable diagnostic code.
    pub code: ReadinessDiagnosticCode,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Task instance associated with the diagnostic.
    pub task_instance_id: Option<String>,
}

impl ReadinessDiagnostic {
    /// Creates a readiness diagnostic without a task id.
    pub fn new(code: ReadinessDiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            task_instance_id: None,
        }
    }

    /// Attaches a task instance id to this diagnostic.
    pub fn with_task(mut self, task_instance_id: impl Into<String>) -> Self {
        self.task_instance_id = Some(task_instance_id.into());
        self
    }
}

/// Stable readiness diagnostic code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadinessDiagnosticCode {
    /// Edge references a missing endpoint.
    MissingEndpoint,
    /// Active task graph contains a cycle.
    CycleDetected,
    /// Conditional edges are not dispatchable in this slice.
    ConditionalDeferred,
    /// Data flow edge is waiting for an upstream artifact.
    ArtifactUnavailable,
}
