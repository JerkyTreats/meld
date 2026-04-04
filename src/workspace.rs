//! Workspace domain: command orchestration, status assembly, and watch runtime.

pub mod capability;
mod ci;
mod commands;
mod danger;
mod facade;
mod format;
mod section;
pub mod summary;
mod types;
mod watch;

pub use facade::*;
