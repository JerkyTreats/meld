//! Shared fixtures are compiled separately by integration targets with different needs.
#![allow(dead_code)]

use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendMode, AppendReceipt, EventAuthority, EventAuthorityOpenOptions,
    EventConsumerRegistryCapability, EventEnvelope, EventPage, EventReplayCapability, LedgerCursor,
    LedgerIdentity, ReplayRequest,
};
use meld_world_model::error::StorageError;
use meld_world_model::world_state::graph::runtime::GraphRuntime;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::{GraphConsumerCursorReporter, GraphEventReplaySource};

const GRAPH_ACTOR_ID: &str = "world_state.graph.reducer";

#[derive(Clone)]
struct TestGraphPorts {
    replay: EventReplayCapability,
    registry: EventConsumerRegistryCapability,
}

impl GraphEventReplaySource for TestGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

impl GraphConsumerCursorReporter for TestGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.registry.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        self.registry.report(GRAPH_ACTOR_ID, cursor)?;
        Ok(())
    }
}

pub struct GraphRuntimeTestFixture {
    authority: EventAuthority,
    runtime: Arc<GraphRuntime>,
}

impl GraphRuntimeTestFixture {
    pub fn open(db: sled::Db) -> Result<Self, StorageError> {
        let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default())
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let ports = Arc::new(TestGraphPorts {
            replay: authority.replay_capability(),
            registry: authority.consumer_registry_capability(),
        });
        let traversal = TraversalStore::shared(db)?;
        let runtime = Arc::new(GraphRuntime::from_ports(ports.clone(), ports, traversal)?);
        Ok(Self { authority, runtime })
    }

    pub fn runtime(&self) -> Arc<GraphRuntime> {
        Arc::clone(&self.runtime)
    }

    pub fn append(&self, envelope: EventEnvelope) -> Result<AppendReceipt, EventAuthorityError> {
        self.authority
            .append_capability()
            .append_durable(envelope, AppendMode::Plain)
    }

    pub fn records(&self) -> Result<Vec<meld_events::EventRecord>, EventAuthorityError> {
        let replay = self.authority.replay_capability();
        let page = replay.replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: replay.ledger_identity(),
                after_seq: 0,
            },
            limit: meld_events::MAX_REPLAY_LIMIT,
        })?;
        Ok(page.records)
    }
}

/// Exercise native assembly for tests focused on Belief projection. The owner
/// scope is optional because these cases deliberately supply no Graph knowledge.
pub fn project_belief(
    belief: &meld_world_model::belief::BeliefStore,
    graph: &TraversalStore,
    key: &meld_world_model::belief::BeliefKey,
) -> meld_world_model::WorldModelView {
    use meld_world_model::graph::contracts::*;
    use meld_world_model::planner::*;
    let fixture = GraphRuntimeTestFixture::open(graph.db().clone()).unwrap();
    let runtime = fixture.runtime();
    runtime.catch_up().unwrap();
    let scope = OwnerPublicationScope {
        scope_id: "belief-projection-test".into(),
        branch_id: Some(key.branch_scope.branch_id.clone()),
        perspective_id: Some(key.perspective.perspective_id.clone()),
        valid_at: None,
    };
    let outcome = PlannerQuery::new(
        meld_world_model::belief::BeliefQuery::new(belief),
        meld_world_model::TraversalQuery::new(graph),
    )
    .assemble_current(PlannerCurrentAssemblyRequest {
        required_derived_evidence: None,
        required_graph_evidence: Vec::new(),
        additional_beliefs: Vec::new(),
        context: PlannerDecisionContext {
            context_id: "projection-context".into(),
            agent_id: "test-agent".into(),
            goal_id: "test-goal".into(),
            subject: key.subject.clone(),
            observation_subject: None,
            scope_id: scope.scope_id.clone(),
            branch_id: key.branch_scope.branch_id.clone(),
            perspective_id: key.perspective.perspective_id.clone(),
            authority_scope_id: "test-authority".into(),
            activation_generation: "test-generation".into(),
            admission_epoch: None,
        },
        policy: PlannerAssemblyPolicy {
            acquisition_question: None,
            policy_revision_id: "belief-projection-policy".into(),
            required_sources: vec![PlannerSourceKind::Graph, PlannerSourceKind::Belief],
            explicitly_not_required: vec![PlannerSourceKind::Causation, PlannerSourceKind::Regime],
        },
        traversal_cut_request: TraversalCutRequest {
            owners: vec![TraversalOwnerRequirement {
                owner_id: key.subject.domain_id.clone(),
                scope: scope.clone(),
                required: false,
                event_source: None,
            }],
            scope,
            event_position: runtime.durable_event_cursor().unwrap(),
            currentness: OwnerCurrentnessPolicy::LatestComplete,
        },
        traversal_request: BoundedTraversalRequest {
            roots: vec![key.subject.clone()],
            direction: TraversalDirection::Both,
            relation_types: None,
            bounds: TraversalBounds {
                max_depth: 1,
                max_objects: 8,
                max_occurrences: 8,
                max_paths: 8,
            },
        },
        belief_key: key.clone(),
        source_positions: Vec::new(),
    });
    match outcome {
        PlannerAssemblyOutcome::Complete(cut) => cut.world_model_view,
        PlannerAssemblyOutcome::Refused(reason) => panic!("native projection refused: {reason:?}"),
    }
}
