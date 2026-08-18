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
use crate::init::world::pipeline::{
    CompleteTheoryInstall, RoutedTheoryInstall, WorldInitContent, WorldInitPipeline,
    WorldInitTheoryBundle,
};
use crate::init::world::theory::{
    load_authority_policy, load_belief_family_config, load_claim_policy, load_curation_rule_config,
    load_maintained_condition, load_outcome_mapping_config, load_strategy_theory_package,
};
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
    theory_source: Option<&Path>,
    session_id: &str,
) -> Result<WorldInitReport, ApiError> {
    let request = WorldInitRequest {
        stages: parse_stage_args(stage_args)?,
    };

    // Stage 0 inputs: XDG-only configuration and the physical binding.
    let config = ConfigLoader::load_global()?;
    let binding = PhysicalBinding::resolve_for_target(&config, target_path)?.ok_or_else(|| {
        ApiError::ConfigError(format!(
            "no stewardship declaration targets '{}'",
            target_path.display()
        ))
    })?;

    // An explicit theory source provisions the XDG theory root before the
    // loaders resolve selection identities against it. Provisioning is
    // config-home file placement only; durable installation stays with the
    // staged pipeline below.
    let subject = DomainObjectRef::new(SUBJECT_DOMAIN_ID, SUBJECT_OBJECT_KIND, &binding.subject)
        .map_err(|error| world_init_error(error.to_string()))?;
    if let Some(source_dir) = theory_source {
        crate::init::world::source::provision_theory_source(
            source_dir,
            &binding.package,
            &subject,
        )?;
    }

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
    let maintained_condition = load_maintained_condition(&binding.package.maintained_condition_id)?;
    let outcome_mapping = load_outcome_mapping_config(&binding.package.evidence_mapping_id)?;
    let strategy_theory = load_strategy_theory_package(&binding.package.strategy_theory_id)?;
    let authority_policy = load_authority_policy(&binding.package.authority_policy_id)?;
    let claim_policy = load_claim_policy(&binding.package.claim_policy_id)?;
    let executable_contracts = select_strategy_contracts(
        &strategy_theory,
        &crate::capability::published_product_contracts(),
    )?;

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

    let complete = CompleteTheoryInstall {
        selection: binding.package.clone(),
        bundle: WorldInitTheoryBundle {
            family: content.family_config.clone(),
            curation_rule: content.curation_rule.clone(),
            maintained_condition,
            outcome_mapping,
            strategy_theory,
            executable_contracts,
            authority_policy,
            claim_policy,
        },
        curation_rules: stores.curation_rule_registry.as_ref(),
        maintained_conditions: stores.maintained_condition_registry.as_ref(),
        outcome_mappings: stores.outcome_mapping_registry.as_ref(),
        strategy_theories: stores.strategy_theory_registry.as_ref(),
        executable_contracts: stores.capability_contract_registry.as_ref(),
        authority_policies: stores.authority_policy_registry.as_ref(),
        claim_policies: stores.claim_policy_registry.as_ref(),
        receipts: stores.theory_receipts.as_ref(),
        routed: routed_install(stores, &request, theory_source, observed_seq)?,
    };
    WorldInitPipeline::new(&mut registry, stores.agent_store.as_ref(), &append)
        .with_complete_theory(complete)
        .run(&request, &content)
        .map_err(|error| world_init_error(error.to_string()))
}

fn routed_install(
    stores: &crate::runtime::storage::OpenProductStores,
    request: &WorldInitRequest,
    theory_source: Option<&Path>,
    observed_seq: u64,
) -> Result<Option<RoutedTheoryInstall>, ApiError> {
    if !request.stages.contains(&WorldInitStage::InstallTheory) {
        return Ok(None);
    }
    if let Some(source) = theory_source.filter(|root| root.join("pds-package.json").is_file()) {
        let prior = stores
            .pds_packages
            .head(crate::docs::theory::DOCS_PACKAGE_ID)
            .map_err(|failure| world_init_error(failure.to_string()))?;
        let receipt = crate::docs::theory::install_package(stores, source, observed_seq)
            .map_err(|failure| world_init_error(failure.to_string()))?;
        let changed = prior.as_ref().map(|head| head.receipt_id.as_str())
            != Some(receipt.receipt_id.as_str());
        return Ok(Some(RoutedTheoryInstall { receipt, changed }));
    }
    let heads = stores
        .pds_packages
        .heads()
        .map_err(|failure| world_init_error(failure.to_string()))?;
    if heads.len() == 1 {
        let receipt = stores
            .pds_packages
            .resolve_receipt(&heads[0].receipt_id)
            .map_err(|failure| world_init_error(failure.to_string()))?
            .ok_or_else(|| world_init_error("routed package head cites a missing receipt"))?;
        return Ok(Some(RoutedTheoryInstall {
            receipt,
            changed: false,
        }));
    }
    Ok(None)
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

fn select_strategy_contracts(
    strategy: &meld_world_model::strategy::StrategyTheoryPackage,
    published: &[meld_execution::capability::CapabilityTypeContract],
) -> Result<Vec<meld_execution::capability::CapabilityTypeContract>, ApiError> {
    let mut selected = Vec::new();
    for capability in &strategy.capabilities {
        let selector = capability
            .operator
            .resolution
            .specific
            .as_ref()
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Strategy capability '{}' has no exact executable selector",
                    capability.operator.operator_id
                ))
            })?;
        let contract = published
            .iter()
            .find(|contract| {
                contract.capability_type_id == selector.capability_type_id
                    && contract.capability_version == selector.capability_version
                    && contract.content_identity() == capability.contract_id
            })
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Strategy capability '{}' has no exact product implementation",
                    capability.operator.operator_id
                ))
            })?;
        if !selected.iter().any(
            |existing: &meld_execution::capability::CapabilityTypeContract| {
                existing.capability_type_id == contract.capability_type_id
                    && existing.capability_version == contract.capability_version
            },
        ) {
            selected.push(contract.clone());
        }
    }
    Ok(selected)
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

    #[test]
    fn strategy_selects_product_contracts_by_exact_identity() {
        let strategy: meld_world_model::strategy::StrategyTheoryPackage = serde_json::from_str(
            include_str!("../../../theory/docs_freshness/strategy_theory.docs_freshness.json"),
        )
        .unwrap();

        let selected =
            select_strategy_contracts(&strategy, &crate::capability::published_product_contracts())
                .unwrap();

        assert_eq!(selected.len(), strategy.capabilities.len());
    }

    #[test]
    fn strategy_contract_selection_rejects_missing_implementation() {
        let mut strategy: meld_world_model::strategy::StrategyTheoryPackage = serde_json::from_str(
            include_str!("../../../theory/docs_freshness/strategy_theory.docs_freshness.json"),
        )
        .unwrap();
        strategy.capabilities[0].contract_id = "absent-contract".to_string();

        let error =
            select_strategy_contracts(&strategy, &crate::capability::published_product_contracts())
                .unwrap_err();

        assert!(error
            .to_string()
            .contains("no exact product implementation"));
    }
}
