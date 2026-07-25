//! Stewardship outcome interpretation: the docs freshness theory as data.
//!
//! These tests prove the committed theory files under
//! `theory/docs_freshness/` (repository root, sibling to the `workflows/`
//! authored surface: theory bodies are stewardship-package content resolved
//! by identity at initialization, not user configuration and not belief
//! code) against the real belief machinery: the config-driven outcome
//! mapping set, promoted-evidence ingestion, the durable evidence-consumer
//! actor, and the weighted Bayesian comparator. Docs vocabulary appears
//! only in the committed theory data and these fixtures.
//!
//! The requirements-gate decision under proof: per-folder evidence facts
//! persist on the ledger, and one derived aggregate belief computed by
//! belief policy over those facts carries the selected-tree question;
//! per-folder outputs must never satisfy the selected-tree goal before
//! aggregate package completion.
//!
//! Comparator math that guarantees no premature satisfaction. The family
//! posterior means `stale_probability`, planner confidence is
//! `1 - posterior`, and satisfaction is `confidence >= threshold` with
//! threshold 0.6. Each ingested record is assessed in its own window, so
//! every revision computes `p' = (p + e) / 2` where `e` is the record's
//! evidence value: folder success asserts 0.5, a failed aggregate asserts
//! 0.75, a completed aggregate asserts 0.0. With prior 0.75, any sequence
//! of folder and failed-aggregate evidence — in any count and any window
//! batching, since a window's value is a weighted mean of members in
//! `{0.5, 0.75}` — keeps the posterior inside `(0.5, 0.75]`, so confidence
//! stays strictly below 0.5 < 0.6. Worst-case per-folder-only confidence is
//! therefore bounded by 0.5 and approached only asymptotically
//! (`confidence_n = 0.5 - 0.25 * 2^-n`). A completed aggregate assessed in
//! its own window maps any reachable posterior `p <= 0.75` to `p/2 <=
//! 0.375`, so confidence is at least 0.625 >= 0.6: completion always
//! crosses, and nothing else can. The required `aggregate_completion`
//! schema additionally holds the belief at `NeedsObservation` with
//! uncertainty 0.9 until either aggregate event arrives.
//!
//! Fixture envelopes replicate execution's real construction byte for byte:
//! per-task publications follow `build_publication_envelope` in
//! `crates/meld-execution/src/task_network/publication.rs` (payload is
//! `dispatch::Outcome`; the envelope carries no workspace-node object, so
//! the folder subject is minted from the payload's first task event, which
//! the executor constructor always emits with the run's target node — see
//! `crates/meld-execution/src/task/executor.rs`), and aggregates follow
//! `build_aggregate_envelope` in
//! `crates/meld-execution/src/task_network/aggregate_publication.rs`
//! (payload is `AggregatePackageOutcome`, whose `selected_scope` is carried
//! precisely so world-model mapping needs no caller reattachment).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use meld_world_model::belief::outcome::interpretation::{
    ConfiguredOutcomeMappingSet, OutcomeMappingConfig, OutcomeMappingSetConfig,
    OutcomeSubjectSource,
};
use meld_world_model::belief::{
    promoted_evidence_identity, BeliefConfigLoader, BeliefFamilyRegistry,
    BeliefFamilyRegistryStore, BeliefKey, BeliefStatus, BeliefStore, BranchScope,
    EvidenceEventReplaySource, EvidenceIngestionActor, EvidenceIngestionRequest, EvidenceValue,
    ObservationReason, OutcomeEvidenceMapping, OutcomeMappingDisposition, OutcomeMappingInput,
};
use meld_world_model::events::error::EventAuthorityError;
use meld_world_model::events::{
    AppendMode, ConsumerCursorError, ConsumerCursorState, DomainObjectRef, DurableConsumerCursor,
    EventAuthority, EventAuthorityOpenOptions, EventEnvelope, EventPage, EventRecord,
    EventRelation, EventReplayCapability, LedgerIdentity, ReplayRequest,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::PerspectiveKey;
use serde_json::{json, Value};

/// Committed theory files under proof; a missing file fails compilation.
const FAMILY_JSON: &str =
    include_str!("../../../theory/docs_freshness/belief_family.docs_freshness.json");
const INTERPRETATION_JSON: &str =
    include_str!("../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json");

const FAMILY_ID: &str = "docs_freshness";
const SET_MAPPING_ID: &str = "docs_freshness_outcome_interpretation_v1";
const NETWORK_ID: &str = "network-docs";
const SCOPE_NODE_ID: &str = "root";
const SESSION_ID: &str = "session-docs";

/// Evidence values authored in the theory files, restated for the math.
const PRIOR: f64 = 0.75;
const THRESHOLD: f64 = 0.6;
const FOLDER_SIGNAL: f64 = 0.5;
const COMPLETED_SIGNAL: f64 = 0.0;
const FAILED_SIGNAL: f64 = 0.75;

fn object(domain_id: &str, kind: &str, id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, kind, id).unwrap()
}

fn exec_object(kind: &str, id: &str) -> DomainObjectRef {
    object("execution", kind, id)
}

fn relation(relation_type: &str, src: DomainObjectRef, dst: DomainObjectRef) -> EventRelation {
    EventRelation::new(relation_type, src, dst).unwrap()
}

fn interpretation_set() -> ConfiguredOutcomeMappingSet {
    let config: OutcomeMappingSetConfig = serde_json::from_str(INTERPRETATION_JSON).unwrap();
    ConfiguredOutcomeMappingSet::new(config).unwrap()
}

/// One task event exactly as the executor serializes it. The first event of
/// every run is the constructor's `task_requested` carrying the run's target
/// node (`crates/meld-execution/src/task/executor.rs`), which the theory's
/// `/task_events/0/target_node_id` subject pointer relies on.
fn task_event(event_type: &str, task_instance_id: &str, folder_node_id: &str) -> Value {
    json!({
        "event_type": event_type,
        "task_id": "task_docs_writer",
        "task_run_id": task_instance_id,
        "capability_instance_id": null,
        "invocation_id": null,
        "target_node_id": folder_node_id,
        "artifact_id": null,
        "artifact_type_id": null,
        "attempt_index": null,
        "ready_count": null,
        "running_count": null,
        "blocked_reason": null,
        "error": null
    })
}

/// `dispatch::Outcome` payload shape, pinned to
/// `crates/meld-execution/src/task_network/dispatch.rs`.
fn folder_outcome_payload(suffix: &str, folder_node_id: &str, status: &str) -> Value {
    json!({
        "outcome_id": format!("outcome-folder-{suffix}"),
        "task_instance_id": format!("task-folder-{suffix}"),
        "lifecycle_epoch": 1,
        "claim_id": format!("claim-folder-{suffix}"),
        "claim_revision": 1,
        "status": status,
        "error": if status == "Succeeded" { Value::Null } else { json!("provider turn failed") },
        "artifact_records": [{
            "artifact_id": format!("artifact-readme-{suffix}"),
            "artifact_type_id": "readme_summary",
            "schema_version": 1,
            "content": { "path": format!("{folder_node_id}/README.md") },
            "producer": {
                "task_id": "task_docs_writer",
                "capability_instance_id": format!("write-{suffix}"),
                "invocation_id": format!("invocation-{suffix}"),
                "output_slot_id": "readme"
            }
        }],
        "task_events": [
            task_event("task_requested", &format!("task-folder-{suffix}"), folder_node_id),
            task_event("task_succeeded", &format!("task-folder-{suffix}"), folder_node_id)
        ]
    })
}

/// Per-task publication envelope replicated from
/// `build_publication_envelope` in
/// `crates/meld-execution/src/task_network/publication.rs`: same payload,
/// stream, record id, object order, and relation order. Deliberately no
/// workspace-node object — that fidelity is what forces the payload-minted
/// subject in the interpretation theory.
fn folder_publication_envelope(suffix: &str, folder_node_id: &str, status: &str) -> EventEnvelope {
    let task_instance_id = format!("task-folder-{suffix}");
    let publication_id = format!("publication-folder-{suffix}");
    let event_type = if status == "Succeeded" {
        "execution.task.succeeded"
    } else {
        "execution.task.failed"
    };
    let network = exec_object("task_network", NETWORK_ID);
    let task_run = exec_object("task_run", &task_instance_id);
    let outcome = exec_object("task_outcome", &format!("outcome-folder-{suffix}"));
    let publication = exec_object("task_publication", &publication_id);
    let artifact = exec_object("artifact", &format!("artifact-readme-{suffix}"));
    let artifact_slot = exec_object(
        "artifact_slot",
        &format!("{task_instance_id}::readme_summary"),
    );
    EventEnvelope::new_domain(
        "2026-07-25T00:00:00Z".to_string(),
        SESSION_ID,
        "execution",
        format!("task_network::{NETWORK_ID}::task::{task_instance_id}"),
        event_type,
        None,
        folder_outcome_payload(suffix, folder_node_id, status),
    )
    .with_record_id(format!(
        "execution::task_network_publication::{publication_id}"
    ))
    .with_graph(
        vec![
            network.clone(),
            task_run.clone(),
            outcome.clone(),
            publication.clone(),
            artifact.clone(),
            artifact_slot.clone(),
        ],
        vec![
            relation("published_from", publication, outcome.clone()),
            relation("produced_by", outcome, task_run.clone()),
            relation("member_of", task_run.clone(), network),
            relation("attached_to", artifact_slot.clone(), task_run),
            relation("selected", artifact_slot, artifact),
        ],
    )
}

/// Aggregate package envelope replicated from `build_aggregate_envelope` in
/// `crates/meld-execution/src/task_network/aggregate_publication.rs`:
/// payload is the intact `AggregatePackageOutcome`, and the object graph
/// names the selected scope and every folder under the same workspace-node
/// vocabulary — which is exactly why the theory binds the subject from the
/// payload's `selected_scope` rather than from envelope objects.
fn aggregate_envelope(run_suffix: &str, status: &str, folder_node_ids: &[&str]) -> EventEnvelope {
    let package_run_id = format!("package-run-{run_suffix}");
    let aggregate_id = format!("aggregate-package-run-{run_suffix}");
    let terminal_outcome_id = format!("outcome-package-run-{run_suffix}");
    let scope = object("workspace_fs", "node", SCOPE_NODE_ID);
    let event_type = match status {
        "Completed" => "execution.package.completed",
        _ => "execution.package.failed",
    };
    let folder_results: Vec<Value> = folder_node_ids
        .iter()
        .map(|node_id| {
            json!({
                "folder": { "domain_id": "workspace_fs", "object_kind": "node", "object_id": node_id },
                "task_instance_id": format!("task-package-run-{run_suffix}"),
                "outcome_id": terminal_outcome_id,
                "artifact_ids": [format!("artifact-readme-{}", node_id.replace('/', "-"))]
            })
        })
        .collect();
    let payload = json!({
        "aggregate_id": aggregate_id,
        "package_run_id": package_run_id,
        "network_id": NETWORK_ID,
        "selected_scope": {
            "domain_id": "workspace_fs",
            "object_kind": "node",
            "object_id": SCOPE_NODE_ID
        },
        "status": status,
        "folder_results": folder_results
    });

    let aggregate_object = exec_object("package_aggregate", &aggregate_id);
    let network = exec_object("task_network", NETWORK_ID);
    let mut objects = vec![aggregate_object.clone(), network.clone(), scope.clone()];
    let mut relations = vec![
        relation("member_of", aggregate_object.clone(), network),
        relation("targets", aggregate_object.clone(), scope),
    ];
    for node_id in folder_node_ids {
        let folder = object("workspace_fs", "node", node_id);
        objects.push(folder.clone());
        relations.push(relation("covers", aggregate_object.clone(), folder));
    }
    // Folder results of one run share the terminal outcome; the graph names
    // each distinct outcome once, mirroring the source construction.
    if !folder_node_ids.is_empty() {
        let outcome = exec_object("task_outcome", &terminal_outcome_id);
        objects.push(outcome.clone());
        relations.push(relation("published_from", aggregate_object, outcome));
    }
    EventEnvelope::new_domain(
        "2026-07-25T00:00:00Z".to_string(),
        SESSION_ID,
        "execution",
        format!("task_network::{NETWORK_ID}::package::{package_run_id}"),
        event_type,
        None,
        payload,
    )
    .with_record_id(format!("execution::task_network_aggregate::{aggregate_id}"))
    .with_graph(objects, relations)
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
        // Install the committed family theory exactly as authored.
        let config = BeliefConfigLoader::load_json(FAMILY_JSON).unwrap().config;
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
            Arc::new(interpretation_set()),
            SET_MAPPING_ID,
            PerspectiveKey::new("default", "default").unwrap(),
            BranchScope::main(),
        )
    }

    fn step(&self) -> meld_world_model::belief::EvidenceIngestionReport {
        let mut actor = self.actor();
        let report = actor.bounded_step(&EvidenceIngestionRequest { max_events: 64 });
        assert!(report.retryable_errors.is_empty(), "{report:?}");
        assert!(report.fatal_errors.is_empty(), "{report:?}");
        report
    }

    /// The one selected-tree belief key the theory aggregates into.
    fn tree_key(&self) -> BeliefKey {
        BeliefKey {
            subject: object("workspace_fs", "node", SCOPE_NODE_ID),
            dimension_id: "docs_freshness".to_string(),
            predicate_id: "confidence".to_string(),
            perspective: PerspectiveKey::new("default", "default").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "default_policy".to_string(),
        }
    }

    fn tree_confidence(&self) -> f64 {
        self.store
            .current_view(&self.tree_key())
            .unwrap()
            .unwrap()
            .planner_projection
            .confidence
    }
}

/// Expected posterior when every listed signal is assessed in its own
/// window, mirroring the real per-record ingestion path: `p' = (p + e) / 2`.
fn chained_posterior(signals: &[f64]) -> f64 {
    signals.iter().fold(PRIOR, |p, e| (p + e) / 2.0)
}

fn record(envelope: EventEnvelope, seq: u64) -> OutcomeMappingInput {
    OutcomeMappingInput {
        record: EventRecord::from_envelope(envelope, seq),
        mapping_id: SET_MAPPING_ID.to_string(),
    }
}

#[test]
fn committed_theory_files_parse_and_round_trip() {
    // Family theory: parses through the runtime loader and round-trips
    // field for field, so the committed file is exactly what installs.
    let family = BeliefConfigLoader::load_json(FAMILY_JSON).unwrap();
    assert_eq!(family.config.family_id, FAMILY_ID);
    assert_eq!(family.config.planner_projection.threshold, THRESHOLD);
    assert_eq!(family.config.default_prior, PRIOR);
    let raw: Value = serde_json::from_str(FAMILY_JSON).unwrap();
    assert_eq!(serde_json::to_value(&family.config).unwrap(), raw);

    // Interpretation theory: parses into the mapping set vocabulary,
    // validates, and round-trips field for field.
    let set_config: OutcomeMappingSetConfig = serde_json::from_str(INTERPRETATION_JSON).unwrap();
    let raw: Value = serde_json::from_str(INTERPRETATION_JSON).unwrap();
    assert_eq!(serde_json::to_value(&set_config).unwrap(), raw);
    let set = ConfiguredOutcomeMappingSet::new(set_config).unwrap();
    assert_eq!(set.mapping_id(), SET_MAPPING_ID);
}

#[test]
fn fixture_envelopes_preserve_the_real_publication_shapes() {
    // The per-task publication envelope carries no workspace-node object;
    // the folder subject exists only inside the outcome payload. This pins
    // the fidelity fact that motivates the payload-minted subject source.
    let publication = folder_publication_envelope("a", "root/a", "Succeeded");
    assert!(publication
        .objects
        .iter()
        .all(|object| object.domain_id != "workspace_fs"));
    assert_eq!(
        publication
            .data
            .pointer("/task_events/0/target_node_id")
            .and_then(Value::as_str),
        Some("root/a")
    );

    // The aggregate envelope names the selected scope and each folder under
    // the same workspace-node vocabulary, so envelope-object binding would
    // be ambiguous; the payload's own selected_scope is the authority.
    let aggregate = aggregate_envelope("1", "Completed", &["root/a", "root/b"]);
    let node_count = aggregate
        .objects
        .iter()
        .filter(|object| object.domain_id == "workspace_fs" && object.object_kind == "node")
        .count();
    assert_eq!(node_count, 3);
    assert_eq!(
        aggregate
            .data
            .pointer("/selected_scope/object_id")
            .and_then(Value::as_str),
        Some(SCOPE_NODE_ID)
    );
}

#[test]
fn folder_success_maps_to_a_per_folder_fact_bound_to_the_selected_tree() {
    let set = interpretation_set();
    let input = record(folder_publication_envelope("a", "root/a", "Succeeded"), 3);

    let OutcomeMappingDisposition::Applicable {
        evidence_id,
        record: promoted,
    } = set.map_outcome(&input)
    else {
        panic!("expected an applicable disposition");
    };

    // Evidence identity is frozen over the publication record id and the
    // installed set identity, never a member rule label.
    assert_eq!(
        evidence_id,
        promoted_evidence_identity(
            "execution::task_network_publication::publication-folder-a",
            SET_MAPPING_ID
        )
    );
    assert_eq!(promoted.source_id, evidence_id);
    assert_eq!(promoted.source_kind, "execution_folder_task_outcome");
    // The durable fact is subject-bound to its folder...
    assert_eq!(promoted.subject, object("workspace_fs", "node", "root/a"));
    // ...while the theory's installed constant names the selected-tree
    // question the fact bears on.
    let mut scope = std::collections::BTreeMap::new();
    scope.insert("domain_id".to_string(), "workspace_fs".to_string());
    scope.insert("object_kind".to_string(), "node".to_string());
    scope.insert("object_id".to_string(), SCOPE_NODE_ID.to_string());
    assert_eq!(
        promoted.fields.get("selected_scope"),
        Some(&EvidenceValue::Map(scope))
    );
    assert_eq!(
        promoted.fields.get("tree_stale_signal"),
        Some(&EvidenceValue::Scalar(FOLDER_SIGNAL))
    );
}

#[test]
fn aggregate_events_map_to_scope_subject_evidence_with_truthful_signals() {
    let set = interpretation_set();

    let completed = set.map_outcome(&record(
        aggregate_envelope("1", "Completed", &["root/a", "root/b"]),
        7,
    ));
    let OutcomeMappingDisposition::Applicable {
        record: promoted, ..
    } = completed
    else {
        panic!("expected an applicable completed disposition");
    };
    assert_eq!(promoted.source_kind, "execution_package_aggregate");
    assert_eq!(
        promoted.subject,
        object("workspace_fs", "node", SCOPE_NODE_ID)
    );
    assert_eq!(
        promoted.fields.get("tree_stale_signal"),
        Some(&EvidenceValue::Scalar(COMPLETED_SIGNAL))
    );

    // A failed aggregate is evidence, not silence: it restores the prior's
    // staleness (0.75) so confidence can never rise from it, while keeping
    // the required aggregate schema truthful about the run having finished.
    let failed = set.map_outcome(&record(aggregate_envelope("2", "Failed", &["root/a"]), 8));
    let OutcomeMappingDisposition::Applicable {
        record: promoted, ..
    } = failed
    else {
        panic!("expected an applicable failed disposition");
    };
    assert_eq!(
        promoted.subject,
        object("workspace_fs", "node", SCOPE_NODE_ID)
    );
    assert_eq!(
        promoted.fields.get("tree_stale_signal"),
        Some(&EvidenceValue::Scalar(FAILED_SIGNAL))
    );
}

#[test]
fn set_dispositions_understand_or_reject_truthfully() {
    let set = interpretation_set();

    // A failed folder task is understood and carries no tree evidence; the
    // failed package aggregate is the negative signal bearer.
    let failed_task = set.map_outcome(&record(
        folder_publication_envelope("a", "root/a", "Failed"),
        1,
    ));
    assert!(matches!(
        failed_task,
        OutcomeMappingDisposition::NotApplicable { .. }
    ));

    // Event type and payload status disagreeing is not evidence.
    let mut disagreeing = folder_publication_envelope("b", "root/b", "Succeeded");
    disagreeing.data["status"] = json!("Failed");
    assert!(matches!(
        set.map_outcome(&record(disagreeing, 2)),
        OutcomeMappingDisposition::NotApplicable { .. }
    ));

    // A matched publication without a record id cannot derive replay-stable
    // evidence identity.
    let mut no_record_id = folder_publication_envelope("c", "root/c", "Succeeded");
    no_record_id.record_id = None;
    assert!(matches!(
        set.map_outcome(&record(no_record_id, 3)),
        OutcomeMappingDisposition::Invalid { .. }
    ));

    // A matched publication whose payload lost its target node cannot bind
    // a folder subject.
    let mut no_target = folder_publication_envelope("d", "root/d", "Succeeded");
    no_target.data["task_events"] = json!([]);
    assert!(matches!(
        set.map_outcome(&record(no_target, 4)),
        OutcomeMappingDisposition::Invalid { .. }
    ));

    // An aggregate whose declared scope drifts from the theory's subject
    // vocabulary is unusable content, not a guess.
    let mut drifted_scope = aggregate_envelope("9", "Completed", &["root/a"]);
    drifted_scope.data["selected_scope"]["object_kind"] = json!("tree");
    assert!(matches!(
        set.map_outcome(&record(drifted_scope, 5)),
        OutcomeMappingDisposition::Invalid { .. }
    ));

    // A selection/installation mismatch is recorded, never reinterpreted.
    let mismatched = OutcomeMappingInput {
        record: EventRecord::from_envelope(
            folder_publication_envelope("e", "root/e", "Succeeded"),
            6,
        ),
        mapping_id: "some_other_mapping".to_string(),
    };
    assert!(matches!(
        set.map_outcome(&mismatched),
        OutcomeMappingDisposition::Invalid { .. }
    ));
}

#[test]
fn mapping_set_vocabulary_rejects_unusable_theory() {
    let parsed: OutcomeMappingSetConfig = serde_json::from_str(INTERPRETATION_JSON).unwrap();

    let empty = OutcomeMappingSetConfig {
        mapping_id: SET_MAPPING_ID.to_string(),
        rules: Vec::new(),
    };
    assert!(ConfiguredOutcomeMappingSet::new(empty).is_err());

    let mut duplicate = parsed.clone();
    duplicate.rules.push(parsed.rules[0].clone());
    assert!(ConfiguredOutcomeMappingSet::new(duplicate).is_err());

    // Payload text supplies only the object id, so the binding must carry
    // the owning domain.
    let mut missing_domain = parsed.clone();
    missing_domain.rules[0].subject.domain_id = None;
    assert!(matches!(
        missing_domain.rules[0].subject.from,
        OutcomeSubjectSource::PayloadObjectId { .. }
    ));
    assert!(ConfiguredOutcomeMappingSet::new(missing_domain).is_err());

    // An installed constant object must be a constructible reference.
    let mut empty_constant = parsed;
    let broken: OutcomeMappingConfig = serde_json::from_value(json!({
        "mapping_id": "broken_constant",
        "source_kind": "execution_folder_task_outcome",
        "match_domain_id": "execution",
        "match_event_type": "execution.task.succeeded",
        "match_content": [],
        "subject": {
            "object_kind": "node",
            "domain_id": "workspace_fs",
            "from": { "source": "payload_object_id", "pointer": "/task_events/0/target_node_id" }
        },
        "evidence_fields": [{
            "field": "selected_scope",
            "source": { "source": "constant_object", "domain_id": "workspace_fs", "object_kind": "node", "object_id": "" }
        }]
    }))
    .unwrap();
    empty_constant.rules = vec![broken];
    assert!(ConfiguredOutcomeMappingSet::new(empty_constant).is_err());
}

#[test]
fn folder_facts_alone_never_reach_satisfaction_for_any_count() {
    let fixture = Fixture::open();
    let folders = ["root/a", "root/b", "root/c", "root/d", "root/e", "root/f"];

    let mut signals = Vec::new();
    for (index, folder) in folders.iter().enumerate() {
        fixture.append(folder_publication_envelope(
            &format!("{index}"),
            folder,
            "Succeeded",
        ));
        let report = fixture.step();
        assert_eq!(report.applicable_count, 1);
        assert_eq!(report.revisions_committed, 1);

        signals.push(FOLDER_SIGNAL);
        let confidence = fixture.tree_confidence();
        // Exact chained-prior value: confidence_n = 0.5 - 0.25 * 2^-n.
        assert_eq!(confidence, 1.0 - chained_posterior(&signals));
        // The provable bound: folder facts alone can never reach 0.5, and
        // the satisfaction threshold sits at 0.6 beyond it.
        assert!(confidence < 0.5);
        assert!(confidence < THRESHOLD);
    }

    // Belief policy also names the gate: the required aggregate-completion
    // schema is missing, so the belief is explicitly unsettled and asks for
    // exactly that evidence.
    let view = fixture
        .store
        .current_view(&fixture.tree_key())
        .unwrap()
        .unwrap();
    assert_eq!(view.status, BeliefStatus::NeedsObservation);
    assert_eq!(view.uncertainty, 0.9);
    let observation = view.observation.unwrap();
    assert_eq!(
        observation.target_evidence_schema_id,
        "aggregate_completion"
    );
    assert_eq!(
        observation.reason,
        ObservationReason::MissingRequiredEvidence
    );
}

#[test]
fn completed_aggregate_evidence_crosses_the_satisfaction_threshold() {
    let fixture = Fixture::open();
    let folders = ["root/a", "root/b", "root/c"];
    for (index, folder) in folders.iter().enumerate() {
        fixture.append(folder_publication_envelope(
            &format!("{index}"),
            folder,
            "Succeeded",
        ));
    }
    fixture.append(aggregate_envelope("1", "Completed", &folders));

    let report = fixture.step();
    assert_eq!(report.applicable_count, 4);
    assert_eq!(report.revisions_committed, 4);

    let view = fixture
        .store
        .current_view(&fixture.tree_key())
        .unwrap()
        .unwrap();
    let expected = 1.0
        - chained_posterior(&[
            FOLDER_SIGNAL,
            FOLDER_SIGNAL,
            FOLDER_SIGNAL,
            COMPLETED_SIGNAL,
        ]);
    assert_eq!(view.planner_projection.confidence, expected);
    assert_eq!(view.planner_projection.confidence, 0.734375);
    assert!(view.planner_projection.confidence >= view.planner_projection.threshold);
    // The required aggregate evidence arrived, so the question settles.
    assert_eq!(view.status, BeliefStatus::Settled);
}

#[test]
fn failed_aggregate_evidence_never_crosses_the_threshold() {
    let fixture = Fixture::open();
    for (index, folder) in ["root/a", "root/b", "root/c"].iter().enumerate() {
        fixture.append(folder_publication_envelope(
            &format!("{index}"),
            folder,
            "Succeeded",
        ));
    }
    fixture.append(aggregate_envelope("1", "Failed", &["root/a"]));
    fixture.step();

    let view = fixture
        .store
        .current_view(&fixture.tree_key())
        .unwrap()
        .unwrap();
    let expected =
        1.0 - chained_posterior(&[FOLDER_SIGNAL, FOLDER_SIGNAL, FOLDER_SIGNAL, FAILED_SIGNAL]);
    assert_eq!(view.planner_projection.confidence, expected);
    assert_eq!(view.planner_projection.confidence, 0.359375);
    assert!(view.planner_projection.confidence < THRESHOLD);
    // The failed run settles the question truthfully at low confidence
    // rather than leaving it forever waiting for observation.
    assert_eq!(view.status, BeliefStatus::Settled);

    // A retried run's folder facts plus a second failure still cannot reach
    // satisfaction: every non-completion signal is at least 0.5, so the
    // posterior never leaves (0.5, 0.75] and confidence stays below 0.5.
    for (index, folder) in ["root/a", "root/b", "root/c"].iter().enumerate() {
        fixture.append(folder_publication_envelope(
            &format!("retry-{index}"),
            folder,
            "Succeeded",
        ));
        fixture.step();
        assert!(fixture.tree_confidence() < 0.5);
    }
    fixture.append(aggregate_envelope("2", "Failed", &["root/a"]));
    fixture.step();
    assert!(fixture.tree_confidence() < 0.5);
    assert!(fixture.tree_confidence() < THRESHOLD);
}

#[test]
fn replayed_publications_change_no_belief_state() {
    let fixture = Fixture::open();
    let folders = ["root/a", "root/b", "root/c"];
    for (index, folder) in folders.iter().enumerate() {
        fixture.append(folder_publication_envelope(
            &format!("{index}"),
            folder,
            "Succeeded",
        ));
    }
    let last_seq = fixture.append(aggregate_envelope("1", "Completed", &folders));
    let key = fixture.tree_key();

    // First run: evidence commits durably, then the injected crash loses
    // the cursor advancement.
    let failing_cursor = Arc::new(AdvanceFailingCursor {
        inner: fixture.authority.consumer_registry_capability(),
        failed_advances: AtomicUsize::new(0),
    });
    let mut crashed_actor = fixture.actor_with_cursor(failing_cursor.clone());
    let crashed = crashed_actor.bounded_step(&EvidenceIngestionRequest { max_events: 64 });
    assert_eq!(crashed.applicable_count, 4);
    assert_eq!(crashed.revisions_committed, 4);
    assert_eq!(failing_cursor.failed_advances.load(Ordering::SeqCst), 1);

    let history_after_crash = fixture.store.revision_history(&key).unwrap();
    let view_after_crash = fixture.store.current_view(&key).unwrap().unwrap();

    // Reopened actor replays the same batch from the durable cursor.
    // Identity-keyed deduplication holds through the authored theory: no
    // new assignments, no new revisions, identical confidence.
    let replayed = fixture.step();
    assert_eq!(replayed.events_replayed, 4);
    assert_eq!(replayed.applicable_count, 4);
    assert_eq!(replayed.new_assignment_count, 0);
    assert_eq!(replayed.revisions_committed, 0);
    assert_eq!(replayed.output_after_seq, last_seq);

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
