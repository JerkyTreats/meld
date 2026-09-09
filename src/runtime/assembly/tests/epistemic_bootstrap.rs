//! Ordinary scheduling must acquire the first judgment through an authorized observation.

use super::*;

#[test]
fn native_unassessed_question_authorizes_observation_before_first_belief() {
    use meld_world_model::{AgentAuthorizedProduct, CurationSelectionPosture};
    let harness = StewardshipHarness::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness");
    let package = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(&source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), package.path().join(entry.file_name())).unwrap();
        }
    }
    let mut template = workspace_fixture_curation_template();
    template.selection_posture = CurationSelectionPosture::PlannedOnly;
    let mut strategy: meld_world_model::strategy::StrategyTheoryPackage = serde_json::from_slice(
        &std::fs::read(source.join("strategy_theory.docs_freshness.json")).unwrap(),
    )
    .unwrap();
    strategy.snapshot.settlement_rules[0].construction =
        meld_world_model::strategy::StrategyConstruction::ObserveUnknown;
    strategy.snapshot.settlement_rules[0]
        .product_ordering
        .clear();
    strategy.capabilities.clear();
    strategy.methods.clear();
    strategy.requested_authority.clear();
    let mut mapping = standing_curation_outcome_mapping();
    mapping.rules[0].subject = OutcomeSubjectBinding {
        object_kind: "node".into(), domain_id: Some("workspace_fs".into()),
        from: meld_world_model::belief::outcome::interpretation::OutcomeSubjectSource::PayloadObjectId {
            pointer: "/semantic_publication/batch/objects/0/qualifications/judgment_subject_id".into(),
        },
    };
    let mut family: serde_json::Value = serde_json::from_slice(
        &std::fs::read(source.join("belief_family.docs_freshness.json")).unwrap(),
    )
    .unwrap();
    family["anchor_requirement"] = "Required".into();
    let replacements = [
        (
            "belief_family.docs_freshness.json",
            "docs_freshness".into(),
            serde_json::to_vec(&family).unwrap(),
        ),
        (
            "epistemic_rule.docs_freshness.json",
            template.rule_id.clone(),
            serde_json::to_vec(&template).unwrap(),
        ),
        (
            "strategy_theory.docs_freshness.json",
            strategy.snapshot.theory_id.clone(),
            serde_json::to_vec(&strategy).unwrap(),
        ),
        (
            "outcome_interpretation.docs_freshness.json",
            mapping.mapping_id.clone(),
            serde_json::to_vec(&mapping).unwrap(),
        ),
    ];
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source.join("pds-package.json")).unwrap()).unwrap();
    for (path, id, bytes) in replacements {
        std::fs::write(package.path().join(path), &bytes).unwrap();
        let component = manifest["components"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["content"]["path"] == path)
            .unwrap();
        component["owner_component_id"] = id.into();
        component["content"]["content_hash"] = blake3::hash(&bytes).to_hex().to_string().into();
    }
    std::fs::write(
        package.path().join("pds-package.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    {
        let assembly = harness.assembly();
        harness.run_world_genesis_from(&assembly, package.path());
        assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(workspace_owner_publication(
                stewardship_subject_ref(&harness.binding).unwrap(),
                "initial-source",
            ))
            .unwrap();
        assembly.flush_product_boundary().unwrap();
    }
    let assembly = harness.assembly();
    let mut supervisor = harness.start_supervisor(&assembly);
    let store = &assembly.stores().agent_store;
    let mut reports = Vec::new();
    for pass in 0..36 {
        reports.push(supervisor.tick(1_100 + pass * 100).unwrap());
        let goals = store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap();
        if goals.iter().any(|goal| {
            store
                .current_reconciliation_plan(&goal.goal.goal_id)
                .unwrap()
                .is_some_and(|plan| {
                    store
                        .goal_disposition_for_plan(&plan.plan_revision_id)
                        .unwrap()
                        .is_some_and(|d| {
                            matches!(d.lifecycle, meld_lang::GoalLifecycle::Satisfied { .. })
                        })
                })
        }) {
            break;
        }
    }
    let goals = store
        .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
        .unwrap();
    assert_eq!(goals.len(), 1, "{reports:#?}");
    let goal = &goals[0].goal.goal_id;
    let authorizations = store.product_authorizations_for_goal(goal).unwrap();
    assert_eq!(authorizations.len(), 1, "{authorizations:#?}; {reports:#?}");
    assert!(
        matches!(
            authorizations[0].product,
            AgentAuthorizedProduct::Epistemic(_)
        ),
        "{authorizations:#?}"
    );
    let first_plan = store
        .reconciliation_plan(&authorizations[0].plan_revision_id)
        .unwrap()
        .unwrap();
    let first_cut = store
        .reconciliation_cut(&first_plan.planner_cut_id)
        .unwrap()
        .unwrap();
    let unanswered = first_cut
        .world_model_view
        .unassessed_belief
        .as_ref()
        .expect("first acquisition must retain Belief's unanswered exact question");
    assert!(first_cut
        .source_positions
        .iter()
        .all(|source| source.kind != meld_world_model::PlannerSourceKind::Belief));
    let revisions = assembly
        .stores()
        .belief_store
        .revision_history(&unanswered.key)
        .unwrap();
    assert!(!revisions.is_empty());
    let plan = store.current_reconciliation_plan(goal).unwrap().unwrap();
    assert!(
        store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .is_some_and(|d| matches!(d.lifecycle, meld_lang::GoalLifecycle::Satisfied { .. })),
        "{reports:#?}"
    );
    let records = assembly
        .ports()
        .event_replay()
        .read_after_limit(0, 512)
        .unwrap();
    let results: Vec<_> = records
        .iter()
        .filter(|r| r.envelope.event_type == meld_world_model::CURATION_RESULT_EVENT_TYPE)
        .collect();
    assert_eq!(results.len(), 1);
    let result: meld_world_model::CurationResult =
        serde_json::from_value(results[0].envelope.data.clone()).unwrap();
    let operation = meld_world_model::CurationQuery::new(&assembly.stores().curation_store)
        .operation(&result.operation_id)
        .unwrap()
        .unwrap();
    assert!(
        operation.request_id.is_some(),
        "automatic Curation must not hide initial Agent acquisition"
    );
    assert!(assembly
        .stores()
        .curation_store
        .planned_authorization_for_operation(&operation.operation_id)
        .unwrap()
        .is_some());
    supervisor.request_shutdown(6_000).unwrap();
}

#[test]
fn planned_nonce_observation_progresses_from_acquisition_through_confirmation() {
    let harness = startup_harness();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/startup");
    let package = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(&source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), package.path().join(entry.file_name())).unwrap();
        }
    }
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source.join("pds-package.json")).unwrap()).unwrap();
    for name in [
        "belief_family.startup_realization.json",
        "epistemic_rule.startup.json",
        "strategy_theory.startup.json",
    ] {
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(source.join(name)).unwrap()).unwrap();
        match name {
            "belief_family.startup_realization.json" => {
                value["anchor_requirement"] = "Required".into()
            }
            "epistemic_rule.startup.json" => value["selection_posture"] = "planned_only".into(),
            _ => {
                let mut acquisition = value["snapshot"]["settlement_rules"][0].clone();
                acquisition["construction"] = "observe_unknown".into();
                acquisition["product_ordering"] = serde_json::json!([]);
                value["snapshot"]["settlement_rules"]
                    .as_array_mut()
                    .unwrap()
                    .insert(0, acquisition);
            }
        }
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(package.path().join(name), &bytes).unwrap();
        let component = manifest["components"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|component| component["content"]["path"] == name)
            .unwrap();
        component["content"]["content_hash"] = blake3::hash(&bytes).to_hex().to_string().into();
    }
    std::fs::write(
        package.path().join("pds-package.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    {
        let assembly = harness.assembly();
        harness.run_world_genesis_from(&assembly, package.path());
    }
    let assembly = harness.assembly();
    harness.bind_production_routes(&assembly);
    let mut supervisor = harness.start_supervisor(&assembly);
    let mut reports = Vec::new();
    let store = &assembly.stores().agent_store;
    let mut satisfied = false;
    for pass in 0..48 {
        reports.push(supervisor.tick(1_100 + pass * 100).unwrap());
        if store
            .reconciliation_goals_for_agent("startup-agent")
            .unwrap()
            .iter()
            .any(|goal| {
                store
                    .current_reconciliation_plan(&goal.goal.goal_id)
                    .unwrap()
                    .is_some_and(|plan| {
                        store
                            .goal_disposition_for_plan(&plan.plan_revision_id)
                            .unwrap()
                            .is_some_and(|d| {
                                matches!(d.lifecycle, meld_lang::GoalLifecycle::Satisfied { .. })
                            })
                    })
            })
        {
            satisfied = true;
            break;
        }
    }
    assert!(satisfied, "planned nonce did not settle: {reports:#?}");
    let goals = store
        .reconciliation_goals_for_agent("startup-agent")
        .unwrap();
    assert_eq!(goals.len(), 1);
    let authorizations = store
        .product_authorizations_for_goal(&goals[0].goal.goal_id)
        .unwrap();
    assert_eq!(authorizations.len(), 3, "{authorizations:#?}");
    let mut acquisition = 0;
    let mut confirmation = 0;
    let mut tasks = 0;
    for authorization in &authorizations {
        let plan = store
            .reconciliation_plan(&authorization.plan_revision_id)
            .unwrap()
            .unwrap();
        let cut = store
            .reconciliation_cut(&plan.planner_cut_id)
            .unwrap()
            .unwrap();
        match &authorization.product {
            meld_world_model::AgentAuthorizedProduct::Epistemic(_) => {
                if cut.world_model_view.unassessed_belief.is_some() {
                    acquisition += 1;
                    assert!(cut.world_model_view.hydration_refs.revision_ids.is_empty());
                } else {
                    confirmation += 1;
                    assert!(cut.world_model_view.pending_derived_evidence.is_some());
                    assert!(!cut.world_model_view.hydration_refs.revision_ids.is_empty());
                    assert!(plan.tasks.is_empty());
                }
            }
            meld_world_model::AgentAuthorizedProduct::Task(_) => {
                tasks += 1;
                assert!(cut.world_model_view.pending_derived_evidence.is_none());
                assert!(cut.world_model_view.unassessed_belief.is_none());
            }
        }
    }
    assert_eq!((acquisition, tasks, confirmation), (1, 1, 1));
    let events = assembly
        .ports()
        .event_replay()
        .read_after_limit(0, 128)
        .unwrap();
    let nonce = events
        .iter()
        .find(|event| event.envelope.event_type == crate::nonce::EVENT_TYPE)
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.envelope.event_type == crate::nonce::EVENT_TYPE)
            .count(),
        1
    );
    let observations: Vec<_> = events
        .iter()
        .filter(|event| event.envelope.event_type == meld_world_model::CURATION_RESULT_EVENT_TYPE)
        .collect();
    assert_eq!(observations.len(), 2);
    assert!(observations[0].seq < nonce.seq && nonce.seq < observations[1].seq);
}
