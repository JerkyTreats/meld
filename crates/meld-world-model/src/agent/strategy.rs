//! Frozen Strategy inputs owned by one Agent reconciliation participant.

use meld_events::DomainObjectRef;
use meld_lang::{AuthorityPolicyBinding, Goal};
use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::planner::PlannerCut;
use crate::strategy::{
    validate_strategy_theory_package, StrategyProblem, StrategySearchBounds, StrategyTheoryPackage,
};

/// Root-supplied immutable Strategy package and authority context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStrategyRuntimeConfig {
    pub subject: DomainObjectRef,
    pub package: StrategyTheoryPackage,
    pub agent_id: String,
    #[serde(default)]
    pub theory_revision: Option<crate::belief::TheoryRevisionRef>,
    #[serde(default)]
    pub authority_policy: Option<AuthorityPolicyBinding>,
}

impl AgentStrategyRuntimeConfig {
    pub fn activate_installed(
        package: StrategyTheoryPackage,
        subject: DomainObjectRef,
        agent_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        validate_strategy_theory_package(&package)?;
        Ok(Self {
            subject,
            package,
            agent_id: agent_id.into(),
            theory_revision: None,
            authority_policy: None,
        })
    }

    pub fn with_authority_policy(mut self, policy: AuthorityPolicyBinding) -> Self {
        self.authority_policy = Some(policy);
        self
    }

    /// Freeze one pure construction problem beneath the exact Planner cut.
    pub fn problem(
        &self,
        goal: Goal,
        planner_cut: PlannerCut,
        curation_operations: Vec<crate::CurationOperation>,
    ) -> StrategyProblem {
        StrategyProblem {
            unavailable_curation_operation_ids: Vec::new(),
            effect_visibility: None,
            task_inputs: Vec::new(),
            problem_id: format!(
                "{}::{}::{}",
                self.package.snapshot.theory_id, goal.goal_id, planner_cut.cut_id
            ),
            goal,
            planner_cut,
            theory: self.package.snapshot.clone(),
            capabilities: self.package.capabilities.clone(),
            methods: self.package.methods.clone(),
            evaluation_policy: self.package.evaluation_policy.clone(),
            curation_operations,
        }
    }

    pub fn bounds(&self) -> StrategySearchBounds {
        self.package.search_bounds.clone()
    }
}
