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
            request_ref: None,
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
            request_ref: None,
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
            request_ref: None,
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
fn admitted_request_attribution_is_exact_and_absent_from_legacy_wire() {
    let catalog = task_network_support::catalog();
    let legacy = request("legacy", "generation-v1");
    let body = serde_json::to_value(&legacy).unwrap();
    assert!(body["lineage"].get("request_ref").is_none());
    let mut attributed = legacy.clone();
    attributed.lineage.authorization_id = "attributed".into();
    attributed.lineage.request_ref = Some(attributed.lineage.goal_id.clone());
    let mut store = InMemoryTaskNetworkStore::new("request-attribution");
    let admitted = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(attributed.clone())
    .unwrap();
    assert_eq!(admitted.decision, TaskAdmissionDecision::Admitted);
    assert_eq!(
        meld_execution::task_network::TaskAdmissionAttribution::from_record(&admitted).request_ref,
        attributed.lineage.request_ref
    );
    attributed.lineage.authorization_id = "foreign".into();
    attributed.lineage.request_ref = Some("foreign-goal".into());
    let rejected = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(attributed)
    .unwrap();
    assert!(matches!(
        rejected.decision,
        TaskAdmissionDecision::Rejected { .. }
    ));
}

#[test]
fn contracts_without_sharing_permission_keep_distinct_attributed_regions() {
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

fn shared_input_request(
    name: &str,
    opt_in: bool,
) -> (
    meld_execution::capability::CapabilityCatalog,
    TaskAdmissionRequest,
) {
    let mut contract = task_network_support::single_input_dataflow_catalog()
        .get("docs.write_metadata", 1)
        .unwrap()
        .clone();
    if opt_in {
        contract.execution_contract.completion_semantics =
            meld_execution::task_network::sharing::EXACT_INPUT_SHARING_V1.into();
    }
    let mut catalog = meld_execution::capability::CapabilityCatalog::new();
    catalog.register(contract.clone()).unwrap();
    let mut request = frozen_input_request();
    request
        .task
        .composition
        .steps
        .retain(|step| step.step_id == "write_metadata");
    request.task.capability_contract_ids = vec![contract.content_identity()];
    request.task.authority_requirements = vec![contract.capability_type_id.clone()];
    let grant = request.lineage.authority_decision.as_mut().unwrap();
    grant.requested_action_ids = request.task.authority_requirements.clone();
    grant.authorized_action_ids = request.task.authority_requirements.clone();
    request.lineage.authorization_id = format!("authorization-{name}");
    request.lineage.goal_id = format!("goal-{name}");
    request.lineage.plan_revision_id = format!("plan-{name}");
    request.lineage.product_id = format!("task-{name}");
    request.task.task_id = request.lineage.product_id.clone();
    request.task.idempotency_key = request.task.task_id.clone();
    request.idempotency_key = request.task.task_id.clone();
    (catalog, request)
}

fn metadata_sharing_proposal(
    state: &meld_execution::task_network::NetworkState,
    catalog: &meld_execution::capability::CapabilityCatalog,
) -> Result<meld_execution::task_network::sharing::ReadyWorkSharing, String> {
    let nodes: Vec<_> = state
        .tasks
        .values()
        .filter(|node| node.lineage.step_id == "write_metadata")
        .collect();
    assert_eq!(nodes.len(), 2);
    meld_execution::task_network::sharing::ReadyWorkSharing::decide(
        state,
        nodes[1],
        nodes[0],
        catalog.get("docs.write_metadata", 1).unwrap(),
    )
}

fn share_metadata_roots(
    store: &mut SledTaskNetworkStore,
    catalog: &meld_execution::capability::CapabilityCatalog,
) {
    let proposal = metadata_sharing_proposal(store.state(), catalog).unwrap();
    let request = task_network_support::apply_sled_command(
        store,
        "share-ready-metadata",
        Command::ShareReadyWork(Box::new(proposal)),
    );
    assert!(matches!(
        store.submit(request).unwrap(),
        meld_execution::task_network::command::Response::Accepted { .. }
    ));
}

#[test]
fn shared_operational_attempt_preserves_two_admission_accounts_across_reopen() {
    use meld_execution::task_network::{dispatch, sharing::admission_discharge_account};
    let db = sled::Config::new().temporary(true).open().unwrap();
    let (catalog, first) = shared_input_request("first", true);
    let (_, second) = shared_input_request("second", true);
    let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    let mut admissions = Vec::new();
    for request in [first, second] {
        let record = TaskAdmissionApi::new(
            &mut store,
            &catalog,
            "generation-v1",
            "policy-content-docs-v1",
        )
        .admit(request)
        .unwrap();
        assert_eq!(
            record.decision,
            TaskAdmissionDecision::Admitted,
            "{record:?}"
        );
        admissions.push(record);
    }
    let actor = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
        TaskCompiler::new(),
        catalog.clone(),
    ));
    let report = actor
        .run_once(
            &mut store,
            TaskAdmissionRuntimeRequest {
                network_id: "network-docs".into(),
                max_items: 2,
            },
        )
        .unwrap();
    assert_eq!(report.committed, 2, "{report:?}");
    assert_eq!(store.state().tasks.len(), 2);
    share_metadata_roots(&mut store, &catalog);
    assert_eq!(store.state().tasks.len(), 1);
    assert_eq!(store.state().shared_steps.len(), 1);
    let before = store.state().clone();
    store.flush().unwrap();
    drop(store);
    let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    assert_eq!(store.state(), &before);
    assert_eq!(
        actor
            .run_once(
                &mut store,
                TaskAdmissionRuntimeRequest {
                    network_id: "network-docs".into(),
                    max_items: 2
                }
            )
            .unwrap()
            .attempted,
        0
    );
    let node_id = store.state().tasks.keys().next().unwrap().clone();
    let state = store.state();
    let request = meld_execution::task_network::command::Request {
        command_id: "one-claim".into(),
        network_id: state.network_id.clone(),
        base_revision: state.revision,
        base_state_hash: state.state_hash.clone(),
        read_preconditions: Vec::new(),
        command: Command::ClaimReadyTask(dispatch::Request {
            claim_id: "one-claim".into(),
            task_instance_id: node_id.clone(),
            worker_id: "worker".into(),
            idempotency_key: "one-attempt".into(),
        }),
    };
    assert!(matches!(
        store.submit(request).unwrap(),
        meld_execution::task_network::command::Response::Accepted { .. }
    ));
    let claim = store.state().claims["one-claim"].clone();
    assert_eq!(claim.shared_action_decision_ids.len(), 1);
    let (_, late) = shared_input_request("late", true);
    let late = TaskAdmissionApi::new(
        &mut store,
        &catalog,
        "generation-v1",
        "policy-content-docs-v1",
    )
    .admit(late)
    .unwrap();
    let late_report = actor
        .run_once(
            &mut store,
            TaskAdmissionRuntimeRequest {
                network_id: "network-docs".into(),
                max_items: 1,
            },
        )
        .unwrap();
    assert_eq!(late_report.committed, 1, "{late_report:?}");
    assert_eq!(store.state().tasks.len(), 2);
    assert_eq!(store.state().shared_steps.len(), 1);
    assert!(admission_discharge_account(store.state(), &late.admission_id).is_none());
    let mut outcome = task_network_support::outcome_for_claim("one-outcome", &node_id, &claim);
    outcome.artifact_records[0].artifact_type_id = "summary_doc".into();
    outcome.artifact_records[0].producer.task_id = node_id.clone();
    outcome.artifact_records[0].producer.capability_instance_id = "write_metadata".into();
    outcome.artifact_records[0].producer.output_slot_id = Some("summary_doc".into());
    let state = store.state();
    let request = meld_execution::task_network::command::Request {
        command_id: "one-outcome".into(),
        network_id: state.network_id.clone(),
        base_revision: state.revision,
        base_state_hash: state.state_hash.clone(),
        read_preconditions: Vec::new(),
        command: Command::RecordTaskOutcome(outcome),
    };
    assert!(matches!(
        store.submit(request).unwrap(),
        meld_execution::task_network::command::Response::Accepted { .. }
    ));
    let accounts: Vec<_> = admissions
        .iter()
        .map(|admission| {
            admission_discharge_account(store.state(), &admission.admission_id).unwrap()
        })
        .collect();
    assert_ne!(accounts[0].account_id, accounts[1].account_id);
    assert_ne!(accounts[0].admission.goal_id, accounts[1].admission.goal_id);
    assert_eq!(accounts[0].outcome_id, accounts[1].outcome_id);
    assert_eq!(
        accounts[0].shared_action_decision_ids,
        accounts[1].shared_action_decision_ids
    );
    assert_eq!(store.state().claims.len(), 1);
    assert_eq!(store.state().outcomes.len(), 1);
    store.flush().unwrap();
    drop(store);
    let reopened = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    for account in accounts {
        assert_eq!(
            admission_discharge_account(reopened.state(), &account.admission.admission_id),
            Some(account)
        );
    }
}

#[test]
fn work_without_owner_permission_or_with_different_input_stays_distinct() {
    for opt_in in [false, true] {
        let (catalog, first) = shared_input_request("first", opt_in);
        let (_, mut second) = shared_input_request("second", opt_in);
        if opt_in {
            second.task.initial_inputs[0].content =
                serde_json::json!({"exact":"different-source-revision"});
        }
        let mut store = InMemoryTaskNetworkStore::new("network-docs");
        for request in [first, second] {
            let record = TaskAdmissionApi::new(
                &mut store,
                &catalog,
                "generation-v1",
                "policy-content-docs-v1",
            )
            .admit(request)
            .unwrap();
            assert_eq!(
                record.decision,
                TaskAdmissionDecision::Admitted,
                "{record:?}"
            );
        }
        let actor = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
            TaskCompiler::new(),
            catalog.clone(),
        ));
        let report = actor
            .run_once_in_memory(
                &mut store,
                TaskAdmissionRuntimeRequest {
                    network_id: "network-docs".into(),
                    max_items: 2,
                },
            )
            .unwrap();
        assert!(metadata_sharing_proposal(store.state(), &catalog).is_err());
        assert_eq!(report.committed, 2, "{report:?}");
        assert_eq!(store.state().tasks.len(), 2);
        assert!(store.state().shared_steps.is_empty());
    }
}

#[test]
fn independently_valid_authority_and_lifecycle_contexts_do_not_imply_compatibility() {
    for difference in [
        "agent",
        "policy",
        "generation",
        "epoch",
        "subject",
        "principal",
    ] {
        let (catalog, first) = shared_input_request("first", true);
        let (_, mut second) = shared_input_request("second", true);
        match difference {
            "agent" => {
                second.lineage.agent_id = "another-agent".into();
                second
                    .lineage
                    .authority_decision
                    .as_mut()
                    .unwrap()
                    .principal_id = "another-agent".into();
            }
            "policy" => {
                second.lineage.authority_policy_content_hash = "another-policy-revision".into();
                second
                    .lineage
                    .authority_decision
                    .as_mut()
                    .unwrap()
                    .policy_content_hash = "another-policy-revision".into();
            }
            "generation" => second.lineage.activation_generation = "generation-v2".into(),
            "epoch" => second.lineage.admission_epoch = Some("epoch-v2".into()),
            "subject" => {
                let subject = DomainObjectRef::new("workspace", "node", "another-readme").unwrap();
                second.lineage.authority_decision.as_mut().unwrap().subject = subject.clone();
                second.task.execution_subject = Some(subject);
            }
            "principal" => {
                second
                    .lineage
                    .authority_decision
                    .as_mut()
                    .unwrap()
                    .principal_id = "another-principal".into()
            }
            _ => unreachable!(),
        }
        let mut store = InMemoryTaskNetworkStore::new("network-docs");
        for request in [first, second] {
            let generation = request.lineage.activation_generation.clone();
            let policy = request.lineage.authority_policy_content_hash.clone();
            let epoch = request.lineage.admission_epoch.clone();
            let record = TaskAdmissionApi::new(&mut store, &catalog, &generation, &policy)
                .with_admission_epoch(epoch.as_deref())
                .admit(request)
                .unwrap();
            assert_eq!(
                record.decision,
                TaskAdmissionDecision::Admitted,
                "{difference}: {record:?}"
            );
        }
        let actor = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
            TaskCompiler::new(),
            catalog.clone(),
        ));
        let report = actor
            .run_once_in_memory(
                &mut store,
                TaskAdmissionRuntimeRequest {
                    network_id: "network-docs".into(),
                    max_items: 2,
                },
            )
            .unwrap();
        assert!(metadata_sharing_proposal(store.state(), &catalog).is_err());
        assert_eq!(report.committed, 2, "{difference}: {report:?}");
        assert_eq!(store.state().tasks.len(), 2, "{difference}");
        assert!(store.state().shared_steps.is_empty(), "{difference}");
    }
}

#[test]
fn sharing_permission_cannot_cover_external_writes_or_missing_inputs() {
    let (catalog, _) = shared_input_request("first", true);
    let contract = catalog.get("docs.write_metadata", 1).unwrap();
    let mut missing_input = contract.clone();
    missing_input.input_contract.clear();
    assert!(missing_input.validate().is_err());
    let mut missing_output = contract.clone();
    missing_output.output_contract.clear();
    assert!(missing_output.validate().is_err());
    let mut writes = contract.clone();
    writes.effect_contract = vec![meld_execution::capability::EffectSpec {
        effect_id: "write".into(),
        kind: meld_execution::capability::EffectKind::Write,
        target: "workspace".into(),
        exclusive: true,
    }];
    assert!(writes.validate().is_err());
    writes.effect_contract[0].kind = meld_execution::capability::EffectKind::Emit;
    assert!(
        writes.validate().is_err(),
        "exclusive effects cannot be shared"
    );
}

#[test]
fn shared_root_feeds_separate_admitted_descendants_and_waits_for_each_return() {
    for (fail_root, delayed_failure) in [(false, false), (true, false), (true, true)] {
        use meld_execution::task_network::{
            dispatch, sharing::admission_discharge_account, TaskInitSource,
        };
        let (mut catalog, first) = shared_input_request("first", true);
        let (_, second) = shared_input_request("second", true);
        let mut consumer = catalog.get("docs.write_metadata", 1).unwrap().clone();
        consumer.capability_type_id = "docs.consume_summary".into();
        consumer.input_contract[0].accepted_artifact_type_ids = vec!["summary_doc".into()];
        consumer.execution_contract.completion_semantics = "result_or_failure".into();
        catalog.register(consumer.clone()).unwrap();
        let db = sled::Config::new().temporary(true).open().unwrap();
        let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
        let mut admissions = Vec::new();
        for mut request in [first, second] {
            let mut downstream = request.task.composition.steps[0].clone();
            downstream.step_id = "consume_summary".into();
            let StepKind::Op(operator) = &mut downstream.kind else {
                unreachable!()
            };
            operator.operator_id = "consume_summary".into();
            operator
                .resolution
                .specific
                .as_mut()
                .unwrap()
                .capability_type_id = consumer.capability_type_id.clone();
            request.task.composition.steps.push(downstream);
            if delayed_failure {
                let mut independent = request.task.composition.steps[0].clone();
                independent.step_id = "independent".into();
                request.task.composition.steps.push(independent);
                let mut input = request.task.initial_inputs[0].clone();
                input.step_id = "independent".into();
                input.content = serde_json::json!({"independent": request.task.task_id});
                request.task.initial_inputs.push(input);
                request.task.composition.edges.push(meld_lang::Edge {
                    from: "independent".into(),
                    to: "consume_summary".into(),
                    kind: meld_lang::EdgeKind::Ordering,
                });
            }

            request.task.composition.edges.push(meld_lang::Edge {
                from: "write_metadata".into(),
                to: "consume_summary".into(),
                kind: meld_lang::EdgeKind::DataFlow {
                    artifact_type: Term::ArtifactType("summary_doc".into()),
                },
            });
            request
                .task
                .capability_contract_ids
                .push(consumer.content_identity());
            request.task.capability_contract_ids.sort();
            request
                .task
                .authority_requirements
                .push(consumer.capability_type_id.clone());
            request.task.authority_requirements.sort();
            let grant = request.lineage.authority_decision.as_mut().unwrap();
            grant.requested_action_ids = request.task.authority_requirements.clone();
            grant.authorized_action_ids = request.task.authority_requirements.clone();
            let record = TaskAdmissionApi::new(
                &mut store,
                &catalog,
                "generation-v1",
                "policy-content-docs-v1",
            )
            .admit(request)
            .unwrap();
            assert_eq!(
                record.decision,
                TaskAdmissionDecision::Admitted,
                "{record:?}"
            );
            admissions.push(record);
        }
        let actor = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
            TaskCompiler::new(),
            catalog.clone(),
        ));
        let report = actor
            .run_once(
                &mut store,
                TaskAdmissionRuntimeRequest {
                    network_id: "network-docs".into(),
                    max_items: 2,
                },
            )
            .unwrap();
        assert_eq!(report.committed, 2, "{report:?}");
        share_metadata_roots(&mut store, &catalog);
        assert_eq!(
            store.state().tasks.len(),
            if delayed_failure { 5 } else { 3 }
        );
        assert_eq!(store.state().shared_steps.len(), 1);
        let root = store
            .state()
            .tasks
            .values()
            .find(|node| node.lineage.step_id == "write_metadata")
            .unwrap()
            .task_instance_id
            .clone();
        let descendants: Vec<_> = store
            .state()
            .tasks
            .values()
            .filter(|node| node.lineage.step_id == "consume_summary")
            .map(|node| {
                let [TaskInitSource::UpstreamArtifact(source)] = node.init_sources.as_slice()
                else {
                    panic!("{node:?}")
                };
                assert_eq!(source.upstream_task_instance_id, root);
                assert!(store
                    .state()
                    .edges
                    .iter()
                    .any(|edge| edge.from == root && edge.to == node.task_instance_id));
                node.task_instance_id.clone()
            })
            .collect();
        let remaining = if delayed_failure {
            store
                .state()
                .tasks
                .values()
                .filter(|node| node.lineage.step_id == "independent")
                .map(|node| node.task_instance_id.clone())
                .collect()
        } else {
            descendants
        };
        for (index, node_id) in std::iter::once(root).chain(remaining).enumerate() {
            let claim_id = format!("claim-{index}");
            let request = task_network_support::apply_sled_command(
                &store,
                &claim_id,
                Command::ClaimReadyTask(dispatch::Request {
                    claim_id: claim_id.clone(),
                    task_instance_id: node_id.clone(),
                    worker_id: "worker".into(),
                    idempotency_key: claim_id.clone(),
                }),
            );
            assert!(matches!(
                store.submit(request).unwrap(),
                meld_execution::task_network::command::Response::Accepted { .. }
            ));
            let claim = store.state().claims[&claim_id].clone();
            let mut outcome = task_network_support::outcome_for_claim(
                &format!("outcome-{index}"),
                &node_id,
                &claim,
            );
            outcome.artifact_records[0].artifact_type_id = "summary_doc".into();
            outcome.artifact_records[0].producer.task_id = node_id.clone();
            outcome.artifact_records[0].producer.capability_instance_id =
                store.state().tasks[&node_id].lineage.step_id.clone();
            outcome.artifact_records[0].producer.output_slot_id = Some("summary_doc".into());
            if fail_root && index == 0 {
                outcome.status = dispatch::OutcomeStatus::Failed;
                outcome.error = Some("shared producer failed".into());
                outcome.artifact_records.clear();
            }
            let request = task_network_support::apply_sled_command(
                &store,
                &format!("return-{index}"),
                Command::RecordTaskOutcome(outcome),
            );
            assert!(matches!(
                store.submit(request).unwrap(),
                meld_execution::task_network::command::Response::Accepted { .. }
            ));
            let accounts: Vec<_> = admissions
                .iter()
                .filter_map(|record| {
                    admission_discharge_account(store.state(), &record.admission_id)
                })
                .collect();
            assert_eq!(
                accounts.len(),
                if fail_root && !delayed_failure {
                    2
                } else {
                    index
                },
                "shared root alone cannot discharge either complete Task"
            );
            for account in accounts {
                assert!(!account.shared_action_decision_ids.is_empty());
                let publication = store
                    .state()
                    .publications
                    .values()
                    .find(|publication| publication.shared_discharge_accounts.contains(&account))
                    .unwrap();
                assert!(publication.shared_discharge_accounts.contains(&account));
                if delayed_failure {
                    assert_ne!(publication.outcome.outcome_id, account.outcome_id);
                }
            }
            if fail_root && !delayed_failure {
                break;
            }
        }
        let before = store.state().clone();
        store.flush().unwrap();
        drop(store);
        let reopened = SledTaskNetworkStore::open(db, "network-docs").unwrap();
        assert_eq!(reopened.state(), &before);
        let accounts: Vec<_> = reopened
            .state()
            .publications
            .values()
            .flat_map(|publication| &publication.shared_discharge_accounts)
            .collect();
        assert_eq!(accounts.len(), 2);
        assert_ne!(accounts[0].account_id, accounts[1].account_id);
    }
}

fn produced_input_sharing_fixture(
    different_inputs: bool,
) -> (
    sled::Db,
    SledTaskNetworkStore,
    meld_execution::capability::CapabilityTypeContract,
    Vec<String>,
) {
    let original_catalog = task_network_support::single_input_dataflow_catalog();
    let mut contract = original_catalog
        .get("docs.write_metadata", 1)
        .unwrap()
        .clone();
    contract.execution_contract.completion_semantics =
        meld_execution::capability::EXACT_INPUT_SHARING_V1.into();
    let mut catalog = meld_execution::capability::CapabilityCatalog::new();
    catalog
        .register(
            original_catalog
                .get("docs.prepare_metadata", 1)
                .unwrap()
                .clone(),
        )
        .unwrap();
    catalog.register(contract.clone()).unwrap();
    let db = sled::Config::new().temporary(true).open().unwrap();
    let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    for name in ["first", "second"] {
        let mut request = dataflow_request(true);
        request.lineage.authorization_id = format!("authorization-{name}");
        request.lineage.goal_id = format!("goal-{name}");
        request.lineage.plan_revision_id = format!("plan-{name}");
        request.lineage.product_id = format!("task-{name}");
        request.task.task_id = request.lineage.product_id.clone();
        request.task.idempotency_key = request.task.task_id.clone();
        request.idempotency_key = request.task.task_id.clone();
        request.task.capability_contract_ids = catalog
            .iter()
            .map(|contract| contract.content_identity())
            .collect();
        request.task.capability_contract_ids.sort();
        let record = TaskAdmissionApi::new(
            &mut store,
            &catalog,
            "generation-v1",
            "policy-content-docs-v1",
        )
        .admit(request)
        .unwrap();
        assert_eq!(
            record.decision,
            TaskAdmissionDecision::Admitted,
            "{record:?}"
        );
    }
    let report =
        TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(TaskCompiler::new(), catalog))
            .run_once(
                &mut store,
                TaskAdmissionRuntimeRequest {
                    network_id: "network-docs".into(),
                    max_items: 2,
                },
            )
            .unwrap();
    assert_eq!(report.committed, 2);
    assert!(store.state().shared_steps.is_empty());
    let consumers: Vec<_> = store
        .state()
        .tasks
        .values()
        .filter(|node| node.lineage.step_id == "write_metadata")
        .map(|node| node.task_instance_id.clone())
        .collect();
    assert!(
        meld_execution::task_network::sharing::ReadyWorkSharing::decide(
            store.state(),
            &store.state().tasks[&consumers[1]],
            &store.state().tasks[&consumers[0]],
            &contract
        )
        .is_err()
    );
    let producers: Vec<_> = store
        .state()
        .tasks
        .values()
        .filter(|node| node.lineage.step_id == "prepare_metadata")
        .map(|node| node.task_instance_id.clone())
        .collect();
    for (index, node_id) in producers.iter().enumerate() {
        let claim_id = format!("producer-claim-{index}");
        let request = task_network_support::apply_sled_command(
            &store,
            &claim_id,
            Command::ClaimReadyTask(meld_execution::task_network::dispatch::Request {
                claim_id: claim_id.clone(),
                task_instance_id: node_id.clone(),
                worker_id: "worker".into(),
                idempotency_key: claim_id.clone(),
            }),
        );
        assert!(matches!(
            store.submit(request).unwrap(),
            meld_execution::task_network::command::Response::Accepted { .. }
        ));
        let claim = &store.state().claims[&claim_id];
        let mut outcome = task_network_support::outcome_for_claim(
            &format!("producer-outcome-{index}"),
            node_id,
            claim,
        );
        let artifact = &mut outcome.artifact_records[0];
        artifact.artifact_type_id = "metadata_doc".into();
        artifact.content =
            serde_json::json!({"source_revision": if different_inputs { index } else { 0 }});
        artifact.producer.task_id = node_id.clone();
        artifact.producer.capability_instance_id = "prepare_metadata".into();
        artifact.producer.output_slot_id = Some("metadata_doc".into());
        let request = task_network_support::apply_sled_command(
            &store,
            &format!("producer-return-{index}"),
            Command::RecordTaskOutcome(outcome),
        );
        assert!(matches!(
            store.submit(request).unwrap(),
            meld_execution::task_network::command::Response::Accepted { .. }
        ));
    }
    (db, store, contract, consumers)
}

#[test]
fn ready_sharing_revalidates_exact_provenance_and_replays_before_claim() {
    use meld_execution::task_network::{command::Response, sharing::ReadyWorkSharing};
    let (db, mut store, contract, consumers) = produced_input_sharing_fixture(false);
    let decision = ReadyWorkSharing::decide(
        store.state(),
        &store.state().tasks[&consumers[1]],
        &store.state().tasks[&consumers[0]],
        &contract,
    )
    .unwrap();
    assert_eq!(
        decision.primary_inputs.payload.init_artifacts,
        decision.contributor_inputs.payload.init_artifacts
    );
    assert_ne!(
        decision.primary_inputs.provenance,
        decision.contributor_inputs.provenance
    );
    for variant in 0..4 {
        let mut forged = decision.clone();
        match variant {
            0 => forged.decision.decision_id = "forged-decision".into(),
            1 => {
                forged.contributor_inputs.payload.init_artifacts[0].content =
                    serde_json::json!({"forged": true})
            }
            2 => forged.contributor_inputs.provenance = forged.primary_inputs.provenance.clone(),
            3 => {
                forged
                    .decision
                    .capability_contract
                    .execution_contract
                    .completion_semantics = "result_or_failure".into()
            }
            _ => unreachable!(),
        }
        let before = store.state().clone();
        let request = task_network_support::apply_sled_command(
            &store,
            &format!("forged-sharing-{variant}"),
            Command::ShareReadyWork(Box::new(forged)),
        );
        assert!(matches!(
            store.submit(request).unwrap(),
            Response::Rejected(_)
        ));
        assert_eq!(store.state(), &before);
    }
    let request = task_network_support::apply_sled_command(
        &store,
        "valid-sharing",
        Command::ShareReadyWork(Box::new(decision.clone())),
    );
    assert!(matches!(
        store.submit(request.clone()).unwrap(),
        Response::Accepted { .. }
    ));
    assert!(matches!(
        store.submit(request).unwrap(),
        Response::Duplicate { .. }
    ));
    assert_eq!(store.state().tasks.len(), 3);
    assert_eq!(store.state().shared_steps.len(), 1);
    let before = store.state().clone();
    store.flush().unwrap();
    drop(store);
    let mut store = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    assert_eq!(store.state(), &before);
    let request = task_network_support::apply_sled_command(
        &store,
        "shared-claim",
        Command::ClaimReadyTask(meld_execution::task_network::dispatch::Request {
            claim_id: "shared-claim".into(),
            task_instance_id: decision.decision.shared_node_id.clone(),
            worker_id: "worker".into(),
            idempotency_key: "shared-once".into(),
        }),
    );
    assert!(matches!(
        store.submit(request).unwrap(),
        Response::Accepted { .. }
    ));
    assert_eq!(
        store.state().claims["shared-claim"].shared_action_decision_ids,
        vec![decision.decision.decision_id]
    );
}

#[test]
fn ready_sharing_refuses_different_produced_values_and_claimed_contributors() {
    use meld_execution::task_network::{command::Response, sharing::ReadyWorkSharing};
    let (_, store, contract, consumers) = produced_input_sharing_fixture(true);
    assert!(ReadyWorkSharing::decide(
        store.state(),
        &store.state().tasks[&consumers[1]],
        &store.state().tasks[&consumers[0]],
        &contract
    )
    .is_err());
    let (_, mut store, contract, consumers) = produced_input_sharing_fixture(false);
    let proposal = ReadyWorkSharing::decide(
        store.state(),
        &store.state().tasks[&consumers[1]],
        &store.state().tasks[&consumers[0]],
        &contract,
    )
    .unwrap();
    let request = task_network_support::apply_sled_command(
        &store,
        "claim-before-sharing",
        Command::ClaimReadyTask(meld_execution::task_network::dispatch::Request {
            claim_id: "prior-claim".into(),
            task_instance_id: consumers[1].clone(),
            worker_id: "worker".into(),
            idempotency_key: "prior-once".into(),
        }),
    );
    assert!(matches!(
        store.submit(request).unwrap(),
        Response::Accepted { .. }
    ));
    let before = store.state().clone();
    let request = task_network_support::apply_sled_command(
        &store,
        "stale-sharing",
        Command::ShareReadyWork(Box::new(proposal)),
    );
    assert!(matches!(
        store.submit(request).unwrap(),
        Response::Rejected(_)
    ));
    assert_eq!(store.state(), &before);
}

#[test]
fn legacy_admission_sharing_reopens_and_freezes_current_claim() {
    use meld_execution::task_network::{
        command::Response,
        contracts::{stable_hash, stable_id},
        mutation,
        sharing::SharedActionDecision,
        JournalRecord,
    };
    let (catalog, first) = shared_input_request("first", true);
    let (_, second) = shared_input_request("second", true);
    let mut memory = InMemoryTaskNetworkStore::new("network-docs");
    for request in [first, second] {
        TaskAdmissionApi::new(
            &mut memory,
            &catalog,
            "generation-v1",
            "policy-content-docs-v1",
        )
        .admit(request)
        .unwrap();
    }
    TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
        TaskCompiler::new(),
        catalog.clone(),
    ))
    .run_once_in_memory(
        &mut memory,
        TaskAdmissionRuntimeRequest {
            network_id: "network-docs".into(),
            max_items: 2,
        },
    )
    .unwrap();
    let mut records = memory.journal().to_vec();
    let JournalRecord::Commit(commit) = records.last_mut().unwrap() else {
        unreachable!()
    };
    let mutation::Mutation::Inject(inject) = &mut commit.mutation_set.mutations[0];
    let primary = memory
        .state()
        .tasks
        .values()
        .find(|node| node.task_instance_id != inject.task_node.task_instance_id)
        .unwrap();
    let contract = catalog.get("docs.write_metadata", 1).unwrap().clone();
    // Exact persisted identity and Inject form authored by bf38e701. This fixture
    // deliberately bypasses today's writer to characterize the historical reader.
    let decision = SharedActionDecision {
        decision_id: stable_id(
            "execution-work-compatibility-v1",
            &(
                memory.state().network_id.as_str(),
                stable_hash(primary),
                stable_hash(&inject.task_node),
                &contract.content_identity(),
            ),
        ),
        shared_node_id: primary.task_instance_id.clone(),
        capability_contract: contract,
    };
    inject.mutation_id = stable_id(
        "shared-admitted-step",
        &(&inject.mutation_id, &decision.decision_id),
    );
    inject.sharing = Some(decision.clone());
    let mut expected = memory.state().clone();
    expected.tasks.remove(&inject.task_node.task_instance_id);
    expected.statuses.remove(&inject.task_node.task_instance_id);
    expected
        .shared_steps
        .insert(inject.task_node.task_instance_id.clone(), inject.clone());
    expected.state_hash = expected.recompute_state_hash();
    let set = mutation::Set::new(
        &commit.mutation_set.network_id,
        &commit.mutation_set.source_task_id,
        &commit.mutation_set.idempotency_key,
        commit.mutation_set.mutations.clone(),
    );
    *commit = mutation::CommitRecord::new(
        &commit.command_id,
        &commit.network_id,
        commit.revision,
        &commit.previous_state_hash,
        &expected.state_hash,
        set.clone(),
    );
    let db = sled::Config::new().temporary(true).open().unwrap();
    let journal = db.open_tree("task_network_journal_by_revision").unwrap();
    for (index, record) in records.iter().enumerate() {
        journal
            .insert(
                ((index + 1) as u64).to_be_bytes(),
                serde_json::to_vec(&serde_json::json!({"record": record})).unwrap(),
            )
            .unwrap();
    }
    db.flush().unwrap();
    let mut reopened = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    assert_eq!(reopened.state(), &expected);
    let request = task_network_support::apply_sled_command(
        &reopened,
        "retired-admission-sharing",
        Command::ApplyMutationSet(set),
    );
    assert!(matches!(
        reopened.submit(request).unwrap(),
        Response::Rejected(_)
    ));
    let request = task_network_support::apply_sled_command(
        &reopened,
        "current-claim",
        Command::ClaimReadyTask(meld_execution::task_network::dispatch::Request {
            claim_id: "current-claim".into(),
            task_instance_id: decision.shared_node_id,
            worker_id: "worker".into(),
            idempotency_key: "current-once".into(),
        }),
    );
    assert!(matches!(
        reopened.submit(request).unwrap(),
        Response::Accepted { .. }
    ));
    assert_eq!(
        reopened.state().claims["current-claim"].shared_action_decision_ids,
        vec![decision.decision_id]
    );
    let expected = reopened.state().clone();
    reopened.flush().unwrap();
    drop(reopened);
    assert_eq!(
        SledTaskNetworkStore::open(db, "network-docs")
            .unwrap()
            .state(),
        &expected
    );
}

#[test]
fn ready_sharing_checks_each_contributors_requested_action() {
    for requested in [true, false] {
        let (catalog, first) = shared_input_request("first", true);
        let (_, mut second) = shared_input_request("second", true);
        let grant = second.lineage.authority_decision.as_mut().unwrap();
        if requested {
            grant
                .requested_action_ids
                .push("extra.requested-action".into());
            grant.requested_action_ids.sort();
        } else {
            grant.requested_action_ids.clear();
        }
        let mut store = InMemoryTaskNetworkStore::new("network-docs");
        for request in [first, second] {
            let record = TaskAdmissionApi::new(
                &mut store,
                &catalog,
                "generation-v1",
                "policy-content-docs-v1",
            )
            .admit(request)
            .unwrap();
            assert_eq!(
                record.decision,
                TaskAdmissionDecision::Admitted,
                "{record:?}"
            );
        }
        let report = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
            TaskCompiler::new(),
            catalog.clone(),
        ))
        .run_once_in_memory(
            &mut store,
            TaskAdmissionRuntimeRequest {
                network_id: "network-docs".into(),
                max_items: 2,
            },
        )
        .unwrap();
        assert_eq!(report.committed, 2);
        assert_eq!(
            metadata_sharing_proposal(store.state(), &catalog).is_ok(),
            requested
        );
    }
}
