//! Tooling & Integration Layer
//!
//! Provides CLI tools, editor hooks, CI integration, and adapters for internal agents.
//! Ensures the context engine can be used from various environments while maintaining
//! determinism and idempotency.

pub mod ci;
pub mod cli;

pub use crate::agent::{AgentAdapter, ContextApiAdapter};
pub use ci::{BatchOperation, BatchReport, CiIntegration};
pub use cli::{Cli, CliContext, Commands};
pub use crate::workspace::{EditorHooks, WatchConfig, WatchDaemon};
