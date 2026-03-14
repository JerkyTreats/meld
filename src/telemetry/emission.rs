//! Emission orchestration and generic summary payload helpers.

pub mod engine;
mod summary_data;

pub use engine::{emit_command_summary, truncate_for_summary};
