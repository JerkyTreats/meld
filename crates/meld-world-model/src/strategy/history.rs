//! Verified readers for immutable Plans written before complete Task products became sole authority.

use super::contracts::*;
use meld_lang::{Bindings, Composition, Proposition};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct LegacyPlan {
    /// Content-derived immutable Plan revision identity.
    pub plan_revision_id: String,
    /// Stable lineage identity for one Agent Goal.
    pub plan_family_id: String,
    /// Problem against which this candidate was constructed.
    pub problem_id: String,
    /// Exact Goal identity.
    pub goal_id: String,
    /// Exact Planner consistency root.
    pub planner_cut_id: String,
    /// Candidate construction origin.
    pub origin: StrategyPlanOrigin,
    /// Ground semantic action graph.
    pub composition: Composition,
    /// Ground bindings used during construction.
    pub bindings: Bindings,
    /// Settlement obligation discharged by the root action.
    pub settlement_obligation: Proposition,
    /// Prospective evidence from executable work, absent when no work is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_route: Option<ProspectiveEvidenceRoute>,
    /// Exact Capability contract identities selected by the candidate.
    pub capability_contract_ids: Vec<String>,
    /// Independently complete executable products.
    pub tasks: Vec<StrategyTask>,
    /// Independently complete bounded epistemic products.
    pub epistemic_operations: Vec<StrategyEpistemicOperation>,
    /// Exact inter-product causal ordering.
    pub dependencies: Vec<StrategyPlanDependency>,
    /// Exact desired conditions and satisfaction meaning.
    pub conditions: Vec<Proposition>,
    /// Frozen construction context identity.
    pub frozen_context_id: String,
    /// Human-inspectable deterministic explanation.
    pub explanation: String,
    /// Named predecessor when this revision reconstructs an earlier Plan.
    pub predecessor_plan_revision_id: Option<String>,
    /// Deterministic minimal evaluation.
    pub evaluation: StrategyPlanEvaluation,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CurrentPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decomposition: Option<StrategyDecomposition>,
    /// Exact installed settlement rule selected during construction; empty for satisfied Plans
    /// and historical records that predate explicit rule identity.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub settlement_rule_id: String,

    /// Content-derived immutable Plan revision identity.
    pub plan_revision_id: String,
    /// Stable lineage identity for one Agent Goal.
    pub plan_family_id: String,
    /// Problem against which this candidate was constructed.
    pub problem_id: String,
    /// Exact Goal identity.
    pub goal_id: String,
    /// Exact Planner consistency root.
    pub planner_cut_id: String,
    /// Candidate construction origin.
    pub origin: StrategyPlanOrigin,
    /// Ground bindings used during construction.
    pub bindings: Bindings,
    /// Settlement obligation discharged by the root action.
    pub settlement_obligation: Proposition,
    /// Prospective evidence from executable work, absent when no work is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_route: Option<ProspectiveEvidenceRoute>,
    /// Independently complete executable products.
    pub tasks: Vec<StrategyTask>,
    /// Independently complete bounded epistemic products.
    pub epistemic_operations: Vec<StrategyEpistemicOperation>,
    /// Exact inter-product causal ordering.
    pub dependencies: Vec<StrategyPlanDependency>,
    /// Exact desired conditions and satisfaction meaning.
    pub conditions: Vec<Proposition>,
    /// Frozen construction context identity.
    pub frozen_context_id: String,
    /// Human-inspectable deterministic explanation.
    pub explanation: String,
    /// Named predecessor when this revision reconstructs an earlier Plan.
    pub predecessor_plan_revision_id: Option<String>,
    /// Deterministic minimal evaluation.
    pub evaluation: StrategyPlanEvaluation,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct HistoricalPlanIdentity {
    legacy: LegacyPlan,
    canonical_body_id: String,
}

impl From<CurrentPlan> for StrategyPlan {
    fn from(wire: CurrentPlan) -> Self {
        Self {
            decomposition: wire.decomposition,
            historical_identity: None,
            settlement_rule_id: wire.settlement_rule_id,
            plan_revision_id: wire.plan_revision_id,
            plan_family_id: wire.plan_family_id,
            problem_id: wire.problem_id,
            goal_id: wire.goal_id,
            planner_cut_id: wire.planner_cut_id,
            origin: wire.origin,
            bindings: wire.bindings,
            settlement_obligation: wire.settlement_obligation,
            evidence_route: wire.evidence_route,
            tasks: wire.tasks,
            epistemic_operations: wire.epistemic_operations,
            dependencies: wire.dependencies,
            conditions: wire.conditions,
            frozen_context_id: wire.frozen_context_id,
            explanation: wire.explanation,
            predecessor_plan_revision_id: wire.predecessor_plan_revision_id,
            evaluation: wire.evaluation,
        }
    }
}

impl From<&StrategyPlan> for CurrentPlan {
    fn from(plan: &StrategyPlan) -> Self {
        Self {
            decomposition: plan.decomposition.clone(),
            settlement_rule_id: plan.settlement_rule_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            plan_family_id: plan.plan_family_id.clone(),
            problem_id: plan.problem_id.clone(),
            goal_id: plan.goal_id.clone(),
            planner_cut_id: plan.planner_cut_id.clone(),
            origin: plan.origin.clone(),
            bindings: plan.bindings.clone(),
            settlement_obligation: plan.settlement_obligation.clone(),
            evidence_route: plan.evidence_route.clone(),
            tasks: plan.tasks.clone(),
            epistemic_operations: plan.epistemic_operations.clone(),
            dependencies: plan.dependencies.clone(),
            conditions: plan.conditions.clone(),
            frozen_context_id: plan.frozen_context_id.clone(),
            explanation: plan.explanation.clone(),
            predecessor_plan_revision_id: plan.predecessor_plan_revision_id.clone(),
            evaluation: plan.evaluation.clone(),
        }
    }
}

impl<'de> serde::Deserialize<'de> for StrategyPlan {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Legacy(LegacyPlan),
            Current(CurrentPlan),
        }
        match Wire::deserialize(deserializer)? {
            Wire::Current(wire) => Ok(wire.into()),
            Wire::Legacy(wire) => {
                let mut identity = wire.clone();
                identity.plan_revision_id.clear();
                let bytes = serde_json::to_vec(&identity).map_err(serde::de::Error::custom)?;
                let expected = format!("strategy-plan-v1::{}", blake3::hash(&bytes).to_hex());
                let steps: Vec<_> = wire
                    .tasks
                    .iter()
                    .flat_map(|task| task.composition.steps.clone())
                    .collect();
                let edges: Vec<_> = wire
                    .tasks
                    .iter()
                    .flat_map(|task| task.composition.edges.clone())
                    .collect();
                let contracts: std::collections::BTreeSet<_> = wire
                    .tasks
                    .iter()
                    .flat_map(|task| task.capability_contract_ids.clone())
                    .collect();
                if wire.plan_revision_id != expected
                    || wire.composition.steps != steps
                    || wire.composition.edges != edges
                    || contracts != wire.capability_contract_ids.iter().cloned().collect()
                {
                    return Err(serde::de::Error::custom(
                        "historical Plan identity or complete Task projection mismatch",
                    ));
                }
                let original = wire.clone();
                let mut plan = StrategyPlan {
                    decomposition: None,
                    historical_identity: None,
                    settlement_rule_id: String::new(),
                    plan_revision_id: wire.plan_revision_id,
                    plan_family_id: wire.plan_family_id,
                    problem_id: wire.problem_id,
                    goal_id: wire.goal_id,
                    planner_cut_id: wire.planner_cut_id,
                    origin: wire.origin,
                    bindings: wire.bindings,
                    settlement_obligation: wire.settlement_obligation,
                    evidence_route: wire.evidence_route,
                    tasks: wire.tasks,
                    epistemic_operations: wire.epistemic_operations,
                    dependencies: wire.dependencies,
                    conditions: wire.conditions,
                    frozen_context_id: wire.frozen_context_id,
                    explanation: wire.explanation,
                    predecessor_plan_revision_id: wire.predecessor_plan_revision_id,
                    evaluation: wire.evaluation,
                };
                let canonical_body_id = super::search::plan_revision_identity(&plan);
                plan.historical_identity = Some(Box::new(HistoricalPlanIdentity {
                    legacy: original,
                    canonical_body_id,
                }));
                Ok(plan)
            }
        }
    }
}

impl serde::Serialize for StrategyPlan {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(history) = &self.historical_identity {
            if self.plan_revision_id == history.legacy.plan_revision_id
                && canonical_body_identity(self) == history.canonical_body_id
            {
                return history.legacy.serialize(serializer);
            }
        }
        CurrentPlan::from(self).serialize(serializer)
    }
}

fn canonical_body_identity(plan: &StrategyPlan) -> String {
    let mut body = CurrentPlan::from(plan);
    body.plan_revision_id.clear();
    let bytes = serde_json::to_vec(&body).expect("Plan is serializable");
    format!("strategy-plan-v2::{}", blake3::hash(&bytes).to_hex())
}

pub(super) fn predecessor_identity_valid(plan: &StrategyPlan) -> bool {
    plan.plan_revision_id == super::search::plan_revision_identity(plan)
        || plan.historical_identity.as_ref().is_some_and(|history| {
            plan.plan_revision_id == history.legacy.plan_revision_id
                && canonical_body_identity(plan) == history.canonical_body_id
        })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn old_wire(plan: &StrategyPlan) -> serde_json::Value {
        let mut value = serde_json::to_value(plan).unwrap();
        value.as_object_mut().unwrap().remove("settlement_rule_id");
        value["composition"] = serde_json::json!({
            "steps": plan.tasks.iter().flat_map(|task| task.composition.steps.clone()).collect::<Vec<_>>(),
            "edges": plan.tasks.iter().flat_map(|task| task.composition.edges.clone()).collect::<Vec<_>>(),
        });
        value["capability_contract_ids"] = serde_json::json!(plan
            .tasks
            .iter()
            .flat_map(|task| task.capability_contract_ids.clone())
            .collect::<std::collections::BTreeSet<_>>());
        let mut legacy: LegacyPlan = serde_json::from_value(value).unwrap();
        legacy.plan_revision_id.clear();
        legacy.plan_revision_id = format!(
            "strategy-plan-v1::{}",
            blake3::hash(&serde_json::to_vec(&legacy).unwrap()).to_hex()
        );
        serde_json::to_value(legacy).unwrap()
    }

    #[test]
    fn exact_historical_plan_reopens_and_cannot_lend_its_identity_to_mutated_tasks() {
        let problem = super::super::tests::problem();
        let plan = super::super::search(&StrategySearchRequest {
            problem: problem.clone(),
            bounds: StrategySearchBounds {
                max_expansions: 16,
                max_depth: 8,
            },
        })
        .recommendation
        .unwrap();
        let wire = old_wire(&plan);
        let old: StrategyPlan = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(old.tasks, plan.tasks);
        assert_eq!(serde_json::to_value(&old).unwrap(), wire);
        assert!(predecessor_identity_valid(&old));
        let reopened: StrategyPlan =
            serde_json::from_slice(&serde_json::to_vec(&old).unwrap()).unwrap();
        assert_eq!(reopened, old);
        assert!(super::super::search::selected_rule(&problem, &old).is_some());
        let mut changed = old.clone();
        changed.tasks[0].idempotency_key = "different-effect".into();
        assert!(!predecessor_identity_valid(&changed));
        let mut tampered = wire;
        tampered["composition"]["steps"] = serde_json::json!([]);
        assert!(serde_json::from_value::<StrategyPlan>(tampered).is_err());
    }
}
