use crate::agent::AgentIdentity;
use crate::context::generation::program::TargetExecutionProgram;

pub fn resolve_target_execution_program(agent: &AgentIdentity) -> TargetExecutionProgram {
    match agent.workflow_binding() {
        Some(workflow_id) => TargetExecutionProgram::workflow(workflow_id),
        None => TargetExecutionProgram::single_shot(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentIdentity, AgentRole};

    #[test]
    fn selection_returns_single_shot_without_workflow_binding() {
        let agent = AgentIdentity::new("writer".to_string(), AgentRole::Writer);
        let program = resolve_target_execution_program(&agent);
        assert_eq!(program, TargetExecutionProgram::single_shot());
    }

    #[test]
    fn selection_returns_workflow_with_binding() {
        let mut agent = AgentIdentity::new("writer".to_string(), AgentRole::Writer);
        agent.workflow_id = Some("docs_writer_thread_v1".to_string());
        let program = resolve_target_execution_program(&agent);
        assert_eq!(
            program,
            TargetExecutionProgram::workflow("docs_writer_thread_v1")
        );
    }
}
