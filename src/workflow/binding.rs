//! Agent to workflow binding contracts.

use crate::agent::identity::{AgentIdentity, AgentRole};
use crate::error::ApiError;
use crate::workflow::registry::WorkflowRegistry;

pub fn resolve_bound_workflow_id(agent: &AgentIdentity) -> Option<&str> {
    agent.workflow_binding()
}

pub fn validate_agent_binding(
    agent: &AgentIdentity,
    registry: &WorkflowRegistry,
) -> Result<(), ApiError> {
    let Some(workflow_id) = resolve_bound_workflow_id(agent) else {
        return Ok(());
    };

    if agent.role != AgentRole::Writer {
        return Err(ApiError::ConfigError(format!(
            "Agent '{}' bound workflow_id '{}' but role {:?} is not writer",
            agent.agent_id, workflow_id, agent.role
        )));
    }

    if !registry.contains(workflow_id) {
        return Err(ApiError::ConfigError(format!(
            "Agent '{}' references unknown workflow_id '{}'",
            agent.agent_id, workflow_id
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::identity::AgentIdentity;
    use crate::config::WorkflowConfig;
    use tempfile::TempDir;

    #[test]
    fn validate_binding_accepts_unbound_agent() {
        let temp = TempDir::new().unwrap();
        let registry = WorkflowRegistry::load(&WorkflowConfig {
            user_profile_dir: Some(temp.path().join("workflows")),
        })
        .unwrap();
        let agent = AgentIdentity::new("writer".to_string(), AgentRole::Writer);

        validate_agent_binding(&agent, &registry).unwrap();
    }

    #[test]
    fn validate_binding_rejects_unknown_workflow() {
        let temp = TempDir::new().unwrap();
        let registry = WorkflowRegistry::load(&WorkflowConfig {
            user_profile_dir: Some(temp.path().join("workflows")),
        })
        .unwrap();
        let mut agent = AgentIdentity::new("writer".to_string(), AgentRole::Writer);
        agent.workflow_id = Some("missing_workflow".to_string());

        let err = validate_agent_binding(&agent, &registry).unwrap_err();
        assert!(matches!(err, ApiError::ConfigError(_)));
    }

    #[test]
    fn validate_binding_rejects_reader_binding() {
        let temp = TempDir::new().unwrap();
        let registry = WorkflowRegistry::load(&WorkflowConfig {
            user_profile_dir: Some(temp.path().join("workflows")),
        })
        .unwrap();
        let mut agent = AgentIdentity::new("reader".to_string(), AgentRole::Reader);
        agent.workflow_id = Some("docs_writer_thread_v1".to_string());

        let err = validate_agent_binding(&agent, &registry).unwrap_err();
        assert!(matches!(err, ApiError::ConfigError(_)));
    }
}
