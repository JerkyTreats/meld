//! MergeService: orchestrates sources, applies merge policy, deserializes to MerkleConfig.

use crate::config::sources::{environment, global_file, workspace_file};
use crate::config::stewardship::selection::SelectionOrigins;
use crate::config::MerkleConfig;
use config::ConfigError;
use std::path::Path;

use super::merge_policy;

/// Whether a workspace-local config participates in the merge.
///
/// Workspace config is absent unless a caller explicitly selects a
/// workspace root; no source is ever discovered from the process working
/// directory.
#[derive(Debug, Clone, Copy)]
pub enum WorkspaceParticipation<'a> {
    /// No workspace-local config source is consulted.
    Absent,
    /// The workspace at this root explicitly contributes its config files.
    Selected(&'a Path),
}

/// Merge service for config composition.
pub struct MergeService;

impl MergeService {
    /// Load config with explicit workspace participation.
    /// Precedence: global file (lowest) -> workspace base -> workspace env -> environment (highest).
    pub fn load_with(participation: WorkspaceParticipation) -> Result<MerkleConfig, ConfigError> {
        let builder = merge_policy::builder_with_defaults()?;
        let builder = global_file::add_to_builder(builder)?;
        let builder = match participation {
            WorkspaceParticipation::Absent => builder,
            WorkspaceParticipation::Selected(workspace_root) => {
                workspace_file::add_to_builder(builder, workspace_root)?
            }
        };
        let builder = environment::add_to_builder(builder)?;

        finish(builder.build()?)
    }

    /// Load config with the given workspace root explicitly selected.
    pub fn load(workspace_root: &Path) -> Result<MerkleConfig, ConfigError> {
        Self::load_with(WorkspaceParticipation::Selected(workspace_root))
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

        finish(builder.build()?)
    }
}

/// Deserialize the merged config and run source-aware docs freshness
/// validation, so an invalid docs field names its config source and field.
fn finish(config: config::Config) -> Result<MerkleConfig, ConfigError> {
    // Origins must be captured before deserialization consumes the tree.
    let origins = SelectionOrigins::from_source(&config);
    let merkle: MerkleConfig = config.try_deserialize()?;
    if let Some(selection) = &merkle.stewardship.docs_freshness {
        selection.validate_sourced(&origins).map_err(|errors| {
            let joined = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            ConfigError::Message(format!(
                "invalid docs freshness stewardship selection: {joined}"
            ))
        })?;
    }
    Ok(merkle)
}
