//! Execution-owned goal set contracts and store implementations.
//!
//! # Example
//!
//! ```rust
//! use meld_events::DomainObjectRef;
//! use meld_execution::goals::{
//!     AddGoalCommand, GoalCommandMetadata, GoalCommandOutcome, GoalSetQuery, GoalSetStore,
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
//!     lifecycle: GoalLifecycle::Active,
//! };
//!
//! let mut store = GoalSetStore::new();
//! let outcome = store.add_goal(AddGoalCommand {
//!     metadata: GoalCommandMetadata {
//!         command_id: "command-a".to_string(),
//!         source_identity: Some("agent-a:node-a".to_string()),
//!         seq: 42,
//!     },
//!     goal: goal.clone(),
//! }).unwrap();
//!
//! assert!(matches!(outcome, GoalCommandOutcome::Applied(_)));
//!
//! let query = GoalSetQuery::new(&store);
//! assert_eq!(query.active_goal("goal-a"), Some(goal));
//! ```

/// Goal command and record contracts.
pub mod contracts;
/// Durable goal set store backed by local persistence.
pub mod persistent_store;
/// Read-only goal set query facade.
pub mod query;
/// In-memory goal set command store.
pub mod store;

pub use contracts::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandMetadata, GoalCommandOutcome,
    ModifyGoalCommand, RemoveGoalCommand, ResumeGoalCommand, SatisfyGoalCommand,
    SuspendGoalCommand,
};
pub use persistent_store::PersistentGoalSetStore;
pub use query::GoalSetQuery;
pub use store::GoalSetStore;
