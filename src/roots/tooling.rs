use std::path::Path;

use crate::branches::query::BranchQueryScope;
use crate::cli::BranchesCommands;
use crate::error::{ApiError, StorageError};
use crate::roots::format::{
    format_branch_graph_status_text, format_federated_neighbors_text, format_federated_walk_text,
    format_roots_status_text,
};
use crate::roots::{BranchQueryRuntime, BranchRuntime};
use crate::telemetry::DomainObjectRef;
use crate::world_state::{GraphWalkSpec, TraversalDirection};

pub fn handle_cli_command(command: &BranchesCommands) -> Result<String, ApiError> {
    handle_cli_command_with_workspace(command, None)
}

pub fn handle_cli_command_with_workspace(
    command: &BranchesCommands,
    workspace_root: Option<&Path>,
) -> Result<String, ApiError> {
    match command {
        BranchesCommands::Status { format } => {
            let output = BranchRuntime::new().status()?;
            render_output(format, &output, format_roots_status_text)
        }
        BranchesCommands::Discover { format } => {
            let output = BranchRuntime::new().discover_branches()?;
            render_output(format, &output, format_roots_status_text)
        }
        BranchesCommands::Migrate { format } => {
            let output = BranchRuntime::new().migrate_branches()?;
            render_output(format, &output, format_roots_status_text)
        }
        BranchesCommands::Attach { path, format } => {
            let output = BranchRuntime::new().attach_branch(path)?;
            render_output(format, &output, format_roots_status_text)
        }
        BranchesCommands::GraphStatus {
            scope,
            branch_ids,
            format,
        } => {
            let output = BranchQueryRuntime::new().graph_status(
                parse_scope(scope, branch_ids)?,
                workspace_root,
            )?;
            render_output(format, &output, format_branch_graph_status_text)
        }
        BranchesCommands::GraphNeighbors {
            scope,
            branch_ids,
            domain,
            object_kind,
            object_id,
            direction,
            relation_types,
            current_only,
            format,
        } => {
            let object = object_ref(domain, object_kind, object_id)?;
            let direction = parse_direction(direction)?;
            let relation_types = relation_types_filter(relation_types);
            let output = BranchQueryRuntime::new().neighbors(
                parse_scope(scope, branch_ids)?,
                workspace_root,
                &object,
                direction,
                relation_types,
                *current_only,
            )?;
            render_output(format, &output, format_federated_neighbors_text)
        }
        BranchesCommands::GraphWalk {
            scope,
            branch_ids,
            domain,
            object_kind,
            object_id,
            direction,
            relation_types,
            max_depth,
            current_only,
            include_facts,
            format,
        } => {
            let object = object_ref(domain, object_kind, object_id)?;
            let direction = parse_direction(direction)?;
            let spec = GraphWalkSpec {
                direction,
                relation_types: relation_types_filter(relation_types).map(|items| items.to_vec()),
                max_depth: *max_depth,
                current_only: *current_only,
                include_facts: *include_facts,
            };
            let output = BranchQueryRuntime::new().walk(
                parse_scope(scope, branch_ids)?,
                workspace_root,
                &object,
                &spec,
            )?;
            render_output(format, &output, format_federated_walk_text)
        }
    }
}

fn render_output<T, F>(format: &str, output: &T, render_text: F) -> Result<String, ApiError>
where
    T: serde::Serialize,
    F: FnOnce(&T) -> String,
{
    if format == "json" {
        serde_json::to_string_pretty(output)
            .map_err(|err| ApiError::StorageError(StorageError::InvalidPath(err.to_string())))
    } else {
        Ok(render_text(output))
    }
}

fn parse_scope(scope: &str, branch_ids: &[String]) -> Result<BranchQueryScope, ApiError> {
    match scope {
        "all" => Ok(BranchQueryScope::All),
        "active" => Ok(BranchQueryScope::Active),
        "branch" => {
            if branch_ids.is_empty() {
                Err(ApiError::ConfigError(
                    "Branch scope requires at least one branch id".to_string(),
                ))
            } else {
                Ok(BranchQueryScope::BranchIds(branch_ids.to_vec()))
            }
        }
        other => Err(ApiError::ConfigError(format!(
            "Unsupported branch scope '{}'",
            other
        ))),
    }
}

fn parse_direction(direction: &str) -> Result<TraversalDirection, ApiError> {
    match direction {
        "outgoing" => Ok(TraversalDirection::Outgoing),
        "incoming" => Ok(TraversalDirection::Incoming),
        "both" => Ok(TraversalDirection::Both),
        other => Err(ApiError::ConfigError(format!(
            "Unsupported traversal direction '{}'",
            other
        ))),
    }
}

fn object_ref(
    domain: &str,
    object_kind: &str,
    object_id: &str,
) -> Result<DomainObjectRef, ApiError> {
    DomainObjectRef::new(domain, object_kind, object_id)
        .map_err(|err| ApiError::StorageError(err))
}

fn relation_types_filter(relation_types: &[String]) -> Option<&[String]> {
    if relation_types.is_empty() {
        None
    } else {
        Some(relation_types)
    }
}
