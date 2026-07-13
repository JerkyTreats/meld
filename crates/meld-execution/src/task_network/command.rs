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

use crate::task_network::{contracts::stable_hash, dispatch, mutation, outcome};
use serde::{Deserialize, Serialize};

/// Current durable command response authentication schema.
pub const RESPONSE_AUTHENTICATION_SCHEMA_VERSION: u32 = 1;

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

/// Tamper-evident binding for one durable command response record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAuthentication {
    schema_version: u32,
    response_digest: String,
}

impl ResponseAuthentication {
    /// Bind one response to its command and stored request identity.
    pub fn bind(command_id: &str, request_hash: &str, response: &Response) -> Result<Self, String> {
        if command_id.trim().is_empty() || request_hash.trim().is_empty() {
            return Err("command response authentication identity must be non-empty".to_string());
        }
        Ok(Self {
            schema_version: RESPONSE_AUTHENTICATION_SCHEMA_VERSION,
            response_digest: response_authentication_digest(command_id, request_hash, response),
        })
    }

    /// Validate the schema and exact response binding.
    pub fn validate(
        &self,
        command_id: &str,
        request_hash: &str,
        response: &Response,
    ) -> Result<(), String> {
        if self.schema_version != RESPONSE_AUTHENTICATION_SCHEMA_VERSION {
            return Err("unsupported command response authentication schema".to_string());
        }
        let expected = Self::bind(command_id, request_hash, response)?;
        if self.response_digest != expected.response_digest {
            return Err("command response authentication digest mismatch".to_string());
        }
        Ok(())
    }

    /// Return the durable authentication schema version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Borrow the response digest.
    pub fn response_digest(&self) -> &str {
        &self.response_digest
    }
}

/// Exact durable command outcome returned by the task-network authority query.
///
/// The authority constructs this receipt from paired request and response
/// records after validating their exact durable binding.
#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeReceipt {
    command_id: String,
    request_hash: String,
    request: Request,
    response: Response,
}

impl OutcomeReceipt {
    pub(crate) fn issued(
        command_id: String,
        request_hash: String,
        request: Request,
        response: Response,
    ) -> Self {
        Self {
            command_id,
            request_hash,
            request,
            response,
        }
    }

    /// Borrow the exact command id proven by this authority receipt.
    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    /// Borrow the stored request identity hash.
    pub fn request_hash(&self) -> &str {
        &self.request_hash
    }

    /// Borrow the exact command request proven by this authority receipt.
    pub fn request(&self) -> &Request {
        &self.request
    }

    /// Borrow the original response stored for the command.
    pub fn response(&self) -> &Response {
        &self.response
    }
}

/// Return the canonical task-network hash of one complete command request.
pub fn request_hash(request: &Request) -> String {
    stable_hash(request)
}

fn response_authentication_digest(
    command_id: &str,
    request_hash: &str,
    response: &Response,
) -> String {
    #[derive(Serialize)]
    struct DigestInput<'a> {
        schema_version: u32,
        command_id: &'a str,
        request_hash: &'a str,
        response: &'a Response,
    }

    stable_hash(&DigestInput {
        schema_version: RESPONSE_AUTHENTICATION_SCHEMA_VERSION,
        command_id,
        request_hash,
        response,
    })
}
