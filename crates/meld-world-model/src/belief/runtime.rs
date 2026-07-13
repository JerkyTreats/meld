//! Belief runtime orchestration.
//!
//! Runtime code ties graph reads, evidence normalization, comparator
//! assessment, revision commit, and view projection into one replayable path.
//! It stops at world-model state and never dispatches execution work.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use meld_world_model::belief::{BeliefRuntime, BeliefStore};
//! use meld_world_model::world_state::graph::store::TraversalStore;
//!
//! # let json = "{}";
//! let temp = tempfile::tempdir().unwrap();
//! let db = sled::open(temp.path()).unwrap();
//! let belief = Arc::new(BeliefStore::new(db.clone()).unwrap());
//! let graph = Arc::new(TraversalStore::new(db).unwrap());
//! let runtime = BeliefRuntime::from_json_config(belief, graph, json);
//! # let _ = runtime;
//! ```

use std::sync::Arc;

use crate::belief::comparator::{BayesianComparator, ComparatorInput};
use crate::belief::config::{BeliefConfigLoader, ConfigSnapshot};
use crate::belief::contracts::{AssessmentLease, BranchScope, LeaseStatus};
use crate::belief::evidence::BeliefEvidenceNormalizer;
use crate::belief::ports::BeliefGraphQuery;
use crate::belief::store::BeliefStore;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Maximum evidence records admitted to one comparator settlement window.
pub const MAX_BELIEF_EVIDENCE_WINDOW_ITEMS: usize = 1024;

const ASSESSMENT_LEASE_CLOCK_TTL: u64 = 100;

struct AssessmentLeaseInput {
    belief_key: crate::belief::contracts::BeliefKey,
    epoch: u64,
    owner_id: String,
    input_cursor_start: u64,
    input_cursor_end: u64,
    assignment_cursor_start: Option<crate::belief::contracts::AssessmentAssignmentCursor>,
    assignment_cursor_end: Option<crate::belief::contracts::AssessmentAssignmentCursor>,
    assignment_window_complete: bool,
    lease_clock: u64,
}

/// Summary returned after one subject assessment.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAssessmentResult {
    /// Count of normalized evidence items accepted for the run.
    pub evidence_count: usize,
    /// Id of the committed revision.
    pub revision_id: String,
    /// Planner-facing confidence from the resulting view.
    pub confidence: f64,
    /// Highest source cursor included in the committed revision.
    pub source_cursor_end: u64,
    /// Exact belief-owned progress sequence committed with this assessment.
    pub assessment_progress_sequence: u64,
}

pub(crate) struct BoundedAssessmentOutcome {
    pub assessment: Option<RuntimeAssessmentResult>,
    pub progress_sequence: Option<u64>,
}

/// Orchestrates the first graph-to-belief runtime slice.
pub struct BeliefRuntime {
    belief_store: Arc<BeliefStore>,
    graph_query: Arc<dyn BeliefGraphQuery>,
    config: ConfigSnapshot,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}

impl BeliefRuntime {
    /// Build a runtime from already opened stores and a validated config.
    pub fn new<Q>(
        belief_store: Arc<BeliefStore>,
        graph_query: Arc<Q>,
        config: ConfigSnapshot,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self
    where
        Q: BeliefGraphQuery + 'static,
    {
        Self {
            belief_store,
            graph_query,
            config,
            perspective,
            branch_scope,
        }
    }

    /// Build a runtime from an already erased graph query contract.
    pub fn from_graph_query(
        belief_store: Arc<BeliefStore>,
        graph_query: Arc<dyn BeliefGraphQuery>,
        config: ConfigSnapshot,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        Self {
            belief_store,
            graph_query,
            config,
            perspective,
            branch_scope,
        }
    }

    /// Build a runtime from JSON config using default perspective and branch.
    pub fn from_json_config<Q>(
        belief_store: Arc<BeliefStore>,
        graph_query: Arc<Q>,
        json: &str,
    ) -> Result<Self, StorageError>
    where
        Q: BeliefGraphQuery + 'static,
    {
        let config = BeliefConfigLoader::load_json(json)?;
        Ok(Self::new(
            belief_store,
            graph_query,
            config,
            PerspectiveKey::new("default", "default")?,
            BranchScope::main(),
        ))
    }

    /// Assess one graph subject through the configured belief family.
    ///
    /// The method persists the config snapshot, evidence, assignment, lease,
    /// revision, and view in that order so recovery has durable checkpoints.
    pub fn assess_subject(
        &self,
        subject: &DomainObjectRef,
        anchor_perspective_kind: &str,
        anchor_perspective_id: &str,
        owner_id: &str,
    ) -> Result<RuntimeAssessmentResult, StorageError> {
        self.persist_config()?;
        let anchor = self
            .graph_query
            .current_anchor_for_subject(subject, anchor_perspective_kind, anchor_perspective_id)?
            .ok_or_else(|| StorageError::InvalidPath("missing graph anchor".to_string()))?;
        let provenance = self.graph_query.provenance_for_anchor(&anchor.anchor_id)?;
        let normalizer = BeliefEvidenceNormalizer::new(
            self.config.config.clone(),
            self.perspective.clone(),
            self.branch_scope.clone(),
        );
        let evidence = match normalizer.normalize_anchor(&anchor, &provenance) {
            Ok(evidence) => evidence,
            Err(rejection) => {
                self.belief_store.put_rejection(&rejection)?;
                return Err(StorageError::InvalidPath(rejection.reason));
            }
        };
        let mut assignments_exact_replay = true;
        for item in &evidence {
            self.belief_store.put_evidence_once(item)?;
            if self
                .belief_store
                .put_assignment_once(&normalizer.assign(item)?)?
            {
                assignments_exact_replay = false;
            }
        }
        let key = evidence
            .first()
            .ok_or_else(|| StorageError::InvalidPath("no normalized evidence".to_string()))?
            .candidate_key
            .clone();
        let source_cursor_start = evidence
            .iter()
            .map(|item| item.source_cursor_start)
            .min()
            .unwrap_or(anchor.selected_at_seq);
        let source_cursor_end = evidence
            .iter()
            .map(|item| item.source_cursor_end)
            .max()
            .unwrap_or(anchor.selected_at_seq);
        if assignments_exact_replay {
            if let Some(result) = self.exact_subject_assessment_result(
                &key,
                &evidence,
                source_cursor_start,
                source_cursor_end,
            )? {
                return Ok(result);
            }
            if self.belief_store.dirty_state(&key)?.is_none() {
                return Err(StorageError::InvalidPath(
                    "replayed subject assignments lack an exact committed result".to_string(),
                ));
            }
        }
        let prior = self.belief_store.current_revision(&key)?;
        let mutation_generation = self
            .belief_store
            .dirty_state(&key)?
            .map(|dirty| dirty.mutation_generation.max(1))
            .unwrap_or(1);
        let lease_clock = self.belief_store.advance_assessment_lease_clock()?;
        let lease = self.build_assessment_lease(AssessmentLeaseInput {
            belief_key: key.clone(),
            epoch: mutation_generation,
            owner_id: owner_id.to_string(),
            input_cursor_start: source_cursor_start,
            input_cursor_end: source_cursor_end,
            assignment_cursor_start: None,
            assignment_cursor_end: None,
            assignment_window_complete: true,
            lease_clock,
        })?;
        let lease = self.belief_store.acquire_lease(lease)?;
        let output = BayesianComparator::assess(ComparatorInput {
            config: self.config.config.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            prior_revision: prior,
            evidence: evidence.clone(),
            source_cursor_start,
            source_cursor_end,
        })?;
        let view = self
            .belief_store
            .project_view(&output.revision, output.view_hydration);
        let (_, assessment_progress_sequence) =
            self.belief_store.commit_belief_assessment_with_progress(
                &lease,
                output.revision.clone(),
                view.clone(),
            )?;
        self.belief_store.flush()?;
        Ok(RuntimeAssessmentResult {
            evidence_count: evidence.len(),
            revision_id: output.revision.revision_id,
            confidence: view.planner_projection.confidence,
            source_cursor_end,
            assessment_progress_sequence,
        })
    }

    /// Recover expired assessment leases and reschedule their keys.
    pub fn recover_expired_leases(&self, current_seq: u64) -> Result<usize, StorageError> {
        Ok(self.belief_store.recover_expired_leases(current_seq)?.len())
    }

    /// Assess one dirty belief key from durable assignments.
    pub fn assess_dirty_key(
        &self,
        key: &crate::belief::contracts::BeliefKey,
        owner_id: &str,
    ) -> Result<Option<RuntimeAssessmentResult>, StorageError> {
        self.assess_dirty_key_bounded(key, owner_id, MAX_BELIEF_EVIDENCE_WINDOW_ITEMS)
    }

    /// Assess one bounded evidence window for a dirty belief key.
    pub fn assess_dirty_key_bounded(
        &self,
        key: &crate::belief::contracts::BeliefKey,
        owner_id: &str,
        max_evidence_items: usize,
    ) -> Result<Option<RuntimeAssessmentResult>, StorageError> {
        let lease_clock = self.belief_store.advance_assessment_lease_clock()?;
        self.assess_dirty_key_bounded_outcome(key, owner_id, max_evidence_items, lease_clock)
            .map(|outcome| outcome.assessment)
    }

    pub(crate) fn assess_dirty_key_bounded_outcome(
        &self,
        key: &crate::belief::contracts::BeliefKey,
        owner_id: &str,
        max_evidence_items: usize,
        lease_clock: u64,
    ) -> Result<BoundedAssessmentOutcome, StorageError> {
        if !(1..=MAX_BELIEF_EVIDENCE_WINDOW_ITEMS).contains(&max_evidence_items) {
            return Err(StorageError::InvalidPath(format!(
                "belief evidence window must be in 1..={MAX_BELIEF_EVIDENCE_WINDOW_ITEMS}"
            )));
        }
        self.belief_store
            .recover_expired_lease_for_key(key, lease_clock)?;
        let Some(dirty) = self.belief_store.dirty_state(key)? else {
            return Ok(BoundedAssessmentOutcome {
                assessment: None,
                progress_sequence: None,
            });
        };
        let prior = self.belief_store.current_revision(key)?;
        let window = self.belief_store.evidence_for_dirty_assessment(
            key,
            &dirty,
            dirty.mutation_generation.max(1),
            dirty.latest_seq,
            max_evidence_items,
        )?;
        let source_cursor_end = prior.as_ref().map_or(window.source_cursor_end, |revision| {
            revision.source_cursor_end.max(window.source_cursor_end)
        });
        let source_cursor_start = window
            .evidence
            .iter()
            .map(|item| item.source_cursor_start)
            .min()
            .unwrap_or(source_cursor_end);
        let proposed_lease = self.build_assessment_lease(AssessmentLeaseInput {
            belief_key: key.clone(),
            epoch: dirty.mutation_generation.max(1),
            owner_id: owner_id.to_string(),
            input_cursor_start: source_cursor_start,
            input_cursor_end: source_cursor_end,
            assignment_cursor_start: window.cursor_start.clone(),
            assignment_cursor_end: window.cursor_end.clone(),
            assignment_window_complete: window.complete,
            lease_clock,
        })?;
        let lease = match self.belief_store.acquire_lease(proposed_lease.clone()) {
            Ok(lease) => lease,
            Err(StorageError::Backpressure(message)) => {
                let Some(active) = self.belief_store.active_lease_for_key(key)? else {
                    return Err(StorageError::Backpressure(message));
                };
                if !same_assessment_window(&active, &proposed_lease) {
                    return Err(StorageError::Backpressure(message));
                }
                // A prior acquire can become indeterminate after its atomic
                // products were installed. Reflush the exact active product
                // before resuming comparator work on source replay.
                self.belief_store.flush().map_err(|error| {
                    StorageError::DurabilityIndeterminate(format!(
                        "belief assessment lease replay flush failed: {error}"
                    ))
                })?;
                active
            }
            Err(error) => return Err(error),
        };
        let evidence = window.evidence;
        if evidence.is_empty() {
            let progress_sequence = self.belief_store.complete_empty_assessment_window(&lease)?;
            return Ok(BoundedAssessmentOutcome {
                assessment: None,
                progress_sequence: Some(progress_sequence),
            });
        }
        let output = BayesianComparator::assess(ComparatorInput {
            config: self.config.config.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            prior_revision: prior,
            evidence: evidence.clone(),
            source_cursor_start,
            source_cursor_end,
        })?;
        let view = self
            .belief_store
            .project_view(&output.revision, output.view_hydration);
        let (_, assessment_progress_sequence) =
            self.belief_store.commit_belief_assessment_with_progress(
                &lease,
                output.revision.clone(),
                view.clone(),
            )?;
        self.belief_store.flush()?;
        Ok(BoundedAssessmentOutcome {
            assessment: Some(RuntimeAssessmentResult {
                evidence_count: evidence.len(),
                revision_id: output.revision.revision_id,
                confidence: view.planner_projection.confidence,
                source_cursor_end,
                assessment_progress_sequence,
            }),
            progress_sequence: Some(assessment_progress_sequence),
        })
    }

    /// Persist and activate the immutable configuration used by this runtime.
    pub fn persist_config(&self) -> Result<(), StorageError> {
        let config_json = serde_json::to_string(&self.config.config).map_err(to_storage_data)?;
        self.belief_store
            .put_config_snapshot_once(&self.config.hash, &config_json)?;
        self.belief_store.put_runtime_meta_internal(
            &format!("active_config_hash::{}", self.config.config.family_id),
            &self.config.hash,
        )?;
        self.belief_store.put_runtime_meta_internal(
            &format!("active_policy_id::{}", self.config.config.family_id),
            &self.config.config.evidence_policy_id,
        )?;
        self.belief_store.flush()
    }

    fn build_assessment_lease(
        &self,
        input: AssessmentLeaseInput,
    ) -> Result<AssessmentLease, StorageError> {
        Ok(AssessmentLease {
            lease_id: format!(
                "lease-{}-{}-{}",
                input.lease_clock,
                input.input_cursor_end,
                input.belief_key.index_key()
            ),
            belief_key: input.belief_key,
            epoch: input.epoch,
            owner_id: input.owner_id,
            input_cursor_start: input.input_cursor_start,
            input_cursor_end: input.input_cursor_end,
            assignment_cursor_start: input.assignment_cursor_start,
            assignment_cursor_end: input.assignment_cursor_end,
            assignment_window_complete: input.assignment_window_complete,
            started_at_seq: input.lease_clock,
            expires_at_seq: input
                .lease_clock
                .checked_add(ASSESSMENT_LEASE_CLOCK_TTL)
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "belief assessment lease expiry is exhausted".to_string(),
                    )
                })?,
            comparator_engine_id: self.config.config.comparator.engine_id.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            status: LeaseStatus::Queued,
        })
    }

    fn exact_subject_assessment_result(
        &self,
        key: &crate::belief::contracts::BeliefKey,
        evidence: &[crate::belief::contracts::EvidenceItem],
        source_cursor_start: u64,
        source_cursor_end: u64,
    ) -> Result<Option<RuntimeAssessmentResult>, StorageError> {
        let Some(first) = evidence.first() else {
            return Ok(None);
        };
        let Some(revision) = self
            .belief_store
            .committed_revision_for_evidence(key, &first.evidence_id)?
        else {
            return Ok(None);
        };
        for item in evidence.iter().skip(1) {
            let Some(committed) = self
                .belief_store
                .committed_revision_for_evidence(key, &item.evidence_id)?
            else {
                return Ok(None);
            };
            if committed.revision_id != revision.revision_id {
                return Ok(None);
            }
        }
        let evidence_ids = evidence
            .iter()
            .map(|item| item.evidence_id.clone())
            .collect::<Vec<_>>();
        if revision.evidence_ids != evidence_ids
            || revision.source_cursor_start != source_cursor_start
            || revision.source_cursor_end != source_cursor_end
            || revision.config_snapshot_hash != self.config.hash
            || revision.comparator_engine_id != self.config.config.comparator.engine_id
        {
            return Ok(None);
        }
        let Some(assessment_progress_sequence) = self
            .belief_store
            .exact_commit_progress_for_revision(&revision)?
        else {
            return Ok(None);
        };
        Ok(Some(RuntimeAssessmentResult {
            evidence_count: evidence.len(),
            revision_id: revision.revision_id,
            confidence: revision.planner_projection.confidence,
            source_cursor_end,
            assessment_progress_sequence,
        }))
    }

    /// Recover expired work and assess every available dirty key once.
    pub fn recover_and_assess_dirty(
        &self,
        current_seq: u64,
        owner_id: &str,
    ) -> Result<usize, StorageError> {
        self.belief_store.recover_expired_leases(current_seq)?;
        let mut committed = 0;
        for dirty in self.belief_store.dirty_key_states()? {
            if self
                .assess_dirty_key(&dirty.belief_key, owner_id)?
                .is_some()
            {
                committed += 1;
            }
        }
        Ok(committed)
    }
}

fn same_assessment_window(active: &AssessmentLease, proposed: &AssessmentLease) -> bool {
    active.status == LeaseStatus::Leased
        && active.belief_key == proposed.belief_key
        && active.epoch == proposed.epoch
        && active.owner_id == proposed.owner_id
        && active.input_cursor_start == proposed.input_cursor_start
        && active.input_cursor_end == proposed.input_cursor_end
        && active.assignment_cursor_start == proposed.assignment_cursor_start
        && active.assignment_cursor_end == proposed.assignment_cursor_end
        && active.assignment_window_complete == proposed.assignment_window_complete
        && active.comparator_engine_id == proposed.comparator_engine_id
        && active.config_snapshot_hash == proposed.config_snapshot_hash
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        err.to_string(),
    ))
}
