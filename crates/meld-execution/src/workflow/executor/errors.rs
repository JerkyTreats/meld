use crate::error::ExecutionInvariantError;

pub(super) fn config_error<E>(message: String) -> E
where
    E: From<ExecutionInvariantError>,
{
    E::from(ExecutionInvariantError::ConfigError(message))
}

pub(super) fn generation_failed<E>(message: String) -> E
where
    E: From<ExecutionInvariantError>,
{
    E::from(ExecutionInvariantError::GenerationFailed(message))
}
