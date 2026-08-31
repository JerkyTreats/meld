use meld_events::DomainObjectRef;
use meld_lang::{
    Condition, Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term,
};
use meld_world_model::{
    AgentCurrentnessCheck, AgentMilestoneAcceptance, AgentPlanJudgment, AgentPlanJudgmentKind,
    AgentProductProgress, AgentProductState, AgentReconciliationGoal, AgentStore,
    PlanMilestoneRequirement,
};

fn goal() -> Goal {
    Goal {
        goal_id: "goal-reconciliation".to_string(),
        agent_id: "agent-reconciliation".to_string(),
        target: Proposition::Holds {
            subject: Term::Object(DomainObjectRef::new("workspace_fs", "node", "meld").unwrap()),
            dimension: Term::Dimension("docs_freshness".to_string()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
        },
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: None,
        },
        source: GoalSource::Maintenance {
            invariant_description: "documentation remains fresh".to_string(),
        },
        lifecycle: GoalLifecycle::Proposed,
    }
}

#[test]
fn reconciliation_owner_records_survive_shared_database_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let db = sled::open(temp.path().join("world_model.sled")).unwrap();
    let store = AgentStore::new(db.clone()).unwrap();
    let goal_record = AgentReconciliationGoal {
        goal: goal(),
        context_id: "context-v1".to_string(),
        authority_scope_id: "authority-v1".to_string(),
        activation_generation: "activation-v1".to_string(),
        created_at_seq: 7,
    };
    let judgment = AgentPlanJudgment {
        judgment_id: "judgment-v1".to_string(),
        agent_id: "agent-reconciliation".to_string(),
        goal_id: goal_record.goal.goal_id.clone(),
        plan_revision_id: "plan-v1".to_string(),
        context_id: "context-v1".to_string(),
        authority_scope_id: "authority-v1".to_string(),
        activation_generation: "activation-v1".to_string(),
        kind: AgentPlanJudgmentKind::Admitted,
    };
    let progress = AgentProductProgress {
        progress_id: "progress-v1".to_string(),
        agent_id: "agent-reconciliation".to_string(),
        goal_id: goal_record.goal.goal_id.clone(),
        plan_revision_id: "plan-v1".to_string(),
        product_id: "task-v1".to_string(),
        context_id: "context-v1".to_string(),
        authority_scope_id: "authority-v1".to_string(),
        activation_generation: "activation-v1".to_string(),
        currentness: AgentCurrentnessCheck {
            frozen_cut_id: "cut-v1".to_string(),
            observed_cut_id: Some("cut-v1".to_string()),
            refusal: None,
        },
        state: AgentProductState::Eligible,
    };
    let milestone = AgentMilestoneAcceptance {
        milestone_id: "milestone-v1".to_string(),
        agent_id: "agent-reconciliation".to_string(),
        goal_id: goal_record.goal.goal_id.clone(),
        plan_revision_id: "plan-v1".to_string(),
        product_id: "epistemic-v1".to_string(),
        requirement: PlanMilestoneRequirement::CurationTerminal {
            operation_id: "curation-operation-v1".to_string(),
        },
        owner_position_id: "curation-result-v1".to_string(),
        context_id: "context-v1".to_string(),
        activation_generation: "activation-v1".to_string(),
    };
    store.put_reconciliation_goal(&goal_record).unwrap();
    store.put_plan_judgment(&judgment).unwrap();
    store.put_product_progress(&progress).unwrap();
    store.put_milestone(&milestone).unwrap();
    store.flush().unwrap();
    drop(store);

    let reopened = AgentStore::new(db).unwrap();
    assert_eq!(
        reopened
            .reconciliation_goal(&goal_record.goal.goal_id)
            .unwrap(),
        Some(goal_record)
    );
    assert_eq!(
        reopened.plan_judgment(&judgment.judgment_id).unwrap(),
        Some(judgment)
    );
    assert_eq!(
        reopened.product_progress(&progress.progress_id).unwrap(),
        Some(progress)
    );
    assert_eq!(
        reopened.milestone(&milestone.milestone_id).unwrap(),
        Some(milestone)
    );
}

#[test]
fn immutable_reconciliation_identity_rejects_divergent_replay() {
    let temp = tempfile::tempdir().unwrap();
    let store = AgentStore::new(sled::open(temp.path().join("world_model.sled")).unwrap()).unwrap();
    let mut record = AgentReconciliationGoal {
        goal: goal(),
        context_id: "context-v1".to_string(),
        authority_scope_id: "authority-v1".to_string(),
        activation_generation: "activation-v1".to_string(),
        created_at_seq: 7,
    };
    store.put_reconciliation_goal(&record).unwrap();
    record.context_id = "context-divergent".to_string();
    assert!(store.put_reconciliation_goal(&record).is_err());
}
