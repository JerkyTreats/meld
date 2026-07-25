//! Canonical aggregate package outcome contract.
//!
//! Owner: execution task network. One package run over a selected tree
//! completes as a whole, and only that aggregate completion is eligible to
//! become selected-tree satisfaction evidence — per-folder progress never
//! satisfies the goal early. The aggregate preserves the canonical
//! per-task outcomes, per-folder publish results, and artifact identities
//! intact rather than summarizing them away.
//!
//! This module carries no implementation. The dispatch and publication
//! workstreams bind production and appending.

use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

use crate::task_network::contracts::stable_id;

/// Event type appended when a package run completes satisfactorily.
pub const AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE: &str = "execution.package.completed";

/// Event type appended when a package run completes unsatisfactorily.
pub const AGGREGATE_PACKAGE_FAILED_EVENT_TYPE: &str = "execution.package.failed";

/// Identity of the canonical aggregate outcome contract, cited by
/// available-action bindings.
pub const AGGREGATE_OUTCOME_CONTRACT_ID: &str = "execution.package.aggregate.v1";

/// Terminal status of one aggregate package run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregatePackageStatus {
    /// Every required per-folder generation and publication completed.
    Completed,
    /// The run terminated without completing all required work.
    Failed,
}

/// Per-folder publication result preserved inside the aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderPublicationResult {
    /// Folder subject the work targeted.
    pub folder: DomainObjectRef,
    /// Task instance that produced the folder result.
    pub task_instance_id: String,
    /// Canonical task outcome identity, unchanged from dispatch.
    pub outcome_id: String,
    /// Artifact records the task produced, by identity.
    pub artifact_ids: Vec<String>,
}

/// Canonical outcome of one complete package run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregatePackageOutcome {
    /// Stable aggregate identity, derived from run and network identity.
    pub aggregate_id: String,
    /// Package run this aggregate closes.
    pub package_run_id: String,
    /// Task network the run executed in.
    pub network_id: String,
    /// Selected-tree subject the stewardship expression targets. Carried
    /// so world-model mapping needs no caller reattachment.
    pub selected_scope: DomainObjectRef,
    /// Terminal status of the run.
    pub status: AggregatePackageStatus,
    /// Per-folder results, intact and in stable order.
    pub folder_results: Vec<FolderPublicationResult>,
}

impl AggregatePackageOutcome {
    /// Derive the stable aggregate identity for a package run.
    pub fn derive_aggregate_id(package_run_id: &str, network_id: &str) -> String {
        #[derive(Serialize)]
        struct Identity<'a> {
            package_run_id: &'a str,
            network_id: &'a str,
        }
        stable_id(
            "task-network-aggregate",
            &Identity {
                package_run_id,
                network_id,
            },
        )
    }
}
