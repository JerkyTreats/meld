//! Package-route terminal outcome recording behavior.
//!
//! Proves the proof-blocking gap is closed: a package run driven to durable
//! completion gains one task-network node, claim, and Succeeded outcome
//! through the command boundary under deterministic run-derived identities;
//! replayed observation is a no-op; incomplete runs record nothing; the
//! recorded outcome carries the run's durable artifact records byte-intact;
//! aggregate production accepts the recorded outcome and yields a publishable
//! aggregate; and the store-surface query returns the outcome by run id.

use async_trait::async_trait;
use futures::executor::block_on;
use meld_events::{
    AppendReceipt, DomainObjectRef, EventAppendCapability, EventAuthority,
    EventAuthorityOpenOptions, EventEnvelope, LedgerIdentity,
};
use meld_execution::capability::{
    BoundCapabilityInstance, CapabilityInvocationPayload, CapabilityInvocationResult,
};
use meld_execution::error::ExecutionInvariantError;
use meld_execution::planning::{
    ActionArtifactMeaning, PlanningWorldStateFrameRef, TaskPackageRoutePlan,
};
use meld_execution::task::{
    package_step_repo_id, ArtifactProducerRef, ArtifactRecord, CompiledTaskDelta,
    CompiledTaskRecord, InitArtifactValue, PackageStepInvoker, TaskArtifactRepo,
    TaskExpansionRequest, TaskInitSlotSpec, TaskInitializationPayload, TaskProgressStore,
    TaskRunContext,
};
use meld_execution::task_network::aggregate::AggregatePackageStatus;
use meld_execution::task_network::aggregate_publication::{
    produce_aggregate_outcome, publish_aggregate, AggregateProduction, AggregatePublicationStore,
    AggregatePublishResult, AggregateRunBinding, PublishAggregateRequest,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest, Response};
use meld_execution::task_network::dispatch::{Claim, OutcomeStatus};
use meld_execution::task_network::dispatch_actor::{
    dispatch_claim_id, dispatch_outcome_id, package_route_run_id, ClaimedInvocationOutcome,
    ClaimedTaskInvoker, DispatchCheckpoint, DispatchPortError, DispatchRuntimeActor,
    DispatchTickRequest, PackageRunPreparer, PreparedPackageRun, TaskNetworkCommandPort,
};
use meld_execution::task_network::publication::EventAppendSink;
use meld_execution::task_network::state::{NetworkState, TaskNode, TaskStatus};
use meld_execution::task_network::store::InMemoryTaskNetworkStore;
use meld_execution::task_network::terminal_recording::{
    package_run_task_instance_id, package_run_terminal_outcome,
    record_package_run_terminal_outcome, PackageRunRecording,
};
use meld_execution::task_network::AGGREGATE_OUTCOME_CONTRACT_ID;
use serde_json::json;
use std::sync::{Arc, Mutex};

type ApiError = ExecutionInvariantError;

const NETWORK_ID: &str = "network-docs";
const WORKER_ID: &str = "worker-a";
const TASK_INIT_PRODUCER: &str = "__task_init__";

fn open_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn package_plan(plan_id: &str) -> TaskPackageRoutePlan {
    TaskPackageRoutePlan {
        plan_id: plan_id.to_string(),
        network_id: NETWORK_ID.to_string(),
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

fn folder_instance(capability_instance_id: &str, scope_ref: &str) -> BoundCapabilityInstance {
    BoundCapabilityInstance {
        capability_instance_id: capability_instance_id.to_string(),
        capability_type_id: "docs.write".to_string(),
        capability_version: 1,
        scope_ref: scope_ref.to_string(),
        scope_kind: "filesystem".to_string(),
        binding_values: vec![],
        input_wiring: vec![],
    }
}

/// Three independent per-folder units plus one required init slot, so the
/// run's durable repo carries an init seed record the terminal outcome must
/// exclude.
fn folder_package_task() -> CompiledTaskRecord {
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
            folder_instance("capinst_folder_a", "docs/a"),
            folder_instance("capinst_folder_b", "docs/b"),
            folder_instance("capinst_folder_c", "docs/c"),
        ],
        dependency_edges: vec![],
    }
}

fn package_init_artifacts() -> Vec<InitArtifactValue> {
    vec![InitArtifactValue {
        init_slot_id: "target_selector".to_string(),
        artifact_type_id: "target_selector".to_string(),
        schema_version: 1,
        content: json!({ "path": "docs" }),
    }]
}

struct ScriptedPackageInvoker;

#[async_trait]
impl PackageStepInvoker for ScriptedPackageInvoker {
    async fn invoke_capability(
        &self,
        _instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![ArtifactRecord {
                artifact_id: format!("{}::out", payload.invocation_id),
                artifact_type_id: "readme_summary".to_string(),
                schema_version: 1,
                content: json!({ "summary": payload.capability_instance_id }),
                producer: ArtifactProducerRef {
                    task_id: "task_docs_writer".to_string(),
                    capability_instance_id: payload.capability_instance_id.clone(),
                    invocation_id: Some(payload.invocation_id.clone()),
                    output_slot_id: Some("out".to_string()),
                },
            }],
        })
    }

    fn compile_expansion(
        &self,
        _compiled_task: &CompiledTaskRecord,
        _request: &TaskExpansionRequest,
    ) -> Result<CompiledTaskDelta, ApiError> {
        unreachable!("this fixture never requests expansion")
    }
}

struct FixturePreparer;

impl PackageRunPreparer for FixturePreparer {
    fn prepare_package_run(
        &self,
        _plan: &TaskPackageRoutePlan,
        task_run_id: &str,
    ) -> Result<PreparedPackageRun, DispatchPortError> {
        Ok(PreparedPackageRun {
            compiled_task: folder_package_task(),
            init_payload: TaskInitializationPayload {
                task_id: "task_docs_writer".to_string(),
                compiled_task_ref: "compiled_task_docs_writer".to_string(),
                init_artifacts: package_init_artifacts(),
                task_run_context: TaskRunContext {
                    task_run_id: task_run_id.to_string(),
                    session_id: Some("session_1".to_string()),
                    trigger: "test".to_string(),
                },
            },
        })
    }
}

/// Claim invoker that must never run: package-run terminal nodes are the only
/// claimable nodes these tests create, and the claim route must never invoke
/// them.
struct ForbiddenClaimInvoker {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ClaimedTaskInvoker for ForbiddenClaimInvoker {
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
        Err(DispatchPortError::fatal(
            "claim route invoked a package-run terminal node",
        ))
    }
}

fn actor(
    db: sled::Db,
    claim_log: Arc<Mutex<Vec<String>>>,
) -> DispatchRuntimeActor<FixturePreparer, ScriptedPackageInvoker, ForbiddenClaimInvoker> {
    DispatchRuntimeActor::new(
        WORKER_ID,
        db,
        FixturePreparer,
        ScriptedPackageInvoker,
        ForbiddenClaimInvoker { log: claim_log },
    )
    .unwrap()
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

/// Drives the plan's three-unit package to durable completion in one tick and
/// returns the terminal-recording checkpoints observed.
fn complete_run(
    db: sled::Db,
    store: &mut InMemoryTaskNetworkStore,
    plan: &TaskPackageRoutePlan,
    claim_log: Arc<Mutex<Vec<String>>>,
) -> Vec<DispatchCheckpoint> {
    let actor = actor(db, claim_log);
    let report = block_on(actor.tick(store, tick_request(1, 10, vec![plan.clone()]))).unwrap();
    assert!(
        report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
        "completion tick must be clean: {report:?}"
    );
    report.checkpoints
}

fn terminal_checkpoints(checkpoints: &[DispatchCheckpoint]) -> Vec<&DispatchCheckpoint> {
    checkpoints
        .iter()
        .filter(|checkpoint| {
            matches!(
                checkpoint,
                DispatchCheckpoint::PackageTerminalOutcomeRecorded { .. }
            )
        })
        .collect()
}

fn durable_emitted_artifacts(db: sled::Db, task_run_id: &str) -> Vec<ArtifactRecord> {
    let repo = TaskArtifactRepo::open_sled(db, package_step_repo_id(task_run_id)).unwrap();
    repo.record()
        .artifacts
        .iter()
        .filter(|artifact| artifact.producer.capability_instance_id != TASK_INIT_PRODUCER)
        .cloned()
        .collect()
}

#[test]
fn completed_run_records_node_claim_and_outcome_with_deterministic_identities() {
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    let claim_log = Arc::new(Mutex::new(Vec::new()));
    let plan = package_plan("plan-terminal");
    let task_run_id = package_route_run_id(&plan.plan_id);

    let checkpoints = complete_run(db, &mut store, &plan, claim_log.clone());

    // Every identity is a pure function of the run id and the worker id.
    let task_instance_id = package_run_task_instance_id(&task_run_id);
    let claim_id = dispatch_claim_id(NETWORK_ID, &task_instance_id, 0, WORKER_ID);
    let outcome_id = dispatch_outcome_id(&claim_id);

    let node = store.state().tasks.get(&task_instance_id).unwrap();
    assert_eq!(node.lifecycle_epoch, 0);
    assert_eq!(node.task_run_context.task_run_id, task_run_id);
    assert!(
        node.compiled_task.capability_instances.is_empty(),
        "terminal marker must not fabricate capability structure"
    );
    assert_eq!(node.compiled_task.task_id, "task_docs_writer");
    assert_eq!(node.lineage.goal_id, plan.goal_id);
    assert_eq!(node.lineage.method_id, plan.method_id);
    assert_eq!(node.lineage.composition_id, plan.composition_id);
    assert_eq!(
        node.lineage.world_state_frame_id,
        plan.world_state_frame.frame_id
    );
    assert_eq!(node.lineage.step_id, plan.action_id);
    assert_eq!(node.lineage.operator_id, plan.package_id);

    let claim = store.state().claims.get(&claim_id).unwrap();
    assert_eq!(claim.worker_id, WORKER_ID);
    assert_eq!(claim.task_instance_id, task_instance_id);

    let outcome = store.state().outcomes.get(&outcome_id).unwrap();
    assert_eq!(outcome.status, OutcomeStatus::Succeeded);
    assert_eq!(outcome.claim_id, claim_id);
    assert_eq!(outcome.claim_revision, claim.claim_revision);
    assert!(outcome.task_events.is_empty(), "no fabricated task events");
    assert!(matches!(
        store.state().statuses.get(&task_instance_id),
        Some(TaskStatus::Succeeded { outcome_id: recorded }) if *recorded == outcome_id
    ));

    let recorded = terminal_checkpoints(&checkpoints);
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        DispatchCheckpoint::PackageTerminalOutcomeRecorded {
            task_run_id: checkpoint_run,
            task_instance_id: checkpoint_instance,
            outcome_id: checkpoint_outcome,
        } if *checkpoint_run == task_run_id
            && *checkpoint_instance == task_instance_id
            && *checkpoint_outcome == outcome_id
    ));
    assert!(
        claim_log.lock().unwrap().is_empty(),
        "the claim route must never invoke the terminal node"
    );
}

#[test]
fn recorded_outcome_artifacts_byte_match_the_durable_records() {
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    let plan = package_plan("plan-bytes");
    let task_run_id = package_route_run_id(&plan.plan_id);

    complete_run(
        db.clone(),
        &mut store,
        &plan,
        Arc::new(Mutex::new(Vec::new())),
    );

    let terminal = package_run_terminal_outcome(store.state(), &task_run_id).unwrap();
    let expected = durable_emitted_artifacts(db.clone(), &task_run_id);
    assert_eq!(expected.len(), 3, "one durable record per folder unit");
    assert_eq!(
        serde_json::to_vec(&terminal.outcome.artifact_records).unwrap(),
        serde_json::to_vec(&expected).unwrap(),
        "outcome artifact records must byte-match the durable repository"
    );

    // The run's init seed is durable in the repo but is an input, not a
    // product, so the terminal outcome excludes it.
    let repo = TaskArtifactRepo::open_sled(db, package_step_repo_id(&task_run_id)).unwrap();
    assert!(repo
        .record()
        .artifacts
        .iter()
        .any(|artifact| artifact.producer.capability_instance_id == TASK_INIT_PRODUCER));
    assert!(terminal
        .outcome
        .artifact_records
        .iter()
        .all(|artifact| artifact.producer.capability_instance_id != TASK_INIT_PRODUCER));
}

#[test]
fn replayed_completion_observation_is_a_noop() {
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    let plan = package_plan("plan-replay");
    let task_run_id = package_route_run_id(&plan.plan_id);

    complete_run(
        db.clone(),
        &mut store,
        &plan,
        Arc::new(Mutex::new(Vec::new())),
    );
    let outcome_before = package_run_terminal_outcome(store.state(), &task_run_id)
        .unwrap()
        .outcome;
    let revision_before = store.state().revision;
    let state_hash_before = store.state().state_hash.clone();

    // A fresh actor over the same database re-observes the completed run.
    let replay_actor = actor(db, Arc::new(Mutex::new(Vec::new())));
    let report =
        block_on(replay_actor.tick(&mut store, tick_request(2, 10, vec![plan.clone()]))).unwrap();

    assert!(report.fatal_errors.is_empty() && report.retryable_errors.is_empty());
    assert!(
        terminal_checkpoints(&report.checkpoints).is_empty(),
        "an already-recorded run reaches no new checkpoint"
    );
    assert!(matches!(
        report.checkpoints.as_slice(),
        [DispatchCheckpoint::PackageRunAlreadyComplete { .. }]
    ));
    assert_eq!(store.state().revision, revision_before, "no new commands");
    assert_eq!(store.state().state_hash, state_hash_before);
    assert_eq!(store.state().outcomes.len(), 1, "no duplicate outcome");
    assert_eq!(store.state().claims.len(), 1, "no duplicate claim");
    assert_eq!(store.state().tasks.len(), 1, "no duplicate node");
    let outcome_after = package_run_terminal_outcome(store.state(), &task_run_id)
        .unwrap()
        .outcome;
    assert_eq!(outcome_after, outcome_before, "same durable outcome");
}

#[test]
fn incomplete_run_records_nothing() {
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    let claim_log = Arc::new(Mutex::new(Vec::new()));
    let plan = package_plan("plan-incomplete");
    let task_run_id = package_route_run_id(&plan.plan_id);

    // Budget one drives one of three units: durably incomplete.
    let partial_actor = actor(db.clone(), claim_log);
    let report =
        block_on(partial_actor.tick(&mut store, tick_request(1, 1, vec![plan.clone()]))).unwrap();

    assert!(terminal_checkpoints(&report.checkpoints).is_empty());
    assert!(store.state().tasks.is_empty(), "no node for incomplete run");
    assert!(store.state().claims.is_empty());
    assert!(store.state().outcomes.is_empty());
    assert_eq!(
        package_run_terminal_outcome(store.state(), &task_run_id),
        None
    );

    // The recorder itself refuses the incomplete snapshot and submits nothing.
    let progress = TaskProgressStore::open(db.clone()).unwrap();
    let snapshot = progress.load(&task_run_id).unwrap().unwrap();
    let artifacts = durable_emitted_artifacts(db, &task_run_id);
    let recording =
        record_package_run_terminal_outcome(&mut store, WORKER_ID, &snapshot, artifacts, None)
            .unwrap();
    assert!(matches!(
        recording,
        PackageRunRecording::Skipped { pending_instance_ids } if pending_instance_ids.len() == 2
    ));
    assert_eq!(store.state().revision, 0, "nothing reached the boundary");
}

#[test]
fn stranded_terminal_node_is_recovered_by_the_claim_route_without_invocation() {
    /// Simulates a crash between the terminal node's inject and its claim:
    /// every claim command is lost before it reaches the store.
    struct FailClaimPort<'a> {
        inner: &'a mut InMemoryTaskNetworkStore,
    }

    impl TaskNetworkCommandPort for FailClaimPort<'_> {
        fn network_state(&self) -> &NetworkState {
            self.inner.state()
        }

        fn submit_command(
            &mut self,
            request: CommandRequest,
        ) -> Result<Response, DispatchPortError> {
            if matches!(request.command, Command::ClaimReadyTask(_)) {
                return Err(DispatchPortError::retryable(
                    "simulated crash before terminal claim",
                ));
            }
            Ok(self.inner.submit(request))
        }
    }

    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    let claim_log = Arc::new(Mutex::new(Vec::new()));
    let plan = package_plan("plan-stranded");
    let task_run_id = package_route_run_id(&plan.plan_id);
    let task_instance_id = package_run_task_instance_id(&task_run_id);

    let crash_actor = actor(db.clone(), claim_log.clone());
    let crashed = block_on(crash_actor.tick(
        &mut FailClaimPort { inner: &mut store },
        tick_request(1, 10, vec![plan.clone()]),
    ))
    .unwrap();
    assert!(crashed
        .retryable_errors
        .iter()
        .any(|issue| issue.code == "terminal_recording_failed"));
    assert_eq!(
        store.state().statuses.get(&task_instance_id),
        Some(&TaskStatus::Pending),
        "the node is stranded before its claim"
    );

    // Recovery tick: no plan handoff, healthy port. The claim route must
    // finish the recording from durable state instead of dispatching the node.
    let recovery_actor = actor(db.clone(), claim_log.clone());
    let recovered = block_on(recovery_actor.tick(&mut store, tick_request(2, 10, vec![]))).unwrap();

    assert!(recovered.fatal_errors.is_empty() && recovered.retryable_errors.is_empty());
    assert_eq!(recovered.items_attempted, 0, "recovery is not bounded work");
    assert_eq!(terminal_checkpoints(&recovered.checkpoints).len(), 1);
    assert!(
        claim_log.lock().unwrap().is_empty(),
        "the terminal node must never be invoked"
    );
    let terminal = package_run_terminal_outcome(store.state(), &task_run_id).unwrap();
    assert_eq!(terminal.outcome.status, OutcomeStatus::Succeeded);
    assert_eq!(
        serde_json::to_vec(&terminal.outcome.artifact_records).unwrap(),
        serde_json::to_vec(&durable_emitted_artifacts(db, &task_run_id)).unwrap(),
        "recovered outcome still carries the durable records byte-intact"
    );
}

#[test]
fn query_returns_the_terminal_outcome_by_run_id() {
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    let plan = package_plan("plan-query");
    let task_run_id = package_route_run_id(&plan.plan_id);

    assert_eq!(
        package_run_terminal_outcome(store.state(), &task_run_id),
        None
    );
    complete_run(db, &mut store, &plan, Arc::new(Mutex::new(Vec::new())));

    let terminal = package_run_terminal_outcome(store.state(), &task_run_id).unwrap();
    assert_eq!(terminal.network_id, NETWORK_ID);
    assert_eq!(
        terminal.task_instance_id,
        package_run_task_instance_id(&task_run_id)
    );
    assert_eq!(terminal.outcome.status, OutcomeStatus::Succeeded);
    assert_eq!(
        package_run_terminal_outcome(store.state(), "package-route::plan-unknown"),
        None
    );
}

/// THE proof-unblocking assertion: aggregate production accepts the recorded
/// terminal outcome for a completed fixture run and the aggregate publishes.
#[test]
fn recorded_outcome_unblocks_aggregate_publication() {
    struct TestEvents {
        append: EventAppendCapability,
    }

    impl EventAppendSink for TestEvents {
        fn ledger_identity(&self) -> LedgerIdentity {
            self.append.ledger_identity()
        }

        fn append_envelope_idempotent(
            &self,
            envelope: EventEnvelope,
        ) -> Result<AppendReceipt, String> {
            EventAppendSink::append_envelope_idempotent(&self.append, envelope)
        }
    }

    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    let plan = package_plan("plan-proof");
    let task_run_id = package_route_run_id(&plan.plan_id);

    complete_run(
        db.clone(),
        &mut store,
        &plan,
        Arc::new(Mutex::new(Vec::new())),
    );

    // Root integration shape: durable snapshot from the progress store, the
    // terminal outcome from the task-network query, and the typed binding.
    let progress = TaskProgressStore::open(db.clone()).unwrap();
    let snapshot = progress.load(&task_run_id).unwrap().unwrap();
    let terminal = package_run_terminal_outcome(store.state(), &task_run_id).unwrap();
    let binding = AggregateRunBinding {
        package_run_id: task_run_id.clone(),
        network_id: terminal.network_id.clone(),
        selected_scope: DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
        folder_unit_capability_types: vec!["docs.write".to_string()],
        semantic_yield_source: None,
    };

    let production =
        produce_aggregate_outcome(&snapshot, Some(&terminal.outcome), &binding).unwrap();
    let AggregateProduction::Produced(aggregate) = production else {
        panic!("recorded terminal outcome must produce the aggregate: {production:?}");
    };
    assert_eq!(aggregate.status, AggregatePackageStatus::Completed);
    assert_eq!(aggregate.folder_results.len(), 3, "one row per folder");
    for folder_result in &aggregate.folder_results {
        assert_eq!(folder_result.outcome_id, terminal.outcome.outcome_id);
        assert_eq!(folder_result.artifact_ids.len(), 1);
    }

    let tempdir = tempfile::tempdir().unwrap();
    let events_db = sled::open(tempdir.path()).unwrap();
    let authority = EventAuthority::open(events_db, EventAuthorityOpenOptions::default()).unwrap();
    let events = TestEvents {
        append: authority.append_capability(),
    };
    let outbox = AggregatePublicationStore::open(open_db()).unwrap();
    let result = publish_aggregate(
        &outbox,
        &events,
        &PublishAggregateRequest {
            session_id: "session-terminal".to_string(),
            worker_id: WORKER_ID.to_string(),
        },
        &aggregate,
    )
    .unwrap();
    assert!(
        matches!(result, AggregatePublishResult::Published { .. }),
        "aggregate must publish: {result:?}"
    );
}
