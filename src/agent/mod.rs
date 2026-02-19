//! Agent Read/Write Model
//!
//! Defines how agents interact with nodes and context frames. Establishes clear
//! boundaries between read and write operations, ensuring agents can safely
//! operate concurrently while maintaining data integrity.

pub mod domain;
mod prompt;
pub mod repository;
mod registry;

pub use domain::AgentConfig;
pub use prompt::{resolve_prompt_path, PromptCache};
pub use registry::{
    AgentIdentity, AgentRegistry, AgentRole, Capability, ValidationResult,
};
pub use repository::{AgentRepository, StoredAgentConfig, XdgAgentRepository};
