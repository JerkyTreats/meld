//! The single named port from agent curation into the execution goal set.
//!
//! Owner: agent domain. Curation output crosses into execution-owned goal
//! storage through this port and nowhere else. The Strategy goal draft gate
//! and admission bundle insert at this port without rewiring curation or the
//! goal set, which is why the seam is named rather than left as two
//! anonymous sink bounds at each call site.

use crate::agent::runtime::{AgentGoalCommandSink, AgentGoalMutationSink};

/// Stable identity of the curation-to-goal-set port.
pub const CURATION_GOAL_SET_PORT_ID: &str = "world_model.curation.goal_set";

/// Named curation-to-goal-set boundary.
///
/// Anything that accepts both curated goal commands and goal mutation
/// commands is this port. Callers binding curation to execution must name
/// this trait so the later draft-gate insertion is a wrapper at one seam.
pub trait CurationGoalSetPort: AgentGoalCommandSink + AgentGoalMutationSink {
    /// Port identity for registration and diagnostics.
    fn port_id(&self) -> &'static str {
        CURATION_GOAL_SET_PORT_ID
    }
}

impl<T: AgentGoalCommandSink + AgentGoalMutationSink> CurationGoalSetPort for T {}
