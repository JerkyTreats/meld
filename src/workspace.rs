//! Workspace domain: command orchestration, status assembly, and watch runtime.

pub mod capability;
mod ci;
mod commands;
mod danger;
pub mod events;
mod facade;
mod format;
pub mod publish;
pub(crate) mod reducer;
pub mod scan;
mod section;
pub mod summary;
pub mod tooling;
mod types;
mod watch;

pub use facade::*;
