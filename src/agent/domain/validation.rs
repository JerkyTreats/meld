//! Agent configuration validation owned by the agent domain.

use super::config::AgentConfig;
use crate::agent::registry::AgentRole;
use std::collections::HashMap;

/// Validate agent configuration.
pub fn validate_agent_config(
    agent: &AgentConfig,
    _providers: &HashMap<String, crate::provider::ProviderConfig>,
) -> Result<(), String> {
    if agent.agent_id.trim().is_empty() {
        return Err("Agent ID cannot be empty".to_string());
    }

    if let Some(ref prompt) = agent.system_prompt {
        if prompt.trim().is_empty() {
            return Err("System prompt cannot be empty if provided".to_string());
        }
    }

    if agent.role != AgentRole::Reader {
        if agent.system_prompt.is_none() && agent.system_prompt_path.is_none() {
            return Err(format!(
                "Agent '{}' (role: {:?}) requires either system_prompt or system_prompt_path",
                agent.agent_id, agent.role
            ));
        }
    }

    if let Some(ref prompt_path) = agent.system_prompt_path {
        if prompt_path.trim().is_empty() {
            return Err("system_prompt_path cannot be empty if provided".to_string());
        }
    }

    Ok(())
}
