//! Durable, standing Security source observations. These are source evidence, never Task returns.

use super::{capability::*, contracts::*, policy::DependencySecurityPolicyV2, publication::OWNER};
use meld_events::{
    AppendMode, EventAppendCapability, EventEnvelope, EventRecord, EventReplayCapability,
    LedgerCursor, ReplayRequest,
};
use meld_lang::AuthorityPolicyBinding;
use serde::{Deserialize, Serialize};

pub const EVENT: &str = "dependency_security.advisory_observed.v1";
pub const INVENTORY_EVENT: &str = "dependency_security.inventory_observed.v1";
pub(super) const INVENTORY_UNAVAILABLE: &str = "dependency_security_inventory_unavailable";
pub(super) const UNAVAILABLE: &str = "dependency_security_advisory_unavailable";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SourceObservation {
    pub subject: DependencySecuritySubjectV1,
    pub policy: DependencySecurityPolicyV2,
    pub authority: AuthorityPolicyBinding,
    pub binding_id: String,
    pub predecessor_source_id: Option<String>,
    pub generation_id: String,
    pub incarnation_id: String,
    pub observed_at: u64,
    pub advisory: Option<AdvisoryKnowledgeSnapshotV1>,
    pub failure: Option<String>,
    // Omitted fields preserve the exact identity of retained advisory v1 records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<DependencyInventorySnapshotV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub execution_causes: Vec<super::returns::ExecutionObservationCause>,
}

pub(super) fn validate_permission(
    authority: &AuthorityPolicyBinding,
    subject: &meld_events::DomainObjectRef,
    action: &str,
) -> Result<(), String> {
    authority.validate().map_err(|e| e.to_string())?;
    let policy = &authority.policy;
    if !matches!(action, ACQUIRE_ADVISORIES | OBSERVE_INVENTORY)
        || &policy.subject != subject
        || !policy
            .principal_granted_action_ids
            .iter()
            .any(|id| id == action)
        || !policy
            .runtime_allowed_action_ids
            .iter()
            .any(|id| id == action)
        || policy.restricted_action_ids.iter().any(|id| id == action)
    {
        return Err("standing Security observation requires the exact granted source read action and subject".into());
    }
    Ok(())
}

impl SourceObservation {
    pub fn action(&self) -> &str {
        self.source_action.as_deref().unwrap_or(ACQUIRE_ADVISORIES)
    }

    pub fn kind(&self) -> &str {
        if self.action() == OBSERVE_INVENTORY {
            INVENTORY
        } else {
            ADVISORIES
        }
    }

    pub fn unavailable_kind(&self) -> &str {
        if self.action() == OBSERVE_INVENTORY {
            INVENTORY_UNAVAILABLE
        } else {
            UNAVAILABLE
        }
    }

    fn event_type(&self) -> &str {
        if self.action() == OBSERVE_INVENTORY {
            INVENTORY_EVENT
        } else {
            EVENT
        }
    }

    pub fn product(&self) -> Result<Option<(String, serde_json::Value)>, String> {
        if let Some(product) = &self.inventory {
            Ok(Some((
                product.snapshot_id.clone(),
                serde_json::to_value(product).map_err(|e| e.to_string())?,
            )))
        } else if let Some(product) = &self.advisory {
            Ok(Some((
                product.snapshot_id.clone(),
                serde_json::to_value(product).map_err(|e| e.to_string())?,
            )))
        } else {
            Ok(None)
        }
    }

    pub fn record_id(&self, ledger: meld_events::LedgerIdentity) -> Result<String, String> {
        Ok(format!(
            "security-source-observation::{}",
            content_hash(&(ledger, self))?
        ))
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_permission(&self.authority, &self.subject.subject, self.action())?;
        self.policy.validate()?;
        if self.execution_causes.len() > super::returns::MAX_CAUSES {
            return Err("Security observation exceeds its bounded producer cause set".into());
        }
        if !self.execution_causes.is_empty() && self.action() != OBSERVE_INVENTORY {
            return Err(
                "Execution returns can request only the bound inventory observation".into(),
            );
        }
        if self.execution_causes.windows(2).any(|pair| {
            (pair[0].outcome.seq, pair[0].materialization.seq)
                >= (pair[1].outcome.seq, pair[1].materialization.seq)
        }) {
            return Err("Security observation causes must be unique and ordered".into());
        }
        if self.binding_id.is_empty()
            || self.generation_id.is_empty()
            || self.incarnation_id.is_empty()
        {
            return Err("Security source observation has no bound owner incarnation".into());
        }
        match (&self.advisory, &self.inventory, &self.failure) {
            (None, Some(inventory), None) if self.action() == OBSERVE_INVENTORY => {
                inventory.validate()?;
                if inventory.subject != self.subject {
                    return Err("observed inventory names a foreign subject".into());
                }
            }
            (Some(advisory), None, None) if self.action() == ACQUIRE_ADVISORIES => {
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
            (None, None, Some(failure)) if !failure.is_empty() => {}
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
        let value: Self = serde_json::from_value(record.data.clone()).map_err(|error| {
            format!("Security source observation is incompatible with policy v2: {error}")
        })?;
        value.validate()?;
        for cause in &value.execution_causes {
            cause.validate(ledger, record.seq)?;
        }
        if record.provenance.source_records != super::returns::positions(&value.execution_causes) {
            return Err(
                "Security observation provenance differs from its accepted Execution causes".into(),
            );
        }
        if record.event_type != value.event_type()
            || record.record_id.as_deref() != Some(value.record_id(ledger)?.as_str())
        {
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
            self.event_type(),
            None,
            serde_json::to_value(self).map_err(|e| e.to_string())?,
        )
        .with_record_id(self.record_id(ledger)?)
        .with_source_records(super::returns::positions(&self.execution_causes)))
    }
}

fn latest(
    capability: &SecurityCapability,
    events: &EventReplayCapability,
) -> Result<Vec<(EventRecord, SourceObservation)>, String> {
    let ledger_id = events.ledger_identity();
    let mut cursor = LedgerCursor {
        ledger_id,
        after_seq: 0,
    };
    let mut tip = None;
    let mut latest = std::collections::BTreeMap::new();
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
            record.seq <= frozen
                && record.domain_id == OWNER
                && matches!(record.event_type.as_str(), EVENT | INVENTORY_EVENT)
        }) {
            let observed = SourceObservation::from_record(&record, ledger_id)?;
            if observed.subject == capability.subject && observed.policy == capability.policy {
                super::returns::validate_retained(&observed, events)?;
                latest.insert(observed.kind().to_owned(), (record, observed));
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
    Ok(latest.into_values().collect())
}

/// Finish an accepted source publication before another acquisition can supersede it.
pub(crate) fn resume_pending(
    capability: &SecurityCapability,
    events: &EventAppendCapability,
) -> Result<bool, String> {
    let currency_resumed = super::currency::resume_pending(capability, events)?;
    let replay = events.replay_capability();
    let observations = latest(capability, &replay)?;
    if observations.is_empty() {
        return Ok(currency_resumed);
    }
    let mut envelopes = Vec::new();
    for (record, observed) in observations {
        let source_id = observed.record_id(events.ledger_identity())?;
        if let Some((_, product)) = observed.product()? {
            let operation = super::publication::observed_source_publication(
                &observed.subject,
                &source_id,
                observed.kind(),
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
    Ok(missing || currency_resumed)
}

pub(crate) fn poll(
    capability: &SecurityCapability,
    authority: &AuthorityPolicyBinding,
    binding_id: &str,
    generation_id: &str,
    incarnation_id: &str,
    events: &EventAppendCapability,
) -> Result<(bool, Option<String>), String> {
    validate_permission(authority, &capability.subject.subject, ACQUIRE_ADVISORIES)?;
    let resumed = resume_pending(capability, events)?;
    let current = super::condition::current(capability, &events.replay_capability())
        .map_err(|e| e.to_string())?;
    let captured = acquire(capability).map(|advisory| CapturedSource::Advisory(Box::new(advisory)));
    record_capture(
        capability,
        authority,
        binding_id,
        generation_id,
        incarnation_id,
        events,
        current,
        captured,
        resumed,
        vec![],
    )
}

pub(crate) async fn poll_inventory(
    capability: &SecurityCapability,
    authority: &AuthorityPolicyBinding,
    binding_id: &str,
    generation_id: &str,
    incarnation_id: &str,
    events: &EventAppendCapability,
) -> Result<(bool, Option<String>), String> {
    validate_permission(authority, &capability.subject.subject, OBSERVE_INVENTORY)?;
    let resumed = resume_pending(capability, events)?;
    let (current, bodies) =
        super::condition::current_state(capability, &events.replay_capability())
            .map_err(|e| e.to_string())?;
    let execution_causes = super::returns::pending(capability, &events.replay_capability())?;
    let previous: Option<DependencyInventorySnapshotV1> = bodies
        .get(INVENTORY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    // Stable source content retains its acquisition identity. Polling is not an age refresh.
    let prior_time = previous.as_ref().map_or(0, |product| product.observed_at);
    let captured = super::inventory::cargo::observe(
        capability
            .workspace
            .as_deref()
            .ok_or("inventory workspace is absent")?,
        capability
            .cargo
            .as_deref()
            .ok_or("inventory executable is absent")?,
        capability.subject.clone(),
        prior_time,
        &capability.limits,
    )
    .await
    .and_then(|product| {
        if previous.as_ref() == Some(&product) {
            return Ok(product);
        }
        DependencyInventorySnapshotV1::canonical(
            product.subject,
            product.workspace_revision,
            product.manifest_content_hash,
            product.lockfile_content_hash,
            product.components,
            product.completeness,
            now()?,
        )
    })
    .map(|product| CapturedSource::Inventory(Box::new(product)));
    record_capture(
        capability,
        authority,
        binding_id,
        generation_id,
        incarnation_id,
        events,
        current,
        captured,
        resumed,
        execution_causes,
    )
}

enum CapturedSource {
    Advisory(Box<AdvisoryKnowledgeSnapshotV1>),
    Inventory(Box<DependencyInventorySnapshotV1>),
}

pub(crate) fn now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
fn record_capture(
    capability: &SecurityCapability,
    authority: &AuthorityPolicyBinding,
    binding_id: &str,
    generation_id: &str,
    incarnation_id: &str,
    events: &EventAppendCapability,
    current: Option<super::condition::CurrentSecurityCondition>,
    captured: Result<CapturedSource, String>,
    resumed: bool,
    execution_causes: Vec<super::returns::ExecutionObservationCause>,
) -> Result<(bool, Option<String>), String> {
    let failure = captured.as_ref().err().cloned();
    let source_id = match &captured {
        Ok(CapturedSource::Advisory(product)) => product.snapshot_id.clone(),
        Ok(CapturedSource::Inventory(product)) => product.snapshot_id.clone(),
        Err(error) => content_hash(error)?,
    };
    let inventory_source = capability.id == OBSERVE_INVENTORY;
    let product_kind = if inventory_source {
        INVENTORY
    } else {
        ADVISORIES
    };
    let unavailable = if inventory_source {
        INVENTORY_UNAVAILABLE
    } else {
        UNAVAILABLE
    };
    let kind = if captured.is_ok() {
        product_kind
    } else {
        unavailable
    };
    if execution_causes.is_empty()
        && current
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
                .get(product_kind)
                .or_else(|| condition.products.get(unavailable))
        })
        .map(|position| position.receipt_id.clone());
    let (advisory, inventory) = match captured.ok() {
        Some(CapturedSource::Advisory(product)) => (Some(*product), None),
        Some(CapturedSource::Inventory(product)) => (None, Some(*product)),
        None => (None, None),
    };
    let observation = SourceObservation {
        subject: capability.subject.clone(),
        policy: capability.policy.clone(),
        authority: authority.clone(),
        binding_id: binding_id.into(),
        predecessor_source_id: prior,
        generation_id: generation_id.into(),
        incarnation_id: incarnation_id.into(),
        observed_at: now()?,
        advisory,
        failure: failure.clone(),
        source_action: inventory_source.then(|| OBSERVE_INVENTORY.into()),
        inventory,
        execution_causes,
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
            assignment_scope_id: "fixture-assignment".into(),
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
                "../../../../theory/dependency_security/policy.cargo_fixture.json"
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
        let observation = SourceObservation {
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
            source_action: None,
            inventory: None,
            execution_causes: vec![],
        };
        #[derive(Serialize)]
        struct RetainedAdvisoryV1<'a> {
            subject: &'a DependencySecuritySubjectV1,
            policy: &'a DependencySecurityPolicyV2,
            authority: &'a AuthorityPolicyBinding,
            binding_id: &'a str,
            predecessor_source_id: &'a Option<String>,
            generation_id: &'a str,
            incarnation_id: &'a str,
            observed_at: u64,
            advisory: &'a Option<AdvisoryKnowledgeSnapshotV1>,
            failure: &'a Option<String>,
        }
        let legacy = RetainedAdvisoryV1 {
            subject: &observation.subject,
            policy: &observation.policy,
            authority: &observation.authority,
            binding_id: &observation.binding_id,
            predecessor_source_id: &observation.predecessor_source_id,
            generation_id: &observation.generation_id,
            incarnation_id: &observation.incarnation_id,
            observed_at: observation.observed_at,
            advisory: &observation.advisory,
            failure: &observation.failure,
        };
        let retained_body = serde_json::to_vec(&legacy).unwrap();
        assert_eq!(
            serde_json::to_vec(&observation).unwrap(),
            retained_body,
            "old receipt identity must reproduce byte for byte"
        );
        let decoded: SourceObservation = serde_json::from_slice(&retained_body).unwrap();
        assert_eq!(decoded, observation);
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
        assert!(SourceObservation::from_record(&tampered, events.ledger_identity()).is_err());
    }

    #[test]
    fn observation_causes_require_exact_provenance_and_retained_owner_products() {
        let root = tempfile::tempdir().unwrap();
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let (mut capability, authority, _) = fixture(&root.path().join("source"));
        capability.id = OBSERVE_INVENTORY.into();
        let mut policy = authority.policy;
        policy.principal_granted_action_ids = vec![OBSERVE_INVENTORY.into()];
        policy.runtime_allowed_action_ids = vec![OBSERVE_INVENTORY.into()];
        let authority =
            AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap()).unwrap();
        for index in 0..2 {
            events
                .append_capability()
                .append_durable(
                    EventEnvelope::with_now_domain(
                        "fixture",
                        "unrelated",
                        "source",
                        "unrelated.fact",
                        None,
                        serde_json::json!({"index":index}),
                    ),
                    AppendMode::Idempotent,
                )
                .unwrap();
        }
        let observed = SourceObservation {
            subject: capability.subject.clone(),
            policy: capability.policy.clone(),
            authority,
            binding_id: "inventory-binding".into(),
            predecessor_source_id: None,
            generation_id: "generation".into(),
            incarnation_id: "incarnation".into(),
            observed_at: 10,
            advisory: None,
            inventory: None,
            failure: Some("source unavailable".into()),
            source_action: Some(OBSERVE_INVENTORY.into()),
            execution_causes: vec![super::super::returns::ExecutionObservationCause {
                materialization: meld_events::EventRecordRef {
                    ledger_id: events.ledger_identity(),
                    seq: 1,
                },
                outcome: meld_events::EventRecordRef {
                    ledger_id: events.ledger_identity(),
                    seq: 2,
                },
            }],
        };
        let mut record = EventRecord {
            seq: 3,
            envelope: observed.envelope(events.ledger_identity()).unwrap(),
        };
        assert!(SourceObservation::from_record(&record, events.ledger_identity()).is_ok());
        assert!(
            super::super::returns::validate_retained(&observed, &events.replay_capability())
                .is_err(),
            "syntactically valid positions cannot borrow unrelated owner records"
        );
        record.envelope.provenance.source_records.clear();
        assert!(SourceObservation::from_record(&record, events.ledger_identity()).is_err());
        let mut unordered = observed.clone();
        unordered.execution_causes[0].materialization.seq = 2;
        let record = EventRecord {
            seq: 3,
            envelope: unordered.envelope(events.ledger_identity()).unwrap(),
        };
        assert!(SourceObservation::from_record(&record, events.ledger_identity()).is_err());
        events
            .append_capability()
            .append_durable(
                observed.envelope(events.ledger_identity()).unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        assert!(
            super::super::condition::current(&capability, &events.replay_capability()).is_err(),
            "current Security meaning refuses an unproved causal receipt"
        );
    }

    #[tokio::test]
    async fn inventory_observation_recovers_then_invalidates_unavailable_source_without_refreshing_unchanged_evidence(
    ) {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        std::fs::create_dir_all(workspace.join("src")).unwrap();
        std::fs::write(workspace.join("src/lib.rs"), "pub fn run() {}\n").unwrap();
        std::fs::write(
            workspace.join("Cargo.toml"),
            "[package]\nname = \"inventory-observer\"\nversion = \"1.0.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let generated = tokio::process::Command::new(env!("CARGO"))
            .args(["generate-lockfile", "--offline"])
            .current_dir(&workspace)
            .output()
            .await
            .unwrap();
        assert!(generated.status.success());
        let lock = std::fs::read(workspace.join("Cargo.lock")).unwrap();
        let (mut capability, authority, _) = fixture(&root.path().join("unused-advisories"));
        capability.id = OBSERVE_INVENTORY.into();
        capability.workspace = Some(workspace.clone());
        capability.cargo = Some(env!("CARGO").into());
        capability.advisories = None;
        let mut policy = authority.policy;
        policy.principal_granted_action_ids = vec![OBSERVE_INVENTORY.into()];
        policy.runtime_allowed_action_ids = vec![OBSERVE_INVENTORY.into()];
        let authority =
            AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap()).unwrap();
        let product = super::super::inventory::cargo::observe(
            &workspace,
            std::path::Path::new(env!("CARGO")),
            capability.subject.clone(),
            7,
            &capability.limits,
        )
        .await
        .unwrap();
        let observed = SourceObservation {
            subject: capability.subject.clone(),
            policy: capability.policy.clone(),
            authority: authority.clone(),
            binding_id: "inventory-binding".into(),
            predecessor_source_id: None,
            generation_id: "generation".into(),
            incarnation_id: "incarnation".into(),
            observed_at: 7,
            advisory: None,
            failure: None,
            source_action: Some(OBSERVE_INVENTORY.into()),
            inventory: Some(product.clone()),
            execution_causes: vec![],
        };
        {
            let events = EventAuthority::open(
                sled::open(root.path().join("events")).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            events
                .append_capability()
                .append_durable_proven(
                    observed.envelope(events.ledger_identity()).unwrap(),
                    AppendMode::Idempotent,
                )
                .unwrap();
        }
        std::fs::remove_file(workspace.join("Cargo.lock")).unwrap();
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        assert!(resume_pending(&capability, &events.append_capability()).unwrap());
        let (recovered, bodies) =
            super::super::condition::current_state(&capability, &events.replay_capability())
                .unwrap();
        assert_eq!(bodies[INVENTORY], serde_json::to_value(&product).unwrap());
        let original = recovered.unwrap().products[INVENTORY].clone();
        let unavailable = poll_inventory(
            &capability,
            &authority,
            "inventory-binding",
            "generation",
            "incarnation",
            &events.append_capability(),
        )
        .await
        .unwrap();
        assert!(unavailable.0 && unavailable.1.is_some());
        let (condition, bodies) =
            super::super::condition::current_state(&capability, &events.replay_capability())
                .unwrap();
        let condition = condition.unwrap();
        assert!(!bodies.contains_key(INVENTORY));
        assert!(condition.products.contains_key(INVENTORY_UNAVAILABLE));
        assert!(!condition.verified_clean && !condition.coverage_current);
        let before = events.watermark_capability().snapshot().unwrap().tip_seq;
        assert!(
            !poll_inventory(
                &capability,
                &authority,
                "inventory-binding",
                "generation",
                "incarnation",
                &events.append_capability()
            )
            .await
            .unwrap()
            .0
        );
        assert_eq!(
            events.watermark_capability().snapshot().unwrap().tip_seq,
            before
        );
        std::fs::write(workspace.join("Cargo.lock"), lock).unwrap();
        assert!(
            poll_inventory(
                &capability,
                &authority,
                "inventory-binding",
                "generation",
                "incarnation",
                &events.append_capability()
            )
            .await
            .unwrap()
            .0
        );
        let restored = super::super::condition::current(&capability, &events.replay_capability())
            .unwrap()
            .unwrap();
        assert_ne!(restored.products[INVENTORY].receipt_id, original.receipt_id);
        let before = events.watermark_capability().snapshot().unwrap().tip_seq;
        assert!(
            !poll_inventory(
                &capability,
                &authority,
                "inventory-binding",
                "generation",
                "incarnation",
                &events.append_capability()
            )
            .await
            .unwrap()
            .0
        );
        assert_eq!(
            events.watermark_capability().snapshot().unwrap().tip_seq,
            before,
            "polling cannot manufacture a newer acquisition time"
        );
        let records = events.replay_capability().newest_page(128).unwrap().records;
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == INVENTORY_EVENT)
                .count(),
            3
        );
        assert!(!records
            .iter()
            .any(|record| record.event_type == super::super::publication::RECEIPT_EVENT));
        let mut denied = authority.policy.clone();
        denied.principal_granted_action_ids = vec![ACQUIRE_ADVISORIES.into()];
        let denied =
            AuthorityPolicyBinding::new(denied.clone(), denied.content_hash().unwrap()).unwrap();
        assert!(poll_inventory(
            &capability,
            &denied,
            "inventory-binding",
            "generation",
            "incarnation",
            &events.append_capability()
        )
        .await
        .unwrap_err()
        .contains("exact granted source read"));
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
            assert!(result.unwrap_err().contains("exact granted source read"));
        }
        assert_eq!(events.watermark_capability().snapshot().unwrap().tip_seq, 0);
    }
}
