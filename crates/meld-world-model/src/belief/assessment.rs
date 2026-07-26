//! Bounded belief assessment actor.
//!
//! Owner: world model belief domain. One step assesses at most a budgeted
//! number of selected belief keys under an injected sequence — no wall clock
//! and no caller-manufactured work items. Progress is durable before the step
//! returns: every committed revision, view, lease transition, and the step
//! checkpoint are flushed, so reopening mid-sequence resumes from durable
//! selection state without double-assessing. The actor resolves the current
//! theory revision per step through the family registry, and that revision
//! enters the lineage of every revision it commits.
//!
//! The request and report pair is domain-owned. Root runtime contracts adapt
//! this report behind their own bounded-step surface; this crate does not
//! depend on them.

use std::sync::Arc;

use crate::belief::contracts::BranchScope;
use crate::belief::registry::{BeliefFamilyRegistry, BeliefFamilyRevision};
use crate::belief::runtime::BeliefRuntime;
use crate::belief::selection::{BeliefSubjectBinding, BeliefWorkKind, BeliefWorkSelector};
use crate::belief::store::BeliefStore;
use crate::error::StorageError;
use crate::waiting::WaitingOnDeclaration;
use crate::world_state::graph::query::TraversalQuery;
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::PerspectiveKey;

/// Bounded belief assessment step request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefAssessmentRequest {
    /// Injected step sequence used for lease expiry and the step checkpoint.
    pub sequence: u64,
    /// Maximum selected work items to attempt during one step.
    pub max_items: usize,
}

/// Diagnostic issue produced by one assessment step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefAssessmentIssue {
    /// Belief key index or family id the issue concerns, when known.
    pub item_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Domain report from one bounded belief assessment step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefAssessmentReport {
    /// Stable actor identifier.
    pub actor_id: String,
    /// Injected sequence supplied by the request.
    pub input_sequence: u64,
    /// Durable step checkpoint read before work.
    pub input_checkpoint: u64,
    /// Durable step checkpoint persisted after work.
    pub output_checkpoint: u64,
    /// Selected work items attempted this step.
    pub items_attempted: usize,
    /// Revisions durably committed this step.
    pub items_committed: usize,
    /// Retryable diagnostics observed during the step.
    pub retryable_errors: Vec<BeliefAssessmentIssue>,
    /// Fatal diagnostics observed during the step.
    pub fatal_errors: Vec<BeliefAssessmentIssue>,
    /// True when eligible work remained after the budget was consumed.
    pub budget_exhausted: bool,
    /// What would make quiet or blocked work eligible (DBG-016).
    ///
    /// Derived from the selection and per-item outcomes this step already
    /// computed; emission never gates or reorders assessment work.
    pub waiting_on: Vec<WaitingOnDeclaration>,
}

/// Bounded actor that discovers and assesses belief work from durable state.
pub struct BeliefAssessmentActor {
    actor_id: String,
    store: Arc<BeliefStore>,
    traversal: Arc<TraversalStore>,
    registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
    family_ids: Vec<String>,
    subjects: Vec<BeliefSubjectBinding>,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}

impl BeliefAssessmentActor {
    /// Bind the actor to durable stores, its registry, and configured scope.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_id: impl Into<String>,
        store: Arc<BeliefStore>,
        traversal: Arc<TraversalStore>,
        registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
        family_ids: Vec<String>,
        subjects: Vec<BeliefSubjectBinding>,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        Self {
            actor_id: actor_id.into(),
            store,
            traversal,
            registry,
            family_ids,
            subjects,
            perspective,
            branch_scope,
        }
    }

    /// Stable actor identity carried in reports and lease ownership.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Run one bounded assessment step at the injected sequence.
    ///
    /// Sequencing: recover expired leases, resolve current theory revisions,
    /// select bounded work, assess item by item (each commit is durable
    /// inside the belief runtime), then persist the step checkpoint. A step
    /// over unchanged durable state selects nothing and reports zero work.
    pub fn bounded_step(&mut self, request: &BeliefAssessmentRequest) -> BeliefAssessmentReport {
        let input_checkpoint = self.read_checkpoint();
        let mut report = BeliefAssessmentReport {
            actor_id: self.actor_id.clone(),
            input_sequence: request.sequence,
            input_checkpoint,
            output_checkpoint: input_checkpoint,
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: Vec::new(),
        };
        if request.max_items == 0 {
            report.fatal_errors.push(issue(
                None,
                "invalid_budget",
                "assessment budget must be greater than zero",
            ));
            return report;
        }

        if let Err(error) = self.store.recover_expired_leases(request.sequence) {
            report
                .retryable_errors
                .push(issue(None, "lease_recovery_failed", &error.to_string()));
        }

        let families = match self.resolve_families(&mut report) {
            Ok(families) => families,
            Err(()) => return report,
        };

        let selection = match BeliefWorkSelector::new(self.store.as_ref()).select(
            &families,
            &self.subjects,
            &self.perspective,
            &self.branch_scope,
            request.max_items,
        ) {
            Ok(selection) => selection,
            Err(error) => {
                report
                    .fatal_errors
                    .push(issue(None, "selection_failed", &error.to_string()));
                return report;
            }
        };
        report.budget_exhausted = selection.more_available;

        // The hardened DBG-016 rule: whenever the selector finds nothing
        // eligible, the report says what would change that, so the
        // eligibility walk always has a chain to resolve.
        if selection.items.is_empty() && !families.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                "belief_work_ineligible",
                format!(
                    "no dirty keys and no unassessed subject bindings across {} installed \
                     families and {} configured subjects",
                    families.len(),
                    self.subjects.len()
                ),
            ));
        }

        for item in selection.items {
            let Some(family) = families
                .iter()
                .find(|family| family.family_id == item.family_id)
            else {
                continue;
            };
            report.items_attempted += 1;
            // The runtime is rebuilt per item from the revision resolved this
            // step, so lineage always cites the theory actually executed.
            let runtime = BeliefRuntime::from_family_revision(
                Arc::clone(&self.store),
                Arc::clone(&self.traversal),
                family,
                self.perspective.clone(),
                self.branch_scope.clone(),
            );
            let item_id = item.key.index_key();
            let outcome = match &item.kind {
                BeliefWorkKind::InitialAssessment { binding } => runtime
                    .assess_subject(
                        &binding.subject,
                        &binding.anchor_perspective_kind,
                        &binding.anchor_perspective_id,
                        &self.actor_id,
                    )
                    .map(Some),
                BeliefWorkKind::DirtyKey { .. } => {
                    runtime.assess_dirty_key(&item.key, &self.actor_id)
                }
            };
            match outcome {
                Ok(Some(_)) => report.items_committed += 1,
                // No commit without an error: the key was contended or its
                // evidence window was empty; durable state still owns it.
                Ok(None) => {}
                Err(StorageError::Backpressure(message)) => {
                    report.waiting_on.push(WaitingOnDeclaration::about(
                        "assessment_lease_held",
                        item_id.clone(),
                        format!("an active lease owns this key: {message}"),
                    ));
                    report
                        .retryable_errors
                        .push(issue(Some(item_id), "lease_contended", &message));
                }
                Err(error) => {
                    self.declare_blocked_item(&mut report, &item.kind, &item_id);
                    report.retryable_errors.push(issue(
                        Some(item_id),
                        "assessment_failed",
                        &error.to_string(),
                    ));
                }
            }
        }

        match self.persist_checkpoint(request.sequence) {
            Ok(()) => report.output_checkpoint = request.sequence,
            Err(error) => {
                report
                    .retryable_errors
                    .push(issue(None, "checkpoint_failed", &error.to_string()));
            }
        }
        report
    }

    /// Declare what a blocked assessment item is waiting on.
    ///
    /// For an initial assessment the load-bearing precondition is the
    /// current graph anchor; the read here is observational (a point
    /// lookup on state the failed attempt just consulted) and names the
    /// exact subject key and perspective, so the anchor stall reads as a
    /// nameable divergence rather than an opaque error.
    fn declare_blocked_item(
        &self,
        report: &mut BeliefAssessmentReport,
        kind: &BeliefWorkKind,
        item_id: &str,
    ) {
        match kind {
            BeliefWorkKind::InitialAssessment { binding } => {
                let anchor = TraversalQuery::new(self.traversal.as_ref())
                    .current_anchor_for_subject(
                        &binding.subject,
                        &binding.anchor_perspective_kind,
                        &binding.anchor_perspective_id,
                    )
                    .ok()
                    .flatten();
                if anchor.is_none() {
                    report.waiting_on.push(WaitingOnDeclaration::about(
                        "graph_anchor_absent",
                        binding.subject.index_key(),
                        format!(
                            "no current anchor for subject {} under {}::{}",
                            binding.subject.index_key(),
                            binding.anchor_perspective_kind,
                            binding.anchor_perspective_id
                        ),
                    ));
                    return;
                }
                report.waiting_on.push(WaitingOnDeclaration::about(
                    "assessment_blocked",
                    item_id,
                    "initial assessment failed past the anchor precondition",
                ));
            }
            BeliefWorkKind::DirtyKey { .. } => {
                report.waiting_on.push(WaitingOnDeclaration::about(
                    "assessment_blocked",
                    item_id,
                    "dirty-key assessment failed; the key stays dirty",
                ));
            }
        }
    }

    /// Resolve the current registry revision for every configured family.
    ///
    /// A missing family is retryable — installation may land later — but a
    /// storage failure is fatal for the step.
    fn resolve_families(
        &self,
        report: &mut BeliefAssessmentReport,
    ) -> Result<Vec<BeliefFamilyRevision>, ()> {
        let mut family_ids = self.family_ids.clone();
        family_ids.sort();
        family_ids.dedup();
        let mut families = Vec::new();
        for family_id in family_ids {
            match self.registry.current(&family_id) {
                Ok(Some(revision)) => families.push(revision),
                Ok(None) => {
                    report.waiting_on.push(WaitingOnDeclaration::broad(
                        "belief_family_absent",
                        format!("belief family '{family_id}' has no installed registry revision"),
                    ));
                    report.retryable_errors.push(issue(
                        Some(family_id.clone()),
                        "family_not_installed",
                        "no current registry revision for configured family",
                    ))
                }
                Err(error) => {
                    report.fatal_errors.push(issue(
                        Some(family_id.clone()),
                        "registry_read_failed",
                        &error.to_string(),
                    ));
                    return Err(());
                }
            }
        }
        Ok(families)
    }

    fn checkpoint_meta_key(&self) -> String {
        format!("assessment_checkpoint::{}", self.actor_id)
    }

    fn read_checkpoint(&self) -> u64 {
        self.store
            .get_runtime_meta(&self.checkpoint_meta_key())
            .ok()
            .flatten()
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(0)
    }

    fn persist_checkpoint(&self, sequence: u64) -> Result<(), StorageError> {
        self.store
            .put_runtime_meta(&self.checkpoint_meta_key(), &sequence.to_string())?;
        self.store.flush()
    }
}

fn issue(item_id: Option<String>, code: &str, message: &str) -> BeliefAssessmentIssue {
    BeliefAssessmentIssue {
        item_id,
        code: code.to_string(),
        message: message.to_string(),
    }
}
