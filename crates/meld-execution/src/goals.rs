//! Execution-owned goal set contracts, acceptance facade, and stores.
//!
//! Execution owns durable goal lifecycle state. Producers such as world-model
//! agents own the decision to propose a goal and cross into this domain through
//! producer-neutral commands.
//!
//! # Example
//!
//! ```rust
//! use meld_events::DomainObjectRef;
//! use meld_execution::goals::{
//!     GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalCommandMetadata, GoalCommandOutcome,
//!     GoalSetApi, GoalSetQuery, GoalSetStore,
//! };
//! use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};
//!
//! let target = Proposition::Accessible {
//!     scope: Term::Object(DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap()),
//! };
//! let goal = Goal {
//!     goal_id: "goal-a".to_string(),
//!     agent_id: "agent-a".to_string(),
//!     target,
//!     priority: GoalPriority {
//!         urgency: 1,
//!         cost_ceiling: None,
//!     },
//!     source: GoalSource::UserDirected {
//!         directive: "Inspect node-a".to_string(),
//!     },
//!     lifecycle: GoalLifecycle::Proposed,
//! };
//!
//! let mut store = GoalSetStore::new();
//! let outcome = GoalSetApi::new(&mut store).accept_goal(GoalAcceptanceRequest {
//!     metadata: GoalCommandMetadata {
//!         command_id: "command-a".to_string(),
//!         source_identity: Some("agent-a:node-a".to_string()),
//!         seq: 42,
//!     },
//!     goal: goal.clone(),
//!     lifecycle_policy: GoalAcceptanceLifecycle::RequireProposedThenActivate,
//! }).unwrap();
//!
//! assert!(matches!(outcome, GoalCommandOutcome::Applied(_)));
//!
//! let query = GoalSetQuery::new(&store);
//! assert_eq!(query.active_goal("goal-a").unwrap().lifecycle, GoalLifecycle::Active);
//! ```

/// Producer-neutral goal acceptance and command facade.
pub mod api;
/// Goal command and record contracts.
pub mod contracts;
/// Durable goal set store backed by local persistence.
pub mod persistent_store;
/// Read-only goal set query facade.
pub mod query;
/// In-memory goal set command store.
pub mod store;

pub use api::{GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalSetApi, GoalSetApiError};
pub use contracts::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandMetadata, GoalCommandOutcome,
    ModifyGoalCommand, RemoveGoalCommand, ReopenGoalCommand, ResumeGoalCommand, SatisfyGoalCommand,
    StaleGoalCommandReason, SuspendGoalCommand,
};
pub use persistent_store::PersistentGoalSetStore;
pub use query::{ActiveGoalQuery, ActiveGoalQueryError, GoalSetQuery};
pub use store::GoalSetStore;
