//! Regression fixture for the retired hand-composed docs PDS image.
//!
//! Production composition lowers declarations through installed receipts.
//! These tests retain the original direct search and lowering proof while
//! using the same owner activation contracts as production.

use meld_events::DomainObjectRef;
use meld_lang::{GoalLifecycle, Proposition, Term, WorldState};
use meld_world_model::{AgentStrategyRuntimeConfig, StrategyTheoryPackage};

use crate::capability::{CapabilityCatalog, CapabilityExecutorRegistry, CapabilityTypeContract};
use crate::docs::capability::DocsCapabilityConfig;
use crate::docs::claim_validation::DocsClaimPolicy;
use crate::error::ApiError;

#[derive(Clone)]
pub struct DocsPdsRuntime {
    pub catalog: CapabilityCatalog,
    pub registry: CapabilityExecutorRegistry,
    pub strategy: AgentStrategyRuntimeConfig,
    pub requested_dimensions: Vec<String>,
}

#[cfg(test)]
pub fn compose(config: DocsCapabilityConfig) -> Result<DocsPdsRuntime, ApiError> {
    let subject = DomainObjectRef::new("workspace_fs", "node", config.subject_id.clone())?;
    let package = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/strategy_theory.docs_freshness.json"
    ))
    .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    let claim_policy = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
    ))
    .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    let mut runtime = compose_with_theory(
        config,
        package,
        claim_policy,
        crate::docs::capability::published_contracts(),
    )?;
    runtime.strategy.problem.goal.target = Proposition::Holds {
        subject: Term::Object(subject),
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: meld_lang::Condition::Above(Term::Literal(meld_lang::Literal::Number(0.7))),
    };
    Ok(runtime)
}

/// Compose docs executors against one exact receipt-resolved theory image.
pub fn compose_with_theory(
    config: DocsCapabilityConfig,
    package: StrategyTheoryPackage,
    claim_policy: DocsClaimPolicy,
    exact_contracts: Vec<CapabilityTypeContract>,
) -> Result<DocsPdsRuntime, ApiError> {
    let subject = DomainObjectRef::new("workspace_fs", "node", config.subject_id.clone())?;
    let mut catalog = CapabilityCatalog::new();
    let mut registry = CapabilityExecutorRegistry::new();
    crate::docs::capability::register_exact_contracts(
        config.clone(),
        claim_policy,
        &exact_contracts,
        &mut catalog,
        &mut registry,
    )?;
    if catalog.iter().count() != exact_contracts.len() {
        return Err(ApiError::ConfigError(
            "receipt executable contract set is incomplete for docs runtime".to_string(),
        ));
    }
    for capability in &package.capabilities {
        let specific = capability
            .operator
            .resolution
            .specific
            .as_ref()
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "strategy capability '{}' is not pinned to an executable contract",
                    capability.operator.operator_id
                ))
            })?;
        let contract = catalog
            .get(&specific.capability_type_id, specific.capability_version)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "strategy capability '{}' version '{}' is absent from the receipt catalog",
                    specific.capability_type_id, specific.capability_version
                ))
            })?;
        if contract.content_identity() != capability.contract_id {
            return Err(ApiError::ConfigError(format!(
                "strategy capability contract identity drift for '{}' version '{}'",
                specific.capability_type_id, specific.capability_version
            )));
        }
    }
    let requested_dimensions = package.requested_dimensions.clone();
    let strategy =
        AgentStrategyRuntimeConfig::activate_installed(package, subject, config.agent_id.clone())
            .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    Ok(DocsPdsRuntime {
        catalog,
        registry,
        strategy,
        requested_dimensions,
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
    use meld_lang::{Condition, Literal};
    use meld_world_model::{search, StrategyCandidateOrigin, StrategySearchRequest};

    #[test]
    fn pds_catalog_dynamically_closes_the_five_capability_chain() {
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
        assert_eq!(candidate.composition.steps.len(), 5);
        assert_eq!(candidate.composition.edges.len(), 5);
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
        assert_eq!(result.recommendation.unwrap().composition.steps.len(), 5);
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
            strategy_theory_id: None,
            strategy_theory_content_hash: None,
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
            panic!("expected authorized composition, got {planned:?}");
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
        assert_eq!(plan.mutations.mutations.len(), 5);
    }
}
