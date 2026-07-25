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
use crate::belief::registry::{BeliefFamilyRevision, TheoryRevisionRef};
use crate::belief::store::BeliefStore;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::{PerspectiveKey, TraversalQuery};

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

/// Orchestrates the first graph-to-belief runtime slice.
pub struct BeliefRuntime {
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
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
        traversal_store: Arc<TraversalStore>,
        config: ConfigSnapshot,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        Self {
            belief_store,
            traversal_store,
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
        traversal_store: Arc<TraversalStore>,
        revision: &BeliefFamilyRevision,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        let theory_revision = revision.revision_ref();
        Self::new(
            belief_store,
            traversal_store,
            ConfigSnapshot {
                config: revision.config.clone(),
                hash: revision.content_hash.clone(),
            },
            perspective,
            branch_scope,
        )
        .with_theory_revision(theory_revision)
    }

    /// Stamp an installed theory revision into committed revision lineage.
    pub fn with_theory_revision(mut self, theory_revision: TheoryRevisionRef) -> Self {
        self.theory_revision = Some(theory_revision);
        self
    }

    /// Build a runtime from JSON config using default perspective and branch.
    pub fn from_json_config(
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
        json: &str,
    ) -> Result<Self, StorageError> {
        let config = BeliefConfigLoader::load_json(json)?;
        Ok(Self::new(
            belief_store,
            traversal_store,
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
        let query = TraversalQuery::new(self.traversal_store.as_ref());
        let anchor = query
            .current_anchor_for_subject(subject, anchor_perspective_kind, anchor_perspective_id)?
            .ok_or_else(|| StorageError::InvalidPath("missing graph anchor".to_string()))?;
        let provenance = query.provenance_for_anchor(&anchor.anchor_id)?;
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
        for item in &evidence {
            self.belief_store.put_evidence(item)?;
            self.belief_store
                .put_assignment(&normalizer.assign(item)?)?;
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
        let prior = self.belief_store.current_revision(&key)?;
        let lease = AssessmentLease {
            lease_id: format!("lease-{}-{}", source_cursor_end, key.index_key()),
            belief_key: key.clone(),
            epoch: source_cursor_end,
            owner_id: owner_id.to_string(),
            input_cursor_start: source_cursor_start,
            input_cursor_end: source_cursor_end,
            started_at_seq: source_cursor_end,
            expires_at_seq: source_cursor_end + 100,
            comparator_engine_id: self.config.config.comparator.engine_id.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            status: LeaseStatus::Queued,
        };
        let lease = self.belief_store.acquire_lease(lease)?;
        let mut output = BayesianComparator::assess(ComparatorInput {
            config: self.config.config.clone(),
            config_snapshot_hash: self.config.hash.clone(),
            prior_revision: prior,
            evidence: evidence.clone(),
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
        Ok(RuntimeAssessmentResult {
            evidence_count: evidence.len(),
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
