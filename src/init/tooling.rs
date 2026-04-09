use crate::cli::{format_init_preview, format_init_summary};
use crate::error::ApiError;

pub fn handle_cli_command(force: bool, list: bool) -> Result<String, ApiError> {
    if list {
        let preview = crate::init::list_initialization()?;
        Ok(format_init_preview(&preview))
    } else {
        let summary = crate::init::initialize_all(force)?;
        Ok(format_init_summary(&summary, force))
    }
}
