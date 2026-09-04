//! Bounded dispatch over durable Task Network claims.

#[path = "support/task_network.rs"]
mod task_network_support;

use async_trait::async_trait;
use futures::executor::block_on;
use meld_execution::task::TaskCompiler;
use meld_execution::task::{
    ArtifactProducerRef, ArtifactRecord, TaskArtifactRepo, TaskInitializationPayload,
};
use meld_execution::task_admission::{
    ExecutionTask, TaskAdmissionApi, TaskAdmissionLineage, TaskAdmissionLowerer,
    TaskAdmissionRecord, TaskAdmissionRequest, TaskAdmissionRuntimeActor,
    TaskAdmissionRuntimeRequest,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest, Response};
use meld_execution::task_network::dispatch::{Claim, OutcomeStatus, Request as DispatchRequest};
use meld_execution::task_network::dispatch_actor::{
    dispatch_claim_id, dispatch_claim_repo_id, dispatch_outcome_id, AdmissionGenerationObserver,
    ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchCheckpoint, DispatchPortError,
    DispatchRuntimeActor, DispatchTickRequest, TaskNetworkCommandPort,
};
use meld_execution::task_network::mutation::{Mutation, Set};
use meld_execution::task_network::state::{
    NetworkState, TaskAdmissionAttribution, TaskLineage, TaskNode, TaskStatus,
};
use meld_execution::task_network::store::InMemoryTaskNetworkStore;
use meld_lang::{AuthorityDecision, AuthorityPolicy, AuthorityPolicyBinding};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MutableGenerationObserver {
    generation: Arc<Mutex<Option<String>>>,
}

struct ReadOnlyNetwork {
    state: NetworkState,
}

impl TaskNetworkCommandPort for ReadOnlyNetwork {
    fn network_state(&self) -> &NetworkState {
        &self.state
    }

    fn submit_command(&mut self, _request: CommandRequest) -> Result<Response, DispatchPortError> {
        panic!("forged attribution must be rejected before command submission")
    }
}

impl AdmissionGenerationObserver for MutableGenerationObserver {
    fn active_generation(&self, _agent_id: &str) -> Result<Option<String>, String> {
        Ok(self.generation.lock().unwrap().clone())
    }
}

fn open_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn authority_policy() -> AuthorityPolicyBinding {
    let policy = AuthorityPolicy {
        policy_id: "docs-local".to_string(),
        principal_id: "workspace-owner".to_string(),
        subject: meld_events::DomainObjectRef::new("workspace", "node", "docs").unwrap(),
        principal_granted_action_ids: vec!["docs.write".to_string()],
        runtime_allowed_action_ids: vec!["docs.write".to_string()],
        restricted_action_ids: Vec::new(),
    };
    let hash = policy.content_hash().unwrap();
    AuthorityPolicyBinding::new(policy, hash).unwrap()
}

fn authority_decision(policy: &AuthorityPolicyBinding) -> AuthorityDecision {
    AuthorityDecision {
        policy_id: policy.policy.policy_id.clone(),
        policy_content_hash: policy.content_hash.clone(),
        principal_id: policy.policy.principal_id.clone(),
        subject: policy.policy.subject.clone(),
        requested_action_ids: vec!["docs.write".to_string()],
        authorized_action_ids: vec!["docs.write".to_string()],
    }
}

fn admitted_record(
    store: &mut InMemoryTaskNetworkStore,
    policy: &AuthorityPolicyBinding,
    generation: &str,
) -> TaskAdmissionRecord {
    let catalog = task_network_support::catalog();
    let mut task: ExecutionTask = task_network_support::composition();
    task.task_id = "task-alpha".to_string();
    task.idempotency_key = "task-alpha".to_string();
    TaskAdmissionApi::new(store, &catalog, generation, &policy.content_hash)
        .admit(TaskAdmissionRequest {
            lineage: TaskAdmissionLineage {
                agent_id: "agent-docs".to_string(),
                goal_id: "goal-docs".to_string(),
                plan_revision_id: "plan-docs-v1".to_string(),
                product_id: "task-alpha".to_string(),
                authorization_id: "authorization-docs-v1".to_string(),
                context_id: "context-docs-v1".to_string(),
                authority_scope_id: policy.policy.policy_id.clone(),
                authority_policy_content_hash: policy.content_hash.clone(),
                authority_decision: Some(authority_decision(policy)),
                activation_generation: generation.to_string(),
            },
            task,
            idempotency_key: "task-alpha".to_string(),
        })
        .unwrap()
}

fn admitted_node(record: &TaskAdmissionRecord) -> TaskNode {
    let mut node = task_network_support::single_task_node("task-alpha");
    node.lineage = TaskLineage::admitted(
        "write".to_string(),
        "write".to_string(),
        "docs.write".to_string(),
        1,
        record.request.lineage.authority_decision.clone(),
        TaskAdmissionAttribution::from_record(record),
    );
    node
}

fn realize_admitted(store: &mut InMemoryTaskNetworkStore) -> String {
    let catalog = task_network_support::catalog();
    let actor =
        TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(TaskCompiler::new(), catalog));
    let report = actor
        .run_once_in_memory(
            store,
            TaskAdmissionRuntimeRequest {
                network_id: "network-docs".to_string(),
                max_items: 1,
            },
        )
        .unwrap();
    assert_eq!(report.committed, 1, "{report:#?}");
    store.state().tasks.keys().next().unwrap().clone()
}

fn tick_request(sequence: u64, max_items: usize) -> DispatchTickRequest {
    DispatchTickRequest {
        sequence,
        max_items,
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
    claim_log: Arc<Mutex<Vec<String>>>,
}

impl DispatchFixture {
    fn new() -> Self {
        Self {
            claim_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn actor(&self, db: sled::Db) -> DispatchRuntimeActor<ScriptedClaimInvoker> {
        DispatchRuntimeActor::new(
            "worker-a",
            db,
            ScriptedClaimInvoker {
                log: self.claim_log.clone(),
            },
        )
        .unwrap()
    }

    fn claim_invocations(&self) -> Vec<String> {
        self.claim_log.lock().unwrap().clone()
    }
}

#[test]
fn claim_route_tick_never_claims_beyond_budget() {
    let fixture = DispatchFixture::new();
    let actor = fixture.actor(open_db());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    task_network_support::commit_single_task(&mut store, "task-beta");

    let report = block_on(actor.tick(&mut store, tick_request(1, 1))).unwrap();

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    assert!(report.budget_exhausted);
    assert_eq!(store.state().claims.len(), 1);
    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Succeeded { .. })
    ));
    assert_eq!(
        store.state().statuses.get("task-beta"),
        Some(&TaskStatus::Pending)
    );
    assert_eq!(fixture.claim_invocations(), vec!["task-alpha"]);
}

#[test]
fn claim_route_records_outcome_after_durable_artifacts() {
    let fixture = DispatchFixture::new();
    let db = open_db();
    let actor = fixture.actor(db.clone());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(&mut store, tick_request(1, 1))).unwrap();

    assert_eq!(report.items_committed, 1);
    let claim_id = dispatch_claim_id("network-docs", "task-alpha", 1, "worker-a");
    let outcome = store
        .state()
        .outcomes
        .get(&dispatch_outcome_id(&claim_id))
        .unwrap();
    assert_eq!(outcome.status, OutcomeStatus::Succeeded);
    let artifact_id = format!("{claim_id}::readme");
    assert_eq!(outcome.artifact_records[0].artifact_id, artifact_id);
    assert!(matches!(
        report.checkpoints.as_slice(),
        [DispatchCheckpoint::TaskOutcomeRecorded {
            status: OutcomeStatus::Succeeded,
            resumed: false,
            ..
        }]
    ));
    let repo = TaskArtifactRepo::open_sled(db, dispatch_claim_repo_id(&claim_id)).unwrap();
    assert!(repo.get_artifact(&artifact_id).is_some());
}

#[test]
fn crash_after_artifact_persist_replays_without_duplicate_outputs() {
    let fixture = DispatchFixture::new();
    let db = open_db();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let claim_id = dispatch_claim_id("network-docs", "task-alpha", 1, "worker-a");
    let artifact_id = format!("{claim_id}::readme");

    let actor = fixture.actor(db.clone());
    let crashed = block_on(actor.tick(
        &mut FailOutcomePort { inner: &mut store },
        tick_request(1, 1),
    ))
    .unwrap();
    assert_eq!(crashed.items_committed, 0);
    assert!(store.state().outcomes.is_empty());
    let repo = TaskArtifactRepo::open_sled(db.clone(), dispatch_claim_repo_id(&claim_id)).unwrap();
    assert!(repo.get_artifact(&artifact_id).is_some());

    let replayed = block_on(
        fixture
            .actor(db.clone())
            .tick(&mut store, tick_request(2, 1)),
    )
    .unwrap();
    assert_eq!(replayed.items_committed, 1);
    assert!(matches!(
        replayed.checkpoints.as_slice(),
        [DispatchCheckpoint::TaskOutcomeRecorded {
            resumed: true,
            status: OutcomeStatus::Succeeded,
            ..
        }]
    ));
    assert_eq!(store.state().claims.len(), 1);
    assert_eq!(store.state().outcomes.len(), 1);
    let repo = TaskArtifactRepo::open_sled(db, dispatch_claim_repo_id(&claim_id)).unwrap();
    assert_eq!(
        repo.record()
            .artifacts
            .iter()
            .filter(|artifact| artifact.artifact_id == artifact_id)
            .count(),
        1
    );
}

#[test]
fn failed_invocation_is_recorded_without_progress_or_same_tick_requeue() {
    let claim_log = Arc::new(Mutex::new(Vec::new()));
    let actor = DispatchRuntimeActor::new(
        "worker-a",
        open_db(),
        FailingClaimInvoker {
            log: claim_log.clone(),
        },
    )
    .unwrap();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(&mut store, tick_request(1, 5))).unwrap();

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 0);
    assert_eq!(claim_log.lock().unwrap().len(), 1);
    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Failed { error, .. }) if error == "provider failed"
    ));
}

#[test]
fn zero_budget_tick_attempts_nothing_and_reports_pending_work() {
    let fixture = DispatchFixture::new();
    let actor = fixture.actor(open_db());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(&mut store, tick_request(1, 0))).unwrap();

    assert_eq!(report.items_attempted, 0);
    assert_eq!(report.items_committed, 0);
    assert!(report.budget_exhausted);
    assert!(store.state().claims.is_empty());
    assert!(fixture.claim_invocations().is_empty());
}

#[test]
fn active_authority_policy_denies_unattributed_task_before_claim() {
    let fixture = DispatchFixture::new();
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(authority_policy());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let report = block_on(actor.tick(&mut store, tick_request(1, 1))).unwrap();

    assert!(report
        .fatal_errors
        .iter()
        .any(|issue| issue.code == "effective_authority_denied"));
    assert!(fixture.claim_invocations().is_empty());
    assert!(store.state().claims.is_empty());
}

#[test]
fn generic_task_cannot_carry_agent_authority_without_admission() {
    let fixture = DispatchFixture::new();
    let policy = authority_policy();
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(policy.clone());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let mut node = task_network_support::single_task_node("task-alpha");
    node.lineage.authority_decision = Some(authority_decision(&policy));
    let set = Set::new(
        "network-docs",
        "task-alpha",
        "inject::command-unadmitted-authority",
        vec![Mutation::Inject(task_network_support::inject_for_node(
            node,
            vec![],
        ))],
    );
    let request = task_network_support::apply_memory_command(
        &store,
        "command-unadmitted-authority",
        Command::ApplyMutationSet(set),
    );
    assert!(matches!(store.submit(request), Response::Rejected(_)));

    let report = block_on(actor.tick(&mut store, tick_request(1, 1))).unwrap();
    assert_eq!(report.items_attempted, 0);
    assert!(store.state().claims.is_empty());
    assert!(fixture.claim_invocations().is_empty());
}

#[test]
fn forged_admission_attribution_is_rejected_before_graph_commit() {
    let policy = authority_policy();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let admission = admitted_record(&mut store, &policy, "generation-v1");
    let mut node = admitted_node(&admission);
    node.lineage.admission.as_mut().unwrap().agent_id = "forged-agent".to_string();
    let set = Set::new(
        "network-docs",
        "task-alpha",
        format!("lower::{}", admission.admission_id),
        vec![Mutation::Inject(task_network_support::inject_for_node(
            node,
            vec![],
        ))],
    );
    let request = task_network_support::apply_memory_command(
        &store,
        "command-forged-admission-attribution",
        Command::ApplyMutationSet(set),
    );

    let response = store.submit(request);
    assert!(matches!(response, Response::Rejected(_)));
    assert!(store.state().tasks.is_empty());
}

#[test]
fn admitted_executable_substitution_is_rejected_before_graph_commit() {
    let policy = authority_policy();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let admission = admitted_record(&mut store, &policy, "generation-v1");
    let mut node = admitted_node(&admission);
    node.compiled_task.task_id = "forged-executable".to_string();
    let set = Set::new(
        "network-docs",
        "task-alpha",
        format!("lower::{}", admission.admission_id),
        vec![Mutation::Inject(task_network_support::inject_for_node(
            node,
            vec![],
        ))],
    );
    let request = task_network_support::apply_memory_command(
        &store,
        "command-forged-executable",
        Command::ApplyMutationSet(set),
    );

    assert!(matches!(store.submit(request), Response::Rejected(_)));
    assert!(store.state().tasks.is_empty());
}

#[test]
fn forged_durable_attribution_is_rejected_before_dispatch_claim() {
    let fixture = DispatchFixture::new();
    let policy = authority_policy();
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(policy.clone())
        .with_admission_generation("generation-v1");
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    admitted_record(&mut store, &policy, "generation-v1");
    let task_instance_id = realize_admitted(&mut store);
    let mut state = store.state().clone();
    state
        .tasks
        .get_mut(&task_instance_id)
        .unwrap()
        .lineage
        .admission
        .as_mut()
        .unwrap()
        .agent_id = "forged-agent".to_string();
    let mut network = ReadOnlyNetwork { state };

    let report = block_on(actor.tick(&mut network, tick_request(1, 1))).unwrap();

    assert!(report.fatal_errors.iter().any(|issue| {
        issue.code == "effective_authority_denied"
            && issue.message.contains("differs from its durable record")
    }));
    assert!(fixture.claim_invocations().is_empty());
}

#[test]
fn forged_durable_executable_is_rejected_before_dispatch_claim() {
    let fixture = DispatchFixture::new();
    let policy = authority_policy();
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(policy.clone())
        .with_admission_generation("generation-v1");
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    admitted_record(&mut store, &policy, "generation-v1");
    let task_instance_id = realize_admitted(&mut store);
    let mut state = store.state().clone();
    state
        .tasks
        .get_mut(&task_instance_id)
        .unwrap()
        .compiled_task
        .task_id = "forged-executable".to_string();
    let mut network = ReadOnlyNetwork { state };

    let report = block_on(actor.tick(&mut network, tick_request(1, 1))).unwrap();

    assert!(report.fatal_errors.iter().any(|issue| {
        issue.code == "effective_authority_denied"
            && issue.message.contains("outside canonical lowering")
    }));
    assert!(fixture.claim_invocations().is_empty());
}

#[test]
fn forged_durable_executable_is_rejected_before_resumed_invocation() {
    let fixture = DispatchFixture::new();
    let policy = authority_policy();
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(policy.clone())
        .with_admission_generation("generation-v1");
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    admitted_record(&mut store, &policy, "generation-v1");
    let task_instance_id = realize_admitted(&mut store);
    let claim = DispatchRequest {
        claim_id: "claim-resumed-forgery".to_string(),
        task_instance_id: task_instance_id.clone(),
        worker_id: "worker-a".to_string(),
        idempotency_key: "claim-resumed-forgery".to_string(),
    };
    let claim_command = task_network_support::apply_memory_command(
        &store,
        "command-claim-before-forgery",
        Command::ClaimReadyTask(claim),
    );
    assert!(matches!(
        store.submit(claim_command),
        Response::Accepted { .. }
    ));
    let mut state = store.state().clone();
    state
        .tasks
        .get_mut(&task_instance_id)
        .unwrap()
        .compiled_task
        .task_id = "forged-resumed-executable".to_string();
    let mut network = ReadOnlyNetwork { state };

    let report = block_on(actor.tick(&mut network, tick_request(2, 1))).unwrap();

    assert!(report.fatal_errors.iter().any(|issue| {
        issue.code == "effective_authority_denied"
            && issue.message.contains("outside canonical lowering")
    }));
    assert!(fixture.claim_invocations().is_empty());
}

#[test]
fn admitted_task_with_stale_generation_is_denied_before_claim() {
    let fixture = DispatchFixture::new();
    let policy = authority_policy();
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(policy.clone())
        .with_admission_generation("generation-v2");
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    admitted_record(&mut store, &policy, "generation-v1");
    realize_admitted(&mut store);

    let report = block_on(actor.tick(&mut store, tick_request(1, 1))).unwrap();

    assert!(report.fatal_errors.iter().any(|issue| {
        issue.code == "effective_authority_denied"
            && issue.message.contains("activation generation")
    }));
    assert!(store.state().claims.is_empty());
    assert!(fixture.claim_invocations().is_empty());
}

#[test]
fn existing_actor_observes_activation_generation_change_before_claim() {
    let fixture = DispatchFixture::new();
    let policy = authority_policy();
    let generation = Arc::new(Mutex::new(Some("generation-v1".to_string())));
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(policy.clone())
        .with_admission_generation_observer(Arc::new(MutableGenerationObserver {
            generation: Arc::clone(&generation),
        }));
    *generation.lock().unwrap() = Some("generation-v2".to_string());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    admitted_record(&mut store, &policy, "generation-v1");
    realize_admitted(&mut store);

    let report = block_on(actor.tick(&mut store, tick_request(1, 1))).unwrap();

    assert!(report.fatal_errors.iter().any(|issue| {
        issue.code == "effective_authority_denied"
            && issue.message.contains("activation generation")
    }));
    assert!(store.state().claims.is_empty());
}

#[test]
fn admitted_task_requires_matching_live_policy_and_generation() {
    let fixture = DispatchFixture::new();
    let policy = authority_policy();
    let actor = fixture
        .actor(open_db())
        .with_authority_policy(policy.clone())
        .with_admission_generation("generation-v1");
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let admission = admitted_record(&mut store, &policy, "generation-v1");
    let expected_admission = Some(TaskAdmissionAttribution::from_record(&admission));
    let task_instance_id = realize_admitted(&mut store);

    let report = block_on(actor.tick(&mut store, tick_request(1, 1))).unwrap();

    assert_eq!(report.items_committed, 1);
    assert_eq!(fixture.claim_invocations(), vec![task_instance_id]);
    let outcome = store.state().outcomes.values().next().unwrap();
    assert_eq!(outcome.admission, expected_admission);
}
