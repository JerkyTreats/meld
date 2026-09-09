//! Workspace domain: command orchestration, status assembly, and watch runtime.

mod ci;
mod commands;
mod danger;
pub mod events;
mod facade;
mod format;
pub(crate) mod lifecycle;
pub mod scan;
mod section;
pub mod summary;
pub mod tooling;
mod types;
mod watch;

pub use facade::*;
