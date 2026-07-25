//! Bounded dispatch actor behavior over both dispatch routes.
//!
//! These tests prove the tick invariants: neither route exceeds the shared
//! tick budget, package handoffs dedupe on deterministic plan identity and
//! resume rather than restart, claim-route artifacts persist durably before
//! the outcome is recorded, the crash window between the two replays without
//! duplicate outputs, and failed bounded work is recorded without counting as
//! progress or re-entering the same tick.

#[path = "support/task_network.rs"]
mod task_network_support;

use async_trait::async_trait;
use futures::executor::block_on;
use meld_execution::capability::{
    BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityInvocationPayload,
    CapabilityInvocationResult,
};
use meld_execution::error::ExecutionInvariantError;
use meld_execution::planning::{
    ActionArtifactMeaning, PlanningWorldStateFrameRef, TaskPackageRoutePlan,
};
use meld_execution::task::{
    package_step_repo_id, ArtifactProducerRef, ArtifactRecord, CompiledTaskDelta,
    CompiledTaskRecord, InitArtifactValue, PackageStepInvoker, TaskArtifactRepo,
    TaskDependencyEdge, TaskDependencyKind, TaskExpansionRequest, TaskInitSlotSpec,
    TaskInitializationPayload, TaskRunContext, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID,
    TASK_EXPANSION_SCHEMA_VERSION,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest, Response};
use meld_execution::task_network::dispatch::{Claim, OutcomeStatus};
use meld_execution::task_network::dispatch_actor::{
    dispatch_claim_id, dispatch_claim_repo_id, dispatch_outcome_id, package_route_run_id,
    ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchCheckpoint, DispatchPortError,
    DispatchRuntimeActor, DispatchTickRequest, PackageRunPreparer, PreparedPackageRun,
    TaskNetworkCommandPort,
};
use meld_execution::task_network::state::{NetworkState, TaskNode, TaskStatus};
use meld_execution::task_network::store::InMemoryTaskNetworkStore;
use meld_execution::task_network::AGGREGATE_OUTCOME_CONTRACT_ID;
use serde_json::json;
use std::sync::{Arc, Mutex};

type ApiError = ExecutionInvariantError;

fn open_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn tick_request(
    sequence: u64,
    max_items: usize,
    package_plans: Vec<TaskPackageRoutePlan>,
) -> DispatchTickRequest {
    DispatchTickRequest {
        sequence,
        max_items,
        package_plans,
    }
}

fn package_plan(plan_id: &str) -> TaskPackageRoutePlan {
    TaskPackageRoutePlan {
        plan_id: plan_id.to_string(),
        network_id: "network-docs".to_string(),
        composition_id: "composition-docs".to_string(),
        goal_id: "goal-docs".to_string(),
        method_id: "refresh_docs_v1".to_string(),
        action_id: "action-docs".to_string(),
        package_id: "docs_writer".to_string(),
        workflow_id: "docs_writer_thread_v1".to_string(),
        outcome_contract_id: AGGREGATE_OUTCOME_CONTRACT_ID.to_string(),
        artifact: ActionArtifactMeaning {
            artifact_type_id: "readme_summary".to_string(),
            schema_version: 1,
        },
        world_state_frame: PlanningWorldStateFrameRef {
            frame_id: "frame-docs".to_string(),
            projection_version: "world_model.planner.v1".to_string(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            source_refs: vec!["source".to_string()],
            warnings: vec![],
        },
    }
}

// Package fixtures mirror the bounded package-step tests so the actor drives
// the exact machinery the step contract already proved.

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

fn package_edge(from: &str, to: &str, kind: TaskDependencyKind) -> TaskDependencyEdge {
    TaskDependencyEdge {
        from_capability_instance_id: from.to_string(),
        to_capability_instance_id: to.to_string(),
        kind,
        reason: "fixture ordering".to_string(),
    }
}

/// Branching fixture: two sibling children fan out first, child_a's success
/// expands the graph with child_c, and the parent prepares only after every
/// child finalized.
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
            package_edge(
                "capinst_child_a",
                "capinst_parent",
                TaskDependencyKind::Artifact,
            ),
            package_edge(
                "capinst_child_b",
                "capinst_parent",
                TaskDependencyKind::Effect,
            ),
        ],
    }
}

fn branching_init_artifacts() -> Vec<InitArtifactValue> {
    vec![InitArtifactValue {
        init_slot_id: "target_selector".to_string(),
        artifact_type_id: "target_selector".to_string(),
        schema_version: 1,
        content: json!({ "path": "docs" }),
    }]
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

fn scripted_artifacts(payload: &CapabilityInvocationPayload) -> Vec<ArtifactRecord> {
    let mut artifacts = vec![out_artifact(
        &payload.capability_instance_id,
        &payload.invocation_id,
    )];
    if payload.capability_instance_id == "capinst_child_a" {
        artifacts.push(ArtifactRecord {
            artifact_id: format!("{}::expansion", payload.invocation_id),
            artifact_type_id: TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID.to_string(),
            schema_version: TASK_EXPANSION_SCHEMA_VERSION,
            content: json!({
                "expansion_id": "expansion_child_c",
                "expansion_kind": "discover_children",
                "content": {}
            }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: payload.capability_instance_id.clone(),
                invocation_id: Some(payload.invocation_id.clone()),
                output_slot_id: Some("expansion".to_string()),
            },
        });
    }
    artifacts
}

struct ScriptedPackageInvoker {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl PackageStepInvoker for ScriptedPackageInvoker {
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
        Ok(CompiledTaskDelta {
            init_slots: vec![],
            init_artifacts: vec![],
            capability_instances: vec![bare_instance("capinst_child_c", vec![])],
            dependency_edges: vec![package_edge(
                "capinst_child_c",
                "capinst_parent",
                TaskDependencyKind::Effect,
            )],
        })
    }
}

struct FixturePreparer {
    compiled: CompiledTaskRecord,
    init_artifacts: Vec<InitArtifactValue>,
    calls: Arc<Mutex<usize>>,
}

impl PackageRunPreparer for FixturePreparer {
    fn prepare_package_run(
        &self,
        _plan: &TaskPackageRoutePlan,
        task_run_id: &str,
    ) -> Result<PreparedPackageRun, DispatchPortError> {
        *self.calls.lock().unwrap() += 1;
        Ok(PreparedPackageRun {
            compiled_task: self.compiled.clone(),
            init_payload: TaskInitializationPayload {
                task_id: self.compiled.task_id.clone(),
                compiled_task_ref: "compiled_task_docs_writer".to_string(),
                init_artifacts: self.init_artifacts.clone(),
                task_run_context: TaskRunContext {
                    task_run_id: task_run_id.to_string(),
                    session_id: Some("session_1".to_string()),
                    trigger: "test".to_string(),
                },
            },
        })
    }
}

fn claim_artifact(claim: &Claim) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("{}::readme", claim.claim_id),
        artifact_type_id: "docs_patch".to_string(),
        schema_version: 1,
        content: json!({ "task_instance_id": claim.task_instance_id }),
        producer: ArtifactProducerRef {
            task_id: format!("compiled-{}", claim.task_instance_id),
            capability_instance_id: "write".to_string(),
            invocation_id: Some(format!("{}::invoke", claim.claim_id)),
            output_slot_id: Some("patch".to_string()),
        },
    }
}

struct ScriptedClaimInvoker {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ClaimedTaskInvoker for ScriptedClaimInvoker {
    async fn invoke_claimed_task(
        &self,
        _node: &TaskNode,
        claim: &Claim,
        _init_payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
        self.log
            .lock()
            .unwrap()
            .push(claim.task_instance_id.clone());
        Ok(ClaimedInvocationOutcome::Completed(vec![claim_artifact(
            claim,
        )]))
    }
}

struct FailingClaimInvoker {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ClaimedTaskInvoker for FailingClaimInvoker {
    async fn invoke_claimed_task(
        &self,
        _node: &TaskNode,
        claim: &Claim,
        _init_payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
        self.log
            .lock()
            .unwrap()
            .push(claim.task_instance_id.clone());
        Ok(ClaimedInvocationOutcome::Failed {
            error: "provider failed".to_string(),
        })
    }
}

/// Simulates a crash between claim-route artifact persistence and outcome
/// recording: every outcome command is lost before it reaches the store.
struct FailOutcomePort<'a> {
    inner: &'a mut InMemoryTaskNetworkStore,
}

impl TaskNetworkCommandPort for FailOutcomePort<'_> {
    fn network_state(&self) -> &NetworkState {
        self.inner.state()
    }

    fn submit_command(&mut self, request: CommandRequest) -> Result<Response, DispatchPortError> {
        if matches!(request.command, Command::RecordTaskOutcome(_)) {
            return Err(DispatchPortError::retryable(
                "simulated crash before outcome record",
            ));
        }
        Ok(self.inner.submit(request))
    }
}

struct DispatchFixture {
    package_log: Arc<Mutex<Vec<String>>>,
    preparer_calls: Arc<Mutex<usize>>,
    claim_log: Arc<Mutex<Vec<String>>>,
}

impl DispatchFixture {
    fn new() -> Self {
        Self {
            package_log: Arc::new(Mutex::new(Vec::new())),
            preparer_calls: Arc::new(Mutex::new(0)),
            claim_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn actor(
        &self,
        db: sled::Db,
        compiled: CompiledTaskRecord,
        init_artifacts: Vec<InitArtifactValue>,
    ) -> DispatchRuntimeActor<FixturePreparer, ScriptedPackageInvoker, ScriptedClaimInvoker> {
        DispatchRuntimeActor::new(
            "worker-a",
            db,
            FixturePreparer {
                compiled,
                init_artifacts,
                calls: self.preparer_calls.clone(),
            },
            ScriptedPackageInvoker {
                log: self.package_log.clone(),
            },
            ScriptedClaimInvoker {
                log: self.claim_log.clone(),
            },
        )
        .unwrap()
    }

    fn package_invocations(&self) -> Vec<String> {
        self.package_log.lock().unwrap().clone()
    }

    fn preparer_call_count(&self) -> usize {
        *self.preparer_calls.lock().unwrap()
    }

    fn claim_invocations(&self) -> Vec<String> {
        self.claim_log.lock().unwrap().clone()
    }
}

fn package_step_checkpoints(
    checkpoints: &[DispatchCheckpoint],
) -> Vec<&meld_execution::task_network::PackageStepReport> {
    checkpoints
        .iter()
        .filter_map(|checkpoint| match checkpoint {
            DispatchCheckpoint::PackageStepDriven { step, .. } => Some(step),
            _ => None,
        })
        .collect()
}

#[test]
fn package_route_tick_never_releases_beyond_budget() {
    let fixture = DispatchFixture::new();
    let actor = fixture.actor(open_db(), sibling_task(), vec![]);
    let mut store = InMemoryTaskNetworkStore::new("network-docs");

    let report = block_on(actor.tick(
        &mut store,
        tick_request(1, 2, vec![package_plan("plan-budget")]),
    ))
    .unwrap();

    assert_eq!(report.items_attempted, 2);
    assert_eq!(report.items_committed, 2);
    assert!(report.budget_exhausted);
    assert_eq!(fixture.package_invocations().len(), 2);
    let steps = package_step_checkpoints(&report.checkpoints);
    assert_eq!(steps.len(), 1, "one plan drives exactly one bounded step");
    assert_eq!(steps[0].items_attempted, 2);
    assert!(!steps[0].package_complete);
}

#[test]
fn claim_route_tick_never_claims_beyond_budget() {
    let fixture = DispatchFixture::new();
    let actor = fixture.actor(open_db(), sibling_task(), vec![]);
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    task_network_support::commit_single_task(&mut store, "task-beta");

    let report = block_on(actor.tick(&mut store, tick_request(1, 1, vec![]))).unwrap();

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    assert!(report.budget_exhausted);
    assert_eq!(store.state().claims.len(), 1);
    // Deterministic ready-set order claims the first task instance id.
    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Succeeded { .. })
    ));
    assert_eq!(
        store.state().statuses.get("task-beta"),
        Some(&TaskStatus::Pending)
    );
    assert_eq!(fixture.claim_invocations(), vec!["task-alpha".to_string()]);
}

#[test]
fn shared_budget_spans_package_and_claim_routes() {
    let fixture = DispatchFixture::new();
    let db = open_db();
    let actor = fixture.actor(db, sibling_task(), vec![]);
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(
        &mut store,
        tick_request(1, 4, vec![package_plan("plan-shared")]),
    ))
    .unwrap();

    // Three sibling package invocations plus one claimed task fill the budget.
    assert_eq!(report.items_attempted, 4);
    assert_eq!(report.items_committed, 4);
    assert!(!report.budget_exhausted);
    assert_eq!(fixture.package_invocations().len(), 3);
    assert_eq!(fixture.claim_invocations().len(), 1);
    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Succeeded { .. })
    ));
}

#[test]
fn package_route_consumes_shared_budget_before_claim_route() {
    let fixture = DispatchFixture::new();
    let actor = fixture.actor(open_db(), sibling_task(), vec![]);
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(
        &mut store,
        tick_request(1, 3, vec![package_plan("plan-starved")]),
    ))
    .unwrap();

    assert_eq!(report.items_attempted, 3);
    assert!(report.budget_exhausted, "the ready task remained unclaimed");
    // The completed package run records its terminal outcome outside the tick
    // budget, so its recording claim is the only claim in the network: the
    // claim route itself dispatched nothing.
    assert!(store
        .state()
        .claims
        .values()
        .all(|claim| claim.task_instance_id.starts_with("package-run::")));
    assert_eq!(
        store.state().statuses.get("task-alpha"),
        Some(&TaskStatus::Pending)
    );
    assert!(fixture.claim_invocations().is_empty());
}

#[test]
fn duplicate_package_handoff_in_one_tick_drives_one_run() {
    let fixture = DispatchFixture::new();
    let actor = fixture.actor(open_db(), branching_task(), branching_init_artifacts());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let plan = package_plan("plan-dupe");

    let report =
        block_on(actor.tick(&mut store, tick_request(1, 10, vec![plan.clone(), plan]))).unwrap();

    assert_eq!(fixture.preparer_call_count(), 1);
    assert_eq!(package_step_checkpoints(&report.checkpoints).len(), 1);
    // The branching fixture fans out two ready children in its first wave.
    assert_eq!(fixture.package_invocations().len(), 2);
}

#[test]
fn completed_package_plan_dedupes_durably_without_restart() {
    let fixture = DispatchFixture::new();
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let plan = package_plan("plan-complete");

    let actor = fixture.actor(db.clone(), sibling_task(), vec![]);
    let first = block_on(actor.tick(&mut store, tick_request(1, 10, vec![plan.clone()]))).unwrap();
    assert!(package_step_checkpoints(&first.checkpoints)[0].package_complete);
    assert_eq!(fixture.package_invocations().len(), 3);
    assert_eq!(fixture.preparer_call_count(), 1);

    // A fresh actor over the same database proves the dedupe derives from the
    // durable progress store, not from anything held in memory.
    let actor = fixture.actor(db, sibling_task(), vec![]);
    let second = block_on(actor.tick(&mut store, tick_request(2, 10, vec![plan.clone()]))).unwrap();

    assert_eq!(second.items_attempted, 0);
    assert!(matches!(
        second.checkpoints.as_slice(),
        [DispatchCheckpoint::PackageRunAlreadyComplete { plan_id, task_run_id }]
            if plan_id == &plan.plan_id && task_run_id == &package_route_run_id(&plan.plan_id)
    ));
    assert_eq!(
        fixture.preparer_call_count(),
        1,
        "a completed plan is deduped before preparation"
    );
    assert_eq!(fixture.package_invocations().len(), 3);
}

#[test]
fn branching_package_resumes_across_ticks_and_completes_with_durable_artifacts() {
    let fixture = DispatchFixture::new();
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let plan = package_plan("plan-branching");

    let mut ticks = 0;
    loop {
        // A fresh actor per tick proves resume-not-restart: nothing survives
        // between ticks except durable state and the shared handoff identity.
        let actor = fixture.actor(db.clone(), branching_task(), branching_init_artifacts());
        let report =
            block_on(actor.tick(&mut store, tick_request(ticks, 1, vec![plan.clone()]))).unwrap();
        assert!(report.items_attempted <= 1, "budget of one must hold");
        ticks += 1;
        assert!(ticks < 20, "bounded run must converge");
        let complete = package_step_checkpoints(&report.checkpoints)
            .iter()
            .any(|step| step.package_complete);
        if complete {
            break;
        }
    }

    assert_eq!(ticks, 4, "one release per tick over four work units");
    let invoked = fixture.package_invocations();
    assert_eq!(invoked.len(), 4, "no completed invocation may repeat");
    assert!(invoked.iter().all(|id| id.ends_with("::attempt::1")));

    // Per-unit artifacts are durable in the run's task-owned repository.
    let repo = TaskArtifactRepo::open_sled(
        db,
        package_step_repo_id(&package_route_run_id(&plan.plan_id)),
    )
    .unwrap();
    for artifact_id in [
        "capinst_child_a::attempt::1::out",
        "capinst_child_b::attempt::1::out",
        "capinst_child_c::attempt::1::out",
        "capinst_parent::attempt::1::out",
    ] {
        assert!(
            repo.get_artifact(artifact_id).is_some(),
            "missing durable artifact '{artifact_id}'"
        );
    }
}

#[test]
fn claim_route_records_outcome_through_command_boundary_with_durable_artifacts() {
    let fixture = DispatchFixture::new();
    let db = open_db();
    let actor = fixture.actor(db.clone(), sibling_task(), vec![]);
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(&mut store, tick_request(1, 1, vec![]))).unwrap();

    assert_eq!(report.items_committed, 1);
    let claim_id = dispatch_claim_id("network-docs", "task-alpha", 1, "worker-a");
    let outcome_id = dispatch_outcome_id(&claim_id);
    let outcome = store.state().outcomes.get(&outcome_id).unwrap();
    assert_eq!(outcome.status, OutcomeStatus::Succeeded);
    assert_eq!(outcome.claim_id, claim_id);
    assert_eq!(outcome.artifact_records.len(), 1);
    let artifact_id = format!("{claim_id}::readme");
    assert_eq!(outcome.artifact_records[0].artifact_id, artifact_id);
    assert!(store
        .state()
        .artifact_availability
        .iter()
        .any(|artifact| artifact.artifact_id == artifact_id));
    assert!(matches!(
        report.checkpoints.as_slice(),
        [DispatchCheckpoint::TaskOutcomeRecorded {
            status: OutcomeStatus::Succeeded,
            resumed: false,
            ..
        }]
    ));

    // The recorded artifact is the durable task-owned record, not a copy that
    // exists only inside the outcome payload.
    let repo = TaskArtifactRepo::open_sled(db, dispatch_claim_repo_id(&claim_id)).unwrap();
    assert!(repo.get_artifact(&artifact_id).is_some());
}

#[test]
fn crash_between_artifact_persist_and_outcome_record_replays_without_duplicates() {
    let fixture = DispatchFixture::new();
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let claim_id = dispatch_claim_id("network-docs", "task-alpha", 1, "worker-a");
    let artifact_id = format!("{claim_id}::readme");

    // Crash window: the claim is fenced and the artifact is durable, but the
    // outcome command never reaches the store.
    let actor = fixture.actor(db.clone(), sibling_task(), vec![]);
    let crashed = block_on(actor.tick(
        &mut FailOutcomePort { inner: &mut store },
        tick_request(1, 1, vec![]),
    ))
    .unwrap();
    assert_eq!(crashed.items_attempted, 1);
    assert_eq!(crashed.items_committed, 0);
    assert!(crashed
        .retryable_errors
        .iter()
        .any(|issue| issue.code == "outcome_submit_failed"));
    assert!(store.state().outcomes.is_empty());
    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Running { claim_id: running }) if *running == claim_id
    ));
    let repo = TaskArtifactRepo::open_sled(db.clone(), dispatch_claim_repo_id(&claim_id)).unwrap();
    assert!(repo.get_artifact(&artifact_id).is_some());

    // Replay: a fresh actor resumes the fenced claim, re-persists the
    // byte-identical artifact as durable replay, and records one outcome.
    let actor = fixture.actor(db.clone(), sibling_task(), vec![]);
    let replayed = block_on(actor.tick(&mut store, tick_request(2, 1, vec![]))).unwrap();

    assert_eq!(replayed.items_attempted, 1);
    assert_eq!(replayed.items_committed, 1);
    assert!(matches!(
        replayed.checkpoints.as_slice(),
        [DispatchCheckpoint::TaskOutcomeRecorded {
            resumed: true,
            status: OutcomeStatus::Succeeded,
            ..
        }]
    ));
    assert_eq!(fixture.claim_invocations().len(), 2, "the claim replays");
    assert_eq!(store.state().claims.len(), 1, "one fenced claim, no second");
    assert_eq!(store.state().outcomes.len(), 1);
    let repo = TaskArtifactRepo::open_sled(db, dispatch_claim_repo_id(&claim_id)).unwrap();
    assert_eq!(
        repo.record()
            .artifacts
            .iter()
            .filter(|artifact| artifact.artifact_id == artifact_id)
            .count(),
        1,
        "duplicate dispatch must not duplicate domain outputs"
    );
}

#[test]
fn failed_invocation_is_recorded_without_progress_or_same_tick_requeue() {
    let claim_log = Arc::new(Mutex::new(Vec::new()));
    let actor = DispatchRuntimeActor::new(
        "worker-a",
        open_db(),
        FixturePreparer {
            compiled: sibling_task(),
            init_artifacts: vec![],
            calls: Arc::new(Mutex::new(0)),
        },
        ScriptedPackageInvoker {
            log: Arc::new(Mutex::new(Vec::new())),
        },
        FailingClaimInvoker {
            log: claim_log.clone(),
        },
    )
    .unwrap();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(&mut store, tick_request(1, 5, vec![]))).unwrap();

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 0, "failed work is not progress");
    assert!(!report.budget_exhausted);
    assert_eq!(
        claim_log.lock().unwrap().len(),
        1,
        "a failed task must not re-enter the same tick"
    );
    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Failed { error, .. }) if error == "provider failed"
    ));
    assert!(matches!(
        report.checkpoints.as_slice(),
        [DispatchCheckpoint::TaskOutcomeRecorded {
            status: OutcomeStatus::Failed,
            ..
        }]
    ));
}

#[test]
fn zero_budget_tick_attempts_nothing_and_reports_pending_work() {
    let fixture = DispatchFixture::new();
    let actor = fixture.actor(open_db(), sibling_task(), vec![]);
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(
        &mut store,
        tick_request(1, 0, vec![package_plan("plan-idle")]),
    ))
    .unwrap();

    assert_eq!(report.items_attempted, 0);
    assert_eq!(report.items_committed, 0);
    assert!(report.budget_exhausted);
    assert!(store.state().claims.is_empty());
    assert!(fixture.package_invocations().is_empty());
    assert!(fixture.claim_invocations().is_empty());
    assert_eq!(fixture.preparer_call_count(), 0);
}
