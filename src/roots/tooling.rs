use crate::cli::RootsCommands;
use crate::error::{ApiError, StorageError};
use crate::roots::{BranchRuntime, format::format_roots_status_text};

pub fn handle_cli_command(command: &RootsCommands) -> Result<String, ApiError> {
    match command {
        RootsCommands::Status { format } => {
            let output = BranchRuntime::new().status()?;
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
