use crate::error::ApiError;
use crate::provider::profile::ProviderConfig;
use crate::provider::storage::{ProviderStorage, StoredProviderConfig};

pub struct XdgProviderStorage;

impl XdgProviderStorage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for XdgProviderStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderStorage for XdgProviderStorage {
    fn list(&self) -> Result<Vec<StoredProviderConfig>, ApiError> {
        let providers_dir = crate::config::xdg::providers_dir()?;
        if !providers_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(&providers_dir).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to read providers directory {}: {}",
                providers_dir.display(),
                e
            ))
        })?;

        let mut loaded = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(
                        "Failed to read directory entry in {}: {}",
                        providers_dir.display(),
                        e
                    );
                    continue;
                }
            };

            let path = entry.path();
            if path.extension() != Some(std::ffi::OsStr::new("toml")) {
                continue;
            }

            let provider_name = match path.file_stem().and_then(|s| s.to_str()) {
                Some(name) => name.to_string(),
                None => {
                    tracing::warn!("Invalid provider filename non UTF8: {:?}", path);
                    continue;
                }
            };

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to read provider config {}: {}", path.display(), e);
                    continue;
                }
            };

            let mut config: ProviderConfig = match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::error!("Failed to parse provider config {}: {}", path.display(), e);
                    continue;
                }
            };

            if let Some(config_name) = &config.provider_name {
                if config_name != &provider_name {
                    tracing::warn!(
                        "Provider name mismatch in {}: filename={}, config={}",
                        path.display(),
                        provider_name,
                        config_name
                    );
                }
            }

            if config.provider_name.is_none() {
                config.provider_name = Some(provider_name.clone());
            }

            if let Err(e) = config.validate() {
                tracing::error!("Invalid provider config {}: {}", path.display(), e);
                continue;
            }

            loaded.push(StoredProviderConfig {
                provider_name,
                config,
                path,
            });
        }

        Ok(loaded)
    }

    fn path_for(&self, provider_name: &str) -> Result<std::path::PathBuf, ApiError> {
        let providers_dir = crate::config::xdg::providers_dir()?;
        Ok(providers_dir.join(format!("{}.toml", provider_name)))
    }

    fn save(&self, provider_name: &str, config: &ProviderConfig) -> Result<(), ApiError> {
        let config_path = self.path_for(provider_name)?;

        let providers_dir = crate::config::xdg::providers_dir()?;
        std::fs::create_dir_all(&providers_dir).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to create providers directory {}: {}",
                providers_dir.display(),
                e
            ))
        })?;

        let mut config = config.clone();
        if config.provider_name.is_none() {
            config.provider_name = Some(provider_name.to_string());
        }

        let toml_content = toml::to_string_pretty(&config).map_err(|e| {
            ApiError::ConfigError(format!("Failed to serialize provider config: {}", e))
        })?;

        std::fs::write(&config_path, toml_content).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to write provider config to {}: {}",
                config_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn delete(&self, provider_name: &str) -> Result<(), ApiError> {
        let config_path = self.path_for(provider_name)?;
        if !config_path.exists() {
            return Err(ApiError::ConfigError(format!(
                "Provider config file not found: {}",
                config_path.display()
            )));
        }

        std::fs::remove_file(&config_path).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to delete provider config file {}: {}",
                config_path.display(),
                e
            ))
        })
    }
}
