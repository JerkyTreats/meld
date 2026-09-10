//! Explicit semantic time evidence for the installed Security currency policy.

use std::collections::BTreeMap;

use meld_events::{AppendMode, EventAppendCapability, EventEnvelope, EventRecord, LedgerIdentity};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    capability::{SecurityCapability, ADVISORIES, INVENTORY},
    condition::{CurrentSecurityCondition, ProductPosition},
    contracts::*,
    policy::DependencySecurityPolicyV2,
    publication::OWNER,
};

pub const EVENT: &str = "dependency_security.currency_observed.v1";
pub(super) const PRODUCT: &str = "dependency_security_currency";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CurrencyObservation {
    pub subject: DependencySecuritySubjectV1,
    pub policy: DependencySecurityPolicyV2,
    pub source_positions: BTreeMap<String, ProductPosition>,
    pub previous_reference_time: u64,
    pub reference_time: u64,
    pub binding_id: String,
    pub generation_id: String,
    pub incarnation_id: String,
}

fn sources(products: &BTreeMap<String, ProductPosition>) -> BTreeMap<String, ProductPosition> {
    products
        .iter()
        .filter(|(kind, _)| matches!(kind.as_str(), INVENTORY | ADVISORIES))
        .map(|(kind, position)| (kind.clone(), position.clone()))
        .collect()
}

fn crosses_boundary(
    policy: &DependencySecurityPolicyV2,
    bodies: &BTreeMap<String, Value>,
    previous: u64,
    at: u64,
) -> Result<bool, String> {
    let inventory: Option<DependencyInventorySnapshotV1> = bodies
        .get(INVENTORY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let advisory: Option<AdvisoryKnowledgeSnapshotV1> = bodies
        .get(ADVISORIES)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let inputs = [
        inventory.map(|product| {
            (
                product.observed_at,
                policy.currency.maximum_inventory_age_seconds,
            )
        }),
        advisory.map(|product| {
            (
                product.acquired_at,
                policy.currency.maximum_source_age_seconds,
            )
        }),
    ];
    Ok(inputs.into_iter().flatten().any(|(start, age)| {
        [
            Some(start),
            start
                .checked_add(age)
                .and_then(|last_current| last_current.checked_add(1)),
        ]
        .into_iter()
        .flatten()
        .any(|boundary| previous < boundary && boundary <= at)
    }))
}

impl CurrencyObservation {
    pub fn record_id(&self, ledger: LedgerIdentity) -> Result<String, String> {
        Ok(format!(
            "security-currency::{}",
            content_hash(&(ledger, self))?
        ))
    }

    pub fn from_record(record: &EventRecord, ledger: LedgerIdentity) -> Result<Self, String> {
        let value: Self = serde_json::from_value(record.data.clone()).map_err(|error| {
            format!("Security currency observation is incompatible with policy v2: {error}")
        })?;
        value.policy.validate()?;
        value
            .subject
            .subject
            .validate()
            .map_err(|e| e.to_string())?;
        if record.event_type != EVENT
            || record.record_id.as_deref() != Some(value.record_id(ledger)?.as_str())
            || record.provenance.source_records != value.source_records(ledger)
            || value
                .source_positions
                .values()
                .any(|position| position.receipt_seq >= record.seq)
            || value.binding_id.is_empty()
            || value.generation_id.is_empty()
            || value.incarnation_id.is_empty()
            || value.reference_time <= value.previous_reference_time
        {
            return Err(
                "Security currency observation has invalid identity or owner lineage".into(),
            );
        }
        Ok(value)
    }

    /// A replayed clock observation must name the exact source basis at its ledger position.
    pub fn validate_basis(
        &self,
        products: &BTreeMap<String, ProductPosition>,
        bodies: &BTreeMap<String, Value>,
        previous: u64,
    ) -> Result<(), String> {
        if self.source_positions != sources(products)
            || self.previous_reference_time != previous
            || !crosses_boundary(&self.policy, bodies, previous, self.reference_time)?
        {
            return Err("Security currency observation does not cross a policy boundary over its exact source basis".into());
        }
        Ok(())
    }

    fn source_records(&self, ledger: LedgerIdentity) -> Vec<meld_events::EventRecordRef> {
        let mut records: Vec<_> = self
            .source_positions
            .values()
            .map(|position| meld_events::EventRecordRef {
                ledger_id: ledger,
                seq: position.receipt_seq,
            })
            .collect();
        records.sort_by_key(|record| record.seq);
        records.dedup();
        records
    }

    fn envelope(&self, ledger: LedgerIdentity) -> Result<EventEnvelope, String> {
        Ok(EventEnvelope::with_now_domain(
            "dependency-security-currency",
            OWNER,
            &self.subject.subject.object_id,
            EVENT,
            None,
            serde_json::to_value(self).map_err(|e| e.to_string())?,
        )
        .with_record_id(self.record_id(ledger)?)
        .with_source_records(self.source_records(ledger)))
    }
}

pub(crate) fn resume_pending(
    capability: &SecurityCapability,
    events: &EventAppendCapability,
) -> Result<bool, String> {
    let replay = events.replay_capability();
    let Some(current) =
        super::condition::current(capability, &replay).map_err(|e| e.to_string())?
    else {
        return Ok(false);
    };
    if !current.products.contains_key(PRODUCT) {
        return Ok(false);
    }
    let envelope = super::condition::publication(capability, &replay).map_err(|e| e.to_string())?;
    if replay
        .prove_existing(&envelope)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Ok(false);
    }
    events
        .append_durable_proven(envelope, AppendMode::Idempotent)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// Called under the prepared owner's publication gate and live incarnation fence.
pub(crate) fn observe(
    capability: &SecurityCapability,
    events: &EventAppendCapability,
    binding_id: &str,
    generation_id: &str,
    incarnation_id: &str,
    reference_time: u64,
) -> Result<bool, String> {
    if binding_id.is_empty() || generation_id.is_empty() || incarnation_id.is_empty() {
        return Err("Security currency observation requires a bound live owner incarnation".into());
    }
    let resumed = resume_pending(capability, events)?;
    let (current, bodies) =
        super::condition::current_state(capability, &events.replay_capability())
            .map_err(|e| e.to_string())?;
    let Some(CurrentSecurityCondition {
        products,
        reference_time: previous,
        ..
    }) = current
    else {
        return Ok(resumed);
    };
    if !crosses_boundary(&capability.policy, &bodies, previous, reference_time)? {
        return Ok(resumed);
    }
    let observed = CurrencyObservation {
        subject: capability.subject.clone(),
        policy: capability.policy.clone(),
        source_positions: sources(&products),
        previous_reference_time: previous,
        reference_time,
        binding_id: binding_id.into(),
        generation_id: generation_id.into(),
        incarnation_id: incarnation_id.into(),
    };
    observed.validate_basis(&products, &bodies, previous)?;
    events
        .append_durable_proven(
            observed.envelope(events.ledger_identity())?,
            AppendMode::Idempotent,
        )
        .map_err(|e| e.to_string())?;
    resume_pending(capability, events)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::{capability::OBSERVE_INVENTORY, observation::SourceObservation};
    use super::*;
    use meld_events::{EventAuthority, EventAuthorityOpenOptions};

    fn fixture(events: &EventAuthority) -> SecurityCapability {
        let (mut capability, authority, advisory) =
            super::super::observation::tests::fixture(std::path::Path::new("/unused-advisory"));
        capability.policy.currency.maximum_source_age_seconds = 10;
        capability.policy.currency.maximum_inventory_age_seconds = 20;
        let inventory = DependencyInventorySnapshotV1::canonical(
            capability.subject.clone(),
            "workspace".into(),
            "manifest".into(),
            "lock".into(),
            vec![],
            InventoryCompleteness::CompleteTransitive,
            10,
        )
        .unwrap();
        let mut policy = authority.policy;
        policy
            .principal_granted_action_ids
            .push(OBSERVE_INVENTORY.into());
        policy
            .runtime_allowed_action_ids
            .push(OBSERVE_INVENTORY.into());
        let authority =
            meld_lang::AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap())
                .unwrap();
        for is_inventory in [true, false] {
            let source = SourceObservation {
                subject: capability.subject.clone(),
                policy: capability.policy.clone(),
                authority: authority.clone(),
                binding_id: "source".into(),
                predecessor_source_id: None,
                generation_id: "generation".into(),
                incarnation_id: "incarnation".into(),
                observed_at: 10,
                advisory: (!is_inventory).then(|| advisory.clone()),
                inventory: is_inventory.then(|| inventory.clone()),
                source_action: is_inventory.then(|| OBSERVE_INVENTORY.into()),
                execution_causes: vec![],
                failure: None,
            };
            let envelope = EventEnvelope::with_now_domain(
                "currency-test",
                OWNER,
                "repo",
                if is_inventory {
                    super::super::observation::INVENTORY_EVENT
                } else {
                    super::super::observation::EVENT
                },
                None,
                serde_json::to_value(&source).unwrap(),
            )
            .with_record_id(source.record_id(events.ledger_identity()).unwrap());
            events
                .append_capability()
                .append_durable_proven(envelope, AppendMode::Idempotent)
                .unwrap();
        }
        super::super::observation::resume_pending(&capability, &events.append_capability())
            .unwrap();
        capability
    }

    #[test]
    fn expiry_is_durable_boundary_evidence_and_recovery_does_not_read_sources() {
        let root = tempfile::tempdir().unwrap();
        let capability;
        let observed;
        {
            let events = EventAuthority::open(
                sled::open(root.path()).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            capability = fixture(&events);
            let (current, bodies) =
                super::super::condition::current_state(&capability, &events.replay_capability())
                    .unwrap();
            let current = current.unwrap();
            assert!(current.coverage_current);
            assert_eq!(
                current.current_posture,
                DependencySecurityPosture::CleanWithinCoverage
            );
            let before = events.watermark_capability().snapshot().unwrap().tip_seq;
            assert!(!observe(
                &capability,
                &events.append_capability(),
                "binding",
                "generation",
                "incarnation",
                20
            )
            .unwrap());
            assert_eq!(
                events.watermark_capability().snapshot().unwrap().tip_seq,
                before,
                "maximum age is inclusive"
            );
            observed = CurrencyObservation {
                subject: capability.subject.clone(),
                policy: capability.policy.clone(),
                source_positions: sources(&current.products),
                previous_reference_time: current.reference_time,
                reference_time: 21,
                binding_id: "binding".into(),
                generation_id: "generation".into(),
                incarnation_id: "incarnation".into(),
            };
            observed
                .validate_basis(&current.products, &bodies, current.reference_time)
                .unwrap();
            let mut foreign = observed.clone();
            foreign
                .source_positions
                .get_mut(INVENTORY)
                .unwrap()
                .receipt_seq += 1;
            assert!(foreign
                .validate_basis(&current.products, &bodies, current.reference_time)
                .is_err());
            foreign = observed.clone();
            foreign.reference_time = 20;
            assert!(foreign
                .validate_basis(&current.products, &bodies, current.reference_time)
                .is_err());
            events
                .append_capability()
                .append_durable_proven(
                    observed.envelope(events.ledger_identity()).unwrap(),
                    AppendMode::Idempotent,
                )
                .unwrap();
            assert!(!super::super::condition::is_published(
                &capability,
                &events.replay_capability()
            )
            .unwrap());
        }
        let events = EventAuthority::open(
            sled::open(root.path()).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        assert!(super::super::observation::resume_pending(
            &capability,
            &events.append_capability()
        )
        .unwrap());
        let current = super::super::condition::current(&capability, &events.replay_capability())
            .unwrap()
            .unwrap();
        assert!(!current.coverage_current && !current.verified_clean);
        assert_eq!(current.current_posture, DependencySecurityPosture::Stale);
        assert_eq!(current.reference_time, 21);
        assert_eq!(
            current.products[PRODUCT].receipt_id,
            observed.record_id(events.ledger_identity()).unwrap()
        );
        let before = events.watermark_capability().snapshot().unwrap().tip_seq;
        for at in [19, 21, 22, 30] {
            assert!(!observe(
                &capability,
                &events.append_capability(),
                "binding",
                "successor-generation",
                "successor-incarnation",
                at
            )
            .unwrap());
        }
        assert_eq!(
            events.watermark_capability().snapshot().unwrap().tip_seq,
            before,
            "restart, clock rollback and unchanged polling cannot repeat a boundary"
        );
        assert!(observe(
            &capability,
            &events.append_capability(),
            "binding",
            "successor-generation",
            "successor-incarnation",
            31
        )
        .unwrap());
        let records = events.replay_capability().newest_page(128).unwrap().records;
        let mut tampered = records
            .iter()
            .find(|record| record.event_type == EVENT)
            .unwrap()
            .clone();
        tampered.envelope.provenance.source_records.clear();
        assert!(CurrencyObservation::from_record(&tampered, events.ledger_identity()).is_err());
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == EVENT)
                .count(),
            2
        );
        assert!(!records
            .iter()
            .any(|record| record.event_type == super::super::publication::RECEIPT_EVENT));
    }

    #[test]
    fn future_evidence_and_overflowing_expiry_have_explicit_boundary_semantics() {
        let events = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let capability = fixture(&events);
        let (_, bodies) =
            super::super::condition::current_state(&capability, &events.replay_capability())
                .unwrap();
        assert!(!crosses_boundary(&capability.policy, &bodies, 0, 9).unwrap());
        assert!(crosses_boundary(&capability.policy, &bodies, 9, 10).unwrap());
        let mut unbounded = capability.policy;
        unbounded.currency.maximum_inventory_age_seconds = u64::MAX;
        unbounded.currency.maximum_source_age_seconds = u64::MAX;
        assert!(!crosses_boundary(&unbounded, &bodies, 10, u64::MAX).unwrap());
    }
}
