use meld_events::DomainObjectRef;
use meld_execution::goals::{
    AddGoalCommand, ExecutionGoalRecord, GoalAcceptanceLifecycle, GoalAcceptanceRequest,
    GoalCommandMetadata, GoalCommandOutcome, GoalSetApi, GoalSetApiError, GoalSetQuery,
    GoalSetStore, ModifyGoalCommand, PersistentGoalSetStore, RemoveGoalCommand, ResumeGoalCommand,
    SatisfyGoalCommand, SuspendGoalCommand,
};
use meld_lang::{
    Condition, Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term,
};
use proptest::prelude::*;

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
fn proposed_goal_is_ignored_until_modified_active() {
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
            goal: goal("goal-1", GoalLifecycle::Active),
        })
        .unwrap();
    assert_eq!(GoalSetQuery::new(&store).active_goals().len(), 1);
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

    let removed = api
        .remove_goal(RemoveGoalCommand {
            metadata: metadata("cmd-5", None, 5),
            goal_id: "goal-1".to_string(),
            reason: "superseded".to_string(),
        })
        .unwrap();
    assert!(matches!(
        removed,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Abandoned { .. })
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
            goal: goal("goal-1", GoalLifecycle::Active),
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
            goal: goal("goal-1", GoalLifecycle::Active),
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

    let removed = store
        .remove_goal(RemoveGoalCommand {
            metadata: metadata("cmd-5", None, 5),
            goal_id: "goal-1".to_string(),
            reason: "done".to_string(),
        })
        .unwrap();
    assert!(matches!(
        removed,
        GoalCommandOutcome::Applied(record)
            if matches!(record.goal.lifecycle, GoalLifecycle::Abandoned { .. })
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
        lifecycle_epoch: 0,
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
                goal: goal("goal-1", GoalLifecycle::Active),
            })
            .unwrap();
        store.flush().unwrap();
    }

    let store = PersistentGoalSetStore::new(sled::open(&path).unwrap()).unwrap();
    let duplicate = store
        .add_goal(AddGoalCommand {
            metadata: metadata("cmd-2", Some("same-source"), 2),
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
                goal: goal("goal-1", GoalLifecycle::Active),
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
fn persistent_goal_store_flush_writes_pending_bytes_to_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals");
    let db = sled::Config::new()
        .path(&path)
        .flush_every_ms(None)
        .open()
        .unwrap();
    let store = PersistentGoalSetStore::new(db.clone()).unwrap();

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

    let before_flush = db.size_on_disk().unwrap();
    store.flush().unwrap();
    let after_flush = db.size_on_disk().unwrap();

    assert!(
        after_flush > before_flush,
        "expected flush to increase on-disk bytes from {before_flush}, got {after_flush}"
    );
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
}
