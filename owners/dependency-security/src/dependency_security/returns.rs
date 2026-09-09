//! Security-owned acceptance of published mutation outcomes as observation requests.

use super::{capability::SecurityCapability, observation::SourceObservation};
use meld_events::{EventRecordRef, EventReplayCapability, LedgerIdentity};
use serde::{Deserialize, Serialize};

pub(super) const MAX_CAUSES: usize = 64;

/// Separate producer positions behind an observation; neither proves Security posture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionObservationCause {
    pub outcome: EventRecordRef,
    pub materialization: EventRecordRef,
}

impl ExecutionObservationCause {
    pub(super) fn validate(&self, ledger: LedgerIdentity, observed_seq: u64) -> Result<(), String> {
        if self.outcome.ledger_id != ledger
            || self.materialization.ledger_id != ledger
            || self.materialization.seq == 0
            || self.materialization.seq >= self.outcome.seq
            || self.outcome.seq >= observed_seq
        {
            return Err(
                "Security observation cause has foreign or unordered producer positions".into(),
            );
        }
        Ok(())
    }

    #[cfg(unix)]
    fn evidence(
        &self,
        events: &EventReplayCapability,
        subject: &meld_events::DomainObjectRef,
    ) -> Result<crate::code_change::contracts::MaterializationEvidence, String> {
        let published =
            meld_execution::task_network::publication::observation::PublishedOutcome::read(
                events,
                &self.outcome,
            )?;
        for artifact in published.produced_artifacts() {
            if artifact.artifact_type_id != crate::code_change::capability::RECEIPT {
                continue;
            }
            let proof = crate::code_change::materialization_evidence(events, artifact)?;
            if proof.materialization == self.materialization && &proof.change.subject == subject {
                return Ok(proof);
            }
        }
        Err(
            "Security observation cause has no matching materialization in its published outcome"
                .into(),
        )
    }
}

pub(super) fn positions(causes: &[ExecutionObservationCause]) -> Vec<EventRecordRef> {
    let mut positions: Vec<_> = causes
        .iter()
        .flat_map(|cause| [cause.materialization, cause.outcome])
        .collect();
    positions.sort_by_key(|position| position.seq);
    positions.dedup();
    positions
}

pub(super) fn validate_retained(
    observed: &SourceObservation,
    events: &EventReplayCapability,
) -> Result<(), String> {
    #[cfg(unix)]
    for cause in &observed.execution_causes {
        cause.evidence(events, &observed.subject.subject)?;
    }
    #[cfg(not(unix))]
    if !observed.execution_causes.is_empty() {
        return Err("native code materialization evidence is unavailable on this platform".into());
    }
    Ok(())
}

/// Select unacknowledged returns before capturing source content. Only the exact
/// directory bound to this observer can receive a code-change observation request.
#[cfg(unix)]
pub(crate) fn pending(
    capability: &SecurityCapability,
    events: &EventReplayCapability,
) -> Result<Vec<ExecutionObservationCause>, String> {
    use meld_events::{LedgerCursor, ReplayRequest};
    use std::{
        collections::{BTreeMap, BTreeSet},
        os::unix::fs::MetadataExt,
    };
    let Some(workspace) = &capability.workspace else {
        return Err("Security observation has no workspace binding".into());
    };
    // Unavailability still flows through normal inventory acquisition and its explicit disposition.
    let Ok(metadata) = std::fs::metadata(workspace) else {
        return Ok(vec![]);
    };
    let resource = (metadata.dev(), metadata.ino());
    let ledger = events.ledger_identity();
    let mut cursor = LedgerCursor {
        ledger_id: ledger,
        after_seq: 0,
    };
    let mut tip = None;
    let mut candidates = BTreeMap::new();
    let mut accepted = BTreeSet::new();
    let subject =
        serde_json::to_value(&capability.subject.subject).map_err(|error| error.to_string())?;
    loop {
        let page = events
            .replay(ReplayRequest {
                cursor,
                limit: 1024,
            })
            .map_err(|error| error.to_string())?;
        if page.coverage.retained_from > 1 {
            return Err(
                "Security return recovery requires retained producer and observation history"
                    .into(),
            );
        }
        let frozen = *tip.get_or_insert(page.coverage.tip_seq);
        for record in page.records.iter().filter(|record| record.seq <= frozen) {
            if record.domain_id == super::publication::OWNER
                && record.event_type == super::observation::INVENTORY_EVENT
            {
                let observation = SourceObservation::from_record(record, ledger)?;
                if observation.subject == capability.subject
                    && observation.policy == capability.policy
                {
                    validate_retained(&observation, events)?;
                    accepted.extend(
                        observation
                            .execution_causes
                            .iter()
                            .map(|cause| (cause.outcome.seq, cause.materialization.seq)),
                    );
                }
            }
            if record.domain_id != "execution"
                || !matches!(
                    record.event_type.as_str(),
                    "execution.task.succeeded" | "execution.task.failed"
                )
            {
                continue;
            }
            if !record.data["artifact_records"]
                .as_array()
                .is_some_and(|artifacts| {
                    artifacts.iter().any(|artifact| {
                        artifact["artifact_type_id"] == crate::code_change::capability::RECEIPT
                            && artifact["content"]["subject"] == subject
                    })
                })
            {
                continue;
            }
            let published = meld_execution::task_network::publication::observation::PublishedOutcome::from_record(record, ledger)?;
            for artifact in published.produced_artifacts() {
                if artifact.artifact_type_id != crate::code_change::capability::RECEIPT
                    || artifact.content["subject"] != subject
                {
                    continue;
                }
                let proof = crate::code_change::materialization_evidence(events, artifact)?;
                if proof.workspace_identity != resource {
                    continue;
                }
                let cause = ExecutionObservationCause {
                    outcome: published.position,
                    materialization: proof.materialization,
                };
                cause.validate(ledger, frozen.saturating_add(1))?;
                candidates.insert((cause.outcome.seq, cause.materialization.seq), cause);
            }
        }
        if page.next_cursor.after_seq >= frozen {
            break;
        }
        if page.next_cursor.after_seq <= cursor.after_seq {
            return Err("Security return replay made no progress".into());
        }
        cursor = page.next_cursor;
    }
    Ok(candidates
        .into_iter()
        .filter(|(key, _)| !accepted.contains(key))
        .map(|(_, cause)| cause)
        .take(MAX_CAUSES)
        .collect())
}

#[cfg(not(unix))]
pub(crate) fn pending(
    _: &SecurityCapability,
    _: &EventReplayCapability,
) -> Result<Vec<ExecutionObservationCause>, String> {
    Ok(vec![])
}
