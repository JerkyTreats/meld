use crate::error::ApiError;
use crate::provider::profile::ProviderConfig;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct StoredProviderConfig {
    pub provider_name: String,
    pub config: ProviderConfig,
    pub path: PathBuf,
}

pub trait ProviderStorage: Send + Sync {
    fn list(&self) -> Result<Vec<StoredProviderConfig>, ApiError>;
    fn path_for(&self, provider_name: &str) -> Result<PathBuf, ApiError>;
    fn save(&self, provider_name: &str, config: &ProviderConfig) -> Result<(), ApiError>;
    fn delete(&self, provider_name: &str) -> Result<(), ApiError>;
}
