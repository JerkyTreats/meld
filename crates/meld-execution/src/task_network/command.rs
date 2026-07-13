//! Serialized task network command boundary.
//!
//! Owner: task network.
//! Inputs: mutation proposals, dispatch claim requests, task outcomes, and
//! publication marks.
//! Outputs: accepted, duplicate, or rejected command responses.
//! Does not own: this module does not reduce state directly.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::command::{Command, Request};
//! use meld_execution::task_network::mutation::Set;
//! use meld_execution::task_network::state::NetworkState;
//!
//! let state = NetworkState::empty("network-a");
//! let command = Request {
//!     command_id: "command-a".to_string(),
//!     network_id: "network-a".to_string(),
//!     base_revision: state.revision,
//!     base_state_hash: state.state_hash,
//!     read_preconditions: vec![],
//!     command: Command::ApplyMutationSet(Set::empty(
//!         "network-a",
//!         "composition-a",
//!         "once",
//!     )),
//! };
//!
//! assert_eq!(command.base_revision, 0);
//! ```

use crate::task_network::{dispatch, mutation, outcome};
use serde::{Deserialize, Serialize};

/// Task network command request with optimistic concurrency metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Caller supplied command id.
    pub command_id: String,
    /// Stable task network identifier.
    pub network_id: String,
    /// Revision the caller read before proposing the command.
    pub base_revision: u64,
    /// State hash the caller read before proposing the command.
    pub base_state_hash: String,
    /// Typed read preconditions that must still hold at acceptance time.
    pub read_preconditions: Vec<mutation::ReadPrecondition>,
    /// Command payload.
    pub command: Command,
}

/// Task network command payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Applies a graph mutation set produced by planning lowering.
    ApplyMutationSet(mutation::Set),
    /// Claims one ready task before task runtime execution.
    ClaimReadyTask(dispatch::Request),
    /// Records a fenced task runtime outcome.
    RecordTaskOutcome(dispatch::Outcome),
    /// Records a fenced task outcome with complete semantic lineage.
    RecordAttributedTaskOutcome(dispatch::AttributedOutcome),
    /// Marks one publication outbox record.
    MarkPublication(outcome::Publication),
}

/// Command response persisted by command id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Response {
    /// Command was accepted at a new revision.
    Accepted {
        /// Revision after command acceptance.
        revision: u64,
        /// State hash after command acceptance.
        state_hash: String,
    },
    /// Command id was already accepted and its prior result was replayed.
    Duplicate {
        /// Revision from the prior accepted command.
        revision: u64,
        /// State hash from the prior accepted command.
        state_hash: String,
    },
    /// Command was rejected before mutation.
    Rejected(mutation::Rejection),
}
