//! Agent access to the context engine.
//!
//! Contract and implementation for agents to read context, write frames, and
//! generate frames via context facade contracts only.

pub mod contract;
pub mod context_api;

pub use contract::AgentAdapter;
pub use context_api::ContextApiAdapter;
