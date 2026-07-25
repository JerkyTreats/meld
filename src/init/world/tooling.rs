//! CLI adapter for the world initialization command surface.
//!
//! Owner: root init. Parses the stage selection, resolves stage 0 inputs
//! (XDG-only configuration, the physical binding, and the theory bodies),
//! guards that the addressed target workspace and the open product stores
//! belong to the configured selection, and delegates to
//! [`WorldInitPipeline`]. No runtime state is created under the target
//! workspace: every durable effect lands in the external product storage
//! root through domain commands and the canonical append.

use std::path::Path;

use meld_events::DomainObjectRef;
use meld_world_model::belief::{BeliefFamilyRegistryStore, BranchScope};
use meld_world_model::PerspectiveKey;

use crate::config::{ConfigLoader, PhysicalBinding};
use crate::error::ApiError;
use crate::init::world::pipeline::{WorldInitContent, WorldInitPipeline};
use crate::init::world::theory::{load_belief_family_config, load_curation_rule_config};
use crate::init::world::{WorldInitReport, WorldInitRequest, WorldInitStage};
use crate::runtime::assembly::ProductRuntimeAssembly;

/// CLI stage-name vocabulary in pipeline order.
const STAGE_NAMES: [(&str, WorldInitStage); 3] = [
    ("install-theory", WorldInitStage::InstallTheory),
    ("genesis-identities", WorldInitStage::GenesisIdentities),
    ("seed-epistemic-facts", WorldInitStage::SeedEpistemicFacts),
];

/// Compatibility perspective registered for the seed agent.
///
/// The physical binding carries no perspective selection yet; the seed
/// identity uses the default perspective on the same convention as the
/// existing genesis fixtures.
const SEED_PERSPECTIVE_KIND: &str = "default";
const SEED_PERSPECTIVE_ID: &str = "default";

/// Domain and kind convention for the selected subtree subject.
///
/// The exact docs subject binding is owned by the world model; until the
/// runtime actor binding consumes it, the selection's subject identity is
/// addressed as a workspace filesystem node object.
const SUBJECT_DOMAIN_ID: &str = "workspace_fs";
const SUBJECT_OBJECT_KIND: &str = "node";

/// Provenance recorded by the initialization command surface.
const INIT_PROVENANCE: &str = "meld world init";

/// Run world initialization stages 2 through 4 for the addressed target.
///
/// Configuration resolves through the XDG config home only. The explicit
/// `target_path` argument must resolve to the configured selection's
/// target root, and the open product stores must belong to the selection's
/// storage root, so the command can never initialize a world other than
/// the one it addressed.
pub fn run_world_init(
    assembly: &ProductRuntimeAssembly,
    target_path: &Path,
    stage_args: &[String],
    session_id: &str,
) -> Result<WorldInitReport, ApiError> {
    let request = WorldInitRequest {
        stages: parse_stage_args(stage_args)?,
    };

    // Stage 0 inputs: XDG-only configuration and the physical binding.
    let config = ConfigLoader::load_global()?;
    let binding = PhysicalBinding::resolve(&config)?;

    let target_root = target_path.canonicalize().map_err(|error| {
        ApiError::ConfigError(format!(
            "world init target path '{}' cannot be resolved: {error}",
            target_path.display()
        ))
    })?;
    if target_root != binding.workspace_root {
        return Err(ApiError::ConfigError(format!(
            "world init target '{}' does not match the configured stewardship target root '{}'",
            target_root.display(),
            binding.workspace_root.display()
        )));
    }
    if assembly.product_root() != binding.storage_root {
        return Err(ApiError::ConfigError(format!(
            "open product storage root '{}' does not match the selection's storage root '{}'; \
             run with --workspace pointing at the configured target root",
            assembly.product_root().display(),
            binding.storage_root.display()
        )));
    }

    let family_config = load_belief_family_config(&binding.package.belief_family_id)?;
    let curation_rule = load_curation_rule_config(&binding.package.curation_rule_id)?;

    // The world-model registry opens over the same shared world-model
    // database the assembly holds; composition derives the handle from the
    // traversal store because the assembly does not yet expose a registry.
    let stores = assembly.stores();
    let mut registry = BeliefFamilyRegistryStore::new(stores.traversal_store.db().clone())
        .map_err(|error| world_init_error(error.to_string()))?;
    let authority = assembly.event_authority();
    let append = authority.append_capability();
    let observed_seq = authority
        .watermark_capability()
        .snapshot()
        .map_err(|error| world_init_error(error.to_string()))?
        .tip_seq;

    let subject = DomainObjectRef::new(SUBJECT_DOMAIN_ID, SUBJECT_OBJECT_KIND, &binding.subject)
        .map_err(|error| world_init_error(error.to_string()))?;
    let perspective = PerspectiveKey::new(SEED_PERSPECTIVE_KIND, SEED_PERSPECTIVE_ID)
        .map_err(|error| world_init_error(error.to_string()))?;
    let observation_scope = family_config.dimension_id.clone();
    let content = WorldInitContent {
        family_config,
        curation_rule,
        agent_id: binding.agent_id.clone(),
        subject,
        perspective,
        branch_scope: BranchScope::main(),
        observation_scope,
        // Compatibility form of the structured Directive record.
        directive: format!(
            "steward '{}' for subject '{}'",
            binding.package.expression, binding.subject
        ),
        provenance: INIT_PROVENANCE.to_string(),
        session_id: session_id.to_string(),
        observed_seq,
    };

    WorldInitPipeline::new(&mut registry, stores.agent_store.as_ref(), &append)
        .run(&request, &content)
        .map_err(|error| world_init_error(error.to_string()))
}

/// Parse `--stage` arguments; an empty selection means every stage.
fn parse_stage_args(stage_args: &[String]) -> Result<Vec<WorldInitStage>, ApiError> {
    if stage_args.is_empty() {
        return Ok(STAGE_NAMES.iter().map(|(_, stage)| *stage).collect());
    }
    stage_args
        .iter()
        .map(|arg| {
            STAGE_NAMES
                .iter()
                .find(|(name, _)| name == arg)
                .map(|(_, stage)| *stage)
                .ok_or_else(|| {
                    let valid = STAGE_NAMES
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    world_init_error(format!("unknown stage '{arg}', expected one of: {valid}"))
                })
        })
        .collect()
}

fn world_init_error(message: impl Into<String>) -> ApiError {
    ApiError::ConfigError(format!("World init failed: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stage_args_select_every_stage_in_pipeline_order() {
        let stages = parse_stage_args(&[]).unwrap();
        assert_eq!(
            stages,
            vec![
                WorldInitStage::InstallTheory,
                WorldInitStage::GenesisIdentities,
                WorldInitStage::SeedEpistemicFacts,
            ]
        );
    }

    #[test]
    fn stage_args_map_to_stages() {
        let stages = parse_stage_args(&[
            "seed-epistemic-facts".to_string(),
            "install-theory".to_string(),
        ])
        .unwrap();
        assert_eq!(
            stages,
            vec![
                WorldInitStage::SeedEpistemicFacts,
                WorldInitStage::InstallTheory,
            ]
        );
    }

    #[test]
    fn unknown_stage_arg_is_rejected() {
        let error = parse_stage_args(&["activate".to_string()]).unwrap_err();
        assert!(error.to_string().contains("unknown stage 'activate'"));
    }
}
