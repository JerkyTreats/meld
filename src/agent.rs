//! Agent Read/Write Model
//!
//! Defines how agents interact with nodes and context frames. Establishes clear
//! boundaries between read and write operations, ensuring agents can safely
//! operate concurrently while maintaining data integrity.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use toml;

/// Agent role defining what operations an agent can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// Reader agents can only query context via GetNode API
    Reader,
    /// Writer agents can create frames via PutFrame API and also read context
    Writer,
    /// Synthesis agents are special writer agents that generate branch/directory frames
    Synthesis,
}

/// Agent capability (for future extensibility)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// Can read context frames
    Read,
    /// Can write context frames
    Write,
    /// Can synthesize branch frames
    Synthesize,
}

/// Agent identity with role and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Unique identifier for the agent
    pub agent_id: String,
    /// Role of the agent (Reader, Writer, or Synthesis)
    pub role: AgentRole,
    /// Additional capabilities (for future extensibility)
    pub capabilities: Vec<Capability>,
    /// Metadata for agent (e.g., system prompts, custom settings)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl AgentIdentity {
    /// Create a new agent identity with the given role
    pub fn new(agent_id: String, role: AgentRole) -> Self {
        let capabilities = match role {
            AgentRole::Reader => vec![Capability::Read],
            AgentRole::Writer => vec![Capability::Read, Capability::Write],
            AgentRole::Synthesis => vec![Capability::Read, Capability::Write, Capability::Synthesize],
        };

        Self {
            agent_id,
            role,
            capabilities,
            metadata: HashMap::new(),
        }
    }

    /// Check if the agent has read capability
    pub fn can_read(&self) -> bool {
        self.capabilities.contains(&Capability::Read)
    }

    /// Check if the agent has write capability
    pub fn can_write(&self) -> bool {
        self.capabilities.contains(&Capability::Write)
    }

    /// Check if the agent has synthesize capability
    pub fn can_synthesize(&self) -> bool {
        self.capabilities.contains(&Capability::Synthesize)
    }

    /// Verify that the agent can perform read operations
    pub fn verify_read(&self) -> Result<(), ApiError> {
        if !self.can_read() {
            return Err(ApiError::Unauthorized(format!(
                "Agent {} (role: {:?}) cannot read",
                self.agent_id, self.role
            )));
        }
        Ok(())
    }

    /// Verify that the agent can perform write operations
    pub fn verify_write(&self) -> Result<(), ApiError> {
        if !self.can_write() {
            return Err(ApiError::Unauthorized(format!(
                "Agent {} (role: {:?}) cannot write",
                self.agent_id, self.role
            )));
        }
        Ok(())
    }

    /// Verify that the agent can perform synthesis operations
    pub fn verify_synthesize(&self) -> Result<(), ApiError> {
        if !self.can_synthesize() {
            return Err(ApiError::Unauthorized(format!(
                "Agent {} (role: {:?}) cannot synthesize",
                self.agent_id, self.role
            )));
        }
        Ok(())
    }
}

/// Agent registry for managing agent identities
///
/// In a production system, this would be backed by persistent storage.
/// For Phase 2A, we use an in-memory registry.
pub struct AgentRegistry {
    agents: HashMap<String, AgentIdentity>,
}

impl AgentRegistry {
    /// Create a new empty agent registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Register a new agent
    pub fn register(&mut self, identity: AgentIdentity) {
        self.agents.insert(identity.agent_id.clone(), identity);
    }

    /// Get an agent identity by ID
    pub fn get(&self, agent_id: &str) -> Option<&AgentIdentity> {
        self.agents.get(agent_id)
    }

    /// Get an agent identity by ID or return an error
    pub fn get_or_error(&self, agent_id: &str) -> Result<&AgentIdentity, ApiError> {
        self.get(agent_id).ok_or_else(|| {
            ApiError::Unauthorized(format!("Agent not found: {}", agent_id))
        })
    }

    /// Get all registered agents
    pub fn list_all(&self) -> Vec<&AgentIdentity> {
        self.agents.values().collect()
    }

    /// Load agents from configuration
    pub fn load_from_config(&mut self, config: &crate::config::MerkleConfig) -> Result<(), ApiError> {
        for (_, agent_config) in &config.agents {
            let mut identity = AgentIdentity::new(
                agent_config.agent_id.clone(),
                agent_config.role,
            );

            // Store system prompt in metadata if provided
            if let Some(system_prompt) = &agent_config.system_prompt {
                identity.metadata.insert("system_prompt".to_string(), system_prompt.clone());
            }

            // Copy metadata from config
            for (key, value) in &agent_config.metadata {
                identity.metadata.insert(key.clone(), value.clone());
            }

            self.register(identity);
        }
        Ok(())
    }

    /// Load agents from XDG directory
    ///
    /// Scans `$XDG_CONFIG_HOME/merkle/agents/*.toml` and loads each agent configuration.
    /// Resolves and loads prompt files if `system_prompt_path` is specified.
    /// Invalid configs are logged but don't stop loading of other agents.
    pub fn load_from_xdg(&mut self) -> Result<(), ApiError> {
        let agents_dir = crate::config::xdg::agents_dir()?;
        let mut prompt_cache = crate::config::PromptCache::new();
        let base_dir = crate::config::xdg::config_home()?
            .join("merkle");
        
        if !agents_dir.exists() {
            // Directory doesn't exist yet - that's okay
            return Ok(());
        }
        
        let entries = match std::fs::read_dir(&agents_dir) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ApiError::ConfigError(format!(
                    "Failed to read agents directory {}: {}", 
                    agents_dir.display(), e
                )));
            }
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(
                        "Failed to read directory entry in {}: {}", 
                        agents_dir.display(), e
                    );
                    continue;
                }
            };
            
            let path = entry.path();
            
            // Only process .toml files
            if path.extension() != Some(OsStr::new("toml")) {
                continue;
            }
            
            let agent_id = match path.file_stem()
                .and_then(|s| s.to_str()) {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        "Invalid agent filename (non-UTF8): {:?}", 
                        path
                    );
                    continue;
                }
            };
            
            // Load and parse TOML
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(
                        "Failed to read agent config {}: {}", 
                        path.display(), e
                    );
                    continue;
                }
            };
            
            let agent_config: crate::config::AgentConfig = match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::error!(
                        "Failed to parse agent config {}: {}", 
                        path.display(), e
                    );
                    continue;
                }
            };
            
            // Validate agent_id matches filename
            if agent_config.agent_id != agent_id {
                tracing::warn!(
                    "Agent ID mismatch in {}: filename={}, config={}",
                    path.display(), agent_id, agent_config.agent_id
                );
            }
            
            // Load system prompt
            let system_prompt = if let Some(ref prompt_path) = agent_config.system_prompt_path {
                // Load from file
                match crate::config::resolve_prompt_path(prompt_path, &base_dir) {
                    Ok(resolved_path) => {
                        match prompt_cache.load_prompt(&resolved_path) {
                            Ok(prompt) => prompt,
                            Err(e) => {
                                tracing::error!(
                                    "Failed to load prompt file for agent {} ({}): {}", 
                                    agent_id, prompt_path, e
                                );
                                // Skip this agent if prompt file can't be loaded
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to resolve prompt path for agent {} ({}): {}", 
                            agent_id, prompt_path, e
                        );
                        continue;
                    }
                }
            } else if let Some(ref prompt) = agent_config.system_prompt {
                // Use inline prompt (backward compatibility)
                prompt.clone()
            } else {
                // No prompt - only valid for Reader agents
                if agent_config.role != AgentRole::Reader {
                    tracing::error!(
                        "Agent {} missing system prompt (Writer/Synthesis require prompts)",
                        agent_id
                    );
                    continue;
                }
                String::new() // Reader agents don't need prompts
            };
            
            // Create agent identity
            let mut identity = AgentIdentity::new(
                agent_config.agent_id.clone(),
                agent_config.role,
            );
            
            // Store system prompt in metadata
            if !system_prompt.is_empty() {
                identity.metadata.insert("system_prompt".to_string(), system_prompt);
            }
            
            // Copy other metadata
            for (key, value) in &agent_config.metadata {
                identity.metadata.insert(key.clone(), value.clone());
            }
            
            // Insert or override (XDG configs override config.toml)
            self.agents.insert(agent_config.agent_id.clone(), identity);
        }
        
        Ok(())
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_agent() {
        let agent = AgentIdentity::new("reader-1".to_string(), AgentRole::Reader);
        assert!(agent.can_read());
        assert!(!agent.can_write());
        assert!(!agent.can_synthesize());
        assert!(agent.verify_read().is_ok());
        assert!(agent.verify_write().is_err());
        assert!(agent.verify_synthesize().is_err());
    }

    #[test]
    fn test_writer_agent() {
        let agent = AgentIdentity::new("writer-1".to_string(), AgentRole::Writer);
        assert!(agent.can_read());
        assert!(agent.can_write());
        assert!(!agent.can_synthesize());
        assert!(agent.verify_read().is_ok());
        assert!(agent.verify_write().is_ok());
        assert!(agent.verify_synthesize().is_err());
    }

    #[test]
    fn test_synthesis_agent() {
        let agent = AgentIdentity::new("synthesis-1".to_string(), AgentRole::Synthesis);
        assert!(agent.can_read());
        assert!(agent.can_write());
        assert!(agent.can_synthesize());
        assert!(agent.verify_read().is_ok());
        assert!(agent.verify_write().is_ok());
        assert!(agent.verify_synthesize().is_ok());
    }

    #[test]
    fn test_agent_registry() {
        let mut registry = AgentRegistry::new();

        let agent1 = AgentIdentity::new("agent-1".to_string(), AgentRole::Reader);
        let agent2 = AgentIdentity::new("agent-2".to_string(), AgentRole::Writer);

        registry.register(agent1);
        registry.register(agent2);

        assert!(registry.get("agent-1").is_some());
        assert!(registry.get("agent-2").is_some());
        assert!(registry.get("agent-3").is_none());

        assert!(registry.get_or_error("agent-1").is_ok());
        assert!(registry.get_or_error("agent-3").is_err());
    }

    // Note: Provider is no longer part of AgentIdentity
    // Providers are managed separately via ProviderRegistry

    #[test]
    fn test_agent_registry_list_all() {
        let mut registry = AgentRegistry::new();

        let agent1 = AgentIdentity::new("agent-1".to_string(), AgentRole::Reader);
        let agent2 = AgentIdentity::new("agent-2".to_string(), AgentRole::Writer);
        let agent3 = AgentIdentity::new("agent-3".to_string(), AgentRole::Synthesis);

        registry.register(agent1);
        registry.register(agent2);
        registry.register(agent3);

        let all_agents = registry.list_all();
        assert_eq!(all_agents.len(), 3);

        let agent_ids: Vec<String> = all_agents.iter().map(|a| a.agent_id.clone()).collect();
        assert!(agent_ids.contains(&"agent-1".to_string()));
        assert!(agent_ids.contains(&"agent-2".to_string()));
        assert!(agent_ids.contains(&"agent-3".to_string()));
    }
}
