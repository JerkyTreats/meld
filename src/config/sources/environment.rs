//! Environment variable source: MERKLE_* prefix with __ separator

use config::builder::DefaultState;
use config::ConfigBuilder;
use config::ConfigError;
use config::Environment;

/// Add environment variable overlay to builder.
/// Uses MERKLE_ prefix and __ as separator for nested keys.
pub fn add_to_builder(
    builder: ConfigBuilder<DefaultState>,
) -> Result<ConfigBuilder<DefaultState>, ConfigError> {
    let builder = builder.add_source(
        Environment::with_prefix("MERKLE")
            .separator("__")
            .try_parsing(true),
    );
    Ok(builder)
}
