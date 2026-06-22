//! Task network publication bridge into the event spine.
//!
//! Owner: task network.
//! Inputs: durable retryable task outcome publication records.
//! Outputs: idempotent event spine appends and publication mark commands.
//! Does not own: this module does not schedule background workers or map
//! execution facts into world model evidence.

use meld_events::events::store::EventStore;
use meld_events::{DomainObjectRef, EventEnvelope, EventRelation};
use serde::Serialize;

use crate::task_network::{
    command::{self, Command},
    contracts::{has_text, stable_hash},
    mutation::{ReadPrecondition, Rejection},
    outcome::{Publication, PublicationState},
    store::SledTaskNetworkStore,
};

const PUBLICATION_ACTOR_ID: &str = "execution.task_network.publication";

/// Request to publish retryable task network publication records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPendingPublicationsRequest {
    /// Event spine session partition for produced envelopes.
    pub session_id: String,
    /// Worker identity for audit and request validation.
    pub worker_id: String,
    /// Optional maximum number of retryable publications to process.
    pub limit: Option<usize>,
}

/// Scope for one publication bridge report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationBridgeScope {
    /// Task network whose publication outbox was scanned.
    pub network_id: String,
    /// Event session used for appended publication facts.
    pub session_id: String,
    /// Worker identity supplied by the caller.
    pub worker_id: String,
}

/// Diagnostic issue produced by a publication bridge tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationBridgeIssue {
    /// Publication id associated with the issue when known.
    pub publication_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Successful event append metadata for one publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationAppend {
    /// Task network publication id.
    pub publication_id: String,
    /// Deterministic event spine record id.
    pub event_record_id: String,
    /// Event spine sequence returned by append.
    pub event_seq: u64,
}

/// Result for one attempted publication bridge item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationPublishResult {
    /// Publication appended and was marked published.
    Published {
        /// Task network publication id.
        publication_id: String,
        /// Deterministic event spine record id.
        event_record_id: String,
        /// Event spine sequence returned by append.
        event_seq: u64,
        /// Task network revision that recorded the mark.
        marked_revision: u64,
    },
    /// Publication was already marked published before this attempt.
    AlreadyPublished {
        /// Task network publication id.
        publication_id: String,
    },
    /// Task network store rejected the publication mark.
    MarkRejected {
        /// Task network publication id.
        publication_id: String,
        /// Rejection summary.
        reason: String,
    },
    /// Event append failed and a retryable failure mark was attempted.
    AppendFailed {
        /// Task network publication id.
        publication_id: String,
        /// Append failure summary.
        error: String,
    },
}

/// Report from a bounded publication bridge pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationBridgeReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Domain-specific report scope.
    pub scope: PublicationBridgeScope,
    /// Task network revision before selecting publications.
    pub input_revision: u64,
    /// Task network revision after processing selected publications.
    pub output_revision: u64,
    /// Publication records attempted by this pass.
    pub items_attempted: usize,
    /// Publication records successfully appended and marked published.
    pub items_committed: usize,
    /// Retryable diagnostics observed during the pass.
    pub retryable_errors: Vec<PublicationBridgeIssue>,
    /// Fatal diagnostics observed during the pass.
    pub fatal_errors: Vec<PublicationBridgeIssue>,
    /// True when more retryable publication records remain after the limit.
    pub budget_exhausted: bool,
    /// Per publication results in deterministic publication id order.
    pub results: Vec<PublicationPublishResult>,
}

/// Publication bridge error.
#[derive(Debug, thiserror::Error)]
pub enum PublicationBridgeError {
    /// Request validation failed before bridge work began.
    #[error("publication bridge request is invalid: {0}")]
    InvalidRequest(String),

    /// Event append failed outside per publication handling.
    #[error("event append failed: {0}")]
    EventAppend(String),

    /// Task network store returned a storage error.
    #[error("task network command failed: {0}")]
    TaskNetworkStore(String),
}

/// Event append capability used by the bridge and tests.
pub trait EventAppendSink {
    /// Appends an envelope idempotently and returns the event spine sequence.
    fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<u64, String>;
}

impl EventAppendSink for EventStore {
    fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<u64, String> {
        EventStore::append_envelope_idempotent(self, envelope).map_err(|error| error.to_string())
    }
}

/// Builds a canonical event spine envelope for one task network publication.
pub fn build_publication_envelope(
    session_id: &str,
    publication: &Publication,
) -> Result<EventEnvelope, PublicationBridgeError> {
    require_text("session_id", session_id)?;
    require_publication_identity(publication)?;

    let event_record_id = publication_event_record_id(&publication.publication_id);
    let stream_id = publication_stream_id(publication);
    let network = object("task_network", &publication.network_id)?;
    let task_run = object("task_run", &publication.outcome.task_instance_id)?;
    let outcome = object("task_outcome", &publication.outcome.outcome_id)?;
    let publication_object = object("task_publication", &publication.publication_id)?;
    let mut objects = vec![
        network.clone(),
        task_run.clone(),
        outcome.clone(),
        publication_object.clone(),
    ];
    let mut relations = vec![
        relation("published_from", publication_object, outcome.clone())?,
        relation("produced_by", outcome, task_run.clone())?,
        relation("member_of", task_run.clone(), network)?,
    ];

    for artifact in &publication.outcome.artifact_records {
        require_text("artifact_id", &artifact.artifact_id)?;
        require_text("artifact_type_id", &artifact.artifact_type_id)?;

        let artifact_object = object("artifact", &artifact.artifact_id)?;
        let slot_id = format!(
            "{}::{}",
            publication.outcome.task_instance_id, artifact.artifact_type_id
        );
        let artifact_slot = object("artifact_slot", &slot_id)?;
        objects.push(artifact_object.clone());
        objects.push(artifact_slot.clone());
        relations.push(relation(
            "attached_to",
            artifact_slot.clone(),
            task_run.clone(),
        )?);
        relations.push(relation("selected", artifact_slot, artifact_object)?);
    }

    Ok(EventEnvelope::with_now_domain(
        session_id,
        "execution",
        stream_id,
        publication.event_type(),
        None,
        publication.event_payload(),
    )
    .with_record_id(event_record_id)
    .with_graph(objects, relations))
}

/// Publishes one task network publication by id.
pub fn publish_publication<E: EventAppendSink>(
    store: &mut SledTaskNetworkStore,
    events: &E,
    request: &PublishPendingPublicationsRequest,
    publication_id: &str,
) -> Result<PublicationPublishResult, PublicationBridgeError> {
    validate_request(request)?;
    let Some(publication) = store.state().publications.get(publication_id).cloned() else {
        return Ok(PublicationPublishResult::MarkRejected {
            publication_id: publication_id.to_string(),
            reason: "publication was not found".to_string(),
        });
    };

    if matches!(publication.state, PublicationState::Published { .. }) {
        return Ok(PublicationPublishResult::AlreadyPublished {
            publication_id: publication.publication_id,
        });
    }

    let event_record_id = publication_event_record_id(&publication.publication_id);
    let envelope = build_publication_envelope(&request.session_id, &publication)?;
    match events.append_envelope_idempotent(envelope) {
        Ok(event_seq) => publish_marked_publication(store, publication, event_record_id, event_seq),
        Err(error) => record_append_failure(store, publication, error),
    }
}

/// Publishes retryable task network publications in deterministic id order.
pub fn publish_pending_publications<E: EventAppendSink>(
    store: &mut SledTaskNetworkStore,
    events: &E,
    request: PublishPendingPublicationsRequest,
) -> Result<PublicationBridgeReport, PublicationBridgeError> {
    validate_request(&request)?;

    let input_revision = store.state().revision;
    let scope = PublicationBridgeScope {
        network_id: store.state().network_id.clone(),
        session_id: request.session_id.clone(),
        worker_id: request.worker_id.clone(),
    };

    let candidate_limit = request.limit.map(|limit| limit.saturating_add(1));
    let publication_ids = store
        .state()
        .publications
        .iter()
        .filter(|(_, publication)| {
            matches!(
                publication.state,
                PublicationState::Pending | PublicationState::Failed { .. }
            )
        })
        .map(|(publication_id, _)| publication_id.clone());
    let mut publication_ids = match candidate_limit {
        Some(limit) => publication_ids.take(limit).collect::<Vec<_>>(),
        None => publication_ids.collect::<Vec<_>>(),
    };
    let budget_exhausted = request
        .limit
        .map(|limit| publication_ids.len() > limit)
        .unwrap_or(false);
    if let Some(limit) = request.limit {
        publication_ids.truncate(limit);
    }

    let mut results = Vec::with_capacity(publication_ids.len());
    for publication_id in publication_ids {
        results.push(publish_publication(
            store,
            events,
            &request,
            &publication_id,
        )?);
    }

    let output_revision = store.state().revision;
    Ok(report_from_results(
        scope,
        input_revision,
        output_revision,
        budget_exhausted,
        results,
    ))
}

fn report_from_results(
    scope: PublicationBridgeScope,
    input_revision: u64,
    output_revision: u64,
    budget_exhausted: bool,
    results: Vec<PublicationPublishResult>,
) -> PublicationBridgeReport {
    let mut retryable_errors = Vec::new();
    let mut fatal_errors = Vec::new();
    let mut items_committed = 0;

    for result in &results {
        match result {
            PublicationPublishResult::Published { .. } => {
                items_committed += 1;
            }
            PublicationPublishResult::AlreadyPublished { .. } => {}
            PublicationPublishResult::MarkRejected {
                publication_id,
                reason,
            } => fatal_errors.push(PublicationBridgeIssue {
                publication_id: Some(publication_id.clone()),
                code: "publication_mark_rejected".to_string(),
                message: reason.clone(),
            }),
            PublicationPublishResult::AppendFailed {
                publication_id,
                error,
            } => retryable_errors.push(PublicationBridgeIssue {
                publication_id: Some(publication_id.clone()),
                code: "publication_append_failed".to_string(),
                message: error.clone(),
            }),
        }
    }

    PublicationBridgeReport {
        actor_id: PUBLICATION_ACTOR_ID.to_string(),
        scope,
        input_revision,
        output_revision,
        items_attempted: results.len(),
        items_committed,
        retryable_errors,
        fatal_errors,
        budget_exhausted,
        results,
    }
}

fn publish_marked_publication(
    store: &mut SledTaskNetworkStore,
    mut publication: Publication,
    event_record_id: String,
    event_seq: u64,
) -> Result<PublicationPublishResult, PublicationBridgeError> {
    let publication_id = publication.publication_id.clone();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        event_seq: Some(event_seq),
    };
    let base_revision = store.state().revision;
    let command_id = format!(
        "task-network-publication-mark::{publication_id}::published::{event_record_id}::rev::{base_revision}"
    );
    match submit_mark_publication(store, command_id, publication)? {
        command::Response::Accepted { .. } | command::Response::Duplicate { .. } => {
            if let Some(result) =
                published_result_from_state(store, &publication_id, event_record_id, event_seq)
            {
                Ok(result)
            } else {
                Ok(PublicationPublishResult::MarkRejected {
                    publication_id,
                    reason: "publication mark did not produce published state".to_string(),
                })
            }
        }
        command::Response::Rejected(rejection) => Ok(PublicationPublishResult::MarkRejected {
            publication_id,
            reason: rejection_summary(&rejection),
        }),
    }
}

fn record_append_failure(
    store: &mut SledTaskNetworkStore,
    mut publication: Publication,
    error: String,
) -> Result<PublicationPublishResult, PublicationBridgeError> {
    let publication_id = publication.publication_id.clone();
    publication.state = PublicationState::Failed {
        error: error.clone(),
    };
    let base_revision = store.state().revision;
    let command_id = format!(
        "task-network-publication-mark::{publication_id}::failed::{}::rev::{base_revision}",
        stable_error_hash(&error)
    );
    match submit_mark_publication(store, command_id, publication)? {
        command::Response::Accepted { .. } | command::Response::Duplicate { .. } => {
            Ok(PublicationPublishResult::AppendFailed {
                publication_id,
                error,
            })
        }
        command::Response::Rejected(rejection) => Ok(PublicationPublishResult::MarkRejected {
            publication_id,
            reason: rejection_summary(&rejection),
        }),
    }
}

fn submit_mark_publication(
    store: &mut SledTaskNetworkStore,
    command_id: String,
    publication: Publication,
) -> Result<command::Response, PublicationBridgeError> {
    let request = command::Request {
        command_id,
        network_id: publication.network_id.clone(),
        base_revision: store.state().revision,
        base_state_hash: store.state().state_hash.clone(),
        read_preconditions: vec![ReadPrecondition::PublicationPending(
            publication.publication_id.clone(),
        )],
        command: Command::MarkPublication(publication),
    };
    store
        .submit(request)
        .map_err(|error| PublicationBridgeError::TaskNetworkStore(error.to_string()))
}

fn published_result_from_state(
    store: &SledTaskNetworkStore,
    publication_id: &str,
    event_record_id: String,
    event_seq: u64,
) -> Option<PublicationPublishResult> {
    let publication = store.state().publications.get(publication_id)?;
    let PublicationState::Published {
        marked_revision,
        event_seq: stored_event_seq,
    } = &publication.state
    else {
        return None;
    };
    Some(PublicationPublishResult::Published {
        publication_id: publication_id.to_string(),
        event_record_id,
        event_seq: (*stored_event_seq).unwrap_or(event_seq),
        marked_revision: *marked_revision,
    })
}

fn validate_request(
    request: &PublishPendingPublicationsRequest,
) -> Result<(), PublicationBridgeError> {
    require_text("session_id", &request.session_id)?;
    require_text("worker_id", &request.worker_id)
}

fn require_publication_identity(publication: &Publication) -> Result<(), PublicationBridgeError> {
    require_text("publication_id", &publication.publication_id)?;
    require_text("network_id", &publication.network_id)?;
    require_text("task_instance_id", &publication.outcome.task_instance_id)?;
    require_text("outcome_id", &publication.outcome.outcome_id)
}

fn require_text(name: &str, value: &str) -> Result<(), PublicationBridgeError> {
    if has_text(value) {
        Ok(())
    } else {
        Err(PublicationBridgeError::InvalidRequest(format!(
            "{name} must be non-empty"
        )))
    }
}

fn object(kind: &str, id: &str) -> Result<DomainObjectRef, PublicationBridgeError> {
    DomainObjectRef::new("execution", kind, id)
        .map_err(|error| PublicationBridgeError::InvalidRequest(error.to_string()))
}

fn relation(
    relation_type: &str,
    src: DomainObjectRef,
    dst: DomainObjectRef,
) -> Result<EventRelation, PublicationBridgeError> {
    EventRelation::new(relation_type, src, dst)
        .map_err(|error| PublicationBridgeError::InvalidRequest(error.to_string()))
}

fn publication_event_record_id(publication_id: &str) -> String {
    format!("execution::task_network_publication::{publication_id}")
}

fn publication_stream_id(publication: &Publication) -> String {
    format!(
        "task_network::{}::task::{}",
        publication.network_id, publication.outcome.task_instance_id
    )
}

fn stable_error_hash(error: &str) -> String {
    #[derive(Serialize)]
    struct ErrorIdentity<'a> {
        error: &'a str,
    }

    stable_hash(&ErrorIdentity { error })
}

fn rejection_summary(rejection: &Rejection) -> String {
    format!("{rejection:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> PublicationBridgeScope {
        PublicationBridgeScope {
            network_id: "network-docs".to_string(),
            session_id: "session-publication".to_string(),
            worker_id: "worker-publication".to_string(),
        }
    }

    #[test]
    fn report_classifies_mark_rejection_as_fatal() {
        let report = report_from_results(
            scope(),
            4,
            4,
            false,
            vec![PublicationPublishResult::MarkRejected {
                publication_id: "publication-a".to_string(),
                reason: "FailedPrecondition".to_string(),
            }],
        );

        assert_eq!(report.items_attempted, 1);
        assert_eq!(report.items_committed, 0);
        assert!(report.retryable_errors.is_empty());
        assert_eq!(report.fatal_errors.len(), 1);
        assert_eq!(
            report.fatal_errors[0].publication_id.as_deref(),
            Some("publication-a")
        );
        assert_eq!(report.fatal_errors[0].code, "publication_mark_rejected");
    }
}
