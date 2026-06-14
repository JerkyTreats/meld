#[path = "../../crates/meld-execution/tests/support/task_network.rs"]
mod task_network_support;

use meld::context::frame::{Basis, Frame};
use meld::prompt_context::PromptContextArtifactKind;
use meld::runtime::storage::{OpenProductStores, ProductStorageLayout, ProductStorageRoot};
use meld_events::{DomainObjectRef, EventEnvelope};
use meld_execution::goals::{AddGoalCommand, GoalCommandMetadata};
use meld_execution::task::{ArtifactProducerRef, ArtifactRecord};
use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};
use meld_world_model::{AgentRecord, AgentStatus, BranchScope, PerspectiveKey};
use serde_json::json;

#[test]
fn product_storage_layout_derives_required_paths() {
    let root = tempfile::tempdir().unwrap().path().join("runtime");
    let layout = ProductStorageLayout::from_root(root.clone());

    assert_eq!(layout.root, root);
    assert_eq!(layout.ledger_db, layout.root.join("ledger.sled"));
    assert_eq!(layout.workspace_db, layout.root.join("workspace.sled"));
    assert_eq!(layout.world_model_db, layout.root.join("world_model.sled"));
    assert_eq!(
        layout.execution_goals_db,
        layout.root.join("execution").join("goals.sled")
    );
    assert_eq!(
        layout.task_artifacts_db,
        layout.root.join("execution").join("task_artifacts.sled")
    );
    assert_eq!(
        layout.task_networks_root,
        layout.root.join("execution").join("task_networks")
    );
    assert_eq!(
        layout.frame_blob_root,
        layout.root.join("context").join("frames")
    );
    assert_eq!(
        layout.prompt_artifact_root,
        layout.root.join("context").join("prompt_artifacts")
    );
}

#[test]
fn product_storage_root_derives_layout_without_changing_root() {
    let root = tempfile::tempdir().unwrap().path().join("runtime");
    let product_root = ProductStorageRoot::new(root.clone());

    let layout = product_root.layout();

    assert_eq!(layout.root, root);
}

#[test]
fn product_storage_open_creates_dirs_and_opens_stores() {
    let temp = tempfile::tempdir().unwrap();
    let layout = ProductStorageLayout::from_root(temp.path().join("runtime"));

    let stores = OpenProductStores::open(&layout).unwrap();
    stores.flush_boundary().unwrap();

    assert!(layout.root.exists());
    assert!(layout.ledger_db.exists());
    assert!(layout.workspace_db.exists());
    assert!(layout.world_model_db.exists());
    assert!(layout.execution_goals_db.exists());
    assert!(layout.task_artifacts_db.exists());
    assert!(layout.task_networks_root.exists());
    assert!(layout.frame_blob_root.exists());
    assert!(layout.prompt_artifact_root.exists());
}

#[test]
fn product_storage_persists_and_reopens_runtime_stores() {
    let temp = tempfile::tempdir().unwrap();
    let layout = ProductStorageLayout::from_root(temp.path().join("runtime"));
    let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
    let stored_frame_id;
    let stored_prompt_ref;

    {
        let stores = OpenProductStores::open(&layout).unwrap();
        stores
            .event_store
            .append_envelope(EventEnvelope::new_domain(
                "2026-06-14T00:00:00Z".to_string(),
                "session-a",
                "execution",
                "stream-a",
                "execution.test",
                Some("record-a".to_string()),
                json!({ "ok": true }),
            ))
            .unwrap();
        stores.traversal_store.set_last_reduced_seq(42).unwrap();
        stores
            .belief_store
            .put_config_snapshot("config-a", "{\"ok\":true}")
            .unwrap();
        stores
            .agent_store
            .put_agent(&agent_record(&subject))
            .unwrap();
        stores
            .goal_store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: "command-goal".to_string(),
                    source_identity: Some("agent-a:node-a".to_string()),
                    seq: 7,
                },
                goal: goal(&subject),
            })
            .unwrap();
        let mut repo = stores.task_artifacts.open_repo("repo-docs").unwrap();
        repo.append_artifact(artifact("artifact-a")).unwrap();
        let mut network = stores.task_networks.open_network("network-docs").unwrap();
        task_network_support::commit_single_task_sled(&mut network, "task-alpha");
        network.flush().unwrap();
        let frame = Frame::new(
            Basis::Node([1u8; 32]),
            b"hello".to_vec(),
            "analysis".to_string(),
            "agent-a".to_string(),
            std::collections::HashMap::new(),
        )
        .unwrap();
        let frame_id = frame.frame_id;
        stores.frame_storage.store(&frame).unwrap();
        let prompt_ref = stores
            .prompt_artifacts
            .write_utf8(PromptContextArtifactKind::RenderedPrompt, "prompt")
            .unwrap();
        stores.flush_boundary().unwrap();

        assert!(stores.frame_storage.get(&frame_id).unwrap().is_some());
        assert_eq!(
            stores.prompt_artifacts.read_verified(&prompt_ref).unwrap(),
            b"prompt"
        );
        stored_frame_id = frame_id;
        stored_prompt_ref = prompt_ref;
    }

    let reopened = OpenProductStores::open(&layout).unwrap();

    assert_eq!(
        reopened.event_store.read_all_events_after(0).unwrap().len(),
        1
    );
    assert_eq!(reopened.traversal_store.last_reduced_seq().unwrap(), 42);
    assert_eq!(
        reopened
            .belief_store
            .get_config_snapshot("config-a")
            .unwrap()
            .as_deref(),
        Some("{\"ok\":true}")
    );
    assert!(reopened.agent_store.get_agent("agent-a").unwrap().is_some());
    assert!(reopened.goal_store.get_goal("goal-a").unwrap().is_some());
    assert!(reopened
        .task_artifacts
        .open_repo("repo-docs")
        .unwrap()
        .get_artifact("artifact-a")
        .is_some());
    assert!(reopened
        .task_networks
        .open_network("network-docs")
        .unwrap()
        .state()
        .tasks
        .contains_key("task-alpha"));
    assert!(reopened
        .frame_storage
        .get(&stored_frame_id)
        .unwrap()
        .is_some());
    assert_eq!(
        reopened
            .prompt_artifacts
            .read_verified(&stored_prompt_ref)
            .unwrap(),
        b"prompt"
    );
}

#[test]
fn product_storage_rejects_invalid_task_network_storage_key() {
    let temp = tempfile::tempdir().unwrap();
    let layout = ProductStorageLayout::from_root(temp.path().join("runtime"));
    let stores = OpenProductStores::open(&layout).unwrap();

    let error = match stores.task_networks.open_network("../network-docs") {
        Ok(_) => panic!("invalid task network id opened a store"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("must contain only"));
    assert!(!layout
        .root
        .join("execution")
        .join("network-docs.sled")
        .exists());
}

fn agent_record(subject: &DomainObjectRef) -> AgentRecord {
    AgentRecord {
        agent_id: "agent-a".to_string(),
        perspective_key: PerspectiveKey::new("agent", "agent-a").unwrap(),
        subject: subject.clone(),
        branch_scope: BranchScope::main(),
        observation_scope: "workspace".to_string(),
        directive: "watch docs".to_string(),
        seed_provenance: "test".to_string(),
        status: AgentStatus::Operational,
        created_at_seq: 1,
        updated_at_seq: 1,
    }
}

fn goal(subject: &DomainObjectRef) -> Goal {
    Goal {
        goal_id: "goal-a".to_string(),
        agent_id: "agent-a".to_string(),
        target: Proposition::Accessible {
            scope: Term::Object(subject.clone()),
        },
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: None,
        },
        source: GoalSource::UserDirected {
            directive: "refresh docs".to_string(),
        },
        lifecycle: GoalLifecycle::Active,
    }
}

fn artifact(artifact_id: &str) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: artifact_id.to_string(),
        artifact_type_id: "docs_patch".to_string(),
        schema_version: 1,
        content: json!({ "artifact_id": artifact_id }),
        producer: ArtifactProducerRef {
            task_id: "task-alpha".to_string(),
            capability_instance_id: "capability-a".to_string(),
            invocation_id: Some("invoke-a".to_string()),
            output_slot_id: Some("out".to_string()),
        },
    }
}
