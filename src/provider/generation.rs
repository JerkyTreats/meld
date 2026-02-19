use crate::error::ApiError;
use crate::provider::clients::ProviderClientResolver;
use crate::provider::profile::ProviderConfig;
use crate::provider::ModelProviderClient;

pub struct ProviderGenerationService<'a, R: ProviderClientResolver> {
    resolver: &'a R,
}

impl<'a, R: ProviderClientResolver> ProviderGenerationService<'a, R> {
    pub fn new(resolver: &'a R) -> Self {
        Self { resolver }
    }

    pub fn resolve_provider(
        &self,
        provider_name: &str,
    ) -> Result<(ProviderConfig, Box<dyn ModelProviderClient>), ApiError> {
        let config = self.resolver.resolve_provider_config(provider_name)?;
        let client = self.resolver.create_provider_client(provider_name)?;
        Ok((config, client))
    }
}
