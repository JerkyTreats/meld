use meld_events::DomainObjectRef;
use meld_execution::planning::{
    CandidateStatus, ExecutionComposition, InvalidMethodReport, MethodCandidateReport,
    NoApplicableMethod, OperatorResolutionReport, OperatorResolutionStatus, PlanningDiagnostic,
    PlanningDiagnosticCode, PlanningIndeterminate, PlanningRequest, PlanningResult,
    PlanningSatisfied, PlanningWorldStateFrameRef, PlanningWorldStateRequest,
};
use meld_lang::{
    Condition, Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term,
    ValidationResult, WorldState,
};
use std::fs;
use std::path::Path;

fn node() -> Term {
    Term::Object(DomainObjectRef::new("workspace", "node", "readme").unwrap())
}

fn goal() -> Goal {
    Goal {
        goal_id: "goal-1".to_string(),
        agent_id: "agent".to_string(),
        target: Proposition::Holds {
            subject: node(),
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
    }
}

fn frame() -> PlanningWorldStateFrameRef {
    PlanningWorldStateFrameRef {
        frame_id: "frame-1".to_string(),
        projection_version: "world_model.planner.v1".to_string(),
        perspective_id: "default".to_string(),
        branch_id: "main".to_string(),
        source_refs: vec!["source".to_string()],
        warnings: vec![],
    }
}

fn world_state_request() -> PlanningWorldStateRequest {
    PlanningWorldStateRequest {
        goal_id: "goal-1".to_string(),
        agent_id: "agent".to_string(),
        target: goal().target,
        perspective_id: "default".to_string(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: vec![],
    }
}

#[test]
fn planning_contracts_round_trip() {
    let diagnostic =
        PlanningDiagnostic::new(PlanningDiagnosticCode::GoalSatisfied, "already satisfied");
    let request = PlanningRequest {
        request_id: "request-1".to_string(),
        goal: goal(),
        world_state: WorldState::empty(),
        world_state_frame: frame(),
        world_state_request: world_state_request(),
    };
    let candidate = MethodCandidateReport {
        method_id: "method-1".to_string(),
        status: CandidateStatus::Applicable,
        bindings: Some(meld_lang::Bindings::empty()),
        diagnostics: vec![diagnostic.clone()],
    };
    let resolution = OperatorResolutionReport {
        operator_id: "write".to_string(),
        status: OperatorResolutionStatus::Resolved,
        capability_type_id: Some("docs.write".to_string()),
        capability_version: Some(1),
        tags: vec!["docs".to_string()],
        diagnostics: vec![diagnostic.clone()],
    };
    let composed = PlanningResult::Composed(ExecutionComposition {
        composition_id: "execution-composition-id".to_string(),
        goal: goal(),
        world_state_frame: frame(),
        method_id: "method-1".to_string(),
        bindings: meld_lang::Bindings::empty(),
        composition: meld_lang::Composition {
            steps: vec![],
            edges: vec![],
        },
        projected_effects: vec![],
        operator_resolutions: vec![resolution],
        validation: ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        },
        diagnostics: vec![diagnostic.clone()],
    });
    let variants = vec![
        PlanningResult::Satisfied(PlanningSatisfied {
            goal: goal(),
            world_state_frame: frame(),
            diagnostics: vec![diagnostic.clone()],
        }),
        composed,
        PlanningResult::NoApplicableMethod(NoApplicableMethod {
            goal: goal(),
            world_state_frame: frame(),
            candidates: vec![candidate],
            diagnostics: vec![diagnostic.clone()],
        }),
        PlanningResult::Indeterminate(PlanningIndeterminate {
            goal: goal(),
            world_state_frame: frame(),
            missing: vec![Term::Dimension("docs_freshness".to_string())],
            diagnostics: vec![diagnostic.clone()],
        }),
        PlanningResult::InvalidMethod(InvalidMethodReport {
            source_ref: Some("fixture".to_string()),
            method_id: Some("method-1".to_string()),
            diagnostics: vec![diagnostic],
        }),
    ];

    let decoded_request: PlanningRequest =
        serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
    assert_eq!(decoded_request, request);
    for variant in variants {
        let decoded: PlanningResult =
            serde_json::from_str(&serde_json::to_string(&variant).unwrap()).unwrap();
        assert_eq!(decoded, variant);
    }
}

#[test]
fn composed_result_contract_excludes_runtime_state() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/planning/contracts.rs")).unwrap();
    for forbidden in [
        "TaskDefinition",
        "task_instance_id",
        "ready_set",
        "in_flight",
        "TaskArtifactRepo",
        "CapabilityInvocationPayload",
    ] {
        assert!(!source.contains(forbidden), "found {forbidden}");
    }
}

#[test]
fn planning_module_boundary_scans_pass() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!manifest_dir.join("src/goals/mod.rs").exists());
    assert!(!manifest_dir.join("src/planning/mod.rs").exists());

    let mut planning_sources = String::new();
    for path in [
        "src/planning.rs",
        "src/planning/contracts.rs",
        "src/planning/method_library.rs",
        "src/planning/runtime.rs",
        "src/planning/world_state.rs",
    ] {
        planning_sources.push_str(&fs::read_to_string(manifest_dir.join(path)).unwrap());
    }
    for forbidden in [
        "crate::task::runtime",
        "execute_task_to_completion",
        "CapabilityInvocationPayload",
        "TaskExecutor",
    ] {
        assert!(!planning_sources.contains(forbidden), "found {forbidden}");
    }
}
