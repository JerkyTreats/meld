//! Bounded package-step behavior over the durable task executor.
//!
//! These tests prove the four contract invariants: budget enforcement, result
//! parity with the execute-to-completion path over one branching fixture,
//! reopen-resume between steps, and no-work steps after completion.

use async_trait::async_trait;
use futures::executor::block_on;
use meld_events::EventEnvelope;
use meld_execution::capability::{
    BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityCatalog,
    CapabilityInvocationPayload, CapabilityInvocationResult,
};
use meld_execution::error::ExecutionInvariantError;
use meld_execution::execution::{EventPublicationPort, ExecutionEventContext};
use meld_execution::task::{
    execute_task_to_completion, package_step_repo_id, ArtifactProducerRef, ArtifactRecord,
    CompiledTaskDelta, CompiledTaskRecord, DurablePackageExecution, InitArtifactValue,
    PackageStepInvoker, TaskArtifactRepo, TaskDependencyEdge, TaskDependencyKind, TaskExecutor,
    TaskExpansionRequest, TaskInitSlotSpec, TaskInitializationPayload, TaskRunContext,
    TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
};
use meld_execution::task_network::package_step::{PackageStep, PackageStepRequest};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

type ApiError = ExecutionInvariantError;

fn open_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn request(max_ready_invocations: usize) -> PackageStepRequest {
    PackageStepRequest {
        max_ready_invocations,
    }
}

fn bare_instance(
    capability_instance_id: &str,
    input_wiring: Vec<BoundInputWiring>,
) -> BoundCapabilityInstance {
    BoundCapabilityInstance {
        capability_instance_id: capability_instance_id.to_string(),
        capability_type_id: "test_capability".to_string(),
        capability_version: 1,
        scope_ref: format!("node_{capability_instance_id}"),
        scope_kind: "node".to_string(),
        binding_values: vec![],
        input_wiring,
    }
}

fn edge(from: &str, to: &str, kind: TaskDependencyKind) -> TaskDependencyEdge {
    TaskDependencyEdge {
        from_capability_instance_id: from.to_string(),
        to_capability_instance_id: to.to_string(),
        kind,
        reason: "fixture ordering".to_string(),
    }
}

/// Branching fixture: two sibling children fan out first, child_a's success
/// expands the graph with child_c, and the parent prepares only after every
/// child finalized (artifact edge from child_a, effect edges from b and c).
fn branching_task() -> CompiledTaskRecord {
    CompiledTaskRecord {
        task_id: "task_docs_writer".to_string(),
        task_version: 1,
        init_slots: vec![TaskInitSlotSpec {
            init_slot_id: "target_selector".to_string(),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
            required: true,
        }],
        capability_instances: vec![
            bare_instance(
                "capinst_child_a",
                vec![BoundInputWiring {
                    slot_id: "target".to_string(),
                    sources: vec![BoundInputWiringSource::TaskInitSlot {
                        init_slot_id: "target_selector".to_string(),
                        artifact_type_id: "target_selector".to_string(),
                        schema_version: 1,
                    }],
                }],
            ),
            bare_instance("capinst_child_b", vec![]),
            bare_instance(
                "capinst_parent",
                vec![BoundInputWiring {
                    slot_id: "summary".to_string(),
                    sources: vec![BoundInputWiringSource::UpstreamOutput {
                        capability_instance_id: "capinst_child_a".to_string(),
                        output_slot_id: "out".to_string(),
                        artifact_type_id: "readme_summary".to_string(),
                        schema_version: 1,
                    }],
                }],
            ),
        ],
        dependency_edges: vec![
            edge(
                "capinst_child_a",
                "capinst_parent",
                TaskDependencyKind::Artifact,
            ),
            edge(
                "capinst_child_b",
                "capinst_parent",
                TaskDependencyKind::Effect,
            ),
        ],
    }
}

fn sibling_task() -> CompiledTaskRecord {
    CompiledTaskRecord {
        task_id: "task_docs_writer".to_string(),
        task_version: 1,
        init_slots: vec![],
        capability_instances: vec![
            bare_instance("capinst_s1", vec![]),
            bare_instance("capinst_s2", vec![]),
            bare_instance("capinst_s3", vec![]),
        ],
        dependency_edges: vec![],
    }
}

fn run_payload(
    task_run_id: &str,
    init_artifacts: Vec<InitArtifactValue>,
) -> TaskInitializationPayload {
    TaskInitializationPayload {
        task_id: "task_docs_writer".to_string(),
        compiled_task_ref: "compiled_task_docs_writer".to_string(),
        init_artifacts,
        task_run_context: TaskRunContext {
            task_run_id: task_run_id.to_string(),
            session_id: Some("session_1".to_string()),
            trigger: "test".to_string(),
        },
    }
}

fn branching_payload(task_run_id: &str) -> TaskInitializationPayload {
    run_payload(
        task_run_id,
        vec![InitArtifactValue {
            init_slot_id: "target_selector".to_string(),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
            content: json!({ "path": "docs" }),
        }],
    )
}

// Shared scripted behavior so the bounded and completion paths execute the
// exact same fixture semantics.

fn out_artifact(capability_instance_id: &str, invocation_id: &str) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("{invocation_id}::out"),
        artifact_type_id: "readme_summary".to_string(),
        schema_version: 1,
        content: json!({ "summary": capability_instance_id }),
        producer: ArtifactProducerRef {
            task_id: "task_docs_writer".to_string(),
            capability_instance_id: capability_instance_id.to_string(),
            invocation_id: Some(invocation_id.to_string()),
            output_slot_id: Some("out".to_string()),
        },
    }
}

fn expansion_artifact(
    payload: &CapabilityInvocationPayload,
    content: serde_json::Value,
) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("{}::expansion", payload.invocation_id),
        artifact_type_id: TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID.to_string(),
        schema_version: TASK_EXPANSION_SCHEMA_VERSION,
        content,
        producer: ArtifactProducerRef {
            task_id: "task_docs_writer".to_string(),
            capability_instance_id: payload.capability_instance_id.clone(),
            invocation_id: Some(payload.invocation_id.clone()),
            output_slot_id: Some("expansion".to_string()),
        },
    }
}

fn scripted_artifacts(payload: &CapabilityInvocationPayload) -> Vec<ArtifactRecord> {
    let mut artifacts = vec![out_artifact(
        &payload.capability_instance_id,
        &payload.invocation_id,
    )];
    if payload.capability_instance_id == "capinst_child_a" {
        artifacts.push(expansion_artifact(
            payload,
            json!({
                "expansion_id": "expansion_child_c",
                "expansion_kind": "discover_children",
                "content": {}
            }),
        ));
    }
    artifacts
}

fn scripted_delta() -> CompiledTaskDelta {
    CompiledTaskDelta {
        init_slots: vec![],
        init_artifacts: vec![],
        capability_instances: vec![bare_instance("capinst_child_c", vec![])],
        dependency_edges: vec![edge(
            "capinst_child_c",
            "capinst_parent",
            TaskDependencyKind::Effect,
        )],
    }
}

struct ScriptedInvoker {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl PackageStepInvoker for ScriptedInvoker {
    async fn invoke_capability(
        &self,
        _instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        self.log.lock().unwrap().push(payload.invocation_id.clone());
        Ok(CapabilityInvocationResult {
            emitted_artifacts: scripted_artifacts(payload),
        })
    }

    fn compile_expansion(
        &self,
        _compiled_task: &CompiledTaskRecord,
        _request: &TaskExpansionRequest,
    ) -> Result<CompiledTaskDelta, ApiError> {
        Ok(scripted_delta())
    }
}

/// Emits a malformed expansion payload from `capinst_s2` on its first attempt
/// only, so one sibling's resolution fails mid-wave while its siblings commit.
struct MalformedExpansionOnFirstAttempt {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl PackageStepInvoker for MalformedExpansionOnFirstAttempt {
    async fn invoke_capability(
        &self,
        _instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        self.log.lock().unwrap().push(payload.invocation_id.clone());
        let mut emitted_artifacts = vec![out_artifact(
            &payload.capability_instance_id,
            &payload.invocation_id,
        )];
        if payload.invocation_id == "capinst_s2::attempt::1" {
            // Missing `expansion_kind` makes the expansion request undecodable.
            emitted_artifacts.push(expansion_artifact(
                payload,
                json!({ "expansion_id": "expansion_bad", "content": {} }),
            ));
        }
        Ok(CapabilityInvocationResult { emitted_artifacts })
    }

    fn compile_expansion(
        &self,
        _compiled_task: &CompiledTaskRecord,
        _request: &TaskExpansionRequest,
    ) -> Result<CompiledTaskDelta, ApiError> {
        Ok(CompiledTaskDelta::default())
    }
}

#[derive(Default)]
struct NoopApi;

impl EventPublicationPort for NoopApi {
    type Error = ExecutionInvariantError;
    type EventEnvelope = EventEnvelope;

    fn publish_execution_envelope(
        &self,
        _event_context: &ExecutionEventContext,
        _envelope: Self::EventEnvelope,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn invoke_scripted<'a>(
    _api: &'a NoopApi,
    log: &'a Mutex<Vec<String>>,
    _instance: &'a BoundCapabilityInstance,
    payload: &'a CapabilityInvocationPayload,
    _event_context: Option<&'a ExecutionEventContext>,
) -> Pin<
    Box<
        dyn Future<Output = Result<CapabilityInvocationResult, ExecutionInvariantError>>
            + Send
            + 'a,
    >,
> {
    Box::pin(async move {
        log.lock().unwrap().push(payload.invocation_id.clone());
        Ok(CapabilityInvocationResult {
            emitted_artifacts: scripted_artifacts(payload),
        })
    })
}

fn compile_scripted(
    _api: &NoopApi,
    _compiled_task: &CompiledTaskRecord,
    _request: &TaskExpansionRequest,
    _catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ExecutionInvariantError> {
    Ok(scripted_delta())
}

fn run_completion_path(task_run_id: &str) -> (TaskExecutor, Vec<String>) {
    let log = Mutex::new(Vec::new());
    let mut executor = TaskExecutor::new(
        branching_task(),
        branching_payload(task_run_id),
        "repo_completion",
    )
    .unwrap();
    block_on(execute_task_to_completion(
        &NoopApi,
        &mut executor,
        &CapabilityCatalog::new(),
        &log,
        invoke_scripted,
        compile_scripted,
        None,
        None,
    ))
    .unwrap();
    (executor, log.into_inner().unwrap())
}

fn artifacts_by_id(executor: &TaskExecutor) -> BTreeMap<String, ArtifactRecord> {
    executor
        .artifact_repo()
        .record()
        .artifacts
        .iter()
        .map(|artifact| (artifact.artifact_id.clone(), artifact.clone()))
        .collect()
}

fn sorted_invocation_ids(log: &[String]) -> BTreeSet<String> {
    log.iter().cloned().collect()
}

fn position(log: &[String], invocation_id: &str) -> usize {
    log.iter()
        .position(|entry| entry == invocation_id)
        .unwrap_or_else(|| panic!("missing invocation '{invocation_id}' in {log:?}"))
}

fn assert_dependency_order(log: &[String]) {
    let parent = position(log, "capinst_parent::attempt::1");
    for child in [
        "capinst_child_a::attempt::1",
        "capinst_child_b::attempt::1",
        "capinst_child_c::attempt::1",
    ] {
        assert!(
            position(log, child) < parent,
            "child '{child}' must finalize before parent preparation in {log:?}"
        );
    }
}

#[test]
fn one_step_never_releases_beyond_budget() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = DurablePackageExecution::open(
        open_db(),
        sibling_task(),
        run_payload("taskrun_budget", vec![]),
        ScriptedInvoker { log: log.clone() },
    )
    .unwrap();

    let first = block_on(stepper.step(&request(2))).unwrap();

    assert_eq!(first.items_attempted, 2);
    assert_eq!(first.items_committed, 2);
    assert!(first.budget_exhausted);
    assert!(!first.package_complete);
    assert_eq!(log.lock().unwrap().len(), 2);

    let second = block_on(stepper.step(&request(2))).unwrap();

    assert_eq!(second.items_attempted, 1);
    assert!(!second.budget_exhausted);
    assert!(second.package_complete);
    assert_eq!(log.lock().unwrap().len(), 3);
}

#[test]
fn zero_budget_step_attempts_no_work_and_reports_pending_readiness() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = DurablePackageExecution::open(
        open_db(),
        sibling_task(),
        run_payload("taskrun_zero", vec![]),
        ScriptedInvoker { log: log.clone() },
    )
    .unwrap();

    let report = block_on(stepper.step(&request(0))).unwrap();

    assert_eq!(report.items_attempted, 0);
    assert_eq!(report.items_committed, 0);
    assert!(report.budget_exhausted);
    assert!(!report.package_complete);
    assert_eq!(report.input_progress, report.output_progress);
    assert!(log.lock().unwrap().is_empty());
}

#[test]
fn bounded_multi_wave_run_matches_completion_path() {
    let (completion_executor, completion_log) = run_completion_path("taskrun_parity");

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = DurablePackageExecution::open(
        open_db(),
        branching_task(),
        branching_payload("taskrun_parity"),
        ScriptedInvoker { log: log.clone() },
    )
    .unwrap();
    let mut steps = 0;
    loop {
        let report = block_on(stepper.step(&request(1))).unwrap();
        assert!(report.items_attempted <= 1, "budget of one must hold");
        steps += 1;
        assert!(steps < 20, "bounded run must converge");
        if report.package_complete {
            break;
        }
    }

    assert_eq!(steps, 4, "one release per step over four work units");
    let bounded_log = log.lock().unwrap().clone();
    assert_eq!(
        sorted_invocation_ids(&bounded_log),
        sorted_invocation_ids(&completion_log)
    );
    assert_dependency_order(&bounded_log);
    assert_dependency_order(&completion_log);

    assert_eq!(
        artifacts_by_id(stepper.executor()),
        artifacts_by_id(&completion_executor)
    );
    let mut bounded_records = stepper.executor().invocation_records().to_vec();
    let mut completion_records = completion_executor.invocation_records().to_vec();
    bounded_records.sort_by(|a, b| a.invocation_id.cmp(&b.invocation_id));
    completion_records.sort_by(|a, b| a.invocation_id.cmp(&b.invocation_id));
    assert_eq!(bounded_records, completion_records);
    assert_eq!(
        stepper.executor().completed_count(),
        completion_executor.completed_count()
    );
    assert_eq!(stepper.executor().expansion_records().len(), 1);
    assert_eq!(
        stepper.executor().expansion_records(),
        completion_executor.expansion_records()
    );
    assert_eq!(
        stepper.executor().compiled_task(),
        completion_executor.compiled_task()
    );
}

#[test]
fn reopen_between_steps_resumes_without_repeating_completed_invocations() {
    let db = open_db();
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut steps = 0;
    loop {
        // A fresh instance per step proves reopen-resume: nothing survives in
        // memory between steps except the shared database.
        let mut stepper = DurablePackageExecution::open(
            db.clone(),
            branching_task(),
            branching_payload("taskrun_reopen"),
            ScriptedInvoker { log: log.clone() },
        )
        .unwrap();
        let report = block_on(stepper.step(&request(1))).unwrap();
        steps += 1;
        assert!(steps < 20, "reopened run must converge");
        if report.package_complete {
            break;
        }
    }

    assert_eq!(steps, 4);
    let invoked = log.lock().unwrap().clone();
    assert_eq!(invoked.len(), 4, "no completed invocation may repeat");
    assert_eq!(sorted_invocation_ids(&invoked).len(), 4);
    assert!(invoked.iter().all(|id| id.ends_with("::attempt::1")));
    assert_dependency_order(&invoked);

    // The reopened durable result matches the completion path exactly.
    let (completion_executor, _) = run_completion_path("taskrun_reopen");
    let reopened = DurablePackageExecution::open(
        db,
        branching_task(),
        branching_payload("taskrun_reopen"),
        ScriptedInvoker { log: log.clone() },
    )
    .unwrap();
    assert_eq!(
        artifacts_by_id(reopened.executor()),
        artifacts_by_id(&completion_executor)
    );
    assert_eq!(reopened.executor().completed_count(), 4);
}

#[test]
fn repeated_steps_after_completion_attempt_no_work() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = DurablePackageExecution::open(
        open_db(),
        branching_task(),
        branching_payload("taskrun_idle"),
        ScriptedInvoker { log: log.clone() },
    )
    .unwrap();
    loop {
        if block_on(stepper.step(&request(10)))
            .unwrap()
            .package_complete
        {
            break;
        }
    }
    let invoked_at_completion = log.lock().unwrap().len();
    let progress_at_completion = stepper.progress();

    for _ in 0..2 {
        let report = block_on(stepper.step(&request(10))).unwrap();
        assert_eq!(report.items_attempted, 0);
        assert_eq!(report.items_committed, 0);
        assert!(report.package_complete);
        assert!(!report.budget_exhausted);
        assert_eq!(report.input_progress, report.output_progress);
    }

    assert_eq!(log.lock().unwrap().len(), invoked_at_completion);
    assert_eq!(stepper.progress(), progress_at_completion);
    assert!(stepper.is_complete());
}

#[test]
fn reopen_rejects_initialization_payload_drift() {
    let db = open_db();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = DurablePackageExecution::open(
        db.clone(),
        branching_task(),
        branching_payload("taskrun_drift"),
        ScriptedInvoker { log: log.clone() },
    )
    .unwrap();
    let _ = block_on(stepper.step(&request(1))).unwrap();
    drop(stepper);

    let mut drifted = branching_payload("taskrun_drift");
    drifted.init_artifacts[0].content = json!({ "path": "other" });

    let error =
        match DurablePackageExecution::open(db, branching_task(), drifted, ScriptedInvoker { log })
        {
            Ok(_) => panic!("drifted payload must be rejected"),
            Err(error) => error,
        };

    assert!(error
        .to_string()
        .contains("different initialization payload"));
}

#[test]
fn mid_wave_resolution_failure_persists_committed_sibling_progress() {
    let db = open_db();
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut stepper = DurablePackageExecution::open(
        db.clone(),
        sibling_task(),
        run_payload("taskrun_midwave", vec![]),
        MalformedExpansionOnFirstAttempt { log: log.clone() },
    )
    .unwrap();
    let error = block_on(stepper.step(&request(3))).unwrap_err();
    assert!(error
        .to_string()
        .contains("Failed to decode task expansion"));
    drop(stepper);

    // Reopen from durable state: both healthy siblings stayed committed and
    // only the failed sibling remains outstanding.
    let mut stepper = DurablePackageExecution::open(
        db,
        sibling_task(),
        run_payload("taskrun_midwave", vec![]),
        MalformedExpansionOnFirstAttempt { log: log.clone() },
    )
    .unwrap();
    let progress = stepper.progress();
    assert_eq!(progress.completed_units, 2);
    assert_eq!(progress.known_units, 3);
    assert_eq!(progress.ready_units, 1);

    let report = block_on(stepper.step(&request(3))).unwrap();

    assert_eq!(report.items_attempted, 1, "only the failed sibling retries");
    assert_eq!(report.items_committed, 1);
    assert!(report.package_complete);
    assert_eq!(
        log.lock().unwrap().clone(),
        vec![
            "capinst_s1::attempt::1".to_string(),
            "capinst_s2::attempt::1".to_string(),
            "capinst_s3::attempt::1".to_string(),
            "capinst_s2::attempt::2".to_string(),
        ],
        "committed siblings never re-run after the mid-wave failure"
    );
}

#[test]
fn interrupted_wave_recommits_byte_identical_artifacts_as_replay() {
    let db = open_db();
    // Simulate the crash window: one sibling's artifact reached the durable
    // repo, but the progress snapshot never committed for that wave.
    let mut repo =
        TaskArtifactRepo::open_sled(db.clone(), package_step_repo_id("taskrun_replay")).unwrap();
    repo.append_artifact(out_artifact("capinst_s1", "capinst_s1::attempt::1"))
        .unwrap();
    drop(repo);

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = DurablePackageExecution::open(
        db,
        sibling_task(),
        run_payload("taskrun_replay", vec![]),
        ScriptedInvoker { log },
    )
    .unwrap();

    let report = block_on(stepper.step(&request(3))).unwrap();

    assert_eq!(report.items_attempted, 3);
    assert_eq!(report.items_committed, 3);
    assert!(report.package_complete);
    // The replayed artifact was accepted without duplicating the record.
    assert_eq!(
        stepper
            .executor()
            .artifact_repo()
            .record()
            .artifacts
            .iter()
            .filter(|artifact| artifact.artifact_id == "capinst_s1::attempt::1::out")
            .count(),
        1
    );
}

#[test]
fn interrupted_wave_rejects_drifted_artifact_recommit() {
    let db = open_db();
    let mut drifted = out_artifact("capinst_s1", "capinst_s1::attempt::1");
    drifted.content = json!({ "summary": "stale divergent content" });
    let mut repo =
        TaskArtifactRepo::open_sled(db.clone(), package_step_repo_id("taskrun_drift_replay"))
            .unwrap();
    repo.append_artifact(drifted).unwrap();
    drop(repo);

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = DurablePackageExecution::open(
        db,
        sibling_task(),
        run_payload("taskrun_drift_replay", vec![]),
        ScriptedInvoker { log },
    )
    .unwrap();

    let error = block_on(stepper.step(&request(3))).unwrap_err();
    assert!(error.to_string().contains("duplicate artifact"));

    // The drifted sibling was recorded as failed, its siblings committed, and
    // a fresh attempt with a new invocation identity completes the run.
    let progress = stepper.progress();
    assert_eq!(progress.completed_units, 2);
    let report = block_on(stepper.step(&request(3))).unwrap();
    assert_eq!(report.items_attempted, 1);
    assert!(report.package_complete);
}
