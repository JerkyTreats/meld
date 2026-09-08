//! Deterministic bounded selection of belief assessment work.
//!
//! Owner: world model belief domain. The selector reads only durable state:
//! configured subject bindings crossed with installed family revisions yield
//! initial-assessment candidates (exact keys with no committed revision), and
//! the durable dirty-key index yields keys invalidated by newer evidence.
//! Output order is deterministic for identical durable state, and no source is
//! scanned unboundedly where an index exists, so a bounded actor can make
//! resumable progress budget by budget.

use crate::belief::contracts::{BeliefKey, BranchScope, DirtyKeyState};
use crate::belief::registry::BeliefFamilyRevision;
use crate::belief::store::BeliefStore;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// One configured subject a belief family should assess.
///
/// Subject bindings come from composition-time configuration, not from
/// scanning graph state, so the initial-assessment candidate set is bounded
/// by what the product explicitly bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefSubjectBinding {
    /// Object whose state the family assesses.
    pub subject: DomainObjectRef,
}

/// Why a belief key was selected for assessment work.
#[derive(Debug, Clone, PartialEq)]
pub enum BeliefWorkKind {
    /// An installed family has no committed revision for this exact key.
    InitialAssessment {
        /// Subject binding to assess through the graph anchor path.
        binding: BeliefSubjectBinding,
    },
    /// Durable dirty state marks this key as invalidated by newer evidence.
    DirtyKey {
        /// Durable dirty record backing the selection.
        state: DirtyKeyState,
    },
}

/// One selected unit of belief assessment work.
#[derive(Debug, Clone, PartialEq)]
pub struct BeliefWorkItem {
    /// Exact configured belief key the work targets.
    pub key: BeliefKey,
    /// Family identity whose installed revision governs the assessment.
    pub family_id: String,
    /// Selection cause and the durable record behind it.
    pub kind: BeliefWorkKind,
}

/// Bounded, deterministic selection result.
#[derive(Debug, Clone, PartialEq)]
pub struct BeliefWorkSelection {
    /// Selected items, at most the requested budget.
    pub items: Vec<BeliefWorkItem>,
    /// True when durable state held more eligible work than the budget.
    pub more_available: bool,
}

/// Derive the exact belief key one family config addresses for one subject.
///
/// This is the single derivation used by selection, assessment, and planner
/// consumers, so "the configured belief key" never diverges between the
/// writer and the reader side.
pub fn configured_belief_key(
    revision: &BeliefFamilyRevision,
    subject: &DomainObjectRef,
    perspective: &PerspectiveKey,
    branch_scope: &BranchScope,
) -> BeliefKey {
    BeliefKey {
        subject: subject.clone(),
        dimension_id: revision.config.dimension_id.clone(),
        predicate_id: revision.config.predicate_id.clone(),
        perspective: perspective.clone(),
        branch_scope: branch_scope.clone(),
        evidence_policy_id: revision.config.evidence_policy_id.clone(),
    }
}

/// Selector over durable belief state for bounded assessment work.
pub struct BeliefWorkSelector<'a> {
    store: &'a BeliefStore,
}

impl<'a> BeliefWorkSelector<'a> {
    /// Bind the selector to durable belief storage.
    pub fn new(store: &'a BeliefStore) -> Self {
        Self { store }
    }

    /// Select up to `max_items` keys needing work in deterministic order.
    ///
    /// Initial-assessment candidates come first, ordered by family id then
    /// subject index key; dirty keys follow in dirty-index order. A key never
    /// appears twice: dirty state supersedes an initial candidate because the
    /// dirty path replays the admitted evidence from durable assignments.
    pub fn select(
        &self,
        families: &[BeliefFamilyRevision],
        subjects: &[BeliefSubjectBinding],
        perspective: &PerspectiveKey,
        branch_scope: &BranchScope,
        max_items: usize,
    ) -> Result<BeliefWorkSelection, StorageError> {
        if max_items == 0 {
            return Err(StorageError::InvalidPath(
                "belief selection budget must be greater than zero".to_string(),
            ));
        }
        let mut ordered_families: Vec<&BeliefFamilyRevision> = families.iter().collect();
        ordered_families.sort_by(|left, right| left.family_id.cmp(&right.family_id));
        let mut ordered_subjects: Vec<&BeliefSubjectBinding> = subjects.iter().collect();
        ordered_subjects.sort_by_key(|binding| binding.subject.index_key());

        let mut items = Vec::new();
        let mut selected_keys = Vec::new();
        let mut more_available = false;

        for family in &ordered_families {
            // No runnable initial assessment exists until Curation admits evidence.
            // Dirty work below becomes eligible through the same durable assignment path.
            if family.config.initial_assessment == crate::belief::InitialAssessmentPolicy::Required
            {
                continue;
            }
            for binding in &ordered_subjects {
                let key =
                    configured_belief_key(family, &binding.subject, perspective, branch_scope);
                // Each candidate is one indexed head lookup plus one dirty
                // lookup; the candidate set itself is bounded by configuration.
                if self.store.current_revision(&key)?.is_some()
                    || self.store.dirty_state(&key)?.is_some()
                {
                    continue;
                }
                if items.len() == max_items {
                    more_available = true;
                    break;
                }
                selected_keys.push(key.index_key());
                items.push(BeliefWorkItem {
                    key,
                    family_id: family.family_id.clone(),
                    kind: BeliefWorkKind::InitialAssessment {
                        binding: (*binding).clone(),
                    },
                });
            }
            if !more_available {
                let (requests, more) = self.store.pending_subscriptions(
                    &family.revision_ref(),
                    perspective,
                    branch_scope,
                    max_items.saturating_sub(items.len()),
                )?;
                more_available |= more;
                for request in requests {
                    let key = request.belief_key;
                    if selected_keys.contains(&key.index_key())
                        || self.store.dirty_state(&key)?.is_some()
                    {
                        continue;
                    }
                    selected_keys.push(key.index_key());
                    items.push(BeliefWorkItem {
                        family_id: family.family_id.clone(),
                        kind: BeliefWorkKind::InitialAssessment {
                            binding: BeliefSubjectBinding {
                                subject: key.subject.clone(),
                            },
                        },
                        key,
                    });
                }
            }
            if more_available {
                break;
            }
        }

        for family in &ordered_families {
            let (dirty_states, dirty_more) = self.store.scoped_dirty_key_states_bounded(
                family,
                perspective,
                branch_scope,
                max_items.saturating_sub(items.len()),
            )?;
            more_available |= dirty_more;
            for state in dirty_states {
                if selected_keys.contains(&state.belief_key.index_key()) {
                    continue;
                }
                selected_keys.push(state.belief_key.index_key());
                items.push(BeliefWorkItem {
                    key: state.belief_key.clone(),
                    family_id: family.family_id.clone(),
                    kind: BeliefWorkKind::DirtyKey { state },
                });
            }
        }

        Ok(BeliefWorkSelection {
            items,
            more_available,
        })
    }
}
