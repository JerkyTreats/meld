use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetExecutionProgramKind {
    SingleShot,
    Workflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TargetExecutionProgram {
    pub kind: TargetExecutionProgramKind,
    pub workflow_id: Option<String>,
}

impl TargetExecutionProgram {
    /// Old serialized Workflow addresses remain recognizable, but cannot enter a queue.
    pub fn validate_execution(&self) -> Result<(), crate::error::ApiError> {
        if self.kind == TargetExecutionProgramKind::Workflow || self.workflow_id.is_some() {
            return Err(crate::workflow::retired_execution_error());
        }
        Ok(())
    }

    pub fn single_shot() -> Self {
        Self {
            kind: TargetExecutionProgramKind::SingleShot,
            workflow_id: None,
        }
    }

    pub fn workflow(workflow_id: impl Into<String>) -> Self {
        Self {
            kind: TargetExecutionProgramKind::Workflow,
            workflow_id: Some(workflow_id.into()),
        }
    }

    pub fn workflow_id(&self) -> Option<&str> {
        self.workflow_id.as_deref()
    }

    pub fn kind_str(&self) -> &'static str {
        self.kind.as_str()
    }
}

impl TargetExecutionProgramKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SingleShot => "single_shot",
            Self::Workflow => "workflow",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_constructor_sets_kind_and_id() {
        let program = TargetExecutionProgram::workflow("docs_writer_thread_v1");
        assert_eq!(program.kind, TargetExecutionProgramKind::Workflow);
        assert_eq!(program.workflow_id(), Some("docs_writer_thread_v1"));
    }

    #[test]
    fn single_shot_constructor_clears_workflow_id() {
        let program = TargetExecutionProgram::single_shot();
        assert_eq!(program.kind, TargetExecutionProgramKind::SingleShot);
        assert_eq!(program.workflow_id(), None);
    }
}
