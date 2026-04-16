//! CLI help and command-name contract for telemetry and routing.

use crate::cli::parse::{
    AgentCommands, AgentPromptCommands, BranchesCommands, Commands, ContextCommands,
    DangerCommands, ProviderCommands, WorkflowCommands, WorkspaceCommands,
};
use crate::telemetry::summary::TypedSummaryEvent;

/// Command name string for session and telemetry, for example `workspace.status`.
pub fn command_name(command: &Commands) -> String {
    match command {
        Commands::Scan { .. } => "scan".to_string(),
        Commands::Workspace { command } => format!("workspace.{}", workspace_command_name(command)),
        Commands::Status { .. } => "status".to_string(),
        Commands::Validate => "validate".to_string(),
        Commands::Watch { .. } => "watch".to_string(),
        Commands::Agent { command } => format!("agent.{}", agent_command_name(command)),
        Commands::Provider { command } => format!("provider.{}", provider_command_name(command)),
        Commands::Init { .. } => "init".to_string(),
        Commands::Context { command } => format!("context.{}", context_command_name(command)),
        Commands::Workflow { command } => format!("workflow.{}", workflow_command_name(command)),
        Commands::Branches { command } => format!("branches.{}", branches_command_name(command)),
        Commands::Danger { command } => format!("danger.{}", danger_command_name(command)),
    }
}

pub fn branches_command_name(command: &BranchesCommands) -> &'static str {
    match command {
        BranchesCommands::Status { .. } => "status",
    }
}

#[allow(dead_code)]
pub fn roots_command_name(command: &BranchesCommands) -> &'static str {
    branches_command_name(command)
}

pub fn danger_command_name(command: &DangerCommands) -> &'static str {
    match command {
        DangerCommands::Flush { .. } => "flush",
    }
}

pub fn workspace_command_name(command: &WorkspaceCommands) -> &'static str {
    match command {
        WorkspaceCommands::Status { .. } => "status",
        WorkspaceCommands::Validate { .. } => "validate",
        WorkspaceCommands::Ignore { .. } => "ignore",
        WorkspaceCommands::Delete { .. } => "delete",
        WorkspaceCommands::Restore { .. } => "restore",
        WorkspaceCommands::Compact { .. } => "compact",
        WorkspaceCommands::ListDeleted { .. } => "list_deleted",
    }
}

pub fn context_command_name(command: &ContextCommands) -> &'static str {
    match command {
        ContextCommands::Generate { .. } => "generate",
        ContextCommands::Regenerate { .. } => "regenerate",
        ContextCommands::Get { .. } => "get",
    }
}

pub fn provider_command_name(command: &ProviderCommands) -> &'static str {
    match command {
        ProviderCommands::Status { .. } => "status",
        ProviderCommands::List { .. } => "list",
        ProviderCommands::Show { .. } => "show",
        ProviderCommands::Create { .. } => "create",
        ProviderCommands::Edit { .. } => "edit",
        ProviderCommands::Remove { .. } => "remove",
        ProviderCommands::Validate { .. } => "validate",
        ProviderCommands::Test { .. } => "test",
    }
}

pub fn workflow_command_name(command: &WorkflowCommands) -> &'static str {
    match command {
        WorkflowCommands::List { .. } => "list",
        WorkflowCommands::Validate { .. } => "validate",
        WorkflowCommands::Inspect { .. } => "inspect",
        WorkflowCommands::Execute { .. } => "execute",
    }
}

pub fn agent_command_name(command: &AgentCommands) -> &'static str {
    match command {
        AgentCommands::Status { .. } => "status",
        AgentCommands::List { .. } => "list",
        AgentCommands::Show { .. } => "show",
        AgentCommands::Create { .. } => "create",
        AgentCommands::Edit { .. } => "edit",
        AgentCommands::Prompt { command } => match command {
            AgentPromptCommands::Show { .. } => "prompt_show",
            AgentPromptCommands::Edit { .. } => "prompt_edit",
        },
        AgentCommands::Remove { .. } => "remove",
        AgentCommands::Validate { .. } => "validate",
    }
}

/// Typed summary contract for telemetry emission. CLI adapts commands to domain producers.
pub fn typed_summary_event(
    command: &Commands,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> Option<TypedSummaryEvent> {
    match command {
        Commands::Workspace { command } => Some(match command {
            WorkspaceCommands::Status { format, breakdown } => {
                crate::workspace::summary::status(format, *breakdown, ok, duration_ms, error)
            }
            WorkspaceCommands::Validate { format } => {
                crate::workspace::summary::validate(format, ok, duration_ms, error)
            }
            WorkspaceCommands::Delete {
                path,
                node,
                dry_run,
                no_ignore,
            } => crate::workspace::summary::delete(
                path.is_some(),
                node.is_some(),
                *dry_run,
                *no_ignore,
                ok,
                duration_ms,
                error,
            ),
            WorkspaceCommands::Restore {
                path,
                node,
                dry_run,
            } => crate::workspace::summary::restore(
                path.is_some(),
                node.is_some(),
                *dry_run,
                ok,
                duration_ms,
                error,
            ),
            WorkspaceCommands::Compact {
                ttl,
                all,
                keep_frames,
                dry_run,
            } => crate::workspace::summary::compact(
                *ttl,
                *all,
                *keep_frames,
                *dry_run,
                ok,
                duration_ms,
                error,
            ),
            WorkspaceCommands::ListDeleted { older_than, format } => {
                crate::workspace::summary::list_deleted(*older_than, format, ok, duration_ms, error)
            }
            WorkspaceCommands::Ignore {
                path,
                dry_run,
                format,
            } => crate::workspace::summary::ignore(
                path.is_some(),
                *dry_run,
                format,
                ok,
                duration_ms,
                error,
            ),
        }),
        Commands::Status {
            format,
            workspace_only,
            agents_only,
            providers_only,
            breakdown,
            test_connectivity,
        } => {
            let include_all = !*workspace_only && !*agents_only && !*providers_only;
            Some(crate::workspace::summary::unified_status(
                format,
                include_all || *workspace_only,
                include_all || *agents_only,
                include_all || *providers_only,
                *breakdown,
                *test_connectivity,
                ok,
                duration_ms,
                error,
            ))
        }
        Commands::Validate => Some(crate::workspace::summary::validate_workspace(
            ok,
            duration_ms,
            error,
        )),
        Commands::Agent { command } => Some(crate::agent::summary::command(
            agent_command_name(command),
            matches!(
                command,
                AgentCommands::Create { .. }
                    | AgentCommands::Edit { .. }
                    | AgentCommands::Prompt {
                        command: AgentPromptCommands::Edit { .. },
                    }
                    | AgentCommands::Remove { .. }
            ),
            ok,
            duration_ms,
            error,
        )),
        Commands::Provider { command } => Some(crate::provider::summary::command(
            provider_command_name(command),
            matches!(
                command,
                ProviderCommands::Create { .. }
                    | ProviderCommands::Edit { .. }
                    | ProviderCommands::Remove { .. }
            ),
            ok,
            duration_ms,
            error,
        )),
        Commands::Context { command } => match command {
            ContextCommands::Generate {
                node,
                path,
                path_positional,
                force,
                no_recursive,
                ..
            } => Some(crate::context::summary::generation(
                context_command_name(command),
                path.is_some() || path_positional.is_some(),
                node.is_some(),
                !*no_recursive,
                *force,
                ok,
                duration_ms,
                error,
            )),
            ContextCommands::Regenerate {
                node,
                path,
                path_positional,
                recursive,
                ..
            } => Some(crate::context::summary::generation(
                context_command_name(command),
                path.is_some() || path_positional.is_some(),
                node.is_some(),
                *recursive,
                true,
                ok,
                duration_ms,
                error,
            )),
            ContextCommands::Get { .. } => None,
        },
        Commands::Init { force, list } => Some(crate::init::summary::command(
            *force,
            *list,
            ok,
            duration_ms,
            error,
        )),
        Commands::Workflow { command } => Some(crate::workflow::summary::command(
            workflow_command_name(command),
            ok,
            duration_ms,
            error,
        )),
        Commands::Branches { .. } => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{branches_command_name, command_name, roots_command_name};
    use crate::cli::parse::{BranchesCommands, Commands};

    #[test]
    fn branch_command_names_are_stable() {
        let command = BranchesCommands::Status {
            format: "text".to_string(),
        };
        assert_eq!(roots_command_name(&command), "status");
        assert_eq!(branches_command_name(&command), "status");
        assert_eq!(
            command_name(&Commands::Branches { command }),
            "branches.status".to_string()
        );
    }
}
