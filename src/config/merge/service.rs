//! MergeService: orchestrates sources, applies merge policy, deserializes to MerkleConfig.

use crate::config::sources::{environment, global_file, workspace_file};
use crate::config::MerkleConfig;
use config::ConfigError;
use std::path::Path;

use super::merge_policy;

/// Merge service for config composition.
pub struct MergeService;

impl MergeService {
    /// Load config from workspace and standard sources.
    /// Precedence: global file (lowest) -> workspace base -> workspace env -> environment (highest).
    pub fn load(workspace_root: &Path) -> Result<MerkleConfig, ConfigError> {
        let builder = merge_policy::builder_with_defaults()?;
        let builder = global_file::add_to_builder(builder)?;
        let builder = workspace_file::add_to_builder(builder, workspace_root)?;
        let builder = environment::add_to_builder(builder)?;

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Load config from a specific file with environment overlay.
    pub fn load_from_file(path: &Path) -> Result<MerkleConfig, ConfigError> {
        use config::Environment;
        use config::File;

        let builder = merge_policy::builder_with_defaults()?;
        let builder = builder.add_source(File::with_name(path.to_str().unwrap()));
        let builder = builder.add_source(
            Environment::with_prefix("MERKLE")
                .separator("__")
                .try_parsing(true),
        );

        let config = builder.build()?;
        config.try_deserialize()
    }
}
