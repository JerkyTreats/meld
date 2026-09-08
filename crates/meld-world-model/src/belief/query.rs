//! Read-only belief query facade.
//!
//! Planners and agents should use this module instead of raw belief store
//! trees. The query surface exposes views and provenance while hiding
//! comparator work state and lease internals.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::belief::{BeliefQuery, BeliefStore};
//!
//! let temp = tempfile::tempdir().unwrap();
//! let store = BeliefStore::new(sled::open(temp.path()).unwrap()).unwrap();
//! let query = BeliefQuery::new(&store);
//! let dirty = query.dirty_keys().unwrap();
//! assert!(dirty.is_empty());
//! ```

use crate::belief::contracts::{
    BeliefKey, BeliefProvenanceSummary, BeliefRevision, BeliefView, DirtyKeyState, EvidenceItem,
    ObservationOpportunity,
};
use crate::belief::store::BeliefStore;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Planner-safe read facade over belief storage.
pub struct BeliefQuery<'a> {
    store: &'a BeliefStore,
}

/// Belief's answer for a durably accepted question before its first assessment.
/// This is an absence of a revision, never a prior or a source revision.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnassessedBeliefQuestion {
    pub key: BeliefKey,
    pub family: crate::belief::TheoryRevisionRef,
    pub subscription_request_id: String,
    pub subscription_acceptance_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BeliefQuestionState {
    Committed(Box<(BeliefRevision, BeliefView)>),
    NoCommittedRevision(Box<UnassessedBeliefQuestion>),
    AssessmentPending,
    StaleRevision { revision_id: String },
}

impl<'a> BeliefQuery<'a> {
    /// Resolve the exact subscribed question without treating owner failure or
    /// pending evidence as an unassessed condition.
    pub fn question_state(
        &self,
        key: &BeliefKey,
        family: &crate::belief::TheoryRevisionRef,
    ) -> Result<BeliefQuestionState, StorageError> {
        family.validate_for_registry("belief_family")?;
        key.validate()?;
        let request = self
            .store
            .accepted_subscription(family, key)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "Belief question has no accepted exact subscription".into(),
                )
            })?;
        let acceptance = self
            .store
            .subscription_acceptance(&request.request_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath("Belief question acceptance is unavailable".into())
            })?;
        if let Some(current) = self.current_revision_and_view(key)? {
            if current.0.theory_revision.as_ref() != Some(family)
                || current.1.freshness.stale
                || matches!(
                    current.1.status,
                    crate::belief::BeliefStatus::Stale | crate::belief::BeliefStatus::Invalid
                )
            {
                return Ok(BeliefQuestionState::StaleRevision {
                    revision_id: current.0.revision_id,
                });
            }
            if matches!(
                current.1.status,
                crate::belief::BeliefStatus::AssessmentPending
                    | crate::belief::BeliefStatus::NeedsAssessment
            ) {
                return Ok(BeliefQuestionState::AssessmentPending);
            }
            return Ok(BeliefQuestionState::Committed(Box::new(current)));
        }
        if self.store.dirty_state(key)?.is_some() {
            return Ok(BeliefQuestionState::AssessmentPending);
        }
        Ok(BeliefQuestionState::NoCommittedRevision(Box::new(
            UnassessedBeliefQuestion {
                key: key.clone(),
                family: family.clone(),
                subscription_request_id: request.request_id,
                subscription_acceptance_id: acceptance.acceptance_id,
            },
        )))
    }
    /// Create a query facade over an existing store.
    pub fn new(store: &'a BeliefStore) -> Self {
        Self { store }
    }

    /// Read the current view for one belief key.
    pub fn current_view(&self, key: &BeliefKey) -> Result<Option<BeliefView>, StorageError> {
        self.store.current_view(key)
    }

    /// Read the current committed revision for one exact belief key.
    pub fn current_revision(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<BeliefRevision>, StorageError> {
        self.store.current_revision(key)
    }

    /// Read the current revision and its planner-safe view for one exact key.
    ///
    /// The revision head is the durable authority: the returned view is
    /// rebuilt from that revision when the view cache is missing, and both
    /// halves always cite the same revision identity. A newer dirty assignment
    /// makes the returned projection pending without rewriting that revision.
    pub fn current_revision_and_view(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<(BeliefRevision, BeliefView)>, StorageError> {
        let Some(revision) = self.store.current_revision(key)? else {
            return Ok(None);
        };
        let mut view = match self.store.current_view(key)? {
            Some(view) if view.current_revision_id.as_deref() == Some(&revision.revision_id) => {
                view
            }
            // Cache miss or a stale cache entry: durable revision state wins.
            _ => self
                .store
                .rebuild_current_view_from_revision(key)?
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "revision head vanished while rebuilding its view".to_string(),
                    )
                })?,
        };
        // The immutable revision remains inspectable while a newer assignment
        // awaits assessment. Its old posterior cannot authorize current work.
        if self
            .store
            .dirty_state(key)?
            .is_some_and(|dirty| dirty.latest_seq > revision.source_cursor_end)
        {
            view.status = crate::belief::BeliefStatus::AssessmentPending;
            view.freshness.stale = true;
            if !view
                .freshness
                .reasons
                .contains(&crate::belief::FreshnessReason::NewerEvidence)
            {
                view.freshness
                    .reasons
                    .push(crate::belief::FreshnessReason::NewerEvidence);
            }
        }
        Ok(Some((revision, view)))
    }

    /// Verify that the selected revision actually consumed the current native
    /// Curation judgment through the exact installed interpretation resources.
    pub fn supports_current_curation(
        &self,
        revision_id: &str,
        basis: &crate::curation::CurationEvidenceBasisProof,
        family: &crate::belief::TheoryRevisionRef,
        mappings: &[crate::belief::TheoryRevisionRef],
    ) -> Result<bool, StorageError> {
        let Some(revision) = self.store.get_revision(revision_id)? else {
            return Ok(false);
        };
        if family.registry != "belief_family"
            || mappings.is_empty()
            || mappings
                .iter()
                .any(|mapping| mapping.registry != "outcome_mapping")
            || revision.theory_revision.as_ref() != Some(family)
        {
            return Ok(false);
        }
        Ok(self
            .store
            .evidence_by_revision(&revision.revision_id)?
            .iter()
            .any(|evidence| {
                evidence.candidate_key == revision.belief_key
                    && evidence.publication_record_id.as_ref() == Some(&basis.publication_record_id)
                    && evidence
                        .outcome_mapping_revision
                        .as_ref()
                        .is_some_and(|mapping| mappings.contains(mapping))
                    && revision
                        .supporting_evidence_ids
                        .contains(&evidence.evidence_id)
            }))
    }

    /// Read current views for one subject and perspective.
    pub fn current_views_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective: &PerspectiveKey,
    ) -> Result<Vec<BeliefView>, StorageError> {
        self.store.views_for_subject(subject, perspective)
    }

    /// Read committed revisions for one key in source order.
    pub fn revision_history(&self, key: &BeliefKey) -> Result<Vec<BeliefRevision>, StorageError> {
        self.store.revision_history(key)
    }

    /// Hydrate evidence used by a revision.
    pub fn evidence_by_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<EvidenceItem>, StorageError> {
        self.store.evidence_by_revision(revision_id)
    }

    /// Read provenance for a revision without exposing raw payloads.
    pub fn provenance_by_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<BeliefProvenanceSummary>, StorageError> {
        self.store.provenance_by_revision(revision_id)
    }

    /// Read open observation opportunities for a subject and perspective.
    pub fn open_observation_opportunities(
        &self,
        subject: &DomainObjectRef,
        perspective: &PerspectiveKey,
    ) -> Result<Vec<ObservationOpportunity>, StorageError> {
        self.store
            .open_observation_opportunities(subject, perspective)
    }

    /// Read dirty key index entries for recovery and diagnostics.
    pub fn dirty_keys(&self) -> Result<Vec<String>, StorageError> {
        self.store.dirty_keys()
    }

    /// Read structured dirty key state for recovery and storm coalescing.
    pub fn dirty_key_states(&self) -> Result<Vec<DirtyKeyState>, StorageError> {
        self.store.dirty_key_states()
    }

    /// Rebuild the current view from durable revision state.
    pub fn rebuild_current_view_from_revision(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<BeliefView>, StorageError> {
        self.store.rebuild_current_view_from_revision(key)
    }
}
