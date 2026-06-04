//! Task network store codec helpers.

use crate::task_network::store::error::TaskNetworkStoreError;

pub(super) fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, TaskNetworkStoreError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_decode)?))
}

pub(super) fn decode_error(message: impl Into<String>) -> TaskNetworkStoreError {
    TaskNetworkStoreError::Decode(message.into())
}

pub(super) fn to_storage(error: sled::Error) -> TaskNetworkStoreError {
    TaskNetworkStoreError::Storage(error.to_string())
}

pub(super) fn to_decode(error: serde_json::Error) -> TaskNetworkStoreError {
    TaskNetworkStoreError::Decode(error.to_string())
}
