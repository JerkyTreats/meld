//! Planner-facing world model projection.
//!
//! This domain converts public graph and belief reads into a ground
//! `meld_lang::WorldState` and durably owns projection requests and frames. It
//! does not own execution planning decisions.

pub mod contracts;
pub mod projection;
pub mod query;
mod runtime;
pub mod store;

pub use contracts::*;
pub use projection::project_world_state;
pub use query::PlannerQuery;
pub use runtime::{
    PlannerProjectionActor, PlannerProjectionIssue, PlannerProjectionTickReport,
    PlannerProjectionTickRequest, MAX_PLANNER_PROJECTION_ITEMS, PLANNER_PROJECTION_ACTOR_ID,
};
pub use store::{PlannerPendingSelection, PlannerProjectionStore};
