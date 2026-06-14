//! Task artifact repository storage codec helpers.

use crate::task::artifact_repo::error::TaskArtifactRepoError;

pub(super) fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, TaskArtifactRepoError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_decode)?))
}

pub(super) fn to_storage(error: sled::Error) -> TaskArtifactRepoError {
    TaskArtifactRepoError::Storage(error.to_string())
}

pub(super) fn to_decode(error: serde_json::Error) -> TaskArtifactRepoError {
    TaskArtifactRepoError::Decode(error.to_string())
}

pub(super) fn decode_error(message: impl Into<String>) -> TaskArtifactRepoError {
    TaskArtifactRepoError::Decode(message.into())
}
