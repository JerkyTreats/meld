//! Agent storage: persist and load agent configs.

pub mod xdg;

use crate::agent::profile::AgentConfig;
use crate::error::ApiError;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct StoredAgentConfig {
    pub agent_id: String,
    pub config: AgentConfig,
    pub path: PathBuf,
    /// System prompt content resolved from system_prompt_path or system_prompt when loading
    pub resolved_system_prompt: Option<String>,
}

pub trait AgentStorage: Send + Sync {
    fn list(&self) -> Result<Vec<StoredAgentConfig>, ApiError>;
    fn path_for(&self, agent_id: &str) -> Result<PathBuf, ApiError>;
    fn save(&self, agent_id: &str, config: &AgentConfig) -> Result<(), ApiError>;
    fn delete(&self, agent_id: &str) -> Result<(), ApiError>;
    /// Agents directory path for init/cli when they need the directory
    fn agents_dir(&self) -> Result<PathBuf, ApiError>;
}

pub use xdg::XdgAgentStorage;
