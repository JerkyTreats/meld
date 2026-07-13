use std::sync::Arc;

use meld_events::DomainObjectRef;
use meld_execution::planning::lowering::{
    Diagnostic as LoweringDiagnostic, DiagnosticCode as LoweringDiagnosticCode,
};
use meld_execution::planning::world_state::canonical_world_state_hash;
use meld_execution::planning::{
    planning_task_network_command_id, CandidateStatus, MethodCandidateReport,
    PlanningAttemptDecisionAudit, PlanningAttemptDiagnosticDisposition, PlanningAttemptErrorClass,
    PlanningAttemptIdentity, PlanningAttemptQuery, PlanningAttemptResultSummary,
    PlanningAttemptState, PlanningAttemptStorageError, PlanningAttemptStore,
    PlanningAttemptTerminalDiagnostic, PlanningDiagnostic, PlanningDiagnosticCode,
    PlanningPerspectiveRef, PlanningProjectionFailureIdentityInputs,
    PlanningProjectionIdentityInputs, PlanningRequestIdentity, PlanningRequestIdentityInputs,
    PlanningWorldStateFrameRef, PlanningWorldStateRequest,
};
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_execution::task_network::{command, mutation::Set, NetworkState, TaskNetworkAuthority};
use meld_lang::{Proposition, Term, WorldState};

fn digest(label: &str) -> String {
    blake3::hash(label.as_bytes()).to_hex().to_string()
}

fn owned_store() -> PlanningAttemptStore {
    let store =
        PlanningAttemptStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
    store
        .bind_owner(None, "execution.planning.runtime", "lease-test")
        .unwrap()
}

fn identity() -> PlanningAttemptIdentity {
    let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
    let request = PlanningWorldStateRequest {
        goal_id: "goal-a".to_string(),
        agent_id: "agent-a".to_string(),
        subject: subject.clone(),
        source_seq: 7,
        target: Proposition::Accessible {
            scope: Term::Object(subject),
        },
        perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: Vec::new(),
    };
    let world_state = WorldState::empty();
    let frame = PlanningWorldStateFrameRef::identified_from_authority(
        "planner.v1",
        digest("projection"),
        canonical_world_state_hash(&world_state).unwrap(),
        &request,
        &world_state,
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let projection_identity =
        PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame).unwrap();
    PlanningAttemptIdentity::bind(
        PlanningRequestIdentity::derive(PlanningRequestIdentityInputs {
            goal_id: "goal-a".to_string(),
            goal_updated_at_seq: 7,
            projection_identity,
            method_library_digest: digest("methods"),
            capability_catalog_digest: digest("capabilities"),
            planning_version: "execution.planning.v1".to_string(),
        })
        .unwrap(),
    )
    .unwrap()
}

fn command(identity: &PlanningAttemptIdentity) -> command::Request {
    let composition_id = "composition-a";
    command::Request {
        command_id: planning_task_network_command_id(identity, composition_id).unwrap(),
        network_id: "network-a".to_string(),
        base_revision: 0,
        base_state_hash: NetworkState::empty("network-a").state_hash,
        read_preconditions: Vec::new(),
        command: command::Command::ApplyMutationSet(Set::empty(
            "network-a",
            composition_id,
            identity.attempt_id(),
        )),
    }
}

fn decision(result_summary: PlanningAttemptResultSummary) -> PlanningAttemptDecisionAudit {
    let selected_method_id = matches!(
        &result_summary,
        PlanningAttemptResultSummary::Composed { .. }
            | PlanningAttemptResultSummary::LoweringFailed { .. }
            | PlanningAttemptResultSummary::NoMutation { .. }
    )
    .then(|| "method-a".to_string());
    let candidate_reports = selected_method_id
        .as_ref()
        .map(|method_id| {
            vec![MethodCandidateReport {
                method_id: method_id.clone(),
                status: CandidateStatus::Applicable,
                bindings: None,
                diagnostics: vec![PlanningDiagnostic::new(
                    PlanningDiagnosticCode::MethodTriggerDerived,
                    "candidate trigger matched",
                )
                .with_method(method_id.clone())],
            }]
        })
        .unwrap_or_default();
    let method_diagnostics = selected_method_id
        .as_ref()
        .map(|method_id| {
            vec![PlanningDiagnostic::new(
                PlanningDiagnosticCode::OperatorResolved,
                "method operators resolved",
            )
            .with_method(method_id.clone())]
        })
        .unwrap_or_default();
    let lowering_diagnostics = selected_method_id
        .as_ref()
        .map(|_| {
            vec![LoweringDiagnostic::new(
                LoweringDiagnosticCode::GoalStepDeferred,
                "recursive goal step remained diagnostic",
            )
            .with_step("step-a")]
        })
        .unwrap_or_default();
    PlanningAttemptDecisionAudit::new(
        selected_method_id,
        candidate_reports,
        vec![
            "projection-warning-b".to_string(),
            "projection-warning-a".to_string(),
            "projection-warning-a".to_string(),
        ],
        method_diagnostics,
        lowering_diagnostics,
        result_summary,
    )
    .unwrap()
}

#[test]
fn public_query_recovers_exact_prepared_command_then_observes_outcome() {
    let store = Arc::new(owned_store());
    let query = PlanningAttemptQuery::new(Arc::clone(&store));
    let identity = identity();
    let request = command(&identity);

    store.open_attempt(identity.clone()).unwrap();
    let decision = decision(PlanningAttemptResultSummary::Composed {
        composition_id: "composition-a".to_string(),
    });
    store
        .record_decision(identity.attempt_id(), decision.clone())
        .unwrap();
    let durable_decision = query
        .decision_audit(identity.attempt_id())
        .unwrap()
        .unwrap();
    assert_eq!(durable_decision, decision);
    assert_eq!(durable_decision.candidate_reports().len(), 1);
    assert_eq!(durable_decision.projection_warnings().len(), 2);
    assert_eq!(durable_decision.method_diagnostics().len(), 1);
    assert_eq!(durable_decision.lowering_diagnostics().len(), 1);
    store
        .prepare_command(identity.attempt_id(), request.clone())
        .unwrap();

    let recovery = query
        .recoverable_commands_bounded(None, 1)
        .unwrap()
        .recoveries;
    assert_eq!(recovery.len(), 1);
    assert_eq!(
        recovery[0].head.state,
        PlanningAttemptState::CommandPrepared
    );
    assert_eq!(recovery[0].prepared.request, request);

    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path().join("networks"));
    let mut authority = TaskNetworkAuthority::open(&factory, "network-a", 8).unwrap();
    let command_port = authority.command_port();
    let authority_query = authority.query_port();
    assert!(authority_query
        .command_outcome(&request.command_id)
        .unwrap()
        .is_none());
    let response = command_port.try_submit(request.clone()).unwrap();
    let accepted_receipt = authority_query
        .command_outcome(&request.command_id)
        .unwrap()
        .unwrap();
    authority.shutdown().unwrap();

    let reopened = TaskNetworkAuthority::open(&factory, "network-a", 8).unwrap();
    let replay = reopened.command_port().try_submit(request.clone()).unwrap();
    assert!(matches!(replay, command::Response::Duplicate { .. }));
    let receipt = reopened
        .query_port()
        .command_outcome(&request.command_id)
        .unwrap()
        .unwrap();
    assert_eq!(receipt, accepted_receipt);
    store
        .record_command_outcome(identity.attempt_id(), receipt)
        .unwrap();

    assert!(query
        .recoverable_commands_bounded(None, 1)
        .unwrap()
        .recoveries
        .is_empty());
    assert_eq!(
        query
            .command_outcome(identity.attempt_id())
            .unwrap()
            .unwrap()
            .response,
        response
    );
}

#[test]
fn public_terminal_diagnostic_and_error_classification_are_typed() {
    let store = Arc::new(owned_store());
    let query = PlanningAttemptQuery::new(Arc::clone(&store));
    let identity = identity();
    store.open_attempt(identity.clone()).unwrap();
    store
        .record_decision(
            identity.attempt_id(),
            decision(PlanningAttemptResultSummary::Indeterminate),
        )
        .unwrap();
    let diagnostic = PlanningAttemptTerminalDiagnostic::new(
        PlanningAttemptDiagnosticDisposition::Indeterminate,
        "projection_indeterminate",
        "projection lacked required facts",
    )
    .unwrap();
    store
        .record_terminal_diagnostic(identity.attempt_id(), diagnostic.clone())
        .unwrap();

    assert_eq!(
        query.terminal_diagnostic(identity.attempt_id()).unwrap(),
        Some(diagnostic)
    );
    assert_eq!(
        PlanningAttemptStorageError::Backpressure("busy".to_string()).class(),
        PlanningAttemptErrorClass::Retryable
    );
    assert_eq!(
        PlanningAttemptStorageError::IdentityConflict("different".to_string()).class(),
        PlanningAttemptErrorClass::Fatal
    );
}

#[test]
fn planning_attempt_authority_rejects_non_planning_task_commands() {
    let store = owned_store();
    let identity = identity();
    store.open_attempt(identity.clone()).unwrap();
    store
        .record_decision(
            identity.attempt_id(),
            decision(PlanningAttemptResultSummary::Composed {
                composition_id: "composition-a".to_string(),
            }),
        )
        .unwrap();
    let mut request = command(&identity);
    request.command = command::Command::MarkPublication(
        meld_execution::task_network::outcome::Publication::pending_for_outcome(
            "network-a",
            &meld_execution::task_network::dispatch::Outcome {
                outcome_id: "outcome-a".to_string(),
                task_instance_id: "task-a".to_string(),
                lifecycle_epoch: 1,
                claim_id: "claim-a".to_string(),
                claim_revision: 1,
                status: meld_execution::task_network::dispatch::OutcomeStatus::Succeeded,
                error: None,
                artifact_records: Vec::new(),
                task_events: Vec::new(),
            },
        ),
    );

    assert!(matches!(
        store.prepare_command(identity.attempt_id(), request),
        Err(PlanningAttemptStorageError::InvalidInput(_))
    ));
}

#[test]
fn planning_attempt_identity_persists_one_canonical_source_unit() {
    let identity = identity();
    let value = serde_json::to_value(&identity).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), 2);
    assert!(object.contains_key("attempt_id"));
    assert!(object.contains_key("source"));
    for duplicate in [
        "goal_id",
        "goal_updated_at_seq",
        "projection_frame_id",
        "method_library_digest",
        "capability_catalog_digest",
    ] {
        assert!(!object.contains_key(duplicate));
    }
    assert_eq!(
        serde_json::from_value::<PlanningAttemptIdentity>(value).unwrap(),
        identity
    );
}

#[test]
fn completed_attempt_identity_rejects_unreachable_or_malformed_selector_inputs() {
    let identity = identity();
    let base = identity.planning_request().unwrap().inputs().clone();

    let mut empty_goal = base.clone();
    empty_goal.goal_id.clear();
    assert!(
        PlanningAttemptIdentity::bind(PlanningRequestIdentity::derive(empty_goal).unwrap())
            .is_err()
    );

    let mut zero_sequence = base.clone();
    zero_sequence.goal_updated_at_seq = 0;
    assert!(
        PlanningAttemptIdentity::bind(PlanningRequestIdentity::derive(zero_sequence).unwrap())
            .is_err()
    );

    let mut malformed_methods = base.clone();
    malformed_methods.method_library_digest = "not-a-digest".to_string();
    assert!(PlanningAttemptIdentity::bind(
        PlanningRequestIdentity::derive(malformed_methods).unwrap()
    )
    .is_err());

    let mut malformed_capabilities = base;
    malformed_capabilities.capability_catalog_digest = "not-a-digest".to_string();
    assert!(PlanningAttemptIdentity::bind(
        PlanningRequestIdentity::derive(malformed_capabilities).unwrap()
    )
    .is_err());
}

#[test]
fn completed_attempt_identity_rejects_cross_goal_projection_reuse_on_bind_and_decode() {
    let identity = identity();
    let mut cross_goal = identity.planning_request().unwrap().inputs().clone();
    cross_goal.goal_id = "goal-b".to_string();
    assert!(
        PlanningAttemptIdentity::bind(PlanningRequestIdentity::derive(cross_goal).unwrap())
            .is_err()
    );

    let mut encoded = serde_json::to_value(identity).unwrap();
    let planning_request = &mut encoded["source"]["ProjectionCompleted"]["planning_request"];
    planning_request["inputs"]["goal_id"] = serde_json::json!("goal-b");
    planning_request["request_id"] = serde_json::json!(blake3::hash(
        &serde_json::to_vec(&planning_request["inputs"]).unwrap()
    )
    .to_hex()
    .to_string());
    assert!(serde_json::from_value::<PlanningAttemptIdentity>(encoded).is_err());
}

#[test]
fn projection_failure_identity_rejects_unbounded_goal_on_bind_and_decode() {
    let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
    let mut request = PlanningWorldStateRequest {
        goal_id: "goal-a".to_string(),
        agent_id: "agent-a".to_string(),
        subject: subject.clone(),
        source_seq: 7,
        target: Proposition::Accessible {
            scope: Term::Object(subject),
        },
        perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: Vec::new(),
    };
    let valid = PlanningProjectionFailureIdentityInputs::bind(
        request.clone(),
        digest("methods"),
        digest("capabilities"),
        "execution.planning.v1".to_string(),
    )
    .unwrap();

    request.goal_id = "g".repeat(257);
    assert!(PlanningProjectionFailureIdentityInputs::bind(
        request,
        digest("methods"),
        digest("capabilities"),
        "execution.planning.v1".to_string(),
    )
    .is_err());

    let mut encoded = serde_json::to_value(valid).unwrap();
    encoded["goal_id"] = serde_json::json!("g".repeat(257));
    assert!(serde_json::from_value::<PlanningProjectionFailureIdentityInputs>(encoded).is_err());
}
