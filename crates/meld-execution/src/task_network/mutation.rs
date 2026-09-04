//! Task network mutation and commit contracts.
//!
//! Owner: task network.
//! Inputs: graph mutation proposals produced by Task admission lowering.
//! Outputs: append-only commit records and typed rejection results.
//! Does not own: this module does not run task executors or invoke
//! capabilities.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::mutation::Set;
//!
//! let set = Set::empty("network-a", "composition-a", "once");
//! assert_eq!(set.schema_version, 1);
//! assert!(set.set_id.starts_with("task-network-mutation-set-"));
//! ```

use crate::task_network::{
    contracts::{stable_id, TASK_NETWORK_SCHEMA_VERSION},
    state,
};
use serde::{Deserialize, Serialize};

/// Proposed task network mutation set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Set {
    /// Schema version for this mutation set.
    pub schema_version: u32,
    /// Stable id derived from the set content.
    pub set_id: String,
    /// Stable task network identifier.
    pub network_id: String,
    /// Agent Task that produced this set.
    #[serde(alias = "source_composition_id")]
    pub source_task_id: String,
    /// Caller supplied idempotency key.
    pub idempotency_key: String,
    /// Proposed mutations.
    pub mutations: Vec<Mutation>,
    /// Historical planning diagnostics retained only while decoding old journals.
    #[serde(default, rename = "diagnostics", skip_serializing_if = "Vec::is_empty")]
    legacy_planning_diagnostics: Vec<serde_json::Value>,
}

impl Set {
    /// Creates an empty mutation set.
    pub fn empty(
        network_id: impl Into<String>,
        source_task_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self::new(network_id, source_task_id, idempotency_key, Vec::new())
    }

    /// Creates a mutation set and derives its stable id.
    pub fn new(
        network_id: impl Into<String>,
        source_task_id: impl Into<String>,
        idempotency_key: impl Into<String>,
        mutations: Vec<Mutation>,
    ) -> Self {
        #[derive(Serialize)]
        struct Identity<'a> {
            schema_version: u32,
            network_id: &'a str,
            source_task_id: &'a str,
            idempotency_key: &'a str,
            mutations: &'a [Mutation],
        }

        let network_id = network_id.into();
        let source_task_id = source_task_id.into();
        let idempotency_key = idempotency_key.into();
        let set_id = stable_id(
            "task-network-mutation-set",
            &Identity {
                schema_version: TASK_NETWORK_SCHEMA_VERSION,
                network_id: &network_id,
                source_task_id: &source_task_id,
                idempotency_key: &idempotency_key,
                mutations: &mutations,
            },
        );

        Self {
            schema_version: TASK_NETWORK_SCHEMA_VERSION,
            set_id,
            network_id,
            source_task_id,
            idempotency_key,
            mutations,
            legacy_planning_diagnostics: Vec::new(),
        }
    }

    /// True when this set came from the retired Goal-planning format.
    pub(crate) fn is_legacy_planning(&self) -> bool {
        !self.legacy_planning_diagnostics.is_empty()
    }
}

/// Task network graph mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mutation {
    /// Injects one task node and its incoming dependency edges.
    Inject(Inject),
}

/// Mutation that injects one task node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inject {
    /// Stable mutation id.
    pub mutation_id: String,
    /// Task node to add.
    pub task_node: state::TaskNode,
    /// Dependency edges entering the task node.
    pub incoming_edges: Vec<state::DependencyEdge>,
}

impl Inject {
    /// Creates an inject mutation and derives its stable id.
    pub fn new(task_node: state::TaskNode, incoming_edges: Vec<state::DependencyEdge>) -> Self {
        #[derive(Serialize)]
        struct Identity<'a> {
            task_instance_id: &'a str,
            lifecycle_epoch: u64,
            incoming_edges: &'a [state::DependencyEdge],
            lineage: &'a state::TaskLineage,
        }

        let mutation_id = stable_id(
            "task-network-inject",
            &Identity {
                task_instance_id: &task_node.task_instance_id,
                lifecycle_epoch: task_node.lifecycle_epoch,
                incoming_edges: &incoming_edges,
                lineage: &task_node.lineage,
            },
        );

        Self {
            mutation_id,
            task_node,
            incoming_edges,
        }
    }
}

/// Commit request produced from a command request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitRequest {
    /// Caller supplied command id.
    pub command_id: String,
    /// Revision the caller read before proposing the mutation.
    pub base_revision: u64,
    /// State hash the caller read before proposing the mutation.
    pub base_state_hash: String,
    /// Typed read preconditions to revalidate.
    pub read_preconditions: Vec<ReadPrecondition>,
    /// Mutation set to commit.
    pub mutation_set: Set,
}

/// Typed read precondition checked at command acceptance time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadPrecondition {
    /// Current revision must match.
    RevisionIs(u64),
    /// Current state hash must match.
    StateHashIs(String),
    /// Task node must exist.
    NodeExists(String),
    /// Task node must be absent.
    NodeAbsent(String),
    /// Task node must have the named status.
    NodeStatusIs {
        /// Task instance id to inspect.
        task_instance_id: String,
        /// Required task status.
        status: state::TaskStatus,
    },
    /// Dependency edge must exist.
    EdgeExists {
        /// Upstream task instance id.
        from: String,
        /// Downstream task instance id.
        to: String,
    },
    /// Dependency edge must be absent.
    EdgeAbsent {
        /// Upstream task instance id.
        from: String,
        /// Downstream task instance id.
        to: String,
    },
    /// Artifact must be available from the task.
    ArtifactAvailable {
        /// Task instance id to inspect.
        task_instance_id: String,
        /// Required artifact type id.
        artifact_type_id: String,
    },
    /// No dependency path may connect the two tasks.
    NoPath {
        /// Upstream task instance id.
        from: String,
        /// Downstream task instance id.
        to: String,
    },
    /// Claim must still fence the task.
    ClaimCurrent {
        /// Task instance id to inspect.
        task_instance_id: String,
        /// Current claim id.
        claim_id: String,
        /// Current claim revision.
        claim_revision: u64,
    },
    /// Publication must be retryable.
    PublicationPending(String),
}

/// Result of attempting to commit a mutation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommitResult {
    /// Mutation request committed.
    Committed(CommitRecord),
    /// Command id had already committed.
    Duplicate(CommitRecord),
    /// Mutation request was rejected.
    Rejected(Rejection),
}

/// Typed task network command rejection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rejection {
    /// Base revision was stale.
    StaleBase {
        /// Revision supplied by the caller.
        expected: u64,
        /// Current revision.
        actual: u64,
    },
    /// Base state hash was stale.
    StateHashMismatch {
        /// State hash supplied by the caller.
        expected: String,
        /// Current state hash.
        actual: String,
    },
    /// A read precondition failed.
    FailedPrecondition(ReadPrecondition),
    /// Command id has already been used for an incompatible request.
    DuplicateCommand(String),
    /// Graph validation failed.
    InvalidGraph(String),
    /// Lifecycle transition was invalid.
    InvalidLifecycleTransition(String),
    /// Claim no longer fences the task.
    StaleClaim {
        /// Stale claim id.
        claim_id: String,
    },
    /// Publication was already marked.
    PublicationAlreadyMarked(String),
}

/// Durable record for one accepted mutation set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitRecord {
    /// Stable commit id.
    pub commit_id: String,
    /// Command id that accepted this commit.
    pub command_id: String,
    /// Stable task network identifier.
    pub network_id: String,
    /// Revision that accepted this commit.
    pub revision: u64,
    /// State hash before this commit.
    pub previous_state_hash: String,
    /// State hash after this commit.
    pub state_hash: String,
    /// Accepted mutation set.
    pub mutation_set: Set,
}

impl CommitRecord {
    /// Creates a commit record and derives its stable id.
    pub fn new(
        command_id: impl Into<String>,
        network_id: impl Into<String>,
        revision: u64,
        previous_state_hash: impl Into<String>,
        state_hash: impl Into<String>,
        mutation_set: Set,
    ) -> Self {
        #[derive(Serialize)]
        struct Identity<'a> {
            command_id: &'a str,
            network_id: &'a str,
            revision: u64,
            previous_state_hash: &'a str,
            state_hash: &'a str,
            set_id: &'a str,
        }

        let command_id = command_id.into();
        let network_id = network_id.into();
        let previous_state_hash = previous_state_hash.into();
        let state_hash = state_hash.into();
        let commit_id = stable_id(
            "task-network-commit",
            &Identity {
                command_id: &command_id,
                network_id: &network_id,
                revision,
                previous_state_hash: &previous_state_hash,
                state_hash: &state_hash,
                set_id: &mutation_set.set_id,
            },
        );

        Self {
            commit_id,
            command_id,
            network_id,
            revision,
            previous_state_hash,
            state_hash,
            mutation_set,
        }
    }
}
