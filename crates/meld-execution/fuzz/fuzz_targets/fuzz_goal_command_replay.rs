#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::goals::{
    AddGoalCommand, GoalCommandMetadata, GoalCommandOutcome, GoalSetStore, SatisfyGoalCommand,
};
use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term};

fn goal() -> Goal {
    Goal {
        goal_id: "goal-fuzz".to_string(),
        agent_id: "agent-fuzz".to_string(),
        target: Proposition::Accessible {
            scope: Term::Literal(Literal::Text("scope-fuzz".to_string())),
        },
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: None,
        },
        source: GoalSource::UserDirected {
            directive: "fuzz goal replay".to_string(),
        },
        lifecycle: GoalLifecycle::Active,
    }
}

fn metadata(command_id: &str, seq: u64) -> GoalCommandMetadata {
    GoalCommandMetadata {
        command_id: command_id.to_string(),
        source_identity: None,
        seq,
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }
    let first_seq = u64::from_le_bytes(data[..8].try_into().unwrap());
    let second_seq = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("command-add", 1),
            goal: goal(),
        })
        .unwrap();
    let first = store
        .satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("command-satisfy", 2),
            goal_id: "goal-fuzz".to_string(),
            at_seq: first_seq,
        })
        .unwrap();
    let replay = store.satisfy_goal(SatisfyGoalCommand {
        metadata: metadata("command-satisfy", 2),
        goal_id: "goal-fuzz".to_string(),
        at_seq: second_seq,
    });

    if first_seq == second_seq {
        assert_eq!(replay.unwrap(), first);
    } else {
        assert!(replay
            .unwrap_err()
            .to_string()
            .contains("replayed with divergent intent"));
        assert!(matches!(
            first,
            GoalCommandOutcome::Applied(record)
                if record.goal.lifecycle == GoalLifecycle::Satisfied { at_seq: first_seq }
        ));
    }
});
