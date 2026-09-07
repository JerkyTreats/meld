//! Published operational returns available to independently owned semantic observers.

use super::build_publication_envelope;
use crate::task_network::{dispatch::Outcome, outcome::Publication};
use meld_events::{
    EventRecord, EventRecordRef, EventReplayCapability, LedgerCursor, LedgerIdentity, ReplayRequest,
};

/// An accepted operational outcome and its exact durable publication position.
/// This does not substitute for the complete admitted Task's discharge account.
#[derive(Clone, Debug, PartialEq)]
pub struct PublishedOutcome {
    /// Position of the canonical Execution publication in its Event authority.
    pub position: EventRecordRef,
    /// Native network that accepted and published this operational return.
    pub network_id: String,
    /// Intact outcome, including artifacts and retained attempt attribution.
    pub outcome: Outcome,
}

impl PublishedOutcome {
    /// Decode the owner's canonical publication, preserving the outcome intact.
    pub fn from_record(record: &EventRecord, ledger: LedgerIdentity) -> Result<Self, String> {
        if record.domain_id != "execution"
            || !matches!(
                record.event_type.as_str(),
                "execution.task.succeeded" | "execution.task.failed"
            )
        {
            return Err("record is not an Execution outcome publication".into());
        }
        let networks: Vec<_> = record
            .objects
            .iter()
            .filter(|object| {
                object.domain_id == "execution" && object.object_kind == "task_network"
            })
            .collect();
        let [network] = networks.as_slice() else {
            return Err("Execution outcome requires one exact network identity".into());
        };
        let outcome: Outcome =
            serde_json::from_value(record.data.clone()).map_err(|error| error.to_string())?;
        let publication = Publication::pending_for_outcome(&network.object_id, &outcome);
        let expected = build_publication_envelope(&record.session, &publication)
            .map_err(|error| error.to_string())?;
        // Additional graph relationships may carry separate shared-admission accounts.
        if record.seq == 0
            || record.record_id != expected.record_id
            || record.stream_id != expected.stream_id
            || record.event_type != expected.event_type
            || record.data != expected.data
            || record.provenance != expected.provenance
            || !expected
                .objects
                .iter()
                .all(|object| record.objects.contains(object))
            || !expected
                .relations
                .iter()
                .all(|relation| record.relations.contains(relation))
        {
            return Err(
                "Execution outcome differs from its canonical publication identity or product"
                    .into(),
            );
        }
        Ok(Self {
            position: EventRecordRef {
                ledger_id: ledger,
                seq: record.seq,
            },
            network_id: network.object_id.clone(),
            outcome,
        })
    }

    /// Outputs authored by this admitted operational node. Legacy publications
    /// can contain copied initialization records or intact forwarded artifacts;
    /// their presence does not attribute the original effect to this node.
    pub fn produced_artifacts(&self) -> impl Iterator<Item = &crate::task::ArtifactRecord> {
        self.outcome.artifact_records.iter().filter(|artifact| {
            artifact.producer.task_id == self.outcome.task_instance_id
                && artifact.producer.capability_instance_id != "__task_init__"
        })
    }

    /// Resolve a retained position against the exact Event authority.
    pub fn read(events: &EventReplayCapability, position: &EventRecordRef) -> Result<Self, String> {
        if position.ledger_id != events.ledger_identity() || position.seq == 0 {
            return Err(
                "Execution observation position names a foreign ledger or zero sequence".into(),
            );
        }
        let page = events
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: position.ledger_id,
                    after_seq: position.seq - 1,
                },
                limit: 1,
            })
            .map_err(|error| error.to_string())?;
        let record = page
            .records
            .first()
            .filter(|record| record.seq == position.seq)
            .ok_or("Execution observation position is not retained")?;
        Self::from_record(record, events.ledger_identity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_network::dispatch::OutcomeStatus;
    use meld_events::{AppendMode, EventAuthority, EventAuthorityOpenOptions};

    #[test]
    fn exact_return_read_refuses_foreign_positions_and_contradictory_publications() {
        let root = tempfile::tempdir().unwrap();
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let other = EventAuthority::open(
            sled::open(root.path().join("other")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let outcome = Outcome {
            outcome_id: "outcome".into(),
            task_instance_id: "node".into(),
            lifecycle_epoch: 1,
            claim_id: "claim".into(),
            claim_revision: 7,
            status: OutcomeStatus::Succeeded,
            error: None,
            artifact_records: vec![],
            task_events: vec![],
            admission: None,
        };
        let mut outcome = outcome;
        for (id, task, capability) in [
            ("produced", "node", "action"),
            ("input", "node", "__task_init__"),
            ("forwarded", "another-node", "action"),
        ] {
            outcome.artifact_records.push(crate::task::ArtifactRecord {
                artifact_id: id.into(),
                artifact_type_id: "receipt".into(),
                schema_version: 1,
                content: serde_json::json!({}),
                producer: crate::task::ArtifactProducerRef {
                    task_id: task.into(),
                    capability_instance_id: capability.into(),
                    invocation_id: Some("invocation".into()),
                    output_slot_id: Some("receipt".into()),
                },
            });
        }
        let publication = Publication::pending_for_outcome("network", &outcome);
        let envelope = build_publication_envelope("source", &publication).unwrap();
        let receipt = events
            .append_capability()
            .append_durable(envelope, AppendMode::Idempotent)
            .unwrap();
        let position = EventRecordRef {
            ledger_id: events.ledger_identity(),
            seq: receipt.seq,
        };
        let observed = PublishedOutcome::read(&events.replay_capability(), &position).unwrap();
        assert_eq!(observed.outcome, outcome);
        assert_eq!(observed.network_id, "network");
        assert_eq!(
            observed
                .produced_artifacts()
                .map(|artifact| artifact.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["produced"]
        );
        assert_eq!(
            observed.outcome.artifact_records.len(),
            3,
            "the retained outcome still travels intact"
        );
        assert!(PublishedOutcome::read(&other.replay_capability(), &position).is_err());
        assert!(PublishedOutcome::read(
            &events.replay_capability(),
            &EventRecordRef {
                seq: position.seq + 1,
                ..position
            }
        )
        .is_err());
        let record = events
            .replay_capability()
            .committed_record(&publication_event_record_id(&publication.publication_id))
            .unwrap()
            .unwrap();
        let mut wrong_status = record.clone();
        wrong_status.envelope.event_type = "execution.task.failed".into();
        assert!(PublishedOutcome::from_record(&wrong_status, events.ledger_identity()).is_err());
        let mut wrong_network = record.clone();
        wrong_network
            .envelope
            .objects
            .iter_mut()
            .find(|object| object.object_kind == "task_network")
            .unwrap()
            .object_id = "foreign".into();
        assert!(PublishedOutcome::from_record(&wrong_network, events.ledger_identity()).is_err());
        let mut wrong_identity = record;
        wrong_identity.envelope.data["outcome_id"] = "another-outcome".into();
        assert!(PublishedOutcome::from_record(&wrong_identity, events.ledger_identity()).is_err());
    }

    fn publication_event_record_id(id: &str) -> String {
        super::super::publication_event_record_id(id)
    }
}
