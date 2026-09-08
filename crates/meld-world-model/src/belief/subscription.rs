//! Belief-owned acceptance of Agent subscription requests.

use serde::{Deserialize, Serialize};

use crate::agent::AgentSubscriptionRequestV1;
use crate::error::StorageError;

use super::{BeliefFamilyRevision, BeliefStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceSubscriptionAcceptanceV1 {
    pub acceptance_id: String,
    pub request_id: String,
    pub source_owner: String,
    pub source_contract_revision: crate::belief::TheoryRevisionRef,
}

impl SourceSubscriptionAcceptanceV1 {
    fn new(request: &AgentSubscriptionRequestV1) -> Result<Self, StorageError> {
        let acceptance_id = stable_hash(&(
            request.request_id.as_str(),
            request.source_owner.as_str(),
            &request.source_contract_revision,
        ))?;
        Ok(Self {
            acceptance_id,
            request_id: request.request_id.clone(),
            source_owner: request.source_owner.clone(),
            source_contract_revision: request.source_contract_revision.clone(),
        })
    }

    fn verify_identity(&self) -> Result<(), StorageError> {
        let expected = stable_hash(&(
            self.request_id.as_str(),
            self.source_owner.as_str(),
            &self.source_contract_revision,
        ))?;
        if self.acceptance_id != expected {
            return Err(StorageError::InvalidPath(
                "source subscription acceptance identity mismatch".to_string(),
            ));
        }
        Ok(())
    }
}

/// Opaque proof that Belief durably accepted one exact subscription request.
///
/// Only [`BeliefSubscriptionAuthority`] can construct this proof. Agent
/// genesis may match its request identity but cannot manufacture or persist
/// Belief's decision.
#[derive(Debug)]
pub struct BeliefSubscriptionAcceptanceProof {
    acceptance: SourceSubscriptionAcceptanceV1,
}

impl BeliefSubscriptionAcceptanceProof {
    pub(crate) fn request_id(&self) -> &str {
        &self.acceptance.request_id
    }

    pub(crate) fn acceptance_id(&self) -> &str {
        &self.acceptance.acceptance_id
    }
}

/// Canonical Belief owner for durable subscription acceptance.
pub struct BeliefSubscriptionAuthority<'a> {
    store: &'a BeliefStore,
}

/// Live source handoff bound to exact family revisions in this Belief store.
pub struct BeliefSubscriptionSource {
    store: std::sync::Arc<BeliefStore>,
    registry: std::sync::Arc<dyn super::BeliefFamilyRegistry + Send + Sync>,
}

impl BeliefSubscriptionSource {
    pub fn new(
        store: std::sync::Arc<BeliefStore>,
        registry: std::sync::Arc<dyn super::BeliefFamilyRegistry + Send + Sync>,
    ) -> Self {
        Self { store, registry }
    }
}

impl crate::agent::AgentEpochSubscriptionPort for BeliefSubscriptionSource {
    fn returned_evidence(
        &self,
        request: &super::BeliefEvidenceReturnRequest,
    ) -> Result<Option<super::BeliefEvidenceReturnProof>, StorageError> {
        request.subscription.validate()?;
        if self
            .store
            .subscription_acceptance(&request.subscription.request_id)?
            .is_none()
        {
            return Ok(None);
        }
        super::evidence_return::resolve(&self.store, request)
    }

    fn subscribe(
        &self,
        request: &AgentSubscriptionRequestV1,
    ) -> Result<BeliefSubscriptionAcceptanceProof, StorageError> {
        request.validate()?;
        let reference = &request.source_contract_revision;
        let revision = self
            .registry
            .resolve(&reference.id, &reference.content_hash)?
            .ok_or_else(|| {
                StorageError::InvalidPath("subscription family revision is not installed".into())
            })?;
        BeliefSubscriptionAuthority::new(&self.store).accept(request, &revision)
    }
}

impl<'a> BeliefSubscriptionAuthority<'a> {
    pub fn new(store: &'a BeliefStore) -> Self {
        Self { store }
    }

    pub fn accept(
        &self,
        request: &AgentSubscriptionRequestV1,
        revision: &BeliefFamilyRevision,
    ) -> Result<BeliefSubscriptionAcceptanceProof, StorageError> {
        request.validate()?;
        if request.source_owner != "belief"
            || request.source_contract_revision.registry != "belief_family"
        {
            return Err(StorageError::InvalidPath(
                "Belief subscription request does not cite this owner and family".to_string(),
            ));
        }
        if revision.family_id != request.source_contract_revision.id
            || revision.content_hash != request.source_contract_revision.content_hash
            || revision.config.dimension_id != request.belief_key.dimension_id
            || revision.config.predicate_id != request.belief_key.predicate_id
            || revision.config.evidence_policy_id != request.belief_key.evidence_policy_id
        {
            return Err(StorageError::InvalidPath(
                "Belief subscription key differs from its exact family contract".to_string(),
            ));
        }
        if let Some(existing) = self.store.subscription_acceptance(&request.request_id)? {
            existing.verify_identity()?;
            if existing.request_id == request.request_id
                && existing.source_owner == request.source_owner
                && existing.source_contract_revision == request.source_contract_revision
            {
                self.store.put_subscription_acceptance(&existing, request)?;
                return Ok(BeliefSubscriptionAcceptanceProof {
                    acceptance: existing,
                });
            }
            return Err(StorageError::InvalidPath(
                "Belief subscription request conflicts with its durable acceptance".to_string(),
            ));
        }
        let acceptance = SourceSubscriptionAcceptanceV1::new(request)?;
        self.store
            .put_subscription_acceptance(&acceptance, request)?;
        Ok(BeliefSubscriptionAcceptanceProof { acceptance })
    }
}

fn stable_hash(value: &impl Serialize) -> Result<String, StorageError> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| StorageError::InvalidPath(failure.to_string()))
}

#[cfg(test)]
mod tests {
    use meld_events::DomainObjectRef;

    use super::*;
    use crate::belief::{
        configured_belief_key, BeliefFamilyConfig, BeliefFamilyRegistry, BeliefFamilyRegistryStore,
        BranchScope,
    };
    use crate::world_state::graph::PerspectiveKey;

    #[test]
    fn question_state_distinguishes_unassessed_pending_and_stale_interpretation() {
        use crate::belief::{
            BeliefAssessmentActor, BeliefAssessmentRequest, BeliefQuery, BeliefQuestionState,
        };
        use std::sync::Arc;
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let mut registry = BeliefFamilyRegistryStore::new(db).unwrap();
        let config: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../../../theory/startup/belief_family.startup_realization.json"
        ))
        .unwrap();
        let family = registry.install(config.clone(), 1).unwrap().1;
        let perspective = PerspectiveKey::new("agent", "observer").unwrap();
        let branch = BranchScope::main();
        let key = configured_belief_key(
            &family,
            &DomainObjectRef::new("nonce", "question", "one").unwrap(),
            &perspective,
            &branch,
        );
        let query = BeliefQuery::new(&store);
        assert!(query.question_state(&key, &family.revision_ref()).is_err());
        let request = AgentSubscriptionRequestV1::new(
            "observer".into(),
            "belief".into(),
            family.revision_ref(),
            key.clone(),
            "from_genesis".into(),
        )
        .unwrap();
        BeliefSubscriptionAuthority::new(&store)
            .accept(&request, &family)
            .unwrap();
        assert!(
            matches!(query.question_state(&key, &family.revision_ref()).unwrap(), BeliefQuestionState::NoCommittedRevision(answer) if answer.subscription_request_id == request.request_id)
        );
        store.mark_dirty(&key, 2).unwrap();
        assert_eq!(
            query.question_state(&key, &family.revision_ref()).unwrap(),
            BeliefQuestionState::AssessmentPending
        );
        store.clear_dirty(&key).unwrap();
        let mut actor = BeliefAssessmentActor::new(
            "assessment",
            store.clone(),
            Arc::new(registry.clone()),
            vec![family.family_id.clone()],
            vec![],
            perspective,
            branch,
        )
        .with_pinned_families(vec![family.clone()]);
        let report = actor.bounded_step(&BeliefAssessmentRequest {
            sequence: 3,
            max_items: 1,
        });
        assert_eq!(report.items_committed, 1, "{report:?}");
        assert!(matches!(
            query.question_state(&key, &family.revision_ref()).unwrap(),
            BeliefQuestionState::Committed(_)
        ));
        let mut changed = config;
        changed.default_prior = 0.2;
        let next_family = registry.install(changed, 4).unwrap().1;
        let next_request = AgentSubscriptionRequestV1::new(
            "observer".into(),
            "belief".into(),
            next_family.revision_ref(),
            key.clone(),
            "from_genesis".into(),
        )
        .unwrap();
        BeliefSubscriptionAuthority::new(&store)
            .accept(&next_request, &next_family)
            .unwrap();
        assert!(matches!(
            query
                .question_state(&key, &next_family.revision_ref())
                .unwrap(),
            BeliefQuestionState::StaleRevision { .. }
        ));
        store.mark_dirty(&key, 5).unwrap();
        assert!(matches!(
            query.question_state(&key, &family.revision_ref()).unwrap(),
            BeliefQuestionState::StaleRevision { .. } | BeliefQuestionState::AssessmentPending
        ));
    }

    #[test]
    fn accepted_observations_drive_bounded_assessment_without_configured_subjects() {
        use crate::belief::{BeliefAssessmentActor, BeliefAssessmentRequest, BeliefWorkSelector};
        use std::sync::Arc;
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path()).unwrap();
        let mut registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let mut config: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../../../theory/startup/belief_family.startup_realization.json"
        ))
        .unwrap();
        config.family_id = "independent_family_name".into();
        let family = registry.install(config, 1).unwrap().1;
        let perspective = PerspectiveKey::new("agent", "observer").unwrap();
        let make_request = |id: &str, branch: BranchScope| {
            AgentSubscriptionRequestV1::new(
                "observer".into(),
                "belief".into(),
                family.revision_ref(),
                configured_belief_key(
                    &family,
                    &DomainObjectRef::new("curation", "expected_nonce", id).unwrap(),
                    &perspective,
                    &branch,
                ),
                "from_genesis".into(),
            )
            .unwrap()
        };
        let first = make_request("a", BranchScope::main());
        let second = make_request("b", BranchScope::main());
        let foreign = make_request("c", BranchScope::new("foreign").unwrap());
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        for request in [&first, &second, &foreign] {
            BeliefSubscriptionAuthority::new(&store)
                .accept(request, &family)
                .unwrap();
        }
        let mut forged = first.clone();
        forged.belief_key.subject.object_id = "forged".into();
        assert!(BeliefSubscriptionAuthority::new(&store)
            .accept(&forged, &family)
            .is_err());
        let selection = BeliefWorkSelector::new(&store)
            .select(
                std::slice::from_ref(&family),
                &[],
                &perspective,
                &BranchScope::main(),
                1,
            )
            .unwrap();
        assert_eq!(selection.items.len(), 1);
        assert!(selection.more_available);
        assert_eq!(selection.items[0].key, first.belief_key);
        let mut actor = BeliefAssessmentActor::new(
            "assessment",
            store.clone(),
            Arc::new(registry),
            vec![family.family_id.clone()],
            vec![],
            perspective.clone(),
            BranchScope::main(),
        );
        let report = actor.bounded_step(&BeliefAssessmentRequest {
            sequence: 1,
            max_items: 1,
        });
        assert!(
            report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.items_committed, 1);
        assert!(report.budget_exhausted);
        let key = &first.belief_key;
        let wake = crate::waiting::StructuralWakeAddress::OwnerRevision(format!(
            "world-model::{}::belief-input::{}::successor",
            store.resource_id(),
            key.index_key()
        ));
        assert!(actor.resolves_wake(&wake).unwrap());
        let foreign_wake = crate::waiting::StructuralWakeAddress::OwnerRevision(format!(
            "world-model::{}::belief-input::{}::successor",
            store.resource_id(),
            foreign.belief_key.index_key()
        ));
        assert!(!actor.resolves_wake(&foreign_wake).unwrap());
        assert!(store.current_revision(&first.belief_key).unwrap().is_some());
        drop(actor);
        drop(store);
        drop(db);
        let db = sled::open(root.path()).unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        assert_eq!(
            store
                .accepted_subscription(&family.revision_ref(), &first.belief_key)
                .unwrap(),
            Some(first.clone())
        );
        let selection = BeliefWorkSelector::new(&store)
            .select(
                std::slice::from_ref(&family),
                &[],
                &perspective,
                &BranchScope::main(),
                1,
            )
            .unwrap();
        assert_eq!(selection.items.len(), 1);
        assert_eq!(selection.items[0].key, second.belief_key);
        let mut actor = BeliefAssessmentActor::new(
            "assessment",
            store.clone(),
            Arc::new(BeliefFamilyRegistryStore::new(db).unwrap()),
            vec![family.family_id.clone()],
            vec![],
            perspective.clone(),
            BranchScope::main(),
        );
        assert_eq!(
            actor
                .bounded_step(&BeliefAssessmentRequest {
                    sequence: 2,
                    max_items: 1
                })
                .items_committed,
            1
        );
        BeliefSubscriptionAuthority::new(&store)
            .accept(&first, &family)
            .unwrap();
        assert!(BeliefWorkSelector::new(&store)
            .select(
                std::slice::from_ref(&family),
                &[],
                &perspective,
                &BranchScope::main(),
                1
            )
            .unwrap()
            .items
            .is_empty());
        assert!(store
            .current_revision(&foreign.belief_key)
            .unwrap()
            .is_none());
    }

    #[test]
    fn dirty_selection_migrates_existing_records_and_excludes_foreign_scopes() {
        use crate::belief::{BeliefWorkSelector, DirtyKeyState, DirtyReason};
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path()).unwrap();
        let mut registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let family = registry
            .install(
                serde_json::from_str(include_str!(
                    "../../../../theory/startup/belief_family.startup_realization.json"
                ))
                .unwrap(),
                1,
            )
            .unwrap()
            .1;
        let perspective = PerspectiveKey::new("agent", "observer").unwrap();
        let key = configured_belief_key(
            &family,
            &DomainObjectRef::new("curation", "expected_nonce", "zzz-local").unwrap(),
            &perspective,
            &BranchScope::main(),
        );
        let mut foreign_branch = key.clone();
        foreign_branch.subject.object_id = "000-foreign-branch".into();
        foreign_branch.branch_scope = BranchScope::new("foreign").unwrap();
        let mut foreign_perspective = key.clone();
        foreign_perspective.subject.object_id = "000-foreign-perspective".into();
        foreign_perspective.perspective.perspective_kind = "other-kind".into();
        let old_tree = db.open_tree("belief_dirty_keys").unwrap();
        for key in [&key, &foreign_branch, &foreign_perspective] {
            let state = DirtyKeyState {
                belief_key: key.clone(),
                dirty_since_seq: 1,
                latest_seq: 1,
                active_lease_id: None,
                reason: DirtyReason::NewEvidence,
            };
            old_tree
                .insert(
                    key.index_key().as_bytes(),
                    serde_json::to_vec(&state).unwrap(),
                )
                .unwrap();
        }
        drop(old_tree);
        let store = BeliefStore::new(db.clone()).unwrap();
        let select = |store: &BeliefStore| {
            BeliefWorkSelector::new(store)
                .select(
                    std::slice::from_ref(&family),
                    &[],
                    &perspective,
                    &BranchScope::main(),
                    1,
                )
                .unwrap()
        };
        let selected = select(&store);
        assert_eq!(selected.items.len(), 1);
        assert_eq!(selected.items[0].key, key);
        assert!(!selected.more_available);
        drop(store);
        drop(registry);
        drop(db);
        let store = BeliefStore::new(sled::open(root.path()).unwrap()).unwrap();
        assert_eq!(select(&store).items[0].key, key);
        store.clear_dirty(&key).unwrap();
        let quiet = select(&store);
        assert!(quiet.items.is_empty() && !quiet.more_available);
        assert_eq!(store.dirty_key_states().unwrap().len(), 2);
        store.mark_dirty(&key, 2).unwrap();
        assert_eq!(select(&store).items[0].key, key);
    }

    #[test]
    fn belief_accepts_only_the_exact_family_key_and_reuses_its_receipt() {
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path()).unwrap();
        let mut registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let config: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/belief_family.docs_freshness.json"
        ))
        .unwrap();
        let revision = registry.install(config, 1).unwrap().1;
        let key = configured_belief_key(
            &revision,
            &DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            &PerspectiveKey::new("default", "default").unwrap(),
            &BranchScope::main(),
        );
        let request = AgentSubscriptionRequestV1::new(
            "agent-a".to_string(),
            "belief".to_string(),
            revision.revision_ref(),
            key,
            "from_genesis".to_string(),
        )
        .unwrap();
        let store = BeliefStore::new(db).unwrap();
        let first = {
            let authority = BeliefSubscriptionAuthority::new(&store);
            let first = authority.accept(&request, &revision).unwrap();
            let second = authority.accept(&request, &revision).unwrap();
            assert_eq!(first.acceptance_id(), second.acceptance_id());

            let mut mismatched = request;
            mismatched.belief_key.predicate_id = "other".to_string();
            assert!(authority.accept(&mismatched, &revision).is_err());
            first.acceptance
        };
        drop(store);
        drop(registry);

        let reopened = BeliefStore::new(sled::open(root.path()).unwrap()).unwrap();
        assert_eq!(
            reopened.subscription_acceptance(&first.request_id).unwrap(),
            Some(first)
        );
    }
}
