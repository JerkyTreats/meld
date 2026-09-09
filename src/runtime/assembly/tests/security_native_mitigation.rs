//! Native Agent selection of a declared intervention followed by a distinct verification Task.

use super::*;
use crate::code_change::{acquisition, capability::APPLY, contracts::*};

#[test]
fn native_agent_reconciles_repeated_mitigations_with_independent_verification() {
    assert_native_security_reconciliation(true, false, SecuritySourceAdvance::NativeMitigation);
}

pub(super) fn repeat_for_new_advisory(
    harness: &StewardshipHarness,
    assembly: &ProductRuntimeAssembly,
    supervisor: &mut RuntimeSupervisor<'_>,
) {
    use meld_world_model::{strategy::PlanMilestoneRequirement, AgentAuthorizedProduct};
    let store = &assembly.stores().agent_store;
    let before_seq = assembly
        .ports()
        .event_append()
        .watermark()
        .unwrap()
        .committed_seq;
    let prior_goals = store
        .reconciliation_goals_for_agent(&harness.binding.agent_id)
        .unwrap();
    let prior_authorizations: Vec<_> = prior_goals
        .iter()
        .flat_map(|goal| {
            store
                .product_authorizations_for_goal(&goal.goal.goal_id)
                .unwrap()
        })
        .collect();
    let prior_dispositions: Vec<_> = prior_goals
        .iter()
        .map(|goal| {
            let plan = store
                .current_reconciliation_plan(&goal.goal.goal_id)
                .unwrap()
                .unwrap();
            store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap()
                .unwrap()
        })
        .collect();
    let root = harness._workspace.path();
    let change = CodeChangeSet::new(
        harness.binding.subject.clone(),
        vec![meld_events::DomainObjectRef::new(
            "dependency-security",
            "advisory",
            "replacement-advisory",
        )
        .unwrap()],
        ["Cargo.toml", "Cargo.lock"]
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(root.join(path)).unwrap();
                FileReplacement {
                    relative_path: path.into(),
                    expected_content_hash: blake3::hash(&bytes).to_hex().to_string(),
                    replacement: String::from_utf8(bytes)
                        .unwrap()
                        .replace("version = \"1.0.0\"", "version = \"2.0.0\""),
                }
            })
            .collect(),
    )
    .unwrap();
    std::fs::write(
        harness._external.path().join("code-proposal.json"),
        serde_json::to_vec(&change).unwrap(),
    )
    .unwrap();
    let source_path = harness._external.path().join("advisories.json");
    let mut source: meld_dependency_security_owner::dependency_security::advisory::AdvisorySourceDocumentV1 =
        serde_json::from_slice(&std::fs::read(&source_path).unwrap()).unwrap();
    source.source_revision = "new-exposure-after-successful-mitigation".into();
    source.advisories[0].source_advisory_id = "replacement-advisory".into();
    source.advisories[0].affected_versions = vec!["1.0.0".into()];
    std::fs::write(source_path, serde_json::to_vec(&source).unwrap()).unwrap();
    for pass in 0..60 {
        supervisor.tick(5_700 + pass * 4).unwrap();
    }
    let goals = store
        .reconciliation_goals_for_agent(&harness.binding.agent_id)
        .unwrap();
    assert_eq!(
        goals, prior_goals,
        "the same admitted maintenance Goal retains its inception"
    );
    let authorizations: Vec<_> = goals
        .iter()
        .flat_map(|goal| {
            store
                .product_authorizations_for_goal(&goal.goal.goal_id)
                .unwrap()
        })
        .filter(|authorization| !prior_authorizations.contains(authorization))
        .collect();
    let mutations: Vec<_> = authorizations
        .iter()
        .filter(|authorization| {
            matches!(&authorization.product, AgentAuthorizedProduct::Task(task)
            if task.authority_requirements.contains(&APPLY.into()))
        })
        .collect();
    assert_eq!(
        mutations.len(),
        1,
        "changed exposure must authorize one new native mutation"
    );
    let mutation = mutations[0];
    let current = goals
        .iter()
        .find(|goal| goal.goal.goal_id == mutation.goal_id)
        .unwrap();
    assert!(!prior_authorizations
        .iter()
        .any(|prior| prior.product_id == mutation.product_id));
    let verifications: Vec<_> = authorizations
        .iter()
        .filter(|authorization| {
            matches!(&authorization.product,
            AgentAuthorizedProduct::Task(task)
                if task.authority_requirements.contains(&"dependency_security.verify".into())
                    && !task.authority_requirements.contains(&APPLY.into()))
        })
        .filter(|authorization| {
            store
                .reconciliation_plan(&authorization.plan_revision_id)
                .unwrap()
                .unwrap()
                .dependencies
                .iter()
                .any(|dependency| {
                    dependency.producer_product_id == mutation.product_id
                        && dependency.consumer_product_id == authorization.product_id
                        && dependency.required_milestone
                            == (PlanMilestoneRequirement::ExecutionTerminal {
                                task_id: mutation.product_id.clone(),
                            })
                })
        })
        .collect();
    assert_eq!(
        verifications.len(),
        1,
        "new mutation requires its own successor verification Task"
    );
    for goal in &goals {
        let plan = store
            .current_reconciliation_plan(&goal.goal.goal_id)
            .unwrap()
            .unwrap();
        let disposition = store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .expect("each native Goal must reach its own satisfaction decision");
        assert!(matches!(
            disposition.lifecycle,
            meld_lang::GoalLifecycle::Satisfied { .. }
        ));
        if goal == current {
            let milestones: Vec<_> = disposition
                .accepted_milestone_ids
                .iter()
                .map(|id| store.milestone(id).unwrap().unwrap())
                .collect();
            for task in [mutation, verifications[0]] {
                assert!(milestones
                    .iter()
                    .any(|milestone| milestone.product_id == task.product_id
                        && milestone.goal_id == current.goal.goal_id
                        && milestone.requirement
                            == (PlanMilestoneRequirement::ExecutionTerminal {
                                task_id: task.product_id.clone(),
                            })));
            }
        }
    }
    for prior in prior_authorizations {
        assert_eq!(
            store
                .product_authorization(&prior.authorization_id)
                .unwrap(),
            Some(prior)
        );
    }
    for prior in prior_dispositions {
        assert_eq!(
            store
                .goal_disposition_for_plan(&prior.plan_revision_id)
                .unwrap(),
            Some(prior)
        );
    }
    for file in &change.files {
        assert_eq!(
            std::fs::read_to_string(root.join(&file.relative_path)).unwrap(),
            file.replacement
        );
    }
    let records = harness
        .authority
        .replay_capability()
        .newest_page(1024)
        .unwrap()
        .records;
    for kind in [
        acquisition::EVENT,
        "code_change.intent.v1",
        "code_change.materialized.v1",
    ] {
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == kind)
                .count(),
            2,
            "{kind}"
        );
    }
    assert_eq!(
        records
            .iter()
            .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
            .count(),
        24
    );
    let network = assembly.task_network_reader().unwrap().snapshot().unwrap();
    let materialized_seq = records
        .iter()
        .find(|record| {
            record.seq > before_seq && record.event_type == "code_change.materialized.v1"
        })
        .unwrap()
        .seq;
    let mut returned_by_task = std::collections::BTreeMap::<String, usize>::new();
    for record in records.iter().filter(|record| {
        record.seq > before_seq && record.event_type == "dependency_security.invocation_return.v1"
    }) {
        let run = record
            .data
            .pointer("/binding/payload/upstream_lineage/task_run_id")
            .and_then(serde_json::Value::as_str)
            .unwrap();
        let node = network
            .tasks
            .values()
            .find(|node| node.task_run_context.task_run_id == run)
            .unwrap();
        let task_id = &node.lineage.admission.as_ref().unwrap().task_id;
        if task_id == &verifications[0].product_id {
            assert!(
                record.seq > materialized_seq,
                "verification observes the materialized effect"
            );
        } else {
            assert!(
                record.seq < materialized_seq,
                "assessment establishes the exposure before mutation"
            );
        }
        *returned_by_task.entry(task_id.clone()).or_default() += 1;
    }
    assert_eq!(returned_by_task.len(), 2, "new exposure requires a current assessment and a distinct post-mutation verification: {returned_by_task:?}");
    assert!(returned_by_task.values().all(|count| *count == 4));
    assert!(returned_by_task.contains_key(&verifications[0].product_id));
}

pub(super) fn declare(harness: &StewardshipHarness, original: &[u8]) {
    let root = harness._workspace.path();
    let replacements = [
        ("Cargo.toml", String::from_utf8(original.to_vec()).unwrap()),
        (
            "Cargo.lock",
            std::fs::read_to_string(root.join("Cargo.lock"))
                .unwrap()
                .replace("version = \"2.0.0\"", "version = \"1.0.0\""),
        ),
    ];
    let change = CodeChangeSet::new(
        harness.binding.subject.clone(),
        vec![meld_events::DomainObjectRef::new(
            "dependency-security",
            "advisory",
            "runtime-advisory",
        )
        .unwrap()],
        replacements
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
    .unwrap();
    std::fs::write(
        harness._external.path().join("code-proposal.json"),
        serde_json::to_vec(&change).unwrap(),
    )
    .unwrap();
}

pub(super) fn admit_coverage(
    harness: &StewardshipHarness,
    assembly: &ProductRuntimeAssembly,
    subject: meld_dependency_security_owner::dependency_security::contracts::DependencySecuritySubjectV1,
) {
    let store = &assembly.stores().agent_store;
    let goals = store
        .reconciliation_goals_for_agent(&harness.binding.agent_id)
        .unwrap();
    assert_eq!(goals.len(), 2);
    assert!(goals.iter().all(|goal| store.product_authorizations_for_goal(&goal.goal.goal_id).unwrap().iter().all(|authorization|
        !matches!(&authorization.product, meld_world_model::AgentAuthorizedProduct::Task(task) if task.authority_requirements.contains(&APPLY.into()))
    )), "missing inventory coverage must not authorize a declared write");
    let inventory = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(
            meld_dependency_security_owner::dependency_security::inventory::cargo::observe(
                harness._workspace.path(),
                Path::new(env!("CARGO")),
                subject,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                &Default::default(),
            ),
        )
        .unwrap();
    let path = harness._external.path().join("advisories.json");
    let mut source: meld_dependency_security_owner::dependency_security::advisory::AdvisorySourceDocumentV1 =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    source.source_revision = "coverage-for-current-and-repaired-inventory".into();
    for component in inventory.components {
        let coverage =
            meld_dependency_security_owner::dependency_security::contracts::ComponentCoverageV1 {
                package_name: component.package_name,
                source_identity: component.source_identity,
            };
        if !source.covered_components.contains(&coverage) {
            source.covered_components.push(coverage);
        }
    }
    std::fs::write(path, serde_json::to_vec(&source).unwrap()).unwrap();
}

pub(super) fn verify(
    harness: &StewardshipHarness,
    assembly: &ProductRuntimeAssembly,
    original: &[u8],
) {
    use meld_world_model::{strategy::PlanMilestoneRequirement, AgentAuthorizedProduct};
    let store = &assembly.stores().agent_store;
    let goals = store
        .reconciliation_goals_for_agent(&harness.binding.agent_id)
        .unwrap();
    assert_eq!(goals.len(), 2);
    let mut mutation = Vec::new();
    for goal in &goals {
        for authorization in store
            .product_authorizations_for_goal(&goal.goal.goal_id)
            .unwrap()
        {
            if matches!(&authorization.product, AgentAuthorizedProduct::Task(task) if task.authority_requirements.contains(&APPLY.into()))
            {
                mutation.push(authorization);
            }
        }
    }
    assert_eq!(
        mutation.len(),
        1,
        "native selection must authorize one declared intervention"
    );
    let mutation = &mutation[0];
    let history = store.completed_history_for_goal(&mutation.goal_id).unwrap();
    assert!(history
        .iter()
        .any(|entry| entry.product_id == mutation.product_id
            && entry.accepted_milestone
                == (PlanMilestoneRequirement::ExecutionTerminal {
                    task_id: mutation.product_id.clone()
                })
            && !entry.owner_position_id.is_empty()));
    let authorizations = store
        .product_authorizations_for_goal(&mutation.goal_id)
        .unwrap();
    let successors: Vec<_> = authorizations
        .iter()
        .filter(|authorization| {
            matches!(&authorization.product, AgentAuthorizedProduct::Task(task)
            if task.authority_requirements.contains(&"dependency_security.verify".into())
                && !task.authority_requirements.contains(&APPLY.into()))
        })
        .filter(|authorization| {
            let plan = store
                .reconciliation_plan(&authorization.plan_revision_id)
                .unwrap()
                .unwrap();
            plan.dependencies.iter().any(|dependency| {
                dependency.producer_product_id == mutation.product_id
                    && dependency.consumer_product_id == authorization.product_id
                    && dependency.required_milestone
                        == (PlanMilestoneRequirement::ExecutionTerminal {
                            task_id: mutation.product_id.clone(),
                        })
            })
        })
        .collect();
    assert_eq!(
        successors.len(),
        1,
        "one independent verification must retain the accepted mutation prerequisite"
    );
    for goal in &goals {
        let plan = store
            .current_reconciliation_plan(&goal.goal.goal_id)
            .unwrap()
            .unwrap();
        assert!(
            store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap()
                .is_some_and(|disposition| matches!(
                    disposition.lifecycle,
                    meld_lang::GoalLifecycle::Satisfied { .. }
                )),
            "Goal {} is not satisfied; origin {:?}; tasks {:?}",
            goal.goal.goal_id,
            plan.origin,
            plan.tasks
                .iter()
                .map(|task| &task.authority_requirements)
                .collect::<Vec<_>>()
        );
    }
    let final_plan = store
        .current_reconciliation_plan(&mutation.goal_id)
        .unwrap()
        .unwrap();
    let disposition = store
        .goal_disposition_for_plan(&final_plan.plan_revision_id)
        .unwrap()
        .unwrap();
    let accepted: Vec<_> = disposition
        .accepted_milestone_ids
        .iter()
        .map(|id| store.milestone(id).unwrap().unwrap())
        .collect();
    for product_id in [&mutation.product_id, &successors[0].product_id] {
        assert!(accepted
            .iter()
            .any(|milestone| &milestone.product_id == product_id
                && milestone.requirement
                    == (PlanMilestoneRequirement::ExecutionTerminal {
                        task_id: product_id.clone()
                    })));
    }
    let records = harness
        .authority
        .replay_capability()
        .newest_page(1024)
        .unwrap()
        .records;
    for event_type in [
        acquisition::EVENT,
        "code_change.intent.v1",
        "code_change.materialized.v1",
    ] {
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == event_type)
                .count(),
            1,
            "{event_type}"
        );
    }
    assert_eq!(
        records
            .iter()
            .filter(|record| record.event_type == "dependency_security.invocation_return.v1")
            .count(),
        16
    );
    assert_eq!(
        std::fs::read(harness._workspace.path().join("Cargo.toml")).unwrap(),
        original
    );
}

#[test]
fn native_mitigation_interruption_recovers_without_repeating_mutation() {
    assert_native_security_reconciliation(
        true,
        false,
        SecuritySourceAdvance::NativeMitigationRestart,
    );
}

pub(super) fn interrupt_after_materialization(
    harness: &StewardshipHarness,
    assembly: &ProductRuntimeAssembly,
    supervisor: &mut RuntimeSupervisor<'_>,
) -> meld_world_model::AgentProductAuthorization {
    for pass in 0..60 {
        supervisor.tick(5_100 + pass * 10).unwrap();
        let records = harness
            .authority
            .replay_capability()
            .newest_page(1024)
            .unwrap()
            .records;
        if records
            .iter()
            .any(|record| record.event_type == "code_change.materialized.v1")
        {
            assert_eq!(records.iter().filter(|record| record.event_type == "dependency_security.invocation_return.v1").count(), 12,
                "stop after mutation, before successor Security acquisition");
            let goals = assembly
                .stores()
                .agent_store
                .reconciliation_goals_for_agent(&harness.binding.agent_id)
                .unwrap();
            let mutation = goals.into_iter().flat_map(|goal|
                assembly.stores().agent_store.product_authorizations_for_goal(&goal.goal.goal_id).unwrap()
            ).find(|authorization| matches!(&authorization.product,
                meld_world_model::AgentAuthorizedProduct::Task(task) if task.authority_requirements.contains(&APPLY.into())
            )).expect("materialization must have a native Agent authorization");
            let plan = assembly
                .stores()
                .agent_store
                .current_reconciliation_plan(&mutation.goal_id)
                .unwrap()
                .unwrap();
            assert!(!assembly
                .stores()
                .agent_store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap()
                .is_some_and(|disposition| matches!(
                    disposition.lifecycle,
                    meld_lang::GoalLifecycle::Satisfied { .. }
                )));
            return mutation;
        }
    }
    panic!("native mitigation did not reach materialization");
}

pub(super) fn resume_after_interruption(
    harness: &StewardshipHarness,
    original: &[u8],
    mutation: &meld_world_model::AgentProductAuthorization,
) {
    let proposal = harness._external.path().join("code-proposal.json");
    std::fs::remove_file(&proposal).unwrap();
    let later = format!(
        "{}\n# later user edit, after materialization\n",
        std::fs::read_to_string(harness._workspace.path().join("Cargo.lock")).unwrap()
    );
    std::fs::write(harness._workspace.path().join("Cargo.lock"), &later).unwrap();
    let assembly = harness.assembly();
    harness.bind_production_routes(&assembly);
    let mut command = SupervisorStartCommand::new("interrupted-mitigation-successor", 7_000);
    command.registration_set = assembly.registration_set().cloned();
    let mut supervisor =
        RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
    for pass in 0..60 {
        supervisor.tick(7_100 + pass * 10).unwrap();
    }
    verify(harness, &assembly, original);
    let store = &assembly.stores().agent_store;
    assert_eq!(
        store
            .product_authorization(&mutation.authorization_id)
            .unwrap()
            .as_ref(),
        Some(mutation)
    );
    let accepted: Vec<_> = store
        .milestones_for_goal(&mutation.goal_id)
        .unwrap()
        .into_iter()
        .filter(|milestone| {
            milestone.product_id == mutation.product_id
                && matches!(
                    milestone.requirement,
                    meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal { .. }
                )
        })
        .collect();
    assert_eq!(accepted.len(), 1);
    assert_eq!(
        accepted[0].activation_generation,
        mutation.activation_generation
    );
    let RuntimeSemanticHandleFactory::AgentActor(agent) = &assembly
        .handle_factories()
        .get(AGENT_RECONCILIATION_RUNTIME_ID)
        .unwrap()
        .semantic
    else {
        unreachable!()
    };
    let current = agent.authority_port.observe().unwrap().unwrap();
    assert_ne!(
        current.activation_generation,
        mutation.activation_generation
    );
    let verification = store
        .product_authorizations_for_goal(&mutation.goal_id)
        .unwrap()
        .into_iter()
        .find(|authorization| {
            store
                .reconciliation_plan(&authorization.plan_revision_id)
                .unwrap()
                .unwrap()
                .dependencies
                .iter()
                .any(|dependency| {
                    dependency.producer_product_id == mutation.product_id
                        && dependency.consumer_product_id == authorization.product_id
                })
        })
        .expect("successor verification retains its mutation prerequisite");
    assert_eq!(
        verification.activation_generation,
        current.activation_generation
    );
    assert_eq!(verification.admission_epoch, current.admission_epoch);
    let plan = store
        .current_reconciliation_plan(&mutation.goal_id)
        .unwrap()
        .unwrap();
    let disposition = store
        .goal_disposition_for_plan(&plan.plan_revision_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        disposition.activation_generation,
        current.activation_generation
    );
    assert!(disposition
        .accepted_milestone_ids
        .contains(&accepted[0].milestone_id));

    assert_eq!(
        std::fs::read_to_string(harness._workspace.path().join("Cargo.lock")).unwrap(),
        later
    );
    assert!(!proposal.exists());
    supervisor.request_shutdown(8_000).unwrap();
}
