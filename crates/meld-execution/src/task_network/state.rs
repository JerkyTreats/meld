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
use std::collections::BTreeMap;

/// Complete reduced task network state at one journal revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkState {
    /// Stable task network identifier.
    pub network_id: String,
    /// Monotonic journal revision applied to this state.
    pub revision: u64,
    /// Stable hash over reduced state content excluding this field.
    pub state_hash: String,
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
            network_id: network_id.into(),
            revision: 0,
            state_hash: String::new(),
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
            network_id: &'a str,
            revision: u64,
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
            network_id: &self.network_id,
            revision: self.revision,
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

/// Executable task node projected from an execution composition.
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
    /// Planning lineage preserved across lowering and dispatch.
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

/// Planning lineage for a task node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskLineage {
    /// Execution composition that produced this task.
    pub composition_id: String,
    /// Goal selected by the planner.
    pub goal_id: String,
    /// Method selected by the planner.
    pub method_id: String,
    /// Composition step lowered into this task.
    pub step_id: String,
    /// Operator identifier resolved for this task.
    pub operator_id: String,
    /// World state frame used during method selection.
    pub world_state_frame_id: String,
    /// Capability type selected for the operator.
    pub capability_type_id: String,
    /// Capability version selected for the operator.
    pub capability_version: u32,
    /// Effective authority retained for independent dispatch enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_decision: Option<meld_lang::AuthorityDecision>,
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
