//! Evidence ingestion actor, config-driven outcome mapping, and cursor
//! ordering tests.
//!
//! Covers frozen promoted-evidence identity derivation, bounded batch
//! budgets, cursor advancement for every disposition, durable invalid-record
//! rejections, and reopen-replay idempotency between evidence commit and
//! cursor commit. Docs vocabulary appears only in these fixtures.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use meld_world_model::belief::{
    promoted_evidence_identity, BeliefFamilyRegistry, BeliefFamilyRegistryStore, BeliefKey,
    BeliefStore, BranchScope, ConfiguredOutcomeMapping, EvidenceEventReplaySource,
    EvidenceIngestionActor, EvidenceIngestionRequest, EvidenceValue, OutcomeContentRule,
    OutcomeEvidenceMapping, OutcomeFieldRule, OutcomeMappingConfig, OutcomeMappingDisposition,
    OutcomeMappingInput, OutcomeSubjectBinding, OutcomeValueSource, EVIDENCE_CONSUMER_ID,
};
use meld_world_model::events::error::EventAuthorityError;
use meld_world_model::events::{
    AppendMode, ConsumerCursorError, ConsumerCursorState, DurableConsumerCursor, EventAuthority,
    EventAuthorityOpenOptions, EventEnvelope, EventPage, EventRecord, EventReplayCapability,
    LedgerIdentity, ReplayRequest,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::PerspectiveKey;
use serde_json::json;

const FAMILY_ID: &str = "docs_freshness";
const MAPPING_ID: &str = "docs_success_to_content_written";

fn object(domain_id: &str, kind: &str, id: &str) -> meld_world_model::events::DomainObjectRef {
    meld_world_model::events::DomainObjectRef::new(domain_id, kind, id).unwrap()
}

fn family_config_json() -> &'static str {
    r#"{
        "family_id": "docs_freshness",
        "dimension_id": "docs_freshness",
        "predicate_id": "confidence",
        "evidence_policy_id": "default_policy",
        "evidence_schemas": [
            {
                "schema_id": "content_written_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            },
            {
                "schema_id": "content_review_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }
        ],
        "source_mappings": [
            {
                "mapping_id": "content_written_to_signal",
                "source_kind": "content_written",
                "evidence_schema_id": "content_written_signal",
                "subject_from": "record.subject",
                "value_field": "stale_probability",
                "factor_id": "content_written_signal"
            },
            {
                "mapping_id": "content_written_to_review",
                "source_kind": "content_written",
                "evidence_schema_id": "content_review_signal",
                "subject_from": "record.subject",
                "value_field": "review_probability",
                "factor_id": "content_review_signal"
            }
        ],
        "comparator": {
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [
                {
                    "factor_id": "content_written_signal",
                    "evidence_schema_id": "content_written_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                },
                {
                    "factor_id": "content_review_signal",
                    "evidence_schema_id": "content_review_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }
            ],
            "missing_evidence_uncertainty": 0.9
        },
        "default_prior": 0.8,
        "planner_projection": {
            "confidence_field": "confidence",
            "threshold": 0.7,
            "posterior_meaning": "stale_probability"
        },
        "config_version": "1"
    }"#
}

fn mapping_config() -> OutcomeMappingConfig {
    OutcomeMappingConfig {
        mapping_id: MAPPING_ID.to_string(),
        source_kind: "content_written".to_string(),
        match_domain_id: "execution".to_string(),
        match_event_type: "execution.task.succeeded".to_string(),
        match_content: vec![OutcomeContentRule::ArrayAnyFieldEquals {
            array_pointer: "/artifact_records".to_string(),
            field: "artifact_type_id".to_string(),
            equals: "docs_patch".to_string(),
        }],
        subject: OutcomeSubjectBinding {
            object_kind: "node".to_string(),
            domain_id: Some("workspace_fs".to_string()),
            // Additive subject-source field; this fixture keeps the original
            // envelope-object binding semantics.
            from: Default::default(),
        },
        evidence_fields: vec![
            OutcomeFieldRule {
                field: "stale_probability".to_string(),
                source: OutcomeValueSource::Constant { value: 0.0 },
            },
            OutcomeFieldRule {
                field: "review_probability".to_string(),
                source: OutcomeValueSource::Constant { value: 0.2 },
            },
        ],
    }
}

fn success_envelope(record_id: &str, subject: Option<&str>) -> EventEnvelope {
    let mut envelope = EventEnvelope::new_domain(
        "2026-07-24T00:00:00Z".to_string(),
        "session-a",
        "execution",
        "workflow-a",
        "execution.task.succeeded",
        Some(format!("sha256:{record_id}")),
        json!({ "artifact_records": [{ "artifact_type_id": "docs_patch" }] }),
    )
    .with_record_id(record_id);
    if let Some(node_id) = subject {
        envelope = envelope.with_graph(vec![object("workspace_fs", "node", node_id)], Vec::new());
    }
    envelope
}

fn unrelated_envelope(event_type: &str) -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-07-24T00:00:00Z".to_string(),
        "session-a",
        "execution",
        "workflow-a",
        event_type,
        None,
        json!({}),
    )
}

struct ReplayPort(EventReplayCapability);

impl EvidenceEventReplaySource for ReplayPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.0.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.0.replay(request)
    }
}

/// Crash-injection cursor: durable reads pass through, but advancement
/// fails, simulating a reopen between evidence commit and cursor commit.
struct AdvanceFailingCursor<C: DurableConsumerCursor> {
    inner: C,
    failed_advances: AtomicUsize,
}

impl<C: DurableConsumerCursor> DurableConsumerCursor for AdvanceFailingCursor<C> {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_identity()
    }

    fn consumer_cursor(
        &self,
        consumer_id: &str,
    ) -> Result<Option<ConsumerCursorState>, ConsumerCursorError> {
        self.inner.consumer_cursor(consumer_id)
    }

    fn advance_consumer_cursor(
        &self,
        _consumer_id: &str,
        _after_seq: u64,
    ) -> Result<ConsumerCursorState, ConsumerCursorError> {
        self.failed_advances.fetch_add(1, Ordering::SeqCst);
        Err(ConsumerCursorError {
            message: "injected crash before cursor commit".to_string(),
            retryable: true,
        })
    }
}

struct Fixture {
    authority: EventAuthority,
    store: Arc<BeliefStore>,
    traversal: Arc<TraversalStore>,
    registry: BeliefFamilyRegistryStore,
    _events_dir: tempfile::TempDir,
    _belief_dir: tempfile::TempDir,
}

impl Fixture {
    fn open() -> Self {
        let events_dir = tempfile::tempdir().unwrap();
        let belief_dir = tempfile::tempdir().unwrap();
        let authority = EventAuthority::open(
            sled::open(events_dir.path()).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let belief_db = sled::open(belief_dir.path()).unwrap();
        let store = Arc::new(BeliefStore::new(belief_db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(belief_db.clone()).unwrap());
        let mut registry = BeliefFamilyRegistryStore::new(belief_db).unwrap();
        let config = serde_json::from_str(family_config_json()).unwrap();
        registry.install(config, 1).unwrap();
        Self {
            authority,
            store,
            traversal,
            registry,
            _events_dir: events_dir,
            _belief_dir: belief_dir,
        }
    }

    fn append(&self, envelope: EventEnvelope) -> u64 {
        self.authority
            .append_capability()
            .append_durable(envelope, AppendMode::Idempotent)
            .unwrap()
            .seq
    }

    fn actor(&self) -> EvidenceIngestionActor {
        self.actor_with_cursor(Arc::new(self.authority.consumer_registry_capability()))
    }

    fn actor_with_cursor(
        &self,
        cursor: Arc<dyn DurableConsumerCursor + Send + Sync>,
    ) -> EvidenceIngestionActor {
        EvidenceIngestionActor::new(
            "world_model.evidence.actor",
            Arc::clone(&self.store),
            Arc::clone(&self.traversal),
            Arc::new(self.registry.clone()),
            FAMILY_ID,
            Arc::new(ReplayPort(self.authority.replay_capability())),
            cursor,
            Arc::new(ConfiguredOutcomeMapping::new(mapping_config()).unwrap()),
            MAPPING_ID,
            PerspectiveKey::new("default", "default").unwrap(),
            BranchScope::main(),
        )
    }

    fn durable_cursor_seq(&self) -> u64 {
        self.authority
            .consumer_registry_capability()
            .consumer_cursor(EVIDENCE_CONSUMER_ID)
            .unwrap()
            .map(|state| state.after_seq)
            .unwrap_or(0)
    }

    fn belief_key(&self, node_id: &str) -> BeliefKey {
        BeliefKey {
            subject: object("workspace_fs", "node", node_id),
            dimension_id: "docs_freshness".to_string(),
            predicate_id: "confidence".to_string(),
            perspective: PerspectiveKey::new("default", "default").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "default_policy".to_string(),
        }
    }
}

#[test]
fn mapping_produces_the_frozen_promoted_evidence_identity() {
    let mapping = ConfiguredOutcomeMapping::new(mapping_config()).unwrap();
    let record = EventRecord::from_envelope(success_envelope("publication-a", Some("node-a")), 3);

    let disposition = mapping.map_outcome(&OutcomeMappingInput {
        record,
        mapping_id: MAPPING_ID.to_string(),
    });

    let OutcomeMappingDisposition::Applicable {
        evidence_id,
        record,
    } = disposition
    else {
        panic!("expected an applicable disposition");
    };
    assert_eq!(
        evidence_id,
        promoted_evidence_identity("publication-a", MAPPING_ID)
    );
    // The frozen identity anchors downstream deduplication as source_id.
    assert_eq!(record.source_id, evidence_id);
    assert_eq!(record.source_kind, "content_written");
    assert_eq!(record.subject, object("workspace_fs", "node", "node-a"));
    assert_eq!(record.source_cursor_start, 3);
    assert_eq!(record.source_cursor_end, 3);
    assert_eq!(
        record.fields.get("stale_probability"),
        Some(&EvidenceValue::Scalar(0.0))
    );
    assert_eq!(
        record.fields.get("review_probability"),
        Some(&EvidenceValue::Scalar(0.2))
    );
}

#[test]
fn mapping_understands_non_matching_records_and_rejects_unusable_matches() {
    let mapping = ConfiguredOutcomeMapping::new(mapping_config()).unwrap();
    let map = |record| {
        mapping.map_outcome(&OutcomeMappingInput {
            record,
            mapping_id: MAPPING_ID.to_string(),
        })
    };

    // Different event type: understood, no evidence.
    let other_type = EventRecord::from_envelope(unrelated_envelope("execution.task.failed"), 1);
    assert!(matches!(
        map(other_type),
        OutcomeMappingDisposition::NotApplicable { .. }
    ));

    // Matching type but content rules unmet: understood, no evidence.
    let mut no_artifact = success_envelope("publication-b", Some("node-a"));
    no_artifact.data = json!({ "artifact_records": [] });
    assert!(matches!(
        map(EventRecord::from_envelope(no_artifact, 2)),
        OutcomeMappingDisposition::NotApplicable { .. }
    ));

    // Matched but no subject object: unusable content.
    let no_subject = EventRecord::from_envelope(success_envelope("publication-c", None), 3);
    assert!(matches!(
        map(no_subject),
        OutcomeMappingDisposition::Invalid { .. }
    ));

    // Matched but no record identity: evidence identity would not be stable.
    let mut no_record_id = success_envelope("publication-d", Some("node-a"));
    no_record_id.record_id = None;
    assert!(matches!(
        map(EventRecord::from_envelope(no_record_id, 4)),
        OutcomeMappingDisposition::Invalid { .. }
    ));
}

#[test]
fn applicable_records_ingest_and_advance_the_cursor_after_durable_evidence() {
    let fixture = Fixture::open();
    let seq = fixture.append(success_envelope("publication-a", Some("node-a")));
    let mut actor = fixture.actor();

    let report = actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });

    assert_eq!(report.events_replayed, 1);
    assert_eq!(report.applicable_count, 1);
    assert!(report.new_assignment_count > 0);
    assert_eq!(report.revisions_committed, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert_eq!(report.output_after_seq, seq);
    assert_eq!(fixture.durable_cursor_seq(), seq);

    let key = fixture.belief_key("node-a");
    let view = fixture.store.current_view(&key).unwrap().unwrap();
    assert!(view.planner_projection.confidence > 0.0);
}

#[test]
fn not_applicable_records_advance_the_cursor_without_domain_writes() {
    let fixture = Fixture::open();
    fixture.append(unrelated_envelope("execution.task.started"));
    let seq = fixture.append(unrelated_envelope("execution.task.failed"));
    let mut actor = fixture.actor();

    let report = actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });

    assert_eq!(report.events_replayed, 2);
    assert_eq!(report.not_applicable_count, 2);
    assert_eq!(report.applicable_count, 0);
    assert_eq!(report.invalid_count, 0);
    assert_eq!(report.new_assignment_count, 0);
    assert_eq!(fixture.durable_cursor_seq(), seq);
}

#[test]
fn every_step_that_absorbs_nothing_declares_what_it_waits_on() {
    use meld_world_model::waiting::conditions;

    let fixture = Fixture::open();
    let mut actor = fixture.actor();

    // A quiet ledger declares itself on the empty page, not only after a
    // replayed-but-unmapped window.
    let quiet = actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });
    assert_eq!(quiet.events_replayed, 0);
    assert_eq!(quiet.waiting_on.len(), 1);
    assert_eq!(
        quiet.waiting_on[0].condition,
        conditions::LEDGER_QUIET_PAST_CURSOR
    );

    // A window where every record falls outside the installed mapping
    // waits on a vocabulary intersection.
    fixture.append(unrelated_envelope("execution.task.started"));
    let unmapped = actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });
    assert_eq!(unmapped.events_replayed, 1);
    assert_eq!(unmapped.waiting_on.len(), 1);
    assert_eq!(
        unmapped.waiting_on[0].condition,
        conditions::NO_MAPPABLE_EVENTS
    );

    // Once the window is absorbed, the following quiet step is quiet again.
    let after = actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });
    assert_eq!(
        after.waiting_on[0].condition,
        conditions::LEDGER_QUIET_PAST_CURSOR
    );
}

#[test]
fn invalid_records_are_rejected_durably_before_the_cursor_advances() {
    let fixture = Fixture::open();
    // Matched publication without any subject object: unusable content.
    let seq = fixture.append(success_envelope("publication-a", None));
    let mut actor = fixture.actor();

    let report = actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });

    assert_eq!(report.invalid_count, 1);
    assert_eq!(report.recorded_rejection_ids.len(), 1);
    assert_eq!(fixture.durable_cursor_seq(), seq);
    let rejection = fixture
        .store
        .get_rejection(&report.recorded_rejection_ids[0])
        .unwrap()
        .unwrap();
    assert_eq!(rejection.source_id, "publication-a");
    assert_eq!(rejection.source_cursor_start, seq);
}

#[test]
fn bounded_batches_respect_the_budget_and_resume_from_the_cursor() {
    let fixture = Fixture::open();
    let mut seqs = Vec::new();
    for index in 0..3 {
        seqs.push(fixture.append(success_envelope(
            &format!("publication-{index}"),
            Some("node-a"),
        )));
    }
    let mut actor = fixture.actor();

    let first = actor.bounded_step(&EvidenceIngestionRequest { max_events: 2 });
    assert_eq!(first.events_replayed, 2);
    assert!(first.more_available);
    assert_eq!(first.output_after_seq, seqs[1]);
    assert_eq!(fixture.durable_cursor_seq(), seqs[1]);

    let second = actor.bounded_step(&EvidenceIngestionRequest { max_events: 2 });
    assert_eq!(second.input_after_seq, seqs[1]);
    assert_eq!(second.events_replayed, 1);
    assert!(!second.more_available);
    assert_eq!(fixture.durable_cursor_seq(), seqs[2]);

    let idle = actor.bounded_step(&EvidenceIngestionRequest { max_events: 2 });
    assert_eq!(idle.events_replayed, 0);
    assert_eq!(idle.new_assignment_count, 0);
}

#[test]
fn reopen_between_evidence_commit_and_cursor_commit_replays_idempotently() {
    let fixture = Fixture::open();
    let seq = fixture.append(success_envelope("publication-a", Some("node-a")));
    let key = fixture.belief_key("node-a");

    // First run: evidence commits durably, then the injected crash loses the
    // cursor advancement.
    let failing_cursor = Arc::new(AdvanceFailingCursor {
        inner: fixture.authority.consumer_registry_capability(),
        failed_advances: AtomicUsize::new(0),
    });
    let mut crashed_actor = fixture.actor_with_cursor(failing_cursor.clone());
    let crashed = crashed_actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });

    assert_eq!(crashed.applicable_count, 1);
    assert!(crashed.new_assignment_count > 0);
    assert_eq!(crashed.revisions_committed, 1);
    assert_eq!(failing_cursor.failed_advances.load(Ordering::SeqCst), 1);
    assert!(crashed
        .retryable_errors
        .iter()
        .any(|issue| issue.code == "cursor_advance_failed"));
    // The cursor never advanced: the durable position still replays the batch.
    assert_eq!(fixture.durable_cursor_seq(), 0);

    let history_after_crash = fixture.store.revision_history(&key).unwrap();
    let view_after_crash = fixture.store.current_view(&key).unwrap().unwrap();

    // Reconstructed actor replays the same batch from the durable cursor.
    let mut reopened_actor = fixture.actor();
    let replayed = reopened_actor.bounded_step(&EvidenceIngestionRequest { max_events: 16 });

    assert_eq!(replayed.input_after_seq, 0);
    assert_eq!(replayed.events_replayed, 1);
    assert_eq!(replayed.applicable_count, 1);
    // Identity-keyed deduplication: no second assignment, revision, or
    // confidence increase from the replayed publication.
    assert_eq!(replayed.new_assignment_count, 0);
    assert_eq!(replayed.revisions_committed, 0);
    assert_eq!(fixture.durable_cursor_seq(), seq);

    let history_after_replay = fixture.store.revision_history(&key).unwrap();
    let view_after_replay = fixture.store.current_view(&key).unwrap().unwrap();
    assert_eq!(history_after_replay.len(), history_after_crash.len());
    assert_eq!(
        view_after_replay.planner_projection.confidence,
        view_after_crash.planner_projection.confidence
    );
    assert_eq!(
        view_after_replay.current_revision_id,
        view_after_crash.current_revision_id
    );
}

#[test]
fn zero_budget_is_a_fatal_request_error() {
    let fixture = Fixture::open();
    let mut actor = fixture.actor();
    let report = actor.bounded_step(&EvidenceIngestionRequest { max_events: 0 });
    assert!(report
        .fatal_errors
        .iter()
        .any(|issue| issue.code == "invalid_budget"));
}
