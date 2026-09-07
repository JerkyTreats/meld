//! Task network journal records.
//!
//! Owner: task network.
//! Inputs: accepted command reductions.
//! Outputs: append-only revision records for graph commits, dispatch claims,
//! task outcomes, and publication marks.
//! Does not own: this module does not persist records or reduce state.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::journal::JournalRecord;
//!
//! fn accepts_record(record: JournalRecord) -> JournalRecord {
//!     record
//! }
//! ```

use crate::task_network::{dispatch, mutation::CommitRecord, outcome::Publication};
use serde::{Deserialize, Serialize};

/// One accepted journal record in the task network revision stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JournalRecord {
    /// Durable consumer decision over one exact Agent-authorized Task.
    Admission(Box<crate::task_admission::TaskAdmissionRecord>),
    /// Accepted graph mutation commit.
    Commit(CommitRecord),
    /// Accepted dispatch claim.
    Claim(dispatch::Claim),
    /// Ready work consolidated before claiming, with exact input provenance.
    SharedWork(Box<super::sharing::ReadyWorkSharing>),
    /// Accepted task outcome and pending publication handoff.
    Outcome {
        /// Pending publication created for the outcome.
        publication: Publication,
    },
    /// Accepted publication state mark.
    Publication(Publication),
}
