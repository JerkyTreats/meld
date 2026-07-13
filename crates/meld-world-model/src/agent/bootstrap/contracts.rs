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

/// Recovery posture owned by the world-model bootstrap boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentBootstrapErrorClass {
    /// Re-run the same idempotent bootstrap input after supervisor recovery.
    Retryable,
    /// Stop automatic recovery because input or durable identity diverged.
    Fatal,
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

impl AgentBootstrapError {
    /// Return the recovery posture for this domain-owned failure.
    pub fn classification(&self) -> AgentBootstrapErrorClass {
        match self {
            Self::Storage { .. } => AgentBootstrapErrorClass::Retryable,
            Self::Validation { .. } | Self::Conflict { .. } | Self::LegacyDirectiveConflict(_) => {
                AgentBootstrapErrorClass::Fatal
            }
        }
    }

    /// Return the stable worker diagnostic code for this failure variant.
    pub fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "agent_bootstrap_validation_failed",
            Self::Conflict { .. } => "agent_bootstrap_conflict",
            Self::LegacyDirectiveConflict(_) => "agent_bootstrap_legacy_directive_conflict",
            Self::Storage { .. } => "agent_bootstrap_storage_failed",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{AgentBootstrapError, AgentBootstrapErrorClass};

    #[test]
    fn storage_is_retryable_without_misclassifying_divergence() {
        let storage = AgentBootstrapError::Storage {
            message: "temporarily unavailable".to_string(),
        };
        let conflict = AgentBootstrapError::Conflict {
            field: "bootstrap.activation_hash".to_string(),
            configured_value_hash: "configured".to_string(),
            durable_value_hash: "durable".to_string(),
        };

        assert_eq!(
            storage.classification(),
            AgentBootstrapErrorClass::Retryable
        );
        assert_eq!(storage.diagnostic_code(), "agent_bootstrap_storage_failed");
        assert_eq!(conflict.classification(), AgentBootstrapErrorClass::Fatal);
        assert_eq!(conflict.diagnostic_code(), "agent_bootstrap_conflict");
    }
}
