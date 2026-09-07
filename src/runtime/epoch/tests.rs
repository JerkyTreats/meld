use super::*;
use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};
use meld_lang::*;
use meld_world_model::agent::*;
use meld_world_model::belief::{BeliefFamilyRegistry, BeliefFamilyRegistryStore, BranchScope};
use meld_world_model::curation::*;
use meld_world_model::planner::*;
use meld_world_model::world_state::graph::contracts::*;
use std::sync::Mutex;

struct Authority(Mutex<Option<AgentAuthorizationFence>>);
impl AgentAuthorityPort for Authority {
    fn observe(&self) -> Result<Option<AgentAuthorizationFence>, StorageError> {
        Ok(self.0.lock().unwrap().clone())
    }
}
struct NoPlanner;
struct CurationEpoch {
    observer: Arc<Authority>,
    base: CurationAuthority,
}
impl CurationAuthorityPort for CurationEpoch {
    fn observe(&self) -> Result<Option<CurationAuthority>, StorageError> {
        Ok(self.observer.observe()?.map(|fence| {
            let mut authority = self.base.clone();
            authority.activation_generation = fence.activation_generation;
            authority.admission_epoch = fence.admission_epoch;
            authority
        }))
    }
}
impl AgentPlannerPort for NoPlanner {
    fn assemble(&self) -> PlannerAssemblyOutcome {
        panic!("must select epoch products")
    }
}
struct NoExecution;
impl AgentExecutionPort for NoExecution {
    fn submit(
        &self,
        _: &AgentProductAuthorization,
    ) -> Result<AgentExecutionPosition, StorageError> {
        panic!("preparation must not execute")
    }
    fn advance(
        &self,
        _: &AgentProductAuthorization,
    ) -> Result<AgentExecutionPosition, StorageError> {
        panic!("preparation must not execute")
    }
}

#[test]
fn native_epoch_specification_drives_curation_and_preserves_distinct_observations() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let events = EventAuthority::open(
        sled::Config::new().temporary(true).open().unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let agent_store = Arc::new(AgentStore::new(db.clone()).unwrap());
    let curation = Arc::new(CurationStore::new(db.clone()).unwrap());
    let traversal = Arc::new(TraversalStore::new(db.clone()).unwrap());
    traversal
        .install_owner_event_route(&crate::nonce::graph_route())
        .unwrap();
    let template = curation
        .install_template(
            CurationRuleTemplate {
                coverage: None,
                rule_id: "confirm-nonce".into(),
                source_owner_id: crate::nonce::OWNER_ID.into(),
                traversal_direction: TraversalDirection::Incoming,
                bounds: TraversalBounds {
                    max_depth: 4,
                    max_objects: 32,
                    max_occurrences: 32,
                    max_paths: 32,
                },
                expected_object_kind: "expected_nonce".into(),
                expected_object_key: "required_emission".into(),
                relation_type: "expects".into(),
                output_policy_revision: "nonce-observation-v1".into(),
                realization: Some(CurationRealizationTemplate {
                    observed_object: CurationObjectSelection::SourceRoot,
                    required_qualifications: Default::default(),
                    realized_relation_type: "realizes".into(),
                    not_realized_relation_type: "not_realized".into(),
                }),
            },
            1,
        )
        .unwrap();
    let (_, condition) = AgentMaintainedConditionRegistryStore::new(db.clone())
        .unwrap()
        .install(
            AgentMaintainedCondition {
                condition_id: "startup-complete".into(),
                dimension_id: "startup_realization".into(),
                desired: Condition::Above(Term::Literal(Literal::Number(0.7))),
                goal_priority: GoalPriority {
                    urgency: 1,
                    cost_ceiling: None,
                },
                desired_summary: "this epoch is confirmed".into(),
                observation_scope: AgentObservationScope::AdmissionEpoch,
            },
            1,
        )
        .unwrap();
    let condition = condition.binding().unwrap();
    let subject = DomainObjectRef::new("runtime", "subject", "primary").unwrap();
    let authority = CurationAuthority {
        agent_id: "startup-agent".into(),
        subject: subject.clone(),
        perspective: PerspectiveKey::new("agent", "startup").unwrap(),
        branch_scope: BranchScope::main(),
        activation_generation: "generation".into(),
        admission_epoch: Some("epoch-one".into()),
    };
    let mut belief_registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
    let family = belief_registry
        .install(
            serde_json::from_str(include_str!(
                "../../../theory/startup/belief_family.startup_realization.json"
            ))
            .unwrap(),
            1,
        )
        .unwrap()
        .1;
    let belief_registry = Arc::new(belief_registry);
    let belief_store = Arc::new(meld_world_model::BeliefStore::new(db.clone()).unwrap());
    let subscriptions = Arc::new(meld_world_model::belief::BeliefSubscriptionSource::new(
        belief_store.clone(),
        belief_registry.clone(),
    ));
    let subscription = AgentSubscriptionRequestV1::new(
        authority.agent_id.clone(),
        "belief".into(),
        family.revision_ref(),
        meld_world_model::belief::BeliefKey {
            subject: subject.clone(),
            dimension_id: "startup_realization".into(),
            predicate_id: "realized".into(),
            perspective: authority.perspective.clone(),
            branch_scope: authority.branch_scope.clone(),
            evidence_policy_id: "startup-realization".into(),
        },
        "from_genesis".into(),
    )
    .unwrap();
    let genesis = AgentGenesisIntentV1::new(
        "assignment".into(),
        "compilation".into(),
        "startup-position".into(),
        vec![
            condition.revision.clone(),
            template.revision_ref(),
            subscription.source_contract_revision.clone(),
        ],
        SeedAgentRegistration {
            agent_id: authority.agent_id.clone(),
            subject: subject.clone(),
            perspective_key: authority.perspective.clone(),
            branch_scope: authority.branch_scope.clone(),
            observation_scope: "runtime-observation".into(),
            directive: "confirm current epoch".into(),
            seed_provenance: "test".into(),
            curation_rule: None,
            curation_rule_revision: None,
            maintained_condition: Some(condition.clone()),
            maintained_condition_revision: Some(condition.revision.clone()),
            created_at_seq: 1,
        },
        vec![subscription],
    )
    .unwrap();
    AgentGenesis::new(&agent_store, &events.append_capability())
        .prepare(genesis)
        .unwrap();
    let mut strategy: StrategyTheoryPackage = serde_json::from_str(include_str!(
        "../../../theory/docs_freshness/strategy_theory.docs_freshness.json"
    ))
    .unwrap();
    let mut operator = strategy.capabilities[0].operator.clone();
    operator.operator_id = "emit-requested-nonce".into();
    operator.resolution.requires_inputs = vec![SlotConstraint {
        artifact_type: Term::ArtifactType(crate::nonce::capability::REQUEST.into()),
        required: true,
    }];
    operator.resolution.requires_outputs = vec![SlotConstraint {
        artifact_type: Term::ArtifactType(crate::nonce::capability::RECEIPT.into()),
        required: true,
    }];
    operator.resolution.scope_kind = Some("domain_object".into());
    operator.resolution.specific = Some(CapabilityRef {
        capability_type_id: crate::nonce::capability::EMIT.into(),
        capability_version: 1,
    });
    strategy.capabilities = vec![meld_world_model::strategy::StrategyCapability {
        contract_id: crate::nonce::capability::contract().content_identity(),
        operator,
        outcome_contract_id: "nonce.emitted.v1".into(),
    }];
    strategy.snapshot.settlement_rules[0]
        .evidence_route
        .outcome_contract_id = "nonce.emitted.v1".into();
    let preparation = Arc::new(
        ProductNonceEpochPreparation::new(
            curation.clone(),
            traversal.clone(),
            template.revision_ref(),
            &strategy,
            events.watermark_capability(),
        )
        .unwrap(),
    );
    let fence = AgentAuthorizationFence {
        activation_generation: authority.activation_generation.clone(),
        admission_epoch: authority.admission_epoch.clone(),
        authority_policy_content_hash: "authority-policy".into(),
    };
    let observer = Arc::new(Authority(Mutex::new(Some(fence.clone()))));
    let intent = AgentReconciliationIntent::MaintainedCondition(condition);
    let runtime_strategy = AgentStrategyRuntimeConfig::activate_installed(
        strategy,
        subject.clone(),
        &authority.agent_id,
    )
    .unwrap();
    let build = |store: Arc<AgentStore>| {
        AgentReconciliationActor::new(
            "startup-agent-driver",
            intent.clone(),
            store,
            Arc::new(NoPlanner),
            observer.clone(),
            fence.clone(),
            Arc::new(crate::runtime::ports::ProductPlannedCurationPort::new(
                curation.clone(),
                traversal.clone(),
            )),
            Arc::new(NoExecution),
            runtime_strategy.clone(),
            authority.clone(),
            AgentPreparation::Epoch {
                products: preparation.clone(),
                subscriptions: subscriptions.clone(),
            },
        )
        .unwrap()
    };
    let before = crate::runtime::ports::ProductEventAppendPort::new(&events)
        .watermark()
        .unwrap();
    let first = build(agent_store.clone()).bounded_step(8);
    assert_eq!(first.fatal_errors.len(), 1, "{first:?}");
    assert!(first.retryable_errors.is_empty(), "{first:?}");
    assert_eq!(first.products_authorized, 0);
    assert!(
        first.waiting_on.is_empty(),
        "an unsupported Planner must fail rather than advertise a viable wake"
    );
    let goal_id = intent.goal_id(&authority.agent_id, &fence.reconciliation_scope());
    let products = agent_store.epoch_products(&goal_id).unwrap().unwrap();
    let request: crate::nonce::NonceRequest =
        serde_json::from_value(products.task_inputs[0].content.clone()).unwrap();
    request.validate().unwrap();
    assert_eq!(request.subject_ref, subject);
    assert_eq!(
        request.correlation_refs,
        products.specification.effect_correlations()
    );
    assert_eq!(
        products.observation_subject,
        products.curation_rule.rule.expected_object().unwrap()
    );
    assert_eq!(
        products.curation_rule.rule.roots,
        vec![request.object_ref().unwrap()]
    );
    assert_eq!(
        products.curation_rule.rule.scope,
        request.publication_scope()
    );
    assert!(agent_store.reconciliation_goal(&goal_id).unwrap().is_none());
    let replay = build(Arc::new(AgentStore::new(db.clone()).unwrap())).bounded_step(8);
    assert_eq!(replay.records_persisted, 0, "{replay:?}");
    let mut next = fence.clone();
    next.admission_epoch = Some("epoch-two".into());
    *observer.0.lock().unwrap() = Some(next.clone());
    let next_report = build(agent_store.clone()).bounded_step(8);
    assert_eq!(next_report.fatal_errors.len(), 1, "{next_report:?}");
    assert!(next_report.retryable_errors.is_empty(), "{next_report:?}");
    let next_id = intent.goal_id(&authority.agent_id, &next.reconciliation_scope());
    let next = agent_store.epoch_products(&next_id).unwrap().unwrap();
    let next_request: crate::nonce::NonceRequest =
        serde_json::from_value(next.task_inputs[0].content.clone()).unwrap();
    assert_ne!(request.nonce_id, next_request.nonce_id);
    assert_ne!(
        request.expected_event_id().unwrap(),
        next_request.expected_event_id().unwrap()
    );
    assert_ne!(products.observation_subject, next.observation_subject);
    use crate::runtime::ports::{
        ProductAgentPlannerPort, ProductEventAppendPort, ProductEventReplayPort,
        ProductGraphCursorPort,
    };
    use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
    let graph = GraphRuntime::from_ports(
        Arc::new(ProductEventReplayPort::new(events.replay_capability())),
        Arc::new(ProductEventAppendPort::new(&events)),
        Arc::new(ProductGraphCursorPort::new(
            events.consumer_registry_capability(),
        )),
        traversal.clone(),
    )
    .unwrap();
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    let build_planner = |request| {
        ProductAgentPlannerPort::new(
            belief_store.clone(),
            traversal.clone(),
            curation.clone(),
            ProductEventAppendPort::new(&events),
            request,
        )
    };
    let mut planner_request = PlannerCurrentAssemblyRequest {
        additional_beliefs: Vec::new(),
        required_derived_evidence: None,
        required_graph_evidence: Vec::new(),
        context: PlannerDecisionContext {
            context_id: "prepared".into(),
            agent_id: authority.agent_id.clone(),
            goal_id: "prepared".into(),
            subject: subject.clone(),
            observation_subject: None,
            scope_id: "prepared".into(),
            branch_id: authority.branch_scope.branch_id.clone(),
            perspective_id: authority.perspective.perspective_id.clone(),
            authority_scope_id: "runtime-authority".into(),
            activation_generation: fence.activation_generation.clone(),
            admission_epoch: None,
        },
        policy: PlannerAssemblyPolicy {
            policy_revision_id: "observation-port-test".into(),
            required_sources: vec![PlannerSourceKind::Graph],
            explicitly_not_required: vec![PlannerSourceKind::Causation, PlannerSourceKind::Regime],
        },
        traversal_cut_request: TraversalCutRequest {
            owners: vec![],
            scope: products.curation_rule.rule.scope.clone(),
            currentness: OwnerCurrentnessPolicy::LatestComplete,
            event_position: meld_events::LedgerCursor {
                ledger_id: before.ledger_id,
                after_seq: before.committed_seq,
            },
        },
        traversal_request: products.curation_rule.rule.traversal_request(),
        belief_key: meld_world_model::belief::BeliefKey {
            subject: subject.clone(),
            dimension_id: "startup_realization".into(),
            predicate_id: "realized".into(),
            perspective: authority.perspective.clone(),
            branch_scope: authority.branch_scope.clone(),
            evidence_policy_id: "startup-realization".into(),
        },
        unanchored_belief: true,
        source_positions: vec![],
    };
    let planner = build_planner(planner_request.clone());
    for selected in [&products, &next] {
        let PlannerAssemblyOutcome::Complete(cut) =
            planner.assemble_epoch(selected, &selected.specification.fence)
        else {
            panic!("exact nonce source absence must be readable after Graph replay");
        };
        assert_eq!(cut.context.subject, subject);
        assert_eq!(
            cut.context.observation_subject(),
            &selected.observation_subject
        );
        assert_eq!(cut.context.goal_id, selected.specification.goal_id);
        assert_eq!(
            cut.traversal_request,
            selected.curation_rule.rule.traversal_request()
        );
        assert!(cut
            .world_model_view
            .warnings
            .contains(&PlannerProjectionWarning::MissingBelief {
                subject: selected.observation_subject.clone()
            }));
        cut.validate().unwrap();
    }
    assert_eq!(
        crate::runtime::ports::ProductEventAppendPort::new(&events)
            .watermark()
            .unwrap(),
        before
    );
    use meld_world_model::belief::{
        configured_belief_key, BeliefAssessmentActor, BeliefAssessmentRequest, BeliefStatus,
        ConfiguredOutcomeMappingSet, EvidenceIngestionActor, EvidenceIngestionRequest,
        OutcomeMappingRegistryStore,
    };
    for selected in [&products, &next] {
        for request in selected.subscription_requests().unwrap() {
            let receipt = agent_store
                .epoch_subscription_receipt(&request.request_id)
                .unwrap()
                .unwrap();
            assert_eq!(receipt.belief_key, request.belief_key);
            assert!(belief_store
                .accepted_subscription(&family.revision_ref(), &request.belief_key)
                .unwrap()
                .is_some());
        }
    }
    let mut assessment = BeliefAssessmentActor::new(
        "startup-belief",
        belief_store.clone(),
        traversal.clone(),
        belief_registry.clone(),
        vec![family.family_id.clone()],
        vec![],
        authority.perspective.clone(),
        authority.branch_scope.clone(),
    )
    .with_pinned_families(vec![family.clone()]);
    let initial = assessment.bounded_step(&BeliefAssessmentRequest {
        sequence: 1,
        max_items: 8,
    });
    assert!(
        initial.fatal_errors.is_empty() && initial.retryable_errors.is_empty(),
        "{initial:?}"
    );
    assert_eq!(initial.items_committed, 2);
    let belief = |selected: &AgentEpochProducts| {
        let key = configured_belief_key(
            &family,
            &selected.observation_subject,
            &authority.perspective,
            &authority.branch_scope,
        );
        belief_store.current_revision(&key).unwrap().unwrap()
    };
    assert_eq!(belief(&next).status, BeliefStatus::NeedsObservation);
    planner_request
        .policy
        .required_sources
        .push(PlannerSourceKind::Belief);
    let planner = build_planner(planner_request);
    let prior_cut = planner.assemble_epoch(&next, &next.specification.fence);
    let PlannerAssemblyOutcome::Complete(unobserved) = prior_cut else {
        panic!("prior cut missing: {prior_cut:?}")
    };
    assert!(!unobserved
        .world_model_view
        .warnings
        .iter()
        .any(|warning| matches!(warning, PlannerProjectionWarning::MissingBelief { .. })));
    let desired = |selected: &AgentEpochProducts| Proposition::Holds {
        subject: Term::Object(selected.observation_subject.clone()),
        dimension: Term::Dimension("startup_realization".into()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    };
    assert!(!unobserved
        .world_model_view
        .world_state
        .satisfies(&desired(&next)));
    let mapping = OutcomeMappingRegistryStore::new(db.clone())
        .unwrap()
        .install(
            serde_json::from_str(include_str!(
                "../../../theory/startup/evidence_mapping.startup_realization.json"
            ))
            .unwrap(),
            1,
        )
        .unwrap()
        .1;
    let mut ingestion = EvidenceIngestionActor::new(
        "startup-evidence",
        belief_store.clone(),
        traversal.clone(),
        belief_registry.clone(),
        family.family_id.clone(),
        Arc::new(ProductEventReplayPort::new(events.replay_capability())),
        Arc::new(events.consumer_registry_capability()),
        Arc::new(ConfiguredOutcomeMappingSet::new(mapping.config.clone()).unwrap()),
        mapping.mapping_id.clone(),
        authority.perspective.clone(),
        authority.branch_scope.clone(),
    )
    .with_family_revision(family.clone())
    .with_mapping_revision(mapping.revision_ref());
    let mut absorb = || {
        let report = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 128 });
        assert!(
            report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.applicable_count, 1, "{report:?}");
        assert_eq!(report.revisions_committed, 1, "{report:?}");
    };
    let source = Arc::new(
        AgentEpochCurationSource::new(agent_store.clone(), authority.agent_id.clone()).unwrap(),
    );
    let curation_actor = StandingCurationActor::new(
        "epoch-curation",
        "epoch-source-test",
        authority.clone(),
        CurationRuleSource::Producer(source),
        curation.clone(),
        Arc::new(crate::runtime::ports::ProductCurationTraversalPort::new(
            traversal.clone(),
        )),
        Arc::new(ProductEventAppendPort::new(&events)),
    )
    .unwrap()
    .with_authority_port(Arc::new(CurationEpoch {
        observer: observer.clone(),
        base: authority.clone(),
    }));
    let latest_result = || {
        events
            .replay_capability()
            .replay(meld_events::ReplayRequest {
                cursor: meld_events::LedgerCursor {
                    ledger_id: before.ledger_id,
                    after_seq: 0,
                },
                limit: 128,
            })
            .unwrap()
            .records
            .into_iter()
            .filter(|record| record.event_type == CURATION_RESULT_EVENT_TYPE)
            .map(|record| serde_json::from_value::<CurationResult>(record.data.clone()).unwrap())
            .next_back()
            .unwrap()
    };
    let negative = curation_actor.bounded_step(32);
    assert!(
        negative.fatal_errors.is_empty() && negative.retryable_errors.is_empty(),
        "{negative:?}"
    );
    assert_eq!(negative.results_persisted, 1, "{negative:?}");
    let result = latest_result();
    assert_eq!(
        CurationQuery::new(&curation)
            .operation(&result.operation_id)
            .unwrap()
            .unwrap()
            .authority
            .admission_epoch
            .as_deref(),
        Some("epoch-two")
    );
    assert!(result
        .semantic_publication
        .unwrap()
        .batch
        .relations
        .iter()
        .any(|relation| relation.relation_type == "not_realized"));
    absorb();
    let negative_belief = belief(&next);
    assert_eq!(negative_belief.status, BeliefStatus::Settled);
    assert_eq!(negative_belief.planner_projection.confidence, 0.0);
    assert_eq!(negative_belief.theory_revision, Some(family.revision_ref()));
    assert_eq!(belief(&products).status, BeliefStatus::NeedsObservation);
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 64 })
        .unwrap();
    crate::nonce::emit(
        &events.append_capability(),
        "epoch-source-test",
        &next_request,
    )
    .unwrap();
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 64 })
        .unwrap();
    let realized = curation_actor.bounded_step(32);
    assert!(
        realized.fatal_errors.is_empty() && realized.retryable_errors.is_empty(),
        "{realized:?}"
    );
    assert_eq!(realized.results_persisted, 1, "{realized:?}");
    let result = latest_result();
    assert!(result
        .semantic_publication
        .unwrap()
        .batch
        .relations
        .iter()
        .any(|relation| relation.relation_type == "realizes"));
    absorb();
    let positive_belief = belief(&next);
    assert_eq!(positive_belief.status, BeliefStatus::Settled);
    assert_eq!(positive_belief.planner_projection.confidence, 1.0);
    assert_eq!(
        positive_belief.prior_revision_id.as_ref(),
        Some(&negative_belief.revision_id)
    );
    assert_ne!(positive_belief.evidence_ids, negative_belief.evidence_ids);
    assert_eq!(belief(&products).status, BeliefStatus::NeedsObservation);
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 64 })
        .unwrap();
    let mut third = fence.clone();
    let PlannerAssemblyOutcome::Complete(positive_cut) =
        planner.assemble_epoch(&next, &next.specification.fence)
    else {
        panic!("realized cut missing")
    };
    assert!(positive_cut
        .world_model_view
        .world_state
        .satisfies(&desired(&next)));
    third.admission_epoch = Some("epoch-three".into());
    *observer.0.lock().unwrap() = Some(third.clone());
    let missing = curation_actor.bounded_step(32);
    assert_eq!(missing.results_persisted, 0, "{missing:?}");
    let wake = &missing.waiting_on[0].wake_addresses[0];
    assert!(curation_actor.resolves_wake(wake).unwrap());
    let meld_world_model::waiting::StructuralWakeAddress::OwnerRevision(value) = wake else {
        panic!("expected Agent owner wake");
    };
    let foreign = meld_world_model::waiting::StructuralWakeAddress::OwnerRevision(
        value.replace(agent_store.resource_id(), "foreign-store"),
    );
    assert!(!curation_actor.resolves_wake(&foreign).unwrap());
    assert_eq!(
        build(agent_store.clone())
            .bounded_step(8)
            .fatal_errors
            .len(),
        1
    );
    let third_goal = intent.goal_id(&authority.agent_id, &third.reconciliation_scope());
    let third_products = agent_store.epoch_products(&third_goal).unwrap().unwrap();
    assert_ne!(third_products.observation_subject, next.observation_subject);
    let after_switch = curation_actor.bounded_step(32);
    assert!(
        after_switch.fatal_errors.is_empty() && after_switch.retryable_errors.is_empty(),
        "{after_switch:?}"
    );
    assert_eq!(after_switch.results_persisted, 1, "{after_switch:?}");
    let result = latest_result();
    let publication = result.semantic_publication.unwrap();
    assert!(publication
        .batch
        .relations
        .iter()
        .any(|relation| relation.relation_type == "not_realized"));
    assert!(!publication
        .batch
        .relations
        .iter()
        .any(|relation| relation.relation_type == "realizes"));
    absorb();
    assert_eq!(belief(&third_products).planner_projection.confidence, 0.0);
    assert_eq!(belief(&next).revision_id, positive_belief.revision_id);
    assert!(agent_store
        .reconciliation_goal(&third_goal)
        .unwrap()
        .is_none());
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 64 })
        .unwrap();
    let PlannerAssemblyOutcome::Complete(third_cut) =
        planner.assemble_epoch(&third_products, &third_products.specification.fence)
    else {
        panic!("new epoch cut missing")
    };
    assert!(!third_cut
        .world_model_view
        .world_state
        .satisfies(&desired(&third_products)));
}
