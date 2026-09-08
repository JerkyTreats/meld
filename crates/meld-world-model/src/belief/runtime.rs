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
//! let runtime = BeliefRuntime::from_json_config(belief, json);
//! # let _ = runtime;
//! ```

use std::sync::Arc;

use crate::belief::comparator::{BayesianComparator, ComparatorInput};
use crate::belief::config::{BeliefConfigLoader, ConfigSnapshot};
use crate::belief::contracts::{
    AssessmentLease, BeliefKey, BranchScope, InitialAssessmentPolicy, LeaseStatus,
};
use crate::belief::registry::{BeliefFamilyRevision, TheoryRevisionRef};
use crate::belief::store::BeliefStore;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Summary returned after one subject assessment.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAssessmentResult {
    /// Count of normalized evidence items accepted for the run.
    pub evidence_count: usize,
    /// Id of the committed revision.
    pub revision_id: String,
    /// Planner-facing confidence from the resulting view.
    pub confidence: f64,
}

/// Assesses admitted evidence under the selected Belief family and policy.
pub struct BeliefRuntime {
    belief_store: Arc<BeliefStore>,
    config: ConfigSnapshot,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
    // Lineage reference stamped onto every committed revision. `None` only
    // for legacy construction paths that predate theory registries.
    theory_revision: Option<TheoryRevisionRef>,
}

impl BeliefRuntime {
    /// Build a runtime from already opened stores and a validated config.
    pub fn new(
        belief_store: Arc<BeliefStore>,
        config: ConfigSnapshot,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        Self {
            belief_store,
            config,
            perspective,
            branch_scope,
            theory_revision: None,
        }
    }

    /// Build a runtime from one installed belief-family registry revision.
    ///
    /// The revision content hash is the config snapshot hash, so replay and
    /// freshness checks bind to exactly the installed theory content, and the
    /// revision lineage reference is stamped onto every committed revision.
    pub fn from_family_revision(
        belief_store: Arc<BeliefStore>,
        revision: &BeliefFamilyRevision,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        let theory_revision = revision.revision_ref();
        Self::new(
            belief_store,
            ConfigSnapshot {
                config: revision.config.clone(),
                hash: revision.content_hash.clone(),
            },
            perspective,
            branch_scope,
        )
        .with_theory_revision(theory_revision)
    }

    // Private on purpose: durable lineage may cite only installed registry
    // revisions, so the sole entry point is `from_family_revision`, which
    // derives the reference from a `BeliefFamilyRevision`. A public stamp
    // would let callers invent never-installed theory references.
    fn with_theory_revision(mut self, theory_revision: TheoryRevisionRef) -> Self {
        self.theory_revision = Some(theory_revision);
        self
    }

    /// Build a runtime from JSON config using default perspective and branch.
    pub fn from_json_config(
        belief_store: Arc<BeliefStore>,
        json: &str,
    ) -> Result<Self, StorageError> {
        let config = BeliefConfigLoader::load_json(json)?;
        Ok(Self::new(
            belief_store,
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
        owner_id: &str,
    ) -> Result<RuntimeAssessmentResult, StorageError> {
        let config_json = serde_json::to_string(&self.config.config).map_err(to_storage_data)?;
        self.belief_store
            .put_config_snapshot(&self.config.hash, &config_json)?;
        self.belief_store.put_runtime_meta(
            &format!("active_config_hash::{}", self.config.config.family_id),
            &self.config.hash,
        )?;
        self.belief_store.put_runtime_meta(
            &format!("active_policy_id::{}", self.config.config.family_id),
            &self.config.config.evidence_policy_id,
        )?;
        let key = BeliefKey {
            subject: subject.clone(),
            dimension_id: self.config.config.dimension_id.clone(),
            predicate_id: self.config.config.predicate_id.clone(),
            perspective: self.perspective.clone(),
            branch_scope: self.branch_scope.clone(),
            evidence_policy_id: self.config.config.evidence_policy_id.clone(),
        };
        if self.belief_store.dirty_state(&key)?.is_some() {
            return self.assess_dirty_key(&key, owner_id)?.ok_or_else(|| {
                StorageError::Backpressure("initial evidence assessment remains pending".into())
            });
        }
        if let Some(view) = self.belief_store.current_view(&key)? {
            if let Some(revision_id) = view.current_revision_id {
                return Ok(RuntimeAssessmentResult {
                    evidence_count: 0,
                    revision_id,
                    confidence: view.planner_projection.confidence,
                });
            }
        }
        if self.config.config.initial_assessment == InitialAssessmentPolicy::PriorAllowed {
            self.assess_prior_subject(subject, owner_id)
        } else {
            Err(StorageError::Unavailable(
                "initial assessment requires curated evidence".into(),
            ))
        }
    }

    /// Assess one subject of an unanchored family to its prior-based
    /// revision.
    ///
    /// No graph anchor is consulted and no evidence is cited. The source
    /// cursor window is pinned to zero so the first promoted evidence at
    /// any ledger sequence re-dirties the key instead of being absorbed by
    /// a window the revision never actually covered.
    fn assess_prior_subject(
        &self,
        subject: &DomainObjectRef,
        owner_id: &str,
    ) -> Result<RuntimeAssessmentResult, StorageError> {
        let config = &self.config.config;
        let key = BeliefKey {
            subject: subject.clone(),
            dimension_id: config.dimension_id.clone(),
            predicate_id: config.predicate_id.clone(),
            perspective: self.perspective.clone(),
            branch_scope: self.branch_scope.clone(),
            evidence_policy_id: config.evidence_policy_id.clone(),
        };
        let prior = self.belief_store.current_revision(&key)?;
        let lease = AssessmentLease {
            lease_id: format!("lease-unanchored-{}", key.index_key()),
            belief_key: key.clone(),
            epoch: 0,
            owner_id: owner_id.to_string(),
            input_cursor_start: 0,
            input_cursor_end: 0,
            started_at_seq: 0,
            expires_at_seq: 100,
            comparator_engine_id: config.comparator.engine_id.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            status: LeaseStatus::Queued,
        };
        let lease = self.belief_store.acquire_lease(lease)?;
        let mut output = BayesianComparator::assess(ComparatorInput {
            config: config.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            prior_revision: prior,
            evidence: Vec::new(),
            subject_key: Some(key),
            source_cursor_start: 0,
            source_cursor_end: 0,
        })?;
        output.revision.theory_revision = self.theory_revision.clone();
        self.belief_store
            .commit_revision(&lease, &output.revision)?;
        let view = self
            .belief_store
            .project_view(&output.revision, output.view_hydration);
        self.belief_store.put_view(&view)?;
        self.belief_store.complete_lease(&lease)?;
        self.belief_store.flush()?;
        Ok(RuntimeAssessmentResult {
            evidence_count: 0,
            revision_id: output.revision.revision_id,
            confidence: view.planner_projection.confidence,
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
        let Some(dirty) = self.belief_store.dirty_state(key)? else {
            return Ok(None);
        };
        let prior = self.belief_store.current_revision(key)?;
        // A committed revision may already cover the dirty window, for
        // example after a replayed dirty mark. Clearing here keeps absorbed
        // keys out of bounded selection instead of re-committing the same
        // window forever.
        if let Some(prior_revision) = &prior {
            if prior_revision.source_cursor_end >= dirty.latest_seq {
                self.belief_store.clear_dirty(key)?;
                self.belief_store.flush()?;
                return Ok(None);
            }
        }
        let source_cursor_start = prior
            .as_ref()
            .map(|revision| revision.source_cursor_end.saturating_add(1))
            .unwrap_or(dirty.dirty_since_seq)
            .min(dirty.dirty_since_seq);
        let source_cursor_end = dirty.latest_seq;
        let lease = AssessmentLease {
            lease_id: format!("lease-{}-{}", source_cursor_end, key.index_key()),
            belief_key: key.clone(),
            epoch: source_cursor_end,
            owner_id: owner_id.to_string(),
            input_cursor_start: source_cursor_start,
            input_cursor_end: source_cursor_end,
            started_at_seq: source_cursor_start,
            expires_at_seq: source_cursor_end + 100,
            comparator_engine_id: self.config.config.comparator.engine_id.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            status: LeaseStatus::Queued,
        };
        let lease = match self.belief_store.acquire_lease(lease) {
            Ok(lease) => lease,
            Err(StorageError::Backpressure(_)) => return Ok(None),
            Err(err) => return Err(err),
        };
        let evidence: Vec<_> = self
            .belief_store
            .evidence_for_key(key)?
            .into_iter()
            .filter(|item| {
                item.source_cursor_end >= source_cursor_start
                    && item.source_cursor_end <= source_cursor_end
            })
            .collect();
        if evidence.is_empty() {
            self.belief_store.complete_lease(&lease)?;
            return Ok(None);
        }
        let mut output = BayesianComparator::assess(ComparatorInput {
            config: self.config.config.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            prior_revision: prior,
            evidence: evidence.clone(),
            subject_key: None,
            source_cursor_start,
            source_cursor_end,
        })?;
        // Stamp lineage before commit so the durable revision, not just the
        // in-memory view, answers which theory produced this belief.
        output.revision.theory_revision = self.theory_revision.clone();
        self.belief_store
            .commit_revision(&lease, &output.revision)?;
        let view = self
            .belief_store
            .project_view(&output.revision, output.view_hydration);
        self.belief_store.put_view(&view)?;
        self.belief_store.complete_lease(&lease)?;
        self.belief_store.flush()?;
        Ok(Some(RuntimeAssessmentResult {
            evidence_count: evidence.len(),
            revision_id: output.revision.revision_id,
            confidence: view.planner_projection.confidence,
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

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        err.to_string(),
    ))
}
