//! Exercise the Execution-to-Security boundary with explicitly supplied mitigation Tasks.
//! The fixture grants mutation separately; Security's installed Strategy remains read-only.

use super::*;
use crate::code_change::{acquisition, capability::*, contracts::*};
use meld_world_model::{AgentAuthorizedProduct, AgentProductAuthorization};

#[test]
fn native_mitigation_returns_before_independent_security_reconciliation() {
    assert_native_security_reconciliation(true, false, SecuritySourceAdvance::Mitigation);
}

pub(super) fn genesis(harness: &StewardshipHarness, assembly: &ProductRuntimeAssembly) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/dependency_security");
    let package = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), package.path().join(entry.file_name())).unwrap();
        }
    }
    let policy_path = "authority_policy.security_read_only.json";
    let mut policy: meld_lang::AuthorityPolicy =
        serde_json::from_slice(&std::fs::read(package.path().join(policy_path)).unwrap()).unwrap();
    policy
        .principal_granted_action_ids
        .extend([APPLY.into(), acquisition::READ.into()]);
    policy
        .runtime_allowed_action_ids
        .extend([APPLY.into(), acquisition::READ.into()]);
    policy.principal_granted_action_ids.sort();
    policy.runtime_allowed_action_ids.sort();
    let bytes = serde_json::to_vec(&policy).unwrap();
    std::fs::write(package.path().join(policy_path), &bytes).unwrap();
    let manifest_path = package.path().join("pds-package.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    let components = manifest["components"].as_array_mut().unwrap();
    for component in &mut *components {
        if component["content"]["path"] == policy_path {
            component["content"]["content_hash"] = blake3::hash(&bytes).to_hex().to_string().into();
        }
    }
    for capability in [contract(), acquisition::contract()] {
        components.push(serde_json::json!({
            "component_id": format!("explicit-{}", capability.capability_type_id),
            "owner_component_id": format!("{}.v1", capability.capability_type_id),
            "route": { "owner_domain": "execution", "component_kind": "capability-contract", "route_version": 1 },
            "component_schema_version": 1,
            "content": { "kind": "published_exact", "publisher": "execution.capability-contract.v1", "id": format!("{}.v1", capability.capability_type_id), "content_identity": capability.content_identity() },
            "requires": []
        }));
    }
    std::fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    harness.run_world_genesis_from(assembly, package.path());
}

fn records(harness: &StewardshipHarness) -> Vec<meld_events::EventRecord> {
    harness
        .authority
        .replay_capability()
        .newest_page(1024)
        .unwrap()
        .records
}

fn assert_unsatisfied(assembly: &ProductRuntimeAssembly, goal_id: &str) {
    let store = &assembly.stores().agent_store;
    let plan = store.current_reconciliation_plan(goal_id).unwrap().unwrap();
    assert!(!store
        .goal_disposition_for_plan(&plan.plan_revision_id)
        .unwrap()
        .is_some_and(|disposition| matches!(
            disposition.lifecycle,
            meld_lang::GoalLifecycle::Satisfied { .. }
        )));
}

fn materialize(
    harness: &StewardshipHarness,
    assembly: &ProductRuntimeAssembly,
    supervisor: &mut RuntimeSupervisor<'_>,
    change: CodeChangeSet,
) -> u64 {
    use meld_execution::task_admission::TaskAdmissionDecision;
    let RuntimeSemanticHandleFactory::AgentActor(agent) = &assembly
        .handle_factories()
        .get(AGENT_RECONCILIATION_RUNTIME_ID)
        .unwrap()
        .semantic
    else {
        unreachable!()
    };
    let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
        .handle_factories()
        .get("execution.task_admission")
        .unwrap()
        .semantic
    else {
        unreachable!()
    };
    let fence = agent.authority_port.observe().unwrap().unwrap();
    let policy = agent.strategy.authority_policy.as_ref().unwrap();
    assert!(!agent
        .strategy
        .package
        .requested_authority
        .iter()
        .any(|action| action == APPLY));
    let operator: meld_lang::Operator = serde_json::from_value(serde_json::json!({
        "operator_id": "apply-declared-mitigation", "preconditions": [],
        "effects": [{ "Assert": { "Exists": { "scope": { "Object": change.subject }, "artifact_type": { "ArtifactType": RECEIPT } } } }],
        "cost": { "time_ms": 1, "money_microdollars": 0, "provider_calls": 0 },
        "resolution": {
            "requires_inputs": [{ "artifact_type": { "ArtifactType": CHANGE_SET }, "required": true }],
            "requires_outputs": [{ "artifact_type": { "ArtifactType": RECEIPT }, "required": true }],
            "scope_kind": "workspace_change", "tags": [],
            "specific": { "capability_type_id": APPLY, "capability_version": 1 }
        }
    })).unwrap();
    let mut reader = operator.clone();
    reader.operator_id = "read-declared-mitigation".into();
    reader.effects = vec![meld_lang::Effect::Assert(meld_lang::Proposition::Exists {
        scope: meld_lang::Term::Object(change.subject.clone()),
        artifact_type: meld_lang::Term::ArtifactType(CHANGE_SET.into()),
    })];
    reader.resolution.requires_outputs = reader.resolution.requires_inputs.clone();
    reader.resolution.requires_inputs.clear();
    reader
        .resolution
        .specific
        .as_mut()
        .unwrap()
        .capability_type_id = acquisition::READ.into();
    let composition = meld_lang::Composition {
        steps: vec![
            meld_lang::Step {
                step_id: "read".into(),
                kind: meld_lang::StepKind::Op(reader),
            },
            meld_lang::Step {
                step_id: "apply".into(),
                kind: meld_lang::StepKind::Op(operator),
            },
        ],
        edges: vec![meld_lang::Edge {
            from: "read".into(),
            to: "apply".into(),
            kind: meld_lang::EdgeKind::DataFlow {
                artifact_type: meld_lang::Term::ArtifactType(CHANGE_SET.into()),
            },
        }],
    };
    let actions = vec![APPLY.into(), acquisition::READ.into()];
    let read_only: meld_lang::AuthorityPolicy = serde_json::from_str(include_str!(
        "../../../../theory/dependency_security/authority_policy.security_read_only.json"
    ))
    .unwrap();
    let read_only = meld_lang::AuthorityPolicyBinding::new(
        read_only.clone(),
        read_only.content_hash().unwrap(),
    )
    .unwrap();
    assert!(
        meld_lang::evaluate_authority(&read_only, &actions, &composition, &change.subject).is_err(),
        "Security's shipped read grant must not authorize mutation"
    );
    let decision =
        meld_lang::evaluate_authority(policy, &actions, &composition, &change.subject).unwrap();
    let task_id = format!("mitigation-task::{}", change.change_id);
    let authorization = AgentProductAuthorization {
        request_ref: None,
        agent_id: agent.strategy.agent_id.clone(),
        goal_id: format!("explicit-change-goal::{}", change.change_id),
        plan_revision_id: format!("explicit-change-plan::{}", change.change_id),
        product_id: task_id.clone(),
        authorization_id: format!("explicit-change-authorization::{}", change.change_id),
        context_id: format!("explicit-change-context::{}", change.change_id),
        authority_scope_id: policy.policy.policy_id.clone(),
        authority_policy_content_hash: policy.content_hash.clone(),
        authority_decision: Some(decision),
        activation_generation: fence.activation_generation.clone(),
        admission_epoch: fence.admission_epoch.clone(),
        curation_authorization: None,
        product: AgentAuthorizedProduct::Task(Box::new(meld_world_model::strategy::StrategyTask {
            source_basis_id: None,
            effect_visibility: None,
            return_milestone: Some(
                meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal {
                    task_id: task_id.clone(),
                },
            ),
            task_id: task_id.clone(),
            execution_subject: Some(change.subject.clone()),
            initial_inputs: vec![],
            composition,
            bindings: meld_lang::Bindings::empty(),
            capability_contract_ids: {
                let mut refs = vec![
                    contract().content_identity(),
                    acquisition::contract().content_identity(),
                ];
                refs.sort();
                refs
            },
            expected_outcome_contract_id: "code_change.materialized.v1".into(),
            authority_requirements: actions,
            idempotency_key: task_id.clone(),
        })),
        idempotency_key: task_id,
    };
    let proposal_path = harness._external.path().join("code-proposal.json");
    std::fs::write(&proposal_path, serde_json::to_vec(&change).unwrap()).unwrap();
    let admitted = agent.execution.submit(&authorization).unwrap();
    assert_eq!(
        execution.network.lock().unwrap().state().admissions[&admitted.admission_id].decision,
        TaskAdmissionDecision::Admitted
    );
    // Let the source owner observe effects before the accepted outcome is published.
    for _ in 0..6 {
        for owner in ["execution.task_admission", "execution.task_dispatch"] {
            let report = supervisor.step_owner_for_test(owner, WorkBudget { max_items: 8 });
            assert!(
                report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                "{owner}: {report:?}"
            );
        }
    }
    let unpublished = agent.execution.observe(&authorization).unwrap().unwrap();
    assert!(unpublished.outcome_id.is_some());
    assert!(unpublished.execution_publication_position_id.is_none());
    for _ in 0..2 {
        let report = supervisor.step_owner_for_test(
            "dependency_security.observation",
            WorkBudget { max_items: 1 },
        );
        assert!(
            report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
            "{report:?}"
        );
    }
    let published =
        supervisor.step_owner_for_test("execution.publication", WorkBudget { max_items: 8 });
    assert!(
        published.fatal_errors.is_empty() && published.retryable_errors.is_empty(),
        "{published:?}"
    );
    let prepared = assembly.prepared_activation().unwrap();
    let compilation = assembly
        .stores()
        .pds_products
        .compilation(&prepared.assignment.product_compilation_receipt_id)
        .unwrap()
        .unwrap();
    let bindings = crate::runtime::owners::preparation::prepare_owner_bindings(
        assembly.stores(),
        &harness.binding,
        &compilation.installed_owner_revisions,
    )
    .unwrap();
    let seed: serde_json::Value =
        serde_json::from_str(bindings.get("owner-runtime::dependency-security").unwrap()).unwrap();
    let resources: BTreeMap<String, String> =
        serde_json::from_value(seed["resources"]["bindings"].clone()).unwrap();
    let pending = meld_dependency_security_owner::dependency_security::test_support::pending_inventory_returns(resources.clone(), &harness.authority.replay_capability(), None).unwrap();
    assert_eq!(
        pending.len(),
        1,
        "late Execution publication still needs an explicit source acknowledgment"
    );
    let other_workspace = tempfile::tempdir().unwrap();
    assert!(
        meld_dependency_security_owner::dependency_security::test_support::pending_inventory_returns(resources, &harness.authority.replay_capability(), Some(other_workspace.path().into()))
        .unwrap()
        .is_empty(),
        "a recognized subject on another physical workspace cannot receive this outcome"
    );
    let returned = agent.execution.observe(&authorization).unwrap().unwrap();
    assert!(returned.outcome_id.is_some(), "{returned:?}");
    assert!(
        returned.execution_publication_position_id.is_some(),
        "{returned:?}"
    );
    let network = execution.network.lock().unwrap();
    let account = meld_execution::task_network::sharing::admission_discharge_account(
        network.state(),
        &admitted.admission_id,
    )
    .unwrap();
    assert_eq!(account.outcome_id, returned.outcome_id.unwrap());
    let outcome = network
        .state()
        .outcomes
        .values()
        .find(|outcome| {
            outcome.artifact_records.iter().any(|artifact| {
                artifact.artifact_type_id == RECEIPT
                    && artifact.content["change_id"] == change.change_id
            })
        })
        .unwrap();
    assert_eq!(
        outcome.status,
        meld_execution::task_network::dispatch::OutcomeStatus::Succeeded
    );
    let receipt: CodeChangeReceipt = serde_json::from_value(
        outcome
            .artifact_records
            .iter()
            .find(|artifact| artifact.artifact_type_id == RECEIPT)
            .unwrap()
            .content
            .clone(),
    )
    .unwrap();
    assert_eq!(receipt.change_id, change.change_id);
    for file in &change.files {
        assert_eq!(
            std::fs::read_to_string(harness._workspace.path().join(&file.relative_path)).unwrap(),
            file.replacement
        );
    }
    records(harness)
        .iter()
        .map(|event| event.seq)
        .max()
        .unwrap()
}

pub(super) fn exercise(
    harness: &StewardshipHarness,
    assembly: &ProductRuntimeAssembly,
    supervisor: &mut RuntimeSupervisor<'_>,
    goal_id: &str,
    original_manifest: &[u8],
) {
    let root = harness._workspace.path();
    let make_change = |files: Vec<(&str, String)>| {
        CodeChangeSet::new(
            harness.binding.subject.clone(),
            vec![meld_events::DomainObjectRef::new(
                "dependency-security",
                meld_dependency_security_owner::dependency_security::capability::ASSESSMENT,
                latest_security_product(
                    &records(harness),
                    meld_dependency_security_owner::dependency_security::capability::ASSESSMENT,
                )
                .1["assessment_id"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()],
            files
                .into_iter()
                .map(|(path, replacement)| FileReplacement {
                    relative_path: path.into(),
                    expected_content_hash: blake3::hash(&std::fs::read(root.join(path)).unwrap())
                        .to_hex()
                        .to_string(),
                    replacement,
                })
                .collect(),
        )
        .unwrap()
    };
    let ineffective = make_change(vec![(
        "Cargo.toml",
        format!(
            "{}\n# mitigation that does not remove the affected version\n",
            std::fs::read_to_string(root.join("Cargo.toml")).unwrap()
        ),
    )]);
    let prior = records(harness)
        .iter()
        .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
        .count();
    materialize(harness, assembly, supervisor, ineffective);
    assert_unsatisfied(assembly, goal_id);
    assert_eq!(
        records(harness)
            .iter()
            .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
            .count(),
        prior,
        "materialization must not author Security products"
    );
    for pass in 0..60 {
        supervisor.tick(4_800 + pass).unwrap();
    }
    assert_unsatisfied(assembly, goal_id);
    assert_eq!(
        records(harness)
            .iter()
            .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
            .count(),
        prior + 4
    );
    let repaired = make_change(vec![
        (
            "Cargo.toml",
            String::from_utf8(original_manifest.to_vec()).unwrap(),
        ),
        (
            "Cargo.lock",
            std::fs::read_to_string(root.join("Cargo.lock"))
                .unwrap()
                .replace("version = \"2.0.0\"", "version = \"1.0.0\""),
        ),
    ]);
    let before = records(harness)
        .iter()
        .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
        .count();
    let returned_at = materialize(harness, assembly, supervisor, repaired);
    assert_unsatisfied(assembly, goal_id);
    assert_eq!(
        records(harness)
            .iter()
            .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
            .count(),
        before
    );
    // Observation can establish a successor inventory, but only the separate Task verifies it.
    for _ in 0..2 {
        let report = supervisor.step_owner_for_test(
            "dependency_security.observation",
            WorkBudget { max_items: 1 },
        );
        assert!(
            report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
            "{report:?}"
        );
    }
    assert!(records(harness)
        .iter()
        .any(|record| record.seq > returned_at
            && record.event_type == "dependency_security.inventory_observed.v1"));
    assert_unsatisfied(assembly, goal_id);
    assert_eq!(
        records(harness)
            .iter()
            .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
            .count(),
        before
    );
}

fn latest_security_product<'a>(
    events: &'a [meld_events::EventRecord],
    kind: &str,
) -> (u64, &'a serde_json::Value) {
    events
        .iter()
        .rev()
        .filter(|event| event.event_type == "dependency_security.invocation_return.v1")
        .find_map(|event| {
            event.data["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|artifact| artifact["artifact_type_id"] == kind)
                .map(|artifact| (event.seq, &artifact["content"]))
        })
        .unwrap()
}

pub(super) fn verify_trace(harness: &StewardshipHarness) {
    use meld_dependency_security_owner::dependency_security::capability::{
        ASSESSMENT, INVENTORY, VERIFICATION,
    };
    let events = records(harness);
    for kind in [
        acquisition::EVENT,
        "code_change.intent.v1",
        "code_change.materialized.v1",
    ] {
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == kind)
                .count(),
            2,
            "{kind}"
        );
    }
    let materialized = events
        .iter()
        .rev()
        .find(|event| event.event_type == "code_change.materialized.v1")
        .unwrap();
    let observed = events
        .iter()
        .find(|event| {
            event.seq > materialized.seq
                && event.event_type == "dependency_security.inventory_observed.v1"
        })
        .unwrap();
    let acknowledgments: Vec<_> = events
        .iter()
        .filter(|event| {
            event.event_type == "dependency_security.inventory_observed.v1"
                && event.data["execution_causes"]
                    .as_array()
                    .is_some_and(|causes| !causes.is_empty())
        })
        .collect();
    assert_eq!(acknowledgments.len(), 2);
    for acknowledgment in &acknowledgments {
        let causes: Vec<
            meld_dependency_security_owner::dependency_security::returns::ExecutionObservationCause,
        > = serde_json::from_value(acknowledgment.data["execution_causes"].clone()).unwrap();
        assert_eq!(causes.len(), 1);
        let cause = &causes[0];
        assert!(
            cause.materialization.seq < cause.outcome.seq && cause.outcome.seq < acknowledgment.seq
        );
        assert_eq!(
            acknowledgment.provenance.source_records,
            vec![cause.materialization, cause.outcome]
        );
        let predecessor = events
            .iter()
            .find(|event| {
                event.record_id.as_ref()
                    == acknowledgment.data["predecessor_source_id"]
                        .as_str()
                        .map(String::from)
                        .as_ref()
            })
            .unwrap();
        assert_eq!(
            acknowledgment.data["inventory"]["snapshot_id"],
            predecessor.data["inventory"]["snapshot_id"],
            "a late outcome is acknowledged even when source content was already observed"
        );
    }
    let (inventory_at, inventory) = latest_security_product(&events, INVENTORY);
    let (assessment_at, assessment) = latest_security_product(&events, ASSESSMENT);
    let (verified_at, verification) = latest_security_product(&events, VERIFICATION);
    assert!(
        materialized.seq < observed.seq
            && observed.seq < inventory_at
            && inventory_at < assessment_at
            && assessment_at < verified_at
    );
    assert_eq!(
        assessment["inventory_snapshot_ref"],
        inventory["snapshot_id"]
    );
    assert_eq!(
        verification["inventory_snapshot_ref"],
        inventory["snapshot_id"]
    );
    assert_eq!(verification["assessment_ref"], assessment["assessment_id"]);
    assert_eq!(verification["verified"], true);
    let condition_event = events
        .iter()
        .rev()
        .find(|event| {
            event.event_type
                == meld_dependency_security_owner::dependency_security::condition::EVENT
        })
        .unwrap();
    let condition: meld_dependency_security_owner::dependency_security::condition::CurrentSecurityCondition =
        serde_json::from_str(
            condition_event.data["batch"]["objects"][0]["qualifications"]["condition"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
    assert!(condition.verified_clean);
}

pub(super) fn restart(harness: &StewardshipHarness) {
    let before = records(harness);
    let root = harness._workspace.path();
    let files = ["Cargo.toml", "Cargo.lock"].map(|path| std::fs::read(root.join(path)).unwrap());
    std::fs::remove_file(harness._external.path().join("code-proposal.json")).unwrap();
    let assembly = harness.assembly();
    harness.bind_production_routes(&assembly);
    let mut command = SupervisorStartCommand::new("mitigation-reopened", 7_000);
    command.registration_set = assembly.registration_set().cloned();
    let mut supervisor =
        RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
    for pass in 0..30 {
        supervisor.tick(7_100 + pass * 10).unwrap();
    }
    let store = &assembly.stores().agent_store;
    let goals = store
        .reconciliation_goals_for_agent(&harness.binding.agent_id)
        .unwrap();
    assert_eq!(goals.len(), 2);
    for goal in goals {
        let plan = store
            .current_reconciliation_plan(&goal.goal.goal_id)
            .unwrap()
            .unwrap();
        assert!(store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .is_some_and(|disposition| matches!(
                disposition.lifecycle,
                meld_lang::GoalLifecycle::Satisfied { .. }
            )));
    }
    let after = records(harness);
    for kind in [
        acquisition::EVENT,
        "code_change.intent.v1",
        "code_change.materialized.v1",
        "dependency_security.invocation_return.v1",
        "dependency_security.inventory_observed.v1",
    ] {
        let prior: Vec<_> = before
            .iter()
            .filter(|event| event.event_type == kind)
            .collect();
        let retained: Vec<_> = after
            .iter()
            .filter(|event| event.event_type == kind)
            .collect();
        assert_eq!(
            prior, retained,
            "restart must retain exact {kind} without repeating producers"
        );
    }
    assert_eq!(
        ["Cargo.toml", "Cargo.lock"].map(|path| std::fs::read(root.join(path)).unwrap()),
        files
    );
    supervisor.request_shutdown(8_000).unwrap();
}
