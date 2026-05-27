//! Planner-facing world model projection.
//!
//! This domain converts public graph and belief reads into a ground
//! `meld_lang::WorldState`. It is intentionally read-only and does not own
//! execution planning decisions.

pub mod contracts;
pub mod projection;
pub mod query;

pub use contracts::*;
pub use projection::project_world_state;
pub use query::PlannerQuery;
