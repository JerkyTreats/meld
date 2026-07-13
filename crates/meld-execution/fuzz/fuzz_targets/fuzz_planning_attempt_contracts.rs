#![no_main]

use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use meld_events::DomainObjectRef;
use meld_execution::planning::lowering::{
    Diagnostic as LoweringDiagnostic, DiagnosticCode as LoweringDiagnosticCode,
};
use meld_execution::planning::world_state::canonical_world_state_hash;
use meld_execution::planning::{
    planning_task_network_command_id, CandidateStatus, MethodCandidateReport,
    PlanningAttemptCommandOutcome, PlanningAttemptContinuation, PlanningAttemptDecisionAudit,
    PlanningAttemptDiagnosticDisposition, PlanningAttemptErrorClass, PlanningAttemptHead,
    PlanningAttemptIdentity, PlanningAttemptQuery, PlanningAttemptRecord,
    PlanningAttemptRecordKind, PlanningAttemptResultSummary, PlanningAttemptState,
    PlanningAttemptStorageError, PlanningAttemptStore, PlanningAttemptTerminalDiagnostic,
    PlanningDiagnostic, PlanningDiagnosticCode, PlanningPerspectiveRef, PlanningPreparedCommand,
    PlanningProjectionFailureIdentityInputs, PlanningProjectionIdentityInputs,
    PlanningRequestIdentity, PlanningRequestIdentityInputs, PlanningWorldStateFrameRef,
    PlanningWorldStateRequest,
};
use meld_execution::task_network::command::{self, OutcomeReceipt, ResponseAuthentication};
use meld_execution::task_network::mutation::{ReadPrecondition, Rejection, Set};
use meld_execution::task_network::store::{SledTaskNetworkStore, TaskNetworkStoreError};
use meld_execution::task_network::{NetworkState, TaskNetworkAuthority};
use meld_lang::{Bindings, Condition, Literal, Proposition, Term, WorldState};
use serde::Serialize;
use serde_json::{json, Value};

const RECORD_HASH_DOMAIN: &[u8] = b"meld.execution.planning-attempt-record.v1";
const RESPONSE_HASH_DOMAIN: &[u8] = b"meld.execution.planning-command-response.v1";
const CONTINUATION_HASH_DOMAIN: &[u8] = b"meld.execution.planning-continuation.v1";
const GOAL_INDEX_HASH_DOMAIN: &[u8] = b"meld.execution.planning-attempt-goal-index.v1";

const TREE_ATTEMPT_RECORDS: &str = "execution_planning_attempt_records";
const TREE_ATTEMPT_HEADS: &str = "execution_planning_attempt_heads";
const TREE_ATTEMPT_GOALS: &str = "execution_planning_attempt_goal_index";
const TREE_ATTEMPT_RECOVERY: &str = "execution_planning_attempt_recovery_index";
const TREE_COMMAND_REQUESTS: &str = "task_network_command_requests";
const TREE_COMMAND_RESPONSES: &str = "task_network_command_responses";
const KEY_SCHEMA: &[u8] = b"__schema";
const MAX_OPERATIONS: usize = 12;

struct Scenario {
    store: Arc<PlanningAttemptStore>,
    query: PlanningAttemptQuery,
    goal_id: String,
    primary_identity: PlanningAttemptIdentity,
    secondary_identity: PlanningAttemptIdentity,
    failure_identity: PlanningAttemptIdentity,
    audit: PlanningAttemptDecisionAudit,
    primary_prepared: PlanningPreparedCommand,
    primary_outcome: PlanningAttemptCommandOutcome,
    primary_receipt: OutcomeReceipt,
    secondary_receipt: OutcomeReceipt,
    records: Vec<PlanningAttemptRecord>,
    recovery_continuation: PlanningAttemptContinuation,
    goal_continuation: PlanningAttemptContinuation,
}

fn input_byte(data: &[u8], index: usize) -> u8 {
    data.get(index)
        .copied()
        .unwrap_or_else(|| index.wrapping_mul(73).wrapping_add(19) as u8)
}

fn digest(label: &str) -> String {
    blake3::hash(label.as_bytes()).to_hex().to_string()
}

fn owned_attempt_store(db: sled::Db) -> PlanningAttemptStore {
    let store = PlanningAttemptStore::new(db).unwrap();
    let current = store.active_owner_fence().unwrap();
    store
        .bind_owner(
            current.as_ref(),
            "execution.planning.runtime",
            "lease-fuzz",
        )
        .unwrap()
}

fn replacement_digest(data: &[u8], label: &str, salt: u8) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(label.as_bytes());
    hasher.update(&[salt]);
    hasher.update(data);
    hasher.finalize().to_hex().to_string()
}

fn hash_with_domain(domain: &[u8], value: &impl Serialize) -> String {
    let encoded = serde_json::to_vec(value).unwrap();
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);
    hasher.finalize().to_hex().to_string()
}

fn projection_request(goal_id: &str, sequence: u64) -> PlanningWorldStateRequest {
    let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
    PlanningWorldStateRequest {
        goal_id: goal_id.to_string(),
        agent_id: "agent-a".to_string(),
        subject: subject.clone(),
        source_seq: sequence,
        target: Proposition::Accessible {
            scope: Term::Object(subject),
        },
        perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: Vec::new(),
        required_preconditions: Vec::new(),
    }
}

fn completed_identity(
    goal_id: &str,
    sequence: u64,
    frame_id: &str,
    salt: u8,
) -> PlanningAttemptIdentity {
    let request = projection_request(goal_id, sequence);
    let world_state = WorldState::empty();
    let frame = PlanningWorldStateFrameRef::identified_from_authority(
        format!("planner.v{}.{frame_id}", u16::from(salt) + 1),
        digest(&format!("projection-{salt}-{frame_id}")),
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
            goal_id: goal_id.to_string(),
            goal_updated_at_seq: sequence,
            projection_identity,
            method_library_digest: digest(&format!("methods-{salt}")),
            capability_catalog_digest: digest(&format!("capabilities-{salt}")),
            planning_version: format!("execution.planning.v{}", u16::from(salt) + 1),
        })
        .unwrap(),
    )
    .unwrap()
}

fn failure_identity(goal_id: &str, sequence: u64, salt: u8) -> PlanningAttemptIdentity {
    let inputs = PlanningProjectionFailureIdentityInputs::bind(
        projection_request(goal_id, sequence),
        digest(&format!("failure-methods-{salt}")),
        digest(&format!("failure-capabilities-{salt}")),
        format!("execution.planning.failure.v{}", u16::from(salt) + 1),
    )
    .unwrap();
    PlanningAttemptIdentity::bind_projection_failure(inputs).unwrap()
}

fn request(identity: &PlanningAttemptIdentity) -> command::Request {
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

fn composed_audit(salt: u8) -> PlanningAttemptDecisionAudit {
    PlanningAttemptDecisionAudit::new(
        Some("method-a".to_string()),
        Vec::new(),
        vec![format!("projection-warning-{salt:03}")],
        Vec::new(),
        Vec::new(),
        PlanningAttemptResultSummary::Composed {
            composition_id: "composition-a".to_string(),
        },
    )
    .unwrap()
}

fn build_receipts(
    first: &command::Request,
    second: &command::Request,
) -> (OutcomeReceipt, OutcomeReceipt) {
    let temp = tempfile::tempdir().unwrap();
    let factory = meld_execution::task_network::store::TaskNetworkStoreFactory::new(temp.path());
    let mut authority = TaskNetworkAuthority::open(&factory, "network-a", 8).unwrap();
    authority.command_port().try_submit(first.clone()).unwrap();
    authority.command_port().try_submit(second.clone()).unwrap();
    let first_receipt = authority
        .query_port()
        .command_outcome(&first.command_id)
        .unwrap()
        .unwrap();
    let second_receipt = authority
        .query_port()
        .command_outcome(&second.command_id)
        .unwrap()
        .unwrap();
    authority.shutdown().unwrap();
    (first_receipt, second_receipt)
}

fn build_scenario(data: &[u8]) -> Scenario {
    let seed = input_byte(data, 0);
    let sequence = u64::from(input_byte(data, 1)) + 1;
    let goal_id = format!("goal-{seed:03}");
    let primary_identity = completed_identity(
        &goal_id,
        sequence,
        &format!("frame-primary-{seed:03}"),
        seed,
    );
    let secondary_sequence = sequence.saturating_add(1);
    let secondary_salt = seed.wrapping_add(1);
    let secondary_identity = completed_identity(
        &goal_id,
        secondary_sequence,
        &format!("frame-secondary-{seed:03}"),
        secondary_salt,
    );
    let failure_identity = failure_identity(
        &format!("failure-goal-{seed:03}"),
        secondary_sequence.saturating_add(1),
        seed.wrapping_add(2),
    );
    let primary_request = request(&primary_identity);
    let secondary_request = request(&secondary_identity);
    let audit = composed_audit(input_byte(data, 2));
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = owned_attempt_store(db.clone());

    for (identity, request) in [
        (&primary_identity, primary_request.clone()),
        (&secondary_identity, secondary_request.clone()),
    ] {
        store.open_attempt(identity.clone()).unwrap();
        store
            .record_decision(identity.attempt_id(), audit.clone())
            .unwrap();
        store
            .prepare_command(identity.attempt_id(), request)
            .unwrap();
    }

    let query = PlanningAttemptQuery::new(Arc::new(store.clone()));
    let recovery_continuation = query
        .recoverable_commands_bounded(None, 1)
        .unwrap()
        .continuation
        .unwrap();
    let goal_continuation = query
        .attempts_for_goal_bounded(&goal_id, None, 1)
        .unwrap()
        .continuation
        .unwrap();
    let primary_prepared = query
        .prepared_command(primary_identity.attempt_id())
        .unwrap()
        .unwrap();
    let (primary_receipt, secondary_receipt) = build_receipts(&primary_request, &secondary_request);
    store
        .record_command_outcome(primary_identity.attempt_id(), primary_receipt.clone())
        .unwrap();

    let failure_audit = PlanningAttemptDecisionAudit::new(
        None,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        PlanningAttemptResultSummary::ProjectionFailed,
    )
    .unwrap();
    let failure_diagnostic = PlanningAttemptTerminalDiagnostic::new(
        PlanningAttemptDiagnosticDisposition::ProjectionFailed,
        "projection_failed",
        "projection authority returned a terminal failure",
    )
    .unwrap();
    store.open_attempt(failure_identity.clone()).unwrap();
    store
        .record_decision(failure_identity.attempt_id(), failure_audit.clone())
        .unwrap();
    store
        .record_terminal_diagnostic(failure_identity.attempt_id(), failure_diagnostic.clone())
        .unwrap();

    assert_eq!(
        store.open_attempt(primary_identity.clone()).unwrap().state,
        PlanningAttemptState::CommandCompleted
    );
    assert_eq!(
        store
            .record_decision(primary_identity.attempt_id(), audit.clone())
            .unwrap()
            .state,
        PlanningAttemptState::CommandCompleted
    );
    assert_eq!(
        store
            .prepare_command(primary_identity.attempt_id(), primary_request)
            .unwrap()
            .state,
        PlanningAttemptState::CommandCompleted
    );
    assert_eq!(
        store
            .record_command_outcome(primary_identity.attempt_id(), primary_receipt.clone())
            .unwrap()
            .state,
        PlanningAttemptState::CommandCompleted
    );
    assert_eq!(
        store
            .record_terminal_diagnostic(failure_identity.attempt_id(), failure_diagnostic)
            .unwrap()
            .state,
        PlanningAttemptState::DiagnosticTerminal
    );

    drop(query);
    drop(store);
    let store = Arc::new(owned_attempt_store(db));
    let query = PlanningAttemptQuery::new(Arc::clone(&store));
    let records = query
        .history_bounded(primary_identity.attempt_id(), None, 4)
        .unwrap()
        .records;
    assert_eq!(records.len(), 4);
    assert_eq!(
        query
            .attempt(primary_identity.attempt_id())
            .unwrap()
            .unwrap()
            .state,
        PlanningAttemptState::CommandCompleted
    );
    assert_eq!(
        query
            .attempt(secondary_identity.attempt_id())
            .unwrap()
            .unwrap()
            .state,
        PlanningAttemptState::CommandPrepared
    );
    assert_eq!(
        query
            .attempt(failure_identity.attempt_id())
            .unwrap()
            .unwrap()
            .state,
        PlanningAttemptState::DiagnosticTerminal
    );
    let primary_outcome = query
        .command_outcome(primary_identity.attempt_id())
        .unwrap()
        .unwrap();

    Scenario {
        store,
        query,
        goal_id,
        primary_identity,
        secondary_identity,
        failure_identity,
        audit,
        primary_prepared,
        primary_outcome,
        primary_receipt,
        secondary_receipt,
        records,
        recovery_continuation,
        goal_continuation,
    }
}

fn recompute_planning_request_id(identity: &mut Value) {
    let inputs: PlanningRequestIdentityInputs = serde_json::from_value(
        identity
            .pointer("/source/ProjectionCompleted/planning_request/inputs")
            .unwrap()
            .clone(),
    )
    .unwrap();
    identity["source"]["ProjectionCompleted"]["planning_request"] =
        serde_json::to_value(PlanningRequestIdentity::derive(inputs).unwrap()).unwrap();
}

fn rebound_attempt_identity(identity: &Value) -> PlanningAttemptIdentity {
    if let Some(planning_request) = identity.pointer("/source/ProjectionCompleted/planning_request")
    {
        let planning_request: PlanningRequestIdentity =
            serde_json::from_value(planning_request.clone()).unwrap();
        PlanningAttemptIdentity::bind(planning_request).unwrap()
    } else {
        let inputs: PlanningProjectionFailureIdentityInputs = serde_json::from_value(
            identity
                .pointer("/source/ProjectionFailed/inputs")
                .unwrap()
                .clone(),
        )
        .unwrap();
        PlanningAttemptIdentity::bind_projection_failure(inputs).unwrap()
    }
}

fn rebound_identity(scenario: &Scenario, data: &[u8], selector: u8) -> PlanningAttemptIdentity {
    let failure_source = selector & 0x80 != 0;
    let original = if failure_source {
        &scenario.failure_identity
    } else {
        &scenario.primary_identity
    };
    let mut identity = serde_json::to_value(original).unwrap();
    let field = selector % 10;
    let salt = selector.wrapping_add(input_byte(data, usize::from(field)));
    if failure_source {
        let path = match field % 6 {
            0 => "/source/ProjectionFailed/inputs/goal_id",
            1 => "/source/ProjectionFailed/inputs/goal_updated_at_seq",
            2 => "/source/ProjectionFailed/inputs/projection_request_hash",
            3 => "/source/ProjectionFailed/inputs/method_library_digest",
            4 => "/source/ProjectionFailed/inputs/capability_catalog_digest",
            _ => "/source/ProjectionFailed/inputs/planning_version",
        };
        *identity.pointer_mut(path).unwrap() = if field % 6 == 1 {
            json!(u64::from(salt) + 1000)
        } else if matches!(field % 6, 2..=4) {
            json!(replacement_digest(data, "failure-identity", salt))
        } else {
            json!(format!("failure-mutated-{field}-{salt}"))
        };
    } else {
        let path = match field {
            0 => "/source/ProjectionCompleted/planning_request/inputs/goal_id",
            1 => "/source/ProjectionCompleted/planning_request/inputs/goal_updated_at_seq",
            2 => {
                "/source/ProjectionCompleted/planning_request/inputs/projection_identity/frame_id"
            }
            3 => "/source/ProjectionCompleted/planning_request/inputs/method_library_digest",
            4 => "/source/ProjectionCompleted/planning_request/inputs/capability_catalog_digest",
            5 => "/source/ProjectionCompleted/planning_request/inputs/planning_version",
            6 => {
                "/source/ProjectionCompleted/planning_request/inputs/projection_identity/projection_hash"
            }
            7 => {
                "/source/ProjectionCompleted/planning_request/inputs/projection_identity/source_request_hash"
            }
            8 => {
                "/source/ProjectionCompleted/planning_request/inputs/projection_identity/world_state_hash"
            }
            _ => {
                "/source/ProjectionCompleted/planning_request/inputs/projection_identity/perspective_id"
            }
        };
        *identity.pointer_mut(path).unwrap() = if field == 1 {
            json!(u64::from(salt) + 1000)
        } else if matches!(field, 3 | 4 | 6 | 7 | 8) {
            json!(replacement_digest(data, "completed-identity", salt))
        } else {
            json!(format!("completed-mutated-{field}-{salt}"))
        };
        if field == 0 {
            identity["source"]["ProjectionCompleted"]["planning_request"]["inputs"]
                ["projection_identity"]["goal_id"] = identity["source"]["ProjectionCompleted"]
                ["planning_request"]["inputs"]["goal_id"]
                .clone();
        } else if field == 1 {
            identity["source"]["ProjectionCompleted"]["planning_request"]["inputs"]
                ["projection_identity"]["goal_updated_at_seq"] = identity["source"]
                ["ProjectionCompleted"]["planning_request"]["inputs"]["goal_updated_at_seq"]
                .clone();
        }
    }

    assert!(serde_json::from_value::<PlanningAttemptIdentity>(identity.clone()).is_err());
    if !failure_source && matches!(field, 0 | 1 | 2 | 6 | 7 | 8 | 9) {
        let planning_inputs = identity
            .pointer("/source/ProjectionCompleted/planning_request/inputs")
            .unwrap()
            .clone();
        assert!(serde_json::from_value::<PlanningRequestIdentityInputs>(planning_inputs).is_err());
        let replacement = completed_identity(
            &format!("authority-rebound-{selector:03}"),
            u64::from(salt) + 1000,
            &format!("authority-frame-rebound-{selector:03}"),
            salt,
        );
        assert_ne!(replacement.attempt_id(), original.attempt_id());
        return replacement;
    }
    if !failure_source {
        recompute_planning_request_id(&mut identity);
    }
    assert!(serde_json::from_value::<PlanningAttemptIdentity>(identity.clone()).is_err());
    let rebound = rebound_attempt_identity(&identity);
    rebound.validate().unwrap();
    assert_ne!(rebound.attempt_id(), original.attempt_id());
    rebound
}

fn recompute_audit_digest(audit: &mut Value) {
    let rebuilt = PlanningAttemptDecisionAudit::new(
        serde_json::from_value(audit["selected_method_id"].clone()).unwrap(),
        serde_json::from_value(audit["candidate_reports"].clone()).unwrap(),
        serde_json::from_value(audit["projection_warnings"].clone()).unwrap(),
        serde_json::from_value(audit["method_diagnostics"].clone()).unwrap(),
        serde_json::from_value(audit["lowering_diagnostics"].clone()).unwrap(),
        serde_json::from_value(audit["result_summary"].clone()).unwrap(),
    )
    .unwrap();
    *audit = serde_json::to_value(rebuilt).unwrap();
}

fn divergent_audit(scenario: &Scenario, data: &[u8], selector: u8) -> PlanningAttemptDecisionAudit {
    let mut audit = serde_json::to_value(&scenario.audit).unwrap();
    let salt = selector.wrapping_add(input_byte(data, usize::from(selector % 4)));
    match selector % 4 {
        0 => {
            audit["projection_warnings"] = json!([
                format!("projection-warning-{salt:03}"),
                format!("projection-warning-mutated-{salt:03}")
            ]);
        }
        1 => audit["selected_method_id"] = json!(format!("method-mutated-{salt:03}")),
        2 => {
            audit["result_summary"] = json!({
                "Composed": { "composition_id": format!("composition-mutated-{salt:03}") }
            });
        }
        _ => {
            audit["projection_warnings"] = json!([format!("audit-payload-{salt:03}")]);
            audit["selected_method_id"] = json!(format!("method-payload-{salt:03}"));
        }
    }
    assert!(serde_json::from_value::<PlanningAttemptDecisionAudit>(audit.clone()).is_err());
    recompute_audit_digest(&mut audit);
    let audit: PlanningAttemptDecisionAudit = serde_json::from_value(audit).unwrap();
    audit.validate().unwrap();
    assert_ne!(&audit, &scenario.audit);
    audit
}

fn recompute_outcome_response_hash(outcome: &mut Value) {
    let response: command::Response = serde_json::from_value(outcome["response"].clone()).unwrap();
    outcome["response_hash"] = json!(hash_with_domain(RESPONSE_HASH_DOMAIN, &response));
}

fn divergent_valid_outcome(
    scenario: &Scenario,
    data: &[u8],
    salt: u8,
) -> PlanningAttemptCommandOutcome {
    let mut outcome = serde_json::to_value(&scenario.primary_outcome).unwrap();
    outcome["response"] = json!({
        "Accepted": {
            "revision": scenario.primary_prepared.request.base_revision + 1,
            "state_hash": replacement_digest(data, "divergent-outcome", salt)
        }
    });
    recompute_outcome_response_hash(&mut outcome);
    let outcome: PlanningAttemptCommandOutcome = serde_json::from_value(outcome).unwrap();
    outcome.validate(&scenario.primary_prepared).unwrap();
    assert_ne!(&outcome, &scenario.primary_outcome);
    outcome
}

fn recompute_record_hash(record: &mut Value) {
    #[derive(Serialize)]
    struct RecordHashInput<'a> {
        schema_version: u32,
        attempt_id: &'a str,
        ordinal: u32,
        previous_record_hash: &'a Option<String>,
        kind: &'a PlanningAttemptRecordKind,
    }

    let schema_version = serde_json::from_value(record["schema_version"].clone()).unwrap();
    let attempt_id = record["attempt_id"].as_str().unwrap();
    let ordinal = serde_json::from_value(record["ordinal"].clone()).unwrap();
    let previous_record_hash =
        serde_json::from_value(record["previous_record_hash"].clone()).unwrap();
    let kind = serde_json::from_value(record["kind"].clone()).unwrap();

    record["record_hash"] = json!(hash_with_domain(
        RECORD_HASH_DOMAIN,
        &RecordHashInput {
            schema_version,
            attempt_id,
            ordinal,
            previous_record_hash: &previous_record_hash,
            kind: &kind,
        },
    ));
}

fn assert_fatal_attempt_error(error: &PlanningAttemptStorageError) {
    assert_eq!(error.class(), PlanningAttemptErrorClass::Fatal);
    assert!(!error.is_retryable());
}

fn exercise_identity_boundary(scenario: &Scenario, data: &[u8], selector: u8) {
    let rebound = rebound_identity(scenario, data, selector);
    assert!(scenario
        .query
        .attempt(rebound.attempt_id())
        .unwrap()
        .is_none());
}

fn exercise_audit_boundary(scenario: &Scenario, data: &[u8], selector: u8) {
    let audit = divergent_audit(scenario, data, selector);
    let error = scenario
        .store
        .record_decision(scenario.primary_identity.attempt_id(), audit)
        .unwrap_err();
    assert!(matches!(
        error,
        PlanningAttemptStorageError::IdentityConflict(_)
    ));
    assert_fatal_attempt_error(&error);
}

fn exercise_record_boundary(scenario: &Scenario, data: &[u8], selector: u8) {
    let mutation = selector % 8;
    let mut record = serde_json::to_value(&scenario.records[usize::from(mutation.min(3))]).unwrap();
    match mutation {
        0 => {
            let rebound = rebound_identity(scenario, data, selector);
            record["attempt_id"] = json!(rebound.attempt_id());
            record["kind"]["Opened"]["identity"] = serde_json::to_value(rebound).unwrap();
        }
        1 => {
            record["kind"]["DecisionAudited"]["decision"] =
                serde_json::to_value(divergent_audit(scenario, data, selector)).unwrap();
        }
        2 => {
            let mut prepared = scenario.primary_prepared.clone();
            prepared.request.base_revision = prepared.request.base_revision.saturating_add(1);
            prepared.request_hash = command::request_hash(&prepared.request);
            record["kind"]["CommandPrepared"]["prepared"] = serde_json::to_value(prepared).unwrap();
        }
        3 => {
            record["kind"]["CommandOutcome"]["outcome"] =
                serde_json::to_value(divergent_valid_outcome(scenario, data, selector)).unwrap();
        }
        4 => {
            record = serde_json::to_value(&scenario.records[2]).unwrap();
            record["previous_record_hash"] =
                json!(replacement_digest(data, "record-chain", selector));
        }
        5 => {
            record = serde_json::to_value(&scenario.records[2]).unwrap();
            record["ordinal"] = json!(9_u32);
        }
        6 => {
            record = serde_json::to_value(&scenario.records[2]).unwrap();
            record["attempt_id"] = json!(format!("other-attempt-{selector:03}"));
        }
        _ => {
            record = serde_json::to_value(&scenario.records[2]).unwrap();
            record["record_hash"] = json!(replacement_digest(data, "raw-record-hash", selector));
        }
    }
    if mutation != 7 {
        recompute_record_hash(&mut record);
    }
    let record: PlanningAttemptRecord = serde_json::from_value(record).unwrap();
    if mutation == 7 {
        assert!(record.validate_shape().is_err());
    } else {
        record.validate_shape().unwrap();
        assert!(!scenario.records.contains(&record));
    }
}

fn exercise_prepared_boundary(scenario: &Scenario, data: &[u8], selector: u8) {
    let mutation = selector % 8;
    let mut prepared = scenario.primary_prepared.clone();
    let salt = selector.wrapping_add(input_byte(data, usize::from(mutation)));
    match mutation {
        0 => {
            prepared.request.base_state_hash =
                replacement_digest(data, "prepared-base-state", salt);
        }
        1 => prepared.request.base_revision = prepared.request.base_revision.saturating_add(1),
        2 => {
            let network = format!("network-mutated-{salt:03}");
            prepared.request.network_id.clone_from(&network);
            if let command::Command::ApplyMutationSet(set) = &mut prepared.request.command {
                set.network_id = network;
            }
        }
        3 => {
            let composition = format!("composition-mutated-{salt:03}");
            prepared.request.command_id =
                planning_task_network_command_id(&scenario.primary_identity, &composition).unwrap();
            prepared.request.command = command::Command::ApplyMutationSet(Set::empty(
                prepared.request.network_id.clone(),
                composition,
                scenario.primary_identity.attempt_id(),
            ));
        }
        4 => {
            prepared.request.command = command::Command::ApplyMutationSet(Set::empty(
                prepared.request.network_id.clone(),
                "composition-a",
                format!("idempotency-mutated-{salt:03}"),
            ));
        }
        5 => prepared.request.command_id = format!("command-mutated-{salt:03}"),
        6 => prepared.request.network_id = format!("network-unpaired-{salt:03}"),
        _ => prepared.request_hash = replacement_digest(data, "prepared-request-hash", salt),
    }
    if mutation != 7 {
        prepared.request_hash = command::request_hash(&prepared.request);
    }
    let locally_valid = prepared.validate(&scenario.primary_identity).is_ok();
    assert_eq!(locally_valid, mutation <= 4);
    if mutation == 7 {
        return;
    }
    let error = scenario
        .store
        .prepare_command(scenario.primary_identity.attempt_id(), prepared.request)
        .unwrap_err();
    assert!(matches!(
        error,
        PlanningAttemptStorageError::InvalidInput(_)
            | PlanningAttemptStorageError::IdentityConflict(_)
    ));
    assert_fatal_attempt_error(&error);
}

fn exercise_outcome_boundary(scenario: &Scenario, data: &[u8], selector: u8) {
    let mutation = selector % 8;
    let mut outcome = serde_json::to_value(&scenario.primary_outcome).unwrap();
    match mutation {
        0 => {
            outcome["response"]["Accepted"]["state_hash"] =
                json!(replacement_digest(data, "outcome-state", selector));
        }
        1 => {
            outcome["response"]["Accepted"]["revision"] =
                json!(scenario.primary_prepared.request.base_revision + 2);
        }
        2 => {
            outcome["response"] = json!({
                "Rejected": {
                    "StaleBase": {
                        "expected": scenario.primary_prepared.request.base_revision,
                        "actual": scenario.primary_prepared.request.base_revision + 1
                    }
                }
            });
        }
        3 => {
            outcome["response"] = json!({
                "Rejected": {
                    "StaleBase": {
                        "expected": scenario.primary_prepared.request.base_revision + 1,
                        "actual": scenario.primary_prepared.request.base_revision + 2
                    }
                }
            });
        }
        4 => {
            outcome["response"] = json!({
                "Duplicate": {
                    "revision": scenario.primary_prepared.request.base_revision + 1,
                    "state_hash": replacement_digest(data, "duplicate-state", selector)
                }
            });
        }
        5 => outcome["command_id"] = json!(format!("other-command-{selector:03}")),
        6 => {
            outcome["request_hash"] = json!(replacement_digest(data, "other-request", selector));
        }
        _ => {
            outcome["response_hash"] = json!(replacement_digest(data, "other-response", selector));
        }
    }
    if mutation != 7 {
        recompute_outcome_response_hash(&mut outcome);
    }
    let outcome: PlanningAttemptCommandOutcome = serde_json::from_value(outcome).unwrap();
    let locally_valid = outcome.validate(&scenario.primary_prepared).is_ok();
    assert_eq!(locally_valid, matches!(mutation, 0 | 2));
    if locally_valid {
        assert_ne!(outcome, scenario.primary_outcome);
    }

    let error = scenario
        .store
        .record_command_outcome(
            scenario.primary_identity.attempt_id(),
            scenario.secondary_receipt.clone(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        PlanningAttemptStorageError::InvalidInput(_)
    ));
    assert_fatal_attempt_error(&error);
}

fn exercise_authentication_boundary(scenario: &Scenario, data: &[u8], selector: u8) {
    let db = sled::Config::new().temporary(true).open().unwrap();
    {
        let mut store = SledTaskNetworkStore::open(db.clone(), "network-a").unwrap();
        assert!(matches!(
            store
                .submit(scenario.primary_prepared.request.clone())
                .unwrap(),
            command::Response::Accepted { .. }
        ));
    }
    let requests = db.open_tree(TREE_COMMAND_REQUESTS).unwrap();
    let responses = db.open_tree(TREE_COMMAND_RESPONSES).unwrap();
    let key = scenario.primary_prepared.request.command_id.as_bytes();
    let raw = responses.get(key).unwrap().unwrap();
    let mut stored: Value = serde_json::from_slice(&raw).unwrap();
    let mutation = selector % 10;
    match mutation {
        0 => stored["command_id"] = json!(format!("auth-command-{selector:03}")),
        1 => {
            stored["request_hash"] = json!(replacement_digest(data, "auth-request", selector));
        }
        2 => {
            stored["response"] = json!({
                "Accepted": {
                    "revision": 1,
                    "state_hash": replacement_digest(data, "auth-response", selector)
                }
            });
        }
        3 => {
            stored["response"] = json!({
                "Accepted": {
                    "revision": 2,
                    "state_hash": replacement_digest(data, "auth-revision", selector)
                }
            });
        }
        4 => {
            stored["authentication"]["response_digest"] =
                json!(replacement_digest(data, "auth-digest", selector));
        }
        5 => stored["authentication"] = Value::Null,
        6 => stored["authentication"]["schema_version"] = json!(99_u32),
        7 => {
            let new_key = format!("moved-command-{selector:03}");
            stored["command_id"] = json!(new_key.clone());
            let authentication = ResponseAuthentication::bind(
                &new_key,
                stored["request_hash"].as_str().unwrap(),
                &serde_json::from_value(stored["response"].clone()).unwrap(),
            )
            .unwrap();
            stored["authentication"] = serde_json::to_value(authentication).unwrap();
            responses.remove(key).unwrap();
            responses
                .insert(new_key.as_bytes(), serde_json::to_vec(&stored).unwrap())
                .unwrap();
        }
        8 => {
            let mut request_record: Value =
                serde_json::from_slice(&requests.get(key).unwrap().unwrap()).unwrap();
            request_record["request"]["base_state_hash"] =
                json!(replacement_digest(data, "stored-request-state", selector));
            let changed_request: command::Request =
                serde_json::from_value(request_record["request"].clone()).unwrap();
            let request_hash = command::request_hash(&changed_request);
            request_record["request_hash"] = json!(request_hash.clone());
            stored["request_hash"] = json!(request_hash);
            requests
                .insert(key, serde_json::to_vec(&request_record).unwrap())
                .unwrap();
        }
        _ => {
            let response: command::Response =
                serde_json::from_value(stored["response"].clone()).unwrap();
            let authentication = ResponseAuthentication::bind(
                stored["command_id"].as_str().unwrap(),
                stored["request_hash"].as_str().unwrap(),
                &response,
            )
            .unwrap();
            stored["authentication"] = serde_json::to_value(authentication).unwrap();
            stored["response"]["Accepted"]["state_hash"] =
                json!(replacement_digest(data, "post-auth-response", selector));
        }
    }
    if mutation <= 3 || mutation == 8 {
        let response: command::Response =
            serde_json::from_value(stored["response"].clone()).unwrap();
        let authentication = ResponseAuthentication::bind(
            stored["command_id"].as_str().unwrap(),
            stored["request_hash"].as_str().unwrap(),
            &response,
        )
        .unwrap();
        authentication
            .validate(
                stored["command_id"].as_str().unwrap(),
                stored["request_hash"].as_str().unwrap(),
                &response,
            )
            .unwrap();
        stored["authentication"] = serde_json::to_value(authentication).unwrap();
    }
    if mutation != 7 {
        responses
            .insert(key, serde_json::to_vec(&stored).unwrap())
            .unwrap();
    }
    db.flush().unwrap();
    assert!(matches!(
        SledTaskNetworkStore::open(db, "network-a"),
        Err(TaskNetworkStoreError::Decode(_))
    ));
}

fn recompute_continuation_hash(continuation: &mut Value) {
    #[derive(Serialize)]
    struct ContinuationHashInput<'a> {
        scope: &'a Value,
        after_key: &'a Value,
    }

    continuation["continuation_hash"] = json!(hash_with_domain(
        CONTINUATION_HASH_DOMAIN,
        &ContinuationHashInput {
            scope: &continuation["scope"],
            after_key: &continuation["after_key"],
        },
    ));
}

fn goal_index_prefix(goal_id: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(GOAL_INDEX_HASH_DOMAIN);
    hasher.update(goal_id.as_bytes());
    format!("{}::", hasher.finalize().to_hex())
}

fn exercise_continuation_boundary(scenario: &Scenario, selector: u8) {
    let mutation = selector % 6;
    let mut continuation = if mutation < 3 {
        serde_json::to_value(&scenario.recovery_continuation).unwrap()
    } else {
        serde_json::to_value(&scenario.goal_continuation).unwrap()
    };
    match mutation {
        0 => continuation["after_key"] = json!(format!("wrong::{selector:03}")),
        1 => {
            continuation["scope"] = json!({ "Goal": { "goal_hash": digest("wrong-goal") } });
        }
        2 => {
            continuation["after_key"] =
                json!(format!("prepared::99999999999999999999::{selector:03}"))
        }
        3 => continuation["after_key"] = json!(format!("wrong::{selector:03}")),
        4 => {
            continuation["scope"] = json!({ "Goal": { "goal_hash": digest("different-goal") } });
        }
        _ => {
            continuation["after_key"] = json!(format!(
                "{}99999999999999999999::{selector:03}",
                goal_index_prefix(&scenario.goal_id)
            ));
        }
    }
    recompute_continuation_hash(&mut continuation);
    let continuation: PlanningAttemptContinuation = serde_json::from_value(continuation).unwrap();
    continuation.validate().unwrap();
    if mutation < 2 {
        let error = scenario
            .query
            .recoverable_commands_bounded(Some(&continuation), 1)
            .unwrap_err();
        assert!(matches!(
            error,
            PlanningAttemptStorageError::InvalidInput(_)
        ));
        assert_fatal_attempt_error(&error);
    } else if mutation == 2 {
        let selection = scenario
            .query
            .recoverable_commands_bounded(Some(&continuation), 1)
            .unwrap();
        assert!(selection.recoveries.len() <= 1);
        for recovery in selection.recoveries {
            assert_eq!(recovery.head.state, PlanningAttemptState::CommandPrepared);
            recovery.prepared.validate(&recovery.head.identity).unwrap();
        }
    } else if mutation < 5 {
        let error = scenario
            .query
            .attempts_for_goal_bounded(&scenario.goal_id, Some(&continuation), 1)
            .unwrap_err();
        assert!(matches!(
            error,
            PlanningAttemptStorageError::InvalidInput(_)
        ));
        assert_fatal_attempt_error(&error);
    } else {
        let selection = scenario
            .query
            .attempts_for_goal_bounded(&scenario.goal_id, Some(&continuation), 1)
            .unwrap();
        assert!(selection.attempts.len() <= 1);
        for attempt in selection.attempts {
            assert_eq!(attempt.identity.goal_id(), scenario.goal_id);
        }
    }
}

fn exercise_lifecycle_order(scenario: &Scenario, data: &[u8], selector: u8) {
    let mutation = selector % 8;
    let salt = selector.wrapping_add(input_byte(data, usize::from(mutation)));
    let identity = completed_identity(
        &format!("order-goal-{selector:03}-{salt:03}"),
        u64::from(salt) + 100,
        &format!("order-frame-{selector:03}-{salt:03}"),
        salt,
    );
    let ordered_request = request(&identity);
    let diagnostic = PlanningAttemptTerminalDiagnostic::new(
        PlanningAttemptDiagnosticDisposition::PlanningFailed,
        "out_of_order",
        "the lifecycle operation is intentionally out of order",
    )
    .unwrap();
    let result = match mutation {
        0 => scenario
            .store
            .record_decision(identity.attempt_id(), scenario.audit.clone()),
        1 => scenario
            .store
            .prepare_command(identity.attempt_id(), ordered_request),
        2 => scenario
            .store
            .record_command_outcome(identity.attempt_id(), scenario.primary_receipt.clone()),
        3 => {
            scenario.store.open_attempt(identity.clone()).unwrap();
            scenario
                .store
                .prepare_command(identity.attempt_id(), ordered_request)
        }
        4 => {
            scenario.store.open_attempt(identity.clone()).unwrap();
            scenario
                .store
                .record_command_outcome(identity.attempt_id(), scenario.primary_receipt.clone())
        }
        5 => {
            scenario.store.open_attempt(identity.clone()).unwrap();
            scenario
                .store
                .record_terminal_diagnostic(identity.attempt_id(), diagnostic)
        }
        6 => scenario
            .store
            .record_terminal_diagnostic(scenario.primary_identity.attempt_id(), diagnostic),
        _ => scenario.store.prepare_command(
            scenario.failure_identity.attempt_id(),
            request(&scenario.primary_identity),
        ),
    };
    let error = result.unwrap_err();
    assert!(matches!(
        error,
        PlanningAttemptStorageError::Corrupt(_)
            | PlanningAttemptStorageError::IdentityConflict(_)
            | PlanningAttemptStorageError::InvalidInput(_)
    ));
    assert_fatal_attempt_error(&error);
}

fn entry_with_value(tree: &sled::Tree, value: &[u8]) -> (sled::IVec, sled::IVec) {
    tree.iter()
        .filter_map(Result::ok)
        .find(|(key, stored)| key.as_ref() != KEY_SCHEMA && stored.as_ref() == value)
        .unwrap()
}

fn record_key(attempt_id: &str, ordinal: u32) -> Vec<u8> {
    format!("{attempt_id}::{ordinal:010}").into_bytes()
}

fn exercise_durable_reopen_boundary(scenario: &Scenario, data: &[u8], selector: u8) {
    let db = sled::Config::new().temporary(true).open().unwrap();
    {
        let store = owned_attempt_store(db.clone());
        for (identity, request) in [
            (
                &scenario.primary_identity,
                scenario.primary_prepared.request.clone(),
            ),
            (
                &scenario.secondary_identity,
                request(&scenario.secondary_identity),
            ),
        ] {
            store.open_attempt(identity.clone()).unwrap();
            store
                .record_decision(identity.attempt_id(), scenario.audit.clone())
                .unwrap();
            store
                .prepare_command(identity.attempt_id(), request)
                .unwrap();
        }
        store
            .record_command_outcome(
                scenario.primary_identity.attempt_id(),
                scenario.primary_receipt.clone(),
            )
            .unwrap();
    }

    let records = db.open_tree(TREE_ATTEMPT_RECORDS).unwrap();
    let heads = db.open_tree(TREE_ATTEMPT_HEADS).unwrap();
    let goals = db.open_tree(TREE_ATTEMPT_GOALS).unwrap();
    let recovery = db.open_tree(TREE_ATTEMPT_RECOVERY).unwrap();
    let mutation = selector % 16;
    match mutation {
        0 => {
            records
                .remove(record_key(scenario.primary_identity.attempt_id(), 2))
                .unwrap();
        }
        1 => {
            let key = record_key(scenario.primary_identity.attempt_id(), 2);
            let mut record: Value =
                serde_json::from_slice(&records.get(&key).unwrap().unwrap()).unwrap();
            record["kind"]["DecisionAudited"]["decision"] =
                serde_json::to_value(divergent_audit(scenario, data, selector)).unwrap();
            recompute_record_hash(&mut record);
            records
                .insert(key, serde_json::to_vec(&record).unwrap())
                .unwrap();
        }
        2 => {
            let key = record_key(scenario.primary_identity.attempt_id(), 3);
            let mut record: Value =
                serde_json::from_slice(&records.get(&key).unwrap().unwrap()).unwrap();
            record["previous_record_hash"] =
                json!(replacement_digest(data, "durable-chain", selector));
            recompute_record_hash(&mut record);
            records
                .insert(key, serde_json::to_vec(&record).unwrap())
                .unwrap();
        }
        3 => {
            let key = record_key(scenario.primary_identity.attempt_id(), 3);
            let mut record: Value =
                serde_json::from_slice(&records.get(&key).unwrap().unwrap()).unwrap();
            let mut prepared = scenario.primary_prepared.clone();
            prepared.request.base_revision = prepared.request.base_revision.saturating_add(1);
            prepared.request_hash = command::request_hash(&prepared.request);
            record["kind"]["CommandPrepared"]["prepared"] = serde_json::to_value(prepared).unwrap();
            recompute_record_hash(&mut record);
            records
                .insert(key, serde_json::to_vec(&record).unwrap())
                .unwrap();
        }
        4 => {
            let key = record_key(scenario.primary_identity.attempt_id(), 4);
            let mut record: Value =
                serde_json::from_slice(&records.get(&key).unwrap().unwrap()).unwrap();
            record["kind"]["CommandOutcome"]["outcome"] =
                serde_json::to_value(divergent_valid_outcome(scenario, data, selector)).unwrap();
            recompute_record_hash(&mut record);
            records
                .insert(key, serde_json::to_vec(&record).unwrap())
                .unwrap();
        }
        5 => {
            let source_key = record_key(scenario.primary_identity.attempt_id(), 4);
            let mut record: Value =
                serde_json::from_slice(&records.get(&source_key).unwrap().unwrap()).unwrap();
            record["ordinal"] = json!(5_u32);
            record["previous_record_hash"] = record["record_hash"].clone();
            recompute_record_hash(&mut record);
            records
                .insert(
                    record_key(scenario.primary_identity.attempt_id(), 5),
                    serde_json::to_vec(&record).unwrap(),
                )
                .unwrap();
        }
        6 => {
            heads
                .remove(scenario.primary_identity.attempt_id().as_bytes())
                .unwrap();
        }
        7 => {
            let key = scenario.primary_identity.attempt_id().as_bytes();
            let mut head: Value =
                serde_json::from_slice(&heads.get(key).unwrap().unwrap()).unwrap();
            head["head_hash"] = json!(replacement_digest(data, "durable-head", selector));
            heads
                .insert(key, serde_json::to_vec(&head).unwrap())
                .unwrap();
        }
        8 => {
            let (key, _) =
                entry_with_value(&goals, scenario.primary_identity.attempt_id().as_bytes());
            goals.remove(key).unwrap();
        }
        9 => {
            let (key, _) =
                entry_with_value(&goals, scenario.primary_identity.attempt_id().as_bytes());
            goals
                .insert(key, scenario.secondary_identity.attempt_id().as_bytes())
                .unwrap();
        }
        10 => {
            let (key, _) = entry_with_value(
                &recovery,
                scenario.secondary_identity.attempt_id().as_bytes(),
            );
            recovery.remove(key).unwrap();
        }
        11 => {
            let (key, _) = entry_with_value(
                &recovery,
                scenario.secondary_identity.attempt_id().as_bytes(),
            );
            recovery
                .insert(key, scenario.primary_identity.attempt_id().as_bytes())
                .unwrap();
        }
        12 => {
            records.insert(KEY_SCHEMA, b"wrong-schema").unwrap();
        }
        13 => {
            heads.insert(KEY_SCHEMA, b"wrong-schema").unwrap();
        }
        14 => {
            goals.insert(KEY_SCHEMA, b"wrong-schema").unwrap();
        }
        _ => {
            recovery.insert(KEY_SCHEMA, b"wrong-schema").unwrap();
        }
    }
    db.flush().unwrap();
    let error = PlanningAttemptStore::new(db)
        .err()
        .expect("mutated durable authority must fail closed on reopen");
    assert!(matches!(
        error,
        PlanningAttemptStorageError::Corrupt(_) | PlanningAttemptStorageError::SchemaConflict(_)
    ));
    assert_fatal_attempt_error(&error);
}

#[derive(Debug, PartialEq)]
struct SemanticAttemptSnapshot {
    head: PlanningAttemptHead,
    records: Vec<PlanningAttemptRecord>,
    decision: Option<PlanningAttemptDecisionAudit>,
    terminal: Option<PlanningAttemptTerminalDiagnostic>,
    prepared: Option<PlanningPreparedCommand>,
    outcome: Option<PlanningAttemptCommandOutcome>,
}

fn semantic_byte(data: &[u8], index: usize) -> u8 {
    if data.is_empty() {
        return input_byte(data, index);
    }
    data[index % data.len()]
}

fn semantic_token(data: &[u8], offset: usize, label: &str) -> String {
    format!(
        "{label}-{:03}-{:03}-{:03}",
        semantic_byte(data, offset),
        semantic_byte(data, offset.saturating_add(1)),
        semantic_byte(data, offset.saturating_add(2))
    )
}

fn semantic_target(subject: &DomainObjectRef, selector: u8) -> Proposition {
    let object = Term::Object(subject.clone());
    let accessible = || Proposition::Accessible {
        scope: object.clone(),
    };
    let exists = || Proposition::Exists {
        scope: object.clone(),
        artifact_type: Term::ArtifactType(format!("artifact-{selector:03}")),
    };
    match selector % 7 {
        0 => accessible(),
        1 => exists(),
        2 => Proposition::Holds {
            subject: object,
            dimension: Term::Dimension(format!("confidence-{selector:03}")),
            condition: Condition::Equals(Term::Literal(Literal::Number(
                f64::from(selector) / 255.0,
            ))),
        },
        3 => Proposition::Related {
            src: object.clone(),
            relation: Term::Literal(Literal::Text(format!("relation-{selector:03}"))),
            dst: object,
        },
        4 => Proposition::All(vec![accessible(), exists()]),
        5 => Proposition::Any(vec![exists(), accessible()]),
        _ => Proposition::Not(Box::new(accessible())),
    }
}

fn semantic_projection_request(
    data: &[u8],
    slot: usize,
    goal_id: &str,
    source_seq: u64,
) -> PlanningWorldStateRequest {
    let offset = 16_usize.saturating_add(slot.saturating_mul(19));
    let subject = DomainObjectRef::new(
        semantic_token(data, offset, "domain"),
        semantic_token(data, offset.saturating_add(3), "kind"),
        semantic_token(data, offset.saturating_add(6), "object"),
    )
    .unwrap();
    let selector = semantic_byte(data, offset.saturating_add(9));
    let target = semantic_target(&subject, selector);
    let required = semantic_target(&subject, selector.wrapping_add(1));
    let additional_required = semantic_target(&subject, selector.wrapping_add(2));
    let repeated_dimension = semantic_token(data, offset.saturating_add(10), "dimension");
    let request = PlanningWorldStateRequest {
        goal_id: goal_id.to_string(),
        agent_id: semantic_token(data, offset.saturating_add(13), "agent"),
        subject,
        source_seq,
        target,
        perspective: PlanningPerspectiveRef::new(
            semantic_token(data, offset.saturating_add(14), "perspective-kind"),
            semantic_token(data, offset.saturating_add(15), "perspective-id"),
        )
        .unwrap(),
        branch_id: semantic_token(data, offset.saturating_add(16), "branch"),
        requested_dimensions: vec![
            repeated_dimension.clone(),
            semantic_token(data, offset.saturating_add(17), "dimension"),
            repeated_dimension,
        ],
        required_preconditions: vec![required, additional_required],
    };
    request.validate().unwrap();
    request
}

fn semantic_completed_identity(
    data: &[u8],
    slot: usize,
    goal_id: &str,
    sequence: u64,
) -> PlanningAttemptIdentity {
    let request = semantic_projection_request(data, slot, goal_id, sequence);
    let world_state = WorldState::new(vec![
        request.target.clone(),
        Proposition::Accessible {
            scope: Term::Object(request.subject.clone()),
        },
    ])
    .unwrap();
    let offset = 128_usize.saturating_add(slot.saturating_mul(13));
    let mut source_refs = vec![
        serde_json::json!({"ProjectionRule": {"rule_id": semantic_token(data, offset, "source")}}).to_string(),
        serde_json::json!({"ProjectionRule": {"rule_id": semantic_token(data, offset.saturating_add(3), "source")}}).to_string(),
        serde_json::json!({"ProjectionRule": {"rule_id": semantic_token(data, offset, "source")}}).to_string(),
    ];
    source_refs.sort();
    source_refs.dedup();
    let frame = PlanningWorldStateFrameRef::identified_from_authority(
        format!(
            "{}-{}-{}",
            semantic_token(data, offset.saturating_add(8), "projection-version"),
            semantic_token(data, offset.saturating_add(6), "frame"),
            semantic_token(data, offset.saturating_add(7), "projection-request")
        ),
        replacement_digest(
            data,
            "semantic-projection",
            semantic_byte(data, offset.saturating_add(9)),
        ),
        canonical_world_state_hash(&world_state).unwrap(),
        &request,
        &world_state,
        source_refs,
        vec![semantic_token(
            data,
            offset.saturating_add(10),
            "frame-warning",
        )],
    )
    .unwrap();
    let projection_identity =
        PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame).unwrap();
    PlanningAttemptIdentity::bind(
        PlanningRequestIdentity::derive(PlanningRequestIdentityInputs {
            goal_id: goal_id.to_string(),
            goal_updated_at_seq: sequence,
            projection_identity,
            method_library_digest: replacement_digest(
                data,
                "semantic-method-library",
                semantic_byte(data, offset.saturating_add(11)),
            ),
            capability_catalog_digest: replacement_digest(
                data,
                "semantic-capability-catalog",
                semantic_byte(data, offset.saturating_add(12)),
            ),
            planning_version: semantic_token(data, offset.saturating_add(11), "planning-version"),
        })
        .unwrap(),
    )
    .unwrap()
}

fn semantic_failure_identity(
    data: &[u8],
    slot: usize,
    goal_id: &str,
    sequence: u64,
) -> PlanningAttemptIdentity {
    let request = semantic_projection_request(data, slot, goal_id, sequence);
    let offset = 224_usize.saturating_add(slot.saturating_mul(7));
    let inputs = PlanningProjectionFailureIdentityInputs::bind(
        request,
        replacement_digest(
            data,
            "semantic-failure-methods",
            semantic_byte(data, offset),
        ),
        replacement_digest(
            data,
            "semantic-failure-capabilities",
            semantic_byte(data, offset.saturating_add(1)),
        ),
        semantic_token(data, offset.saturating_add(2), "failure-planning-version"),
    )
    .unwrap();
    PlanningAttemptIdentity::bind_projection_failure(inputs).unwrap()
}

fn semantic_audit(
    data: &[u8],
    offset: usize,
    result_summary: PlanningAttemptResultSummary,
) -> PlanningAttemptDecisionAudit {
    let selected_method = matches!(
        result_summary,
        PlanningAttemptResultSummary::Composed { .. }
            | PlanningAttemptResultSummary::LoweringFailed { .. }
            | PlanningAttemptResultSummary::NoMutation { .. }
    )
    .then(|| semantic_token(data, offset, "method-applicable"));
    let statuses = [
        CandidateStatus::TriggerMiss,
        CandidateStatus::PreconditionsUnsatisfied,
        CandidateStatus::PreconditionsIndeterminate,
        CandidateStatus::CostRejected,
        CandidateStatus::EffectProjectionFailed,
        CandidateStatus::EffectMiss,
        CandidateStatus::Applicable,
    ];
    let candidate_reports = statuses
        .into_iter()
        .enumerate()
        .map(|(index, status)| {
            let code = match index {
                0 => PlanningDiagnosticCode::MethodTriggerMiss,
                1 => PlanningDiagnosticCode::MethodPreconditionUnsatisfied,
                2 => PlanningDiagnosticCode::MethodPreconditionIndeterminate,
                3 => PlanningDiagnosticCode::MethodCostCeilingExceeded,
                4 => PlanningDiagnosticCode::MethodEffectProjectionFailed,
                5 => PlanningDiagnosticCode::MethodEffectMiss,
                _ => PlanningDiagnosticCode::MethodTriggerDerived,
            };
            let method_id = if index == 6 {
                selected_method.clone().unwrap_or_else(|| {
                    semantic_token(data, offset.saturating_add(index), "method-applicable")
                })
            } else {
                semantic_token(data, offset.saturating_add(index), "method-candidate")
            };
            MethodCandidateReport {
                method_id: method_id.clone(),
                status,
                bindings: (index % 2 == 0).then(Bindings::empty),
                diagnostics: vec![PlanningDiagnostic::new(
                    code,
                    semantic_token(
                        data,
                        offset.saturating_add(index).saturating_add(8),
                        "candidate-diagnostic",
                    ),
                )
                .with_method(method_id)
                .with_step(semantic_token(
                    data,
                    offset.saturating_add(index).saturating_add(16),
                    "candidate-step",
                ))],
            }
        })
        .collect();
    let method_diagnostics = vec![
        PlanningDiagnostic::new(
            PlanningDiagnosticCode::OperatorResolved,
            semantic_token(data, offset.saturating_add(25), "method-diagnostic"),
        )
        .with_method(
            selected_method
                .clone()
                .unwrap_or_else(|| semantic_token(data, offset, "method-observed")),
        )
        .with_step(semantic_token(
            data,
            offset.saturating_add(26),
            "method-step",
        )),
        PlanningDiagnostic::new(
            PlanningDiagnosticCode::OperatorTagsDiagnosticOnly,
            semantic_token(data, offset.saturating_add(27), "method-tags"),
        ),
    ];
    let lowering_diagnostics = vec![
        LoweringDiagnostic::new(
            LoweringDiagnosticCode::GoalStepDeferred,
            semantic_token(data, offset.saturating_add(28), "lowering-goal"),
        )
        .with_step(semantic_token(
            data,
            offset.saturating_add(29),
            "lowering-step",
        )),
        LoweringDiagnostic::new(
            LoweringDiagnosticCode::NoOperatorStep,
            semantic_token(data, offset.saturating_add(30), "lowering-empty"),
        )
        .with_operator(semantic_token(
            data,
            offset.saturating_add(31),
            "lowering-operator",
        )),
    ];
    let repeated_warning = semantic_token(data, offset.saturating_add(32), "projection-warning");
    PlanningAttemptDecisionAudit::new(
        selected_method,
        candidate_reports,
        vec![
            repeated_warning.clone(),
            semantic_token(data, offset.saturating_add(33), "projection-warning"),
            repeated_warning,
        ],
        method_diagnostics,
        lowering_diagnostics,
        result_summary,
    )
    .unwrap()
}

fn semantic_terminal_case(
    data: &[u8],
    offset: usize,
) -> (
    PlanningAttemptResultSummary,
    PlanningAttemptDiagnosticDisposition,
) {
    match semantic_byte(data, offset) % 7 {
        0 => (
            PlanningAttemptResultSummary::Satisfied,
            PlanningAttemptDiagnosticDisposition::Satisfied,
        ),
        1 => (
            PlanningAttemptResultSummary::NoApplicableMethod,
            PlanningAttemptDiagnosticDisposition::NoApplicableMethod,
        ),
        2 => (
            PlanningAttemptResultSummary::Indeterminate,
            PlanningAttemptDiagnosticDisposition::Indeterminate,
        ),
        3 => (
            PlanningAttemptResultSummary::InvalidMethod,
            PlanningAttemptDiagnosticDisposition::InvalidMethod,
        ),
        4 => (
            PlanningAttemptResultSummary::PlanningFailed,
            PlanningAttemptDiagnosticDisposition::PlanningFailed,
        ),
        5 => (
            PlanningAttemptResultSummary::LoweringFailed {
                composition_id: semantic_token(data, offset.saturating_add(1), "composition"),
            },
            PlanningAttemptDiagnosticDisposition::LoweringFailed,
        ),
        _ => (
            PlanningAttemptResultSummary::NoMutation {
                composition_id: semantic_token(data, offset.saturating_add(1), "composition"),
            },
            PlanningAttemptDiagnosticDisposition::NoMutation,
        ),
    }
}

fn semantic_terminal_diagnostic(
    data: &[u8],
    offset: usize,
    disposition: PlanningAttemptDiagnosticDisposition,
) -> PlanningAttemptTerminalDiagnostic {
    PlanningAttemptTerminalDiagnostic::new(
        disposition,
        semantic_token(data, offset, "terminal-code"),
        semantic_token(data, offset.saturating_add(3), "terminal-message"),
    )
    .unwrap()
}

fn semantic_command_request(
    data: &[u8],
    offset: usize,
    identity: &PlanningAttemptIdentity,
    composition_id: &str,
    network: &NetworkState,
) -> command::Request {
    let diagnostics = vec![PlanningDiagnostic::new(
        PlanningDiagnosticCode::OperatorResolved,
        semantic_token(data, offset, "mutation-diagnostic"),
    )
    .with_method(semantic_token(
        data,
        offset.saturating_add(3),
        "mutation-method",
    ))
    .with_step(semantic_token(
        data,
        offset.saturating_add(6),
        "mutation-step",
    ))];
    let mutation_set = Set::new(
        network.network_id.clone(),
        composition_id,
        identity.attempt_id(),
        Vec::new(),
        diagnostics,
    );
    let mut read_preconditions = vec![ReadPrecondition::RevisionIs(network.revision)];
    if semantic_byte(data, offset.saturating_add(9)) & 1 != 0 {
        read_preconditions.push(ReadPrecondition::StateHashIs(network.state_hash.clone()));
    }
    command::Request {
        command_id: planning_task_network_command_id(identity, composition_id).unwrap(),
        network_id: network.network_id.clone(),
        base_revision: network.revision,
        base_state_hash: network.state_hash.clone(),
        read_preconditions,
        command: command::Command::ApplyMutationSet(mutation_set),
    }
}

fn semantic_order(length: usize, data: &[u8], offset: usize) -> Vec<usize> {
    let mut order = (0..length).collect::<Vec<_>>();
    for index in 0..length {
        let swap = usize::from(semantic_byte(data, offset.saturating_add(index))) % length;
        order.swap(index, swap);
    }
    order
}

fn semantic_goal_attempts(
    query: &PlanningAttemptQuery,
    goal_id: &str,
    max_items: usize,
) -> Vec<String> {
    let mut continuation = None;
    let mut attempt_ids = Vec::new();
    for _ in 0..8 {
        let selection = query
            .attempts_for_goal_bounded(goal_id, continuation.as_ref(), max_items)
            .unwrap();
        assert_eq!(selection.budget_exhausted, selection.continuation.is_some());
        attempt_ids.extend(
            selection
                .attempts
                .iter()
                .map(|head| head.identity.attempt_id().to_string()),
        );
        continuation = selection.continuation;
        if continuation.is_none() {
            return attempt_ids;
        }
    }
    panic!("bounded semantic goal selection did not converge");
}

fn semantic_recoveries(query: &PlanningAttemptQuery, max_items: usize) -> Vec<String> {
    let mut continuation = None;
    let mut attempt_ids = Vec::new();
    for _ in 0..8 {
        let selection = query
            .recoverable_commands_bounded(continuation.as_ref(), max_items)
            .unwrap();
        assert_eq!(selection.budget_exhausted, selection.continuation.is_some());
        for recovery in selection.recoveries {
            recovery.prepared.validate(&recovery.head.identity).unwrap();
            attempt_ids.push(recovery.head.identity.attempt_id().to_string());
        }
        continuation = selection.continuation;
        if continuation.is_none() {
            return attempt_ids;
        }
    }
    panic!("bounded semantic recovery selection did not converge");
}

fn semantic_snapshots(
    query: &PlanningAttemptQuery,
    identities: &[PlanningAttemptIdentity],
) -> Vec<SemanticAttemptSnapshot> {
    identities
        .iter()
        .map(|identity| SemanticAttemptSnapshot {
            head: query.attempt(identity.attempt_id()).unwrap().unwrap(),
            records: query
                .history_bounded(identity.attempt_id(), None, 4)
                .unwrap()
                .records,
            decision: query.decision_audit(identity.attempt_id()).unwrap(),
            terminal: query.terminal_diagnostic(identity.attempt_id()).unwrap(),
            prepared: query.prepared_command(identity.attempt_id()).unwrap(),
            outcome: query.command_outcome(identity.attempt_id()).unwrap(),
        })
        .collect()
}

fn semantic_replay_all(
    store: &PlanningAttemptStore,
    identities: &[PlanningAttemptIdentity],
    audits: &[PlanningAttemptDecisionAudit],
    requests: &[command::Request],
    receipts: &[OutcomeReceipt],
    diagnostics: &[PlanningAttemptTerminalDiagnostic],
    data: &[u8],
) {
    for index in semantic_order(identities.len(), data, 244) {
        store.open_attempt(identities[index].clone()).unwrap();
        store
            .record_decision(identities[index].attempt_id(), audits[index].clone())
            .unwrap();
        match index {
            0 | 1 => {
                store
                    .prepare_command(identities[index].attempt_id(), requests[index].clone())
                    .unwrap();
                store
                    .record_command_outcome(identities[index].attempt_id(), receipts[index].clone())
                    .unwrap();
            }
            2 | 3 => {
                store
                    .record_terminal_diagnostic(
                        identities[index].attempt_id(),
                        diagnostics[index - 2].clone(),
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
    }
}

fn exercise_semantic_typed_refusals(store: &PlanningAttemptStore, data: &[u8], sequence: u64) {
    let empty_network = NetworkState::empty(semantic_token(data, 266, "refusal-network"));
    for variant in 0..4 {
        let goal_id = semantic_token(data, 270 + variant * 3, "refusal-goal");
        let identity = semantic_completed_identity(
            data,
            8 + variant,
            &goal_id,
            sequence
                .saturating_add(u64::try_from(variant).unwrap())
                .saturating_add(1),
        );
        let composition_id = semantic_token(data, 286 + variant * 3, "refusal-composition");
        let request = semantic_command_request(
            data,
            300 + variant * 11,
            &identity,
            &composition_id,
            &empty_network,
        );
        let error = match variant {
            0 => store
                .record_decision(
                    identity.attempt_id(),
                    semantic_audit(
                        data,
                        340,
                        PlanningAttemptResultSummary::Composed { composition_id },
                    ),
                )
                .unwrap_err(),
            1 => {
                store.open_attempt(identity.clone()).unwrap();
                store
                    .prepare_command(identity.attempt_id(), request)
                    .unwrap_err()
            }
            2 => {
                store.open_attempt(identity.clone()).unwrap();
                store
                    .record_terminal_diagnostic(
                        identity.attempt_id(),
                        semantic_terminal_diagnostic(
                            data,
                            360,
                            PlanningAttemptDiagnosticDisposition::PlanningFailed,
                        ),
                    )
                    .unwrap_err()
            }
            _ => {
                store.open_attempt(identity.clone()).unwrap();
                store
                    .record_decision(
                        identity.attempt_id(),
                        semantic_audit(
                            data,
                            372,
                            PlanningAttemptResultSummary::NoMutation { composition_id },
                        ),
                    )
                    .unwrap();
                store
                    .prepare_command(identity.attempt_id(), request)
                    .unwrap_err()
            }
        };
        assert!(matches!(
            error,
            PlanningAttemptStorageError::InvalidInput(_)
                | PlanningAttemptStorageError::IdentityConflict(_)
        ));
        assert_fatal_attempt_error(&error);
    }
}

fn exercise_semantic_lifecycle(data: &[u8]) {
    let root = tempfile::tempdir().unwrap();
    let attempt_path = root.path().join("planning-attempts");
    let network_path = root.path().join("task-networks");
    let goal_id = semantic_token(data, 0, "semantic-goal");
    let sequence = u64::from(semantic_byte(data, 3)).saturating_add(1);
    let network_id = semantic_token(data, 4, "semantic-network");
    let empty_network = NetworkState::empty(network_id.clone());
    let accepted_composition = semantic_token(data, 7, "accepted-composition");
    let rejected_composition = semantic_token(data, 10, "rejected-composition");
    let accepted_identity = semantic_completed_identity(data, 0, &goal_id, sequence);
    let rejected_identity =
        semantic_completed_identity(data, 1, &goal_id, sequence.saturating_add(1));
    let failure_identity = semantic_failure_identity(data, 2, &goal_id, sequence.saturating_add(2));
    let terminal_identity =
        semantic_completed_identity(data, 3, &goal_id, sequence.saturating_add(3));
    let identities = vec![
        accepted_identity,
        rejected_identity,
        failure_identity,
        terminal_identity,
    ];
    let (terminal_summary, terminal_disposition) = semantic_terminal_case(data, 13);
    let audits = vec![
        semantic_audit(
            data,
            32,
            PlanningAttemptResultSummary::Composed {
                composition_id: accepted_composition.clone(),
            },
        ),
        semantic_audit(
            data,
            64,
            PlanningAttemptResultSummary::Composed {
                composition_id: rejected_composition.clone(),
            },
        ),
        semantic_audit(data, 96, PlanningAttemptResultSummary::ProjectionFailed),
        semantic_audit(data, 128, terminal_summary),
    ];
    let diagnostics = vec![
        semantic_terminal_diagnostic(
            data,
            176,
            PlanningAttemptDiagnosticDisposition::ProjectionFailed,
        ),
        semantic_terminal_diagnostic(data, 184, terminal_disposition),
    ];
    let requests = vec![
        semantic_command_request(
            data,
            192,
            &identities[0],
            &accepted_composition,
            &empty_network,
        ),
        semantic_command_request(
            data,
            208,
            &identities[1],
            &rejected_composition,
            &empty_network,
        ),
    ];

    let attempt_db = sled::Config::new().path(&attempt_path).open().unwrap();
    let store = Arc::new(owned_attempt_store(attempt_db.clone()));
    let query = PlanningAttemptQuery::new(Arc::clone(&store));
    let open_order = semantic_order(identities.len(), data, 224);
    let decision_order = semantic_order(identities.len(), data, 228);
    let mut decision_recorded = vec![false; identities.len()];
    for index in open_order {
        store.open_attempt(identities[index].clone()).unwrap();
        if semantic_byte(data, 232 + index) & 1 != 0 {
            store
                .record_decision(identities[index].attempt_id(), audits[index].clone())
                .unwrap();
            decision_recorded[index] = true;
        }
    }
    for index in decision_order {
        if !decision_recorded[index] {
            store
                .record_decision(identities[index].attempt_id(), audits[index].clone())
                .unwrap();
        }
    }
    for index in semantic_order(2, data, 236) {
        store
            .prepare_command(identities[index].attempt_id(), requests[index].clone())
            .unwrap();
    }
    for index in semantic_order(2, data, 238) {
        store
            .record_terminal_diagnostic(
                identities[index + 2].attempt_id(),
                diagnostics[index].clone(),
            )
            .unwrap();
    }

    let recovery_page_size = usize::from(semantic_byte(data, 240) % 2).saturating_add(1);
    let mut expected_recovery_ids = identities[..2]
        .iter()
        .map(|identity| identity.attempt_id().to_string())
        .collect::<Vec<_>>();
    let mut recovered_ids = semantic_recoveries(&query, recovery_page_size);
    expected_recovery_ids.sort();
    recovered_ids.sort();
    assert_eq!(recovered_ids, expected_recovery_ids);

    let goal_page_size = usize::from(semantic_byte(data, 241) % 3).saturating_add(1);
    let mut expected_goal_ids = identities
        .iter()
        .map(|identity| identity.attempt_id().to_string())
        .collect::<Vec<_>>();
    let mut goal_ids = semantic_goal_attempts(&query, &goal_id, goal_page_size);
    expected_goal_ids.sort();
    goal_ids.sort();
    assert_eq!(goal_ids, expected_goal_ids);

    let factory = meld_execution::task_network::store::TaskNetworkStoreFactory::new(&network_path);
    let mut authority = TaskNetworkAuthority::open(&factory, &network_id, 8).unwrap();
    let command_port = authority.command_port();
    let network_query = authority.query_port();
    let accepted_response = command_port.try_submit(requests[0].clone()).unwrap();
    assert!(matches!(
        accepted_response,
        command::Response::Accepted { .. }
    ));
    let rejected_response = command_port.try_submit(requests[1].clone()).unwrap();
    assert!(matches!(
        rejected_response,
        command::Response::Rejected(Rejection::StaleBase {
            expected: 0,
            actual: 1
        })
    ));
    let receipts = vec![
        network_query
            .command_outcome(&requests[0].command_id)
            .unwrap()
            .unwrap(),
        network_query
            .command_outcome(&requests[1].command_id)
            .unwrap()
            .unwrap(),
    ];
    for index in 0..2 {
        assert_eq!(receipts[index].command_id(), requests[index].command_id);
        assert_eq!(
            receipts[index].request_hash(),
            command::request_hash(&requests[index])
        );
    }
    assert_eq!(receipts[0].response(), &accepted_response);
    assert_eq!(receipts[1].response(), &rejected_response);
    let journal = network_query.journal().unwrap();
    assert_eq!(journal.len(), 1);
    let meld_execution::task_network::journal::JournalRecord::Commit(commit) = &journal[0] else {
        panic!("semantic planning command did not create a commit journal record");
    };
    assert_eq!(commit.command_id, requests[0].command_id);
    assert_eq!(
        commit.mutation_set,
        match &requests[0].command {
            command::Command::ApplyMutationSet(set) => set.clone(),
            _ => unreachable!(),
        }
    );
    let network_state = network_query.state().unwrap();
    authority.shutdown().unwrap();

    let mut reopened_authority = TaskNetworkAuthority::open(&factory, &network_id, 8).unwrap();
    let reopened_query = reopened_authority.query_port();
    assert_eq!(reopened_query.journal().unwrap(), journal);
    assert_eq!(reopened_query.state().unwrap(), network_state);
    for index in 0..2 {
        assert_eq!(
            reopened_query
                .command_outcome(&requests[index].command_id)
                .unwrap()
                .unwrap(),
            receipts[index]
        );
    }
    let duplicate = reopened_authority
        .command_port()
        .try_submit(requests[0].clone())
        .unwrap();
    assert!(matches!(duplicate, command::Response::Duplicate { .. }));
    assert_eq!(
        reopened_authority
            .command_port()
            .try_submit(requests[1].clone())
            .unwrap(),
        rejected_response
    );
    assert_eq!(reopened_query.journal().unwrap(), journal);
    reopened_authority.shutdown().unwrap();

    for index in semantic_order(2, data, 242) {
        store
            .record_command_outcome(identities[index].attempt_id(), receipts[index].clone())
            .unwrap();
    }
    assert!(semantic_recoveries(&query, recovery_page_size).is_empty());
    semantic_replay_all(
        &store,
        &identities,
        &audits,
        &requests,
        &receipts,
        &diagnostics,
        data,
    );
    let before_reopen = semantic_snapshots(&query, &identities);
    drop(query);
    drop(store);
    drop(attempt_db);

    let reopened_db = sled::Config::new().path(&attempt_path).open().unwrap();
    let reopened_store = Arc::new(owned_attempt_store(reopened_db));
    let reopened_query = PlanningAttemptQuery::new(Arc::clone(&reopened_store));
    assert_eq!(
        semantic_snapshots(&reopened_query, &identities),
        before_reopen
    );
    assert_eq!(
        semantic_goal_attempts(&reopened_query, &goal_id, goal_page_size),
        semantic_goal_attempts(&reopened_query, &goal_id, 4)
    );
    assert!(semantic_recoveries(&reopened_query, 1).is_empty());
    semantic_replay_all(
        &reopened_store,
        &identities,
        &audits,
        &requests,
        &receipts,
        &diagnostics,
        data,
    );
    assert_eq!(
        semantic_snapshots(&reopened_query, &identities),
        before_reopen
    );
    exercise_semantic_typed_refusals(&reopened_store, data, sequence.saturating_add(100));
}

fn exercise_valid_convergence(scenario: &Scenario) {
    for record in &scenario.records {
        record.validate_shape().unwrap();
    }
    scenario
        .primary_outcome
        .validate(&scenario.primary_prepared)
        .unwrap();
    assert_eq!(
        scenario
            .store
            .open_attempt(scenario.primary_identity.clone())
            .unwrap()
            .state,
        PlanningAttemptState::CommandCompleted
    );
    assert_eq!(
        scenario
            .store
            .record_command_outcome(
                scenario.primary_identity.attempt_id(),
                scenario.primary_receipt.clone(),
            )
            .unwrap()
            .state,
        PlanningAttemptState::CommandCompleted
    );
    let recovery = scenario
        .query
        .recoverable_commands_bounded(None, 2)
        .unwrap();
    assert_eq!(recovery.recoveries.len(), 1);
    assert_eq!(
        recovery.recoveries[0].head.identity.attempt_id(),
        scenario.secondary_identity.attempt_id()
    );
}

fuzz_target!(|data: &[u8]| {
    exercise_semantic_lifecycle(data);
    let scenario = build_scenario(data);
    exercise_valid_convergence(&scenario);

    let operation_count = data.len().clamp(1, MAX_OPERATIONS);
    let mut durable_reopen_exercised = false;
    let mut authentication_exercised = false;
    for index in 0..operation_count {
        let opcode = input_byte(data, index) % 10;
        let selector = input_byte(data, index.saturating_add(1))
            .wrapping_add(u8::try_from(index).unwrap_or(u8::MAX));
        match opcode {
            0 => exercise_valid_convergence(&scenario),
            1 => exercise_identity_boundary(&scenario, data, selector),
            2 => exercise_audit_boundary(&scenario, data, selector),
            3 => exercise_record_boundary(&scenario, data, selector),
            4 => exercise_prepared_boundary(&scenario, data, selector),
            5 => exercise_outcome_boundary(&scenario, data, selector),
            6 if !authentication_exercised => {
                exercise_authentication_boundary(&scenario, data, selector);
                authentication_exercised = true;
            }
            6 => exercise_outcome_boundary(&scenario, data, selector),
            7 => exercise_continuation_boundary(&scenario, selector),
            8 => exercise_lifecycle_order(&scenario, data, selector),
            _ if !durable_reopen_exercised => {
                exercise_durable_reopen_boundary(&scenario, data, selector);
                durable_reopen_exercised = true;
            }
            _ => exercise_record_boundary(&scenario, data, selector),
        }
    }

    if let Ok(record) = serde_json::from_slice::<PlanningAttemptRecord>(data) {
        let valid = record.validate_shape().is_ok();
        let encoded = serde_json::to_vec(&record).unwrap();
        let decoded = serde_json::from_slice::<PlanningAttemptRecord>(&encoded).unwrap();
        assert_eq!(decoded.validate_shape().is_ok(), valid);
    }
});
