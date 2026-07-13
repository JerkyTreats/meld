//! Bounded belief assessment actor owned by the world-model domain.

use std::sync::Arc;

use crate::belief::{
    BeliefGraphQuery, BeliefRuntime, BeliefStore, BranchScope, ConfigSnapshot,
    MAX_BELIEF_EVIDENCE_WINDOW_ITEMS,
};
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Canonical recurring belief assessment runtime identity.
pub const BELIEF_ASSESSMENT_ACTOR_ID: &str = "world_model.belief_assessment";

/// Maximum dirty belief keys one actor tick may select.
pub const MAX_BELIEF_ASSESSMENT_ITEMS: usize = 1024;

/// Request to assess one configured graph subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefAssessmentRequest {
    /// Graph subject whose selected anchor supplies evidence.
    pub subject: DomainObjectRef,
    /// Perspective kind used to select the current graph anchor.
    pub anchor_perspective_kind: String,
    /// Perspective id used to select the current graph anchor.
    pub anchor_perspective_id: String,
    /// Active supervisor lease id recorded as assessment ownership.
    pub lease_owner_id: String,
}

/// Request for one bounded dirty-key assessment tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefDirtyKeyTickRequest {
    /// Maximum dirty belief keys selected in deterministic key order.
    pub max_items: usize,
    /// Active supervisor lease id recorded as assessment ownership.
    pub lease_owner_id: String,
}

/// Stable diagnostic issue from one belief assessment tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefRuntimeIssue {
    /// Optional belief key or graph subject identity.
    pub item_id: Option<String>,
    /// Stable machine-readable category.
    pub code: String,
    /// Human-readable diagnostic detail.
    pub message: String,
}

/// Bounded diagnostic report from one belief assessment tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefRuntimeTickReport {
    /// Canonical actor identity.
    pub actor_id: String,
    /// Durable belief-owned assessment progress before work.
    pub input_sequence: u64,
    /// Highest belief-owned assessment progress committed by this tick.
    pub output_sequence: u64,
    /// Source watermark acknowledged by canonical evidence ingestion receipts.
    pub source_high_water: u64,
    /// Durable logical clock used only for recurring lease expiry.
    pub lease_clock: u64,
    /// Candidate subjects or dirty keys selected for work.
    pub selected_count: usize,
    /// Belief revisions committed by this tick.
    pub committed_count: usize,
    /// Expired assessment leases recovered before reassessment.
    pub recovered_lease_count: usize,
    /// Selected items that produced no durable revision.
    pub no_work_count: usize,
    /// Retryable issues that preserve durable input for a later tick.
    pub retryable_errors: Vec<BeliefRuntimeIssue>,
    /// Fatal issues that require configuration or data repair.
    pub fatal_errors: Vec<BeliefRuntimeIssue>,
    /// True when more deterministic candidates remained after selection.
    pub budget_exhausted: bool,
}

impl BeliefRuntimeTickReport {
    fn new(input_sequence: u64, source_high_water: u64, lease_clock: u64) -> Self {
        Self {
            actor_id: BELIEF_ASSESSMENT_ACTOR_ID.to_string(),
            input_sequence,
            output_sequence: input_sequence,
            source_high_water,
            lease_clock,
            selected_count: 0,
            committed_count: 0,
            recovered_lease_count: 0,
            no_work_count: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }
}

/// Domain-owned actor that selects and assesses belief work.
pub struct BeliefAssessmentActor {
    store: Arc<BeliefStore>,
    runtime: BeliefRuntime,
}

impl BeliefAssessmentActor {
    /// Bind durable stores and validated belief-family configuration.
    pub fn new<Q>(
        store: Arc<BeliefStore>,
        graph_query: Arc<Q>,
        config: ConfigSnapshot,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self
    where
        Q: BeliefGraphQuery + 'static,
    {
        let graph_query: Arc<dyn BeliefGraphQuery> = graph_query;
        let runtime = BeliefRuntime::from_graph_query(
            Arc::clone(&store),
            graph_query,
            config,
            perspective,
            branch_scope,
        );
        Self { store, runtime }
    }

    /// Bind an already erased graph query contract.
    pub fn from_graph_query(
        store: Arc<BeliefStore>,
        graph_query: Arc<dyn BeliefGraphQuery>,
        config: ConfigSnapshot,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        let runtime = BeliefRuntime::from_graph_query(
            Arc::clone(&store),
            graph_query,
            config,
            perspective,
            branch_scope,
        );
        Self { store, runtime }
    }

    /// Assess one configured graph subject without treating a missing anchor as fatal.
    pub fn assess_subject(&self, request: BeliefAssessmentRequest) -> BeliefRuntimeTickReport {
        let mut report = match starting_report(&self.store) {
            Ok(report) => report,
            Err(error) => {
                let mut report = BeliefRuntimeTickReport::new(0, 0, 0);
                push_storage_issue(&mut report, None, error);
                return report;
            }
        };
        report.selected_count = 1;
        if let Err(message) = validate_owner(&request.lease_owner_id) {
            report.fatal_errors.push(issue(
                Some(request.subject.index_key()),
                "invalid_lease_owner",
                message,
            ));
            return report;
        }
        match self.runtime.assess_subject(
            &request.subject,
            &request.anchor_perspective_kind,
            &request.anchor_perspective_id,
            &request.lease_owner_id,
        ) {
            Ok(result) => {
                report.committed_count = 1;
                report.output_sequence = report
                    .output_sequence
                    .max(result.assessment_progress_sequence);
            }
            Err(StorageError::InvalidPath(message)) if message == "missing graph anchor" => {
                report.no_work_count = 1;
                report.retryable_errors.push(issue(
                    Some(request.subject.index_key()),
                    "missing_graph_anchor",
                    message,
                ));
            }
            Err(error) => push_storage_issue(&mut report, Some(request.subject.index_key()), error),
        }
        match self.store.assessment_lease_clock() {
            Ok(lease_clock) => report.lease_clock = lease_clock,
            Err(error) => push_storage_issue(&mut report, None, error),
        }
        report
    }

    /// Recover expired leases and assess a deterministic bounded dirty-key window.
    pub fn tick_dirty(&self, request: BeliefDirtyKeyTickRequest) -> BeliefRuntimeTickReport {
        let mut report = match starting_report(&self.store) {
            Ok(report) => report,
            Err(error) => {
                let mut report = BeliefRuntimeTickReport::new(0, 0, 0);
                push_storage_issue(&mut report, None, error);
                return report;
            }
        };
        if let Err(message) = validate_tick_request(&request) {
            report
                .fatal_errors
                .push(issue(None, "invalid_request", message));
            return report;
        }
        if let Err(error) = self.runtime.persist_config() {
            push_storage_issue(&mut report, None, error);
            return report;
        }
        report.lease_clock = match self.store.advance_assessment_lease_clock() {
            Ok(sequence) => sequence,
            Err(error) => {
                push_storage_issue(&mut report, None, error);
                return report;
            }
        };
        let page = match self.store.select_dirty_work_page(request.max_items) {
            Ok(page) => page,
            Err(error) => {
                push_storage_issue(&mut report, None, error);
                return report;
            }
        };
        report.budget_exhausted = page.has_more;
        let mut expected_cursor = page.expected_cursor;
        for state in page.items {
            let item_id = state.belief_key.index_key();
            report.selected_count += 1;
            match self
                .store
                .recover_expired_lease_for_key(&state.belief_key, report.lease_clock)
            {
                Ok(Some(_)) => report.recovered_lease_count += 1,
                Ok(None) => {}
                Err(error) => {
                    push_storage_issue(&mut report, Some(item_id.clone()), error);
                }
            }
            match self.runtime.assess_dirty_key_bounded_outcome(
                &state.belief_key,
                &request.lease_owner_id,
                MAX_BELIEF_EVIDENCE_WINDOW_ITEMS,
                report.lease_clock,
            ) {
                Ok(outcome) => {
                    if outcome.assessment.is_some() {
                        report.committed_count += 1;
                    } else {
                        report.no_work_count += 1;
                    }
                    if let Some(progress_sequence) = outcome.progress_sequence {
                        report.output_sequence = report.output_sequence.max(progress_sequence);
                    }
                }
                Err(error) => push_storage_issue(&mut report, Some(item_id.clone()), error),
            }
            match self
                .store
                .acknowledge_dirty_work_cursor(expected_cursor.as_deref(), &state.belief_key)
            {
                Ok(next) => expected_cursor = Some(next),
                Err(error) => {
                    push_storage_issue(&mut report, Some(item_id), error);
                    break;
                }
            }
        }
        report
    }
}

fn starting_report(store: &BeliefStore) -> Result<BeliefRuntimeTickReport, StorageError> {
    Ok(BeliefRuntimeTickReport::new(
        store.assessment_progress_sequence()?,
        store.assessment_source_high_water()?,
        store.assessment_lease_clock()?,
    ))
}

fn validate_tick_request(request: &BeliefDirtyKeyTickRequest) -> Result<(), String> {
    validate_owner(&request.lease_owner_id)?;
    if !(1..=MAX_BELIEF_ASSESSMENT_ITEMS).contains(&request.max_items) {
        return Err(format!(
            "belief assessment budget must be in 1..={MAX_BELIEF_ASSESSMENT_ITEMS}"
        ));
    }
    Ok(())
}

fn validate_owner(owner_id: &str) -> Result<(), String> {
    if owner_id.trim().is_empty() {
        Err("supervisor lease owner id must be non-empty".to_string())
    } else {
        Ok(())
    }
}

fn push_storage_issue(
    report: &mut BeliefRuntimeTickReport,
    item_id: Option<String>,
    error: StorageError,
) {
    let retryable = matches!(
        error,
        StorageError::Backpressure(_)
            | StorageError::Unavailable(_)
            | StorageError::DurabilityIndeterminate(_)
            | StorageError::IoError(_)
    );
    let code = match &error {
        StorageError::Backpressure(_) => "assessment_conflict",
        StorageError::Unavailable(_) => "storage_unavailable",
        StorageError::DurabilityIndeterminate(_) => "durability_indeterminate",
        StorageError::IoError(_) => "storage_io",
        _ => "assessment_invalid",
    };
    let target = if retryable {
        &mut report.retryable_errors
    } else {
        &mut report.fatal_errors
    };
    target.push(issue(item_id, code, error.to_string()));
}

fn issue(
    item_id: Option<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) -> BeliefRuntimeIssue {
    BeliefRuntimeIssue {
        item_id,
        code: code.into(),
        message: message.into(),
    }
}
