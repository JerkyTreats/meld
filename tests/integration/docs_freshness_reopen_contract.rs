use std::ops::Deref;
use std::sync::Arc;

#[path = "../../crates/meld-execution/tests/support/task_network.rs"]
mod task_network_support;

use meld::runtime::assembly::{ProductRuntimeAssembly, ProductRuntimeConfig};
use meld::runtime::contracts::WorkerTickReport;
use meld::runtime::ports::{ProductRuntimePorts, ProviderPortConfig};
use meld::runtime::storage::{OpenProductStores, ProductStorageLayout};
use meld_events::{AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope};
use meld_execution::planning::{
    PlanningPerspectiveRef, PlanningRequest, PlanningResult, PlanningWorldStateFrameRef,
    PlanningWorldStateRequest,
};
use meld_execution::task::{ArtifactProducerRef, ArtifactRecord};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::{Outcome, OutcomeStatus, Request as DispatchRequest};
use meld_execution::task_network::outcome::PublicationState;
use meld_execution::task_network::publication::{
    publish_pending_publications, EventAppendSink, PublicationPublishResult,
    PublishPendingPublicationsRequest,
};
use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::state::TaskStatus;
use meld_execution::task_network::store::SledTaskNetworkStore;
use meld_execution::task_network::{PublicationRuntime, TaskNetworkAuthority};
use meld_lang::{Condition, GoalLifecycle, Literal, Proposition, Term, WorldState};
use meld_world_model::agent::{
    AgentCurationDedupeKey, AgentGoalCurationRuntime, AgentQuery, AgentRegistration,
    AgentSatisfactionCurationRuntime, AgentSelectedGoalTick, AgentSemanticSelector, AgentStatus,
    AgentSubscription, SubscribeAgentCommand, BELIEF_REVISION_REVIEW_SOURCE,
};
use meld_world_model::belief::{
    BeliefConfigLoader, BeliefQuery, BeliefRuntime, DocsTaskEvidenceIngestionRuntime,
    DocsTaskEvidenceReplayRequest,
};
use meld_world_model::planner::{PlannerQuery, PLANNER_PROJECTION_VERSION};
use meld_world_model::TraversalQuery;
use serde_json::json;

use super::docs_freshness_fixture::{
    DocsFreshnessFirstProofFixture, AGENT_ID, ARTIFACT_ID, DIMENSION_ID, GRAPH_PERSPECTIVE_ID,
    GRAPH_PERSPECTIVE_KIND, OUTCOME_ID, PUBLICATION_EVENT_TYPE, PUBLICATION_ID,
    REQUIRED_ARTIFACT_TYPE_ID, SESSION_ID, TASK_ARTIFACT_REPO_ID, TASK_INSTANCE_ID,
    TASK_NETWORK_ID, WORKER_ID,
};

struct ReopenHarness {
    temp: tempfile::TempDir,
    layout: ProductStorageLayout,
    fixture: DocsFreshnessFirstProofFixture,
}

struct OpenReopenProduct {
    stores: OpenProductStores,
    authority: EventAuthority,
}

impl Deref for OpenReopenProduct {
    type Target = OpenProductStores;

    fn deref(&self) -> &Self::Target {
        &self.stores
    }
}

impl ReopenHarness {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path().join("runtime"));
        Self {
            temp,
            layout,
            fixture: DocsFreshnessFirstProofFixture::new(),
        }
    }

    fn open(&self) -> OpenReopenProduct {
        let _keep_tempdir = self.temp.path();
        let stores = OpenProductStores::open(&self.layout).unwrap();
        let db = sled::open(&self.layout.ledger_db).unwrap();
        let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
        OpenReopenProduct { stores, authority }
    }

    fn assembly(&self) -> ProductRuntimeAssembly {
        let _keep_tempdir = self.temp.path();
        self.layout.create_dirs().unwrap();
        let authority = Arc::new(
            EventAuthority::open(
                sled::open(&self.layout.ledger_db).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap(),
        );
        ProductRuntimeAssembly::load_with_authority(
            ProductRuntimeConfig::for_product_root(self.layout.root.clone()),
            authority,
        )
        .unwrap()
    }

    fn ports(&self, product: &OpenReopenProduct) -> ProductRuntimePorts {
        ProductRuntimePorts::from_authority(
            &product.stores,
            &product.authority,
            ProviderPortConfig::default(),
        )
        .unwrap()
    }

    fn task_evidence_runtime(
        &self,
        product: &OpenReopenProduct,
        ports: &ProductRuntimePorts,
    ) -> DocsTaskEvidenceIngestionRuntime {
        DocsTaskEvidenceIngestionRuntime::new(
            Arc::new(ports.event_replay().clone()),
            Arc::clone(&product.belief_store),
            Arc::clone(&product.traversal_store),
        )
    }

    fn flush_and_reopen(&self, product: OpenReopenProduct) -> OpenReopenProduct {
        product.stores.flush_boundary().unwrap();
        product.authority.append_capability().barrier().unwrap();
        drop(product);
        self.open()
    }

    fn open_network(&self, stores: &OpenProductStores) -> SledTaskNetworkStore {
        stores.task_networks.open_network(TASK_NETWORK_ID).unwrap()
    }
}

#[test]
fn minimal_runtime_flywheel_turn_persists_and_satisfies_goal() {
    let harness = ReopenHarness::new();
    let stores = setup_reopened_pending_publication(&harness);
    let ports = harness.ports(&stores);
    let network = harness.open_network(&stores);
    seed_event_allocator_before_publication(&stores, &harness.fixture);
    network.flush().unwrap();
    drop(network);

    let publication_runtime = PublicationRuntime::new();
    let mut authority =
        TaskNetworkAuthority::open(&stores.task_networks, TASK_NETWORK_ID, 32).unwrap();
    let query = authority.query_port();
    let commands = authority.command_port();
    let report = publication_runtime
        .publish_pending(
            &query,
            &commands,
            ports.event_append(),
            PublishPendingPublicationsRequest {
                session_id: SESSION_ID.to_string(),
                worker_id: WORKER_ID.to_string(),
                limit: Some(1),
            },
        )
        .unwrap();
    authority.shutdown().unwrap();
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 1);
    assert!(report.retryable_errors.is_empty(), "{report:?}");
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert!(report.output_revision > report.input_revision);

    stores.flush_boundary().unwrap();
    drop(ports);
    drop(stores);

    let stores = harness.open();
    let ports = harness.ports(&stores);
    let records = ports
        .event_replay()
        .read_after_limit(harness.fixture.publication_event_seq() - 1, 1)
        .unwrap();
    assert_eq!(records.len(), 1);
    let event = records[0].clone();
    assert_eq!(event.seq, harness.fixture.publication_event_seq());
    assert_eq!(event.envelope.event_type, PUBLICATION_EVENT_TYPE);
    let duplicate_seq = ports
        .event_append()
        .append_envelope_idempotent(event.envelope.clone())
        .unwrap();
    assert_eq!(duplicate_seq.seq, event.seq);

    let network = stores.task_networks.open_network(TASK_NETWORK_ID).unwrap();
    let publication = network
        .state()
        .publications
        .values()
        .next()
        .expect("expected published publication");
    assert!(matches!(
        &publication.state,
        PublicationState::Published {
            receipt: Some(receipt),
            ..
        } if receipt.seq == event.seq
    ));
    drop(network);

    let evidence_runtime = harness.task_evidence_runtime(&stores, &ports);
    let ingestion = evidence_runtime
        .ingest_after_limit(DocsTaskEvidenceReplayRequest {
            after_seq: harness.fixture.publication_event_seq() - 1,
            limit: 1,
            subject: harness.fixture.subject(),
            config: BeliefConfigLoader::load_json(harness.fixture.belief_config_json()).unwrap(),
            perspective: harness.fixture.perspective(),
            branch_scope: harness.fixture.branch_scope(),
            owner_id: WORKER_ID.to_string(),
            required_artifact_type_id: Some(REQUIRED_ARTIFACT_TYPE_ID.to_string()),
        })
        .unwrap();
    assert_eq!(ingestion.events_attempted, 1);
    assert_eq!(ingestion.promoted_evidence_count, 1);
    assert_eq!(ingestion.normalized_evidence_count, 2);
    assert_eq!(ingestion.new_assignment_count, 2);
    assert!(ingestion
        .ingestions
        .iter()
        .any(|result| !result.committed.is_empty()));
    drop(evidence_runtime);

    let (satisfaction_report, review) = {
        let belief_query = BeliefQuery::new(stores.belief_store.as_ref());
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(stores.belief_store.as_ref()),
            TraversalQuery::new(stores.traversal_store.as_ref()),
        );
        let selection = AgentSemanticSelector::new(stores.agent_store.as_ref())
            .select_satisfaction_reviews(&belief_query, BELIEF_REVISION_REVIEW_SOURCE, 1)
            .unwrap()
            .pop()
            .expect("expected satisfaction review selection");
        let review = selection.review.clone();
        let mut goal_query = ports.goal_command().clone();
        let mut outcome_query = ports.goal_command().clone();
        let mut mutation_sink = ports.goal_mutation().clone();
        let report = AgentSatisfactionCurationRuntime::new(stores.agent_store.as_ref())
            .handle_selected_review(
                selection,
                &belief_query,
                &planner_query,
                &mut goal_query,
                &mut outcome_query,
                &mut mutation_sink,
            );
        (report, review)
    };
    assert_eq!(satisfaction_report.delivered_count, 1);
    assert_eq!(satisfaction_report.decision_count, 1);
    assert_eq!(satisfaction_report.sink_submission_count, 1);
    assert!(satisfaction_report.retryable_errors.is_empty());
    assert!(satisfaction_report.fatal_errors.is_empty());
    assert_eq!(satisfaction_report.output_sequence, review.review_seq);

    let persisted = AgentQuery::new(stores.agent_store.as_ref())
        .decision_by_satisfaction_review(&review)
        .unwrap()
        .expect("expected persisted satisfaction decision");
    let persisted_outcome = AgentQuery::new(stores.agent_store.as_ref())
        .curation_outcome(&persisted.decision_id)
        .unwrap();
    let mutation = persisted_outcome
        .goal_mutation_command
        .expect("expected durable satisfaction command");
    assert_eq!(
        persisted.goal_mutation_command_id.as_deref(),
        Some(mutation.command_id.as_str())
    );
    assert_eq!(mutation.review_seq, review.review_seq);
    stores.flush_boundary().unwrap();
    drop(ports);
    drop(stores);

    let final_assembly = harness.assembly();
    assert_eq!(final_assembly.product_root(), harness.layout.root.as_path());
    let record = final_assembly
        .stores()
        .goal_store
        .get_goal(&harness.fixture.expected_goal_id())
        .unwrap()
        .expect("expected final goal record");
    assert_eq!(
        record.goal.lifecycle,
        harness.fixture.expected_final_lifecycle(review.review_seq)
    );
}

#[test]
fn docs_freshness_reopens_after_goal_acceptance_from_product_stores() {
    let harness = ReopenHarness::new();
    let stores = setup_reopened_active_goal(&harness);
    let fixture = harness.fixture;

    let record = stores
        .goal_store
        .get_goal(&fixture.expected_goal_id())
        .unwrap()
        .expect("expected reopened goal record");
    assert_eq!(record.goal.lifecycle, GoalLifecycle::Active);
    assert_eq!(
        record.source_identity.as_deref(),
        Some(fixture.expected_goal_source_identity().as_str())
    );

    let decision = AgentQuery::new(stores.agent_store.as_ref())
        .decision_by_dedupe_key(&threshold_dedupe_key(&fixture))
        .unwrap()
        .expect("expected persisted curation decision");
    assert_eq!(
        record.source_command_id.as_deref(),
        decision.goal_command_id.as_deref()
    );
    assert!(decision.goal_command_id.is_some());

    let active = stores.goal_store.active_goals().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].goal_id, fixture.expected_goal_id());
    assert_eq!(active[0].agent_id, AGENT_ID);
}

#[test]
fn docs_freshness_reopens_after_pending_publication_from_product_stores() {
    let harness = ReopenHarness::new();
    let stores = setup_reopened_pending_publication(&harness);
    let network = harness.open_network(&stores);

    assert!(matches!(
        network.state().statuses.get(TASK_INSTANCE_ID),
        Some(TaskStatus::Succeeded { outcome_id }) if outcome_id == OUTCOME_ID
    ));
    assert!(network.state().outcomes.contains_key(OUTCOME_ID));
    assert_eq!(network.state().publications.len(), 1);

    let publication = network
        .state()
        .publications
        .values()
        .next()
        .expect("expected pending publication");
    assert_eq!(publication.publication_id, PUBLICATION_ID);
    assert_eq!(publication.event_type(), PUBLICATION_EVENT_TYPE);
    assert!(matches!(publication.state, PublicationState::Pending));
    assert!(publication
        .event_payload()
        .get("artifact_records")
        .and_then(|value| value.as_array())
        .unwrap()
        .iter()
        .any(|artifact| artifact
            .get("artifact_type_id")
            .and_then(|value| value.as_str())
            == Some(REQUIRED_ARTIFACT_TYPE_ID)));

    let artifact_repo = stores
        .task_artifacts
        .open_repo(TASK_ARTIFACT_REPO_ID)
        .unwrap();
    let artifact = artifact_repo
        .get_artifact(ARTIFACT_ID)
        .expect("expected reopened task artifact");
    assert_eq!(artifact.artifact_type_id, REQUIRED_ARTIFACT_TYPE_ID);
    assert_eq!(artifact.content["patch"], json!("updated docs"));
}

#[test]
fn docs_freshness_reopens_after_publication_append_before_satisfaction() {
    let harness = ReopenHarness::new();
    let mut stores = setup_reopened_pending_publication(&harness);
    let network = harness.open_network(&stores);
    seed_event_allocator_before_publication(&stores, &harness.fixture);
    let publication_ports = harness.ports(&stores);
    network.flush().unwrap();
    drop(network);
    let mut authority =
        TaskNetworkAuthority::open(&stores.task_networks, TASK_NETWORK_ID, 32).unwrap();
    let query = authority.query_port();
    let commands = authority.command_port();

    let report = publish_pending_publications(
        &query,
        &commands,
        publication_ports.event_append(),
        PublishPendingPublicationsRequest {
            session_id: SESSION_ID.to_string(),
            worker_id: WORKER_ID.to_string(),
            limit: Some(1),
        },
    )
    .unwrap();
    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(matches!(
        report.results.as_slice(),
        [PublicationPublishResult::Published { .. }]
    ));
    let worker_report: WorkerTickReport = report.into();
    assert!(worker_report.made_progress());

    authority.shutdown().unwrap();
    drop(publication_ports);
    stores = harness.flush_and_reopen(stores);
    let network = harness.open_network(&stores);
    let ports = harness.ports(&stores);

    let records = ports
        .event_replay()
        .read_after_limit(harness.fixture.publication_event_seq() - 1, 1)
        .unwrap();
    assert_eq!(records.len(), 1);
    let event = records[0].clone();
    assert_eq!(event.seq, harness.fixture.publication_event_seq());
    assert_eq!(event.envelope.event_type, PUBLICATION_EVENT_TYPE);
    let publication = network
        .state()
        .publications
        .values()
        .next()
        .expect("expected published publication");
    assert_eq!(
        event.envelope.record_id.as_deref(),
        Some(
            DocsFreshnessFirstProofFixture::publication_record_id(&publication.publication_id)
                .as_str()
        )
    );
    assert!(matches!(
        &publication.state,
        PublicationState::Published {
            receipt: Some(receipt),
            ..
        } if receipt.seq == event.seq
    ));

    let evidence_runtime = harness.task_evidence_runtime(&stores, &ports);
    let ingestion = evidence_runtime
        .ingest_after_limit(DocsTaskEvidenceReplayRequest {
            after_seq: event.seq - 1,
            limit: 1,
            subject: harness.fixture.subject(),
            config: BeliefConfigLoader::load_json(harness.fixture.belief_config_json()).unwrap(),
            perspective: harness.fixture.perspective(),
            branch_scope: harness.fixture.branch_scope(),
            owner_id: WORKER_ID.to_string(),
            required_artifact_type_id: Some(REQUIRED_ARTIFACT_TYPE_ID.to_string()),
        })
        .unwrap();
    assert_eq!(ingestion.events_attempted, 1);
    assert_eq!(ingestion.promoted_evidence_count, 1);
    assert_eq!(ingestion.normalized_evidence_count, 2);
    assert_eq!(ingestion.new_assignment_count, 2);
    assert!(ingestion
        .ingestions
        .iter()
        .any(|result| !result.committed.is_empty()));

    let (satisfaction_report, review) = {
        let belief_query = BeliefQuery::new(stores.belief_store.as_ref());
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(stores.belief_store.as_ref()),
            TraversalQuery::new(stores.traversal_store.as_ref()),
        );
        let selection = AgentSemanticSelector::new(stores.agent_store.as_ref())
            .select_satisfaction_reviews(&belief_query, BELIEF_REVISION_REVIEW_SOURCE, 1)
            .unwrap()
            .pop()
            .expect("expected satisfaction review selection");
        let review = selection.review.clone();
        let mut goal_query = ports.goal_command().clone();
        let mut outcome_query = ports.goal_command().clone();
        let mut mutation_sink = ports.goal_mutation().clone();
        let report = AgentSatisfactionCurationRuntime::new(stores.agent_store.as_ref())
            .handle_selected_review(
                selection,
                &belief_query,
                &planner_query,
                &mut goal_query,
                &mut outcome_query,
                &mut mutation_sink,
            );
        (report, review)
    };
    assert_eq!(satisfaction_report.delivered_count, 1);
    assert_eq!(satisfaction_report.decision_count, 1);
    assert_eq!(satisfaction_report.sink_submission_count, 1);
    assert!(satisfaction_report.retryable_errors.is_empty());
    assert!(satisfaction_report.fatal_errors.is_empty());
    assert_eq!(satisfaction_report.output_sequence, review.review_seq);

    let persisted = AgentQuery::new(stores.agent_store.as_ref())
        .decision_by_satisfaction_review(&review)
        .unwrap()
        .expect("expected persisted satisfaction decision");
    let persisted_outcome = AgentQuery::new(stores.agent_store.as_ref())
        .curation_outcome(&persisted.decision_id)
        .unwrap();
    let mutation = persisted_outcome
        .goal_mutation_command
        .expect("expected durable satisfaction command");
    assert_eq!(
        persisted.goal_mutation_command_id.as_deref(),
        Some(mutation.command_id.as_str()),
        "{persisted:?}"
    );
    assert_eq!(mutation.review_seq, review.review_seq);

    let record = stores
        .goal_store
        .get_goal(&harness.fixture.expected_goal_id())
        .unwrap()
        .expect("expected final goal record");
    assert_eq!(
        record.goal.lifecycle,
        harness.fixture.expected_final_lifecycle(review.review_seq)
    );
}

fn mark_fixture_agent_operational(db: &sled::Db) {
    let records = db.open_tree("agent_records").unwrap();
    let statuses = db.open_tree("agent_by_status").unwrap();
    let mut agent: meld_world_model::AgentRecord = serde_json::from_slice(
        &records
            .get(AGENT_ID)
            .unwrap()
            .expect("registered fixture agent"),
    )
    .unwrap();
    let prior_status_key = format!(
        "{}::{:020}::{}",
        agent.status.index_key(),
        agent.updated_at_seq,
        agent.agent_id
    );
    agent.status = AgentStatus::Operational;
    agent.updated_at_seq += 1;
    records
        .insert(AGENT_ID, serde_json::to_vec(&agent).unwrap())
        .unwrap();
    statuses.remove(prior_status_key.as_bytes()).unwrap();
    statuses
        .insert(
            format!(
                "{}::{:020}::{}",
                agent.status.index_key(),
                agent.updated_at_seq,
                agent.agent_id
            )
            .as_bytes(),
            AGENT_ID.as_bytes(),
        )
        .unwrap();
    db.flush().unwrap();
}

fn setup_reopened_active_goal(harness: &ReopenHarness) -> OpenReopenProduct {
    let stores = harness.open();
    let fixture = harness.fixture;
    fixture.seed_graph_into(stores.traversal_store.as_ref());
    BeliefRuntime::from_json_config(
        stores.belief_store.clone(),
        stores.traversal_store.clone(),
        fixture.belief_config_json(),
    )
    .unwrap()
    .assess_subject(
        &fixture.subject(),
        GRAPH_PERSPECTIVE_KIND,
        GRAPH_PERSPECTIVE_ID,
        WORKER_ID,
    )
    .unwrap();
    AgentRegistration::new(stores.agent_store.as_ref())
        .register_seed_agent(fixture.seed_agent_registration())
        .unwrap();
    let initial_revision_seq = BeliefQuery::new(stores.belief_store.as_ref())
        .current_revision(&fixture.belief_key())
        .unwrap()
        .expect("initial assessed belief revision")
        .source_cursor_end;
    let subscription = AgentSubscription::new(stores.agent_store.as_ref())
        .subscribe(SubscribeAgentCommand {
            agent_id: AGENT_ID.to_string(),
            belief_key: fixture.belief_key(),
            created_at_seq: initial_revision_seq,
        })
        .unwrap();
    assert_eq!(
        subscription.subscription_id,
        fixture.expected_subscription_id()
    );
    stores.flush_boundary().unwrap();
    mark_fixture_agent_operational(stores.traversal_store.db());

    let ports = harness.ports(&stores);
    let belief_query = BeliefQuery::new(stores.belief_store.as_ref());
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(stores.belief_store.as_ref()),
        TraversalQuery::new(stores.traversal_store.as_ref()),
    );
    let selection = AgentSemanticSelector::new(stores.agent_store.as_ref())
        .select_deliveries(&belief_query, 1)
        .unwrap()
        .pop()
        .expect("expected goal delivery selection");
    assert_eq!(selection.delivery.agent_id, AGENT_ID);
    assert_eq!(
        selection.delivery.subscription_id,
        subscription.subscription_id
    );
    let selected_revision_seq = selection.delivery.revision_seq;
    let mut goal_query = ports.goal_command().clone();
    let mut outcome_query = ports.goal_command().clone();
    let mut sink = ports.goal_command().clone();
    let report = AgentGoalCurationRuntime::new(stores.agent_store.as_ref())
        .handle_selected_delivery(
            AgentSelectedGoalTick {
                selection,
                belief_query: &belief_query,
                planner_query: &planner_query,
                rule_config: fixture.curation_rule_config(),
            },
            &mut goal_query,
            &mut outcome_query,
            &mut sink,
        );
    assert_eq!(report.delivered_count, 1);
    assert_eq!(report.decision_count, 1);
    assert_eq!(report.sink_submission_count, 1);
    assert!(report.retryable_errors.is_empty(), "{report:?}");
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert_eq!(report.output_sequence, selected_revision_seq);
    drop(goal_query);
    drop(outcome_query);
    drop(sink);
    drop(ports);

    harness.flush_and_reopen(stores)
}

fn setup_reopened_pending_publication(harness: &ReopenHarness) -> OpenReopenProduct {
    let stores = setup_reopened_active_goal(harness);
    let active_goal = stores
        .goal_store
        .active_goal(&harness.fixture.expected_goal_id())
        .unwrap()
        .expect("expected active goal");
    let planned = harness
        .fixture
        .planning_runtime()
        .plan_goal(planning_request(
            active_goal,
            world_state_below_threshold(&harness.fixture),
        ))
        .unwrap();
    assert!(matches!(
        planned,
        PlanningResult::Composed(ref composition)
            if composition.method_id == super::docs_freshness_fixture::METHOD_ID
    ));

    let mut network = harness.open_network(&stores);
    let response = network
        .submit(task_network_support::apply_sled_command(
            &network,
            "command-plan-docs",
            Command::ApplyMutationSet(task_network_support::single_task_mutation_set(
                TASK_INSTANCE_ID,
            )),
        ))
        .unwrap();
    assert!(matches!(response, Response::Accepted { .. }));
    claim_ready_task(&mut network);
    record_successful_outcome(&stores, &mut network);

    network.flush().unwrap();
    drop(network);
    harness.flush_and_reopen(stores)
}

fn claim_ready_task(network: &mut SledTaskNetworkStore) {
    let ready = compute_ready_set(network.state());
    assert_eq!(ready.task_instance_ids, vec![TASK_INSTANCE_ID.to_string()]);
    let request = DispatchRequest {
        claim_id: "claim-alpha".to_string(),
        task_instance_id: TASK_INSTANCE_ID.to_string(),
        worker_id: WORKER_ID.to_string(),
        idempotency_key: "claim-alpha-once".to_string(),
    };
    let response = network
        .submit(task_network_support::apply_sled_command(
            network,
            "command-claim-docs",
            Command::ClaimReadyTask(request),
        ))
        .unwrap();
    assert!(matches!(response, Response::Accepted { .. }));
}

fn record_successful_outcome(stores: &OpenProductStores, network: &mut SledTaskNetworkStore) {
    let claim = network
        .state()
        .claims
        .get("claim-alpha")
        .expect("expected claim")
        .clone();
    let artifact = ArtifactRecord {
        artifact_id: ARTIFACT_ID.to_string(),
        artifact_type_id: REQUIRED_ARTIFACT_TYPE_ID.to_string(),
        schema_version: 1,
        content: json!({
            "patch": "updated docs",
        }),
        producer: ArtifactProducerRef {
            task_id: format!("compiled-{TASK_INSTANCE_ID}"),
            capability_instance_id: "write".to_string(),
            invocation_id: Some(format!("invoke-{OUTCOME_ID}")),
            output_slot_id: Some("patch".to_string()),
        },
    };
    let mut artifact_repo = stores
        .task_artifacts
        .open_repo(TASK_ARTIFACT_REPO_ID)
        .unwrap();
    artifact_repo.append_artifact(artifact.clone()).unwrap();
    artifact_repo.flush().unwrap();

    let outcome = Outcome {
        outcome_id: OUTCOME_ID.to_string(),
        task_instance_id: TASK_INSTANCE_ID.to_string(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id,
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Succeeded,
        error: None,
        artifact_records: vec![artifact],
        task_events: Vec::new(),
    };
    let response = network
        .submit(task_network_support::apply_sled_command(
            network,
            "command-outcome-docs",
            Command::RecordTaskOutcome(outcome),
        ))
        .unwrap();
    assert!(matches!(response, Response::Accepted { .. }));
}

fn seed_event_allocator_before_publication(
    product: &OpenReopenProduct,
    fixture: &DocsFreshnessFirstProofFixture,
) {
    let prior_seq = fixture.publication_event_seq() - 1;
    let envelopes = (1..=prior_seq)
        .map(|seq| {
            EventEnvelope::new_domain(
                "2026-06-09T00:00:00Z".to_string(),
                SESSION_ID,
                "runtime",
                "rtg-6::checkpoint",
                "runtime.checkpoint.before_publication",
                Some(format!("hash-before-publication-{seq}")),
                json!({
                    "checkpoint": "before_publication",
                    "allocator_seq": seq,
                }),
            )
            .with_record_id(format!("runtime::checkpoint::before-publication::{seq}"))
        })
        .collect();
    let receipts = product
        .authority
        .append_capability()
        .append_durable_batch(envelopes, AppendMode::Plain)
        .unwrap();
    assert_eq!(receipts.last().map(|receipt| receipt.seq), Some(prior_seq));
}

fn planning_request(goal: meld_lang::Goal, world_state: WorldState) -> PlanningRequest {
    let world_state_request = PlanningWorldStateRequest::for_goal(
        &goal,
        1,
        PlanningPerspectiveRef::new("default", "default").unwrap(),
        "main",
        vec![DIMENSION_ID.to_string()],
        Vec::new(),
    )
    .unwrap();
    let world_state_frame = PlanningWorldStateFrameRef::identified_from_authority(
        PLANNER_PROJECTION_VERSION,
        blake3::hash(b"projection-hash-1").to_hex().to_string(),
        meld_execution::planning::world_state::canonical_world_state_hash(&world_state).unwrap(),
        &world_state_request,
        &world_state,
        vec![serde_json::json!({"ProjectionRule": {"rule_id": "source"}}).to_string()],
        Vec::new(),
    )
    .unwrap();
    PlanningRequest {
        request_id: "request-1".to_string(),
        world_state_request,
        goal,
        world_state,
        world_state_frame,
    }
}

fn world_state_below_threshold(fixture: &DocsFreshnessFirstProofFixture) -> WorldState {
    WorldState::new(vec![
        Proposition::Accessible {
            scope: fixture.subject_term(),
        },
        Proposition::Holds {
            subject: fixture.subject_term(),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
        },
    ])
    .unwrap()
}

fn threshold_dedupe_key(fixture: &DocsFreshnessFirstProofFixture) -> AgentCurationDedupeKey {
    AgentCurationDedupeKey::threshold_rule(
        AGENT_ID.to_string(),
        &fixture.subject(),
        &fixture.branch_scope(),
        &fixture.curation_rule_config(),
    )
}
