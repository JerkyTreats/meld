//! Exact code changes and durable materialization evidence, independent of Security judgment.

pub mod acquisition;
pub mod capability;
pub mod contracts;
mod mutation;
mod operation;

pub use operation::materialization_evidence;
