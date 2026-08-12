//! Docs freshness PDS publication.

use meld_events::DomainObjectRef;
use meld_lang::{
    CapabilityRef, Condition, CostEstimate, Effect, Goal, GoalLifecycle, GoalPriority, GoalSource,
    Literal, Operator, Proposition, Resolution, SlotConstraint, Term, WorldState,
};
use meld_world_model::{
    AgentStrategyRuntimeConfig, ProspectiveEvidenceRoute, StrategyCapability,
    StrategyEvaluationPolicy, StrategyProblem, StrategySearchBounds, StrategySettlementRule,
    StrategyTheorySnapshot,
};

use crate::capability::{CapabilityCatalog, CapabilityExecutorRegistry, CapabilityTypeContract};
use crate::docs::capability::{
    AssessPublishedScopeCapability, DocsCapabilityConfig, DraftPatchSetCapability,
    InspectScopeCapability, PublishPatchSetCapability, ASSESS_PUBLISHED_SCOPE, DRAFT_PATCH_SET,
    EVIDENCE_BUNDLE, FRESHNESS_ASSESSMENT, INSPECT_SCOPE, PATCH_SET, PUBLICATION_RECEIPT,
    PUBLISH_PATCH_SET,
};
use crate::error::ApiError;

pub const ASSESSMENT_OUTCOME_CONTRACT: &str = "docs.freshness_assessed.v1";
pub const ASSESSMENT_EVIDENCE_SCHEMA: &str = "docs_freshness_assessment_v1";

#[derive(Clone)]
pub struct DocsPdsRuntime {
    pub catalog: CapabilityCatalog,
    pub registry: CapabilityExecutorRegistry,
    pub strategy: AgentStrategyRuntimeConfig,
    pub requested_dimensions: Vec<String>,
}

pub fn compose(config: DocsCapabilityConfig) -> Result<DocsPdsRuntime, ApiError> {
    let mut catalog = CapabilityCatalog::new();
    let mut registry = CapabilityExecutorRegistry::new();
    registry.register(&mut catalog, InspectScopeCapability::new(config.clone()))?;
    registry.register(&mut catalog, DraftPatchSetCapability::new(config.clone()))?;
    registry.register(&mut catalog, PublishPatchSetCapability::new(config.clone()))?;
    registry.register(
        &mut catalog,
        AssessPublishedScopeCapability::new(config.clone()),
    )?;
    let capabilities = strategy_capabilities(&catalog);
    let strategy = strategy_runtime(capabilities, &config)?;
    Ok(DocsPdsRuntime {
        catalog,
        registry,
        strategy,
        requested_dimensions: vec!["docs_freshness".to_string()],
    })
}

fn strategy_capabilities(catalog: &CapabilityCatalog) -> Vec<StrategyCapability> {
    vec![
        strategy_capability(
            catalog.get(INSPECT_SCOPE, 1).unwrap(),
            "inspect-docs-scope",
            None,
            EVIDENCE_BUNDLE,
            "docs.scope_inspected.v1",
            Vec::new(),
        ),
        strategy_capability(
            catalog.get(DRAFT_PATCH_SET, 1).unwrap(),
            "draft-docs-patch-set",
            Some(EVIDENCE_BUNDLE),
            PATCH_SET,
            "docs.patch_set_drafted.v1",
            Vec::new(),
        ),
        strategy_capability(
            catalog.get(PUBLISH_PATCH_SET, 1).unwrap(),
            "publish-docs-patch-set",
            Some(PATCH_SET),
            PUBLICATION_RECEIPT,
            "docs.patch_set_published.v1",
            Vec::new(),
        ),
        strategy_capability(
            catalog.get(ASSESS_PUBLISHED_SCOPE, 1).unwrap(),
            "assess-published-docs-scope",
            Some(PUBLICATION_RECEIPT),
            FRESHNESS_ASSESSMENT,
            ASSESSMENT_OUTCOME_CONTRACT,
            vec![Effect::Assert(Proposition::Exists {
                scope: Term::Variable("?subject".to_string()),
                artifact_type: Term::ArtifactType(FRESHNESS_ASSESSMENT.to_string()),
            })],
        ),
    ]
}

fn strategy_capability(
    contract: &CapabilityTypeContract,
    operator_id: &str,
    input: Option<&str>,
    output: &str,
    outcome_contract_id: &str,
    effects: Vec<Effect>,
) -> StrategyCapability {
    StrategyCapability {
        contract_id: contract.content_identity(),
        operator: Operator {
            operator_id: operator_id.to_string(),
            // Publishing the capability in this activated PDS catalog is the
            // availability claim. The validated physical binding supplies its
            // target scope, including for intentionally unanchored belief
            // families, so graph accessibility is not an operator precondition.
            preconditions: Vec::new(),
            effects,
            cost: CostEstimate {
                time_ms: if contract.capability_type_id == DRAFT_PATCH_SET {
                    60_000
                } else {
                    100
                },
                money_microdollars: 0,
                provider_calls: u32::from(contract.capability_type_id == DRAFT_PATCH_SET),
            },
            resolution: Resolution {
                requires_inputs: input
                    .into_iter()
                    .map(|artifact| SlotConstraint {
                        artifact_type: Term::ArtifactType(artifact.to_string()),
                        required: true,
                    })
                    .collect(),
                requires_outputs: vec![SlotConstraint {
                    artifact_type: Term::ArtifactType(output.to_string()),
                    required: true,
                }],
                scope_kind: Some("repository".to_string()),
                tags: vec!["docs".to_string()],
                specific: Some(CapabilityRef {
                    capability_type_id: contract.capability_type_id.clone(),
                    capability_version: contract.capability_version,
                }),
            },
        },
        outcome_contract_id: outcome_contract_id.to_string(),
    }
}

fn strategy_runtime(
    capabilities: Vec<StrategyCapability>,
    config: &DocsCapabilityConfig,
) -> Result<AgentStrategyRuntimeConfig, ApiError> {
    let placeholder_subject =
        DomainObjectRef::new("workspace_fs", "node", config.subject_id.clone())?;
    let placeholder_goal = Goal {
        goal_id: "docs-strategy-template".to_string(),
        agent_id: config.agent_id.clone(),
        target: Proposition::Holds {
            subject: Term::Object(placeholder_subject.clone()),
            dimension: Term::Dimension("docs_freshness".to_string()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
        },
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: None,
        },
        source: GoalSource::Maintenance {
            invariant_description: "documentation freshness".to_string(),
        },
        lifecycle: GoalLifecycle::Proposed,
    };
    Ok(AgentStrategyRuntimeConfig {
        problem: StrategyProblem {
            problem_id: "docs-freshness-strategy-v1".to_string(),
            goal: placeholder_goal,
            world_state: WorldState::new(vec![Proposition::Accessible {
                scope: Term::Object(placeholder_subject),
            }])
            .map_err(|error| ApiError::ConfigError(format!("{error:?}")))?,
            planner_snapshot_id: "runtime-projection".to_string(),
            theory: StrategyTheorySnapshot {
                theory_id: "docs-freshness-settlement-v1".to_string(),
                settlement_rules: vec![StrategySettlementRule {
                    goal_pattern: Proposition::Holds {
                        subject: Term::Variable("?subject".to_string()),
                        dimension: Term::Dimension("docs_freshness".to_string()),
                        condition: Condition::Above(Term::Variable("?threshold".to_string())),
                    },
                    settlement_obligation: Proposition::Exists {
                        scope: Term::Variable("?subject".to_string()),
                        artifact_type: Term::ArtifactType(FRESHNESS_ASSESSMENT.to_string()),
                    },
                    evidence_route: ProspectiveEvidenceRoute {
                        route_id: "docs-freshness-assessment-route-v1".to_string(),
                        dimension_id: "docs_freshness".to_string(),
                        outcome_contract_id: ASSESSMENT_OUTCOME_CONTRACT.to_string(),
                        evidence_schema_id: ASSESSMENT_EVIDENCE_SCHEMA.to_string(),
                    },
                }],
            },
            capabilities,
            methods: Vec::new(),
            evaluation_policy: StrategyEvaluationPolicy {
                policy_id: "minimal-capability-chain-v1".to_string(),
                prefer_fewer_steps: true,
            },
        },
        bounds: StrategySearchBounds {
            max_expansions: 32,
            max_depth: 8,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderExecutionBinding;
    use meld_execution::goals::ExecutionStrategyAuthorization;
    use meld_execution::planning::{
        CompositionLoweringRequest, ExecutionCompositionLowerer, MethodLibrary, PlanningRequest,
        PlanningResult, PlanningRuntime, PlanningWorldStateFrameRef,
    };
    use meld_execution::task::TaskCompiler;
    use meld_world_model::{search, StrategyCandidateOrigin, StrategySearchRequest};

    #[test]
    fn pds_catalog_dynamically_closes_the_four_capability_chain() {
        let root = tempfile::tempdir().unwrap();
        let config = DocsCapabilityConfig {
            target_root: root.path().to_path_buf(),
            subject_id: "subject".to_string(),
            agent_id: "agent".to_string(),
            provider: ProviderExecutionBinding::new(
                "provider",
                crate::provider::ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
        };
        let runtime = compose(config).unwrap();
        let result = search(&StrategySearchRequest {
            problem: runtime.strategy.problem,
            bounds: runtime.strategy.bounds,
        });
        let candidate = result.recommendation.unwrap();
        assert_eq!(candidate.origin, StrategyCandidateOrigin::Direct);
        assert_eq!(candidate.composition.steps.len(), 4);
        assert_eq!(candidate.composition.edges.len(), 3);
    }

    #[test]
    fn pds_strategy_does_not_require_a_graph_anchor_for_its_bound_scope() {
        let root = tempfile::tempdir().unwrap();
        let config = DocsCapabilityConfig {
            target_root: root.path().to_path_buf(),
            subject_id: "subject".to_string(),
            agent_id: "agent".to_string(),
            provider: ProviderExecutionBinding::new(
                "provider",
                crate::provider::ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
        };
        let mut runtime = compose(config).unwrap();
        runtime.strategy.problem.world_state = WorldState::new(vec![Proposition::Holds {
            subject: Term::Object(DomainObjectRef::new("workspace_fs", "node", "subject").unwrap()),
            dimension: Term::Dimension("docs_freshness".to_string()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.5))),
        }])
        .unwrap();

        let result = search(&StrategySearchRequest {
            problem: runtime.strategy.problem,
            bounds: runtime.strategy.bounds,
        });

        assert_eq!(
            result.completion,
            meld_world_model::StrategySearchCompletion::Exhaustive
        );
        assert_eq!(result.recommendation.unwrap().composition.steps.len(), 4);
    }

    #[test]
    fn authorized_dynamic_chain_lowers_without_a_method_or_workflow_route() {
        let root = tempfile::tempdir().unwrap();
        let config = DocsCapabilityConfig {
            target_root: root.path().to_path_buf(),
            subject_id: "subject".to_string(),
            agent_id: "agent".to_string(),
            provider: ProviderExecutionBinding::new(
                "provider",
                crate::provider::ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
        };
        let pds = compose(config).unwrap();
        let problem = pds.strategy.problem.clone();
        let candidate = search(&StrategySearchRequest {
            problem: problem.clone(),
            bounds: pds.strategy.bounds,
        })
        .recommendation
        .unwrap();
        let authorization = ExecutionStrategyAuthorization {
            authorization_id: "authorization".to_string(),
            agent_decision_id: "decision".to_string(),
            candidate_id: candidate.candidate_id.clone(),
            goal_id: candidate.goal_id.clone(),
            planner_snapshot_id: candidate.planner_snapshot_id.clone(),
            composition: candidate.composition.clone(),
            bindings: candidate.bindings.clone(),
            capability_contract_ids: candidate.capability_contract_ids.clone(),
            method_id: None,
        };
        let mut active_goal = problem.goal;
        active_goal.lifecycle = GoalLifecycle::Active;
        let frame = PlanningWorldStateFrameRef {
            frame_id: candidate.planner_snapshot_id,
            projection_version: "world_model.planner.v1".to_string(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            source_refs: Vec::new(),
            warnings: Vec::new(),
        };
        let planner = PlanningRuntime::new(
            MethodLibrary::from_methods(Vec::new(), &pds.catalog),
            pds.catalog.clone(),
        );
        let projection_request = meld_execution::planning::PlanningWorldStateRequest {
            goal_id: active_goal.goal_id.clone(),
            agent_id: active_goal.agent_id.clone(),
            target: active_goal.target.clone(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["docs_freshness".to_string()],
            required_preconditions: Vec::new(),
        };
        let planned = planner
            .plan_authorized_goal(
                PlanningRequest {
                    request_id: "request".to_string(),
                    goal: active_goal,
                    world_state: problem.world_state,
                    world_state_frame: frame,
                    world_state_request: projection_request,
                },
                &authorization,
            )
            .unwrap();
        let PlanningResult::Composed(composition) = planned else {
            panic!("expected authorized composition");
        };
        let plan = ExecutionCompositionLowerer::new(TaskCompiler::new(), pds.catalog)
            .lower(CompositionLoweringRequest {
                request_id: "lower".to_string(),
                network_id: "network".to_string(),
                composition,
                idempotency_key: "once".to_string(),
            })
            .unwrap();
        assert!(plan.diagnostics.is_empty());
        assert_eq!(plan.mutations.mutations.len(), 4);
    }
}
