#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::DomainObjectRef;
use meld_execution::task::TaskCompiler;
use meld_execution::task_admission::{
    ExecutionTask, TaskAdmissionApi, TaskAdmissionDecision, TaskAdmissionLineage,
    TaskAdmissionRecord, TaskAdmissionRequest,
};
use meld_execution::task_admission::{
    TaskAdmissionLowerer, TaskAdmissionRuntimeActor, TaskAdmissionRuntimeRequest,
};
use meld_execution::task_network::{Command, InMemoryTaskNetworkStore, SledTaskNetworkStore};
use meld_lang::{AuthorityDecision, Bindings, CapabilityRef, Literal, StepKind, Term};

fn request(authorization_id: &str, generation: &str) -> TaskAdmissionRequest {
    let catalog = task_network_support::catalog();
    let contract = catalog.get("docs.write", 1).unwrap();
    let task_id = "task-docs".to_string();
    let mut composition = task_network_support::composition().composition;
    let StepKind::Op(operator) = &mut composition.steps[0].kind else {
        unreachable!();
    };
    operator.resolution.specific = Some(CapabilityRef {
        capability_type_id: "docs.write".to_string(),
        capability_version: 1,
    });
    TaskAdmissionRequest {
        lineage: TaskAdmissionLineage {
            agent_id: "agent-docs".to_string(),
            goal_id: "goal-docs".to_string(),
            plan_revision_id: "plan-docs-v1".to_string(),
            product_id: task_id.clone(),
            authorization_id: authorization_id.to_string(),
            context_id: "context-docs-v1".to_string(),
            authority_scope_id: "policy-docs".to_string(),
            authority_policy_content_hash: "policy-content-docs-v1".to_string(),
            authority_decision: Some(AuthorityDecision {
                policy_id: "policy-docs".to_string(),
                policy_content_hash: "policy-content-docs-v1".to_string(),
                principal_id: "agent-docs".to_string(),
                subject: DomainObjectRef::new("workspace", "node", "readme").unwrap(),
                requested_action_ids: vec!["docs.write".to_string()],
                authorized_action_ids: vec!["docs.write".to_string()],
            }),
            activation_generation: generation.to_string(),
            admission_epoch: None,
        },
        task: ExecutionTask {
            execution_subject: None,
            initial_inputs: Vec::new(),
            task_id: task_id.clone(),
            composition,
            bindings: Bindings::empty(),
            capability_contract_ids: vec![contract.content_identity()],
            expected_outcome_contract_id: "docs-patch-v1".to_string(),
            authority_requirements: vec!["docs.write".to_string()],
            idempotency_key: task_id.clone(),
        },
        idempotency_key: task_id,
    }
}

fn dataflow_request(with_edge: bool) -> TaskAdmissionRequest {
    let catalog = task_network_support::single_input_dataflow_catalog();
    let mut composition = task_network_support::single_input_dataflow_composition().composition;
    if !with_edge {
        composition.edges.clear();
    }
    for step in &mut composition.steps {
        let StepKind::Op(operator) = &mut step.kind else {
            unreachable!();
        };
        let capability_type_id = match step.step_id.as_str() {
            "prepare_metadata" => "docs.prepare_metadata",
            "write_metadata" => "docs.write_metadata",
            _ => unreachable!(),
        };
        operator.resolution.specific = Some(CapabilityRef {
            capability_type_id: capability_type_id.to_string(),
            capability_version: 1,
        });
    }
    let mut contract_ids = ["docs.prepare_metadata", "docs.write_metadata"]
        .iter()
        .map(|capability| catalog.get(capability, 1).unwrap().content_identity())
        .collect::<Vec<_>>();
    contract_ids.sort();
    let task_id = "task-dataflow".to_string();
    let actions = vec![
        "docs.prepare_metadata".to_string(),
        "docs.write_metadata".to_string(),
    ];
    TaskAdmissionRequest {
        lineage: TaskAdmissionLineage {
            agent_id: "agent-docs".to_string(),
            goal_id: "goal-docs".to_string(),
            plan_revision_id: "plan-docs-v1".to_string(),
            product_id: task_id.clone(),
            authorization_id: "authorization-dataflow".to_string(),
            context_id: "context-docs-v1".to_string(),
            authority_scope_id: "policy-docs".to_string(),
            authority_policy_content_hash: "policy-content-docs-v1".to_string(),
            authority_decision: Some(AuthorityDecision {
                policy_id: "policy-docs".to_string(),
                policy_content_hash: "policy-content-docs-v1".to_string(),
                principal_id: "agent-docs".to_string(),
                subject: DomainObjectRef::new("workspace", "node", "readme").unwrap(),
                requested_action_ids: actions.clone(),
                authorized_action_ids: actions.clone(),
            }),
            activation_generation: "generation-v1".to_string(),
            admission_epoch: None,
        },
        task: ExecutionTask {
            execution_subject: None,
            initial_inputs: Vec::new(),
            task_id: task_id.clone(),
            composition,
            bindings: Bindings::empty(),
            capability_contract_ids: contract_ids,
            expected_outcome_contract_id: "docs-summary-v1".to_string(),
            authority_requirements: actions,
            idempotency_key: task_id.clone(),
        },
        idempotency_key: task_id,
    }
}

fn duplicate_optional_dataflow_request() -> TaskAdmissionRequest {
    let catalog = task_network_support::optional_input_dataflow_catalog();
    let mut composition =
        task_network_support::duplicate_optional_input_dataflow_composition().composition;
    for step in &mut composition.steps {
        let StepKind::Op(operator) = &mut step.kind else {
            unreachable!();
        };
        let capability_type_id = match step.step_id.as_str() {
            "produce_optional_note" => "docs.produce_optional_note",
            "produce_other_note" => "docs.produce_other_note",
            "consume_optional_note" => "docs.consume_optional_note",
            _ => unreachable!(),
        };
        operator.resolution.specific = Some(CapabilityRef {
            capability_type_id: capability_type_id.to_string(),
            capability_version: 1,
        });
    }
    let actions = vec![
        "docs.produce_optional_note".to_string(),
        "docs.produce_other_note".to_string(),
        "docs.consume_optional_note".to_string(),
    ];
    let contract_ids = actions
        .iter()
        .map(|capability| catalog.get(capability, 1).unwrap().content_identity())
        .collect::<Vec<_>>();
    let task_id = "task-duplicate-optional-input".to_string();
    TaskAdmissionRequest {
        lineage: TaskAdmissionLineage {
            agent_id: "agent-docs".to_string(),
            goal_id: "goal-docs".to_string(),
            plan_revision_id: "plan-docs-v1".to_string(),
            product_id: task_id.clone(),
            authorization_id: "authorization-duplicate-optional-input".to_string(),
            context_id: "context-docs-v1".to_string(),
            authority_scope_id: "policy-docs".to_string(),
            authority_policy_content_hash: "policy-content-docs-v1".to_string(),
            authority_decision: Some(AuthorityDecision {
                policy_id: "policy-docs".to_string(),
                policy_content_hash: "policy-content-docs-v1".to_string(),
                principal_id: "agent-docs".to_string(),
                subject: DomainObjectRef::new("workspace", "node", "readme").unwrap(),
                requested_action_ids: actions.clone(),
                authorized_action_ids: actions.clone(),
            }),
            activation_generation: "generation-v1".to_string(),
            admission_epoch: None,
        },
        task: ExecutionTask {
            execution_subject: None,
            initial_inputs: Vec::new(),
            task_id: task_id.clone(),
            composition,
            bindings: Bindings::empty(),
            capability_contract_ids: contract_ids,
            expected_outcome_contract_id: "docs-summary-v1".to_string(),
            authority_requirements: actions,
            idempotency_key: task_id.clone(),
        },
        idempotency_key: task_id,
    }
}

#[test]
fn exact_epoch_fences_new_offers_without_erasing_prior_admission() {
    let catalog = task_network_support::catalog();
    let mut store = InMemoryTaskNetworkStore::new("epoch-network");
    let mut first = request("epoch-1-authority", "generation-v1");
    first.lineage.admission_epoch = Some("epoch-1".into());
    let accepted = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .with_admission_epoch(Some("epoch-1"))
    .admit(first.clone())
    .unwrap();
    assert_eq!(accepted.decision, TaskAdmissionDecision::Admitted);
    let mut stale = first.clone();
    stale.lineage.authorization_id = "foreign-epoch-authority".into();
    let refused = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .with_admission_epoch(Some("epoch-2"))
    .admit(stale)
    .unwrap();
    assert!(matches!(
        refused.decision,
        TaskAdmissionDecision::StaleFence { .. }
    ));
    let replay = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .with_admission_epoch(Some("epoch-2"))
    .admit(first)
    .unwrap();
    assert_eq!(replay, accepted);
    let mut next = request("epoch-2-authority", "generation-v1");
    next.lineage.admission_epoch = Some("epoch-2".into());
    assert_eq!(
        TaskAdmissionApi::new(
            &mut store,
            &catalog,
            "generation-v1",
            "policy-content-docs-v1"
        )
        .with_admission_epoch(Some("epoch-2"))
        .admit(next)
        .unwrap()
        .decision,
        TaskAdmissionDecision::Admitted
    );
}

#[test]
fn admission_persists_exact_decision_and_replays_without_revision_growth() {
    let catalog = task_network_support::catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let offered = request("authorization-docs-v1", "generation-v1");
    let first = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(offered.clone())
    .unwrap();
    let revision = store.state().revision;
    let replay = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(offered)
    .unwrap();

    assert_eq!(first.decision, TaskAdmissionDecision::Admitted);
    assert_eq!(replay, first);
    assert_eq!(store.state().revision, revision);
    assert_eq!(
        store.state().admissions.get(&first.admission_id),
        Some(&first)
    );
}

#[test]
fn admission_records_stale_and_rejected_offers_without_lowering_them() {
    let catalog = task_network_support::catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let stale = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v2",
        "policy-content-docs-v1",
    )
    .admit(request("authorization-stale", "generation-v1"))
    .unwrap();
    let mut denied = request("authorization-denied", "generation-v2");
    denied.lineage.authority_decision = None;
    let rejected = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v2",
        "policy-content-docs-v1",
    )
    .admit(denied)
    .unwrap();

    assert!(matches!(
        stale.decision,
        TaskAdmissionDecision::StaleFence { .. }
    ));
    assert!(matches!(
        rejected.decision,
        TaskAdmissionDecision::Rejected { .. }
    ));
    let lowerer = TaskAdmissionLowerer::new(TaskCompiler::new(), catalog);
    assert!(!lowerer.lower("network-docs", &stale).diagnostics.is_empty());
    assert!(!lowerer
        .lower("network-docs", &rejected)
        .diagnostics
        .is_empty());
}

#[test]
fn two_authorizations_commit_distinct_attributed_operational_regions() {
    let temp = tempfile::tempdir().unwrap();
    let db = sled::open(temp.path().join("execution.sled")).unwrap();
    let catalog = task_network_support::catalog();
    let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    let first = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(request("authorization-a", "generation-v1"))
    .unwrap();
    let second = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(request("authorization-b", "generation-v1"))
    .unwrap();
    assert_ne!(first.admission_id, second.admission_id);
    assert_eq!(
        first.decision,
        TaskAdmissionDecision::Admitted,
        "{first:#?}"
    );
    assert_eq!(
        second.decision,
        TaskAdmissionDecision::Admitted,
        "{second:#?}"
    );

    let actor =
        TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(TaskCompiler::new(), catalog));
    let report = actor
        .run_once(
            &mut store,
            TaskAdmissionRuntimeRequest {
                network_id: "network-docs".to_string(),
                max_items: 2,
            },
        )
        .unwrap();
    assert_eq!(report.attempted, 2, "{report:#?}");
    assert_eq!(report.committed, 2, "{report:#?}");
    assert_eq!(store.state().tasks.len(), 2);
    let admissions = store
        .state()
        .tasks
        .values()
        .map(|node| {
            let attribution = node.lineage.admission.as_ref().unwrap();
            assert_eq!(
                node.lineage.authority_decision,
                store
                    .state()
                    .admissions
                    .get(&attribution.admission_id)
                    .unwrap()
                    .request
                    .lineage
                    .authority_decision
            );
            attribution.admission_id.clone()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(admissions.len(), 2);

    store.flush().unwrap();
    drop(store);
    let reopened = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    assert_eq!(reopened.state().tasks.len(), 2);
    assert_eq!(reopened.state().admissions.len(), 2);
}

#[test]
fn admission_rejects_an_execution_subject_outside_the_exact_grant() {
    let catalog = task_network_support::catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let mut offered = request("authorization-foreign-subject", "generation-v1");
    offered.task.execution_subject = Some(
        meld_events::DomainObjectRef::new("curation", "expectation", "different-subject").unwrap(),
    );
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(offered)
    .unwrap();
    let TaskAdmissionDecision::Rejected { grounds } = record.decision else {
        panic!("foreign subject admitted")
    };
    assert!(grounds
        .iter()
        .any(|ground| ground.contains("execution subject")));
}

#[test]
fn admission_contract_round_trips_with_complete_authority_lineage() {
    let catalog = task_network_support::catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(request("authorization-round-trip", "generation-v1"))
    .unwrap();
    let encoded = serde_json::to_vec(&record).unwrap();
    let decoded: TaskAdmissionRecord = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, record);
}

#[test]
fn public_command_contract_rejects_forged_admission_decision() {
    let catalog = task_network_support::catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(request("authorization-forged", "generation-v1"))
    .unwrap();
    let forged = serde_json::json!({ "RecordTaskAdmission": record });

    assert!(serde_json::from_value::<Command>(forged).is_err());
}

#[test]
fn admission_rejects_contract_and_action_sets_that_are_not_exact() {
    let catalog = task_network_support::catalog();
    let mut offered = request("authorization-extra-contract", "generation-v1");
    offered
        .task
        .capability_contract_ids
        .push("speculative-contract".to_string());
    offered
        .task
        .authority_requirements
        .push("docs.speculative".to_string());
    offered
        .lineage
        .authority_decision
        .as_mut()
        .unwrap()
        .authorized_action_ids
        .push("docs.speculative".to_string());
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(offered)
    .unwrap();

    let TaskAdmissionDecision::Rejected { grounds } = record.decision else {
        panic!("inexact Task contract and action sets were admitted");
    };
    assert!(grounds
        .iter()
        .any(|ground| ground.contains("exactly equal")));
}

#[test]
fn admission_rejects_missing_required_binding_without_lowering_repair() {
    let catalog = task_network_support::catalog_with_required_node_binding();
    let mut offered = request("authorization-missing-binding", "generation-v1");
    offered.task.capability_contract_ids =
        vec![catalog.get("docs.write", 1).unwrap().content_identity()];
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(offered)
    .unwrap();

    let TaskAdmissionDecision::Rejected { grounds } = record.decision else {
        panic!("Task with an absent required binding was admitted");
    };
    assert!(grounds
        .iter()
        .any(|ground| ground.contains("required binding")));
}

#[test]
fn admitted_optional_contract_binding_is_preserved_during_lowering() {
    let catalog = task_network_support::catalog_with_required_node_binding();
    let mut offered = request("authorization-optional-binding", "generation-v1");
    offered.task.capability_contract_ids =
        vec![catalog.get("docs.write", 1).unwrap().content_identity()];
    offered.task.bindings = Bindings::empty()
        .bind(
            "node".to_string(),
            Term::Literal(Literal::Text("readme".to_string())),
        )
        .unwrap()
        .bind(
            "mode".to_string(),
            Term::Literal(Literal::Text("strict".to_string())),
        )
        .unwrap();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(offered)
    .unwrap();
    assert_eq!(record.decision, TaskAdmissionDecision::Admitted);

    let plan =
        TaskAdmissionLowerer::new(TaskCompiler::new(), catalog).lower("network-docs", &record);
    assert!(plan.diagnostics.is_empty(), "{:#?}", plan.diagnostics);
    let capability = plan
        .mutations
        .mutations
        .iter()
        .map(|mutation| match mutation {
            meld_execution::task_network::Mutation::Inject(inject) => {
                &inject.task_node.compiled_task.capability_instances[0]
            }
        })
        .next()
        .unwrap();
    assert_eq!(capability.binding_values.len(), 2);
    let optional = capability
        .binding_values
        .iter()
        .find(|binding| binding.binding_id == "mode")
        .unwrap();
    assert_eq!(
        optional.value,
        serde_json::to_value(Term::Literal(Literal::Text("strict".to_string()))).unwrap()
    );
}

#[test]
fn admission_rejects_ambiguous_binding_aliases() {
    let catalog = task_network_support::catalog_with_required_node_binding();
    let mut offered = request("authorization-ambiguous-binding", "generation-v1");
    offered.task.capability_contract_ids =
        vec![catalog.get("docs.write", 1).unwrap().content_identity()];
    offered.task.bindings = Bindings::empty()
        .bind(
            "node".to_string(),
            Term::Literal(Literal::Text("readme".to_string())),
        )
        .unwrap()
        .bind(
            "?node".to_string(),
            Term::Literal(Literal::Text("other".to_string())),
        )
        .unwrap();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(offered)
    .unwrap();

    let TaskAdmissionDecision::Rejected { grounds } = record.decision else {
        panic!("Task with ambiguous binding aliases was admitted");
    };
    assert!(grounds
        .iter()
        .any(|ground| ground.contains("ambiguous aliases")));
}

#[test]
fn admission_rejects_required_input_without_exact_task_dataflow() {
    let catalog = task_network_support::single_input_dataflow_catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(dataflow_request(false))
    .unwrap();

    let TaskAdmissionDecision::Rejected { grounds } = record.decision else {
        panic!("Task with an unclosed required input was admitted");
    };
    assert!(grounds.iter().any(|ground| ground.contains("not closed")));
}

#[test]
fn admission_rejects_duplicate_optional_input_edges_before_lowering() {
    let catalog = task_network_support::optional_input_dataflow_catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(duplicate_optional_dataflow_request())
    .unwrap();

    let TaskAdmissionDecision::Rejected { grounds } = record.decision else {
        panic!("Task with duplicate optional input edges was admitted");
    };
    assert!(grounds
        .iter()
        .any(|ground| ground.contains("accepts at most one")));
}

#[test]
fn exact_task_dataflow_lowers_without_static_seed_invention() {
    let catalog = task_network_support::single_input_dataflow_catalog();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(dataflow_request(true))
    .unwrap();
    assert_eq!(record.decision, TaskAdmissionDecision::Admitted);

    let plan =
        TaskAdmissionLowerer::new(TaskCompiler::new(), catalog).lower("network-docs", &record);
    assert!(plan.diagnostics.is_empty(), "{:#?}", plan.diagnostics);
    let sources = plan
        .mutations
        .mutations
        .iter()
        .flat_map(|mutation| match mutation {
            meld_execution::task_network::Mutation::Inject(inject) => {
                inject.task_node.init_sources.iter().collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(sources.len(), 1);
    assert!(matches!(
        sources[0],
        meld_execution::task_network::TaskInitSource::UpstreamArtifact(_)
    ));
}

fn frozen_input_request() -> TaskAdmissionRequest {
    let mut request = dataflow_request(false);
    let catalog = task_network_support::single_input_dataflow_catalog();
    let slot = &catalog
        .get("docs.write_metadata", 1)
        .unwrap()
        .input_contract[0];
    request.task.initial_inputs.push(meld_lang::TaskInput {
        step_id: "write_metadata".into(),
        slot_id: slot.slot_id.clone(),
        artifact_type_id: slot.accepted_artifact_type_ids[0].clone(),
        schema_version: 1,
        content: serde_json::json!({"exact": "request payload"}),
    });
    request
}

#[test]
fn frozen_task_input_lowers_with_its_exact_value_and_rejects_ambiguous_or_foreign_sources() {
    let catalog = task_network_support::single_input_dataflow_catalog();
    let request = frozen_input_request();
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let record = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(request.clone())
    .unwrap();
    assert_eq!(record.decision, TaskAdmissionDecision::Admitted);
    let plan = TaskAdmissionLowerer::new(TaskCompiler::new(), catalog.clone())
        .lower("network-docs", &record);
    assert!(plan.diagnostics.is_empty(), "{:#?}", plan.diagnostics);
    let sources: Vec<_> = plan
        .mutations
        .mutations
        .iter()
        .flat_map(|mutation| match mutation {
            meld_execution::task_network::Mutation::Inject(inject) => {
                inject.task_node.init_sources.iter()
            }
        })
        .collect();
    let [meld_execution::task_network::TaskInitSource::StaticSeed(seed)] = sources.as_slice()
    else {
        panic!("one exact seed expected: {sources:?}");
    };
    assert_eq!(seed.content, request.task.initial_inputs[0].content);
    assert_eq!(seed.init_slot_id, request.task.initial_inputs[0].slot_id);

    for variant in 0..5 {
        let mut invalid = request.clone();
        match variant {
            0 => invalid.task.initial_inputs[0].slot_id = "foreign-slot".into(),
            1 => invalid.task.initial_inputs[0].step_id = "foreign-step".into(),
            2 => invalid.task.initial_inputs[0].schema_version = u32::MAX,
            3 => invalid
                .task
                .initial_inputs
                .push(invalid.task.initial_inputs[0].clone()),
            4 => invalid.task.composition.edges = dataflow_request(true).task.composition.edges,
            _ => unreachable!(),
        }
        let mut store = InMemoryTaskNetworkStore::new("network-docs");
        let record = TaskAdmissionApi::new(
            &mut store,
            &catalog,
            "generation-v1",
            "policy-content-docs-v1",
        )
        .admit(invalid)
        .unwrap();
        assert!(
            matches!(record.decision, TaskAdmissionDecision::Rejected { .. }),
            "variant {variant}: {record:?}"
        );
    }
}
