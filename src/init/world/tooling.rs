//! CLI adapter for the world initialization command surface.
//!
//! Owner: root init. Parses the stage selection, resolves stage 0 inputs
//! (XDG-only configuration, the physical binding, and the theory bodies),
//! guards that the addressed target workspace and the open product stores
//! belong to the configured selection, and delegates to the crate-private
//! world initialization pipeline. No runtime state is created under the target
//! workspace: every durable effect lands in the external product storage
//! root through domain commands and the canonical append.

use std::collections::BTreeMap;
use std::path::Path;

use meld_events::DomainObjectRef;
use meld_execution::capability::CapabilityContractRevisionRef;
use meld_world_model::belief::{BeliefFamilyRegistryStore, BranchScope};
use meld_world_model::PerspectiveKey;

use crate::capability::{OwnerBindingView, ProductCapabilityInventory};
use crate::config::{
    AdapterPlacement, AssignedAgentPositionV1, ConfigLoader, OperationalLimits, PhysicalBinding,
    PhysicalBindingRef, RuntimeIsolationRequirements, StewardshipActivationV1,
    StewardshipAssignmentV1,
};
use crate::error::ApiError;
use crate::init::world::pipeline::{
    CompleteProductInitialization, CompleteTheoryInstall, RoutedTheoryInstall, WorldInitPipeline,
    WorldInitRunMetadata,
};
use crate::init::world::{WorldInitReport, WorldInitRequest, WorldInitStage};
use crate::runtime::assembly::ProductRuntimeAssembly;
use crate::theory::{product_topology_id, ProductCompilationReceiptV1};

/// CLI stage-name vocabulary in pipeline order.
const STAGE_NAMES: [(&str, WorldInitStage); 4] = [
    ("install-theory", WorldInitStage::InstallTheory),
    ("genesis-identities", WorldInitStage::GenesisIdentities),
    ("prepare-activation", WorldInitStage::PrepareActivation),
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

/// Run world initialization stages 2 through 5 for the addressed target.
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

    let metadata = WorldInitRunMetadata {
        provenance: INIT_PROVENANCE.to_string(),
        session_id: session_id.to_string(),
        observed_seq,
    };

    let routed = routed_install(stores, &request, theory_source, observed_seq)?.ok_or_else(|| {
        world_init_error(
            "the canonical docs package is not installed; supply its package source to install-theory",
        )
    })?;
    let complete_product =
        compile_product_initialization(stores, &binding, &routed.receipt, observed_seq)?;
    let complete = CompleteTheoryInstall { routed };
    WorldInitPipeline::new(
        &mut registry,
        stores.belief_store.as_ref(),
        stores.agent_store.as_ref(),
        &append,
    )
    .with_complete_theory(complete)
    .with_complete_product(complete_product)
    .run(&request, &metadata)
    .map_err(|error| world_init_error(error.to_string()))
}

fn routed_install(
    stores: &crate::runtime::storage::OpenProductStores,
    request: &WorldInitRequest,
    theory_source: Option<&Path>,
    observed_seq: u64,
) -> Result<Option<RoutedTheoryInstall>, ApiError> {
    if request.stages.contains(&WorldInitStage::InstallTheory) {
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
    }
    if let Some(head) = stores
        .pds_packages
        .head(crate::docs::theory::DOCS_PACKAGE_ID)
        .map_err(|failure| world_init_error(failure.to_string()))?
    {
        let receipt = stores
            .pds_packages
            .resolve_receipt(&head.receipt_id)
            .map_err(|failure| world_init_error(failure.to_string()))?
            .ok_or_else(|| world_init_error("routed package head cites a missing receipt"))?;
        return Ok(Some(RoutedTheoryInstall {
            receipt,
            changed: false,
        }));
    }
    Ok(None)
}

pub(crate) fn compile_product_initialization<'a>(
    stores: &'a crate::runtime::storage::OpenProductStores,
    binding: &PhysicalBinding,
    package_receipt: &crate::theory::PdsPackageInstallationReceiptV1,
    observed_seq: u64,
) -> Result<CompleteProductInitialization<'a>, ApiError> {
    let declaration = crate::docs::theory::product_declaration(
        &binding.package.expression,
        &binding.package.principal_id,
        package_receipt,
        "docs-belief-family",
        &format!(
            "steward '{}' for subject '{}'",
            binding.package.expression, binding.subject
        ),
    )
    .map_err(|error| world_init_error(error.to_string()))?;
    let compilation = ProductCompilationReceiptV1::compile(
        &declaration,
        vec![package_receipt.clone()],
        observed_seq,
    )
    .map_err(|error| world_init_error(error.to_string()))?;
    let topology_id = product_topology_id(&declaration.agent_topology)
        .map_err(|error| world_init_error(error.to_string()))?;
    let subject = DomainObjectRef::new(SUBJECT_DOMAIN_ID, SUBJECT_OBJECT_KIND, &binding.subject)
        .map_err(|error| world_init_error(error.to_string()))?;
    let perspective = PerspectiveKey::new(SEED_PERSPECTIVE_KIND, SEED_PERSPECTIVE_ID)
        .map_err(|error| world_init_error(error.to_string()))?;
    let assignment = StewardshipAssignmentV1::new(
        compilation.compilation_receipt_id.clone(),
        declaration.product_revision_id.clone(),
        declaration.principal_id.clone(),
        subject,
        perspective.index_key(),
        BranchScope::main().branch_id,
        topology_id,
        vec![AssignedAgentPositionV1 {
            position_id: "steward".to_string(),
            agent_id: binding.agent_id.clone(),
        }],
        declaration.requested_authority_ref.clone(),
        declaration.principal_grant_ref.clone(),
    )?;
    let capability_inventory: ProductCapabilityInventory =
        crate::capability::product_capability_inventory()
            .map_err(|error| world_init_error(error.to_string()))?;
    let selected_contracts = executable_contract_refs(&complete_contracts_for_product(
        &capability_inventory,
        &compilation,
    )?);
    let selected_implementations = selected_contracts
        .iter()
        .map(|contract_ref| {
            capability_inventory
                .unique_implementation_ref(contract_ref)
                .map(|implementation_ref| (contract_ref.clone(), implementation_ref))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|error| world_init_error(error.to_string()))?;
    let capability_bindings = OwnerBindingView::new(BTreeMap::from([
        (
            "workspace".to_string(),
            binding.workspace_root.display().to_string(),
        ),
        ("subject".to_string(), binding.subject.clone()),
        ("agent".to_string(), binding.agent_id.clone()),
        ("provider".to_string(), binding.provider_id.clone()),
    ]));
    let activation = StewardshipActivationV1::new(
        assignment.assignment_id.clone(),
        BTreeMap::from([
            (
                "workspace".to_string(),
                PhysicalBindingRef::WorkspaceRef(binding.workspace_root.display().to_string()),
            ),
            (
                "subject".to_string(),
                PhysicalBindingRef::ConfigRef(binding.subject.clone()),
            ),
            (
                "agent".to_string(),
                PhysicalBindingRef::ConfigRef(binding.agent_id.clone()),
            ),
            (
                "provider".to_string(),
                PhysicalBindingRef::ProviderRef(binding.provider_id.clone()),
            ),
        ]),
        selected_implementations,
        AdapterPlacement::InProcess,
        RuntimeIsolationRequirements::default(),
        OperationalLimits::default(),
    )?;
    Ok(CompleteProductInitialization {
        product_store: stores.pds_products.as_ref(),
        maintained_conditions: stores.maintained_condition_registry.as_ref(),
        declaration,
        compilation,
        assignment,
        activation,
        capability_inventory,
        capability_bindings,
    })
}

fn complete_contracts_for_product(
    inventory: &ProductCapabilityInventory,
    compilation: &ProductCompilationReceiptV1,
) -> Result<Vec<meld_execution::capability::CapabilityContractRevision>, ApiError> {
    let selected = compilation
        .installed_owner_revisions
        .iter()
        .filter(|component| {
            component.route.owner_domain == "execution"
                && component.route.component_kind == "capability-contract"
        })
        .map(|component| component.owner_revision.content_hash.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let contracts = inventory
        .contracts()
        .filter(|revision| selected.contains(revision.content_identity.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if contracts.len() != selected.len() {
        return Err(world_init_error(
            "compiled product capability revisions are absent from the process inventory",
        ));
    }
    Ok(contracts)
}

fn executable_contract_refs(
    contracts: &[meld_execution::capability::CapabilityContractRevision],
) -> Vec<CapabilityContractRevisionRef> {
    contracts
        .iter()
        .map(|revision| revision.revision_ref())
        .collect()
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
                WorldInitStage::PrepareActivation,
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
