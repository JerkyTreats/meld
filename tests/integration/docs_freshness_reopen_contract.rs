use std::ops::Deref;
use std::sync::Arc;

#[path = "../../crates/meld-execution/tests/support/task_network.rs"]
mod task_network_support;

use meld::runtime::assembly::{ProductRuntimeAssembly, ProductRuntimeConfig};
use meld::runtime::contracts::WorkerTickReport;
use meld::runtime::ports::{
    DocsTaskEvidenceReplayRequest, ProductRuntimePorts, ProviderPortConfig,
};
use meld::runtime::storage::{OpenProductStores, ProductStorageLayout};
use meld_events::{AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope};
use meld_execution::goals::GoalCommandOutcome;
use meld_execution::planning::{
    PlanningRequest, PlanningResult, PlanningWorldStateFrameRef, PlanningWorldStateRequest,
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
use meld_execution::task_network::PublicationRuntime;
use meld_lang::{Condition, GoalLifecycle, Literal, Proposition, Term, WorldState};
use meld_world_model::agent::{
    ActiveGoalSummary, AgentActiveGoalQueryError, AgentCurationDedupeKey, AgentDelivery,
    AgentGoalCommand, AgentGoalCurationRuntime, AgentGoalMutationCommand, AgentQuery,
    AgentRegistration, AgentSatisfactionCurationRuntime, AgentSatisfactionReview, AgentSinkError,
    AgentSinkSubmission, AgentSubscription, SubscribeAgentCommand,
};
use meld_world_model::belief::{
    BeliefConfigLoader, BeliefProvenanceSummary, BeliefQuery, BeliefRuntime, ContradictionState,
    FreshnessState, HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::planner::{PlannerQuery, PLANNER_PROJECTION_VERSION};
use meld_world_model::{BeliefStatus, BeliefView, TraversalQuery};
use serde_json::json;

use super::docs_freshness_fixture::{
    DocsFreshnessFirstProofFixture, AGENT_ID, ARTIFACT_ID, DIMENSION_ID, GOAL_COMMAND_REVISION_ID,
    GRAPH_PERSPECTIVE_ID, GRAPH_PERSPECTIVE_KIND, OUTCOME_ID, PUBLICATION_EVENT_TYPE,
    PUBLICATION_ID, REQUIRED_ARTIFACT_TYPE_ID, SESSION_ID, TASK_ARTIFACT_REPO_ID, TASK_INSTANCE_ID,
    TASK_NETWORK_ID, THRESHOLD, WORKER_ID,
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
    let mut network = harness.open_network(&stores);
    seed_event_allocator_before_publication(&stores, &harness.fixture);

    let publication_runtime = PublicationRuntime::new();
    let report = publication_runtime
        .publish_pending(
            &mut network,
            ports.event_append(),
            PublishPendingPublicationsRequest {
                session_id: SESSION_ID.to_string(),
                worker_id: WORKER_ID.to_string(),
                limit: Some(1),
            },
        )
        .unwrap();
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(report.output_revision > report.input_revision);

    network.flush().unwrap();
    drop(network);
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

    let ingestion = ports
        .docs_task_evidence()
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

    let review = AgentSatisfactionReview {
        agent_id: AGENT_ID.to_string(),
        subscription_id: harness.fixture.expected_subscription_id(),
        review_seq: harness.fixture.satisfaction_review_seq(),
    };
    let mut goal_query = |_agent_id: &str| {
        stores
            .goal_store
            .active_goals()
            .map(|goals| ActiveGoalSummary { goals })
            .map_err(|error| AgentActiveGoalQueryError::retryable(error.to_string()))
    };
    let mut mutation_sink = |command: &AgentGoalMutationCommand| match ports
        .goal_mutation()
        .satisfy_agent_goal_mutation(command.clone())
    {
        Ok(outcome) => submission_from_mutation_outcome(command, outcome),
        Err(error) => Err(AgentSinkError::retryable(error.to_string())),
    };
    let satisfaction_report = {
        let belief_query = BeliefQuery::new(stores.belief_store.as_ref());
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(stores.belief_store.as_ref()),
            TraversalQuery::new(stores.traversal_store.as_ref()),
        );
        AgentSatisfactionCurationRuntime::new(stores.agent_store.as_ref())
            .handle_review_with_goal_query(
                review.clone(),
                &belief_query,
                &planner_query,
                &mut goal_query,
                &mut mutation_sink,
            )
    };
    assert_eq!(satisfaction_report.delivered_count, 1);
    assert_eq!(satisfaction_report.decision_count, 1);
    assert_eq!(satisfaction_report.sink_submission_count, 1);
    assert!(satisfaction_report.retryable_errors.is_empty());
    assert!(satisfaction_report.fatal_errors.is_empty());
    assert_eq!(
        satisfaction_report.output_sequence,
        harness.fixture.satisfaction_review_seq()
    );

    let persisted = AgentQuery::new(stores.agent_store.as_ref())
        .decision_by_satisfaction_review(&review)
        .unwrap()
        .expect("expected persisted satisfaction decision");
    assert_eq!(
        persisted.goal_mutation_command_id.as_deref(),
        Some(
            harness
                .fixture
                .expected_satisfaction_mutation_command_id()
                .as_str()
        )
    );
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
        harness
            .fixture
            .expected_final_lifecycle(harness.fixture.satisfaction_review_seq())
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
        record.source_command_id.as_deref(),
        Some(fixture.expected_goal_command_id().as_str())
    );
    assert_eq!(
        record.source_identity.as_deref(),
        Some(fixture.expected_goal_source_identity().as_str())
    );

    let decision = AgentQuery::new(stores.agent_store.as_ref())
        .decision_by_dedupe_key(&threshold_dedupe_key(&fixture))
        .unwrap()
        .expect("expected persisted curation decision");
    assert_eq!(
        decision.goal_command_id.as_deref(),
        Some(fixture.expected_goal_command_id().as_str())
    );

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
    let mut network = harness.open_network(&stores);
    seed_event_allocator_before_publication(&stores, &harness.fixture);
    let publication_ports = harness.ports(&stores);

    let report = publish_pending_publications(
        &mut network,
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

    network.flush().unwrap();
    drop(network);
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

    let ingestion = ports
        .docs_task_evidence()
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

    let review = AgentSatisfactionReview {
        agent_id: AGENT_ID.to_string(),
        subscription_id: harness.fixture.expected_subscription_id(),
        review_seq: harness.fixture.satisfaction_review_seq(),
    };
    let mut goal_query = |_agent_id: &str| {
        stores
            .goal_store
            .active_goals()
            .map(|goals| ActiveGoalSummary { goals })
            .map_err(|error| AgentActiveGoalQueryError::retryable(error.to_string()))
    };
    let mut mutation_sink = |command: &AgentGoalMutationCommand| match ports
        .goal_mutation()
        .satisfy_agent_goal_mutation(command.clone())
    {
        Ok(outcome) => submission_from_mutation_outcome(command, outcome),
        Err(error) => Err(AgentSinkError::retryable(error.to_string())),
    };
    let satisfaction_report = {
        let belief_query = BeliefQuery::new(stores.belief_store.as_ref());
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(stores.belief_store.as_ref()),
            TraversalQuery::new(stores.traversal_store.as_ref()),
        );
        AgentSatisfactionCurationRuntime::new(stores.agent_store.as_ref())
            .handle_review_with_goal_query(
                review.clone(),
                &belief_query,
                &planner_query,
                &mut goal_query,
                &mut mutation_sink,
            )
    };
    assert_eq!(satisfaction_report.delivered_count, 1);
    assert_eq!(satisfaction_report.decision_count, 1);
    assert_eq!(satisfaction_report.sink_submission_count, 1);
    assert!(satisfaction_report.retryable_errors.is_empty());
    assert!(satisfaction_report.fatal_errors.is_empty());
    assert_eq!(
        satisfaction_report.output_sequence,
        harness.fixture.satisfaction_review_seq()
    );

    let persisted = AgentQuery::new(stores.agent_store.as_ref())
        .decision_by_satisfaction_review(&review)
        .unwrap()
        .expect("expected persisted satisfaction decision");
    assert_eq!(
        persisted.goal_mutation_command_id.as_deref(),
        Some(
            harness
                .fixture
                .expected_satisfaction_mutation_command_id()
                .as_str()
        ),
        "{persisted:?}"
    );

    let record = stores
        .goal_store
        .get_goal(&harness.fixture.expected_goal_id())
        .unwrap()
        .expect("expected final goal record");
    assert_eq!(
        record.goal.lifecycle,
        harness
            .fixture
            .expected_final_lifecycle(harness.fixture.satisfaction_review_seq())
    );
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
    let subscription = AgentSubscription::new(stores.agent_store.as_ref())
        .subscribe(SubscribeAgentCommand {
            agent_id: AGENT_ID.to_string(),
            belief_key: fixture.belief_key(),
            created_at_seq: fixture.goal_acceptance_seq(),
        })
        .unwrap();
    assert_eq!(
        subscription.subscription_id,
        fixture.expected_subscription_id()
    );
    stores
        .belief_store
        .put_view(&belief_view(&fixture, 0.2, GOAL_COMMAND_REVISION_ID, 7))
        .unwrap();

    let ports = harness.ports(&stores);
    let mut sink = |command: &AgentGoalCommand| match ports
        .goal_command()
        .accept_agent_goal_command(command.clone(), fixture.goal_acceptance_seq())
    {
        Ok(outcome) => submission_from_goal_outcome(command, outcome),
        Err(error) => Err(AgentSinkError::retryable(error.to_string())),
    };
    let belief_query = BeliefQuery::new(stores.belief_store.as_ref());
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(stores.belief_store.as_ref()),
        TraversalQuery::new(stores.traversal_store.as_ref()),
    );
    let delivery = AgentDelivery {
        agent_id: AGENT_ID.to_string(),
        subscription_id: subscription.subscription_id,
        belief_revision_id: GOAL_COMMAND_REVISION_ID.to_string(),
        revision_seq: fixture.goal_acceptance_seq(),
    };
    let mut goal_query = |_agent_id: &str| {
        stores
            .goal_store
            .active_goals()
            .map(|goals| ActiveGoalSummary { goals })
            .map_err(|error| AgentActiveGoalQueryError::retryable(error.to_string()))
    };
    let report = AgentGoalCurationRuntime::new(stores.agent_store.as_ref())
        .handle_delivery_with_goal_query(
            delivery,
            &belief_query,
            &planner_query,
            &mut goal_query,
            fixture.curation_rule_config(),
            &mut sink,
        );
    assert_eq!(report.delivered_count, 1);
    assert_eq!(report.decision_count, 1);
    assert_eq!(report.sink_submission_count, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert_eq!(report.output_sequence, fixture.goal_acceptance_seq());
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

fn submission_from_goal_outcome(
    command: &AgentGoalCommand,
    outcome: GoalCommandOutcome,
) -> Result<AgentSinkSubmission, AgentSinkError> {
    match outcome {
        GoalCommandOutcome::Applied(record) => Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            record.goal.goal_id.clone(),
            "applied",
        )),
        GoalCommandOutcome::Duplicate { existing_goal_id } => Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            existing_goal_id,
            "duplicate",
        )),
        GoalCommandOutcome::NotFound { goal_id } => Err(AgentSinkError::fatal(format!(
            "goal command sink did not find goal '{goal_id}'"
        ))),
    }
}

fn submission_from_mutation_outcome(
    command: &AgentGoalMutationCommand,
    outcome: GoalCommandOutcome,
) -> Result<AgentSinkSubmission, AgentSinkError> {
    match outcome {
        GoalCommandOutcome::Applied(record) => Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            record.goal.goal_id.clone(),
            "applied",
        )),
        GoalCommandOutcome::Duplicate { existing_goal_id } => Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            existing_goal_id,
            "duplicate",
        )),
        GoalCommandOutcome::NotFound { goal_id } => Err(AgentSinkError::fatal(format!(
            "goal mutation sink did not find goal '{goal_id}'"
        ))),
    }
}

fn planning_request(goal: meld_lang::Goal, world_state: WorldState) -> PlanningRequest {
    PlanningRequest {
        request_id: "request-1".to_string(),
        world_state_request: PlanningWorldStateRequest {
            goal_id: goal.goal_id.clone(),
            agent_id: goal.agent_id.clone(),
            target: goal.target.clone(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            requested_dimensions: vec![DIMENSION_ID.to_string()],
            required_preconditions: Vec::new(),
        },
        goal,
        world_state,
        world_state_frame: PlanningWorldStateFrameRef {
            frame_id: "frame-1".to_string(),
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            source_refs: vec!["source".to_string()],
            warnings: Vec::new(),
        },
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

fn belief_view(
    fixture: &DocsFreshnessFirstProofFixture,
    confidence: f64,
    revision_id: &str,
    seq: u64,
) -> BeliefView {
    let key = fixture.belief_key();
    BeliefView {
        view_id: format!("view-{revision_id}"),
        key,
        current_revision_id: Some(revision_id.to_string()),
        status: BeliefStatus::Settled,
        posterior: PosteriorSummary {
            probability: confidence,
            meaning: "probability".to_string(),
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: "confidence".to_string(),
            confidence,
            threshold: THRESHOLD,
        },
        uncertainty: 1.0 - confidence,
        precision: 1.0,
        freshness: FreshnessState {
            stale: false,
            reasons: Vec::new(),
            high_water_seq: seq,
        },
        contradiction: ContradictionState {
            contradicted: false,
            reasons: Vec::new(),
            supporting_evidence_ids: Vec::new(),
            contradicted_evidence_ids: Vec::new(),
        },
        observation: None,
        assessment_state: "settled".to_string(),
        advisory_posture: "ready".to_string(),
        provenance: BeliefProvenanceSummary {
            evidence_ids: vec![format!("evidence-{revision_id}")],
            source_fact_ids: vec![format!("ledger-{seq}")],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            objects: vec![fixture.subject()],
            relations: Vec::new(),
            revision_ids: vec![revision_id.to_string()],
        },
        hydration: HydrationRefs {
            evidence_ids: vec![format!("evidence-{revision_id}")],
            source_fact_ids: vec![format!("ledger-{seq}")],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            revision_id: Some(revision_id.to_string()),
        },
    }
}

fn threshold_dedupe_key(fixture: &DocsFreshnessFirstProofFixture) -> AgentCurationDedupeKey {
    AgentCurationDedupeKey::threshold_rule(
        AGENT_ID.to_string(),
        &fixture.subject(),
        &fixture.branch_scope(),
        &fixture.curation_rule_config(),
    )
}
