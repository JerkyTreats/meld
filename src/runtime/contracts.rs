//! Runtime worker diagnostic contracts.

use meld_execution::task_network::PublicationBridgeReport;
use meld_world_model::world_state::graph::runtime::GraphCatchUpReport;

/// Bounded work request shared by runtime host adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkBudget {
    /// Maximum durable input items to attempt during one tick.
    pub max_items: usize,
}

/// Diagnostic scope for one worker tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerScope {
    /// Owning domain identifier.
    pub domain_id: String,
    /// Optional event stream or aggregate stream identifier.
    pub stream_id: Option<String>,
    /// Optional stable owner-specific unit of work.
    pub work_key: Option<String>,
    /// Optional world-model agent identifier.
    pub agent_id: Option<String>,
    /// Optional perspective identifier.
    pub perspective_key: Option<String>,
    /// Optional branch identifier.
    pub branch_id: Option<String>,
    /// Optional subject index key.
    pub subject_key: Option<String>,
}

/// Durable input or output checkpoint observed by a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCheckpoint {
    /// Stable checkpoint name.
    pub name: String,
    /// Monotonic checkpoint value copied from the owning domain.
    pub value: u64,
}

/// Diagnostic issue observed during a worker tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTickIssue {
    /// Optional domain item identifier.
    pub item_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Host-facing report from one bounded worker tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTickReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Diagnostic scope copied from the owning domain.
    pub scope: WorkerScope,
    /// Durable input cursor or revision before work.
    pub input_checkpoint: WorkerCheckpoint,
    /// Durable output cursor or revision after work.
    pub output_checkpoint: WorkerCheckpoint,
    /// Durable input items attempted.
    pub items_attempted: usize,
    /// Durable business outputs committed.
    pub items_committed: usize,
    /// Retryable diagnostics observed during the tick.
    pub retryable_errors: Vec<WorkerTickIssue>,
    /// Fatal diagnostics observed during the tick.
    pub fatal_errors: Vec<WorkerTickIssue>,
    /// True when the worker stopped because its budget was consumed.
    pub budget_exhausted: bool,
}

impl WorkerTickReport {
    /// Returns true when the report indicates durable progress.
    pub fn made_progress(&self) -> bool {
        self.output_checkpoint.value > self.input_checkpoint.value || self.items_committed > 0
    }
}

impl From<GraphCatchUpReport> for WorkerTickReport {
    fn from(report: GraphCatchUpReport) -> Self {
        Self {
            actor_id: report.actor_id,
            scope: WorkerScope {
                domain_id: "world_state".to_string(),
                stream_id: None,
                work_key: Some("graph".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: report.input_event_seq,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: report.output_event_seq,
            },
            items_attempted: report.events_attempted,
            items_committed: report.traversal_events_applied,
            retryable_errors: report
                .retryable_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.item_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            fatal_errors: report
                .fatal_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.item_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            budget_exhausted: report.budget_exhausted,
        }
    }
}

impl From<PublicationBridgeReport> for WorkerTickReport {
    fn from(report: PublicationBridgeReport) -> Self {
        Self {
            actor_id: report.actor_id,
            scope: WorkerScope {
                domain_id: "execution".to_string(),
                stream_id: Some(report.scope.network_id),
                work_key: Some("publication_outbox".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.input_revision,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.output_revision,
            },
            items_attempted: report.items_attempted,
            items_committed: report.items_committed,
            retryable_errors: report
                .retryable_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.publication_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            fatal_errors: report
                .fatal_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.publication_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            budget_exhausted: report.budget_exhausted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_execution::task_network::publication::{
        PublicationBridgeIssue, PublicationBridgeScope,
    };

    #[test]
    fn graph_report_maps_to_worker_report() {
        let report = GraphCatchUpReport {
            actor_id: "world_state.graph.reducer".to_string(),
            input_event_seq: 1,
            output_event_seq: 3,
            events_attempted: 2,
            traversal_events_applied: 1,
            derived_events_appended: 1,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        };

        let worker: WorkerTickReport = report.into();

        assert_eq!(worker.scope.domain_id, "world_state");
        assert_eq!(worker.scope.work_key.as_deref(), Some("graph"));
        assert_eq!(worker.input_checkpoint.name, "event_spine_seq");
        assert_eq!(worker.output_checkpoint.value, 3);
        assert_eq!(worker.items_attempted, 2);
        assert_eq!(worker.items_committed, 1);
        assert!(worker.made_progress());
    }

    #[test]
    fn publication_report_maps_to_worker_report() {
        let report = PublicationBridgeReport {
            actor_id: "execution.task_network.publication".to_string(),
            scope: PublicationBridgeScope {
                network_id: "network-a".to_string(),
                session_id: "session-a".to_string(),
                worker_id: "worker-a".to_string(),
            },
            input_revision: 4,
            output_revision: 5,
            items_attempted: 1,
            items_committed: 0,
            retryable_errors: vec![PublicationBridgeIssue {
                publication_id: Some("publication-a".to_string()),
                code: "publication_append_failed".to_string(),
                message: "append failed".to_string(),
            }],
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            results: Vec::new(),
        };

        let worker: WorkerTickReport = report.into();

        assert_eq!(worker.scope.domain_id, "execution");
        assert_eq!(worker.scope.stream_id.as_deref(), Some("network-a"));
        assert_eq!(worker.input_checkpoint.name, "task_network_revision");
        assert_eq!(worker.output_checkpoint.value, 5);
        assert_eq!(
            worker.retryable_errors[0].item_id.as_deref(),
            Some("publication-a")
        );
        assert!(worker.made_progress());
    }
}
