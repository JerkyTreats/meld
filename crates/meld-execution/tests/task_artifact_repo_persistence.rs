#[path = "support/task_network.rs"]
mod task_network_support;

use futures::executor::block_on;
use meld_events::EventEnvelope;
use meld_execution::capability::{
    BoundCapabilityInstance, CapabilityCatalog, CapabilityInvocationPayload,
    CapabilityInvocationResult,
};
use meld_execution::error::ExecutionInvariantError;
use meld_execution::execution::{EventPublicationPort, ExecutionEventContext};
use meld_execution::task::{
    execute_task_to_completion, ArtifactLinkRelation, ArtifactProducerRef, ArtifactRecord,
    CompiledTaskDelta, CompiledTaskRecord, InitArtifactValue, TaskArtifactRepo,
    TaskArtifactRepoError, TaskArtifactRepoFactory, TaskExecutor, TaskExpansionRequest,
    TaskInitializationPayload, TaskRunContext,
};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::{
    build_executor_for_claim_with_artifact_repo, succeeded_outcome_from_executor,
    Request as DispatchRequest,
};
use meld_execution::task_network::initialization::materialize_task_initialization;
use meld_execution::task_network::mutation::{Mutation, Set};
use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::store::SledTaskNetworkStore;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;

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

fn open_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn artifact_key(repo_id: &str, artifact_id: &str) -> Vec<u8> {
    let repo_bytes = repo_id.as_bytes();
    let mut key = Vec::new();
    key.extend_from_slice(&(repo_bytes.len() as u32).to_be_bytes());
    key.extend_from_slice(repo_bytes);
    key.extend_from_slice(artifact_id.as_bytes());
    key
}

fn artifact(
    artifact_id: &str,
    capability_instance_id: &str,
    output_slot_id: &str,
) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: artifact_id.to_string(),
        artifact_type_id: "docs_patch".to_string(),
        schema_version: 1,
        content: json!({ "artifact_id": artifact_id }),
        producer: ArtifactProducerRef {
            task_id: "task-docs".to_string(),
            capability_instance_id: capability_instance_id.to_string(),
            invocation_id: Some("invoke-1".to_string()),
            output_slot_id: Some(output_slot_id.to_string()),
        },
    }
}

fn simple_compiled_task() -> CompiledTaskRecord {
    CompiledTaskRecord {
        task_id: "task-docs".to_string(),
        task_version: 1,
        init_slots: vec![],
        capability_instances: vec![BoundCapabilityInstance {
            capability_instance_id: "write".to_string(),
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
            scope_ref: "node-readme".to_string(),
            scope_kind: "filesystem".to_string(),
            binding_values: vec![],
            input_wiring: vec![],
        }],
        dependency_edges: vec![],
    }
}

fn init_payload() -> TaskInitializationPayload {
    TaskInitializationPayload {
        task_id: "task-docs".to_string(),
        compiled_task_ref: "task-docs@1".to_string(),
        init_artifacts: vec![],
        task_run_context: TaskRunContext {
            task_run_id: "run-docs".to_string(),
            session_id: Some("session-docs".to_string()),
            trigger: "test".to_string(),
        },
    }
}

fn init_payload_with_seed() -> TaskInitializationPayload {
    TaskInitializationPayload {
        task_id: "task-docs".to_string(),
        compiled_task_ref: "task-docs@1".to_string(),
        init_artifacts: vec![InitArtifactValue {
            init_slot_id: "target".to_string(),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
            content: json!({ "path": "docs" }),
        }],
        task_run_context: TaskRunContext {
            task_run_id: "run-docs".to_string(),
            session_id: Some("session-docs".to_string()),
            trigger: "test".to_string(),
        },
    }
}

fn compiled_task_with_seed() -> CompiledTaskRecord {
    let mut task = simple_compiled_task();
    task.init_slots = vec![meld_execution::task::TaskInitSlotSpec {
        init_slot_id: "target".to_string(),
        artifact_type_id: "target_selector".to_string(),
        schema_version: 1,
        required: true,
    }];
    task
}

fn invoke_success<'a>(
    _api: &'a NoopApi,
    _state: &'a (),
    instance: &'a BoundCapabilityInstance,
    payload: &'a CapabilityInvocationPayload,
    _event_context: Option<&'a ExecutionEventContext>,
) -> Pin<
    Box<
        dyn Future<Output = Result<CapabilityInvocationResult, ExecutionInvariantError>>
            + Send
            + 'a,
    >,
> {
    let invocation_id = payload.invocation_id.clone();
    let capability_instance_id = payload.capability_instance_id.clone();
    let task_id = instance.capability_type_id.clone();
    Box::pin(async move {
        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![ArtifactRecord {
                artifact_id: format!("{invocation_id}::docs_patch"),
                artifact_type_id: "docs_patch".to_string(),
                schema_version: 1,
                content: json!({ "task_id": task_id.clone() }),
                producer: ArtifactProducerRef {
                    task_id,
                    capability_instance_id,
                    invocation_id: Some(invocation_id),
                    output_slot_id: Some("patch".to_string()),
                },
            }],
        })
    })
}

fn compile_no_expansion(
    _api: &NoopApi,
    _compiled_task: &CompiledTaskRecord,
    _request: &TaskExpansionRequest,
    _catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ExecutionInvariantError> {
    Ok(CompiledTaskDelta::default())
}

#[test]
fn task_artifact_repo_artifact_survives_reopen() {
    let db = open_db();
    {
        let mut repo = TaskArtifactRepo::open_sled(db.clone(), "repo-docs").unwrap();
        repo.append_artifact(artifact("artifact-a", "write", "patch"))
            .unwrap();
    }

    let repo = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();
    let artifact = repo.get_artifact("artifact-a").unwrap();

    assert_eq!(artifact.artifact_type_id, "docs_patch");
    assert_eq!(artifact.schema_version, 1);
    assert_eq!(artifact.content, json!({ "artifact_id": "artifact-a" }));
    assert_eq!(artifact.producer.capability_instance_id, "write");
    assert_eq!(artifact.producer.output_slot_id.as_deref(), Some("patch"));
}

#[test]
fn task_artifact_repo_scopes_records_by_repo_id_in_shared_store() {
    let db = open_db();
    {
        let mut left = TaskArtifactRepo::open_sled(db.clone(), "repo-left").unwrap();
        left.append_artifact(artifact("artifact-shared", "write-left", "patch"))
            .unwrap();
        let mut right = TaskArtifactRepo::open_sled(db.clone(), "repo-right").unwrap();
        right
            .append_artifact(artifact("artifact-shared", "write-right", "patch"))
            .unwrap();
    }

    let left = TaskArtifactRepo::open_sled(db.clone(), "repo-left").unwrap();
    let right = TaskArtifactRepo::open_sled(db, "repo-right").unwrap();

    assert_eq!(left.record().artifacts.len(), 1);
    assert_eq!(right.record().artifacts.len(), 1);
    assert_eq!(
        left.get_artifact("artifact-shared")
            .unwrap()
            .producer
            .capability_instance_id,
        "write-left"
    );
    assert_eq!(
        right
            .get_artifact("artifact-shared")
            .unwrap()
            .producer
            .capability_instance_id,
        "write-right"
    );
}

#[test]
fn task_artifact_repo_links_survive_reopen_in_append_order() {
    let db = open_db();
    {
        let mut repo = TaskArtifactRepo::open_sled(db.clone(), "repo-docs").unwrap();
        repo.append_artifact(artifact("artifact-a", "write", "patch"))
            .unwrap();
        repo.append_artifact(artifact("artifact-b", "write", "patch"))
            .unwrap();
        repo.append_artifact(artifact("artifact-c", "write", "patch"))
            .unwrap();
        repo.mark_superseded("artifact-a", "artifact-b", "first")
            .unwrap();
        repo.mark_superseded("artifact-b", "artifact-c", "second")
            .unwrap();
    }

    let repo = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();

    assert_eq!(repo.record().artifact_links.len(), 2);
    assert_eq!(repo.record().artifact_links[0].detail, "first");
    assert_eq!(repo.record().artifact_links[1].detail, "second");
    assert_eq!(
        repo.record().artifact_links[0].relation,
        ArtifactLinkRelation::Supersedes
    );
}

#[test]
fn task_artifact_repo_duplicate_artifact_id_is_rejected_without_mutating_snapshot() {
    let db = open_db();
    let mut repo = TaskArtifactRepo::open_sled(db.clone(), "repo-docs").unwrap();
    repo.append_artifact(artifact("artifact-a", "write", "patch"))
        .unwrap();

    let error = repo
        .append_artifact(artifact("artifact-a", "write", "patch"))
        .unwrap_err();

    assert!(error.to_string().contains("already contains artifact"));
    assert_eq!(repo.record().artifacts.len(), 1);
    let reopened = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();
    assert_eq!(reopened.record().artifacts.len(), 1);
}

#[test]
fn task_artifact_repo_missing_link_endpoint_is_rejected_without_durable_write() {
    let db = open_db();
    let mut repo = TaskArtifactRepo::open_sled(db.clone(), "repo-docs").unwrap();
    repo.append_artifact(artifact("artifact-a", "write", "patch"))
        .unwrap();

    let error = repo
        .mark_superseded("artifact-a", "artifact-missing", "missing")
        .unwrap_err();

    assert!(error.to_string().contains("artifact-missing"));
    assert!(repo.record().artifact_links.is_empty());
    let reopened = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();
    assert!(reopened.record().artifact_links.is_empty());
}

#[test]
fn task_artifact_repo_corrupt_artifact_json_returns_decode_error() {
    let db = open_db();
    db.open_tree("task_artifact_records")
        .unwrap()
        .insert(
            artifact_key("repo-docs", "artifact-bad"),
            b"not json".as_slice(),
        )
        .unwrap();

    let error = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap_err();

    assert!(matches!(error, TaskArtifactRepoError::Decode(_)));
}

#[test]
fn task_artifact_repo_mismatched_stored_repo_id_returns_decode_error() {
    let db = open_db();
    let stored = json!({
        "record_schema_version": 1,
        "repo_id": "repo-other",
        "artifact": artifact("artifact-a", "write", "patch"),
    });
    db.open_tree("task_artifact_records")
        .unwrap()
        .insert(
            artifact_key("repo-docs", "artifact-a"),
            serde_json::to_vec(&stored).unwrap(),
        )
        .unwrap();

    let error = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap_err();

    assert!(matches!(error, TaskArtifactRepoError::Decode(_)));
}

#[test]
fn task_artifact_repo_unsupported_schema_version_returns_decode_error() {
    let db = open_db();
    let stored = json!({
        "record_schema_version": 2,
        "repo_id": "repo-docs",
        "artifact": artifact("artifact-a", "write", "patch"),
    });
    db.open_tree("task_artifact_records")
        .unwrap()
        .insert(
            artifact_key("repo-docs", "artifact-a"),
            serde_json::to_vec(&stored).unwrap(),
        )
        .unwrap();

    let error = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap_err();

    assert!(matches!(error, TaskArtifactRepoError::Decode(_)));
    assert!(error
        .to_string()
        .contains("unsupported task artifact record schema version"));
}

#[test]
fn task_artifact_repo_store_can_open_on_caller_supplied_path_outside_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    assert!(!storage.path().starts_with(workspace.path()));

    let db = sled::open(storage.path().join("task-artifacts")).unwrap();
    let mut repo = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();
    repo.append_artifact(artifact("artifact-a", "write", "patch"))
        .unwrap();
    repo.flush().unwrap();
}

#[test]
fn task_artifact_repo_flush_writes_pending_bytes_to_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("task-artifacts");
    {
        let db = sled::Config::new()
            .path(&path)
            .flush_every_ms(None)
            .open()
            .unwrap();
        let mut repo = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();
        repo.append_artifact(artifact("artifact-a", "write", "patch"))
            .unwrap();
        repo.flush().unwrap();
    }

    let db = sled::open(path).unwrap();
    let reopened = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();
    assert!(reopened.get_artifact("artifact-a").is_some());
}

#[test]
fn task_artifact_repo_executor_seeds_init_artifacts_once_across_reopen() {
    let db = open_db();
    {
        let repo = TaskArtifactRepo::open_sled(db.clone(), "repo-docs").unwrap();
        let executor = TaskExecutor::new_with_artifact_repo(
            compiled_task_with_seed(),
            init_payload_with_seed(),
            repo,
        )
        .unwrap();
        assert_eq!(executor.artifact_repo().record().artifacts.len(), 1);
    }

    let repo = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();
    let executor = TaskExecutor::new_with_artifact_repo(
        compiled_task_with_seed(),
        init_payload_with_seed(),
        repo,
    )
    .unwrap();

    assert_eq!(executor.artifact_repo().record().artifacts.len(), 1);
}

#[test]
fn task_artifact_repo_executor_emitted_artifacts_survive_reopen() {
    let db = open_db();
    {
        let repo = TaskArtifactRepo::open_sled(db.clone(), "repo-docs").unwrap();
        let mut executor =
            TaskExecutor::new_with_artifact_repo(simple_compiled_task(), init_payload(), repo)
                .unwrap();
        let payloads = executor
            .release_ready_invocations(Default::default())
            .unwrap();
        executor
            .record_success(
                &payloads[0].invocation_id,
                vec![artifact("artifact-a", "write", "patch")],
            )
            .unwrap();
        executor.artifact_repo().flush().unwrap();
    }

    let repo = TaskArtifactRepo::open_sled(db, "repo-docs").unwrap();

    assert!(repo.get_artifact("artifact-a").is_some());
}

#[test]
fn task_artifact_repo_task_network_bridge_accepts_executor_with_durable_repo() {
    let task_db = open_db();
    let artifact_db = open_db();
    let mut store = SledTaskNetworkStore::open(task_db, "network-docs").unwrap();
    let mut node = task_network_support::single_task_node("task-alpha");
    node.compiled_task.capability_instances = simple_compiled_task().capability_instances;
    let set = Set::new(
        "network-docs",
        "composition-fixture",
        "durable-artifact-task",
        vec![Mutation::Inject(task_network_support::inject_for_node(
            node.clone(),
            vec![],
        ))],
    );
    let commit = task_network_support::apply_sled_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(set),
    );
    assert!(matches!(
        store.submit(commit).unwrap(),
        Response::Accepted { .. }
    ));
    let ready = compute_ready_set(store.state());
    let task_instance_id = ready.task_instance_ids[0].clone();
    let claim_request = DispatchRequest {
        claim_id: "claim-alpha".to_string(),
        task_instance_id: task_instance_id.clone(),
        worker_id: "worker-a".to_string(),
        idempotency_key: "claim-alpha-once".to_string(),
    };
    let claim_command = task_network_support::apply_sled_command(
        &store,
        "command-claim",
        Command::ClaimReadyTask(claim_request),
    );
    assert!(matches!(
        store.submit(claim_command).unwrap(),
        Response::Accepted { .. }
    ));
    let claim = store.state().claims["claim-alpha"].clone();
    let materialized = materialize_task_initialization(store.state(), &task_instance_id).unwrap();
    let repo = TaskArtifactRepo::open_sled(
        artifact_db.clone(),
        "task_artifacts::network-docs::task-alpha::claim-alpha",
    )
    .unwrap();
    let mut executor =
        build_executor_for_claim_with_artifact_repo(&node, &claim, materialized.payload, repo)
            .unwrap();

    block_on(execute_task_to_completion(
        &NoopApi,
        &mut executor,
        &task_network_support::catalog(),
        &(),
        invoke_success,
        compile_no_expansion,
        None,
        None,
    ))
    .unwrap();
    let outcome = succeeded_outcome_from_executor("outcome-alpha", &claim, &executor);
    let artifact_id = outcome.artifact_records[0].artifact_id.clone();
    let outcome_command = task_network_support::apply_sled_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );

    assert!(matches!(
        store.submit(outcome_command).unwrap(),
        Response::Accepted { .. }
    ));
    assert_eq!(
        store.state().outcomes["outcome-alpha"]
            .artifact_records
            .len(),
        1
    );

    let repo = TaskArtifactRepo::open_sled(
        artifact_db,
        "task_artifacts::network-docs::task-alpha::claim-alpha",
    )
    .unwrap();
    assert!(repo.get_artifact(&artifact_id).is_some());
}

#[test]
fn task_artifact_factory_opens_flushes_and_reopens_repo() {
    let db = open_db();
    let factory = TaskArtifactRepoFactory::new(db.clone());
    {
        let mut repo = factory.open_repo("repo-docs").unwrap();
        repo.append_artifact(artifact("artifact-1", "capability-a", "out"))
            .unwrap();
        factory.flush().unwrap();
    }

    let reopened = TaskArtifactRepoFactory::new(db)
        .open_repo("repo-docs")
        .unwrap();

    assert!(reopened.get_artifact("artifact-1").is_some());
}

#[test]
fn task_artifact_factory_isolates_repo_ids_in_one_database() {
    let db = open_db();
    let factory = TaskArtifactRepoFactory::new(db.clone());
    {
        let mut left = factory.open_repo("repo-left").unwrap();
        left.append_artifact(artifact("artifact-left", "capability-a", "out"))
            .unwrap();
        let mut right = factory.open_repo("repo-right").unwrap();
        right
            .append_artifact(artifact("artifact-right", "capability-a", "out"))
            .unwrap();
        factory.flush().unwrap();
    }

    let factory = TaskArtifactRepoFactory::new(db);
    let left = factory.open_repo("repo-left").unwrap();
    let right = factory.open_repo("repo-right").unwrap();

    assert!(left.get_artifact("artifact-left").is_some());
    assert!(left.get_artifact("artifact-right").is_none());
    assert!(right.get_artifact("artifact-right").is_some());
    assert!(right.get_artifact("artifact-left").is_none());
}

#[test]
fn task_artifact_factory_flush_writes_pending_bytes_to_disk() {
    let temp = tempfile::tempdir().unwrap();
    let db = sled::Config::new()
        .path(temp.path())
        .flush_every_ms(None)
        .open()
        .unwrap();
    let factory = TaskArtifactRepoFactory::new(db.clone());
    {
        let mut repo = factory.open_repo("repo-docs").unwrap();
        repo.append_artifact(artifact("artifact-flush", "capability-a", "out"))
            .unwrap();
        factory.flush().unwrap();
    }
    drop(factory);
    drop(db);

    let db = sled::open(temp.path()).unwrap();
    let reopened = TaskArtifactRepoFactory::new(db)
        .open_repo("repo-docs")
        .unwrap();

    assert!(reopened.get_artifact("artifact-flush").is_some());
}
