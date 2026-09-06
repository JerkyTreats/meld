//! Agent-owned genesis intent, subscription request, and receipt handling.

use serde::{Deserialize, Serialize};

use crate::belief::{BeliefKey, BeliefSubscriptionAcceptanceProof, TheoryRevisionRef};
use crate::error::StorageError;
use crate::events::{AppendMode, EventAppendCapability, EventEnvelope, LedgerCursor};

use super::registration::AgentRegistration;
use super::subscription::AgentSubscription;
use super::{AgentStore, AgentSubscriptionRecord, SeedAgentRegistration, SubscribeAgentCommand};

/// One exact source relationship requested by an Agent genesis intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSubscriptionRequestV1 {
    pub request_id: String,
    pub agent_id: String,
    pub source_owner: String,
    pub source_contract_revision: TheoryRevisionRef,
    pub belief_key: BeliefKey,
    pub initial_cursor_policy: String,
}

impl AgentSubscriptionRequestV1 {
    pub fn validate(&self) -> Result<(), StorageError> {
        let canonical = Self::new(
            self.agent_id.clone(),
            self.source_owner.clone(),
            self.source_contract_revision.clone(),
            self.belief_key.clone(),
            self.initial_cursor_policy.clone(),
        )?;
        if &canonical != self {
            return Err(StorageError::InvalidPath(
                "Agent subscription identity differs from its request".into(),
            ));
        }
        Ok(())
    }

    pub fn new(
        agent_id: String,
        source_owner: String,
        source_contract_revision: TheoryRevisionRef,
        belief_key: BeliefKey,
        initial_cursor_policy: String,
    ) -> Result<Self, StorageError> {
        let values = [
            agent_id.as_str(),
            source_owner.as_str(),
            initial_cursor_policy.as_str(),
        ];
        if values.iter().any(|value| value.trim().is_empty()) {
            return Err(StorageError::InvalidPath(
                "Agent subscription request identities must be non-empty".to_string(),
            ));
        }
        source_contract_revision.validate_for_registry("belief_family")?;
        belief_key.validate()?;
        let request_id = stable_hash(&(
            agent_id.as_str(),
            source_owner.as_str(),
            &source_contract_revision,
            &belief_key,
            initial_cursor_policy.as_str(),
        ))?;
        Ok(Self {
            request_id,
            agent_id,
            source_owner,
            source_contract_revision,
            belief_key,
            initial_cursor_policy,
        })
    }
}

/// Complete Agent-owned request for one declared topology position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGenesisIntentV1 {
    pub intent_id: String,
    pub assignment_id: String,
    pub product_compilation_receipt_id: String,
    pub topology_position_id: String,
    pub installed_owner_revisions: Vec<TheoryRevisionRef>,
    pub registration: SeedAgentRegistration,
    pub required_subscriptions: Vec<AgentSubscriptionRequestV1>,
}

impl AgentGenesisIntentV1 {
    pub fn new(
        assignment_id: String,
        product_compilation_receipt_id: String,
        topology_position_id: String,
        mut installed_owner_revisions: Vec<TheoryRevisionRef>,
        registration: SeedAgentRegistration,
        mut required_subscriptions: Vec<AgentSubscriptionRequestV1>,
    ) -> Result<Self, StorageError> {
        registration.validate()?;
        installed_owner_revisions.sort_by(|left, right| {
            left.registry
                .cmp(&right.registry)
                .then(left.id.cmp(&right.id))
                .then(left.content_hash.cmp(&right.content_hash))
        });
        installed_owner_revisions.dedup();
        required_subscriptions.sort_by(|left, right| left.request_id.cmp(&right.request_id));
        if assignment_id.trim().is_empty()
            || product_compilation_receipt_id.trim().is_empty()
            || topology_position_id.trim().is_empty()
            || installed_owner_revisions.is_empty()
            || required_subscriptions.is_empty()
            || required_subscriptions
                .iter()
                .any(|request| request.agent_id != registration.agent_id)
            || !required_subscriptions
                .windows(2)
                .all(|pair| pair[0].request_id != pair[1].request_id)
            || installed_owner_revisions.iter().any(|revision| {
                revision.registry.trim().is_empty()
                    || revision.id.trim().is_empty()
                    || revision.content_hash.trim().is_empty()
            })
        {
            return Err(StorageError::InvalidPath(
                "Agent genesis intent is incomplete or internally inconsistent".to_string(),
            ));
        }
        let intent_id = stable_hash(&(
            assignment_id.as_str(),
            product_compilation_receipt_id.as_str(),
            topology_position_id.as_str(),
            &installed_owner_revisions,
            &registration,
            &required_subscriptions,
        ))?;
        Ok(Self {
            intent_id,
            assignment_id,
            product_compilation_receipt_id,
            topology_position_id,
            installed_owner_revisions,
            registration,
            required_subscriptions,
        })
    }
}

/// Durable Agent publication operation prepared during genesis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGenesisPublicationV1 {
    pub publication_id: String,
    pub intent_id: String,
    pub agent_id: String,
    pub assignment_id: String,
    pub product_compilation_receipt_id: String,
    pub topology_position_id: String,
}

impl AgentGenesisPublicationV1 {
    fn new(intent: &AgentGenesisIntentV1) -> Result<Self, StorageError> {
        let publication_id = stable_hash(&(
            "agent.genesis.v1",
            intent.intent_id.as_str(),
            intent.registration.agent_id.as_str(),
            intent.assignment_id.as_str(),
            intent.product_compilation_receipt_id.as_str(),
            intent.topology_position_id.as_str(),
        ))?;
        Ok(Self {
            publication_id,
            intent_id: intent.intent_id.clone(),
            agent_id: intent.registration.agent_id.clone(),
            assignment_id: intent.assignment_id.clone(),
            product_compilation_receipt_id: intent.product_compilation_receipt_id.clone(),
            topology_position_id: intent.topology_position_id.clone(),
        })
    }

    pub fn event_record_id(&self) -> String {
        format!("agent-genesis::{}", self.publication_id)
    }

    pub fn event_envelope(&self, session_id: &str) -> Result<EventEnvelope, StorageError> {
        let data = serde_json::to_value(self)
            .map_err(|failure| StorageError::InvalidPath(failure.to_string()))?;
        Ok(EventEnvelope::with_now_domain(
            session_id,
            "world-model",
            format!("agent-genesis::{}", self.agent_id),
            "world-model.agent-genesis.v1",
            None,
            data,
        )
        .with_record_id(self.event_record_id()))
    }
}

/// Complete Agent-owned acceptance of one topology position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGenesisReceiptV1 {
    pub genesis_receipt_id: String,
    pub intent_id: String,
    pub assignment_id: String,
    pub product_compilation_receipt_id: String,
    pub topology_position_id: String,
    pub agent_id: String,
    pub agent_record_revision_id: String,
    pub installed_owner_revisions: Vec<TheoryRevisionRef>,
    pub source_acceptance_ids: Vec<String>,
    pub subscription_ids: Vec<String>,
    pub publication_id: String,
    pub event_record_id: String,
    pub event_position: LedgerCursor,
}

impl AgentGenesisReceiptV1 {
    pub fn verify_identity(&self) -> Result<(), StorageError> {
        let expected = stable_hash(&(
            self.intent_id.as_str(),
            self.assignment_id.as_str(),
            self.product_compilation_receipt_id.as_str(),
            self.topology_position_id.as_str(),
            self.agent_id.as_str(),
            self.agent_record_revision_id.as_str(),
            &self.installed_owner_revisions,
            &self.source_acceptance_ids,
            &self.subscription_ids,
            self.publication_id.as_str(),
            self.event_record_id.as_str(),
            self.event_position,
        ))?;
        if self.genesis_receipt_id != expected || self.event_position.after_seq == 0 {
            return Err(StorageError::InvalidPath(
                "Agent genesis receipt identity or Event position differs".to_string(),
            ));
        }
        Ok(())
    }
}

/// Prepared Agent-owned state waiting on named source owners.
pub struct PendingAgentGenesis {
    intent: AgentGenesisIntentV1,
    agent_record_revision_id: String,
    publication: AgentGenesisPublicationV1,
}

/// Canonical Agent genesis authority over the existing Agent store.
pub struct AgentGenesis<'a> {
    store: &'a AgentStore,
    append: &'a EventAppendCapability,
}

impl<'a> AgentGenesis<'a> {
    pub fn new(store: &'a AgentStore, append: &'a EventAppendCapability) -> Self {
        Self { store, append }
    }

    pub fn prepare(
        &self,
        intent: AgentGenesisIntentV1,
    ) -> Result<PendingAgentGenesis, StorageError> {
        self.store
            .claim_genesis_lineage(&intent.registration.agent_id, &intent.intent_id)?;
        self.store.put_genesis_intent(&intent)?;
        let record =
            AgentRegistration::new(self.store).register_seed_agent(intent.registration.clone())?;
        for request in &intent.required_subscriptions {
            self.store.put_subscription_request(request)?;
        }
        let publication = AgentGenesisPublicationV1::new(&intent)?;
        self.store.put_genesis_publication(&publication)?;
        let agent_record_revision_id = stable_hash(&(
            &record.agent_id,
            &record.perspective_key,
            &record.subject,
            &record.branch_scope,
            &record.observation_scope,
            &record.directive,
            &record.seed_provenance,
            &record.curation_rule,
            &record.curation_rule_revision,
            &record.maintained_condition,
            &record.maintained_condition_revision,
        ))?;
        Ok(PendingAgentGenesis {
            intent,
            agent_record_revision_id,
            publication,
        })
    }

    pub fn complete(
        &self,
        pending: PendingAgentGenesis,
        mut acceptances: Vec<BeliefSubscriptionAcceptanceProof>,
        session_id: &str,
    ) -> Result<(AgentGenesisReceiptV1, bool), StorageError> {
        let event_record_id = pending.publication.event_record_id();
        acceptances.sort_by(|left, right| left.request_id().cmp(right.request_id()));
        if acceptances.len() != pending.intent.required_subscriptions.len() {
            return Err(StorageError::InvalidPath(
                "Agent genesis source acceptance set is incomplete".to_string(),
            ));
        }
        for (request, acceptance) in pending
            .intent
            .required_subscriptions
            .iter()
            .zip(&acceptances)
        {
            if acceptance.request_id() != request.request_id {
                return Err(StorageError::InvalidPath(
                    "Agent genesis source acceptance does not match its request".to_string(),
                ));
            }
        }
        let append_proof = self
            .append
            .append_durable_proven(
                pending.publication.event_envelope(session_id)?,
                AppendMode::Idempotent,
            )
            .map_err(|failure| StorageError::InvalidPath(failure.to_string()))?;
        if append_proof.record_id() != event_record_id {
            return Err(StorageError::InvalidPath(
                "Agent genesis Event proof belongs to another publication".to_string(),
            ));
        }
        let mut subscriptions: Vec<AgentSubscriptionRecord> = Vec::new();
        for request in &pending.intent.required_subscriptions {
            subscriptions.push(AgentSubscription::new(self.store).subscribe(
                SubscribeAgentCommand {
                    agent_id: request.agent_id.clone(),
                    belief_key: request.belief_key.clone(),
                    created_at_seq: pending.intent.registration.created_at_seq,
                },
            )?);
        }
        AgentRegistration::new(self.store).mark_operational(
            &pending.intent.registration.agent_id,
            pending.intent.registration.created_at_seq,
        )?;
        let event_position = LedgerCursor {
            ledger_id: append_proof.ledger_id(),
            after_seq: append_proof.seq(),
        };
        let publication_id = pending.publication.publication_id;
        let source_acceptance_ids = acceptances
            .iter()
            .map(|acceptance| acceptance.acceptance_id().to_string())
            .collect::<Vec<_>>();
        let subscription_ids = subscriptions
            .iter()
            .map(|subscription| subscription.subscription_id.clone())
            .collect::<Vec<_>>();
        let genesis_receipt_id = stable_hash(&(
            pending.intent.intent_id.as_str(),
            pending.intent.assignment_id.as_str(),
            pending.intent.product_compilation_receipt_id.as_str(),
            pending.intent.topology_position_id.as_str(),
            pending.intent.registration.agent_id.as_str(),
            pending.agent_record_revision_id.as_str(),
            &pending.intent.installed_owner_revisions,
            &source_acceptance_ids,
            &subscription_ids,
            publication_id.as_str(),
            event_record_id.as_str(),
            event_position,
        ))?;
        let receipt = AgentGenesisReceiptV1 {
            genesis_receipt_id,
            intent_id: pending.intent.intent_id,
            assignment_id: pending.intent.assignment_id,
            product_compilation_receipt_id: pending.intent.product_compilation_receipt_id,
            topology_position_id: pending.intent.topology_position_id,
            agent_id: pending.intent.registration.agent_id,
            agent_record_revision_id: pending.agent_record_revision_id,
            installed_owner_revisions: pending.intent.installed_owner_revisions,
            source_acceptance_ids,
            subscription_ids,
            publication_id,
            event_record_id,
            event_position,
        };
        let changed = self.store.put_genesis_receipt(&receipt)?;
        Ok((receipt, changed))
    }
}

fn stable_hash(value: &impl Serialize) -> Result<String, StorageError> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| StorageError::InvalidPath(failure.to_string()))
}

#[cfg(test)]
mod tests {
    use meld_events::{AppendMode, DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};

    use super::*;
    use crate::belief::{
        configured_belief_key, BeliefFamilyConfig, BeliefFamilyRegistry, BeliefFamilyRegistryStore,
        BeliefStore, BeliefSubscriptionAuthority, BranchScope,
    };
    use crate::world_state::graph::PerspectiveKey;

    fn registration() -> SeedAgentRegistration {
        SeedAgentRegistration {
            agent_id: "agent-a".to_string(),
            perspective_key: PerspectiveKey::new("default", "default").unwrap(),
            subject: DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            branch_scope: BranchScope::main(),
            observation_scope: "docs".to_string(),
            directive: "steward docs".to_string(),
            seed_provenance: "test".to_string(),
            curation_rule: None,
            curation_rule_revision: None,
            maintained_condition: None,
            maintained_condition_revision: None,
            created_at_seq: 1,
        }
    }

    fn subscription_request(
        db: &sled::Db,
    ) -> (
        AgentSubscriptionRequestV1,
        crate::belief::BeliefFamilyRevision,
    ) {
        let mut registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let config: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/belief_family.docs_freshness.json"
        ))
        .unwrap();
        let revision = registry.install(config, 1).unwrap().1;
        let perspective = PerspectiveKey::new("default", "default").unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "docs").unwrap();
        let key = configured_belief_key(&revision, &subject, &perspective, &BranchScope::main());
        let request = AgentSubscriptionRequestV1::new(
            "agent-a".to_string(),
            "belief".to_string(),
            revision.revision_ref(),
            key,
            "from_genesis".to_string(),
        )
        .unwrap();
        (request, revision)
    }

    fn intent(assignment_id: &str, request: AgentSubscriptionRequestV1) -> AgentGenesisIntentV1 {
        AgentGenesisIntentV1::new(
            assignment_id.to_string(),
            "compilation-a".to_string(),
            "steward".to_string(),
            vec![request.source_contract_revision.clone()],
            registration(),
            vec![request],
        )
        .unwrap()
    }

    #[test]
    fn complete_genesis_is_idempotent_and_requires_exact_owner_acceptance() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = AgentStore::new(db.clone()).unwrap();
        let authority = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let append = authority.append_capability();
        let genesis = AgentGenesis::new(&store, &append);
        let (request, revision) = subscription_request(&db);
        let expected = intent("assignment-a", request.clone());
        let pending = genesis.prepare(expected.clone()).unwrap();
        assert!(genesis
            .complete(pending, Vec::new(), "agent-genesis-test")
            .is_err());

        let belief = BeliefStore::new(db).unwrap();
        let owner = BeliefSubscriptionAuthority::new(&belief);
        let first_acceptance = owner.accept(&request, &revision).unwrap();
        let first = genesis
            .complete(
                genesis.prepare(expected.clone()).unwrap(),
                vec![first_acceptance],
                "agent-genesis-test",
            )
            .unwrap();
        let second_acceptance = owner.accept(&request, &revision).unwrap();
        let second = genesis
            .complete(
                genesis.prepare(expected).unwrap(),
                vec![second_acceptance],
                "agent-genesis-test",
            )
            .unwrap();
        assert!(first.1);
        assert!(!second.1);
        assert_eq!(first.0, second.0);
        first.0.verify_identity().unwrap();
        let mut corrupt = first.0;
        corrupt.event_position.after_seq += 1;
        assert!(corrupt.verify_identity().is_err());
    }

    #[test]
    fn one_agent_id_cannot_silently_move_to_another_assignment() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = AgentStore::new(db.clone()).unwrap();
        let authority = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let append = authority.append_capability();
        let genesis = AgentGenesis::new(&store, &append);
        let request = subscription_request(&db).0;
        genesis
            .prepare(intent("assignment-a", request.clone()))
            .unwrap();
        let error = genesis
            .prepare(intent("assignment-b", request))
            .err()
            .unwrap();
        assert!(error.to_string().contains("different genesis lineage"));
    }

    #[test]
    fn genesis_publication_uses_the_injected_canonical_event_authority() {
        let canonical = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let canonical_append = canonical.append_capability();
        let foreign = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = AgentStore::new(db.clone()).unwrap();
        let genesis = AgentGenesis::new(&store, &canonical_append);
        let (request, revision) = subscription_request(&db);
        let expected_intent = intent("assignment-a", request.clone());
        let pending = genesis.prepare(expected_intent.clone()).unwrap();
        let foreign_proof = foreign
            .append_capability()
            .append_durable_proven(
                pending
                    .publication
                    .event_envelope("foreign-agent-genesis")
                    .unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        let belief = BeliefStore::new(db).unwrap();
        let acceptance = BeliefSubscriptionAuthority::new(&belief)
            .accept(&request, &revision)
            .unwrap();
        let expected_record_id = pending.publication.event_record_id();
        let receipt = genesis
            .complete(pending, vec![acceptance], "canonical-agent-genesis")
            .unwrap()
            .0;

        assert_ne!(canonical.ledger_identity(), foreign.ledger_identity());
        assert_eq!(foreign_proof.record_id(), expected_record_id);
        assert_eq!(receipt.event_record_id, expected_record_id);
        assert_eq!(
            receipt.event_position.ledger_id,
            canonical.ledger_identity()
        );
        assert_ne!(receipt.event_position.ledger_id, foreign_proof.ledger_id());
    }
}
