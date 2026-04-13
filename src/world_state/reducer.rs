use crate::error::StorageError;
use crate::telemetry::events::{ProgressEnvelope, ProgressEvent};
use crate::telemetry::sinks::store::ProgressStore;
use crate::telemetry::DomainObjectRef;
use crate::world_state::contracts::{
    ClaimKind, ClaimRecord, EvidenceRecord, SettlementStatus,
};
use crate::world_state::events::{
    claim_added_envelope, claim_superseded_envelope, evidence_attached_envelope,
    ClaimAddedEventData, ClaimSupersededEventData, EvidenceAttachedEventData,
};
use crate::world_state::projection::{ClaimProvenanceProjection, CurrentClaimProjection};
use crate::world_state::store::{StoredWorldStateFact, WorldStateStore};

pub struct WorldStateReducer {
    pub current_claims: CurrentClaimProjection,
    pub provenance: ClaimProvenanceProjection,
    pub emitted_envelopes: Vec<ProgressEnvelope>,
}

impl WorldStateReducer {
    pub fn replay_from_spine(
        spine: &ProgressStore,
        store: &WorldStateStore,
        after_seq: u64,
    ) -> Result<Self, StorageError> {
        let mut reducer = Self {
            current_claims: CurrentClaimProjection::default(),
            provenance: ClaimProvenanceProjection::default(),
            emitted_envelopes: Vec::new(),
        };
        for event in spine.read_all_events_after(after_seq)? {
            reducer.apply_event(store, &event)?;
        }
        Ok(reducer)
    }

    fn apply_event(
        &mut self,
        store: &WorldStateStore,
        event: &ProgressEvent,
    ) -> Result<(), StorageError> {
        if event.domain_id != "execution" {
            return Ok(());
        }

        match event.event_type.as_str() {
            "execution.control.node_completed" => {
                let Some(subject) = find_object_ref(&event.objects, "workspace_fs", "node") else {
                    return Ok(());
                };
                self.materialize_claim(
                    store,
                    event,
                    subject,
                    ClaimKind::GenerationSucceeded,
                )?;
            }
            "execution.control.node_failed" => {
                let Some(subject) = find_object_ref(&event.objects, "workspace_fs", "node") else {
                    return Ok(());
                };
                self.materialize_claim(
                    store,
                    event,
                    subject,
                    ClaimKind::GenerationFailed,
                )?;
            }
            "execution.task.artifact_emitted" => {
                let Some(subject) = find_object_ref(&event.objects, "execution", "task_run") else {
                    return Ok(());
                };
                self.materialize_claim(
                    store,
                    event,
                    subject,
                    ClaimKind::ArtifactAvailable,
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    fn materialize_claim(
        &mut self,
        store: &WorldStateStore,
        event: &ProgressEvent,
        subject: DomainObjectRef,
        claim_kind: ClaimKind,
    ) -> Result<(), StorageError> {
        let subject_key = subject.index_key();
        let claim_id = format!("claim::{}::{}::{}", claim_kind.as_str(), subject_key, event.seq);
        let source_fact_id = source_fact_id(event.seq);
        let evidence_id = format!("evidence::{claim_id}");

        for active_claim in store.current_claims_for_object(&subject)? {
            if active_claim.claim_kind == claim_kind {
                continue;
            }
            if !matches!(
                active_claim.claim_kind,
                ClaimKind::GenerationSucceeded | ClaimKind::GenerationFailed
            ) {
                continue;
            }
            if !matches!(
                claim_kind,
                ClaimKind::GenerationSucceeded | ClaimKind::GenerationFailed
            ) {
                continue;
            }

            let superseded_fact_id =
                format!("world_state::claim_superseded::{}::{}", active_claim.claim_id, event.seq);
            let mut superseded_claim = active_claim.clone();
            superseded_claim.status = SettlementStatus::Superseded;
            superseded_claim.superseded_by = Some(claim_id.clone());
            superseded_claim.last_updated_seq = event.seq;
            store.put_claim(&superseded_claim)?;
            store.clear_claim_active(&subject, &superseded_claim.claim_id)?;
            store.put_supersession(&superseded_claim.claim_id, &claim_id)?;
            store.put_fact(&StoredWorldStateFact {
                fact_id: superseded_fact_id.clone(),
                event_type: "world_state.claim_superseded".to_string(),
                claim_id: Some(superseded_claim.claim_id.clone()),
                evidence_id: None,
                source_spine_fact_id: Some(source_fact_id.clone()),
                seq: event.seq,
            })?;
            self.current_claims
                .supersede(&subject_key, &superseded_claim.claim_id, event.seq);
            self.provenance.add_supersession(
                &superseded_claim.claim_id,
                claim_id.clone(),
                event.seq,
            );
            self.emitted_envelopes.push(claim_superseded_envelope(
                &event.session,
                ClaimSupersededEventData {
                    fact_id: superseded_fact_id,
                    claim_id: superseded_claim.claim_id,
                    superseded_by: claim_id.clone(),
                    subject: subject.clone(),
                    source_fact_id: source_fact_id.clone(),
                    seq: event.seq,
                },
            ));
        }

        let claim_fact_id = format!("world_state::claim_added::{claim_id}");
        let evidence_fact_id = format!("world_state::evidence_attached::{evidence_id}");
        let claim = ClaimRecord {
            claim_id: claim_id.clone(),
            claim_kind: claim_kind.clone(),
            subject: subject.clone(),
            status: SettlementStatus::Active,
            supporting_fact_ids: vec![source_fact_id.clone()],
            superseded_by: None,
            created_by_fact_id: claim_fact_id.clone(),
            created_at_seq: event.seq,
            last_updated_seq: event.seq,
        };
        let evidence = EvidenceRecord {
            evidence_id: evidence_id.clone(),
            claim_id: claim_id.clone(),
            source_fact_id: source_fact_id.clone(),
            source_event_type: event.event_type.clone(),
            objects: event.objects.clone(),
            relations: event.relations.clone(),
        };

        store.put_claim(&claim)?;
        store.set_claim_active(&subject, &claim_id)?;
        store.put_evidence(&evidence)?;
        store.put_fact(&StoredWorldStateFact {
            fact_id: claim_fact_id.clone(),
            event_type: "world_state.claim_added".to_string(),
            claim_id: Some(claim_id.clone()),
            evidence_id: None,
            source_spine_fact_id: Some(source_fact_id.clone()),
            seq: event.seq,
        })?;
        store.put_fact(&StoredWorldStateFact {
            fact_id: evidence_fact_id.clone(),
            event_type: "world_state.evidence_attached".to_string(),
            claim_id: Some(claim_id.clone()),
            evidence_id: Some(evidence_id.clone()),
            source_spine_fact_id: Some(source_fact_id.clone()),
            seq: event.seq,
        })?;

        self.current_claims
            .activate(subject_key, claim_id.clone(), event.seq);
        self.provenance
            .add_evidence(&claim_id, evidence_id.clone(), source_fact_id.clone(), event.seq);
        self.emitted_envelopes.push(claim_added_envelope(
            &event.session,
            ClaimAddedEventData {
                fact_id: claim_fact_id,
                claim_id: claim_id.clone(),
                claim_kind: claim_kind.as_str().to_string(),
                subject: subject.clone(),
                source_fact_id: source_fact_id.clone(),
                seq: event.seq,
            },
        ));
        self.emitted_envelopes.push(evidence_attached_envelope(
            &event.session,
            &claim_id,
            EvidenceAttachedEventData {
                fact_id: evidence_fact_id,
                evidence_id,
                claim_id: claim_id.clone(),
                source_fact_id,
                source_event_type: event.event_type.clone(),
                seq: event.seq,
            },
            event.objects.clone(),
            event.relations.clone(),
        ));

        Ok(())
    }
}

fn find_object_ref(
    objects: &[DomainObjectRef],
    domain_id: &str,
    object_kind: &str,
) -> Option<DomainObjectRef> {
    objects
        .iter()
        .find(|object| object.domain_id == domain_id && object.object_kind == object_kind)
        .cloned()
}

fn source_fact_id(seq: u64) -> String {
    format!("spine::{seq}")
}
