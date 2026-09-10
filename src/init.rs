//! Native product preparation through config, theory and genesis owners.

mod bootstrap;
mod startup;
pub mod summary;
pub mod tooling;
pub mod world;

pub use bootstrap::{prepare_configuration, PreparedConfiguration};

/// Prepare a configured native product through the canonical package/genesis path.
pub fn prepare_product(
    assembly: &crate::runtime::assembly::ProductRuntimeAssembly,
    config: &crate::config::MerkleConfig,
    workspace: &std::path::Path,
    package: Option<&std::path::Path>,
    session_id: &str,
) -> Result<world::WorldInitReport, crate::error::ApiError> {
    let binding = crate::config::PhysicalBinding::resolve_for_target(config, workspace)?
        .ok_or_else(|| {
            crate::error::ApiError::ConfigError("init requires a configured native product".into())
        })?;
    let bundled = if package.is_none() && binding.package.expression == "startup" {
        Some(startup::package_source()?)
    } else {
        None
    };
    world::tooling::run_world_init(
        assembly,
        config,
        workspace,
        &[],
        package.or(bundled.as_deref()),
        session_id,
    )
}
