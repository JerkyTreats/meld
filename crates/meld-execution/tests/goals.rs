use meld_events::DomainObjectRef;
use meld_execution::goals::{
    AddGoalCommand, ExecutionGoalRecord, GoalAcceptanceLifecycle, GoalAcceptanceRequest,
    GoalCommandCommitReceipt, GoalCommandKind, GoalCommandMetadata, GoalCommandOutcome,
    GoalCommandRequestContract, GoalCommandRequestIdentity, GoalSetApi, GoalSetApiError,
    GoalSetQuery, GoalSetStore, ModifyGoalCommand, PersistentGoalSetStore, RemoveGoalCommand,
    ResumeGoalCommand, SatisfyGoalCommand, SuspendGoalCommand,
};
use meld_lang::{
    Condition, Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term,
};
use proptest::prelude::*;
use std::sync::{Arc, Barrier};
use std::thread;

fn node(id: &str) -> Term {
    Term::Object(DomainObjectRef::new("workspace", "node", id).unwrap())
}

fn goal(goal_id: &str, lifecycle: GoalLifecycle) -> Goal {
    Goal {
        goal_id: goal_id.to_string(),
        agent_id: "agent".to_string(),
        target: Proposition::Holds {
            subject: node("readme"),
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
        lifecycle,
    }
}

fn goal_with_urgency(goal_id: &str, urgency: u32) -> Goal {
    let mut goal = goal(goal_id, GoalLifecycle::Active);
    goal.priority.urgency = urgency;
    goal
}

fn metadata(command_id: &str, source_identity: Option<&str>, seq: u64) -> GoalCommandMetadata {
    GoalCommandMetadata {
        command_id: command_id.to_string(),
        source_identity: source_identity.map(str::to_string),
        seq,
    }
}

fn acceptance_request(
    command_id: &str,
    source_identity: Option<&str>,
    seq: u64,
    goal: Goal,
    lifecycle_policy: GoalAcceptanceLifecycle,
) -> GoalAcceptanceRequest {
    GoalAcceptanceRequest {
        metadata: metadata(command_id, source_identity, seq),
        goal,
        lifecycle_policy,
    }
}

#[test]
fn add_and_query_one_active_ground_goal() {
    let mut store = GoalSetStore::new();
    let outcome = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some("source-a"), 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap();

    assert!(matches!(outcome, GoalCommandOutcome::Applied(_)));
    let query = GoalSetQuery::new(&store);
    assert_eq!(query.active_goals().len(), 1);
    assert_eq!(query.active_goal("goal-1").unwrap().goal_id, "goal-1");
}

#[test]
fn reject_non_ground_goal_target() {
    let mut invalid = goal("goal-1", GoalLifecycle::Active);
    invalid.target = Proposition::Accessible {
        scope: Term::Variable("?node".to_string()),
    };
    let mut store = GoalSetStore::new();

    let error = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", None, 1),
            goal: invalid,
        })
        .unwrap_err();

    assert!(error.to_string().contains("target must be ground"));
}

#[test]
fn reject_empty_goal_identity_agent_and_command_ids() {
    let mut store = GoalSetStore::new();
    let mut empty_goal_id = goal("goal-1", GoalLifecycle::Active);
    empty_goal_id.goal_id.clear();

    let goal_id_error = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", None, 1),
            goal: empty_goal_id,
        })
        .unwrap_err();

    assert!(goal_id_error.to_string().contains("goal id"));

    let mut empty_agent_id = goal("goal-2", GoalLifecycle::Active);
    empty_agent_id.agent_id.clear();
    let agent_id_error = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", None, 2),
            goal: empty_agent_id,
        })
        .unwrap_err();

    assert!(agent_id_error.to_string().contains("agent id"));

    let command_id_error = store
        .add_goal(AddGoalCommand {
            metadata: metadata("", None, 3),
            goal: goal("goal-3", GoalLifecycle::Active),
        })
        .unwrap_err();

    assert!(command_id_error.to_string().contains("goal command id"));

    let source_identity_error = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-4", Some("   "), 4),
            goal: goal("goal-4", GoalLifecycle::Active),
        })
        .unwrap_err();

    assert!(source_identity_error
        .to_string()
        .contains("goal source identity"));
}

#[test]
fn modify_preserves_proposed_lifecycle() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", None, 1),
            goal: goal("goal-1", GoalLifecycle::Proposed),
        })
        .unwrap();
    assert!(GoalSetQuery::new(&store).active_goals().is_empty());
    assert!(matches!(
        GoalSetQuery::new(&store).get_goal("goal-1"),
        Some(record) if matches!(record.goal.lifecycle, GoalLifecycle::Proposed)
    ));

    store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-2", None, 2),
            goal: goal("goal-1", GoalLifecycle::Proposed),
        })
        .unwrap();
    assert!(GoalSetQuery::new(&store).active_goals().is_empty());
}

#[test]
fn duplicate_source_identity_returns_existing_goal() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some("same-source"), 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap();

    let outcome = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", Some("same-source"), 2),
            goal: goal("goal-2", GoalLifecycle::Active),
        })
        .unwrap();

    assert_eq!(
        outcome,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn producer_neutral_goal_acceptance_activates_proposed_goal() {
    let mut store = GoalSetStore::new();
    let request = acceptance_request(
        "cmd-1",
        Some("source-a"),
        1,
        goal("goal-1", GoalLifecycle::Proposed),
        GoalAcceptanceLifecycle::RequireProposedThenActivate,
    );

    let outcome = GoalSetApi::new(&mut store).accept_goal(request).unwrap();

    assert!(matches!(
        outcome,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Active)
    ));
    assert!(GoalSetQuery::new(&store).active_goal("goal-1").is_some());
}

#[test]
fn producer_neutral_goal_acceptance_accepts_active_goal() {
    let mut store = GoalSetStore::new();
    let request = acceptance_request(
        "cmd-1",
        None,
        1,
        goal("goal-1", GoalLifecycle::Active),
        GoalAcceptanceLifecycle::RequireActive,
    );

    let outcome = GoalSetApi::new(&mut store).accept_goal(request).unwrap();

    assert!(matches!(
        outcome,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Active)
    ));
}

#[test]
fn producer_neutral_goal_acceptance_rejects_non_ground_goal() {
    let mut store = GoalSetStore::new();
    let mut invalid = goal("goal-1", GoalLifecycle::Proposed);
    invalid.target = Proposition::Accessible {
        scope: Term::Variable("?node".to_string()),
    };

    let error = GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request(
            "cmd-1",
            None,
            1,
            invalid,
            GoalAcceptanceLifecycle::RequireProposedThenActivate,
        ))
        .unwrap_err();

    assert!(matches!(error, GoalSetApiError::InvalidCommand(_)));
    assert!(error.to_string().contains("goal target must be ground"));
}

#[test]
fn producer_neutral_goal_acceptance_rejects_blank_source_identity() {
    let mut store = GoalSetStore::new();

    let error = GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request(
            "cmd-1",
            Some(""),
            1,
            goal("goal-1", GoalLifecycle::Proposed),
            GoalAcceptanceLifecycle::RequireProposedThenActivate,
        ))
        .unwrap_err();

    assert!(matches!(error, GoalSetApiError::InvalidCommand(_)));
    assert!(error.to_string().contains("goal source identity"));
}

#[test]
fn producer_neutral_goal_acceptance_rejects_satisfied_lifecycle() {
    let mut store = GoalSetStore::new();

    let error = GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request(
            "cmd-1",
            None,
            1,
            goal("goal-1", GoalLifecycle::Satisfied { at_seq: 9 }),
            GoalAcceptanceLifecycle::RequireProposedThenActivate,
        ))
        .unwrap_err();

    assert!(matches!(error, GoalSetApiError::InvalidCommand(_)));
    assert!(error
        .to_string()
        .contains("goal lifecycle must be proposed"));
}

#[test]
fn producer_neutral_goal_acceptance_replays_same_command_id() {
    let mut store = GoalSetStore::new();
    let request = acceptance_request(
        "cmd-1",
        Some("source-a"),
        1,
        goal("goal-1", GoalLifecycle::Proposed),
        GoalAcceptanceLifecycle::RequireProposedThenActivate,
    );
    let mut api = GoalSetApi::new(&mut store);

    let first = api.accept_goal(request.clone()).unwrap();
    let second = api.accept_goal(request).unwrap();

    assert_eq!(first, second);
}

#[test]
fn producer_neutral_goal_acceptance_dedupes_source_identity() {
    let mut store = GoalSetStore::new();
    let mut api = GoalSetApi::new(&mut store);
    api.accept_goal(acceptance_request(
        "cmd-1",
        Some("same-source"),
        1,
        goal("goal-1", GoalLifecycle::Proposed),
        GoalAcceptanceLifecycle::RequireProposedThenActivate,
    ))
    .unwrap();

    let duplicate = api
        .accept_goal(acceptance_request(
            "cmd-2",
            Some("same-source"),
            2,
            goal("goal-2", GoalLifecycle::Proposed),
            GoalAcceptanceLifecycle::RequireProposedThenActivate,
        ))
        .unwrap();

    assert_eq!(
        duplicate,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn goal_set_api_facade_applies_lifecycle_commands() {
    let mut store = GoalSetStore::new();
    let mut api = GoalSetApi::new(&mut store);
    api.add_goal(AddGoalCommand {
        metadata: metadata("cmd-1", None, 1),
        goal: goal("goal-1", GoalLifecycle::Active),
    })
    .unwrap();

    let suspended = api
        .suspend_goal(SuspendGoalCommand {
            metadata: metadata("cmd-2", None, 2),
            goal_id: "goal-1".to_string(),
            reason: "pause".to_string(),
        })
        .unwrap();
    assert!(matches!(
        suspended,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Suspended { .. })
    ));

    let resumed = api
        .resume_goal(ResumeGoalCommand {
            metadata: metadata("cmd-3", None, 3),
            goal_id: "goal-1".to_string(),
        })
        .unwrap();
    assert!(matches!(
        resumed,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Active)
    ));

    let satisfied = api
        .satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("cmd-4", None, 4),
            goal_id: "goal-1".to_string(),
            at_seq: 44,
        })
        .unwrap();
    assert!(matches!(
        satisfied,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Satisfied { at_seq: 44 })
    ));
}

#[test]
fn goal_acceptance_contracts_round_trip() {
    let request = acceptance_request(
        "cmd-1",
        Some("source-a"),
        1,
        goal("goal-1", GoalLifecycle::Proposed),
        GoalAcceptanceLifecycle::RequireProposedThenActivate,
    );

    let decoded: GoalAcceptanceRequest =
        serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();

    assert_eq!(decoded, request);
}

#[test]
fn goal_set_store_reindexes_source_identity_on_modify() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some("source-a"), 1),
            goal: goal("goal-1", GoalLifecycle::Proposed),
        })
        .unwrap();
    store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-2", Some("source-b"), 2),
            goal: goal("goal-1", GoalLifecycle::Proposed),
        })
        .unwrap();

    let old_source_outcome = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-3", Some("source-a"), 3),
            goal: goal("goal-2", GoalLifecycle::Active),
        })
        .unwrap();
    let new_source_outcome = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-4", Some("source-b"), 4),
            goal: goal("goal-3", GoalLifecycle::Active),
        })
        .unwrap();

    assert!(matches!(old_source_outcome, GoalCommandOutcome::Applied(_)));
    assert_eq!(
        new_source_outcome,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn goal_set_store_allows_modify_with_same_source_identity() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some("source-a"), 1),
            goal: goal("goal-1", GoalLifecycle::Proposed),
        })
        .unwrap();

    let outcome = store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-2", Some("source-a"), 2),
            goal: goal("goal-1", GoalLifecycle::Proposed),
        })
        .unwrap();

    assert!(matches!(outcome, GoalCommandOutcome::Applied(_)));
}

#[test]
fn goal_set_store_rejects_modify_to_existing_source_identity() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some("source-a"), 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", Some("source-b"), 2),
            goal: goal("goal-2", GoalLifecycle::Proposed),
        })
        .unwrap();

    let outcome = store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-3", Some("source-a"), 3),
            goal: goal("goal-2", GoalLifecycle::Proposed),
        })
        .unwrap();

    assert_eq!(
        outcome,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn lifecycle_commands_update_records_deterministically() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", None, 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap();

    let suspended = store
        .suspend_goal(SuspendGoalCommand {
            metadata: metadata("cmd-2", None, 2),
            goal_id: "goal-1".to_string(),
            reason: "pause".to_string(),
        })
        .unwrap();
    assert!(matches!(
        suspended,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Suspended { .. })
    ));

    store
        .resume_goal(ResumeGoalCommand {
            metadata: metadata("cmd-3", None, 3),
            goal_id: "goal-1".to_string(),
        })
        .unwrap();
    assert!(GoalSetQuery::new(&store).active_goal("goal-1").is_some());

    let satisfied = store
        .satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("cmd-4", None, 4),
            goal_id: "goal-1".to_string(),
            at_seq: 44,
        })
        .unwrap();
    assert!(matches!(
        satisfied,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Satisfied { at_seq: 44 })
    ));
}

#[test]
fn commands_are_idempotent_by_command_id() {
    let mut store = GoalSetStore::new();
    let command = AddGoalCommand {
        metadata: metadata("cmd-1", Some("source"), 1),
        goal: goal("goal-1", GoalLifecycle::Active),
    };
    let first = store.add_goal(command.clone()).unwrap();
    let second = store.add_goal(command).unwrap();
    assert_eq!(first, second);
}

#[test]
fn goal_contracts_round_trip() {
    let command = AddGoalCommand {
        metadata: metadata("cmd-1", Some("source"), 1),
        goal: goal("goal-1", GoalLifecycle::Active),
    };
    let decoded: AddGoalCommand =
        serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap();
    assert_eq!(decoded, command);
}

#[test]
fn persistent_goal_store_reopens_records_and_command_outcomes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    let command = AddGoalCommand {
        metadata: metadata("cmd-1", Some("source"), 1),
        goal: goal("goal-1", GoalLifecycle::Active),
    };

    let first = {
        let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
        let outcome = store.add_goal(command.clone()).unwrap();
        assert_eq!(store.active_goals().unwrap().len(), 1);
        store.flush().unwrap();
        outcome
    };

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    assert_eq!(
        store.active_goal("goal-1").unwrap().unwrap().goal_id,
        "goal-1"
    );
    assert_eq!(store.add_goal(command).unwrap(), first);
}

#[test]
fn persistent_goal_store_recovers_applied_outcome_from_split_record_write() {
    let dir = tempfile::tempdir().unwrap();
    let db = sled::open(dir.path().join("goals")).unwrap();
    let command = AddGoalCommand {
        metadata: metadata("cmd-1", Some("source"), 1),
        goal: goal("goal-1", GoalLifecycle::Active),
    };
    let record = ExecutionGoalRecord {
        goal: command.goal.clone(),
        source_command_id: Some(command.metadata.command_id.clone()),
        source_identity: command.metadata.source_identity.clone(),
        created_at_seq: command.metadata.seq,
        updated_at_seq: command.metadata.seq,
    };

    db.open_tree("execution_goal_records")
        .unwrap()
        .insert(
            command.goal.goal_id.as_bytes(),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    let store = PersistentGoalSetStore::new(db).unwrap();
    let expected = GoalCommandOutcome::Applied(Box::new(record));
    let recovered = store.add_goal(command.clone()).unwrap();
    assert_eq!(recovered, expected);
    assert_eq!(store.add_goal(command).unwrap(), expected);

    let duplicate = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", Some("source"), 2),
            goal: goal("goal-2", GoalLifecycle::Active),
        })
        .unwrap();

    assert_eq!(
        duplicate,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn persistent_goal_store_rejects_blank_source_identity() {
    let dir = tempfile::tempdir().unwrap();
    let store = PersistentGoalSetStore::new(sled::open(dir.path().join("goals")).unwrap()).unwrap();

    let error = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some("   "), 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap_err();

    assert!(error.to_string().contains("goal source identity"));
    assert!(store.goal_records().unwrap().is_empty());
}

#[test]
fn persistent_goal_store_preserves_source_identity_dedupe_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");

    {
        let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
        store
            .add_goal(AddGoalCommand {
                metadata: metadata("cmd-1", Some("same-source"), 1),
                goal: goal("goal-1", GoalLifecycle::Proposed),
            })
            .unwrap();
        store.flush().unwrap();
    }

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    let duplicate = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", Some("same-source"), 2),
            goal: goal("goal-2", GoalLifecycle::Proposed),
        })
        .unwrap();

    assert_eq!(
        duplicate,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn persistent_goal_store_preserves_lifecycle_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");

    {
        let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
        store
            .add_goal(AddGoalCommand {
                metadata: metadata("cmd-1", None, 1),
                goal: goal("goal-1", GoalLifecycle::Active),
            })
            .unwrap();
        store
            .satisfy_goal(SatisfyGoalCommand {
                metadata: metadata("cmd-2", None, 2),
                goal_id: "goal-1".to_string(),
                at_seq: 44,
            })
            .unwrap();
        store.flush().unwrap();
    }

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    assert!(store.active_goal("goal-1").unwrap().is_none());
    let record = store.get_goal("goal-1").unwrap().unwrap();
    assert!(matches!(
        record.goal.lifecycle,
        GoalLifecycle::Satisfied { at_seq: 44 }
    ));
}

#[test]
fn persistent_goal_store_reindexes_source_identity_on_modify() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");

    {
        let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
        store
            .add_goal(AddGoalCommand {
                metadata: metadata("cmd-1", Some("source-a"), 1),
                goal: goal("goal-1", GoalLifecycle::Proposed),
            })
            .unwrap();
        store
            .modify_goal(ModifyGoalCommand {
                metadata: metadata("cmd-2", Some("source-b"), 2),
                goal: goal("goal-1", GoalLifecycle::Proposed),
            })
            .unwrap();
        store.flush().unwrap();
    }

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    let old_source_outcome = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-3", Some("source-a"), 3),
            goal: goal("goal-2", GoalLifecycle::Active),
        })
        .unwrap();
    let new_source_outcome = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-4", Some("source-b"), 4),
            goal: goal("goal-3", GoalLifecycle::Active),
        })
        .unwrap();

    assert!(matches!(old_source_outcome, GoalCommandOutcome::Applied(_)));
    assert_eq!(
        new_source_outcome,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn persistent_goal_store_rejects_modify_to_existing_source_identity() {
    let dir = tempfile::tempdir().unwrap();
    let store = PersistentGoalSetStore::new(sled::open(dir.path().join("goals")).unwrap()).unwrap();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some("source-a"), 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", Some("source-b"), 2),
            goal: goal("goal-2", GoalLifecycle::Proposed),
        })
        .unwrap();

    let outcome = store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-3", Some("source-a"), 3),
            goal: goal("goal-2", GoalLifecycle::Proposed),
        })
        .unwrap();

    assert_eq!(
        outcome,
        GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string()
        }
    );
}

#[test]
fn persistent_goal_store_returns_records_in_goal_id_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = PersistentGoalSetStore::new(sled::open(dir.path().join("goals")).unwrap()).unwrap();
    for (command_id, goal_id) in [
        ("cmd-1", "goal-c"),
        ("cmd-2", "goal-a"),
        ("cmd-3", "goal-b"),
    ] {
        store
            .add_goal(AddGoalCommand {
                metadata: metadata(command_id, None, 1),
                goal: goal(goal_id, GoalLifecycle::Active),
            })
            .unwrap();
    }

    let goal_ids = store
        .goal_records()
        .unwrap()
        .into_iter()
        .map(|record| record.goal.goal_id)
        .collect::<Vec<_>>();

    assert_eq!(goal_ids, vec!["goal-a", "goal-b", "goal-c"]);
}

#[test]
fn persistent_goal_store_reports_corrupt_record_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = sled::open(dir.path().join("goals")).unwrap();
    db.open_tree("execution_goal_records")
        .unwrap()
        .insert("goal-bad", b"not json".as_slice())
        .unwrap();
    db.flush().unwrap();

    let store = PersistentGoalSetStore::new(db).unwrap();
    let error = store.get_goal("goal-bad").unwrap_err();

    assert!(error.to_string().contains("goal store JSON failed"));
}

#[test]
fn persistent_goal_store_can_be_opened_outside_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    assert!(!storage.path().starts_with(workspace.path()));

    let store =
        PersistentGoalSetStore::new(sled::open(storage.path().join("goals")).unwrap()).unwrap();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", None, 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap();
    store.flush().unwrap();
}

#[test]
fn persistent_goal_store_commands_flush_pending_bytes_to_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    let db = sled::Config::new()
        .path(&path)
        .flush_every_ms(None)
        .open()
        .unwrap();
    let store = PersistentGoalSetStore::new(db.clone()).unwrap();

    let before_commits = db.size_on_disk().unwrap();
    for index in 0..64 {
        let mut goal = goal(&format!("goal-{index}"), GoalLifecycle::Active);
        goal.source = GoalSource::UserDirected {
            directive: "refresh docs ".repeat(256),
        };
        store
            .add_goal(AddGoalCommand {
                metadata: metadata(&format!("cmd-{index}"), None, index + 1),
                goal,
            })
            .unwrap();
    }

    let after_commits = db.size_on_disk().unwrap();

    assert!(
        after_commits > before_commits,
        "expected durable commands to increase on-disk bytes from {before_commits}, got {after_commits}"
    );
}

#[test]
fn in_memory_store_rejects_divergent_add_replay() {
    let mut store = GoalSetStore::new();
    let first = AddGoalCommand {
        metadata: metadata("cmd-reused", Some("source-a"), 1),
        goal: goal("goal-a", GoalLifecycle::Active),
    };
    let mut divergent = first.clone();
    divergent.goal.priority.urgency = 99;

    store.add_goal(first).unwrap();
    let error = store.add_goal(divergent).unwrap_err();

    assert!(error.to_string().contains("replayed with divergent intent"));
    assert_eq!(
        GoalSetQuery::new(&store)
            .get_goal("goal-a")
            .unwrap()
            .goal
            .priority
            .urgency,
        1
    );
}

#[test]
fn acceptance_replay_identity_includes_pre_normalized_policy_and_lifecycle() {
    let mut store = GoalSetStore::new();
    let proposed = acceptance_request(
        "cmd-accept",
        Some("source-a"),
        1,
        goal("goal-a", GoalLifecycle::Proposed),
        GoalAcceptanceLifecycle::RequireProposedThenActivate,
    );
    let active = acceptance_request(
        "cmd-accept",
        Some("source-a"),
        1,
        goal("goal-a", GoalLifecycle::Active),
        GoalAcceptanceLifecycle::RequireActive,
    );
    let mut api = GoalSetApi::new(&mut store);

    api.accept_goal(proposed).unwrap();
    let error = api.accept_goal(active).unwrap_err();

    assert!(error.to_string().contains("replayed with divergent intent"));
}

#[test]
fn in_memory_store_rejects_divergent_lifecycle_replay() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-add", None, 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        })
        .unwrap();
    let first = SatisfyGoalCommand {
        metadata: metadata("cmd-satisfy", None, 2),
        goal_id: "goal-a".to_string(),
        at_seq: 40,
    };
    let mut divergent = first.clone();
    divergent.at_seq = 41;

    store.satisfy_goal(first).unwrap();
    let error = store.satisfy_goal(divergent).unwrap_err();

    assert!(error.to_string().contains("replayed with divergent intent"));
    assert!(matches!(
        GoalSetQuery::new(&store)
            .get_goal("goal-a")
            .unwrap()
            .goal
            .lifecycle,
        GoalLifecycle::Satisfied { at_seq: 40 }
    ));
}

#[test]
fn active_goal_selection_orders_by_urgency_then_stable_goal_id() {
    let mut store = GoalSetStore::new();
    for command in [
        AddGoalCommand {
            metadata: metadata("cmd-c", None, 1),
            goal: goal_with_urgency("goal-c", 2),
        },
        AddGoalCommand {
            metadata: metadata("cmd-b", None, 2),
            goal: goal_with_urgency("goal-b", 1),
        },
        AddGoalCommand {
            metadata: metadata("cmd-a", None, 3),
            goal: goal_with_urgency("goal-a", 1),
        },
    ] {
        store.add_goal(command).unwrap();
    }

    let goal_ids = GoalSetQuery::new(&store)
        .active_goals()
        .into_iter()
        .map(|goal| goal.goal_id.clone())
        .collect::<Vec<_>>();

    assert_eq!(goal_ids, vec!["goal-a", "goal-b", "goal-c"]);
}

#[test]
fn persistent_active_goal_selection_orders_by_urgency_then_stable_goal_id() {
    let dir = tempfile::tempdir().unwrap();
    let store = PersistentGoalSetStore::new(sled::open(dir.path().join("goals")).unwrap()).unwrap();
    for command in [
        AddGoalCommand {
            metadata: metadata("cmd-c", None, 1),
            goal: goal_with_urgency("goal-c", 2),
        },
        AddGoalCommand {
            metadata: metadata("cmd-b", None, 2),
            goal: goal_with_urgency("goal-b", 1),
        },
        AddGoalCommand {
            metadata: metadata("cmd-a", None, 3),
            goal: goal_with_urgency("goal-a", 1),
        },
    ] {
        store.add_goal(command).unwrap();
    }

    let goal_ids = store
        .active_goals()
        .unwrap()
        .into_iter()
        .map(|goal| goal.goal_id)
        .collect::<Vec<_>>();

    assert_eq!(goal_ids, vec!["goal-a", "goal-b", "goal-c"]);
}

#[test]
fn persistent_modify_commit_reopens_as_one_complete_unit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    let modify = ModifyGoalCommand {
        metadata: metadata("cmd-modify", Some("source-b"), 2),
        goal: goal_with_urgency("goal-a", 7),
    };
    let expected_identity = modify.request_identity().unwrap();
    let receipt = {
        let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
        store
            .add_goal(AddGoalCommand {
                metadata: metadata("cmd-add", Some("source-a"), 1),
                goal: goal("goal-a", GoalLifecycle::Active),
            })
            .unwrap();
        let outcome = store.commit_modify_goal(modify.clone()).unwrap();
        assert!(matches!(outcome.0, GoalCommandOutcome::Applied(_)));
        outcome.1
    };

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    assert_eq!(
        store.command_identity("cmd-modify").unwrap(),
        Some(expected_identity)
    );
    assert_eq!(
        store.command_receipt("cmd-modify").unwrap(),
        Some(receipt.clone())
    );
    assert_eq!(
        store
            .get_goal("goal-a")
            .unwrap()
            .unwrap()
            .goal
            .priority
            .urgency,
        7
    );
    assert_eq!(store.commit_modify_goal(modify).unwrap().1, receipt);
}

#[test]
fn persistent_store_rejects_divergent_replay_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    let first = AddGoalCommand {
        metadata: metadata("cmd-reused", Some("source-a"), 1),
        goal: goal("goal-a", GoalLifecycle::Active),
    };
    {
        let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
        store.add_goal(first.clone()).unwrap();
    }
    let mut divergent = first;
    divergent.goal.priority.urgency = 99;

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    let error = store.add_goal(divergent).unwrap_err();

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
}

#[test]
fn concurrent_divergent_writers_commit_exactly_one_request() {
    let dir = tempfile::tempdir().unwrap();
    let store = PersistentGoalSetStore::new(sled::open(dir.path().join("goals")).unwrap()).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let commands = [
        AddGoalCommand {
            metadata: metadata("cmd-race", None, 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        },
        AddGoalCommand {
            metadata: metadata("cmd-race", None, 1),
            goal: goal("goal-b", GoalLifecycle::Active),
        },
    ];
    let handles = commands.map(|command| {
        let store = store.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            let identity = command.request_identity().unwrap();
            let result = store.commit_add_goal(command);
            (identity, result)
        })
    });
    let results = handles.map(|handle| handle.join().unwrap());

    assert_eq!(results.iter().filter(|result| result.1.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.1.is_err()).count(), 1);
    let winner = results.iter().find(|result| result.1.is_ok()).unwrap();
    let stored_identity = store.command_identity("cmd-race").unwrap().unwrap();
    let stored_receipt = store.command_receipt("cmd-race").unwrap().unwrap();
    assert_eq!(stored_identity, winner.0);
    assert_eq!(stored_receipt.identity, winner.0);
    assert_eq!(store.goal_records().unwrap().len(), 1);
}

#[test]
fn durable_api_receipt_survives_reopen_without_manual_flush() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    let receipt = {
        let db = sled::Config::new()
            .path(&path)
            .flush_every_ms(None)
            .open()
            .unwrap();
        let mut store = PersistentGoalSetStore::new(db).unwrap();
        let command = AddGoalCommand {
            metadata: metadata("cmd-durable", None, 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        };
        GoalSetApi::new(&mut store)
            .add_goal_durable(command)
            .unwrap()
            .1
    };

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    assert_eq!(store.command_receipt("cmd-durable").unwrap(), Some(receipt));
    assert!(store.get_goal("goal-a").unwrap().is_some());
}

#[test]
fn concurrent_public_receipt_observation_flushes_pending_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let db = sled::Config::new()
        .path(dir.path().join("goals"))
        .flush_every_ms(None)
        .open()
        .unwrap();
    let store = PersistentGoalSetStore::new(db.clone()).unwrap();
    let receipt = GoalCommandCommitReceipt {
        identity: GoalCommandRequestIdentity {
            command_id: "cmd-pending".to_string(),
            command_kind: GoalCommandKind::Add,
            request_hash: "0".repeat(64),
        },
        outcome_hash: "1".repeat(64),
    };
    db.open_tree("execution_goal_command_receipts")
        .unwrap()
        .insert("cmd-pending", serde_json::to_vec(&receipt).unwrap())
        .unwrap();
    let observer = store.clone();

    let observed = thread::spawn(move || observer.command_receipt("cmd-pending").unwrap())
        .join()
        .unwrap();

    assert_eq!(observed, Some(receipt));
    assert_eq!(db.flush().unwrap(), 0);
}

#[test]
fn compatible_legacy_applied_outcome_is_verified_and_upgraded() {
    let dir = tempfile::tempdir().unwrap();
    let db = sled::open(dir.path().join("goals")).unwrap();
    let command = AddGoalCommand {
        metadata: metadata("cmd-legacy", Some("source-a"), 1),
        goal: goal("goal-a", GoalLifecycle::Active),
    };
    let record = ExecutionGoalRecord {
        goal: command.goal.clone(),
        source_command_id: Some(command.metadata.command_id.clone()),
        source_identity: command.metadata.source_identity.clone(),
        created_at_seq: command.metadata.seq,
        updated_at_seq: command.metadata.seq,
    };
    db.open_tree("execution_goal_records")
        .unwrap()
        .insert("goal-a", serde_json::to_vec(&record).unwrap())
        .unwrap();
    db.open_tree("execution_goal_command_outcomes")
        .unwrap()
        .insert(
            "cmd-legacy",
            serde_json::to_vec(&GoalCommandOutcome::Applied(Box::new(record.clone()))).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    let store = PersistentGoalSetStore::new(db).unwrap();
    let outcome = store.add_goal(command.clone()).unwrap();

    assert_eq!(outcome, GoalCommandOutcome::Applied(Box::new(record)));
    assert_eq!(
        store.command_identity("cmd-legacy").unwrap(),
        Some(command.request_identity().unwrap())
    );
    assert!(store.command_receipt("cmd-legacy").unwrap().is_some());
}

#[test]
fn legacy_add_outcome_does_not_guess_pre_normalized_acceptance_intent() {
    let dir = tempfile::tempdir().unwrap();
    let db = sled::open(dir.path().join("goals")).unwrap();
    let request = acceptance_request(
        "cmd-legacy",
        Some("source-a"),
        1,
        goal("goal-a", GoalLifecycle::Proposed),
        GoalAcceptanceLifecycle::RequireProposedThenActivate,
    );
    let record = ExecutionGoalRecord {
        goal: goal("goal-a", GoalLifecycle::Active),
        source_command_id: Some("cmd-legacy".to_string()),
        source_identity: Some("source-a".to_string()),
        created_at_seq: 1,
        updated_at_seq: 1,
    };
    db.open_tree("execution_goal_command_outcomes")
        .unwrap()
        .insert(
            "cmd-legacy",
            serde_json::to_vec(&GoalCommandOutcome::Applied(Box::new(record))).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();
    let mut store = PersistentGoalSetStore::new(db).unwrap();

    let error = GoalSetApi::new(&mut store)
        .accept_goal(request)
        .unwrap_err();

    assert!(error.to_string().contains("replayed with divergent intent"));
    assert!(store.command_identity("cmd-legacy").unwrap().is_none());
}

#[test]
fn ambiguous_legacy_lifecycle_outcome_is_not_upgraded() {
    let dir = tempfile::tempdir().unwrap();
    let db = sled::open(dir.path().join("goals")).unwrap();
    let command = RemoveGoalCommand {
        metadata: metadata("cmd-legacy", None, 2),
        goal_id: "goal-a".to_string(),
        reason: "done".to_string(),
    };
    let record = ExecutionGoalRecord {
        goal: goal(
            "goal-a",
            GoalLifecycle::Abandoned {
                reason: "done".to_string(),
            },
        ),
        source_command_id: Some("cmd-legacy".to_string()),
        source_identity: Some("source-a".to_string()),
        created_at_seq: 1,
        updated_at_seq: 2,
    };
    db.open_tree("execution_goal_command_outcomes")
        .unwrap()
        .insert(
            "cmd-legacy",
            serde_json::to_vec(&GoalCommandOutcome::Applied(Box::new(record))).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    let store = PersistentGoalSetStore::new(db).unwrap();
    let error = store.remove_goal(command).unwrap_err();

    assert!(error.to_string().contains("replayed with divergent intent"));
    assert!(store.command_identity("cmd-legacy").unwrap().is_none());
}

#[test]
fn stale_unique_modify_and_lifecycle_commands_are_rejected() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-add", None, 10),
            goal: goal("goal-a", GoalLifecycle::Active),
        })
        .unwrap();
    let mut replacement = goal_with_urgency("goal-a", 9);
    replacement.lifecycle = GoalLifecycle::Active;

    let modify_error = store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-modify", None, 10),
            goal: replacement,
        })
        .unwrap_err();
    let lifecycle_error = store
        .satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("cmd-satisfy", None, 9),
            goal_id: "goal-a".to_string(),
            at_seq: 9,
        })
        .unwrap_err();

    assert!(modify_error.to_string().contains("must be newer"));
    assert!(lifecycle_error.to_string().contains("must be newer"));
    let record = GoalSetQuery::new(&store).get_goal("goal-a").unwrap();
    assert_eq!(record.updated_at_seq, 10);
    assert_eq!(record.goal.priority.urgency, 1);
    assert!(matches!(record.goal.lifecycle, GoalLifecycle::Active));
}

#[test]
fn legacy_replay_policy_selection_stays_crate_owned() {
    let goals_module = include_str!("../src/goals.rs");
    let contracts = include_str!("../src/goals/contracts.rs");
    let persistent_store = include_str!("../src/goals/persistent_store.rs");

    assert!(!goals_module.contains("LegacyGoalCommandReplayPolicy"));
    assert!(!contracts.contains("pub enum LegacyGoalCommandReplayPolicy"));
    assert!(!persistent_store.contains("pub fn with_legacy_replay_policy"));
}

#[test]
fn modify_cannot_bypass_lifecycle_commands_or_resume_terminal_goal() {
    let mut store = GoalSetStore::new();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-add", None, 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        })
        .unwrap();
    let mut invalid_modify = goal_with_urgency("goal-a", 8);
    invalid_modify.lifecycle = GoalLifecycle::Satisfied { at_seq: 2 };
    let modify_error = store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-modify", None, 2),
            goal: invalid_modify,
        })
        .unwrap_err();
    store
        .satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("cmd-satisfy", None, 3),
            goal_id: "goal-a".to_string(),
            at_seq: 3,
        })
        .unwrap();
    let resume_error = store
        .resume_goal(ResumeGoalCommand {
            metadata: metadata("cmd-resume", None, 4),
            goal_id: "goal-a".to_string(),
        })
        .unwrap_err();

    assert!(modify_error
        .to_string()
        .contains("preserve the stored lifecycle"));
    assert!(resume_error.to_string().contains("illegal Resume"));
    let record = GoalSetQuery::new(&store).get_goal("goal-a").unwrap();
    assert_eq!(record.goal.priority.urgency, 1);
    assert!(matches!(
        record.goal.lifecycle,
        GoalLifecycle::Satisfied { at_seq: 3 }
    ));
}

#[test]
fn persistent_stale_lifecycle_rejection_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    {
        let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
        store
            .add_goal(AddGoalCommand {
                metadata: metadata("cmd-add", None, 10),
                goal: goal("goal-a", GoalLifecycle::Active),
            })
            .unwrap();
    }
    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    let modify_error = store
        .modify_goal(ModifyGoalCommand {
            metadata: metadata("cmd-stale-modify", None, 10),
            goal: goal_with_urgency("goal-a", 9),
        })
        .unwrap_err();
    let lifecycle_error = store
        .satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("cmd-stale-lifecycle", None, 9),
            goal_id: "goal-a".to_string(),
            at_seq: 9,
        })
        .unwrap_err();

    assert!(modify_error.to_string().contains("must be newer"));
    assert!(lifecycle_error.to_string().contains("must be newer"));
    assert!(matches!(
        store.get_goal("goal-a").unwrap().unwrap().goal.lifecycle,
        GoalLifecycle::Active
    ));
    assert!(store
        .command_identity("cmd-stale-modify")
        .unwrap()
        .is_none());
    assert!(store
        .command_identity("cmd-stale-lifecycle")
        .unwrap()
        .is_none());
}

#[test]
fn concurrent_modifies_converge_on_newest_sequence_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-add", None, 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        })
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles = [("cmd-two", 2, 2), ("cmd-three", 3, 3)].map(|(command_id, seq, urgency)| {
        let store = store.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            store.modify_goal(ModifyGoalCommand {
                metadata: metadata(command_id, None, seq),
                goal: goal_with_urgency("goal-a", urgency),
            })
        })
    });
    let results = handles.map(|handle| handle.join().unwrap());
    assert!(results[1].is_ok());
    drop(store);

    let reopened = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    let record = reopened.get_goal("goal-a").unwrap().unwrap();
    assert_eq!(record.updated_at_seq, 3);
    assert_eq!(record.goal.priority.urgency, 3);
    assert!(matches!(record.goal.lifecycle, GoalLifecycle::Active));
}

#[test]
fn source_collision_does_not_mask_stale_modify_in_either_store() {
    let mut memory = GoalSetStore::new();
    let dir = tempfile::tempdir().unwrap();
    let persistent =
        PersistentGoalSetStore::new(sled::open(dir.path().join("goals")).unwrap()).unwrap();
    for command in [
        AddGoalCommand {
            metadata: metadata("cmd-add-a", Some("source-a"), 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        },
        AddGoalCommand {
            metadata: metadata("cmd-add-b", Some("source-b"), 10),
            goal: goal("goal-b", GoalLifecycle::Active),
        },
    ] {
        memory.add_goal(command.clone()).unwrap();
        persistent.add_goal(command).unwrap();
    }
    let stale = ModifyGoalCommand {
        metadata: metadata("cmd-collision", Some("source-a"), 10),
        goal: goal("goal-b", GoalLifecycle::Active),
    };

    let memory_error = memory.modify_goal(stale.clone()).unwrap_err();
    let persistent_error = persistent.modify_goal(stale).unwrap_err();
    assert!(memory_error.to_string().contains("must be newer"));
    assert!(persistent_error.to_string().contains("must be newer"));

    let corrected = ModifyGoalCommand {
        metadata: metadata("cmd-collision", Some("source-a"), 11),
        goal: goal("goal-b", GoalLifecycle::Active),
    };
    let expected = GoalCommandOutcome::Duplicate {
        existing_goal_id: "goal-a".to_string(),
    };
    assert_eq!(memory.modify_goal(corrected.clone()).unwrap(), expected);
    assert_eq!(persistent.modify_goal(corrected).unwrap(), expected);
}

#[test]
fn source_collision_does_not_mask_illegal_modify_lifecycle_in_either_store() {
    let mut memory = GoalSetStore::new();
    let dir = tempfile::tempdir().unwrap();
    let persistent =
        PersistentGoalSetStore::new(sled::open(dir.path().join("goals")).unwrap()).unwrap();
    for command in [
        AddGoalCommand {
            metadata: metadata("cmd-add-a", Some("source-a"), 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        },
        AddGoalCommand {
            metadata: metadata("cmd-add-b", Some("source-b"), 10),
            goal: goal("goal-b", GoalLifecycle::Active),
        },
    ] {
        memory.add_goal(command.clone()).unwrap();
        persistent.add_goal(command).unwrap();
    }
    let illegal = ModifyGoalCommand {
        metadata: metadata("cmd-collision", Some("source-a"), 11),
        goal: goal("goal-b", GoalLifecycle::Satisfied { at_seq: 11 }),
    };

    let memory_error = memory.modify_goal(illegal.clone()).unwrap_err();
    let persistent_error = persistent.modify_goal(illegal).unwrap_err();
    assert!(memory_error
        .to_string()
        .contains("preserve the stored lifecycle"));
    assert!(persistent_error
        .to_string()
        .contains("preserve the stored lifecycle"));

    let corrected = ModifyGoalCommand {
        metadata: metadata("cmd-collision", Some("source-a"), 11),
        goal: goal("goal-b", GoalLifecycle::Active),
    };
    let expected = GoalCommandOutcome::Duplicate {
        existing_goal_id: "goal-a".to_string(),
    };
    assert_eq!(memory.modify_goal(corrected.clone()).unwrap(), expected);
    assert_eq!(persistent.modify_goal(corrected).unwrap(), expected);
}

proptest! {
    #[test]
    fn repeated_source_identity_is_deterministic(source in "[a-z][a-z0-9_]{0,12}") {
        let mut store = GoalSetStore::new();
        store.add_goal(AddGoalCommand {
            metadata: metadata("cmd-1", Some(&source), 1),
            goal: goal("goal-1", GoalLifecycle::Active),
        }).unwrap();
        let duplicate = store.add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", Some(&source), 2),
            goal: goal("goal-2", GoalLifecycle::Active),
        }).unwrap();

        prop_assert_eq!(duplicate, GoalCommandOutcome::Duplicate {
            existing_goal_id: "goal-1".to_string(),
        });
    }

    #[test]
    fn changing_any_satisfaction_sequence_rejects_replay(first_seq in any::<u64>(), second_seq in any::<u64>()) {
        prop_assume!(first_seq != second_seq);
        let mut store = GoalSetStore::new();
        store.add_goal(AddGoalCommand {
            metadata: metadata("cmd-add", None, 1),
            goal: goal("goal-a", GoalLifecycle::Active),
        }).unwrap();
        store.satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("cmd-satisfy", None, 2),
            goal_id: "goal-a".to_string(),
            at_seq: first_seq,
        }).unwrap();

        let replay = store.satisfy_goal(SatisfyGoalCommand {
            metadata: metadata("cmd-satisfy", None, 2),
            goal_id: "goal-a".to_string(),
            at_seq: second_seq,
        });

        prop_assert!(replay.unwrap_err().to_string().contains("replayed with divergent intent"));
    }
}
