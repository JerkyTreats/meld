use std::sync::Arc;

use meld_events::{DomainObjectRef, EventEnvelope, EventRecord, EventRelation};
use meld_execution::capability::{
    ArtifactSchemaVersionRange, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld_execution::planning::{MethodLibrary, PlanningRuntime};
use meld_lang::{
    Composition, Condition, CostEstimate, Effect, GoalLifecycle, Literal, Method, Operator,
    Proposition, Resolution, SlotConstraint, Step, StepKind, Term,
};
use meld_world_model::agent::{
    AgentCurationDedupeKey, AgentCurationRuleConfig, AgentSubscriptionRecord, SeedAgentRegistration,
};
use meld_world_model::belief::{BeliefKey, BranchScope};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{AnchorSelectionRecord, PerspectiveKey, TraversalFactRecord};
use serde_json::json;

pub const AGENT_ID: &str = "seed.docs_freshness";
pub const DIMENSION_ID: &str = "docs_freshness";
pub const PREDICATE_ID: &str = "confidence";
pub const EVIDENCE_POLICY_ID: &str = "default_policy";
pub const THRESHOLD: f64 = 0.7;
pub const PRIORITY_URGENCY: u32 = 50;

pub const SUBJECT_DOMAIN_ID: &str = "workspace_fs";
pub const SUBJECT_OBJECT_KIND: &str = "node";
pub const SUBJECT_OBJECT_ID: &str = "node-a";
pub const PERSPECTIVE_KIND: &str = "default";
pub const PERSPECTIVE_ID: &str = "default";
pub const GRAPH_PERSPECTIVE_KIND: &str = "frame_type";
pub const GRAPH_PERSPECTIVE_ID: &str = "analysis";
pub const TASK_NETWORK_ID: &str = "network-docs";
pub const SESSION_ID: &str = "session-docs";
pub const WORKER_ID: &str = "worker-docs";
pub const METHOD_ID: &str = "refresh_docs_v1";
pub const REQUIRED_ARTIFACT_TYPE_ID: &str = "docs_patch";
pub const CONTENT_SOURCE_KIND: &str = "content_written";
pub const PUBLICATION_EVENT_TYPE: &str = "execution.task.succeeded";
pub const FAILURE_EVENT_TYPE: &str = "execution.task.failed";
pub const PUBLICATION_ID: &str =
    "task-network-publication-9ec007895dc9863796b15c44ce3d11cae6e7c4d19abeb95b797aa53c05747e11";
pub const OUTCOME_ID: &str = "outcome-alpha";
pub const TASK_INSTANCE_ID: &str = "task-alpha";
pub const ARTIFACT_ID: &str = "artifact-alpha";
pub const TASK_ARTIFACT_REPO_ID: &str = "repo-docs";
pub const SEED_GRAPH_SEQ: u64 = 1;
pub const GOAL_ACCEPTED_SEQ: u64 = 7;
pub const PUBLICATION_EVENT_SEQ: u64 = 21;
pub const SATISFACTION_REVIEW_SEQ: u64 = 22;
pub const GOAL_COMMAND_REVISION_ID: &str = "revision-a";
pub const SATISFIED_REVISION_ID: &str = "revision-satisfied";

#[derive(Debug, Clone, Copy, Default)]
pub struct DocsFreshnessFirstProofFixture;

impl DocsFreshnessFirstProofFixture {
    pub fn new() -> Self {
        Self
    }

    pub fn subject(&self) -> DomainObjectRef {
        domain_object(SUBJECT_DOMAIN_ID, SUBJECT_OBJECT_KIND, SUBJECT_OBJECT_ID)
    }

    pub fn subject_term(&self) -> Term {
        Term::Object(self.subject())
    }

    pub fn perspective(&self) -> PerspectiveKey {
        PerspectiveKey::new(PERSPECTIVE_KIND, PERSPECTIVE_ID).unwrap()
    }

    pub fn graph_perspective(&self) -> PerspectiveKey {
        PerspectiveKey::new(GRAPH_PERSPECTIVE_KIND, GRAPH_PERSPECTIVE_ID).unwrap()
    }

    pub fn branch_scope(&self) -> BranchScope {
        BranchScope::main()
    }

    pub fn belief_key(&self) -> BeliefKey {
        BeliefKey {
            subject: self.subject(),
            dimension_id: DIMENSION_ID.to_string(),
            predicate_id: PREDICATE_ID.to_string(),
            perspective: self.perspective(),
            branch_scope: self.branch_scope(),
            evidence_policy_id: EVIDENCE_POLICY_ID.to_string(),
        }
    }

    pub fn seed_agent_registration(&self) -> SeedAgentRegistration {
        SeedAgentRegistration {
            agent_id: AGENT_ID.to_string(),
            perspective_key: self.perspective(),
            subject: self.subject(),
            branch_scope: self.branch_scope(),
            observation_scope: DIMENSION_ID.to_string(),
            directive_id: "directive.docs_freshness".to_string(),
            seed_provenance: "trusted init".to_string(),
            created_at_seq: 0,
        }
    }

    pub fn curation_rule_config(&self) -> AgentCurationRuleConfig {
        AgentCurationRuleConfig {
            dimension_id: DIMENSION_ID.to_string(),
            threshold: THRESHOLD,
            priority_urgency: PRIORITY_URGENCY,
            desired_summary: "confidence>0.7".to_string(),
            source_kind: "belief_divergence".to_string(),
        }
    }

    pub fn goal_acceptance_seq(&self) -> u64 {
        GOAL_ACCEPTED_SEQ
    }

    pub fn publication_event_seq(&self) -> u64 {
        PUBLICATION_EVENT_SEQ
    }

    pub fn satisfaction_review_seq(&self) -> u64 {
        SATISFACTION_REVIEW_SEQ
    }

    pub fn expected_goal_source_identity(&self) -> String {
        self.threshold_dedupe_key().index_key()
    }

    pub fn expected_goal_command_id(&self) -> String {
        deterministic_id(
            "goal-command",
            &format!(
                "{}::{:?}",
                self.expected_goal_source_identity(),
                Some(GOAL_COMMAND_REVISION_ID.to_string())
            ),
        )
    }

    pub fn expected_goal_id(&self) -> String {
        deterministic_id("goal", &self.expected_goal_source_identity())
    }

    pub fn expected_satisfaction_mutation_command_id(&self) -> String {
        let decision_key = format!(
            "{}::satisfy::{}::{}",
            self.expected_goal_source_identity(),
            self.expected_goal_id(),
            self.satisfaction_review_seq()
        );
        deterministic_id("goal-mutation-command", &decision_key)
    }

    pub fn expected_subscription_id(&self) -> String {
        let natural_key = AgentSubscriptionRecord::natural_key(AGENT_ID, &self.belief_key());
        deterministic_id("subscription", &natural_key)
    }

    pub fn belief_config_json(&self) -> &'static str {
        r#"{
            "family_id": "docs_freshness",
            "dimension_id": "docs_freshness",
            "predicate_id": "confidence",
            "evidence_policy_id": "default_policy",
            "evidence_schemas": [
                {
                    "schema_id": "graph_anchor_signal",
                    "required": true,
                    "role": "Support",
                    "reliability": 1.0,
                    "precision": 1.0
                },
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
                    "mapping_id": "anchor_to_signal",
                    "source_kind": "graph_anchor",
                    "evidence_schema_id": "graph_anchor_signal",
                    "subject_from": "anchor.subject",
                    "value_field": "ended",
                    "factor_id": "freshness_signal"
                },
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
                        "factor_id": "freshness_signal",
                        "evidence_schema_id": "graph_anchor_signal",
                        "weight": 1.0,
                        "polarity": "Supports"
                    },
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

    pub fn capability_catalog(&self) -> CapabilityCatalog {
        let mut catalog = CapabilityCatalog::new();
        catalog
            .register(CapabilityTypeContract {
                capability_type_id: "docs.write".to_string(),
                capability_version: 1,
                owning_domain: "docs".to_string(),
                scope_contract: ScopeContract {
                    scope_kind: "filesystem".to_string(),
                    scope_ref_kind: "node_id".to_string(),
                    allow_fan_out: false,
                },
                binding_contract: vec![],
                input_contract: vec![InputSlotSpec {
                    slot_id: "source".to_string(),
                    accepted_artifact_type_ids: vec!["source_doc".to_string()],
                    schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                    required: false,
                    cardinality: InputCardinality::One,
                }],
                output_contract: vec![OutputSlotSpec {
                    slot_id: "patch".to_string(),
                    artifact_type_id: REQUIRED_ARTIFACT_TYPE_ID.to_string(),
                    schema_version: 1,
                    guaranteed: true,
                }],
                effect_contract: vec![],
                execution_contract: ExecutionContract {
                    execution_class: ExecutionClass::Queued,
                    completion_semantics: "result_or_failure".to_string(),
                    retry_class: "provider_io".to_string(),
                    cancellation_supported: true,
                },
            })
            .unwrap();
        catalog
    }

    pub fn docs_method(&self) -> Method {
        Method {
            method_id: METHOD_ID.to_string(),
            trigger: Proposition::Holds {
                subject: Term::Variable("?node".to_string()),
                dimension: Term::Dimension(DIMENSION_ID.to_string()),
                condition: Condition::Above(Term::Literal(Literal::Number(THRESHOLD))),
            },
            preconditions: vec![Proposition::Accessible {
                scope: Term::Variable("?node".to_string()),
            }],
            composition: Composition {
                steps: vec![Step {
                    step_id: "write".to_string(),
                    kind: StepKind::Op(Operator {
                        operator_id: "write".to_string(),
                        preconditions: vec![Proposition::Accessible {
                            scope: Term::Variable("?node".to_string()),
                        }],
                        effects: vec![
                            Effect::Assert(Proposition::Exists {
                                scope: Term::Variable("?node".to_string()),
                                artifact_type: Term::ArtifactType(
                                    REQUIRED_ARTIFACT_TYPE_ID.to_string(),
                                ),
                            }),
                            Effect::Update {
                                subject: Term::Variable("?node".to_string()),
                                dimension: Term::Dimension(DIMENSION_ID.to_string()),
                                value: Term::Literal(Literal::Number(0.95)),
                            },
                        ],
                        cost: self.method_cost(),
                        resolution: Resolution {
                            requires_inputs: vec![],
                            requires_outputs: vec![SlotConstraint {
                                artifact_type: Term::ArtifactType(
                                    REQUIRED_ARTIFACT_TYPE_ID.to_string(),
                                ),
                                required: true,
                            }],
                            scope_kind: Some("filesystem".to_string()),
                            tags: vec!["write".to_string(), "docs".to_string()],
                            specific: None,
                        },
                    }),
                }],
                edges: vec![],
            },
            net_effects: vec![Effect::Update {
                subject: Term::Variable("?node".to_string()),
                dimension: Term::Dimension(DIMENSION_ID.to_string()),
                value: Term::Literal(Literal::Number(0.95)),
            }],
            cost: self.method_cost(),
            preference: 1,
        }
    }

    pub fn planning_runtime(&self) -> PlanningRuntime {
        let catalog = self.capability_catalog();
        let library = MethodLibrary::from_methods(vec![self.docs_method()], &catalog);
        PlanningRuntime::new(library, catalog)
    }

    pub fn empty_planning_runtime(&self) -> PlanningRuntime {
        let catalog = self.capability_catalog();
        let library = MethodLibrary::from_methods(Vec::<Method>::new(), &catalog);
        PlanningRuntime::new(library, catalog)
    }

    pub fn seeded_graph(&self) -> (tempfile::TempDir, Arc<TraversalStore>, DomainObjectRef) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Arc::new(
            TraversalStore::new(sled::open(temp_dir.path().join("graph")).unwrap()).unwrap(),
        );
        self.seed_graph_into(store.as_ref());
        (temp_dir, store, self.subject())
    }

    pub fn seed_graph_into(&self, store: &TraversalStore) {
        let node = self.subject();
        let frame = domain_object("context", "frame", "frame-a");
        let anchor_ref = domain_object("context", "head", "node-a::analysis");
        let relation = EventRelation::new("selected", node.clone(), frame.clone()).unwrap();
        let fact = TraversalFactRecord {
            fact_id: "fact-a".to_string(),
            source_spine_fact_id: "ledger-a".to_string(),
            seq: SEED_GRAPH_SEQ,
            event_type: "context.head.selected".to_string(),
            objects: vec![node.clone(), frame.clone()],
            relations: vec![relation],
        };
        let anchor = AnchorSelectionRecord {
            anchor_id: "anchor-a".to_string(),
            anchor_ref,
            subject: node.clone(),
            perspective: self.graph_perspective(),
            target: frame,
            source_fact_ids: vec!["ledger-a".to_string()],
            created_by_fact_id: "fact-a".to_string(),
            selected_at_seq: SEED_GRAPH_SEQ,
            ended_at_seq: None,
            ended_by_anchor_id: None,
            ended_by_fact_id: None,
        };
        store.put_fact(&fact).unwrap();
        store.put_anchor(&anchor).unwrap();
        store.set_current_anchor(&anchor).unwrap();
    }

    pub fn task_success_event(&self, seq: u64) -> EventRecord {
        self.task_event(PUBLICATION_EVENT_TYPE, seq, PUBLICATION_ID)
    }

    pub fn task_failure_event(&self, seq: u64) -> EventRecord {
        self.task_event(FAILURE_EVENT_TYPE, seq, PUBLICATION_ID)
    }

    pub fn unrelated_task_success_event(&self, seq: u64) -> EventRecord {
        let envelope = EventEnvelope::new_domain(
            "2026-06-09T00:00:00Z".to_string(),
            "session-other",
            "execution",
            "task_network::network-other::task::task-beta",
            PUBLICATION_EVENT_TYPE,
            Some("hash-b".to_string()),
            json!({
                "outcome_id": "outcome-beta",
                "task_instance_id": "task-beta",
                "artifact_records": [
                    {
                        "artifact_id": "artifact-beta",
                        "artifact_type_id": "metrics_report",
                        "schema_version": 1,
                        "content": {
                            "summary": "not docs"
                        }
                    }
                ]
            }),
        )
        .with_record_id(Self::publication_record_id("pub-b"));
        EventRecord::from_envelope(envelope, seq)
    }

    pub fn publication_record_id(publication_id: &str) -> String {
        format!("execution::task_network_publication::{publication_id}")
    }

    pub fn task_stream_id(&self) -> String {
        format!("task_network::{TASK_NETWORK_ID}::task::{TASK_INSTANCE_ID}")
    }

    pub fn expected_final_lifecycle(&self, seq: u64) -> GoalLifecycle {
        GoalLifecycle::Satisfied { at_seq: seq }
    }

    fn task_event(&self, event_type: &str, seq: u64, publication_id: &str) -> EventRecord {
        let envelope = EventEnvelope::new_domain(
            "2026-06-09T00:00:00Z".to_string(),
            SESSION_ID,
            "execution",
            self.task_stream_id(),
            event_type,
            Some("hash-a".to_string()),
            json!({
                "outcome_id": OUTCOME_ID,
                "task_instance_id": TASK_INSTANCE_ID,
                "artifact_records": [
                    {
                        "artifact_id": ARTIFACT_ID,
                        "artifact_type_id": REQUIRED_ARTIFACT_TYPE_ID,
                        "schema_version": 1,
                        "content": {
                            "patch": "updated docs"
                        }
                    }
                ]
            }),
        )
        .with_record_id(Self::publication_record_id(publication_id));
        EventRecord::from_envelope(envelope, seq)
    }

    fn method_cost(&self) -> CostEstimate {
        CostEstimate {
            time_ms: 10_000,
            money_microdollars: 25_000,
            provider_calls: 1,
        }
    }

    fn threshold_dedupe_key(&self) -> AgentCurationDedupeKey {
        AgentCurationDedupeKey::threshold_rule(
            AGENT_ID.to_string(),
            &self.subject(),
            &self.branch_scope(),
            &self.curation_rule_config(),
        )
    }
}

pub fn domain_object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn deterministic_id(prefix: &str, key: &str) -> String {
    format!("{prefix}-{}", stable_hash_hex(key.as_bytes()))
}

fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}
