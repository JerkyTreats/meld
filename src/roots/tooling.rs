use crate::cli::BranchesCommands;
use crate::error::{ApiError, StorageError};
use crate::roots::{BranchRuntime, format::format_roots_status_text};

pub fn handle_cli_command(command: &BranchesCommands) -> Result<String, ApiError> {
    match command {
        BranchesCommands::Status { format } => {
            let output = BranchRuntime::new().status()?;
            if format == "json" {
                serde_json::to_string_pretty(&output).map_err(|err| {
                    ApiError::StorageError(StorageError::InvalidPath(err.to_string()))
                })
            } else {
                Ok(format_roots_status_text(&output))
            }
        }
        BranchesCommands::Discover { format } => {
            let output = BranchRuntime::new().discover_branches()?;
            if format == "json" {
                serde_json::to_string_pretty(&output).map_err(|err| {
                    ApiError::StorageError(StorageError::InvalidPath(err.to_string()))
                })
            } else {
                Ok(format_roots_status_text(&output))
            }
        }
        BranchesCommands::Migrate { format } => {
            let output = BranchRuntime::new().migrate_branches()?;
            if format == "json" {
                serde_json::to_string_pretty(&output).map_err(|err| {
                    ApiError::StorageError(StorageError::InvalidPath(err.to_string()))
                })
            } else {
                Ok(format_roots_status_text(&output))
            }
        }
        BranchesCommands::Attach { path, format } => {
            let output = BranchRuntime::new().attach_branch(path)?;
            if format == "json" {
                serde_json::to_string_pretty(&output).map_err(|err| {
                    ApiError::StorageError(StorageError::InvalidPath(err.to_string()))
                })
            } else {
                Ok(format_roots_status_text(&output))
            }
        }
    }
}
