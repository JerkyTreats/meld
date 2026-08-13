//! Domain Convergence Wave exit evidence: the Goal-and-Belief isolate.
//!
//! Executes the worked scenario from the agent-native debugger requirements
//! as a scripted session — the first live composition of the epistemic
//! upstream over one shared temporary root. Stage 2 installs the theory,
//! stage 3 registers the genesis identities, stage 4 seeds epistemic facts
//! through the canonical append capability only, and the session then steps
//! evidence ingestion, belief assessment, and goal curation actor by actor
//! with injected sequences and explicit budgets. Every stimulus enters
//! through the event append capability or a public domain command, and every
//! inspection reads through the public query facades or actor reports. Docs
//! vocabulary appears only in these fixtures.

use std::sync::Arc;

use meld_lang::GoalLifecycle;
use meld_world_model::agent::{
    ActiveGoalSummary, AgentActiveGoalQueryError, AgentCurationRuleBinding,
    AgentCurationRuleConfig, AgentDecisionKind, AgentGoalCommand, AgentGoalCurationActor,
    AgentGoalMutationCommand, AgentQuery, AgentRegistration, AgentSinkError, AgentSinkSubmission,
    AgentStatus, AgentStepRequest, AgentStore, AgentSubscription, SeedAgentRegistration,
    SubscribeAgentCommand,
};
use meld_world_model::belief::{
    configured_belief_key, BeliefAssessmentActor, BeliefAssessmentRequest, BeliefFamilyRegistry,
    BeliefFamilyRegistryStore, BeliefFamilyRevision, BeliefKey, BeliefQuery, BeliefStore,
    BeliefSubjectBinding, BranchScope, ConfiguredOutcomeMapping, EvidenceEventReplaySource,
    EvidenceIngestionActor, EvidenceIngestionRequest, OutcomeContentRule, OutcomeFieldRule,
    OutcomeMappingConfig, OutcomeSubjectBinding, OutcomeValueSource, TheoryInstallDisposition,
    EVIDENCE_CONSUMER_ID,
};
use meld_world_model::events::error::EventAuthorityError;
use meld_world_model::events::{
    AppendMode, DomainObjectRef, DurableConsumerCursor, EventAuthority, EventAuthorityOpenOptions,
    EventEnvelope, EventPage, EventReplayCapability, LedgerIdentity, ReplayRequest,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::PerspectiveKey;
use serde_json::json;

const FAMILY_ID: &str = "docs_freshness";
const MAPPING_ID: &str = "docs_success_to_content_written";
const AGENT_ID: &str = "seed.docs_freshness";
const RULE_THRESHOLD: f64 = 0.7;

const INGESTION_ACTOR_ID: &str = "world_model.evidence.actor";
const ASSESSMENT_ACTOR_ID: &str = "world_model.belief.assessment";
const CURATION_ACTOR_ID: &str = "world_model.agent.goal_curation";

fn subject() -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap()
}

fn default_perspective() -> PerspectiveKey {
    PerspectiveKey::new("default", "default").unwrap()
}

/// Stage 2 theory content: the belief family installed as JSON config.
fn family_config_json() -> &'static str {
    r#"{
        "family_id": "docs_freshness",
        "dimension_id": "docs_freshness",
        "predicate_id": "confidence",
        "evidence_policy_id": "default_policy",
        "evidence_schemas": [
            {
                "schema_id": "content_written_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            },
            {
                "schema_id": "content_review_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }
        ],
        "source_mappings": [
            {
                "mapping_id": "content_written_to_signal",
                "source_kind": "content_written",
                "evidence_schema_id": "content_written_signal",
                "subject_from": "record.subject",
                "value_field": "stale_probability",
                "factor_id": "content_written_signal"
            },
            {
                "mapping_id": "content_written_to_review",
                "source_kind": "content_written",
                "evidence_schema_id": "content_review_signal",
                "subject_from": "record.subject",
                "value_field": "review_probability",
                "factor_id": "content_review_signal"
            }
        ],
        "comparator": {
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [
                {
                    "factor_id": "content_written_signal",
                    "evidence_schema_id": "content_written_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                },
                {
                    "factor_id": "content_review_signal",
                    "evidence_schema_id": "content_review_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }
            ],
            "missing_evidence_uncertainty": 0.9
        },
        "default_prior": 0.8,
        "planner_projection": {
            "confidence_field": "confidence",
            "threshold": 0.7,
            "posterior_meaning": "stale_probability"
        },
        "config_version": "1"
    }"#
}

/// Outcome mapping consistent with the installed family: publication
/// outcomes for docs patches promote to `content_written` evidence.
fn mapping_config() -> OutcomeMappingConfig {
    OutcomeMappingConfig {
        mapping_id: MAPPING_ID.to_string(),
        source_kind: "content_written".to_string(),
        match_domain_id: "execution".to_string(),
        match_event_type: "execution.task.succeeded".to_string(),
        match_content: vec![OutcomeContentRule::ArrayAnyFieldEquals {
            array_pointer: "/artifact_records".to_string(),
            field: "artifact_type_id".to_string(),
            equals: "docs_patch".to_string(),
        }],
        subject: OutcomeSubjectBinding {
            object_kind: "node".to_string(),
            domain_id: Some("workspace_fs".to_string()),
            // Additive subject-source field; this fixture keeps the original
            // envelope-object binding semantics.
            from: Default::default(),
        },
        evidence_fields: vec![
            OutcomeFieldRule {
                field: "stale_probability".to_string(),
                source: OutcomeValueSource::Constant { value: 0.0 },
            },
            OutcomeFieldRule {
                field: "review_probability".to_string(),
                source: OutcomeValueSource::Constant { value: 0.2 },
            },
        ],
    }
}

fn curation_rule() -> AgentCurationRuleConfig {
    AgentCurationRuleConfig {
        dimension_id: FAMILY_ID.to_string(),
        threshold: RULE_THRESHOLD,
        priority_urgency: 50,
        desired_summary: "confidence>0.7".to_string(),
        source_kind: "belief_divergence".to_string(),
    }
}

/// Stage 4 stimulus: a synthetic publication outcome the installed mapping
/// promotes to evidence. Timestamps are injected fixture constants.
fn publication_envelope(record_id: &str) -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-07-24T00:00:00Z".to_string(),
        "isolate-session",
        "execution",
        "workflow-docs",
        "execution.task.succeeded",
        Some(format!("sha256:{record_id}")),
        json!({ "artifact_records": [{ "artifact_type_id": "docs_patch" }] }),
    )
    .with_record_id(record_id)
    .with_graph(vec![subject()], Vec::new())
}

/// Adapter giving the belief domain its replay port over the shared ledger.
struct ReplayPort(EventReplayCapability);

impl EvidenceEventReplaySource for ReplayPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.0.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.0.replay(request)
    }
}

/// Test double standing in for execution at the named curation-to-goal-set
/// port. Implementing both sinks makes it the port via the blanket impl.
#[derive(Default)]
struct RecordingPort {
    goal_commands: Vec<AgentGoalCommand>,
    mutation_commands: Vec<AgentGoalMutationCommand>,
}

impl meld_world_model::agent::AgentGoalCommandSink for RecordingPort {
    fn submit_goal_command(
        &mut self,
        command: &AgentGoalCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        self.goal_commands.push(command.clone());
        Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            command.goal.goal_id.clone(),
            "applied",
        ))
    }
}

impl meld_world_model::agent::AgentGoalMutationSink for RecordingPort {
    fn submit_goal_mutation(
        &mut self,
        command: &AgentGoalMutationCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        self.mutation_commands.push(command.clone());
        Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            command.goal_id.clone(),
            "applied",
        ))
    }
}

fn goal_query_returning(
    summary: ActiveGoalSummary,
) -> impl FnMut(&str) -> Result<ActiveGoalSummary, AgentActiveGoalQueryError> {
    move |_: &str| Ok(summary.clone())
}

/// The booted isolate: one shared temporary root holding the event ledger,
/// belief store, family registry, traversal store, and agent store, with the
/// theory installed and the genesis identities registered.
struct IsolateSession {
    _root: tempfile::TempDir,
    authority: EventAuthority,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    registry: BeliefFamilyRegistryStore,
    agent_store: Arc<AgentStore>,
    install_disposition: TheoryInstallDisposition,
    theory: BeliefFamilyRevision,
    belief_key: BeliefKey,
    subscription_id: String,
}

impl IsolateSession {
    /// Stages 1 through 3: open the shared root, install the theory, and
    /// register the seed agent with its rule binding and subscription.
    fn boot() -> Self {
        // Stage 1: every store and the ledger share one temporary root.
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path().join("isolate")).unwrap();
        let authority =
            EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default()).unwrap();
        let belief_store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal_store = Arc::new(TraversalStore::new(db.clone()).unwrap());
        let mut registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let agent_store = Arc::new(AgentStore::new(db).unwrap());

        // Stage 2: install the belief family theory from JSON config.
        let config = serde_json::from_str(family_config_json()).unwrap();
        let (install_disposition, theory) = registry.install(config, 1).unwrap();
        let belief_key = configured_belief_key(
            &theory,
            &subject(),
            &default_perspective(),
            &BranchScope::main(),
        );

        // Stage 3: genesis identities through the public command surface —
        // seed registration with an installed curation rule binding, the
        // subscription to the configured belief key, and the operational
        // transition.
        AgentRegistration::new(&agent_store)
            .register_seed_agent(SeedAgentRegistration {
                agent_id: AGENT_ID.to_string(),
                perspective_key: default_perspective(),
                subject: subject(),
                branch_scope: BranchScope::main(),
                observation_scope: FAMILY_ID.to_string(),
                directive: "curate docs freshness goals".to_string(),
                seed_provenance: "trusted init".to_string(),
                curation_rule: Some(AgentCurationRuleBinding::for_rule(curation_rule()).unwrap()),
                curation_rule_revision: None,
                created_at_seq: 2,
            })
            .unwrap();
        let subscription = AgentSubscription::new(&agent_store)
            .subscribe(SubscribeAgentCommand {
                agent_id: AGENT_ID.to_string(),
                belief_key: belief_key.clone(),
                created_at_seq: 3,
            })
            .unwrap();
        AgentRegistration::new(&agent_store)
            .mark_operational(AGENT_ID, 4)
            .unwrap();

        Self {
            _root: root,
            authority,
            belief_store,
            traversal_store,
            registry,
            agent_store,
            install_disposition,
            theory,
            belief_key,
            subscription_id: subscription.subscription_id,
        }
    }

    /// Stage 4 and later stimuli: canonical append is the only event inlet.
    fn append_publication(&self, record_id: &str) -> u64 {
        self.authority
            .append_capability()
            .append_durable(publication_envelope(record_id), AppendMode::Idempotent)
            .unwrap()
            .seq
    }

    fn ingestion_actor(&self) -> EvidenceIngestionActor {
        EvidenceIngestionActor::new(
            INGESTION_ACTOR_ID,
            Arc::clone(&self.belief_store),
            Arc::clone(&self.traversal_store),
            Arc::new(self.registry.clone()),
            FAMILY_ID,
            Arc::new(ReplayPort(self.authority.replay_capability())),
            Arc::new(self.authority.consumer_registry_capability()),
            Arc::new(ConfiguredOutcomeMapping::new(mapping_config()).unwrap()),
            MAPPING_ID,
            default_perspective(),
            BranchScope::main(),
        )
    }

    fn assessment_actor(&self) -> BeliefAssessmentActor {
        BeliefAssessmentActor::new(
            ASSESSMENT_ACTOR_ID,
            Arc::clone(&self.belief_store),
            Arc::clone(&self.traversal_store),
            Arc::new(self.registry.clone()),
            vec![FAMILY_ID.to_string()],
            vec![BeliefSubjectBinding {
                subject: subject(),
                anchor_perspective_kind: "frame_type".to_string(),
                anchor_perspective_id: "analysis".to_string(),
            }],
            default_perspective(),
            BranchScope::main(),
        )
    }

    fn curation_actor(&self) -> AgentGoalCurationActor {
        AgentGoalCurationActor::new(
            CURATION_ACTOR_ID,
            AGENT_ID,
            Arc::clone(&self.agent_store),
            Arc::clone(&self.belief_store),
            Arc::clone(&self.traversal_store),
        )
    }

    /// Typed inspection of the durable evidence cursor on the shared ledger.
    fn durable_evidence_cursor(&self) -> u64 {
        self.authority
            .consumer_registry_capability()
            .consumer_cursor(EVIDENCE_CONSUMER_ID)
            .unwrap()
            .map(|state| state.after_seq)
            .unwrap_or(0)
    }

    fn delivered_seq(&self) -> u64 {
        AgentQuery::new(&self.agent_store)
            .subscription(&self.subscription_id)
            .unwrap()
            .unwrap()
            .last_delivered_seq
    }
}

/// The full worked session: boot, install, register, seed facts through the
/// canonical append, then step ingestion, assessment, and curation actor by
/// actor until the composition converges and quiesces.
#[test]
fn goal_and_belief_isolate_boots_steps_and_converges() {
    let session = IsolateSession::boot();
    assert_eq!(
        session.install_disposition,
        TheoryInstallDisposition::Installed
    );
    let agent_query = AgentQuery::new(&session.agent_store);
    let genesis_agent = agent_query.agent(AGENT_ID).unwrap().unwrap();
    assert_eq!(genesis_agent.status, AgentStatus::Operational);
    assert_eq!(
        genesis_agent.installed_curation_rule().unwrap().clone(),
        curation_rule()
    );
    assert_eq!(session.delivered_seq(), 0);

    // Stage 4: seed one epistemic fact through the canonical append only.
    let first_seq = session.append_publication("publication-a");

    let mut ingestion = session.ingestion_actor();
    let mut assessment = session.assessment_actor();
    let curation = session.curation_actor();

    // Step a: evidence ingestion — the publication becomes promoted evidence
    // and the durable evidence cursor advances past it.
    let ingested = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 8 });
    assert!(
        ingested.fatal_errors.is_empty(),
        "{:?}",
        ingested.fatal_errors
    );
    assert!(
        ingested.retryable_errors.is_empty(),
        "{:?}",
        ingested.retryable_errors
    );
    assert_eq!(ingested.events_replayed, 1);
    assert_eq!(ingested.applicable_count, 1);
    assert!(ingested.new_assignment_count > 0);
    assert_eq!(ingested.revisions_committed, 1);
    assert_eq!(ingested.output_after_seq, first_seq);
    assert_eq!(session.durable_evidence_cursor(), first_seq);

    // Step b: belief assessment. The ingestion path already reassessed the
    // dirtied key under the same installed theory, so the assessment step
    // truthfully attempts nothing — the epistemic state is settled — while
    // its durable checkpoint still advances to the injected sequence.
    let assessed = assessment.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 4,
    });
    assert!(
        assessed.fatal_errors.is_empty(),
        "{:?}",
        assessed.fatal_errors
    );
    assert!(
        assessed.retryable_errors.is_empty(),
        "{:?}",
        assessed.retryable_errors
    );
    assert_eq!(assessed.items_attempted, 0);
    assert_eq!(assessed.output_checkpoint, 10);

    // Inspect through the belief query facade: one revision for the exact
    // configured key, below the rule threshold, citing the installed theory.
    let belief_query = BeliefQuery::new(&session.belief_store);
    let (first_revision, first_view) = belief_query
        .current_revision_and_view(&session.belief_key)
        .unwrap()
        .unwrap();
    assert!(first_view.planner_projection.confidence < RULE_THRESHOLD);
    assert_eq!(
        first_revision.theory_revision,
        Some(session.theory.revision_ref())
    );
    assert!(!belief_query
        .evidence_by_revision(&first_revision.revision_id)
        .unwrap()
        .is_empty());
    assert_eq!(
        belief_query
            .revision_history(&session.belief_key)
            .unwrap()
            .len(),
        1
    );

    // Step c: goal curation through the named port sink — one decision, one
    // dedupe key, and one emitted goal command carrying a ground goal.
    let mut port = RecordingPort::default();
    let mut no_goals = goal_query_returning(ActiveGoalSummary::default());
    let curated = curation.bounded_step(
        &AgentStepRequest {
            sequence: 11,
            max_items: 4,
        },
        &mut no_goals,
        &mut port,
    );
    assert!(
        curated.fatal_errors.is_empty(),
        "{:?}",
        curated.fatal_errors
    );
    assert_eq!(curated.items_selected, 1);
    assert_eq!(curated.items_attempted, 1);
    assert_eq!(curated.decisions_persisted, 1);
    assert_eq!(curated.sink_submissions, 1);
    assert_eq!(curated.sink_receipts.len(), 1);
    assert_eq!(port.goal_commands.len(), 1);
    let command = port.goal_commands[0].clone();
    assert!(command.goal.target.grounding_issue().is_none());
    assert!(matches!(command.goal.lifecycle, GoalLifecycle::Proposed));
    // Cross-domain causality: the dedupe key ties back to the configured
    // belief key the theory installed and the agent subscribed to.
    assert_eq!(command.dedupe_key.agent_id, AGENT_ID);
    assert_eq!(
        command.dedupe_key.dimension_id,
        session.belief_key.dimension_id
    );
    assert_eq!(
        command.dedupe_key.subject_key,
        session.belief_key.subject.index_key()
    );

    let first_decision = agent_query
        .decision_by_dedupe_key(&command.dedupe_key)
        .unwrap()
        .unwrap();
    assert_eq!(first_decision.decision, AgentDecisionKind::GoalCommand);
    assert_eq!(
        first_decision.goal_command_id,
        Some(command.command_id.clone())
    );
    assert_eq!(
        first_decision.input_refs.belief_revision_id,
        Some(first_revision.revision_id.clone())
    );
    assert_eq!(first_decision.input_refs.belief_key, session.belief_key);
    assert_eq!(session.delivered_seq(), first_seq);

    // Step d: strengthening evidence through the canonical append, then one
    // ingestion step and one assessment step. The posterior moves and a
    // second revision with a distinct identity links back to the first.
    let second_seq = session.append_publication("publication-b");
    let strengthened = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 8 });
    assert!(
        strengthened.fatal_errors.is_empty() && strengthened.retryable_errors.is_empty(),
        "{:?} {:?}",
        strengthened.fatal_errors,
        strengthened.retryable_errors
    );
    assert_eq!(strengthened.events_replayed, 1);
    assert_eq!(strengthened.revisions_committed, 1);
    assert_eq!(session.durable_evidence_cursor(), second_seq);
    let reassessed = assessment.bounded_step(&BeliefAssessmentRequest {
        sequence: 12,
        max_items: 4,
    });
    assert!(reassessed.fatal_errors.is_empty() && reassessed.retryable_errors.is_empty());
    assert_eq!(reassessed.items_attempted, 0);

    let (second_revision, second_view) = belief_query
        .current_revision_and_view(&session.belief_key)
        .unwrap()
        .unwrap();
    assert_ne!(second_revision.revision_id, first_revision.revision_id);
    assert_eq!(
        second_revision.prior_revision_id,
        Some(first_revision.revision_id.clone())
    );
    // The stale-probability posterior dropped, so planner confidence rose
    // across the rule threshold.
    assert!(second_revision.posterior.probability < first_revision.posterior.probability);
    assert!(second_view.planner_projection.confidence > first_view.planner_projection.confidence);
    assert!(second_view.planner_projection.confidence > RULE_THRESHOLD);
    assert_eq!(
        belief_query
            .revision_history(&session.belief_key)
            .unwrap()
            .len(),
        2
    );

    // Step e: curation over the moved posterior absorbs — the goal command
    // already admitted for this dedupe key is not re-emitted.
    let admitted = ActiveGoalSummary::from_goals(vec![command.goal.clone()]);
    let mut admitted_goals = goal_query_returning(admitted);
    let refire = curation.bounded_step(
        &AgentStepRequest {
            sequence: 13,
            max_items: 4,
        },
        &mut admitted_goals,
        &mut port,
    );
    assert!(refire.fatal_errors.is_empty(), "{:?}", refire.fatal_errors);
    assert_eq!(refire.items_selected, 1);
    assert_eq!(refire.items_attempted, 1);
    assert_eq!(refire.decisions_persisted, 1);
    assert_eq!(refire.sink_submissions, 0);
    assert_eq!(port.goal_commands.len(), 1);
    assert!(port.mutation_commands.is_empty());
    let absorbed_decision = agent_query
        .decision_by_dedupe_key(&command.dedupe_key)
        .unwrap()
        .unwrap();
    assert_eq!(absorbed_decision.decision, AgentDecisionKind::Absorbed);
    assert_eq!(absorbed_decision.goal_command_id, None);
    assert_ne!(absorbed_decision.decision_id, first_decision.decision_id);
    assert_eq!(
        absorbed_decision.input_refs.belief_revision_id,
        Some(second_revision.revision_id.clone())
    );
    assert_eq!(session.delivered_seq(), second_seq);

    // Step f: full quiescence — with no new events every actor attempts
    // zero work.
    let idle_ingest = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 8 });
    assert_eq!(idle_ingest.events_replayed, 0);
    assert_eq!(idle_ingest.revisions_committed, 0);
    assert_eq!(session.durable_evidence_cursor(), second_seq);
    let idle_assess = assessment.bounded_step(&BeliefAssessmentRequest {
        sequence: 14,
        max_items: 4,
    });
    assert_eq!(idle_assess.items_attempted, 0);
    assert_eq!(idle_assess.items_committed, 0);
    let idle_curate = curation.bounded_step(
        &AgentStepRequest {
            sequence: 15,
            max_items: 4,
        },
        &mut admitted_goals,
        &mut port,
    );
    assert_eq!(idle_curate.items_selected, 0);
    assert_eq!(idle_curate.items_attempted, 0);
    assert_eq!(idle_curate.decisions_persisted, 0);
    assert_eq!(port.goal_commands.len(), 1);
}

/// Quiescence-and-wake half of the worked scenario: unchanged durable state
/// absorbs repeated steps with zero attempts, and one new canonical event
/// wakes the composition end to end. Absorption here comes from the belief
/// itself crossing the rule threshold, without help from the goal snapshot.
#[test]
fn isolate_absorbs_unchanged_state_and_wakes_on_new_evidence() {
    let session = IsolateSession::boot();
    let first_seq = session.append_publication("publication-a");

    let mut ingestion = session.ingestion_actor();
    let mut assessment = session.assessment_actor();
    let curation = session.curation_actor();
    let mut port = RecordingPort::default();
    let mut no_goals = goal_query_returning(ActiveGoalSummary::default());

    // Converge once: ingest, assess, curate — one goal command emitted.
    let ingested = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 8 });
    assert_eq!(ingested.revisions_committed, 1);
    assessment.bounded_step(&BeliefAssessmentRequest {
        sequence: 20,
        max_items: 4,
    });
    let curated = curation.bounded_step(
        &AgentStepRequest {
            sequence: 21,
            max_items: 4,
        },
        &mut no_goals,
        &mut port,
    );
    assert!(
        curated.fatal_errors.is_empty(),
        "{:?}",
        curated.fatal_errors
    );
    assert_eq!(port.goal_commands.len(), 1);
    let agent_query = AgentQuery::new(&session.agent_store);
    assert_eq!(agent_query.recent_decisions(AGENT_ID, 8).unwrap().len(), 1);

    // Unchanged durable state: repeated stepping attempts no work anywhere.
    for sequence in 22..24 {
        let idle_ingest = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 8 });
        assert_eq!(idle_ingest.events_replayed, 0);
        let idle_assess = assessment.bounded_step(&BeliefAssessmentRequest {
            sequence,
            max_items: 4,
        });
        assert_eq!(idle_assess.items_attempted, 0);
        let idle_curate = curation.bounded_step(
            &AgentStepRequest {
                sequence,
                max_items: 4,
            },
            &mut no_goals,
            &mut port,
        );
        assert_eq!(idle_curate.items_selected, 0);
        assert_eq!(idle_curate.decisions_persisted, 0);
    }
    assert_eq!(port.goal_commands.len(), 1);
    assert_eq!(session.durable_evidence_cursor(), first_seq);
    assert_eq!(agent_query.recent_decisions(AGENT_ID, 8).unwrap().len(), 1);

    // One new canonical event wakes the composition: ingestion commits a
    // second revision and curation reviews the delivery. The posterior now
    // sits above the rule threshold, so curation absorbs on the belief
    // alone and no duplicate goal command crosses the port.
    let second_seq = session.append_publication("publication-b");
    let woken_ingest = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 8 });
    assert_eq!(woken_ingest.events_replayed, 1);
    assert_eq!(woken_ingest.revisions_committed, 1);
    assert_eq!(session.durable_evidence_cursor(), second_seq);
    let woken_curate = curation.bounded_step(
        &AgentStepRequest {
            sequence: 30,
            max_items: 4,
        },
        &mut no_goals,
        &mut port,
    );
    assert!(
        woken_curate.fatal_errors.is_empty(),
        "{:?}",
        woken_curate.fatal_errors
    );
    assert_eq!(woken_curate.items_selected, 1);
    assert_eq!(woken_curate.decisions_persisted, 1);
    assert_eq!(woken_curate.sink_submissions, 0);
    assert_eq!(port.goal_commands.len(), 1);

    let decisions = agent_query.recent_decisions(AGENT_ID, 8).unwrap();
    assert_eq!(decisions.len(), 2);
    let dedupe_key = port.goal_commands[0].dedupe_key.clone();
    let latest = agent_query
        .decision_by_dedupe_key(&dedupe_key)
        .unwrap()
        .unwrap();
    assert_eq!(latest.decision, AgentDecisionKind::Absorbed);
    assert_eq!(latest.goal_command_id, None);
}
