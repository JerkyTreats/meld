//! Adapt native init to the existing package and genesis pipeline.

use std::path::Path;

use crate::config::MerkleConfig;
use crate::error::ApiError;
use crate::runtime::assembly::ProductRuntimeAssembly;

pub fn handle_cli_command(
    assembly: &ProductRuntimeAssembly,
    config: &MerkleConfig,
    workspace: &Path,
    package: Option<&Path>,
    json: bool,
    session_id: &str,
) -> Result<String, ApiError> {
    let report = super::prepare_product(assembly, config, workspace, package, session_id)?;
    let format = if json { "json" } else { "text" };
    let mut output = crate::cli::format_world_init_report(&report, format)?;
    if !json {
        output.push_str("\nPrepared. Start with: meld runtime run\n");
    }
    Ok(output)
}
