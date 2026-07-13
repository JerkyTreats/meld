#[path = "support/task_network.rs"]
mod task_network_support;

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::time::{Duration, Instant};

use meld_events::DomainObjectRef;
use meld_execution::goals::{AddGoalCommand, GoalCommandMetadata, PersistentGoalSetStore};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::mutation::{Mutation, Rejection, Set};
use meld_execution::task_network::store::SledTaskNetworkStore;
use meld_execution::task_network::AttributedOutcome;
use meld_lang::{
    Condition, Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term,
};

const CHILD_ROLE: &str = "MELD_W1B5_EXECUTION_CHILD_ROLE";
const CHILD_DB_PATH: &str = "MELD_W1B5_EXECUTION_DB_PATH";
const CHILD_MARKER_PATH: &str = "MELD_W1B5_EXECUTION_MARKER_PATH";

fn write_marker(path: &Path, value: &str) {
    let mut marker = File::create(path).unwrap();
    marker.write_all(value.as_bytes()).unwrap();
    marker.sync_all().unwrap();
}

fn run_goal_boundary_child() -> bool {
    let Ok(role) = std::env::var(CHILD_ROLE) else {
        return false;
    };
    let db_path = std::env::var_os(CHILD_DB_PATH).unwrap();
    let marker_path = std::env::var_os(CHILD_MARKER_PATH).unwrap();
    let store = PersistentGoalSetStore::new(sled::open(db_path).unwrap()).unwrap();
    match role.as_str() {
        "before_commit" => write_marker(Path::new(&marker_path), "before_commit"),
        "after_commit" => {
            store.commit_add_goal(goal_command()).unwrap();
            write_marker(Path::new(&marker_path), "after_commit");
        }
        other => panic!("unknown execution child role {other}"),
    }
    std::process::abort();
}

fn run_aborting_child(role: &str, db_path: &Path, marker_path: &Path) {
    let mut child = ProcessCommand::new(std::env::current_exe().unwrap())
        .arg("abrupt_goal_commit_boundary_survives_process_termination")
        .arg("--exact")
        .current_dir(marker_path.parent().unwrap())
        .env(CHILD_ROLE, role)
        .env(CHILD_DB_PATH, db_path)
        .env(CHILD_MARKER_PATH, marker_path)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("execution boundary child exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(!status.success());
    assert_eq!(std::fs::read_to_string(marker_path).unwrap(), role);
}

fn goal_command() -> AddGoalCommand {
    AddGoalCommand {
        metadata: GoalCommandMetadata {
            command_id: "command-goal-a".to_string(),
            source_identity: Some("agent-delivery-a".to_string()),
            seq: 1,
        },
        goal: Goal {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: Proposition::Holds {
                subject: Term::Object(
                    DomainObjectRef::new("workspace_fs", "node", "readme").unwrap(),
                ),
                dimension: Term::Dimension("docs_freshness".to_string()),
                condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
            },
            priority: GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: GoalSource::UserDirected {
                directive: "refresh docs".to_string(),
            },
            lifecycle: GoalLifecycle::Active,
        },
    }
}

#[test]
fn divergent_goal_replay_never_displaces_the_canonical_commit_across_reopens() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("goals");
    let db = sled::open(&path).unwrap();
    let canonical = goal_command();
    let expected = {
        let store = PersistentGoalSetStore::new(db.clone()).unwrap();
        store.commit_add_goal(canonical.clone()).unwrap()
    };
    let mut divergent = canonical.clone();
    divergent.goal.priority.urgency = 99;

    for _ in 0..4 {
        let store = PersistentGoalSetStore::new(db.clone()).unwrap();
        let error = store.commit_add_goal(divergent.clone()).unwrap_err();
        assert!(error.to_string().contains("replayed with divergent intent"));
        assert_eq!(
            store
                .get_goal("goal-a")
                .unwrap()
                .unwrap()
                .goal
                .priority
                .urgency,
            1
        );
        assert_eq!(
            store.command_receipt("command-goal-a").unwrap(),
            Some(expected.1.clone())
        );
        assert_eq!(store.commit_add_goal(canonical.clone()).unwrap(), expected);
    }
}

#[test]
fn abrupt_goal_commit_boundary_survives_process_termination() {
    if run_goal_boundary_child() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let before_db = temp.path().join("before-goals");
    let before_marker = temp.path().join("before.marker");
    run_aborting_child("before_commit", &before_db, &before_marker);
    let before_store = PersistentGoalSetStore::new(sled::open(&before_db).unwrap()).unwrap();
    assert!(before_store.get_goal("goal-a").unwrap().is_none());
    assert!(before_store
        .command_receipt("command-goal-a")
        .unwrap()
        .is_none());
    drop(before_store);

    let after_db = temp.path().join("after-goals");
    let after_marker = temp.path().join("after.marker");
    run_aborting_child("after_commit", &after_db, &after_marker);
    let after_store = PersistentGoalSetStore::new(sled::open(&after_db).unwrap()).unwrap();
    let canonical = goal_command();
    let receipt = after_store
        .command_receipt("command-goal-a")
        .unwrap()
        .unwrap();
    assert_eq!(after_store.commit_add_goal(canonical).unwrap().1, receipt);
    assert_eq!(
        after_store
            .get_goal("goal-a")
            .unwrap()
            .unwrap()
            .goal
            .priority
            .urgency,
        1
    );
}

#[test]
fn attributed_outcome_replays_canonically_without_duplicate_publication_after_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("task-network");
    let db = sled::open(&path).unwrap();
    let request;
    let accepted_revision;
    let accepted_hash;
    let publication_id;
    {
        let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
        let mut node = task_network_support::single_task_node("task-alpha");
        node.lineage.subject = Some(
            DomainObjectRef::new("workspace_fs", "node", "readme")
                .expect("subject identity must be valid"),
        );
        let set = Set::new(
            "network-docs",
            "composition-fixture",
            "inject-attributed",
            vec![Mutation::Inject(task_network_support::inject_for_node(
                node.clone(),
                vec![],
            ))],
            vec![],
        );
        let inject = task_network_support::apply_sled_command(
            &store,
            "command-inject",
            Command::ApplyMutationSet(set),
        );
        assert!(matches!(
            store.submit(inject).unwrap(),
            Response::Accepted { .. }
        ));
        let task_instance_id =
            task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-a");
        let claim = store.state().claims.get("claim-a").unwrap().clone();
        let outcome =
            task_network_support::outcome_for_claim("outcome-a", &task_instance_id, &claim);
        let attributed = AttributedOutcome::for_task(outcome, &node).unwrap();
        request = task_network_support::apply_sled_command(
            &store,
            "command-outcome",
            Command::RecordAttributedTaskOutcome(attributed),
        );
        let response = store.submit(request.clone()).unwrap();
        let Response::Accepted {
            revision,
            state_hash,
        } = response
        else {
            panic!("attributed outcome must be accepted");
        };
        accepted_revision = revision;
        accepted_hash = state_hash;
        publication_id = store.state().publications.keys().next().unwrap().clone();
        assert_eq!(store.journal().len(), 3);
    }

    for _ in 0..4 {
        let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
        let canonical_state = store.state().clone();
        let canonical_journal = store.journal().to_vec();
        let mut altered_outcome = request.clone();
        let Command::RecordAttributedTaskOutcome(attributed) = &mut altered_outcome.command else {
            unreachable!();
        };
        attributed.outcome.artifact_records[0].content = serde_json::json!({ "altered": true });
        assert!(matches!(
            store.submit(altered_outcome).unwrap(),
            Response::Rejected(Rejection::DuplicateCommand(command_id))
                if command_id == "command-outcome"
        ));
        assert_eq!(store.state(), &canonical_state);
        assert_eq!(store.journal(), canonical_journal.as_slice());

        let mut altered_lineage = request.clone();
        let Command::RecordAttributedTaskOutcome(attributed) = &mut altered_lineage.command else {
            unreachable!();
        };
        attributed.semantic_lineage.subject =
            DomainObjectRef::new("workspace_fs", "node", "other").unwrap();
        assert!(matches!(
            store.submit(altered_lineage).unwrap(),
            Response::Rejected(Rejection::DuplicateCommand(command_id))
                if command_id == "command-outcome"
        ));
        assert_eq!(store.state(), &canonical_state);
        assert_eq!(store.journal(), canonical_journal.as_slice());

        assert_eq!(
            store.submit(request.clone()).unwrap(),
            Response::Duplicate {
                revision: accepted_revision,
                state_hash: accepted_hash.clone(),
            }
        );
        assert_eq!(store.journal(), canonical_journal.as_slice());
        assert_eq!(store.state().publications.len(), 1);
        assert_eq!(store.state(), &canonical_state);
        let lineage = store.state().publications[&publication_id]
            .semantic_lineage
            .as_ref()
            .unwrap();
        assert_eq!(lineage.goal.object_id, "goal-fixture");
        assert_eq!(lineage.method.object_id, "method-fixture");
        assert_eq!(lineage.projection_frame.object_id, "frame-fixture");
        assert_eq!(lineage.subject.object_id, "readme");
    }
}
