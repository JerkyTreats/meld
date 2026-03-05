//! Agent access to the context engine.
//!
//! Contract and implementation for agents to read context, write frames, and
//! generate frames via context facade contracts only.

pub mod context_api;
pub mod contract;

pub use context_api::ContextApiAdapter;
pub use contract::AgentAdapter;
