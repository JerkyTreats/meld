//! Prerequisite publication and consumer visibility are separate durable boundaries.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

struct PausedCurationPublication {
    inner: ProductEventAppendPort,
    paused: AtomicBool,
}

impl CurationEventPort for PausedCurationPublication {
    fn watermark(&self) -> Result<meld_events::EventWatermark, String> {
        self.inner.watermark().map_err(|error| error.to_string())
    }

    fn append_idempotent(
        &self,
        envelope: meld_events::EventEnvelope,
    ) -> Result<meld_events::AppendReceipt, String> {
        if self.paused.load(Ordering::SeqCst) {
            return Err("injected publication interruption after result persistence".into());
        }
        self.inner.append_idempotent(envelope)
    }
}

struct FailedPrerequisiteTraversal(ProductCurationTraversalPort);

impl CurationTraversalPort for FailedPrerequisiteTraversal {
    fn cut(
        &self,
        request: &meld_world_model::world_state::graph::contracts::TraversalCutRequest,
    ) -> Result<
        meld_world_model::world_state::graph::contracts::TraversalCut,
        meld_world_model::error::StorageError,
    > {
        self.0.cut(request)
    }
    fn traverse(
        &self,
        _cut: &meld_world_model::world_state::graph::contracts::TraversalCut,
        _request: &meld_world_model::world_state::graph::contracts::BoundedTraversalRequest,
    ) -> Result<
        meld_world_model::world_state::graph::contracts::TraversalResult,
        meld_world_model::error::StorageError,
    > {
        Err(meld_world_model::error::StorageError::InvalidPath(
            "injected prerequisite traversal failure".into(),
        ))
    }
}

#[test]
fn failed_prerequisite_publication_does_not_enable_dependent_work() {
    assert_prerequisite_return(true, false);
}

#[test]
fn prerequisite_result_cannot_authorize_task_before_publication_and_graph_visibility() {
    assert_prerequisite_return(false, false);
}

#[test]
fn closed_epoch_absorbs_prerequisite_visibility_without_reopening_task_authority() {
    assert_prerequisite_return(false, true);
}

fn assert_prerequisite_return(fail_traversal: bool, close_epoch: bool) {
    let harness = StewardshipHarness::new();
    let subject = stewardship_subject_ref(&harness.binding).unwrap();
    {
        let assembly = harness.assembly();
        harness.run_workspace_fixture_genesis(&assembly);
        assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(workspace_owner_publication(subject, "workspace-v1"))
            .unwrap();
        assembly
            .graph_runtime()
            .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
            .unwrap();
        assembly.flush_product_boundary().unwrap();
    }
    let assembly = harness.assembly();
    harness.bind_production_routes(&assembly);
    let _supervisor = harness.start_supervisor(&assembly);
    let mut belief = assembly
        .handle_factories()
        .get("world_model.belief_assessment")
        .unwrap()
        .build_handle();
    belief
        .start_after_lease(RuntimeLeaseContext {
            runtime_id: "world_model.belief_assessment".into(),
            lease_id: "prerequisite-belief".into(),
        })
        .unwrap();
    let report = belief.tick(WorkBudget { max_items: 8 }).unwrap();
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    let mut agent = assembly
        .handle_factories()
        .get(AGENT_RECONCILIATION_RUNTIME_ID)
        .unwrap()
        .build_handle();
    agent
        .start_after_lease(RuntimeLeaseContext {
            runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.into(),
            lease_id: "prerequisite-agent".into(),
        })
        .unwrap();
    for _ in 0..8 {
        let report = agent.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(report.fatal_errors.is_empty(), "{report:?}");
        if assembly
            .stores()
            .curation_store
            .next_planned_operation(STEWARD_AGENT_ID)
            .unwrap()
            .is_some()
        {
            break;
        }
    }
    let operation = assembly
        .stores()
        .curation_store
        .next_planned_operation(STEWARD_AGENT_ID)
        .unwrap()
        .unwrap();
    let goal_id = operation
        .planned_authorization
        .as_ref()
        .unwrap()
        .goal_id
        .clone();
    let RuntimeSemanticHandleFactory::StandingCuration(factory) = &assembly
        .handle_factories()
        .get("world_model.standing_curation")
        .unwrap()
        .semantic
    else {
        unreachable!()
    };
    let events = Arc::new(PausedCurationPublication {
        inner: factory.events.clone(),
        paused: AtomicBool::new(true),
    });
    let build_curation = || {
        StandingCurationActor::new(
            factory.runtime_id.clone(),
            factory.session_id.clone(),
            factory.authority.clone(),
            factory.rule.clone(),
            Arc::clone(&factory.store),
            if fail_traversal {
                Arc::new(FailedPrerequisiteTraversal(factory.traversal.clone()))
                    as Arc<dyn CurationTraversalPort>
            } else {
                Arc::new(factory.traversal.clone())
            },
            events.clone(),
        )
        .unwrap()
        .with_authority_port(Arc::clone(&factory.authority_port))
    };
    let curation = build_curation();
    let report = curation.bounded_step(1);
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    let result = factory
        .store
        .result_for_operation(&operation.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        result.disposition,
        if fail_traversal {
            meld_world_model::CurationTerminalDisposition::Failed
        } else {
            meld_world_model::CurationTerminalDisposition::Applied
        }
    );
    assert!(factory
        .store
        .publication_receipt(
            &result.result_id,
            meld_world_model::CurationPublicationKind::Terminal
        )
        .unwrap()
        .is_none());
    let no_task = |phase: &str| {
        let report = agent.tick(WorkBudget { max_items: 16 }).unwrap();
        assert!(report.fatal_errors.is_empty(), "{phase}: {report:?}");
        let grants = assembly
            .stores()
            .agent_store
            .product_authorizations_for_goal(&goal_id)
            .unwrap();
        assert!(!grants.iter().any(|grant| matches!(grant.product, meld_world_model::AgentAuthorizedProduct::Task(_))),
            "{phase}: producer terminal result authorized dependent work without consumer visibility");
    };
    let mut no_task = no_task;
    no_task("publication missing");
    if close_epoch {
        let prepared = assembly.prepared_activation().unwrap();
        assembly
            .lifecycle_store()
            .unwrap()
            .begin_drain(
                &prepared.assignment.assignment_id,
                &operation.authority.activation_generation,
            )
            .unwrap();
        no_task("admission closed while publication is pending");
    }
    drop(curation);
    events.paused.store(false, Ordering::SeqCst);
    let curation = build_curation();
    let report = curation.bounded_step(1);
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert!(factory
        .store
        .publication_receipt(
            &result.result_id,
            meld_world_model::CurationPublicationKind::Terminal
        )
        .unwrap()
        .is_some());
    no_task("Graph has not consumed the publication");
    if close_epoch {
        assembly
            .graph_runtime()
            .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
            .unwrap();
        no_task("closed epoch may absorb the return but cannot authorize a Task");
        let history = assembly
            .stores()
            .agent_store
            .completed_history_for_goal(&goal_id)
            .unwrap();
        assert!(history.iter().any(|entry| entry.owner_position_id == result.result_id
            && matches!(&entry.accepted_milestone, meld_world_model::strategy::PlanMilestoneRequirement::CurationVisible { operation_id } if operation_id == &operation.operation_id)),
            "closed admission stranded a durably published prerequisite return");
        let original_plan = assembly
            .stores()
            .agent_store
            .reconciliation_plan(
                &operation
                    .planned_authorization
                    .as_ref()
                    .unwrap()
                    .plan_revision_id,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            assembly
                .stores()
                .agent_store
                .reconciliation_cut(&original_plan.planner_cut_id)
                .unwrap()
                .unwrap()
                .traversal_cut,
            operation.source_cut
        );
        let before = assembly.stores().agent_store.reconciliation_position();
        no_task("repeated closed-epoch observation");
        assert_eq!(
            assembly.stores().agent_store.reconciliation_position(),
            before
        );
        return;
    }
    if fail_traversal {
        for _ in 0..4 {
            assembly
                .graph_runtime()
                .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
                .unwrap();
            no_task("failed prerequisite has been published and consumed");
        }
        assert!(!assembly
            .stores()
            .agent_store
            .completed_history_for_goal(&goal_id)
            .unwrap()
            .iter()
            .any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::CurationVisible { .. }
            )));
        assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(workspace_owner_publication(
                operation.authority.subject.clone(),
                "workspace-after-prerequisite-failure",
            ))
            .unwrap();
        let mut healthy = assembly
            .handle_factories()
            .get("world_model.standing_curation")
            .unwrap()
            .build_handle();
        healthy
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.standing_curation".into(),
                lease_id: "recovered-prerequisite".into(),
            })
            .unwrap();
        for _ in 0..12 {
            assembly
                .graph_runtime()
                .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
                .unwrap();
            belief.tick(WorkBudget { max_items: 8 }).unwrap();
            let report = agent.tick(WorkBudget { max_items: 16 }).unwrap();
            assert!(report.fatal_errors.is_empty(), "{report:?}");
            assert!(report.retryable_errors.is_empty(), "{report:?}");
            let grants = assembly
                .stores()
                .agent_store
                .product_authorizations_for_goal(&goal_id)
                .unwrap();
            assert_eq!(grants.iter().filter(|grant| matches!(&grant.product,
                meld_world_model::AgentAuthorizedProduct::Epistemic(product) if product.operation.operation_id == operation.operation_id)).count(), 1);
            if grants.iter().any(|grant| {
                matches!(
                    grant.product,
                    meld_world_model::AgentAuthorizedProduct::Task(_)
                )
            }) {
                let RuntimeSemanticHandle::AgentActor(native) = &agent.semantic else {
                    unreachable!()
                };
                let expected_pending: Vec<_> = grants
                    .iter()
                    .filter(|grant| {
                        matches!(
                            grant.product,
                            meld_world_model::AgentAuthorizedProduct::Task(_)
                        )
                    })
                    .collect();
                let expected = format!(
                    "agent-unresolved-products::{}",
                    blake3::hash(&serde_json::to_vec(&expected_pending).unwrap()).to_hex()
                );
                assert_eq!(
                    native
                        .actor
                        .lifecycle_evidence()
                        .unwrap()
                        .unresolved_operation_summary_ref,
                    expected,
                    "the returned failed prerequisite is still reported as unfinished work"
                );
                return;
            }
            let report = healthy.tick(WorkBudget { max_items: 1 }).unwrap();
            assert!(report.fatal_errors.is_empty(), "{report:?}");
        }
        panic!("changed source knowledge did not recover the failed prerequisite");
    }
    for _ in 0..8 {
        assembly
            .graph_runtime()
            .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
            .unwrap();
        let report = belief.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(report.fatal_errors.is_empty(), "{report:?}");
        let report = agent.tick(WorkBudget { max_items: 16 }).unwrap();
        assert!(report.fatal_errors.is_empty(), "{report:?}");
        if assembly
            .stores()
            .agent_store
            .product_authorizations_for_goal(&goal_id)
            .unwrap()
            .iter()
            .any(|grant| {
                matches!(
                    grant.product,
                    meld_world_model::AgentAuthorizedProduct::Task(_)
                )
            })
        {
            let grants = assembly
                .stores()
                .agent_store
                .product_authorizations_for_goal(&goal_id)
                .unwrap();
            assert_eq!(
                grants.len(),
                2,
                "the prerequisite's own publication must not authorize a second Curation"
            );
            let original_plan = assembly
                .stores()
                .agent_store
                .reconciliation_plan(
                    &operation
                        .planned_authorization
                        .as_ref()
                        .unwrap()
                        .plan_revision_id,
                )
                .unwrap()
                .unwrap();
            let current = assembly
                .stores()
                .agent_store
                .current_reconciliation_plan(&goal_id)
                .unwrap()
                .unwrap();
            assert_ne!(current.planner_cut_id, original_plan.planner_cut_id);
            assert_eq!(
                current.predecessor_plan_revision_id.as_ref(),
                Some(&original_plan.plan_revision_id)
            );
            let history = assembly
                .stores()
                .agent_store
                .reconciliation_plan_history(&current.plan_revision_id)
                .unwrap();
            assert!(history.iter().any(|entry| entry.owner_position_id == result.result_id
                && matches!(&entry.accepted_milestone, meld_world_model::strategy::PlanMilestoneRequirement::CurationVisible { operation_id } if operation_id == &operation.operation_id)));
            let cut = assembly
                .stores()
                .agent_store
                .reconciliation_cut(&current.planner_cut_id)
                .unwrap()
                .unwrap();
            let query = meld_world_model::CurationQuery::new(&factory.store);
            let graph = meld_world_model::world_state::graph::query::TraversalQuery::new(
                assembly.stores().traversal_store.opened().unwrap(),
            );
            assert!(query
                .prerequisite_visibility(&operation.operation_id, &cut.traversal_cut, &graph)
                .unwrap()
                .is_some());
            assert!(
                query
                    .prerequisite_visibility(&operation.operation_id, &operation.source_cut, &graph)
                    .unwrap()
                    .is_none(),
                "later publication cannot rewrite the original frozen cut"
            );
            let mut foreign = cut.traversal_cut.clone();
            foreign
                .receipts
                .iter_mut()
                .find(|receipt| receipt.owner_id == "curation")
                .unwrap()
                .revision_id = "foreign-result".into();
            assert!(
                !matches!(
                    query.prerequisite_visibility(&operation.operation_id, &foreign, &graph),
                    Ok(Some(_))
                ),
                "tampered owner evidence cannot establish a foreign revision"
            );
            let other_db = sled::Config::new().temporary(true).open().unwrap();
            let other_graph =
                meld_world_model::world_state::graph::store::TraversalStore::new(other_db).unwrap();
            assert!(
                !matches!(
                    query.prerequisite_visibility(
                        &operation.operation_id,
                        &cut.traversal_cut,
                        &meld_world_model::world_state::graph::query::TraversalQuery::new(
                            &other_graph
                        )
                    ),
                    Ok(Some(_))
                ),
                "the exact cut cannot be borrowed by another Graph instance"
            );
            assembly.flush_product_boundary().unwrap();
            drop(agent);
            let mut restarted = assembly
                .handle_factories()
                .get(AGENT_RECONCILIATION_RUNTIME_ID)
                .unwrap()
                .build_handle();
            restarted
                .start_after_lease(RuntimeLeaseContext {
                    runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.into(),
                    lease_id: "prerequisite-restarted".into(),
                })
                .unwrap();
            let report = restarted.tick(WorkBudget { max_items: 16 }).unwrap();
            assert!(report.fatal_errors.is_empty(), "{report:?}");
            assert_eq!(
                assembly
                    .stores()
                    .agent_store
                    .product_authorizations_for_goal(&goal_id)
                    .unwrap(),
                grants
            );
            return;
        }
    }
    panic!("published and Graph-visible prerequisite did not enable a current Task");
}
