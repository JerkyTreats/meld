//! Configuration System
//!
//! Runtime-driven configuration system that enables dynamic agent behavior and model provider
//! management. Supports hierarchical configuration with environment variable overrides and
//! runtime validation. Tests included.

#[cfg(test)]
use crate::agent::AgentRole;
use crate::error::ApiError;
use crate::logging::LoggingConfig;
#[cfg(test)]
use crate::provider::CompletionOptions;
#[cfg(test)]
use crate::provider::ModelProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[cfg(test)]
use std::sync::Mutex;

pub use crate::agent::AgentConfig;
pub use crate::provider::{ProviderConfig, ProviderType};

mod facade;
mod merge;
mod paths;
mod sources;
mod workspace;

pub use facade::ConfigLoader;
pub use workspace::StorageConfig;

/// Backward-compatible re-export of XDG path helpers
pub mod xdg {
    pub use super::paths::xdg_root::*;
}

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MerkleConfig {
    /// Workspace root path (defaults to current directory)
    pub workspace_root: Option<PathBuf>,

    /// Model provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Agent definitions
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,

    /// System-wide settings
    #[serde(default)]
    pub system: SystemConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Workflow profile loading configuration
    #[serde(default)]
    pub workflows: WorkflowConfig,
}

/// System-wide configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Default workspace root (if not specified)
    #[serde(default = "default_workspace_root")]
    pub default_workspace_root: PathBuf,

    /// Storage paths
    #[serde(default)]
    pub storage: StorageConfig,
}

/// Workflow profile loading configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowConfig {
    /// User profile directory.
    /// Relative values are resolved against XDG config home.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_profile_dir: Option<PathBuf>,
}

fn default_workspace_root() -> PathBuf {
    PathBuf::from(".")
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            default_workspace_root: default_workspace_root(),
            storage: StorageConfig::default(),
        }
    }
}

impl WorkflowConfig {
    /// Resolve user workflow profile directory.
    pub fn resolve_user_profile_dir(&self) -> Result<PathBuf, ApiError> {
        let configured = self
            .user_profile_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("meld").join("workflows"));
        if configured.is_absolute() {
            return Ok(configured);
        }

        let config_home = crate::config::xdg::config_home()?;
        Ok(config_home.join(configured))
    }

    /// Validate workflow configuration.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(path) = &self.user_profile_dir {
            if path.as_os_str().is_empty() {
                return Err("user_profile_dir cannot be empty".to_string());
            }
        }
        Ok(())
    }
}

/// Configuration validation errors
#[derive(Debug, Clone)]
pub enum ValidationError {
    Provider(String, String),
    Agent(String, String),
    System(String),
    Workflow(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Provider(name, msg) => {
                write!(f, "Provider '{}': {}", name, msg)
            }
            ValidationError::Agent(name, msg) => {
                write!(f, "Agent '{}': {}", name, msg)
            }
            ValidationError::System(msg) => {
                write!(f, "System: {}", msg)
            }
            ValidationError::Workflow(msg) => {
                write!(f, "Workflow: {}", msg)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl SystemConfig {
    /// Validate system configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate storage paths are not empty
        if self.storage.store_path.as_os_str().is_empty() {
            return Err("Store path cannot be empty".to_string());
        }
        if self.storage.frames_path.as_os_str().is_empty() {
            return Err("Frames path cannot be empty".to_string());
        }
        if self.storage.artifacts_path.as_os_str().is_empty() {
            return Err("Artifacts path cannot be empty".to_string());
        }

        Ok(())
    }
}

impl MerkleConfig {
    /// Validate the entire configuration
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Validate providers
        for (name, provider) in &self.providers {
            if let Err(e) = provider.validate() {
                errors.push(ValidationError::Provider(name.clone(), e));
            }
        }

        // Validate agents
        for (name, agent) in &self.agents {
            if let Err(e) = agent.validate(&self.providers) {
                errors.push(ValidationError::Agent(name.clone(), e));
            }
        }

        // Validate system config
        if let Err(e) = self.system.validate() {
            errors.push(ValidationError::System(e));
        }

        // Validate workflow config
        if let Err(e) = self.workflows.validate() {
            errors.push(ValidationError::Workflow(e));
        }

        // Check for duplicate agent IDs
        let mut agent_ids = HashMap::new();
        for (name, agent) in &self.agents {
            if let Some(existing) = agent_ids.insert(&agent.agent_id, name) {
                errors.push(ValidationError::Agent(
                    name.clone(),
                    format!(
                        "Duplicate agent_id '{}' (also defined in '{}')",
                        agent.agent_id, existing
                    ),
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Configuration manager for runtime updates
pub struct ConfigManager {
    config: Arc<RwLock<MerkleConfig>>,
}

impl ConfigManager {
    /// Create a new configuration manager with the given config
    pub fn new(config: MerkleConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Reload configuration from files
    pub fn reload(&self, workspace_root: &Path) -> Result<(), ApiError> {
        let new_config = ConfigLoader::load(workspace_root)
            .map_err(|e| ApiError::ConfigError(format!("Failed to load config: {}", e)))?;

        // Validate new configuration
        new_config.validate().map_err(|errors| {
            let error_msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            ApiError::ConfigError(format!(
                "Configuration validation failed:\n{}",
                error_msgs.join("\n")
            ))
        })?;

        *self.config.write().unwrap() = new_config;
        Ok(())
    }

    /// Get current configuration (read-only)
    pub fn get(&self) -> MerkleConfig {
        self.config.read().unwrap().clone()
    }

    /// Get a mutable reference to the configuration (for runtime updates)
    pub fn get_mut(&mut self) -> &mut MerkleConfig {
        // This requires &mut self, so we need to restructure if we want thread-safe updates
        // For now, we'll use reload for updates
        unimplemented!("Use reload() for configuration updates")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = MerkleConfig::default();
        assert!(config.providers.is_empty());
        assert!(config.agents.is_empty());
        assert_eq!(config.system.default_workspace_root, PathBuf::from("."));
    }

    #[test]
    fn test_provider_config_validation() {
        let mut provider = ProviderConfig {
            provider_name: Some("test-openai".to_string()),
            provider_type: ProviderType::OpenAI,
            model: "gpt-4".to_string(),
            api_key: Some("test-key".to_string()),
            endpoint: None,
            default_options: CompletionOptions::default(),
        };
        assert!(provider.validate().is_ok());

        // Empty model should fail
        provider.model = "".to_string();
        assert!(provider.validate().is_err());

        // Invalid endpoint should fail
        provider.model = "gpt-4".to_string();
        provider.endpoint = Some("not-a-url".to_string());
        assert!(provider.validate().is_err());
    }

    #[test]
    fn test_agent_config_validation() {
        let mut providers = HashMap::new();
        providers.insert(
            "test-provider".to_string(),
            ProviderConfig {
                provider_name: Some("test-provider".to_string()),
                provider_type: ProviderType::Ollama,
                model: "llama2".to_string(),
                api_key: None,
                endpoint: None,
                default_options: CompletionOptions::default(),
            },
        );

        let agent = AgentConfig {
            agent_id: "test-agent".to_string(),
            role: AgentRole::Writer,
            system_prompt: Some("Test prompt".to_string()),
            system_prompt_path: None,
            workflow_id: None,
            metadata: Default::default(),
        };
        assert!(agent.validate(&providers).is_ok());

        // Writer agents require either system_prompt or system_prompt_path
        let agent_bad = AgentConfig {
            agent_id: "test-agent-2".to_string(),
            role: AgentRole::Writer,
            system_prompt: None,
            system_prompt_path: None,
            workflow_id: None,
            metadata: Default::default(),
        };
        assert!(agent_bad.validate(&providers).is_err());

        // Reader agents don't require prompts
        let agent_reader = AgentConfig {
            agent_id: "test-agent-3".to_string(),
            role: AgentRole::Reader,
            system_prompt: None,
            system_prompt_path: None,
            workflow_id: None,
            metadata: Default::default(),
        };
        assert!(agent_reader.validate(&providers).is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = MerkleConfig::default();

        // Add a valid provider
        config.providers.insert(
            "test-provider".to_string(),
            ProviderConfig {
                provider_name: Some("test-provider".to_string()),
                provider_type: ProviderType::Ollama,
                model: "llama2".to_string(),
                api_key: None,
                endpoint: None,
                default_options: CompletionOptions::default(),
            },
        );

        // Add a valid agent
        config.agents.insert(
            "test-agent".to_string(),
            AgentConfig {
                agent_id: "test-agent".to_string(),
                role: AgentRole::Writer,
                system_prompt: Some("Test".to_string()),
                system_prompt_path: None,
                workflow_id: None,
                metadata: Default::default(),
            },
        );

        assert!(config.validate().is_ok());

        // Duplicate agent IDs should fail
        config.agents.insert(
            "test-agent-2".to_string(),
            AgentConfig {
                agent_id: "test-agent".to_string(), // Same ID
                role: AgentRole::Reader,
                system_prompt: None,
                system_prompt_path: None,
                workflow_id: None,
                metadata: Default::default(),
            },
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_provider_to_model_provider() {
        let provider_config = ProviderConfig {
            provider_name: Some("test-ollama".to_string()),
            provider_type: ProviderType::Ollama,
            model: "llama2".to_string(),
            api_key: None,
            endpoint: Some("http://localhost:11434".to_string()),
            default_options: CompletionOptions::default(),
        };

        let model_provider = provider_config.to_model_provider().unwrap();
        match model_provider {
            ModelProvider::Ollama { model, base_url } => {
                assert_eq!(model, "llama2");
                assert_eq!(base_url, Some("http://localhost:11434".to_string()));
            }
            _ => panic!("Wrong provider type"),
        }
    }

    #[test]
    fn test_config_loader_default() {
        let config = ConfigLoader::load_default();
        assert!(config.providers.is_empty());
        assert!(config.agents.is_empty());
    }

    #[test]
    fn test_load_from_toml_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test_config.toml");

        std::fs::write(
            &config_file,
            r#"
[system]
default_workspace_root = "."

[system.storage]
store_path = ".meld/store"
frames_path = ".meld/frames"

[providers.test-ollama]
provider_type = "ollama"
model = "llama2"
endpoint = "http://localhost:11434"

[agents.test-agent]
agent_id = "test-agent"
role = "Writer"
system_prompt = "Test prompt"
provider_name = "test-ollama"
"#,
        )
        .unwrap();

        let config = ConfigLoader::load_from_file(&config_file).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.agents.len(), 1);

        let provider = config.providers.get("test-ollama").unwrap();
        assert_eq!(provider.model, "llama2");

        let agent = config.agents.get("test-agent").unwrap();
        assert_eq!(agent.agent_id, "test-agent");
        assert_eq!(agent.system_prompt.as_ref().unwrap(), "Test prompt");
    }

    // Serializes HOME and XDG_CONFIG_HOME manipulation across tests and
    // restores both (even on panic) when the sandbox drops.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvSandbox {
        _guard: std::sync::MutexGuard<'static, ()>,
        saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl EnvSandbox {
        fn new() -> Self {
            let guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
            let saved = ["HOME", "XDG_CONFIG_HOME"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect();
            std::env::remove_var("XDG_CONFIG_HOME");
            Self {
                _guard: guard,
                saved,
            }
        }
    }

    impl Drop for EnvSandbox {
        fn drop(&mut self) {
            for (key, value) in self.saved.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    fn write_global_config(home: &Path, body: &str) -> PathBuf {
        let config_dir = home.join(".config").join("meld");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("config.toml");
        std::fs::write(&config_file, body).unwrap();
        config_file
    }

    const PROVIDER_ONLY_GLOBAL: &str = r#"
[providers.xdg-provider]
provider_type = "ollama"
model = "xdg-model"
endpoint = "http://localhost:11434"
"#;

    #[test]
    fn test_xdg_config_path() {
        let _sandbox = EnvSandbox::new();
        std::env::set_var("HOME", "/test/home");

        assert_eq!(
            ConfigLoader::xdg_config_path().unwrap(),
            PathBuf::from("/test/home/.config/meld/config.toml")
        );

        std::env::set_var("XDG_CONFIG_HOME", "/test/xdg");
        assert_eq!(
            ConfigLoader::xdg_config_path().unwrap(),
            PathBuf::from("/test/xdg/meld/config.toml")
        );
    }

    #[test]
    fn test_load_with_xdg_config() {
        let _sandbox = EnvSandbox::new();
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        let mock_home = temp_dir.path().join("mock_home");
        std::fs::create_dir_all(&mock_home).unwrap();
        std::env::set_var("HOME", mock_home.canonicalize().unwrap());
        let xdg_config_file = write_global_config(&mock_home, PROVIDER_ONLY_GLOBAL);

        assert_eq!(ConfigLoader::xdg_config_path().unwrap(), xdg_config_file);

        let config = ConfigLoader::load(workspace_root).unwrap();
        let provider = config.providers.get("xdg-provider").unwrap();
        assert_eq!(provider.model, "xdg-model");
    }

    #[test]
    fn xdg_config_home_wins_over_home() {
        let _sandbox = EnvSandbox::new();
        let temp_dir = TempDir::new().unwrap();

        // HOME fallback config says one model, XDG_CONFIG_HOME another.
        let mock_home = temp_dir.path().join("mock_home");
        std::fs::create_dir_all(&mock_home).unwrap();
        write_global_config(
            &mock_home,
            r#"
[providers.shared-provider]
provider_type = "ollama"
model = "home-model"
endpoint = "http://localhost:11434"
"#,
        );
        let xdg_home = temp_dir.path().join("xdg_home");
        let xdg_meld_dir = xdg_home.join("meld");
        std::fs::create_dir_all(&xdg_meld_dir).unwrap();
        std::fs::write(
            xdg_meld_dir.join("config.toml"),
            r#"
[providers.shared-provider]
provider_type = "ollama"
model = "xdg-model"
endpoint = "http://localhost:11434"
"#,
        )
        .unwrap();
        std::env::set_var("HOME", mock_home.canonicalize().unwrap());
        std::env::set_var("XDG_CONFIG_HOME", xdg_home.canonicalize().unwrap());

        let config = ConfigLoader::load(temp_dir.path()).unwrap();

        let provider = config.providers.get("shared-provider").unwrap();
        assert_eq!(provider.model, "xdg-model");
    }

    #[test]
    fn test_workspace_config_overrides_xdg_config() {
        let _sandbox = EnvSandbox::new();
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        let mock_home = temp_dir.path().join("mock_home_override");
        std::fs::create_dir_all(&mock_home).unwrap();
        std::env::set_var("HOME", mock_home.canonicalize().unwrap());
        write_global_config(&mock_home, PROVIDER_ONLY_GLOBAL);

        let workspace_config_dir = workspace_root.join("config");
        std::fs::create_dir_all(&workspace_config_dir).unwrap();
        std::fs::write(
            workspace_config_dir.join("config.toml"),
            r#"
[providers.xdg-provider]
provider_type = "ollama"
model = "workspace-model"
endpoint = "http://localhost:11434"
"#,
        )
        .unwrap();

        // The workspace config wins over the global file.
        let config = ConfigLoader::load(workspace_root).unwrap();
        let provider = config.providers.get("xdg-provider").unwrap();
        assert_eq!(provider.model, "workspace-model");
    }

    #[test]
    fn test_load_without_xdg_config() {
        let _sandbox = EnvSandbox::new();
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        let mock_home = temp_dir.path().join("mock_home_no_config");
        std::fs::create_dir_all(&mock_home).unwrap();
        std::env::set_var("HOME", mock_home.canonicalize().unwrap());

        let config = ConfigLoader::load(workspace_root).unwrap();
        assert_eq!(config.providers.len(), 0);
        assert_eq!(config.agents.len(), 0);
    }

    #[test]
    fn test_load_without_home_env() {
        let _sandbox = EnvSandbox::new();
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        std::env::remove_var("HOME");

        assert!(
            ConfigLoader::xdg_config_path().is_none(),
            "XDG config path should be None when HOME is not set"
        );

        let config = ConfigLoader::load(workspace_root).unwrap();
        assert_eq!(config.providers.len(), 0);
        assert_eq!(config.agents.len(), 0);
    }
}
