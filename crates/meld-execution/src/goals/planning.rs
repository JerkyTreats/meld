//! Execution-owned goal selection contract for bounded planning actors.

use crate::goals::ExecutionGoalRecord;

/// Maximum number of active goals one planning selection may claim.
pub const MAX_PLANNING_SELECTION_LIMIT: usize = 1_024;

/// Opaque durable claim over one bounded planning selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalPlanningClaim {
    claim_id: String,
}

impl GoalPlanningClaim {
    pub(crate) fn new(claim_id: String) -> Self {
        Self { claim_id }
    }

    pub(crate) fn claim_id(&self) -> &str {
        &self.claim_id
    }
}

/// One exclusive bounded selection of active execution goals.
#[derive(Debug)]
pub struct GoalPlanningSelection {
    /// Exact goal records claimed for this planning pass.
    pub records: Vec<ExecutionGoalRecord>,
    /// Total active goals represented by the durable planning index.
    pub active_goal_count: usize,
    /// True when more active work exists outside this selection.
    pub budget_exhausted: bool,
    /// Claim that must be released after the pass completes.
    pub claim: Option<GoalPlanningClaim>,
}

/// Read failure at the execution-owned goal planning boundary.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum GoalPlanningSelectionError {
    /// A transient persistence failure may recover on a later pass.
    #[error("goal planning selection storage failed: {0}")]
    TransientStorage(String),
    /// Durable goal selection state failed exact validation.
    #[error("goal planning selection state is corrupt: {0}")]
    CorruptState(String),
    /// Concurrent selectors prevented a stable bounded claim.
    #[error("goal planning selection remained contended")]
    SelectionContended,
}

impl GoalPlanningSelectionError {
    /// Return whether a later pass may recover without durable repair.
    pub fn retryable(&self) -> bool {
        matches!(self, Self::TransientStorage(_) | Self::SelectionContended)
    }
}

/// Public execution contract consumed by the planning runtime.
pub trait GoalPlanningSelectionPort {
    /// Exclusively claim one bounded set of active goals.
    fn claim_active_goals(
        &self,
        limit: usize,
    ) -> Result<GoalPlanningSelection, GoalPlanningSelectionError>;

    /// Release a prior planning claim and make unchanged goals available again.
    fn release_goal_claim(
        &self,
        claim: &GoalPlanningClaim,
    ) -> Result<(), GoalPlanningSelectionError>;

    /// Read one exact goal record for the pre-submission staleness fence.
    fn planning_goal_record(
        &self,
        goal_id: &str,
    ) -> Result<Option<ExecutionGoalRecord>, GoalPlanningSelectionError>;
}
