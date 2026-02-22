//! Agent command service: single entry point per agent CLI command variant.
//!
//! Owns all agent workflow logic; CLI parses, calls one method per variant, and formats output.

use crate::agent::identity::{AgentRole, ValidationResult};
use crate::agent::profile::AgentConfig;
use crate::agent::registry::AgentRegistry;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AgentCommandService;

/// Result of agent list command.
#[derive(Debug, Clone)]
pub struct AgentListResult {
    pub agents: Vec<AgentListItem>,
}

#[derive(Debug, Clone)]
pub struct AgentListItem {
    pub agent_id: String,
    pub role: AgentRole,
}

/// Result of agent show command.
#[derive(Debug, Clone)]
pub struct AgentShowResult {
    pub agent_id: String,
    pub role: AgentRole,
    pub prompt_content: Option<String>,
}

/// Result of agent validate (single agent).
#[derive(Debug, Clone)]
pub struct AgentValidateSingleResult {
    pub result: ValidationResult,
}

/// Result of agent validate --all.
#[derive(Debug, Clone)]
pub struct AgentValidateAllResult {
    pub results: Vec<(String, ValidationResult)>,
}

/// Result of agent status command (one entry per agent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusEntryResult {
    pub agent_id: String,
    pub role: String,
    pub valid: bool,
    pub prompt_path_exists: bool,
}

/// Result of agent create command.
#[derive(Debug, Clone)]
pub struct AgentCreateResult {
    pub agent_id: String,
    pub config_path: PathBuf,
}

/// Result of agent edit command.
#[derive(Debug, Clone)]
pub struct AgentEditResult {
    pub agent_id: String,
}

/// Result of agent remove command.
#[derive(Debug, Clone)]
pub struct AgentRemoveResult {
    pub agent_id: String,
    pub config_path: PathBuf,
}

impl AgentCommandService {
    /// Parse role string to AgentRole.
    pub fn parse_role(role_str: &str) -> Result<AgentRole, ApiError> {
        match role_str {
            "Reader" => Ok(AgentRole::Reader),
            "Writer" => Ok(AgentRole::Writer),
            _ => Err(ApiError::ConfigError(format!(
                "Invalid role: {}. Must be Reader or Writer",
                role_str
            ))),
        }
    }

    /// List agents, optionally filtered by role.
    pub fn list(
        registry: &AgentRegistry,
        role_filter: Option<&str>,
    ) -> Result<AgentListResult, ApiError> {
        let role = role_filter
            .map(Self::parse_role)
            .transpose()?;
        let agents = registry.list_by_role(role);
        let items = agents
            .iter()
            .map(|a| AgentListItem {
                agent_id: a.agent_id.clone(),
                role: a.role,
            })
            .collect();
        Ok(AgentListResult { agents: items })
    }

    /// Show one agent; include_prompt controls whether prompt content is loaded.
    pub fn show(
        registry: &AgentRegistry,
        agent_id: &str,
        include_prompt: bool,
    ) -> Result<AgentShowResult, ApiError> {
        let agent = registry.get_or_error(agent_id)?;
        let prompt_content = if include_prompt {
            agent.metadata.get("system_prompt").cloned()
        } else {
            None
        };
        Ok(AgentShowResult {
            agent_id: agent.agent_id.clone(),
            role: agent.role,
            prompt_content,
        })
    }

    /// Validate a single agent.
    pub fn validate_single(
        registry: &AgentRegistry,
        agent_id: &str,
    ) -> Result<AgentValidateSingleResult, ApiError> {
        let result = registry.validate_agent(agent_id)?;
        Ok(AgentValidateSingleResult { result })
    }

    /// Validate all agents.
    pub fn validate_all(
        registry: &AgentRegistry,
    ) -> Result<AgentValidateAllResult, ApiError> {
        let agents = registry.list_all();
        let mut results = Vec::new();
        for agent in agents {
            let validation = registry
                .validate_agent(&agent.agent_id)
                .unwrap_or_else(|e| {
                    let mut r = ValidationResult::new(agent.agent_id.clone());
                    r.add_error(format!("Failed to validate: {}", e));
                    r
                });
            results.push((agent.agent_id.clone(), validation));
        }
        Ok(AgentValidateAllResult { results })
    }

    /// Status: list all agents with validation and prompt file status.
    pub fn status(registry: &AgentRegistry) -> Result<Vec<AgentStatusEntryResult>, ApiError> {
        let agents = registry.list_all();
        let mut entries = Vec::new();
        for agent in agents {
            let result = match registry.validate_agent(&agent.agent_id) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let role_str = match agent.role {
                AgentRole::Reader => "Reader",
                AgentRole::Writer => "Writer",
            };
            let prompt_path_exists = result
                .checks
                .iter()
                .any(|(desc, passed)| desc == "Prompt file exists" && *passed);
            entries.push(AgentStatusEntryResult {
                agent_id: agent.agent_id.clone(),
                role: role_str.to_string(),
                valid: result.is_valid(),
                prompt_path_exists,
            });
        }
        Ok(entries)
    }

    /// Create agent (non-interactive). Caller must supply role and prompt_path for Writer.
    pub fn create(
        registry: &mut AgentRegistry,
        agent_id: &str,
        role: AgentRole,
        prompt_path: Option<String>,
    ) -> Result<AgentCreateResult, ApiError> {
        if role != AgentRole::Reader && prompt_path.is_none() {
            return Err(ApiError::ConfigError(
                "Prompt path is required for Writer agents.".to_string(),
            ));
        }
        let mut agent_config = AgentConfig {
            agent_id: agent_id.to_string(),
            role,
            system_prompt: None,
            system_prompt_path: prompt_path.clone(),
            metadata: HashMap::new(),
        };
        if role != AgentRole::Reader {
            if let Some(ref path) = prompt_path {
                agent_config.metadata.insert(
                    "user_prompt_file".to_string(),
                    format!("Analyze the file at {{path}} using the system prompt from {}", path),
                );
                agent_config.metadata.insert(
                    "user_prompt_directory".to_string(),
                    format!("Analyze the directory at {{path}} using the system prompt from {}", path),
                );
            }
        }
        registry.save_agent_config(agent_id, &agent_config)?;
        registry.load_from_xdg()?;
        let config_path = registry.agent_config_path(agent_id)?;
        Ok(AgentCreateResult {
            agent_id: agent_id.to_string(),
            config_path,
        })
    }

    /// Update agent by flags (prompt_path and/or role). Does not open editor.
    pub fn update_flags(
        registry: &mut AgentRegistry,
        agent_id: &str,
        prompt_path: Option<&str>,
        role: Option<&str>,
    ) -> Result<AgentEditResult, ApiError> {
        let config_path = registry.agent_config_path(agent_id)?;
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ApiError::ConfigError(format!("Failed to read config: {}", e)))?;
        let mut agent_config: AgentConfig = toml::from_str(&content)
            .map_err(|e| ApiError::ConfigError(format!("Failed to parse config: {}", e)))?;
        if let Some(p) = prompt_path {
            agent_config.system_prompt_path = Some(p.to_string());
        }
        if let Some(r) = role {
            agent_config.role = Self::parse_role(r)?;
        }
        registry.save_agent_config(agent_id, &agent_config)?;
        registry.load_from_xdg()?;
        Ok(AgentEditResult {
            agent_id: agent_id.to_string(),
        })
    }

    /// Persist edited config (after CLI invokes editor and parses result). Validates agent_id matches.
    pub fn persist_edited_config(
        registry: &mut AgentRegistry,
        agent_id: &str,
        agent_config: AgentConfig,
    ) -> Result<AgentEditResult, ApiError> {
        if agent_config.agent_id != agent_id {
            return Err(ApiError::ConfigError(format!(
                "Agent ID mismatch: config has '{}' but expected '{}'",
                agent_config.agent_id, agent_id
            )));
        }
        registry.save_agent_config(agent_id, &agent_config)?;
        registry.load_from_xdg()?;
        Ok(AgentEditResult {
            agent_id: agent_id.to_string(),
        })
    }

    /// Remove agent (delete config and reload registry).
    pub fn remove(
        registry: &mut AgentRegistry,
        agent_id: &str,
    ) -> Result<AgentRemoveResult, ApiError> {
        registry.get_or_error(agent_id)?;
        let config_path = registry.agent_config_path(agent_id)?;
        registry.delete_agent_config(agent_id)?;
        registry.load_from_xdg()?;
        Ok(AgentRemoveResult {
            agent_id: agent_id.to_string(),
            config_path,
        })
    }
}
