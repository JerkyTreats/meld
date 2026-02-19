//! Agent profile: config shape and validation.

pub mod config;
pub mod validation;

pub use config::AgentConfig;
pub use validation::validate_agent_config;
