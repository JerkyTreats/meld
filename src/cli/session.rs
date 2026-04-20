use crate::error::ApiError;
use crate::telemetry::ProgressRuntime;

pub fn start_command_session(
    progress: &ProgressRuntime,
    command_name: &str,
) -> Result<String, ApiError> {
    progress.start_command_session(command_name.to_string())
}

pub fn finish_command_session(
    progress: &ProgressRuntime,
    session_id: &str,
    success: bool,
    error: Option<String>,
) -> Result<(), ApiError> {
    progress.finish_command_session(session_id, success, error)
}
