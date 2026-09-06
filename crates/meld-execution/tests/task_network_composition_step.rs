//! Task-path parity for the bounded package-step contract.
//!
//! These tests prove the second consumer of the frozen contract over a
//! committed Task Network graph produced by direct Task admission lowering: a
//! branching sibling fan-out of three children finalizes before their parent,
//! matching the package route's dependency-respecting execution shape over an
//! equivalent logical tree; committed edges carry their recorded Semantic
//! origin end to end; steps never exceed the wave budget; reopen between
//! every wave resumes without repeating completed invocations; and a harness
//! driving either implementor sees the same contract semantics.

use async_trait::async_trait;
use futures::executor::block_on;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog, CapabilityInvocationPayload, CapabilityInvocationResult,
    CapabilityTypeContract, ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec,
    OutputSlotSpec, ScopeContract, SuppliedValueRef,
};
use meld_execution::error::ExecutionInvariantError;
use meld_execution::task::{
    ArtifactProducerRef, ArtifactRecord, CompiledTaskDelta, CompiledTaskRecord,
    DurablePackageExecution, PackageStepInvoker, TaskArtifactRepo, TaskCompiler,
    TaskDependencyEdge, TaskDependencyKind, TaskExpansionRequest, TaskInitializationPayload,
    TaskRunContext,
};
use meld_execution::task_admission::{
    ExecutionTask, TaskAdmissionApi, TaskAdmissionLineage, TaskAdmissionLowerer,
    TaskAdmissionLoweringPlan, TaskAdmissionRecord, TaskAdmissionRequest,
    TaskAdmissionRuntimeActor, TaskAdmissionRuntimeRequest,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest, Response};
use meld_execution::task_network::composition_step::{
    CompositionInvocationError, CompositionInvocationOutcome, CompositionNetworkExecution,
    CompositionStepError, CompositionTaskInvoker,
};
use meld_execution::task_network::dispatch::Claim;
use meld_execution::task_network::dispatch_actor::{
    dispatch_claim_repo_id, DispatchPortError, TaskNetworkCommandPort,
};
use meld_execution::task_network::mutation::{Inject, Mutation, Set};
use meld_execution::task_network::package_step::{PackageStep, PackageStepRequest};
use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::state::{
    DependencyEdge, DependencyEdgeOrigin, DependencyKind, NetworkState, TaskNode, TaskStatus,
};
use meld_execution::task_network::store::{InMemoryTaskNetworkStore, SledTaskNetworkStore};
use meld_lang::{
    Bindings, CapabilityRef, Composition, CostEstimate, Edge, EdgeKind, Operator, Resolution,
    SlotConstraint, Step, StepKind, Term,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

const NETWORK_ID: &str = "network-fanout";
const WORKER_ID: &str = "worker-fanout";
const CHILD_STEPS: [&str; 3] = ["child_a", "child_b", "child_c"];

fn open_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn request(max_ready_invocations: usize) -> PackageStepRequest {
    PackageStepRequest {
        max_ready_invocations,
    }
}

// Task fixture lowered through direct admission lowering: three sibling
// children fan out first, child_a feeds the parent through a data flow edge,
// and child_b and child_c gate the parent through ordering edges, matching
// the package route's artifact-plus-effect child-before-parent tree.

fn child_contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: "fanout.child".to_string(),
        capability_version: 1,
        owning_domain: "fanout".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "node".to_string(),
            scope_ref_kind: "node_id".to_string(),
            allow_fan_out: false,
        },
        binding_contract: vec![],
        input_contract: vec![],
        output_contract: vec![OutputSlotSpec {
            slot_id: "out".to_string(),
            artifact_type_id: "branch_note".to_string(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Queued,
            completion_semantics: "result_or_failure".to_string(),
            retry_class: "provider_io".to_string(),
            cancellation_supported: true,
        },
    }
}

fn parent_contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: "fanout.parent".to_string(),
        capability_version: 1,
        owning_domain: "fanout".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "node".to_string(),
            scope_ref_kind: "node_id".to_string(),
            allow_fan_out: false,
        },
        binding_contract: vec![],
        input_contract: vec![InputSlotSpec {
            slot_id: "summary".to_string(),
            accepted_artifact_type_ids: vec!["branch_note".to_string()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
            required: true,
            cardinality: InputCardinality::One,
        }],
        output_contract: vec![OutputSlotSpec {
            slot_id: "rollup".to_string(),
            artifact_type_id: "rollup_note".to_string(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Queued,
            completion_semantics: "result_or_failure".to_string(),
            retry_class: "provider_io".to_string(),
            cancellation_supported: true,
        },
    }
}

fn fanout_catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog.register(child_contract()).unwrap();
    catalog.register(parent_contract()).unwrap();
    catalog
}

fn operator_step(step_id: &str, output_artifact_type_id: &str) -> Step {
    let capability_type_id = if step_id == "parent" {
        "fanout.parent"
    } else {
        "fanout.child"
    };
    Step {
        step_id: step_id.to_string(),
        kind: StepKind::Op(Operator {
            operator_id: step_id.to_string(),
            preconditions: vec![],
            effects: vec![],
            cost: CostEstimate::zero(),
            resolution: Resolution {
                requires_inputs: vec![],
                requires_outputs: vec![SlotConstraint {
                    artifact_type: Term::ArtifactType(output_artifact_type_id.to_string()),
                    required: true,
                }],
                scope_kind: Some("node".to_string()),
                tags: vec![],
                specific: Some(CapabilityRef {
                    capability_type_id: capability_type_id.to_string(),
                    capability_version: 1,
                }),
            },
        }),
    }
}

fn fanout_composition(task_id: &str, steps: Vec<Step>, edges: Vec<Edge>) -> ExecutionTask {
    let catalog = fanout_catalog();
    let mut authority_requirements = steps
        .iter()
        .filter_map(|step| match &step.kind {
            StepKind::Op(operator) => operator
                .resolution
                .specific
                .as_ref()
                .map(|specific| specific.capability_type_id.clone()),
            StepKind::Goal(_) => None,
        })
        .collect::<Vec<_>>();
    authority_requirements.sort();
    authority_requirements.dedup();
    let mut capability_contract_ids = authority_requirements
        .iter()
        .map(|action| catalog.get(action, 1).unwrap().content_identity())
        .collect::<Vec<_>>();
    capability_contract_ids.sort();
    ExecutionTask {
        initial_inputs: Vec::new(),
        task_id: task_id.to_string(),
        composition: Composition { steps, edges },
        bindings: Bindings::empty()
            .bind(
                "subject".to_string(),
                Term::Dimension("scope-fanout".to_string()),
            )
            .unwrap(),
        capability_contract_ids,
        expected_outcome_contract_id: "test.outcome".to_string(),
        authority_requirements,
        idempotency_key: format!("{task_id}::once"),
    }
}

fn branching_fanout_composition() -> ExecutionTask {
    let steps = vec![
        operator_step("child_a", "branch_note"),
        operator_step("child_b", "branch_note"),
        operator_step("child_c", "branch_note"),
        operator_step("parent", "rollup_note"),
    ];
    let edges = vec![
        Edge {
            from: "child_a".to_string(),
            to: "parent".to_string(),
            kind: EdgeKind::DataFlow {
                artifact_type: Term::ArtifactType("branch_note".to_string()),
            },
        },
        Edge {
            from: "child_b".to_string(),
            to: "parent".to_string(),
            kind: EdgeKind::Ordering,
        },
        Edge {
            from: "child_c".to_string(),
            to: "parent".to_string(),
            kind: EdgeKind::Ordering,
        },
    ];
    fanout_composition("composition-fanout", steps, edges)
}

fn admission_request(task: ExecutionTask) -> TaskAdmissionRequest {
    let task_id = task.task_id.clone();
    TaskAdmissionRequest {
        lineage: TaskAdmissionLineage {
            agent_id: "agent-fanout".to_string(),
            goal_id: "goal-fanout".to_string(),
            plan_revision_id: "plan-fanout-v1".to_string(),
            product_id: task_id.clone(),
            authorization_id: format!("authorization::{task_id}"),
            context_id: "context-fanout".to_string(),
            authority_scope_id: "scope-fanout".to_string(),
            authority_policy_content_hash: String::new(),
            authority_decision: None,
            activation_generation: "generation-fanout".to_string(),
            admission_epoch: None,
        },
        idempotency_key: task.idempotency_key.clone(),
        task,
    }
}

fn lower_record(admission: &TaskAdmissionRecord) -> TaskAdmissionLoweringPlan {
    let lowerer = TaskAdmissionLowerer::new(TaskCompiler::new(), fanout_catalog());
    lowerer.lower(NETWORK_ID, admission)
}

fn command_for_state(state: &NetworkState, command_id: &str, command: Command) -> CommandRequest {
    CommandRequest {
        command_id: command_id.to_string(),
        network_id: state.network_id.clone(),
        base_revision: state.revision,
        base_state_hash: state.state_hash.clone(),
        read_preconditions: vec![],
        command,
    }
}

fn commit_branching_fanout(store: &mut InMemoryTaskNetworkStore) {
    let admission = TaskAdmissionApi::new(store, &fanout_catalog(), "generation-fanout", "")
        .admit(admission_request(branching_fanout_composition()))
        .unwrap();
    assert_eq!(
        admission.decision,
        meld_execution::task_admission::TaskAdmissionDecision::Admitted
    );
    let actor = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
        TaskCompiler::new(),
        fanout_catalog(),
    ));
    let report = actor
        .run_once_in_memory(
            store,
            TaskAdmissionRuntimeRequest {
                network_id: NETWORK_ID.to_string(),
                max_items: 1,
            },
        )
        .unwrap();
    assert_eq!(report.committed, 1, "{report:#?}");
}

fn commit_branching_fanout_sled(store: &mut SledTaskNetworkStore) {
    let admission = TaskAdmissionApi::new(store, &fanout_catalog(), "generation-fanout", "")
        .admit(admission_request(branching_fanout_composition()))
        .unwrap();
    assert_eq!(
        admission.decision,
        meld_execution::task_admission::TaskAdmissionDecision::Admitted
    );
    let actor = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
        TaskCompiler::new(),
        fanout_catalog(),
    ));
    let report = actor
        .run_once(
            store,
            TaskAdmissionRuntimeRequest {
                network_id: NETWORK_ID.to_string(),
                max_items: 1,
            },
        )
        .unwrap();
    assert_eq!(report.committed, 1, "{report:#?}");
}

fn task_ids_by_step(state: &NetworkState) -> BTreeMap<String, String> {
    state
        .tasks
        .iter()
        .map(|(task_instance_id, node)| (node.lineage.step_id.clone(), task_instance_id.clone()))
        .collect()
}

// Both invokers script the same fixture semantics per logical node so parity
// compares content, not transport: children emit one branch note keyed by
// their logical name, and the parent echoes the summary it was fed.

fn child_content(step_id: &str) -> Value {
    json!({ "note": step_id })
}

fn parent_content(summary: &Value) -> Value {
    json!({ "rollup": summary })
}

struct FanOutInvoker {
    log: Arc<Mutex<Vec<String>>>,
}

impl FanOutInvoker {
    fn artifact(
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> ArtifactRecord {
        let step_id = node.lineage.step_id.clone();
        let (artifact_type_id, content) = if step_id == "parent" {
            let summary = init_payload
                .init_artifacts
                .iter()
                .find(|artifact| artifact.init_slot_id == "summary")
                .expect("parent init payload carries the summary slot")
                .content
                .clone();
            ("rollup_note".to_string(), parent_content(&summary))
        } else {
            ("branch_note".to_string(), child_content(&step_id))
        };
        ArtifactRecord {
            artifact_id: format!("{}::out", claim.claim_id),
            artifact_type_id,
            schema_version: 1,
            content,
            producer: ArtifactProducerRef {
                task_id: node.compiled_task.task_id.clone(),
                capability_instance_id: step_id,
                invocation_id: Some(format!("{}::invoke", claim.claim_id)),
                output_slot_id: Some("out".to_string()),
            },
        }
    }
}

#[async_trait]
impl CompositionTaskInvoker for FanOutInvoker {
    async fn invoke_composition_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<CompositionInvocationOutcome, CompositionInvocationError> {
        self.log.lock().unwrap().push(node.lineage.step_id.clone());
        Ok(CompositionInvocationOutcome::Completed(vec![
            Self::artifact(node, claim, init_payload),
        ]))
    }
}

/// Resolves one named step to a bounded failure on every attempt.
struct FailingStepInvoker {
    failing_step_id: String,
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl CompositionTaskInvoker for FailingStepInvoker {
    async fn invoke_composition_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<CompositionInvocationOutcome, CompositionInvocationError> {
        self.log.lock().unwrap().push(node.lineage.step_id.clone());
        if node.lineage.step_id == self.failing_step_id {
            return Ok(CompositionInvocationOutcome::Failed {
                error: "provider failed".to_string(),
            });
        }
        Ok(CompositionInvocationOutcome::Completed(vec![
            FanOutInvoker::artifact(node, claim, init_payload),
        ]))
    }
}

/// Simulates a crash between artifact persistence and outcome recording: the
/// next outcome commands are lost before they reach the store.
struct FailOutcomePort {
    inner: InMemoryTaskNetworkStore,
    failures_remaining: usize,
}

impl TaskNetworkCommandPort for FailOutcomePort {
    fn network_state(&self) -> &NetworkState {
        self.inner.state()
    }

    fn submit_command(&mut self, request: CommandRequest) -> Result<Response, DispatchPortError> {
        if self.failures_remaining > 0 && matches!(request.command, Command::RecordTaskOutcome(_)) {
            self.failures_remaining -= 1;
            return Err(DispatchPortError::retryable(
                "simulated crash before outcome record",
            ));
        }
        Ok(self.inner.submit(request))
    }
}

// Package-route fixture over the equivalent logical tree: the same three
// children and one parent, with the artifact edge from child_a and effect
// edges from child_b and child_c, driven by the first implementor.

fn package_instance(
    capability_instance_id: &str,
    capability_type_id: &str,
    input_wiring: Vec<BoundInputWiring>,
) -> BoundCapabilityInstance {
    BoundCapabilityInstance {
        capability_instance_id: capability_instance_id.to_string(),
        capability_type_id: capability_type_id.to_string(),
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

fn package_tree() -> CompiledTaskRecord {
    CompiledTaskRecord {
        task_id: "task_fanout".to_string(),
        task_version: 1,
        init_slots: vec![],
        capability_instances: vec![
            package_instance("child_a", "fanout.child", vec![]),
            package_instance("child_b", "fanout.child", vec![]),
            package_instance("child_c", "fanout.child", vec![]),
            package_instance(
                "parent",
                "fanout.parent",
                vec![BoundInputWiring {
                    slot_id: "summary".to_string(),
                    sources: vec![BoundInputWiringSource::UpstreamOutput {
                        capability_instance_id: "child_a".to_string(),
                        output_slot_id: "out".to_string(),
                        artifact_type_id: "branch_note".to_string(),
                        schema_version: 1,
                    }],
                }],
            ),
        ],
        dependency_edges: vec![
            package_edge("child_a", "parent", TaskDependencyKind::Artifact),
            package_edge("child_b", "parent", TaskDependencyKind::Effect),
            package_edge("child_c", "parent", TaskDependencyKind::Effect),
        ],
    }
}

fn package_payload(task_run_id: &str) -> TaskInitializationPayload {
    TaskInitializationPayload {
        task_id: "task_fanout".to_string(),
        compiled_task_ref: "compiled_task_fanout".to_string(),
        init_artifacts: vec![],
        task_run_context: TaskRunContext {
            task_run_id: task_run_id.to_string(),
            session_id: None,
            trigger: "test".to_string(),
        },
    }
}

struct PackageTreeInvoker {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl PackageStepInvoker for PackageTreeInvoker {
    async fn invoke_capability(
        &self,
        _instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ExecutionInvariantError> {
        let capability_instance_id = payload.capability_instance_id.clone();
        self.log
            .lock()
            .unwrap()
            .push(capability_instance_id.clone());
        let (artifact_type_id, content) = if capability_instance_id == "parent" {
            let summary = payload
                .supplied_inputs
                .iter()
                .find(|input| input.slot_id == "summary")
                .map(|input| match &input.value {
                    SuppliedValueRef::StructuredValue(value) => value.clone(),
                    SuppliedValueRef::Artifact(artifact) => artifact.content.clone(),
                })
                .expect("parent invocation carries the summary input");
            ("rollup_note".to_string(), parent_content(&summary))
        } else {
            (
                "branch_note".to_string(),
                child_content(&capability_instance_id),
            )
        };
        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![ArtifactRecord {
                artifact_id: format!("{}::out", payload.invocation_id),
                artifact_type_id,
                schema_version: 1,
                content,
                producer: ArtifactProducerRef {
                    task_id: "task_fanout".to_string(),
                    capability_instance_id,
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
    ) -> Result<CompiledTaskDelta, ExecutionInvariantError> {
        Ok(CompiledTaskDelta::default())
    }
}

fn assert_children_before_parent(log: &[String]) {
    let parent = log
        .iter()
        .position(|entry| entry == "parent")
        .unwrap_or_else(|| panic!("missing parent invocation in {log:?}"));
    for child in CHILD_STEPS {
        let position = log
            .iter()
            .position(|entry| entry == child)
            .unwrap_or_else(|| panic!("missing child '{child}' invocation in {log:?}"));
        assert!(
            position < parent,
            "child '{child}' must finalize before the parent in {log:?}"
        );
    }
}

fn composition_contents_by_step(state: &NetworkState) -> BTreeMap<String, Value> {
    state
        .tasks
        .iter()
        .map(|(task_instance_id, node)| {
            let Some(TaskStatus::Succeeded { outcome_id }) = state.statuses.get(task_instance_id)
            else {
                panic!("task '{task_instance_id}' did not succeed");
            };
            let records = &state.outcomes[outcome_id].artifact_records;
            assert_eq!(records.len(), 1);
            (node.lineage.step_id.clone(), records[0].content.clone())
        })
        .collect()
}

fn package_contents_by_instance(
    run: &DurablePackageExecution<PackageTreeInvoker>,
) -> BTreeMap<String, Value> {
    run.executor()
        .artifact_repo()
        .record()
        .artifacts
        .iter()
        .filter(|artifact| artifact.producer.capability_instance_id != "__task_init__")
        .map(|artifact| {
            (
                artifact.producer.capability_instance_id.clone(),
                artifact.content.clone(),
            )
        })
        .collect()
}

/// Drives one implementor to completion under one budget, asserting the
/// shared contract semantics both implementors must expose: zero-budget steps
/// attempt nothing, no step exceeds its budget, completion is observable via
/// `is_complete`, and steps after completion attempt no work.
fn drive_and_assert_contract_semantics<S>(stepper: &mut S, budget: usize) -> usize
where
    S: PackageStep,
    S::Error: std::fmt::Debug,
{
    assert!(!stepper.is_complete());
    let zero = block_on(stepper.step(&request(0))).unwrap();
    assert_eq!(zero.items_attempted, 0);
    assert_eq!(zero.items_committed, 0);
    assert!(zero.budget_exhausted);
    assert!(!zero.package_complete);
    assert_eq!(zero.input_progress, zero.output_progress);

    let mut steps = 0;
    loop {
        let report = block_on(stepper.step(&request(budget))).unwrap();
        assert!(
            report.items_attempted <= budget,
            "budget of {budget} must hold"
        );
        steps += 1;
        assert!(steps < 20, "bounded run must converge");
        if report.package_complete {
            break;
        }
        assert!(!stepper.is_complete());
    }
    assert!(stepper.is_complete());

    for _ in 0..2 {
        let idle = block_on(stepper.step(&request(budget))).unwrap();
        assert_eq!(idle.items_attempted, 0);
        assert_eq!(idle.items_committed, 0);
        assert!(idle.package_complete);
        assert!(!idle.budget_exhausted);
        assert_eq!(idle.input_progress, idle.output_progress);
    }
    steps
}

#[test]
fn lowering_commits_branching_fan_out_with_semantic_origin_edges() {
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    commit_branching_fanout(&mut store);

    let state = store.state();
    assert_eq!(state.tasks.len(), 4);
    assert_eq!(state.edges.len(), 3);
    let by_step = task_ids_by_step(state);
    let parent_id = &by_step["parent"];
    let mut kinds = Vec::new();
    for edge in &state.edges {
        assert_eq!(&edge.to, parent_id, "every edge gates the parent");
        assert_eq!(
            edge.origin,
            DependencyEdgeOrigin::Semantic,
            "lowered composition edges must record their Semantic origin"
        );
        kinds.push(edge.kind.clone());
    }
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| matches!(
                kind,
                DependencyKind::DataFlow { artifact_type_id } if artifact_type_id == "branch_note"
            ))
            .count(),
        1
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| matches!(kind, DependencyKind::Ordering))
            .count(),
        2
    );

    let ready = compute_ready_set(state);
    assert_eq!(ready.task_instance_ids.len(), 3, "three siblings fan out");
    assert!(!ready.task_instance_ids.contains(parent_id));
}

#[test]
fn branching_composition_matches_package_route_shape_over_equivalent_tree() {
    // Composition path: committed graph from real lowering, driven by the
    // second implementor in bounded waves of two.
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    commit_branching_fanout(&mut store);
    let composition_log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = CompositionNetworkExecution::new(
        WORKER_ID,
        open_db(),
        store,
        FanOutInvoker {
            log: composition_log.clone(),
        },
    )
    .unwrap();

    let first = block_on(stepper.step(&request(2))).unwrap();
    assert_eq!(first.items_attempted, 2);
    assert_eq!(first.items_committed, 2);
    assert!(first.budget_exhausted, "a third sibling stayed ready");
    assert!(!first.package_complete);

    let second = block_on(stepper.step(&request(2))).unwrap();
    assert_eq!(
        second.items_attempted, 1,
        "the parent readied by this wave's outcomes waits for a later step"
    );
    assert!(!second.budget_exhausted);
    assert!(!second.package_complete);

    let third = block_on(stepper.step(&request(2))).unwrap();
    assert_eq!(third.items_attempted, 1);
    assert!(third.package_complete);
    assert!(stepper.is_complete());

    let progress = stepper.progress();
    assert_eq!(progress.known_units, 4);
    assert_eq!(progress.completed_units, 4);
    assert_eq!(progress.ready_units, 0);
    assert_eq!(progress.applied_expansions, 0, "no expansion machinery ran");
    assert_eq!(progress.persisted_artifacts, 4);

    // Package route: the first implementor over the equivalent logical tree.
    let package_log = Arc::new(Mutex::new(Vec::new()));
    let mut package_run = DurablePackageExecution::open(
        open_db(),
        package_tree(),
        package_payload("taskrun_fanout_parity"),
        PackageTreeInvoker {
            log: package_log.clone(),
        },
    )
    .unwrap();
    loop {
        if block_on(package_run.step(&request(2)))
            .unwrap()
            .package_complete
        {
            break;
        }
    }

    // Same completed logical set, same dependency-respecting shape.
    let composition_invocations = composition_log.lock().unwrap().clone();
    let package_invocations = package_log.lock().unwrap().clone();
    assert_eq!(
        composition_invocations.iter().collect::<BTreeSet<_>>(),
        package_invocations.iter().collect::<BTreeSet<_>>()
    );
    assert_children_before_parent(&composition_invocations);
    assert_children_before_parent(&package_invocations);

    // Per-node artifact semantics match over the same logical tree: the
    // parent's rollup embeds child_a's note on both paths.
    let state = stepper.network().state();
    let composition_contents = composition_contents_by_step(state);
    let package_contents = package_contents_by_instance(&package_run);
    assert_eq!(composition_contents, package_contents);
    assert_eq!(
        composition_contents["parent"],
        parent_content(&child_content("child_a"))
    );

    // Committed edges carry their recorded origin end to end.
    assert!(state
        .edges
        .iter()
        .all(|edge| edge.origin == DependencyEdgeOrigin::Semantic));
}

#[test]
fn reopen_between_every_wave_resumes_without_repeating_completed_invocations() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    {
        let mut store = SledTaskNetworkStore::open(db.clone(), NETWORK_ID).unwrap();
        commit_branching_fanout_sled(&mut store);
    }

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut steps = 0;
    loop {
        // A fresh store and a fresh implementor per wave prove reopen-resume:
        // nothing survives between waves except the shared database.
        let store = SledTaskNetworkStore::open(db.clone(), NETWORK_ID).unwrap();
        let mut stepper = CompositionNetworkExecution::new(
            WORKER_ID,
            db.clone(),
            store,
            FanOutInvoker { log: log.clone() },
        )
        .unwrap();
        let report = block_on(stepper.step(&request(1))).unwrap();
        assert!(report.items_attempted <= 1, "budget of one must hold");
        steps += 1;
        assert!(steps < 20, "reopened run must converge");
        if report.package_complete {
            break;
        }
    }

    assert_eq!(steps, 4, "one release per wave over four work units");
    let invoked = log.lock().unwrap().clone();
    assert_eq!(invoked.len(), 4, "no completed invocation may repeat");
    assert_eq!(invoked.iter().collect::<BTreeSet<_>>().len(), 4);
    assert_children_before_parent(&invoked);

    let store = SledTaskNetworkStore::open(db.clone(), NETWORK_ID).unwrap();
    let reopened =
        CompositionNetworkExecution::new(WORKER_ID, db, store, FanOutInvoker { log }).unwrap();
    assert!(reopened.is_complete());
    assert_eq!(
        composition_contents_by_step(reopened.network().state())["parent"],
        parent_content(&child_content("child_a"))
    );
}

#[test]
fn crash_between_artifact_persist_and_outcome_record_resumes_running_claim_first() {
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    commit_branching_fanout(&mut store);
    let log = Arc::new(Mutex::new(Vec::new()));

    // Crash window: the first sibling's claim is fenced and its artifact is
    // durable, but the outcome command never reaches the store.
    let mut stepper = CompositionNetworkExecution::new(
        WORKER_ID,
        db.clone(),
        FailOutcomePort {
            inner: store,
            failures_remaining: 1,
        },
        FanOutInvoker { log: log.clone() },
    )
    .unwrap();
    let error = block_on(stepper.step(&request(3))).unwrap_err();
    assert!(matches!(error, CompositionStepError::CommandPort(_)));
    let store = stepper.into_network().inner;

    let running_claims: Vec<Claim> = store
        .state()
        .claims
        .values()
        .filter(|claim| {
            matches!(
                store.state().statuses.get(&claim.task_instance_id),
                Some(TaskStatus::Running { claim_id }) if *claim_id == claim.claim_id
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        running_claims.len(),
        1,
        "one fenced claim survives the crash"
    );
    let crashed_claim = running_claims[0].clone();
    let crashed_step = store.state().tasks[&crashed_claim.task_instance_id]
        .lineage
        .step_id
        .clone();
    assert!(store.state().outcomes.is_empty());
    let artifact_id = format!("{}::out", crashed_claim.claim_id);
    let repo =
        TaskArtifactRepo::open_sled(db.clone(), dispatch_claim_repo_id(&crashed_claim.claim_id))
            .unwrap();
    assert!(repo.get_artifact(&artifact_id).is_some());
    drop(repo);

    // Replay: a fresh implementor resumes the fenced claim before claiming
    // new ready work, and the byte-identical re-emitted artifact is accepted
    // as durable replay.
    let mut stepper = CompositionNetworkExecution::new(
        WORKER_ID,
        db.clone(),
        store,
        FanOutInvoker { log: log.clone() },
    )
    .unwrap();
    let report = block_on(stepper.step(&request(3))).unwrap();
    assert_eq!(report.items_attempted, 3);
    assert_eq!(report.items_committed, 3);

    let invoked = log.lock().unwrap().clone();
    assert_eq!(invoked[0], crashed_step, "the crashed claim ran first");
    assert_eq!(
        invoked[1], crashed_step,
        "the resumed claim runs before new ready claims"
    );
    assert_eq!(invoked.len(), 4, "three siblings plus one replay");

    let state = stepper.network().state();
    assert_eq!(
        state.claims.len(),
        3,
        "one fenced claim per sibling, no second"
    );
    assert!(matches!(
        state.statuses[&crashed_claim.task_instance_id],
        TaskStatus::Succeeded { .. }
    ));
    let repo =
        TaskArtifactRepo::open_sled(db, dispatch_claim_repo_id(&crashed_claim.claim_id)).unwrap();
    assert_eq!(
        repo.record()
            .artifacts
            .iter()
            .filter(|artifact| artifact.artifact_id == artifact_id)
            .count(),
        1,
        "duplicate dispatch must not duplicate domain outputs"
    );

    let last = block_on(stepper.step(&request(3))).unwrap();
    assert!(last.package_complete);
}

#[test]
fn harness_sees_same_contract_semantics_across_both_implementors() {
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    commit_branching_fanout(&mut store);
    let mut composition_stepper = CompositionNetworkExecution::new(
        WORKER_ID,
        open_db(),
        store,
        FanOutInvoker {
            log: Arc::new(Mutex::new(Vec::new())),
        },
    )
    .unwrap();

    let mut package_stepper = DurablePackageExecution::open(
        open_db(),
        package_tree(),
        package_payload("taskrun_fanout_contract"),
        PackageTreeInvoker {
            log: Arc::new(Mutex::new(Vec::new())),
        },
    )
    .unwrap();

    let composition_steps = drive_and_assert_contract_semantics(&mut composition_stepper, 2);
    let package_steps = drive_and_assert_contract_semantics(&mut package_stepper, 2);
    assert_eq!(
        composition_steps, package_steps,
        "both implementors converge in the same bounded wave count over the equivalent tree"
    );
}

#[test]
fn public_mutation_cannot_extend_an_admitted_region() {
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    commit_branching_fanout(&mut store);
    let parent_task_id = task_ids_by_step(store.state())["parent"].clone();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = CompositionNetworkExecution::new(
        WORKER_ID,
        open_db(),
        store,
        FanOutInvoker { log: log.clone() },
    )
    .unwrap();

    let first = block_on(stepper.step(&request(3))).unwrap();
    assert_eq!(first.items_committed, 3, "the sibling wave completes first");
    assert_eq!(stepper.progress().known_units, 4);

    // A second admitted Task cannot attach executable content through the
    // generic graph command, even when its lineage and step are authentic.
    let late_task = fanout_composition(
        "composition-fanout-late",
        vec![operator_step("late", "branch_note")],
        vec![],
    );
    let mut store = stepper.into_network();
    let late_admission =
        TaskAdmissionApi::new(&mut store, &fanout_catalog(), "generation-fanout", "")
            .admit(admission_request(late_task))
            .unwrap();
    let late_plan = lower_record(&late_admission);
    assert!(
        late_plan.diagnostics.is_empty(),
        "{:?}",
        late_plan.diagnostics
    );
    let [Mutation::Inject(lowered)] = late_plan.mutations.mutations.as_slice() else {
        panic!("late composition lowers to one inject mutation");
    };
    let inject = Inject::new(
        lowered.task_node.clone(),
        vec![DependencyEdge {
            from: parent_task_id.clone(),
            to: lowered.task_node.task_instance_id.clone(),
            kind: DependencyKind::Ordering,
            origin: DependencyEdgeOrigin::Semantic,
        }],
    );
    let mutations = Set::new(
        NETWORK_ID,
        late_plan.mutations.source_task_id.clone(),
        late_plan.mutations.idempotency_key.clone(),
        vec![Mutation::Inject(inject)],
    );
    let command = command_for_state(
        store.state(),
        "command-commit-late",
        Command::ApplyMutationSet(mutations),
    );
    assert!(matches!(store.submit(command), Response::Rejected(_)));
    assert_eq!(store.state().tasks.len(), 4);
    assert!(!store
        .state()
        .tasks
        .contains_key(&lowered.task_node.task_instance_id));
}

#[test]
fn failed_invocation_records_failed_outcome_without_commit_or_requeue() {
    let mut store = InMemoryTaskNetworkStore::new(NETWORK_ID);
    commit_branching_fanout(&mut store);
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stepper = CompositionNetworkExecution::new(
        WORKER_ID,
        open_db(),
        store,
        FailingStepInvoker {
            failing_step_id: "child_b".to_string(),
            log: log.clone(),
        },
    )
    .unwrap();

    let report = block_on(stepper.step(&request(3))).unwrap();
    assert_eq!(report.items_attempted, 3);
    assert_eq!(report.items_committed, 2, "failed work is not progress");
    assert!(!report.package_complete);
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|step| step.as_str() == "child_b")
            .count(),
        1,
        "a failed task must not re-enter the same step"
    );
    let failed_task_id = task_ids_by_step(stepper.network().state())["child_b"].clone();
    assert!(matches!(
        &stepper.network().state().statuses[&failed_task_id],
        TaskStatus::Failed { error, .. } if error == "provider failed"
    ));

    // Retry policy stays deferred: with the parent blocked by the failure,
    // a later step finds no eligible work and attempts nothing.
    let idle = block_on(stepper.step(&request(3))).unwrap();
    assert_eq!(idle.items_attempted, 0);
    assert!(!idle.package_complete);
    assert!(!idle.budget_exhausted);
    assert!(!stepper.is_complete());
}
