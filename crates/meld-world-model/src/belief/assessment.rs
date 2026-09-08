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
use crate::waiting::{conditions, StructuralWakeAddress, WaitingOnDeclaration};
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
    registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
    family_ids: Vec<String>,
    subjects: Vec<BeliefSubjectBinding>,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
    pinned_families: Option<Vec<BeliefFamilyRevision>>,
    lifecycle: crate::lifecycle::NativeLifecycle,
    work_lock: parking_lot::Mutex<()>,
}

impl BeliefAssessmentActor {
    /// Bind the actor to durable stores, its registry, and configured scope.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_id: impl Into<String>,
        store: Arc<BeliefStore>,
        registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
        family_ids: Vec<String>,
        subjects: Vec<BeliefSubjectBinding>,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        let actor_id = actor_id.into();
        Self {
            lifecycle: crate::lifecycle::NativeLifecycle::new(actor_id.clone()),
            work_lock: parking_lot::Mutex::new(()),
            actor_id,
            store,
            registry,
            family_ids,
            subjects,
            perspective,
            branch_scope,
            pinned_families: None,
        }
    }

    /// Freeze exact family revisions for the lifetime of this actor.
    pub fn with_pinned_families(mut self, revisions: Vec<BeliefFamilyRevision>) -> Self {
        self.pinned_families = Some(revisions);
        self
    }

    /// Stable actor identity carried in reports and lease ownership.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Return the durable assessment checkpoint used by lifecycle recovery.
    pub fn lifecycle_checkpoint(&self) -> u64 {
        self.read_checkpoint()
    }

    /// Resolve waits only for this store and the exact configured assessment scope.
    pub fn resolves_wake(&self, wake: &StructuralWakeAddress) -> Result<bool, String> {
        let value = match wake {
            StructuralWakeAddress::OwnerRevision(value)
            | StructuralWakeAddress::DurableDeadline(value) => value,
            _ => return Ok(false),
        };
        let Some(value) = crate::waiting::bound_address(value, self.store.resource_id()) else {
            return Ok(false);
        };
        if matches!(wake, StructuralWakeAddress::OwnerRevision(_))
            && crate::waiting::after_position(
                value,
                &format!("belief-dirty-work::{}", self.actor_id),
            )
        {
            return Ok(!self.family_ids.is_empty());
        }
        let families = match &self.pinned_families {
            Some(families) => families.clone(),
            None => self
                .family_ids
                .iter()
                .map(|id| self.registry.current(id).map_err(|error| error.to_string()))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect(),
        };
        for family in &families {
            let key = match wake {
                StructuralWakeAddress::OwnerRevision(_) => ["belief-dirty-key::", "belief-input::"]
                    .iter()
                    .find_map(|prefix| {
                        value
                            .strip_prefix(prefix)
                            .and_then(|value| value.strip_suffix("::successor"))
                    })
                    .map(str::to_string),
                StructuralWakeAddress::DurableDeadline(_) => value
                    .strip_prefix("belief-assessment-lease::")
                    .and_then(|value| value.rsplit_once("::after::"))
                    .filter(|(_, seq)| seq.parse::<u64>().is_ok())
                    .map(|(key, _)| key.to_string()),
                _ => None,
            };
            if let Some(key) = key {
                if let Some(request) = self
                    .store
                    .subscription_for_index_key(
                        &family.revision_ref(),
                        &self.perspective,
                        &self.branch_scope,
                        &key,
                    )
                    .map_err(|error| error.to_string())?
                {
                    return Ok(!matches!(wake, StructuralWakeAddress::DurableDeadline(_))
                        || self
                            .store
                            .active_lease_for_key(&request.belief_key)
                            .map_err(|error| error.to_string())?
                            .is_some());
                }
            }
            for subject in &self.subjects {
                let key = crate::belief::configured_belief_key(
                    family,
                    &subject.subject,
                    &self.perspective,
                    &self.branch_scope,
                );
                if matches!(wake, StructuralWakeAddress::OwnerRevision(_))
                    && ["belief-dirty-key", "belief-input"]
                        .iter()
                        .any(|kind| value == format!("{kind}::{}::successor", key.index_key()))
                {
                    return Ok(true);
                }
                if matches!(wake, StructuralWakeAddress::DurableDeadline(_))
                    && crate::waiting::after_position(
                        value,
                        &format!("belief-assessment-lease::{}", key.index_key()),
                    )
                {
                    return Ok(self
                        .store
                        .active_lease_for_key(&key)
                        .map_err(|error| error.to_string())?
                        .is_some());
                }
            }
        }
        Ok(false)
    }

    /// Read lifecycle evidence from this owner's bound stores and installed inputs.
    pub fn lifecycle_evidence(&self) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let _guard = self.work_lock.lock();
        self.lifecycle_evidence_inner()
    }

    fn lifecycle_evidence_inner(
        &self,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let families = if let Some(families) = &self.pinned_families {
            families.clone()
        } else {
            self.family_ids
                .iter()
                .map(|id| {
                    self.registry
                        .current(id)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| format!("Belief family {id} is not installed"))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        if families.is_empty() {
            return Err("Belief has no installed family".into());
        }
        let position = self
            .store
            .get_runtime_meta(&self.checkpoint_meta_key())
            .map_err(|error| error.to_string())?
            .map(|raw| raw.parse::<u64>())
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or(0);
        let work = BeliefWorkSelector::new(self.store.as_ref())
            .select(
                &families,
                &self.subjects,
                &self.perspective,
                &self.branch_scope,
                usize::MAX,
            )
            .map_err(|error| error.to_string())?;
        let pending: Vec<_> = work
            .items
            .iter()
            .map(|item| (&item.key, format!("{:?}", item.kind)))
            .collect();
        self.store.flush().map_err(|error| error.to_string())?;
        let checkpoint_ref = format!("belief-assessment::{}::{position}", self.actor_id);
        let mut subscription_refs = vec![format!("belief-dirty-work::{}", self.actor_id)];
        for family in &families {
            subscription_refs.extend(
                self.store
                    .bound_subscriptions(
                        &family.revision_ref(),
                        &self.perspective,
                        &self.branch_scope,
                    )
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|request| format!("belief-subscription::{}", request.request_id)),
            );
        }
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: families
                .iter()
                .map(|family| {
                    crate::lifecycle::evidence_ref("belief-family", &family.revision_ref())
                })
                .collect::<Result<_, _>>()?,
            binding_refs: self
                .subjects
                .iter()
                .map(|binding| format!("belief-subject::{}", binding.subject.index_key()))
                .collect(),
            subscription_refs,
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: crate::lifecycle::evidence_ref(
                "belief-selected-work",
                &pending,
            )?,
        })
    }

    /// Author native start evidence with bounded work excluded.
    pub fn lifecycle_start(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .start(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native safe point evidence with bounded work excluded.
    pub fn lifecycle_safe_point(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .safe_point(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native stop evidence with bounded work excluded.
    pub fn lifecycle_stop(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .stop(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native release evidence with bounded work excluded.
    pub fn lifecycle_release(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .release(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Run one bounded assessment step at the injected sequence.
    ///
    /// Sequencing: recover expired leases, resolve current theory revisions,
    /// select bounded work, assess item by item (each commit is durable
    /// inside the belief runtime), then persist the step checkpoint. A step
    /// over unchanged durable state selects nothing and reports zero work.
    pub fn bounded_step(&mut self, request: &BeliefAssessmentRequest) -> BeliefAssessmentReport {
        let _guard = self.work_lock.lock();
        let mut report = self.bounded_step_inner(request);
        crate::waiting::bind_waits(&mut report.waiting_on, self.store.resource_id());
        report
    }

    fn bounded_step_inner(&self, request: &BeliefAssessmentRequest) -> BeliefAssessmentReport {
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
        if selection.items.is_empty() && families.is_empty() && report.waiting_on.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                conditions::BELIEF_FAMILIES_UNCONFIGURED,
                "no belief family id is configured for this actor",
                vec![StructuralWakeAddress::OperatorAction(format!(
                    "belief-family-configuration::{}",
                    self.actor_id
                ))],
            ));
        }
        if selection.items.is_empty() && !families.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                conditions::BELIEF_WORK_INELIGIBLE,
                format!(
                    "no admitted evidence or prior-authorized initial work across {} installed \
                     families and {} configured subjects",
                    families.len(),
                    self.subjects.len()
                ),
                vec![StructuralWakeAddress::OwnerRevision(format!(
                    "belief-dirty-work::{}::after::{}",
                    self.actor_id, report.output_checkpoint
                ))],
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
                family,
                self.perspective.clone(),
                self.branch_scope.clone(),
            );
            let item_id = item.key.index_key();
            let outcome = match &item.kind {
                BeliefWorkKind::InitialAssessment { binding } => runtime
                    .assess_subject(&binding.subject, &self.actor_id)
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
                        conditions::ASSESSMENT_LEASE_HELD,
                        item_id.clone(),
                        format!("an active lease owns this key: {message}"),
                        vec![StructuralWakeAddress::DurableDeadline(format!(
                            "belief-assessment-lease::{item_id}::after::{}",
                            request.sequence
                        ))],
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

    /// Name the evidence or lease successor that can unblock assessment.
    fn declare_blocked_item(
        &self,
        report: &mut BeliefAssessmentReport,
        kind: &BeliefWorkKind,
        item_id: &str,
    ) {
        let reason = match kind {
            BeliefWorkKind::InitialAssessment { .. } => {
                "initial assessment awaits admitted evidence or a lease successor"
            }
            BeliefWorkKind::DirtyKey { .. } => "dirty-key assessment failed; the key stays dirty",
        };
        report.waiting_on.push(WaitingOnDeclaration::about(
            conditions::ASSESSMENT_BLOCKED,
            item_id,
            reason,
            vec![StructuralWakeAddress::OwnerRevision(format!(
                "belief-input::{item_id}::successor"
            ))],
        ));
    }

    /// Resolve the current registry revision for every configured family.
    ///
    /// A missing family is retryable — installation may land later — but a
    /// storage failure is fatal for the step.
    fn resolve_families(
        &self,
        report: &mut BeliefAssessmentReport,
    ) -> Result<Vec<BeliefFamilyRevision>, ()> {
        if let Some(families) = &self.pinned_families {
            return Ok(families.clone());
        }
        let mut family_ids = self.family_ids.clone();
        family_ids.sort();
        family_ids.dedup();
        let mut families = Vec::new();
        for family_id in family_ids {
            match self.registry.current(&family_id) {
                Ok(Some(revision)) => families.push(revision),
                Ok(None) => {
                    report.waiting_on.push(WaitingOnDeclaration::broad(
                        conditions::BELIEF_FAMILY_ABSENT,
                        format!("belief family '{family_id}' has no installed registry revision"),
                        vec![StructuralWakeAddress::OperatorAction(format!(
                            "belief-family-install::{family_id}"
                        ))],
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
