#[path = "support/task_network.rs"]
mod task_network_support;

use std::path::Path;
use std::time::{Duration, Instant};

use meld_events::DomainObjectRef;
use meld_execution::goals::{AddGoalCommand, GoalCommandMetadata, PersistentGoalSetStore};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::mutation::{Mutation, Set};
use meld_execution::task_network::store::SledTaskNetworkStore;
use meld_execution::task_network::AttributedOutcome;
use meld_lang::{
    Condition, Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term,
};

fn reopen_sled(path: &Path) -> sled::Db {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("cannot reopen execution database: {error}"),
        }
    }
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
    let canonical = goal_command();
    let expected = {
        let store = PersistentGoalSetStore::new(reopen_sled(&path)).unwrap();
        store.commit_add_goal(canonical.clone()).unwrap()
    };
    let mut divergent = canonical.clone();
    divergent.goal.priority.urgency = 99;

    for _ in 0..4 {
        let store = PersistentGoalSetStore::new(reopen_sled(&path)).unwrap();
        assert_eq!(store.commit_add_goal(canonical.clone()).unwrap(), expected);
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
    }
}

#[test]
fn attributed_outcome_replays_canonically_without_duplicate_publication_after_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("task-network");
    let request;
    let accepted_revision;
    let accepted_hash;
    let publication_id;
    {
        let mut store = SledTaskNetworkStore::open(reopen_sled(&path), "network-docs").unwrap();
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
        let mut store = SledTaskNetworkStore::open(reopen_sled(&path), "network-docs").unwrap();
        assert_eq!(
            store.submit(request.clone()).unwrap(),
            Response::Duplicate {
                revision: accepted_revision,
                state_hash: accepted_hash.clone(),
            }
        );
        assert_eq!(store.journal().len(), 3);
        assert_eq!(store.state().publications.len(), 1);
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
