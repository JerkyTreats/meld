//! CLI adapter for the world initialization command surface.
//!
//! Owner: root init. Parses the stage selection, resolves stage 0 inputs
//! from the selected CLI configuration, physical bindings, and the native package,
//! guards that the addressed target workspace and the open product stores
//! belong to the configured selection, and delegates to the crate-private
//! world initialization pipeline. No runtime state is created under the target
//! workspace: every durable effect lands in the external product storage
//! root through domain commands and the canonical append.

use std::collections::BTreeMap;
use std::path::Path;

use meld_execution::capability::CapabilityContractRevisionRef;
use meld_world_model::belief::{BeliefFamilyRegistryStore, BranchScope};
use meld_world_model::PerspectiveKey;

use crate::capability::ProductCapabilityInventory;
use crate::config::{
    AdapterPlacement, AssignedAgentPositionV1, MerkleConfig, OperationalLimits, PhysicalBinding,
    RuntimeIsolationRequirements, StewardshipActivationV1, StewardshipAssignmentV1,
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

/// Provenance recorded by the initialization command surface.
const INIT_PROVENANCE: &str = "meld world init";

/// Run world initialization stages 2 through 5 for the addressed target.
///
/// Configuration is the same resolved value used to open the CLI runtime. The explicit
/// `target_path` selects a declared workspace when one is required.
/// The open product stores must belong to the selection's
/// storage root, so the command can never initialize a world other than
/// the one it addressed.
pub fn run_world_init(
    assembly: &ProductRuntimeAssembly,
    config: &MerkleConfig,
    target_path: &Path,
    stage_args: &[String],
    theory_source: Option<&Path>,
    session_id: &str,
) -> Result<WorldInitReport, ApiError> {
    let request = WorldInitRequest {
        stages: parse_stage_args(stage_args)?,
    };

    let binding = PhysicalBinding::resolve_for_target(config, target_path)?.ok_or_else(|| {
        ApiError::ConfigError(format!(
            "no stewardship declaration targets '{}'",
            target_path.display()
        ))
    })?;

    if let Some(workspace_root) = &binding.workspace_root {
        let target_root = target_path
            .canonicalize()
            .map_err(|error| world_init_error(error.to_string()))?;
        if &target_root != workspace_root {
            return Err(world_init_error(
                "world init target differs from the configured workspace",
            ));
        }
    }
    if assembly.product_root() != binding.storage_root {
        return Err(ApiError::ConfigError(format!(
            "open product storage root '{}' does not match the selection's storage root '{}'",
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

    let routed = routed_install(stores, &binding.package, &request, theory_source, observed_seq)?.ok_or_else(|| {
        world_init_error(
            "no installed package matches the selected Strategy; supply its package source to install-theory",
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
    selection: &crate::config::SelectedStewardshipPackage,
    request: &WorldInitRequest,
    theory_source: Option<&Path>,
    observed_seq: u64,
) -> Result<Option<RoutedTheoryInstall>, ApiError> {
    if request.stages.contains(&WorldInitStage::InstallTheory) {
        if let Some(source) = theory_source.filter(|root| root.join("pds-package.json").is_file()) {
            let prior = stores
                .pds_packages
                .heads()
                .map_err(|error| world_init_error(error.to_string()))?;
            let receipt = super::product::install_package(stores, source, None, observed_seq)
                .map_err(|error| world_init_error(error.to_string()))?;
            let changed = !prior
                .iter()
                .any(|head| head.receipt_id == receipt.receipt_id);
            return Ok(Some(RoutedTheoryInstall { receipt, changed }));
        }
    }
    let mut matches = Vec::new();
    for head in stores
        .pds_packages
        .heads()
        .map_err(|error| world_init_error(error.to_string()))?
    {
        let receipt = stores
            .pds_packages
            .resolve_receipt(&head.receipt_id)
            .map_err(|error| world_init_error(error.to_string()))?
            .ok_or_else(|| world_init_error("package head cites a missing receipt"))?;
        let closure = stores
            .pds_packages
            .resolve_closure(std::slice::from_ref(&receipt.receipt_id))
            .map_err(|error| world_init_error(error.to_string()))?;
        if closure
            .iter()
            .flat_map(|package| &package.components)
            .any(|component| {
                component.route.owner_domain == "world-model"
                    && component.route.component_kind == "strategy-theory"
                    && component.owner_revision.id == selection.strategy_theory_id
            })
        {
            matches.push(receipt);
        }
    }
    if matches.len() > 1 {
        return Err(world_init_error("multiple installed packages supply the selected Strategy; supply the exact package source"));
    }
    Ok(matches.pop().map(|receipt| RoutedTheoryInstall {
        receipt,
        changed: false,
    }))
}

pub(crate) fn compile_product_initialization<'a>(
    stores: &'a crate::runtime::storage::OpenProductStores,
    binding: &PhysicalBinding,
    package_receipt: &crate::theory::PdsPackageInstallationReceiptV1,
    observed_seq: u64,
) -> Result<CompleteProductInitialization<'a>, ApiError> {
    let resolved_package = crate::theory::PdsPackageResolver::new(
        super::routes::current_product_route_catalog(stores)
            .map_err(|error| world_init_error(error.to_string()))?,
        stores.pds_packages.as_ref().clone(),
    )
    .resolve(&package_receipt.receipt_id)
    .map_err(|error| world_init_error(error.to_string()))?;
    let declaration = super::product::product_declaration(
        &binding.package.expression,
        &binding.package.principal_id,
        &resolved_package,
        &stores.pds_products,
    )
    .map_err(|error| world_init_error(error.to_string()))?;
    let position = match declaration.agent_topology.as_slice() {
        [position] => position,
        _ => return Err(world_init_error("physical binding must supply one Agent for each declared position; this binding selects a single Agent")),
    };
    let compilation = ProductCompilationReceiptV1::compile(
        &declaration,
        vec![package_receipt.clone()],
        &stores.pds_packages,
        observed_seq,
    )
    .map_err(|error| world_init_error(error.to_string()))?;
    let topology_id = product_topology_id(&declaration.agent_topology)
        .map_err(|error| world_init_error(error.to_string()))?;
    let subject = binding.subject.clone();
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
            position_id: position.position_id.clone(),
            agent_id: binding.agent_id.clone(),
        }],
        declaration.requested_authority_ref.clone(),
        declaration.principal_grant_ref.clone(),
    )?;
    let capability_inventory: ProductCapabilityInventory =
        crate::capability::product_capability_inventory_with_owners(&stores.owners)
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
    let capability_bindings = crate::runtime::owners::preparation::prepare_owner_bindings(
        stores,
        binding,
        &compilation.installed_owner_revisions,
    )
    .map_err(|error| world_init_error(error.to_string()))?;
    let activation = StewardshipActivationV1::new(
        assignment.assignment_id.clone(),
        binding.activation_bindings(),
        selected_implementations,
        if stores.owners.is_empty() {
            AdapterPlacement::InProcess
        } else {
            AdapterPlacement::SerializedLocal
        },
        RuntimeIsolationRequirements::default(),
        OperationalLimits::default(),
    )?;
    Ok(CompleteProductInitialization {
        owners: &stores.owners,
        product_store: stores.pds_products.as_ref(),
        maintained_conditions: stores.maintained_condition_registry.as_ref(),
        curation_store: stores.curation_store.as_ref(),
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
    #[test]
    fn shared_initializer_selects_security_components_and_authority_from_the_package() {
        let root = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let stores = crate::runtime::storage::OpenProductStores::open(
            &crate::runtime::storage::ProductStorageLayout::from_root(root.path()),
        )
        .unwrap();
        let selected = crate::config::SelectedStewardshipPackage {
            expression: "dependency_security_fixture".into(),
            principal_id: "workspace-owner".into(),
            belief_family_id: "dependency_security_posture".into(),
            evidence_mapping_id: "dependency_security_outcome_mapping_v1".into(),
            curation_rule_id: "dependency_security_posture".into(),
            maintained_condition_id: "dependency_security_posture".into(),
            strategy_theory_id: "dependency_security_fixture".into(),
            authority_policy_id: "dependency_security_fixture_read_only".into(),
            claim_policy_id: String::new(),
        };
        let request = WorldInitRequest {
            stages: vec![WorldInitStage::InstallTheory],
        };
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/dependency_security");
        let routed = routed_install(&stores, &selected, &request, Some(&source), 1)
            .unwrap()
            .unwrap();
        assert_eq!(
            routed.receipt.package_id,
            meld_dependency_security_owner::dependency_security::theory::PACKAGE_ID
        );
        assert!(routed.changed);
        meld_docs_owner::docs::theory::install_package(
            &stores,
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness"),
            2,
        )
        .unwrap();
        let replay = routed_install(&stores, &selected, &request, None, 2)
            .unwrap()
            .unwrap();
        assert_eq!(replay.receipt.receipt_id, routed.receipt.receipt_id);
        assert!(!replay.changed);
        let binding = PhysicalBinding {
            bindings: Default::default(),
            workspace_root: Some(workspace.path().into()),
            subject: meld_events::DomainObjectRef::new("workspace_fs", "node", "dependency-graph")
                .unwrap(),
            agent_id: "security-steward".into(),
            provider_id: None,
            package: selected,
            storage_root: root.path().into(),
        };
        let product =
            compile_product_initialization(&stores, &binding, &replay.receipt, 2).unwrap();
        assert_eq!(
            product.declaration.agent_topology[0].observation_scope_component_id,
            "security-posture-belief"
        );
        assert_eq!(
            product.declaration.agent_topology[0].required_subscriptions[0]
                .source_contract_component_id,
            "security-posture-belief"
        );
        assert_eq!(
            product.assignment.requested_authority_ref,
            "dependency_security_fixture_read_only"
        );
        assert_eq!(product.activation.selected_implementations.len(), 4);
        assert!(product
            .declaration
            .participant_plan
            .participants
            .iter()
            .any(|participant| participant.participant_id
                == meld_world_model::agent::AGENT_RECONCILIATION_RUNTIME_ID));
    }
}
