use crate::error::ApiError;
use crate::provider::profile::ProviderConfig;
use crate::provider::ModelProviderClient;

pub trait ProviderClientResolver: Send + Sync {
    fn resolve_provider_config(&self, provider_name: &str) -> Result<ProviderConfig, ApiError>;
    fn create_provider_client(
        &self,
        provider_name: &str,
    ) -> Result<Box<dyn ModelProviderClient>, ApiError>;
}
