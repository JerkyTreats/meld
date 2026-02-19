//! Agent Read/Write Model
//!
//! Defines how agents interact with nodes and context frames. Establishes clear
//! boundaries between read and write operations, ensuring agents can safely
//! operate concurrently while maintaining data integrity.

pub mod identity;
pub mod profile;
pub mod prompt;
pub mod registry;
pub mod storage;

pub use identity::{AgentIdentity, AgentRole, Capability, ValidationResult};
pub use profile::AgentConfig;
pub use prompt::{resolve_prompt_path, PromptCache};
pub use registry::AgentRegistry;
pub use storage::{AgentStorage, StoredAgentConfig, XdgAgentStorage};
