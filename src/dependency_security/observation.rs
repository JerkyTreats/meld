//! Durable, standing advisory observations. These are source evidence, never Task returns.

use super::{capability::*, contracts::*, policy::DependencySecurityPolicyV1, publication::OWNER};
use meld_events::{
    AppendMode, EventAppendCapability, EventEnvelope, EventRecord, EventReplayCapability,
    LedgerCursor, ReplayRequest,
};
use meld_lang::AuthorityPolicyBinding;
use serde::{Deserialize, Serialize};

pub const EVENT: &str = "dependency_security.advisory_observed.v1";
pub(super) const UNAVAILABLE: &str = "dependency_security_advisory_unavailable";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct AdvisoryObservation {
    pub subject: DependencySecuritySubjectV1,
    pub policy: DependencySecurityPolicyV1,
    pub authority: AuthorityPolicyBinding,
    pub binding_id: String,
    pub predecessor_source_id: Option<String>,
    pub generation_id: String,
    pub incarnation_id: String,
    pub observed_at: u64,
    pub advisory: Option<AdvisoryKnowledgeSnapshotV1>,
    pub failure: Option<String>,
}

pub(super) fn validate_permission(
    authority: &AuthorityPolicyBinding,
    subject: &meld_events::DomainObjectRef,
) -> Result<(), String> {
    authority.validate().map_err(|e| e.to_string())?;
    let policy = &authority.policy;
    if &policy.subject != subject
        || !policy
            .principal_granted_action_ids
            .iter()
            .any(|id| id == ACQUIRE_ADVISORIES)
        || !policy
            .runtime_allowed_action_ids
            .iter()
            .any(|id| id == ACQUIRE_ADVISORIES)
        || policy
            .restricted_action_ids
            .iter()
            .any(|id| id == ACQUIRE_ADVISORIES)
    {
        return Err("standing Security observation requires the exact granted advisory read action and subject".into());
    }
    Ok(())
}

impl AdvisoryObservation {
    pub fn record_id(&self, ledger: meld_events::LedgerIdentity) -> Result<String, String> {
        Ok(format!(
            "security-source-observation::{}",
            content_hash(&(ledger, self))?
        ))
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_permission(&self.authority, &self.subject.subject)?;
        self.policy.validate()?;
        if self.binding_id.is_empty()
            || self.generation_id.is_empty()
            || self.incarnation_id.is_empty()
        {
            return Err("Security source observation has no bound owner incarnation".into());
        }
        match (&self.advisory, &self.failure) {
            (Some(advisory), None) => {
                advisory.validate()?;
                if advisory.source_id != self.policy.required_advisory_source_id
                    || advisory.covered_ecosystem != self.policy.ecosystem
                {
                    return Err(
                        "observed advisory source differs from the installed Security policy"
                            .into(),
                    );
                }
            }
            (None, Some(failure)) if !failure.is_empty() => {}
            _ => {
                return Err(
                    "source observation requires one exact product or unavailable disposition"
                        .into(),
                )
            }
        }
        Ok(())
    }

    pub fn from_record(
        record: &EventRecord,
        ledger: meld_events::LedgerIdentity,
    ) -> Result<Self, String> {
        let value: Self = serde_json::from_value(record.data.clone()).map_err(|e| e.to_string())?;
        value.validate()?;
        if record.record_id.as_deref() != Some(value.record_id(ledger)?.as_str()) {
            return Err("Security observation identity differs from its retained body".into());
        }
        Ok(value)
    }

    fn envelope(&self, ledger: meld_events::LedgerIdentity) -> Result<EventEnvelope, String> {
        self.validate()?;
        Ok(EventEnvelope::with_now_domain(
            "dependency-security-observation",
            OWNER,
            &self.subject.subject.object_id,
            EVENT,
            None,
            serde_json::to_value(self).map_err(|e| e.to_string())?,
        )
        .with_record_id(self.record_id(ledger)?))
    }
}

fn latest(
    capability: &SecurityCapability,
    events: &EventReplayCapability,
) -> Result<Option<(EventRecord, AdvisoryObservation)>, String> {
    let ledger_id = events.ledger_identity();
    let mut cursor = LedgerCursor {
        ledger_id,
        after_seq: 0,
    };
    let mut tip = None;
    let mut latest = None;
    loop {
        let page = events
            .replay(ReplayRequest {
                cursor,
                limit: 1024,
            })
            .map_err(|e| e.to_string())?;
        if page.coverage.retained_from > 1 {
            return Err("Security source recovery requires retained owner history".into());
        }
        let frozen = *tip.get_or_insert(page.coverage.tip_seq);
        for record in page.records.into_iter().filter(|record| {
            record.seq <= frozen && record.domain_id == OWNER && record.event_type == EVENT
        }) {
            let observed = AdvisoryObservation::from_record(&record, ledger_id)?;
            if observed.subject == capability.subject && observed.policy == capability.policy {
                latest = Some((record, observed));
            }
        }
        if page.next_cursor.after_seq >= frozen {
            break;
        }
        if page.next_cursor.after_seq <= cursor.after_seq {
            return Err("Security source replay made no progress".into());
        }
        cursor = page.next_cursor;
    }
    Ok(latest)
}

/// Finish an accepted source publication before another acquisition can supersede it.
pub(crate) fn resume_pending(
    capability: &SecurityCapability,
    events: &EventAppendCapability,
) -> Result<bool, String> {
    let replay = events.replay_capability();
    let Some((record, observed)) = latest(capability, &replay)? else {
        return Ok(false);
    };
    let source_id = observed.record_id(events.ledger_identity())?;
    let mut envelopes = Vec::new();
    if let Some(product) = &observed.advisory {
        let operation = super::publication::observed_advisory_publication(
            &observed.subject,
            &source_id,
            product,
        )
        .map_err(|e| e.to_string())?;
        let mut envelope =
            meld_world_model::world_state::graph::events::owner_publication_envelope(
                "dependency-security-observation",
                &operation,
            )
            .map_err(|e| e.to_string())?;
        envelope.event_type = super::publication::EVENT.into();
        envelopes.push(
            envelope.with_source_records(vec![meld_events::EventRecordRef {
                ledger_id: events.ledger_identity(),
                seq: record.seq,
            }]),
        );
    }
    envelopes.push(super::condition::publication(capability, &replay).map_err(|e| e.to_string())?);
    let missing = envelopes
        .iter()
        .map(|envelope| {
            replay
                .prove_existing(envelope)
                .map(|proof| proof.is_none())
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|missing| missing);
    events
        .append_durable_batch(envelopes.clone(), AppendMode::Idempotent)
        .map_err(|e| e.to_string())?;
    for envelope in envelopes {
        if replay
            .prove_existing(&envelope)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err("Security observation publication is not durably proven".into());
        }
    }
    Ok(missing)
}

pub(crate) fn poll(
    capability: &SecurityCapability,
    authority: &AuthorityPolicyBinding,
    binding_id: &str,
    generation_id: &str,
    incarnation_id: &str,
    events: &EventAppendCapability,
) -> Result<(bool, Option<String>), String> {
    validate_permission(authority, &capability.subject.subject)?;
    let resumed = resume_pending(capability, events)?;
    let current = super::condition::current(capability, &events.replay_capability())
        .map_err(|e| e.to_string())?;
    let captured = acquire(capability);
    let failure = captured.as_ref().err().cloned();
    let source_id = match &captured {
        Ok(advisory) => advisory.snapshot_id.clone(),
        Err(error) => content_hash(error)?,
    };
    let kind = if captured.is_ok() {
        ADVISORIES
    } else {
        UNAVAILABLE
    };
    if current
        .as_ref()
        .and_then(|condition| condition.products.get(kind))
        .is_some_and(|position| position.product_id == source_id)
    {
        return Ok((resumed, failure));
    }
    let prior = current
        .as_ref()
        .and_then(|condition| {
            condition
                .products
                .get(ADVISORIES)
                .or_else(|| condition.products.get(UNAVAILABLE))
        })
        .map(|position| position.receipt_id.clone());
    let observation = AdvisoryObservation {
        subject: capability.subject.clone(),
        policy: capability.policy.clone(),
        authority: authority.clone(),
        binding_id: binding_id.into(),
        predecessor_source_id: prior,
        generation_id: generation_id.into(),
        incarnation_id: incarnation_id.into(),
        observed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs(),
        advisory: captured.ok(),
        failure: failure.clone(),
    };
    events
        .append_durable_proven(
            observation.envelope(events.ledger_identity())?,
            AppendMode::Idempotent,
        )
        .map_err(|e| e.to_string())?;
    resume_pending(capability, events)?;
    Ok((true, failure))
}

pub(crate) fn acquire(
    capability: &SecurityCapability,
) -> Result<AdvisoryKnowledgeSnapshotV1, String> {
    let bytes = super::inventory::cargo::read_bounded(
        capability
            .advisories
            .as_deref()
            .ok_or("advisory source absent")?,
        capability.limits.maximum_bytes,
    )?;
    let source: super::advisory::AdvisorySourceDocumentV1 =
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let advisory = source.admit()?;
    if advisory.source_id != capability.policy.required_advisory_source_id
        || advisory.covered_ecosystem != capability.policy.ecosystem
    {
        return Err("advisory source does not match the exact installed policy".into());
    }
    Ok(advisory)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};

    pub(crate) fn fixture(
        path: &std::path::Path,
    ) -> (
        SecurityCapability,
        AuthorityPolicyBinding,
        AdvisoryKnowledgeSnapshotV1,
    ) {
        let subject = DependencySecuritySubjectV1 {
            subject: DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new(OWNER, "manifest_source", "manifest").unwrap(),
                lockfile_ref: DomainObjectRef::new(OWNER, "lockfile_source", "lock").unwrap(),
                include_transitive: true,
            },
        };
        let policy = meld_lang::AuthorityPolicy {
            policy_id: "source-read".into(),
            principal_id: "owner".into(),
            subject: subject.subject.clone(),
            principal_granted_action_ids: vec![ACQUIRE_ADVISORIES.into()],
            runtime_allowed_action_ids: vec![ACQUIRE_ADVISORIES.into()],
            restricted_action_ids: vec![],
        };
        let authority =
            AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap()).unwrap();
        let capability = SecurityCapability {
            publication_gate: Default::default(),
            id: ACQUIRE_ADVISORIES.into(),
            subject,
            policy: serde_json::from_str(include_str!(
                "../../theory/dependency_security/policy.cargo_fixture.json"
            ))
            .unwrap(),
            workspace: None,
            cargo: None,
            advisories: Some(path.into()),
            limits: Default::default(),
        };
        let product = AdvisoryKnowledgeSnapshotV1::canonical(
            "fixture".into(),
            "source-a".into(),
            PackageEcosystem::Cargo,
            vec![],
            vec![],
            vec![],
            AdvisoryCompleteness::CompleteForDeclaredCoverage,
            10,
        )
        .unwrap();
        (capability, authority, product)
    }

    #[test]
    fn interrupted_source_publication_recovers_and_unavailability_cannot_borrow_prior_knowledge() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source.json");
        let (capability, authority, product) = fixture(&path);
        let source = serde_json::to_vec(&super::super::advisory::AdvisorySourceDocumentV1::from(
            product.clone(),
        ))
        .unwrap();
        std::fs::write(&path, &source).unwrap();
        let observation = AdvisoryObservation {
            subject: capability.subject.clone(),
            policy: capability.policy.clone(),
            authority: authority.clone(),
            binding_id: "source-binding".into(),
            predecessor_source_id: None,
            generation_id: "generation".into(),
            incarnation_id: "incarnation".into(),
            observed_at: 10,
            advisory: Some(product.clone()),
            failure: None,
        };
        let first_id;
        {
            let events = EventAuthority::open(
                sled::open(root.path().join("events")).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            first_id = observation.record_id(events.ledger_identity()).unwrap();
            events
                .append_capability()
                .append_durable_proven(
                    observation.envelope(events.ledger_identity()).unwrap(),
                    AppendMode::Idempotent,
                )
                .unwrap();
            assert!(!super::super::condition::is_published(
                &capability,
                &events.replay_capability()
            )
            .unwrap());
        }
        std::fs::write(&path, "not a usable advisory document").unwrap();
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        assert!(resume_pending(&capability, &events.append_capability()).unwrap());
        let recovered = super::super::condition::current(&capability, &events.replay_capability())
            .unwrap()
            .unwrap();
        assert_eq!(
            recovered.products[ADVISORIES].product_id,
            product.snapshot_id
        );
        assert_eq!(recovered.products[ADVISORIES].receipt_id, first_id);
        let unavailable = poll(
            &capability,
            &authority,
            "source-binding",
            "generation",
            "incarnation",
            &events.append_capability(),
        )
        .unwrap();
        assert!(unavailable.0 && unavailable.1.is_some());
        let current = super::super::condition::current(&capability, &events.replay_capability())
            .unwrap()
            .unwrap();
        assert!(!current.products.contains_key(ADVISORIES));
        assert!(current.products.contains_key(UNAVAILABLE));
        assert!(!current.coverage_current && !current.verified_clean);
        let before = events.watermark_capability().snapshot().unwrap().tip_seq;
        assert!(
            !poll(
                &capability,
                &authority,
                "source-binding",
                "generation",
                "incarnation",
                &events.append_capability()
            )
            .unwrap()
            .0
        );
        assert_eq!(
            events.watermark_capability().snapshot().unwrap().tip_seq,
            before
        );
        std::fs::write(&path, source).unwrap();
        assert!(
            poll(
                &capability,
                &authority,
                "source-binding",
                "generation",
                "incarnation",
                &events.append_capability()
            )
            .unwrap()
            .0
        );
        let restored = super::super::condition::current(&capability, &events.replay_capability())
            .unwrap()
            .unwrap();
        assert_eq!(
            restored.products[ADVISORIES].product_id,
            product.snapshot_id
        );
        assert_ne!(
            restored.products[ADVISORIES].receipt_id, first_id,
            "restoration is new evidence after unavailability"
        );
        let records = events.replay_capability().newest_page(128).unwrap().records;
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == EVENT)
                .count(),
            3
        );
        assert!(!records
            .iter()
            .any(|record| record.event_type == super::super::publication::RECEIPT_EVENT));
        let mut tampered = records
            .iter()
            .find(|record| record.event_type == EVENT)
            .unwrap()
            .clone();
        tampered.data["binding_id"] = "foreign-binding".into();
        assert!(AdvisoryObservation::from_record(&tampered, events.ledger_identity()).is_err());
    }

    #[test]
    fn standing_source_read_requires_exact_subject_and_unrestricted_principal_grant() {
        let root = tempfile::tempdir().unwrap();
        let (capability, authority, _) = fixture(&root.path().join("not-opened"));
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        for variant in 0..3 {
            let mut policy = authority.policy.clone();
            match variant {
                0 => policy.principal_granted_action_ids.clear(),
                1 => policy.restricted_action_ids.push(ACQUIRE_ADVISORIES.into()),
                _ => policy.subject.object_id = "foreign-subject".into(),
            }
            let denied =
                AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap())
                    .unwrap();
            let result = poll(
                &capability,
                &denied,
                "binding",
                "generation",
                "incarnation",
                &events.append_capability(),
            );
            assert!(result.unwrap_err().contains("exact granted advisory read"));
        }
        assert_eq!(events.watermark_capability().snapshot().unwrap().tip_seq, 0);
    }
}
