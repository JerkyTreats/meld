use std::sync::Arc;

use meld_events::{EventAuthority, EventAuthorityOpenOptions, LedgerCursor, ReplayRequest};

use super::*;

fn request(fence: &str) -> NonceRequest {
    NonceRequest::new(
        "principal-a".into(),
        meld_events::DomainObjectRef::new("sample", "subject", "one").unwrap(),
        vec!["correlation-a".into(), "correlation-b".into()],
        fence.into(),
    )
    .unwrap()
}

#[test]
fn exact_request_replays_one_event_and_changed_fence_creates_a_distinct_nonce() {
    let temp = tempfile::tempdir().unwrap();
    let authority = Arc::new(
        EventAuthority::open(
            sled::open(temp.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap(),
    );
    let original = request("fence-1");
    let expected = original.expected_event_id().unwrap();
    let first = emit(&authority.append_capability(), "session-1", &original).unwrap();
    let replay = emit(
        &authority.append_capability(),
        "recovered-session",
        &original,
    )
    .unwrap();
    assert_eq!(first, replay);
    assert_eq!(first.event_record_id, expected);
    let second = emit(
        &authority.append_capability(),
        "session-2",
        &request("fence-2"),
    )
    .unwrap();
    assert_ne!(first.nonce_id, second.nonce_id);
    let page = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: first.position.ledger_id,
                after_seq: 0,
            },
            limit: 8,
        })
        .unwrap();
    assert_eq!(page.records.len(), 2);
    assert_eq!(hydrate(&page.records[0]).unwrap(), original);
    assert_eq!(page.records[0].event_type, "nonce");
    let mut tampered = page.records[0].clone();
    tampered.domain_id = "foreign".into();
    assert!(hydrate(&tampered).is_err());
}

#[test]
fn closed_request_schema_rejects_forged_identity_and_diagnostics() {
    let mut original = request("fence-1");
    original.correlation_refs.reverse();
    assert!(original.validate().is_err());
    let mut value = serde_json::to_value(request("fence-1")).unwrap();
    value["diagnostics"] = serde_json::json!({"clock": 123});
    assert!(serde_json::from_value::<NonceRequest>(value).is_err());
}

#[test]
fn nonce_owner_event_reaches_graph_only_through_its_exact_installed_route() {
    use crate::runtime::ports::{
        ProductEventAppendPort, ProductEventReplayPort, ProductGraphCursorPort,
    };
    use meld_world_model::world_state::graph::contracts::*;
    use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
    use meld_world_model::world_state::graph::store::TraversalStore;
    let temp = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let db = sled::open(temp.path().join("world-model")).unwrap();
    let store = Arc::new(TraversalStore::new(db.clone()).unwrap());
    let route = graph_route();
    let reference = store.install_owner_event_route(&route).unwrap();
    assert_eq!(store.install_owner_event_route(&route).unwrap(), reference);
    let mut foreign_contract = route.clone();
    foreign_contract.enumeration_rule_revision = "foreign-revision".into();
    assert!(store.install_owner_event_route(&foreign_contract).is_err());
    let store = Arc::new(TraversalStore::new(db).unwrap());
    assert_eq!(
        store.owner_event_route(OWNER_ID, EVENT_TYPE).unwrap(),
        Some(route)
    );
    assert!(store
        .owner_event_route("foreign-owner", EVENT_TYPE)
        .unwrap()
        .is_none());
    let graph = GraphRuntime::from_ports(
        Arc::new(ProductEventReplayPort::new(authority.replay_capability())),
        Arc::new(ProductEventAppendPort::new(&authority)),
        Arc::new(ProductGraphCursorPort::new(
            authority.consumer_registry_capability(),
        )),
        store.clone(),
    )
    .unwrap();
    let request = request("fence-graph");
    let emitted = emit(&authority.append_capability(), "graph-proof", &request).unwrap();
    let scope = request.publication_scope();
    let cut_request = TraversalCutRequest {
        owners: vec![TraversalOwnerRequirement {
            event_source: None,
            owner_id: OWNER_ID.into(),
            scope: scope.clone(),
            required: true,
        }],
        scope,
        currentness: OwnerCurrentnessPolicy::LatestComplete,
        event_position: emitted.position,
    };
    let query = meld_world_model::TraversalQuery::new(&store);
    assert_eq!(
        query.cut(&cut_request).unwrap().status,
        TraversalCutStatus::Incomplete
    );
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 16 })
        .unwrap();
    let cut = query.cut(&cut_request).unwrap();
    assert_eq!(cut.status, TraversalCutStatus::Complete);
    let result = query
        .traverse(
            &cut,
            &BoundedTraversalRequest {
                roots: vec![request.object_ref().unwrap()],
                direction: TraversalDirection::Outgoing,
                relation_types: None,
                bounds: TraversalBounds {
                    max_depth: 2,
                    max_objects: 16,
                    max_occurrences: 16,
                    max_paths: 16,
                },
            },
        )
        .unwrap();
    assert!(result
        .objects
        .iter()
        .any(|object| object.object_ref == request.object_ref().unwrap()));
    let projected = store
        .owner_publications_through_seq(emitted.position.after_seq)
        .unwrap();
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].source_event.seq, emitted.position.after_seq);
    assert_eq!(projected[0].operation, request.publication().unwrap());
    let mut invalid = request.clone();
    invalid.nonce_id = "invalid".into();
    assert!(invalid.envelope("graph-proof").is_err());
}

fn context_api(root: &std::path::Path) -> crate::api::ContextApi {
    crate::api::ContextApi::new(
        Arc::new(crate::store::SledNodeRecordStore::new(root.join("nodes")).unwrap()),
        Arc::new(crate::context::frame::FrameStorage::new(root.join("frames")).unwrap()),
        Arc::new(parking_lot::RwLock::new(crate::heads::HeadIndex::new())),
        Arc::new(
            crate::prompt_context::PromptContextArtifactStorage::new(root.join("prompts")).unwrap(),
        ),
        Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new())),
        Arc::new(parking_lot::RwLock::new(
            crate::provider::ProviderRegistry::new(),
        )),
        Arc::new(crate::concurrency::NodeLockManager::new()),
    )
}

#[tokio::test]
async fn capability_requires_exact_effect_authority_and_returns_a_proven_append_receipt() {
    use crate::capability::*;
    let temp = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let api = context_api(temp.path());
    let request = request("capability-fence");
    let contract = capability::contract();
    let runtime = CapabilityRuntimeInit {
        capability_instance_id: "nonce-instance".into(),
        capability_type_id: contract.capability_type_id,
        capability_version: contract.capability_version,
        scope_ref: request.subject_ref.object_id.clone(),
        scope_kind: contract.scope_contract.scope_kind,
        binding_values: Vec::new(),
        input_contract: contract.input_contract,
        output_contract: contract.output_contract,
        effect_contract: contract.effect_contract,
        execution_contract: contract.execution_contract,
    };
    let payload = CapabilityInvocationPayload {
        invocation_id: "nonce-invocation".into(),
        capability_instance_id: runtime.capability_instance_id.clone(),
        supplied_inputs: vec![SuppliedInputValue {
            slot_id: capability::REQUEST.into(),
            source: InputValueSource::InitPayload,
            value: SuppliedValueRef::StructuredValue(serde_json::to_value(&request).unwrap()),
        }],
        upstream_lineage: None,
        execution_context: CapabilityExecutionContext::default(),
    };
    let context = crate::execution::ExecutionEventContext {
        session_id: "emitter-proof".into(),
        effect_authority: Some(meld_execution::ExecutionEffectAuthority {
            principal_id: request.issuer_ref.clone(),
            subject: request.subject_ref.clone(),
            fence_ref: request.fence_ref.clone(),
        }),
    };
    let emitter = capability::NonceEmitter;
    assert!(emitter
        .invoke(&api, &runtime, &payload, Some(&context))
        .await
        .is_err());
    api.bind_event_append(authority.append_capability())
        .unwrap();
    let foreign = EventAuthority::open(
        sled::open(temp.path().join("foreign-events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    assert!(api.bind_event_append(foreign.append_capability()).is_err());
    for field in 0..3 {
        let mut wrong = context.clone();
        let grant = wrong.effect_authority.as_mut().unwrap();
        match field {
            0 => grant.principal_id = "foreign-issuer".into(),
            1 => grant.subject.object_id = "foreign-subject".into(),
            _ => grant.fence_ref = "foreign-fence".into(),
        }
        assert!(emitter
            .invoke(&api, &runtime, &payload, Some(&wrong))
            .await
            .is_err());
    }
    let mut no_grant = context.clone();
    no_grant.effect_authority = None;
    assert!(emitter
        .invoke(&api, &runtime, &payload, Some(&no_grant))
        .await
        .is_err());
    let first = emitter
        .invoke(&api, &runtime, &payload, Some(&context))
        .await
        .unwrap();
    let replay = emitter
        .invoke(&api, &runtime, &payload, Some(&context))
        .await
        .unwrap();
    assert_eq!(first.emitted_artifacts, replay.emitted_artifacts);
    let receipt: NonceEmissionReceipt =
        serde_json::from_value(first.emitted_artifacts[0].content.clone()).unwrap();
    assert_eq!(
        receipt.event_record_id,
        request.expected_event_id().unwrap()
    );
    assert_eq!(receipt.position.after_seq, 1);
    let page = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: receipt.position.ledger_id,
                after_seq: 0,
            },
            limit: 8,
        })
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(hydrate(&page.records[0]).unwrap(), request);
}

#[tokio::test]
async fn admitted_nonce_task_reaches_graph_before_outcome_and_recovers_one_durable_emission() {
    use crate::runtime::ports::{
        ProductEventAppendPort, ProductEventReplayPort, ProductGraphCursorPort,
        ProductionDispatchRouteContext,
    };
    use meld_execution::task::TaskCompiler;
    use meld_execution::task_admission::*;
    use meld_execution::task_network::dispatch_actor::*;
    use meld_execution::task_network::{
        command, NetworkState, SledTaskNetworkStore, TaskAdmissionAttribution,
    };
    use meld_lang::*;
    use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
    use meld_world_model::world_state::graph::store::TraversalStore;

    struct OpenEpoch;
    impl AdmissionGenerationObserver for OpenEpoch {
        fn active_generation(&self, agent: &str) -> Result<Option<String>, String> {
            Ok((agent == "principal-a").then(|| "generation-a".into()))
        }
        fn validates_admission(
            &self,
            admission: &TaskAdmissionAttribution,
        ) -> Result<bool, String> {
            Ok(admission.agent_id == "principal-a"
                && admission.activation_generation == "generation-a"
                && admission.admission_epoch.as_deref() == Some("task-epoch"))
        }
    }
    struct BeforeOutcome<'a>(&'a mut SledTaskNetworkStore);
    impl TaskNetworkCommandPort for BeforeOutcome<'_> {
        fn network_state(&self) -> &NetworkState {
            self.0.state()
        }
        fn submit_command(
            &mut self,
            request: command::Request,
        ) -> Result<command::Response, DispatchPortError> {
            if matches!(request.command, command::Command::RecordTaskOutcome(_)) {
                return Err(DispatchPortError::retryable(
                    "crash after effect, before outcome",
                ));
            }
            self.0.submit_command(request)
        }
    }

    let temp = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let db = sled::open(temp.path().join("execution")).unwrap();
    let mut network = SledTaskNetworkStore::open(db.clone(), "nonce-network").unwrap();
    let api = Arc::new(context_api(temp.path()));
    api.bind_event_append(authority.append_capability())
        .unwrap();
    let nonce = request("task-epoch");
    let mut catalog = crate::capability::CapabilityCatalog::new();
    let mut registry = crate::capability::CapabilityExecutorRegistry::new();
    registry
        .register(&mut catalog, capability::NonceEmitter)
        .unwrap();
    let contract = capability::contract();
    let policy = AuthorityPolicy {
        policy_id: "nonce-policy".into(),
        principal_id: nonce.issuer_ref.clone(),
        subject: nonce.subject_ref.clone(),
        principal_granted_action_ids: vec![capability::EMIT.into()],
        runtime_allowed_action_ids: vec![capability::EMIT.into()],
        restricted_action_ids: vec![],
    };
    let policy =
        AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap()).unwrap();
    let task = ExecutionTask {
        task_id: "nonce-task".into(),
        initial_inputs: vec![TaskInput {
            step_id: "emit".into(),
            slot_id: capability::REQUEST.into(),
            artifact_type_id: capability::REQUEST.into(),
            schema_version: 1,
            content: serde_json::to_value(&nonce).unwrap(),
        }],
        composition: Composition {
            steps: vec![Step {
                step_id: "emit".into(),
                kind: StepKind::Op(Operator {
                    operator_id: "emit".into(),
                    preconditions: vec![],
                    effects: vec![],
                    cost: CostEstimate {
                        time_ms: 1,
                        money_microdollars: 0,
                        provider_calls: 0,
                    },
                    resolution: Resolution {
                        requires_inputs: vec![SlotConstraint {
                            artifact_type: Term::ArtifactType(capability::REQUEST.into()),
                            required: true,
                        }],
                        requires_outputs: vec![SlotConstraint {
                            artifact_type: Term::ArtifactType(capability::RECEIPT.into()),
                            required: true,
                        }],
                        scope_kind: Some("domain_object".into()),
                        tags: vec![],
                        specific: Some(CapabilityRef {
                            capability_type_id: capability::EMIT.into(),
                            capability_version: 1,
                        }),
                    },
                }),
            }],
            edges: vec![],
        },
        bindings: Bindings::empty(),
        capability_contract_ids: vec![contract.content_identity()],
        expected_outcome_contract_id: "nonce-emitted-v1".into(),
        authority_requirements: vec![capability::EMIT.into()],
        idempotency_key: "nonce-task".into(),
    };
    let admission =
        TaskAdmissionApi::new(&mut network, &catalog, "generation-a", &policy.content_hash)
            .with_admission_epoch(Some("task-epoch"))
            .admit(TaskAdmissionRequest {
                lineage: TaskAdmissionLineage {
                    agent_id: nonce.issuer_ref.clone(),
                    goal_id: "nonce-goal".into(),
                    plan_revision_id: "nonce-plan".into(),
                    product_id: task.task_id.clone(),
                    authorization_id: "nonce-authorization".into(),
                    context_id: "nonce-context".into(),
                    authority_scope_id: policy.policy.policy_id.clone(),
                    authority_policy_content_hash: policy.content_hash.clone(),
                    authority_decision: Some(AuthorityDecision {
                        policy_id: policy.policy.policy_id.clone(),
                        policy_content_hash: policy.content_hash.clone(),
                        principal_id: nonce.issuer_ref.clone(),
                        subject: nonce.subject_ref.clone(),
                        requested_action_ids: vec![capability::EMIT.into()],
                        authorized_action_ids: vec![capability::EMIT.into()],
                    }),
                    activation_generation: "generation-a".into(),
                    admission_epoch: Some("task-epoch".into()),
                },
                task,
                idempotency_key: "nonce-task".into(),
            })
            .unwrap();
    assert_eq!(admission.decision, TaskAdmissionDecision::Admitted);
    let lowered = TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
        TaskCompiler::new(),
        catalog.clone(),
    ))
    .run_once(
        &mut network,
        TaskAdmissionRuntimeRequest {
            network_id: "nonce-network".into(),
            max_items: 1,
        },
    )
    .unwrap();
    assert_eq!(lowered.committed, 1, "{lowered:?}");
    let routes = ProductionDispatchRouteContext {
        api,
        session_id: Some("nonce-task-session".into()),
        catalog,
        registry,
    }
    .into_claim_port();
    let dispatch = || {
        DispatchRuntimeActor::new("nonce-worker", db.clone(), routes.clone())
            .unwrap()
            .with_authority_policy(policy.clone())
            .with_admission_generation_observer(Arc::new(OpenEpoch))
    };
    let first = dispatch()
        .tick(
            &mut BeforeOutcome(&mut network),
            DispatchTickRequest {
                sequence: 1,
                max_items: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(first.items_attempted, 1, "{first:?}");
    assert!(network.state().outcomes.is_empty(), "{first:?}");
    let ledger_id = authority.append_capability().ledger_identity();
    let events = || {
        authority
            .replay_capability()
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id,
                    after_seq: 0,
                },
                limit: 100,
            })
            .unwrap()
            .records
    };
    let nonce_events: Vec<_> = events()
        .into_iter()
        .filter(|event| event.event_type == EVENT_TYPE)
        .collect();
    assert_eq!(nonce_events.len(), 1, "{first:?}");
    assert_eq!(hydrate(&nonce_events[0]).unwrap(), nonce);

    let world =
        Arc::new(TraversalStore::new(sled::open(temp.path().join("world")).unwrap()).unwrap());
    world.install_owner_event_route(&graph_route()).unwrap();
    GraphRuntime::from_ports(
        Arc::new(ProductEventReplayPort::new(authority.replay_capability())),
        Arc::new(ProductEventAppendPort::new(&authority)),
        Arc::new(ProductGraphCursorPort::new(
            authority.consumer_registry_capability(),
        )),
        world.clone(),
    )
    .unwrap()
    .catch_up_bounded(GraphCatchUpBudget { max_items: 100 })
    .unwrap();
    let publication = world
        .owner_publications_through_seq(nonce_events[0].seq)
        .unwrap();
    assert!(publication
        .iter()
        .any(|record| record.operation == nonce.publication().unwrap()));
    assert!(network.state().outcomes.is_empty());

    network.flush().unwrap();
    drop(network);
    let mut network = SledTaskNetworkStore::open(db.clone(), "nonce-network").unwrap();
    let recovered = dispatch()
        .tick(
            &mut network,
            DispatchTickRequest {
                sequence: 2,
                max_items: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(recovered.items_committed, 1, "{recovered:?}");
    assert_eq!(network.state().outcomes.len(), 1);
    assert_eq!(
        events()
            .iter()
            .filter(|event| event.event_type == EVENT_TYPE)
            .count(),
        1
    );
    let outcome = network.state().outcomes.values().next().unwrap();
    assert_eq!(
        outcome.status,
        meld_execution::task_network::dispatch::OutcomeStatus::Succeeded
    );
    let claim = network.state().claims.values().next().unwrap();
    let repo = meld_execution::task::TaskArtifactRepo::open_sled(
        db.clone(),
        dispatch_claim_repo_id(&claim.claim_id),
    )
    .unwrap();
    let artifact = repo
        .record()
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_type_id == capability::RECEIPT)
        .unwrap();
    let receipt: NonceEmissionReceipt = serde_json::from_value(artifact.content.clone()).unwrap();
    assert_eq!(receipt.event_record_id, nonce.expected_event_id().unwrap());
    assert_eq!(receipt.position.after_seq, nonce_events[0].seq);
}

#[test]
fn shipped_nonce_package_installs_exact_owner_contracts_without_starting_work() {
    use crate::runtime::storage::{OpenProductStores, ProductStorageLayout};
    let temp = tempfile::tempdir().unwrap();
    let stores = OpenProductStores::open(&ProductStorageLayout::from_root(temp.path())).unwrap();
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/nonce");
    let receipt =
        crate::init::world::product::install_package(&stores, &package, Some("meld.nonce"), 0)
            .unwrap();
    assert_eq!(receipt.components.len(), 2);
    assert_eq!(
        stores
            .traversal_store
            .owner_event_route(OWNER_ID, EVENT_TYPE)
            .unwrap(),
        Some(graph_route())
    );
    assert!(receipt
        .components
        .iter()
        .any(|component| component.component_id == "nonce-capability-emit"));
    assert!(stores
        .traversal_store
        .owner_publications_through_seq(u64::MAX)
        .unwrap()
        .is_empty());
    let replay =
        crate::init::world::product::install_package(&stores, &package, Some("meld.nonce"), 0)
            .unwrap();
    assert_eq!(replay, receipt);
}

#[test]
fn curation_assesses_nonce_source_under_a_distinct_exact_agent_judgment_scope() {
    use crate::runtime::ports::{
        ProductCurationTraversalPort, ProductEventAppendPort, ProductEventReplayPort,
        ProductGraphCursorPort,
    };
    use meld_world_model::belief::BranchScope;
    use meld_world_model::curation::*;
    use meld_world_model::world_state::graph::contracts::*;
    use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
    use meld_world_model::world_state::graph::store::TraversalStore;
    let temp = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let db = sled::open(temp.path().join("world")).unwrap();
    let graph_store = Arc::new(TraversalStore::new(db.clone()).unwrap());
    graph_store
        .install_owner_event_route(&graph_route())
        .unwrap();
    let graph = GraphRuntime::from_ports(
        Arc::new(ProductEventReplayPort::new(authority.replay_capability())),
        Arc::new(ProductEventAppendPort::new(&authority)),
        Arc::new(ProductGraphCursorPort::new(
            authority.consumer_registry_capability(),
        )),
        graph_store.clone(),
    )
    .unwrap();
    let nonce = request("epoch-source");
    let emitted = emit(&authority.append_capability(), "source-proof", &nonce).unwrap();
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    let judgment = CurationJudgmentScope {
        subject: nonce.subject_ref.clone(),
        perspective: PerspectiveKey::new("agent", "nonce-observation").unwrap(),
        branch_scope: BranchScope::main(),
    };
    let binding = CurationRuleBinding {
        agent_id: nonce.issuer_ref.clone(),
        subject: nonce.subject_ref.clone(),
        scope: OwnerPublicationScope {
            scope_id: nonce.subject_ref.object_id.clone(),
            branch_id: Some("main".into()),
            perspective_id: Some("nonce-observation".into()),
            valid_at: None,
        },
    };
    let source = CurationSourceBinding {
        event_source: None,
        scope: nonce.publication_scope(),
        roots: vec![nonce.object_ref().unwrap()],
    };
    let template = CurationRuleTemplate {
        rule_id: "confirm-requested-nonce".into(),
        source_owner_id: OWNER_ID.into(),
        traversal_direction: TraversalDirection::Incoming,
        bounds: TraversalBounds {
            max_depth: 4,
            max_objects: 32,
            max_occurrences: 32,
            max_paths: 32,
        },
        expected_object_kind: "expected_nonce".into(),
        expected_object_key: "requested_nonce".into(),
        relation_type: "expects".into(),
        output_policy_revision: "nonce-observed-v1".into(),
        realization: Some(CurationRealizationTemplate {
            observed_object: CurationObjectSelection::SourceRoot,
            required_qualifications: std::collections::BTreeMap::from([(
                "fence_ref".into(),
                nonce.fence_ref.clone(),
            )]),
            realized_relation_type: "realizes".into(),
            not_realized_relation_type: "not_realized_within_cut".into(),
        }),
    };
    let store = Arc::new(CurationStore::new(db.clone()).unwrap());
    let installed = store
        .install_template(template, emitted.position.after_seq)
        .unwrap();
    let revision = store
        .prepare_rule_for_source(
            &installed.revision_ref(),
            &binding,
            &source,
            &judgment,
            emitted.position.after_seq,
        )
        .unwrap();
    assert_eq!(revision.rule.scope, nonce.publication_scope());
    assert_ne!(revision.rule.roots[0], judgment.subject);
    let reopened = CurationStore::new(db).unwrap();
    assert_eq!(
        reopened
            .resolve_source_bound_rule(
                &installed.revision_ref(),
                &binding,
                &source,
                &judgment,
                &revision.revision_ref()
            )
            .unwrap(),
        revision
    );
    let mut wrong_source = source.clone();
    wrong_source.roots = vec![request("other-epoch").object_ref().unwrap()];
    assert!(reopened
        .resolve_source_bound_rule(
            &installed.revision_ref(),
            &binding,
            &wrong_source,
            &judgment,
            &revision.revision_ref()
        )
        .is_err());
    let agent_authority = CurationAuthority {
        agent_id: nonce.issuer_ref.clone(),
        perspective: judgment.perspective.clone(),
        branch_scope: judgment.branch_scope.clone(),
        subject: judgment.subject.clone(),
        activation_generation: "generation-source".into(),
        admission_epoch: Some(nonce.fence_ref.clone()),
    };
    let actor = StandingCurationActor::new(
        "curation",
        "source-proof",
        agent_authority.clone(),
        revision.clone(),
        store.clone(),
        Arc::new(ProductCurationTraversalPort::new(graph_store.clone())),
        Arc::new(ProductEventAppendPort::new(&authority)),
    )
    .unwrap();
    let report = actor.bounded_step(1);
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert_eq!(report.results_persisted, 1, "{report:?}");
    let records = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: emitted.position.ledger_id,
                after_seq: emitted.position.after_seq,
            },
            limit: 32,
        })
        .unwrap()
        .records;
    let result: CurationResult = serde_json::from_value(
        records
            .iter()
            .find(|record| record.event_type == CURATION_RESULT_EVENT_TYPE)
            .unwrap()
            .data
            .clone(),
    )
    .unwrap();
    assert_eq!(result.disposition, CurationTerminalDisposition::Applied);
    let publication = result.semantic_publication.as_ref().unwrap();
    let realization = publication
        .batch
        .relations
        .iter()
        .find(|relation| relation.relation_type == "realizes")
        .unwrap();
    assert_eq!(realization.dst, nonce.object_ref().unwrap());
    assert_eq!(
        realization.qualifications.get("admission_epoch"),
        Some(&nonce.fence_ref)
    );
    assert_eq!(
        realization.qualifications.get("perspective"),
        Some(&judgment.perspective.index_key())
    );
    let operation = CurationQuery::new(&store)
        .operation(&result.operation_id)
        .unwrap()
        .unwrap();
    for variant in 0..4 {
        let mut foreign = agent_authority.clone();
        match variant {
            0 => foreign.subject.object_id = "foreign-subject".into(),
            1 => foreign.perspective.perspective_kind = "foreign-kind".into(),
            2 => foreign.perspective.perspective_id = "foreign-perspective".into(),
            _ => foreign.branch_scope.branch_id = "foreign-branch".into(),
        }
        let altered = CurationOperation::reconstruct(
            foreign,
            revision.revision_ref(),
            operation.source_cut.clone(),
            operation.traversal_request.clone(),
        )
        .unwrap();
        assert_eq!(
            CurationAcceptanceRecord::for_operation(&altered, &revision)
                .unwrap()
                .decision,
            CurationAdmissionDecision::Rejected
        );
    }
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    let repeated = actor.bounded_step(1);
    assert!(repeated.fatal_errors.is_empty(), "{repeated:?}");
    let records = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: emitted.position.ledger_id,
                after_seq: 0,
            },
            limit: 32,
        })
        .unwrap()
        .records;
    let results: Vec<CurationResult> = records
        .iter()
        .filter(|record| record.event_type == CURATION_RESULT_EVENT_TYPE)
        .map(|record| serde_json::from_value(record.data.clone()).unwrap())
        .collect();
    assert_eq!(
        results.last().unwrap().disposition,
        CurationTerminalDisposition::Unchanged
    );
}

#[test]
fn complete_event_source_proves_absence_then_realization_without_fabricated_events() {
    use crate::runtime::ports::{
        ProductCurationTraversalPort, ProductEventAppendPort, ProductEventReplayPort,
        ProductGraphCursorPort,
    };
    use meld_world_model::belief::BranchScope;
    use meld_world_model::curation::*;
    use meld_world_model::world_state::graph::contracts::*;
    use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
    use meld_world_model::world_state::graph::store::TraversalStore;
    let temp = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ledger_id = authority.append_capability().ledger_identity();
    let db = sled::open(temp.path().join("world")).unwrap();
    let graph_store = Arc::new(TraversalStore::new(db.clone()).unwrap());
    graph_store
        .install_owner_event_route(&graph_route())
        .unwrap();
    let graph = GraphRuntime::from_ports(
        Arc::new(ProductEventReplayPort::new(authority.replay_capability())),
        Arc::new(ProductEventAppendPort::new(&authority)),
        Arc::new(ProductGraphCursorPort::new(
            authority.consumer_registry_capability(),
        )),
        graph_store.clone(),
    )
    .unwrap();
    let nonce = request("covered-epoch");
    let source_ref = graph_route().source_ref().unwrap();
    let cut_request = |nonce: &NonceRequest, seq| TraversalCutRequest {
        owners: vec![TraversalOwnerRequirement {
            event_source: Some(source_ref.clone()),
            owner_id: OWNER_ID.into(),
            scope: nonce.publication_scope(),
            required: true,
        }],
        scope: nonce.publication_scope(),
        currentness: OwnerCurrentnessPolicy::LatestComplete,
        event_position: LedgerCursor {
            ledger_id,
            after_seq: seq,
        },
    };
    let query = meld_world_model::TraversalQuery::new(&graph_store);
    assert_eq!(
        query.cut(&cut_request(&nonce, 0)).unwrap().status,
        TraversalCutStatus::Incomplete
    );
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    let empty_cut = query.cut(&cut_request(&nonce, 0)).unwrap();
    assert_eq!(empty_cut.status, TraversalCutStatus::Complete);
    let mut forged_source = cut_request(&nonce, 0);
    forged_source.owners[0]
        .event_source
        .as_mut()
        .unwrap()
        .content_hash = "foreign-revision".into();
    assert_eq!(
        query.cut(&forged_source).unwrap().status,
        TraversalCutStatus::Incomplete
    );
    assert!(!graph_store
        .covers_event_source(meld_events::LedgerIdentity::new(), OWNER_ID, &source_ref)
        .unwrap());
    assert!(empty_cut.receipts[0].source_event.is_none());
    assert_eq!(
        empty_cut.receipts[0]
            .event_coverage
            .as_ref()
            .unwrap()
            .through
            .after_seq,
        0
    );
    let traversal_request = BoundedTraversalRequest {
        roots: vec![nonce.object_ref().unwrap()],
        direction: TraversalDirection::Incoming,
        relation_types: None,
        bounds: TraversalBounds {
            max_depth: 4,
            max_objects: 32,
            max_occurrences: 32,
            max_paths: 32,
        },
    };
    let empty = query.traverse(&empty_cut, &traversal_request).unwrap();
    assert_eq!(empty.absent_roots, traversal_request.roots);
    assert!(empty.objects.is_empty());
    assert!(empty.frontier.is_empty());
    let judgment = CurationJudgmentScope {
        subject: nonce.subject_ref.clone(),
        perspective: PerspectiveKey::new("agent", "nonce").unwrap(),
        branch_scope: BranchScope::main(),
    };
    let store = Arc::new(CurationStore::new(db.clone()).unwrap());
    let rule = store
        .install_rule(
            StandingCurationRule {
                source_event_route: Some(source_ref.clone()),
                judgment_scope: Some(judgment.clone()),
                rule_id: "covered-nonce-rule".into(),
                agent_id: nonce.issuer_ref.clone(),
                source_owner_id: OWNER_ID.into(),
                scope: nonce.publication_scope(),
                roots: traversal_request.roots.clone(),
                traversal_direction: TraversalDirection::Incoming,
                bounds: traversal_request.bounds.clone(),
                expected_object_kind: "expected_nonce".into(),
                expected_object_id: format!("expected::{}", nonce.nonce_id),
                relation_type: "expects".into(),
                output_policy_revision: "nonce-realization-v1".into(),
                realization: Some(CurationRealizationRule {
                    observed_object: nonce.object_ref().unwrap(),
                    required_qualifications: std::collections::BTreeMap::from([(
                        "fence_ref".into(),
                        nonce.fence_ref.clone(),
                    )]),
                    realized_relation_type: "realizes".into(),
                    not_realized_relation_type: "not_realized_within_cut".into(),
                }),
            },
            0,
        )
        .unwrap();
    let agent_authority = CurationAuthority {
        agent_id: nonce.issuer_ref.clone(),
        subject: judgment.subject.clone(),
        perspective: judgment.perspective.clone(),
        branch_scope: judgment.branch_scope.clone(),
        activation_generation: "covered-generation".into(),
        admission_epoch: Some(nonce.fence_ref.clone()),
    };
    let actor = StandingCurationActor::new(
        "curation",
        "covered-source",
        agent_authority,
        rule,
        store,
        Arc::new(ProductCurationTraversalPort::new(graph_store.clone())),
        Arc::new(ProductEventAppendPort::new(&authority)),
    )
    .unwrap();
    let results = || {
        authority
            .replay_capability()
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id,
                    after_seq: 0,
                },
                limit: 64,
            })
            .unwrap()
            .records
            .into_iter()
            .filter(|record| record.event_type == CURATION_RESULT_EVENT_TYPE)
            .map(|record| serde_json::from_value::<CurationResult>(record.data.clone()).unwrap())
            .collect::<Vec<_>>()
    };
    let negative = actor.bounded_step(1);
    assert_eq!(negative.results_persisted, 1, "{negative:?}");
    let result = results().pop().unwrap();
    assert_eq!(result.disposition, CurationTerminalDisposition::Applied);
    assert!(result
        .semantic_publication
        .as_ref()
        .unwrap()
        .batch
        .relations
        .iter()
        .any(|relation| relation.relation_type == "not_realized_within_cut"));
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    let unchanged = actor.bounded_step(1);
    assert!(unchanged.fatal_errors.is_empty(), "{unchanged:?}");
    assert_eq!(
        results().last().unwrap().disposition,
        CurationTerminalDisposition::Unchanged
    );
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    let stable = actor.bounded_step(1);
    assert_eq!(stable.results_persisted, 0, "{stable:?}");
    assert_eq!(stable.publications_appended, 0, "{stable:?}");

    let emitted = emit(&authority.append_capability(), "covered-source", &nonce).unwrap();
    assert_eq!(
        query
            .cut(&cut_request(&nonce, emitted.position.after_seq))
            .unwrap()
            .status,
        TraversalCutStatus::Incomplete
    );
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    let realized_cut = query
        .cut(&cut_request(&nonce, emitted.position.after_seq))
        .unwrap();
    assert_eq!(realized_cut.status, TraversalCutStatus::Complete);
    assert_eq!(
        realized_cut.receipts[0].source_event.unwrap().seq,
        emitted.position.after_seq
    );
    let realized = actor.bounded_step(1);
    assert_eq!(realized.results_persisted, 1, "{realized:?}");
    let result = results().pop().unwrap();
    assert!(result
        .semantic_publication
        .as_ref()
        .unwrap()
        .batch
        .relations
        .iter()
        .any(|relation| relation.relation_type == "realizes"));
    let historical = query.traverse(&empty_cut, &traversal_request).unwrap();
    assert_eq!(historical, empty);
    let next = request("next-epoch");
    let next_cut = query
        .cut(&cut_request(&next, emitted.position.after_seq))
        .unwrap();
    let mut next_request = traversal_request;
    next_request.roots = vec![next.object_ref().unwrap()];
    assert_eq!(
        query
            .traverse(&next_cut, &next_request)
            .unwrap()
            .absent_roots,
        next_request.roots
    );
    let reopened = TraversalStore::new(db).unwrap();
    assert!(reopened
        .covers_event_source(ledger_id, OWNER_ID, &source_ref)
        .unwrap());
    let events = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id,
                after_seq: 0,
            },
            limit: 64,
        })
        .unwrap()
        .records;
    assert_eq!(
        events
            .iter()
            .filter(|record| record.event_type == EVENT_TYPE)
            .count(),
        1
    );
}

#[test]
fn late_owner_route_gains_coverage_only_after_bounded_durable_historical_replay() {
    use crate::runtime::ports::{
        ProductEventAppendPort, ProductEventReplayPort, ProductGraphCursorPort,
    };
    use meld_world_model::world_state::graph::contracts::*;
    use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
    use meld_world_model::world_state::graph::store::TraversalStore;
    let temp = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let db = sled::open(temp.path().join("world")).unwrap();
    let store = Arc::new(TraversalStore::new(db.clone()).unwrap());
    let nonce = request("late-route");
    let emitted = emit(&authority.append_capability(), "late-route", &nonce).unwrap();
    authority
        .append_capability()
        .append_durable(
            meld_events::EventEnvelope::with_now_domain(
                "late-route",
                "unrelated",
                "history",
                "unrelated.event",
                None,
                serde_json::json!({}),
            ),
            meld_events::AppendMode::Plain,
        )
        .unwrap();
    let last = emit(
        &authority.append_capability(),
        "late-route",
        &request("other-history"),
    )
    .unwrap();
    let make_graph = |store: Arc<TraversalStore>| {
        GraphRuntime::from_ports(
            Arc::new(ProductEventReplayPort::new(authority.replay_capability())),
            Arc::new(ProductEventAppendPort::new(&authority)),
            Arc::new(ProductGraphCursorPort::new(
                authority.consumer_registry_capability(),
            )),
            store,
        )
        .unwrap()
    };
    let graph = make_graph(store.clone());
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
        .unwrap();
    assert_eq!(graph.durable_event_cursor().unwrap(), last.position);
    store.install_owner_event_route(&graph_route()).unwrap();
    let source = graph_route().source_ref().unwrap();
    let cut_request = TraversalCutRequest {
        owners: vec![TraversalOwnerRequirement {
            event_source: Some(source.clone()),
            owner_id: OWNER_ID.into(),
            scope: nonce.publication_scope(),
            required: true,
        }],
        scope: nonce.publication_scope(),
        currentness: OwnerCurrentnessPolicy::LatestComplete,
        event_position: last.position,
    };
    assert_eq!(
        meld_world_model::TraversalQuery::new(&store)
            .cut(&cut_request)
            .unwrap()
            .status,
        TraversalCutStatus::Incomplete
    );
    let pending = graph.lifecycle_evidence().unwrap();
    let first = graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 1 })
        .unwrap();
    assert_eq!(first.events_attempted, 1);
    assert_eq!(first.traversal_events_applied, 1);
    assert_eq!(first.source_replay.as_ref().unwrap().after.after_seq, 1);
    assert_eq!(first.input_event_seq, last.position.after_seq);
    assert_eq!(first.output_event_seq, last.position.after_seq);
    assert_eq!(
        meld_world_model::TraversalQuery::new(&store)
            .cut(&cut_request)
            .unwrap()
            .status,
        TraversalCutStatus::Incomplete
    );
    assert_ne!(
        graph.lifecycle_evidence().unwrap().binding_refs,
        pending.binding_refs
    );
    assert_eq!(
        authority
            .consumer_registry_capability()
            .get(&source.consumer_id())
            .unwrap()
            .unwrap()
            .reported_seq,
        1
    );
    drop(graph);
    drop(store);
    let store = Arc::new(TraversalStore::new(db).unwrap());
    let graph = make_graph(store.clone());
    let second = graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 1 })
        .unwrap();
    assert_eq!(second.source_replay.as_ref().unwrap().before.after_seq, 1);
    assert_eq!(second.source_replay.as_ref().unwrap().after.after_seq, 2);
    assert_eq!(second.traversal_events_applied, 0);
    let worker: crate::runtime::contracts::WorkerTickReport = second.into();
    assert_eq!(worker.items_committed, 1);
    assert_eq!(worker.input_checkpoint.name, source.consumer_id());
    assert_eq!(worker.input_checkpoint.value, 1);
    assert_eq!(worker.output_checkpoint.value, 2);
    assert!(!store
        .covers_event_source(emitted.position.ledger_id, OWNER_ID, &source)
        .unwrap());
    let third = graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 1 })
        .unwrap();
    assert_eq!(third.events_attempted, 1);
    assert_eq!(third.source_replay.unwrap().after, last.position);
    assert!(store
        .covers_event_source(emitted.position.ledger_id, OWNER_ID, &source)
        .unwrap());
    let cut = meld_world_model::TraversalQuery::new(&store)
        .cut(&cut_request)
        .unwrap();
    assert_eq!(cut.status, TraversalCutStatus::Complete);
    assert_eq!(
        cut.receipts[0].source_event.unwrap().seq,
        emitted.position.after_seq
    );
    assert_eq!(graph.durable_event_cursor().unwrap(), last.position);
    let new = emit(
        &authority.append_capability(),
        "late-route",
        &request("live-after-history"),
    )
    .unwrap();
    let live = graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 1 })
        .unwrap();
    assert!(live.source_replay.is_none());
    assert_eq!(live.output_event_seq, new.position.after_seq);
    assert_eq!(
        authority
            .consumer_registry_capability()
            .get(&source.consumer_id())
            .unwrap()
            .unwrap()
            .reported_seq,
        new.position.after_seq
    );
    assert_eq!(
        store
            .owner_publications_through_seq(u64::MAX)
            .unwrap()
            .len(),
        3
    );
}
