//! Bounded durable actor for planner-facing world-model projection.

use std::sync::Arc;

use meld_lang::{Proposition, WorldState};

use crate::belief::{BeliefQuery, BeliefStore};
use crate::error::StorageError;
use crate::planner::contracts::{
    PlannerFieldProjectionConfig, PlannerGraphScope, PlannerHydrationRefs,
    PlannerProjectionContext, PlannerProjectionError, PlannerProjectionFrame,
    PlannerProjectionFrameIdentity, PlannerProjectionInput, PlannerProjectionOutput,
    PlannerProjectionRequest, PLANNER_PROJECTION_VERSION,
};
use crate::planner::projection::project_world_state;
use crate::planner::store::{PlannerProjectionStore, PlannerProjectionTerminalCapability};
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::TraversalQuery;

/// Canonical recurring planner projection runtime identity.
pub const PLANNER_PROJECTION_ACTOR_ID: &str = "world_model.planner_projection";

/// Hard maximum projection requests one actor tick may select.
pub const MAX_PLANNER_PROJECTION_ITEMS: usize = 1024;

/// Input for one deterministic bounded planner projection tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerProjectionTickRequest {
    /// Maximum pending requests selected in creation and identity order.
    pub max_items: usize,
}

/// Stable diagnostic emitted by one planner projection tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerProjectionIssue {
    /// Request identity when the issue belongs to one selected item.
    pub item_id: Option<String>,
    /// Stable machine-readable category.
    pub code: String,
    /// Human-readable bounded diagnostic detail.
    pub message: String,
}

/// Native report for one bounded planner projection tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerProjectionTickReport {
    /// Canonical actor identity.
    pub actor_id: String,
    /// Highest durable request checkpoint observed in the selected window.
    pub input_request_sequence: u64,
    /// Highest durable terminal checkpoint committed by this tick.
    pub output_request_sequence: u64,
    /// Pending requests selected for work.
    pub selected_count: usize,
    /// Requests completed with durable frames.
    pub completed_count: usize,
    /// Requests durably transitioned to failed.
    pub failed_count: usize,
    /// Retryable issues that leave a request pending.
    pub retryable_errors: Vec<PlannerProjectionIssue>,
    /// Fatal issues caused by invalid request or projection content.
    pub fatal_errors: Vec<PlannerProjectionIssue>,
    /// True when more deterministic pending work remains.
    pub budget_exhausted: bool,
}

impl PlannerProjectionTickReport {
    fn empty() -> Self {
        Self {
            actor_id: PLANNER_PROJECTION_ACTOR_ID.to_string(),
            input_request_sequence: 0,
            output_request_sequence: 0,
            selected_count: 0,
            completed_count: 0,
            failed_count: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }
}

/// World-model-owned actor that materializes durable planner frames.
pub struct PlannerProjectionActor {
    projection_store: Arc<PlannerProjectionStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
}

impl PlannerProjectionActor {
    /// Bind planner authority and public lower-layer query stores.
    pub fn new(
        projection_store: Arc<PlannerProjectionStore>,
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
    ) -> Self {
        Self {
            projection_store,
            belief_store,
            traversal_store,
        }
    }

    /// Project one deterministic bounded pending-request window.
    pub fn tick(&self, request: PlannerProjectionTickRequest) -> PlannerProjectionTickReport {
        let mut report = PlannerProjectionTickReport::empty();
        if let Err(message) = validate_tick_request(&request) {
            report
                .fatal_errors
                .push(issue(None, "invalid_request", message));
            return report;
        }
        // A prior terminal transaction may have committed before its flush
        // became indeterminate. Reflush the authority before selecting work so
        // a later tick resolves durability even when no pending item remains.
        if let Err(error) = self.projection_store.flush() {
            push_storage_issue(&mut report, None, error);
            return report;
        }
        let selection = match self
            .projection_store
            .pending_requests_bounded(request.max_items)
        {
            Ok(selection) => selection,
            Err(error) => {
                push_storage_issue(&mut report, None, error);
                return report;
            }
        };
        report.budget_exhausted = selection.budget_exhausted;
        report.selected_count = selection.records.len();
        report.input_request_sequence = selection
            .records
            .iter()
            .map(|record| record.updated_at_seq)
            .max()
            .unwrap_or(0);
        report.output_request_sequence = report.input_request_sequence;

        for record in selection.records {
            let terminal_capability = if record.request.attested_belief.is_some() {
                match self.projection_store.terminal_capability_for_actor(&record) {
                    Ok(capability) => Some(capability),
                    Err(error) => {
                        push_storage_issue(&mut report, Some(record.request.request_id), error);
                        continue;
                    }
                }
            } else {
                None
            };
            let Some(transition_sequence) = record.updated_at_seq.checked_add(1) else {
                report.fatal_errors.push(issue(
                    Some(record.request.request_id),
                    "request_sequence_exhausted",
                    "planner request sequence cannot advance beyond its durable checkpoint",
                ));
                continue;
            };
            match self.project_request(&record.request) {
                Ok(output) => {
                    let identity = match PlannerProjectionFrameIdentity::identified(
                        &record.request,
                        &output,
                    ) {
                        Ok(identity) => identity,
                        Err(error) => {
                            self.fail_projection(
                                &mut report,
                                &record.request.request_id,
                                record.updated_at_seq,
                                transition_sequence,
                                error.to_string(),
                                terminal_capability.as_ref(),
                            );
                            continue;
                        }
                    };
                    let frame = PlannerProjectionFrame {
                        identity,
                        output,
                        completed_at_seq: transition_sequence,
                    };
                    let completion = self.complete_projection(
                        terminal_capability.as_ref(),
                        &record.request.request_id,
                        record.updated_at_seq,
                        frame.clone(),
                        transition_sequence,
                    );
                    let completion =
                        if matches!(&completion, Err(StorageError::DurabilityIndeterminate(_))) {
                            self.complete_projection(
                                terminal_capability.as_ref(),
                                &record.request.request_id,
                                record.updated_at_seq,
                                frame,
                                transition_sequence,
                            )
                        } else {
                            completion
                        };
                    match completion {
                        Ok(completed) => {
                            report.completed_count += 1;
                            report.output_request_sequence =
                                report.output_request_sequence.max(completed.updated_at_seq);
                        }
                        Err(error) => {
                            push_storage_issue(&mut report, Some(record.request.request_id), error)
                        }
                    }
                }
                Err(PlannerProjectionError::Storage(error)) => {
                    push_storage_issue(&mut report, Some(record.request.request_id), error)
                }
                Err(error) => self.fail_projection(
                    &mut report,
                    &record.request.request_id,
                    record.updated_at_seq,
                    transition_sequence,
                    error.to_string(),
                    terminal_capability.as_ref(),
                ),
            }
        }
        report
    }

    fn complete_projection(
        &self,
        capability: Option<&PlannerProjectionTerminalCapability>,
        request_id: &str,
        expected_updated_at_seq: u64,
        frame: PlannerProjectionFrame,
        completed_at_seq: u64,
    ) -> Result<crate::planner::PlannerProjectionRequestRecord, StorageError> {
        match capability {
            Some(capability) => self.projection_store.complete_attested_for_actor(
                capability,
                request_id,
                expected_updated_at_seq,
                frame,
                completed_at_seq,
            ),
            None => self.projection_store.complete(
                request_id,
                expected_updated_at_seq,
                frame,
                completed_at_seq,
            ),
        }
    }

    fn project_request(
        &self,
        request: &PlannerProjectionRequest,
    ) -> Result<PlannerProjectionOutput, PlannerProjectionError> {
        let belief_query = BeliefQuery::new(self.belief_store.as_ref());
        let traversal_query = TraversalQuery::new(self.traversal_store.as_ref());
        let views = match self.projection_store.attested_view_for_request(request)? {
            Some(view) => vec![view],
            None => {
                belief_query.current_views_for_subject(&request.subject, &request.perspective)?
            }
        };
        let anchors = traversal_query.current_anchors_for_subject(&request.subject)?;
        let graph_scope = if anchors.is_empty() {
            None
        } else {
            Some(PlannerGraphScope {
                accessible: true,
                anchor_ids: anchors
                    .iter()
                    .map(|anchor| anchor.anchor_id.clone())
                    .collect(),
                source_fact_ids: anchors
                    .iter()
                    .flat_map(|anchor| anchor.source_fact_ids.clone())
                    .collect(),
            })
        };

        let mut propositions = Vec::new();
        let mut source_refs = Vec::new();
        let mut hydration_refs = PlannerHydrationRefs::default();
        let mut warnings = Vec::new();
        for dimension_id in &request.requested_dimensions {
            let belief_view = views
                .iter()
                .find(|view| {
                    view.key.dimension_id == *dimension_id
                        && view.key.branch_scope == request.branch_scope
                })
                .cloned();
            let field_config = belief_view
                .as_ref()
                .map(PlannerFieldProjectionConfig::from_belief_view)
                .unwrap_or_default();
            let projected = project_world_state(PlannerProjectionInput {
                context: PlannerProjectionContext {
                    subject: request.subject.clone(),
                    perspective: request.perspective.clone(),
                    branch_scope: request.branch_scope.clone(),
                    projection_version: PLANNER_PROJECTION_VERSION.to_string(),
                },
                belief_view,
                graph_scope: graph_scope.clone(),
                field_config,
            })?;
            propositions.extend(projected.world_state.propositions().iter().cloned());
            source_refs.extend(projected.source_refs);
            hydration_refs
                .evidence_ids
                .extend(projected.hydration_refs.evidence_ids);
            hydration_refs
                .source_fact_ids
                .extend(projected.hydration_refs.source_fact_ids);
            hydration_refs
                .graph_anchor_ids
                .extend(projected.hydration_refs.graph_anchor_ids);
            hydration_refs
                .revision_ids
                .extend(projected.hydration_refs.revision_ids);
            warnings.extend(projected.warnings);
        }
        sort_propositions(&mut propositions)?;
        propositions.dedup();
        source_refs.sort();
        source_refs.dedup();
        hydration_refs.sort_and_dedup();
        warnings.sort();
        warnings.dedup();
        Ok(PlannerProjectionOutput {
            world_state: WorldState::new(propositions)?,
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
            source_refs,
            hydration_refs,
            warnings,
        })
    }

    fn fail_projection(
        &self,
        report: &mut PlannerProjectionTickReport,
        request_id: &str,
        expected_updated_at_seq: u64,
        failed_at_seq: u64,
        message: String,
        capability: Option<&PlannerProjectionTerminalCapability>,
    ) {
        let message = bounded_message(message);
        let fail = || match capability {
            Some(capability) => self.projection_store.fail_attested_for_actor(
                capability,
                request_id,
                expected_updated_at_seq,
                message.clone(),
                failed_at_seq,
            ),
            None => self.projection_store.fail(
                request_id,
                expected_updated_at_seq,
                message.clone(),
                failed_at_seq,
            ),
        };
        let failure = fail();
        let failure = if matches!(&failure, Err(StorageError::DurabilityIndeterminate(_))) {
            fail()
        } else {
            failure
        };
        match failure {
            Ok(failed) => {
                report.failed_count += 1;
                report.output_request_sequence =
                    report.output_request_sequence.max(failed.updated_at_seq);
                report.fatal_errors.push(issue(
                    Some(request_id.to_string()),
                    "projection_failed",
                    message,
                ));
            }
            Err(error) => {
                push_storage_issue(report, Some(request_id.to_string()), error);
            }
        }
    }
}

fn validate_tick_request(request: &PlannerProjectionTickRequest) -> Result<(), String> {
    if !(1..=MAX_PLANNER_PROJECTION_ITEMS).contains(&request.max_items) {
        return Err(format!(
            "planner projection budget must be in 1..={MAX_PLANNER_PROJECTION_ITEMS}"
        ));
    }
    Ok(())
}

fn sort_propositions(propositions: &mut Vec<Proposition>) -> Result<(), PlannerProjectionError> {
    let mut keyed = propositions
        .drain(..)
        .map(|proposition| serde_json::to_string(&proposition).map(|key| (key, proposition)))
        .collect::<Result<Vec<_>, _>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    propositions.extend(keyed.into_iter().map(|(_, proposition)| proposition));
    Ok(())
}

fn push_storage_issue(
    report: &mut PlannerProjectionTickReport,
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
        StorageError::Backpressure(_) => "projection_conflict",
        StorageError::Unavailable(_) => "storage_unavailable",
        StorageError::DurabilityIndeterminate(_) => "durability_indeterminate",
        StorageError::IoError(_) => "storage_io",
        _ => "projection_invalid",
    };
    let target = if retryable {
        &mut report.retryable_errors
    } else {
        &mut report.fatal_errors
    };
    target.push(issue(item_id, code, bounded_message(error.to_string())));
}

fn issue(
    item_id: Option<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) -> PlannerProjectionIssue {
    PlannerProjectionIssue {
        item_id,
        code: code.into(),
        message: message.into(),
    }
}

fn bounded_message(message: String) -> String {
    if message.len() <= 1024 {
        return message;
    }
    let mut end = 1024;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::BranchScope;
    use crate::events::DomainObjectRef;
    use crate::planner::PlannerProjectionRequestStatus;
    use crate::world_state::graph::PerspectiveKey;

    #[test]
    fn actor_exactly_replays_an_indeterminate_terminal_flush() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let projection_store = Arc::new(PlannerProjectionStore::new(db.clone()).unwrap());
        let belief_store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal_store = Arc::new(TraversalStore::new(db).unwrap());
        let request = PlannerProjectionRequest::identified(
            "source-request",
            "agent-docs",
            DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
            PerspectiveKey::new("agent", "docs").unwrap(),
            BranchScope::main(),
            vec!["docs_freshness".to_string()],
            Vec::new(),
        )
        .unwrap();
        projection_store.put_pending(request.clone(), 2).unwrap();
        let actor = PlannerProjectionActor::new(
            Arc::clone(&projection_store),
            belief_store,
            traversal_store,
        );
        projection_store.fail_flush_after(1);

        let report = actor.tick(PlannerProjectionTickRequest { max_items: 1 });

        assert_eq!(report.completed_count, 1);
        assert!(report.retryable_errors.is_empty());
        assert_eq!(
            projection_store
                .get_request(&request.request_id)
                .unwrap()
                .unwrap()
                .status,
            PlannerProjectionRequestStatus::Completed
        );
    }
}
