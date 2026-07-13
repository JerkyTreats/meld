//! Bootstrap progress, diagnostics, and failure contracts.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::activation::{AgentBootstrapReceipt, LegacyDirectiveMigrationConflict};

/// Maximum diagnostic entries returned by one bootstrap invocation.
pub const MAX_BOOTSTRAP_DIAGNOSTICS: usize = 8;

/// Durable bootstrap stage used for exact reopen recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentBootstrapStage {
    /// Bootstrap identity and started activation are durable.
    Started,
    /// Belief family configuration is durably bound.
    BeliefConfigured,
    /// Directive and canonical seed agent are durable.
    AgentRegistered,
    /// Configured curation rule is durable.
    RuleRegistered,
    /// Deterministic belief subscription is durable.
    SubscriptionBound,
    /// Every configured seed product has been durably confirmed.
    ProductsConfirmed,
    /// Final bootstrap receipt is durable.
    Completed,
}

/// Durable lifecycle summary for a one-shot bootstrap identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentBootstrapProgressStatus {
    /// At least the bootstrap identity and activation start are durable.
    Started,
    /// Every semantic bootstrap product and final receipt are durable.
    Completed,
}

/// Durable progress recovered exclusively from the world-model store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBootstrapProgress {
    /// Stable logical bootstrap identity.
    pub bootstrap_id: String,
    /// Stable product activation identity.
    pub activation_id: String,
    /// Root normalized activation hash.
    pub activation_hash: String,
    /// Hash of the exact owner-scoped world-model input.
    pub input_hash: String,
    /// Most recent durable stage.
    pub stage: AgentBootstrapStage,
    /// Coarse lifecycle state used by runtime recovery.
    pub status: AgentBootstrapProgressStatus,
    /// Bootstrap-owned sequence assigned to this progress update.
    pub updated_at_seq: u64,
}

/// One bounded diagnostic emitted for a bootstrap stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBootstrapDiagnostic {
    /// Stage inspected or advanced.
    pub stage: AgentBootstrapStage,
    /// Stable bounded outcome label.
    pub disposition: String,
}

/// Result of one bootstrap invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBootstrapReport {
    /// Final durable receipt.
    pub receipt: AgentBootstrapReceipt,
    /// Final durable progress.
    pub progress: AgentBootstrapProgress,
    /// Whether this invocation advanced any durable stage.
    pub work_performed: bool,
    /// Stage already durable when this invocation began.
    pub resumed_from: Option<AgentBootstrapStage>,
    /// Bounded stage diagnostics.
    pub diagnostics: Vec<AgentBootstrapDiagnostic>,
}

/// Failure returned by world-model bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentBootstrapError {
    /// The source-neutral owner package failed validation.
    Validation {
        /// Stable invalid field path.
        field: String,
        /// Bounded validation detail.
        message: String,
    },
    /// A stable identity already names divergent durable content.
    Conflict {
        /// Semantic field or record identity that diverged.
        field: String,
        /// Hash of the configured value.
        configured_value_hash: String,
        /// Hash of the durable value.
        durable_value_hash: String,
    },
    /// A legacy embedded directive record diverged from activation input.
    LegacyDirectiveConflict(Box<LegacyDirectiveMigrationConflict>),
    /// Durable storage could not complete or decode a stage.
    Storage {
        /// Bounded storage detail.
        message: String,
    },
}

impl fmt::Display for AgentBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation { field, message } => write!(formatter, "{field}: {message}"),
            Self::Conflict { field, .. } => {
                write!(formatter, "bootstrap content conflict at '{field}'")
            }
            Self::LegacyDirectiveConflict(conflict) => write!(
                formatter,
                "legacy directive migration conflict at '{:?}'",
                conflict.field
            ),
            Self::Storage { message } => write!(formatter, "bootstrap storage failure: {message}"),
        }
    }
}

impl Error for AgentBootstrapError {}
