use std::collections::BTreeMap;
use std::fs;

use meld_execution::activation::{
    bind_builtin_execution_activation, bind_execution_activation, validate_execution_activation,
    BuiltInExecutionActivationRegistry, CapabilityContractRef, ExecutionActivationInput,
    ExecutionActivationSelection, ExecutionActivationValidationContext, ExecutionForcePolicy,
    ExecutionTargetKind, ExecutionTargetSelector, PublicationMapping, RequiredArtifactContract,
};
use meld_execution::planning::MethodSourceRef;
use meld_execution::task::package::{
    map_task_package_output_artifact, resolve_task_package_output_mapping, PackageExpansionSpec,
};
use meld_lang::{Condition, EdgeKind, Literal, Proposition, Term};
use tempfile::TempDir;

fn selection(target: String) -> ExecutionActivationSelection {
    ExecutionActivationSelection {
        activation_hash: "a".repeat(64),
        activation_id: "docs_freshness".to_string(),
        method_id: "refresh_docs_v1".to_string(),
        task_package_id: "docs_writer".to_string(),
        workflow_id: "docs_writer_thread_v1".to_string(),
        workspace_scan_step_id: "scan_workspace".to_string(),
        workspace_scan: CapabilityContractRef {
            capability_type_id: "workspace_scan".to_string(),
            capability_version: 1,
        },
        task_network_id: "network-docs".to_string(),
        required_artifact: RequiredArtifactContract {
            artifact_type_id: "docs_patch".to_string(),
            schema_version: 1,
        },
        provider_binding_ref: "docs-writer".to_string(),
        frame_type: "docs".to_string(),
        force_policy: ExecutionForcePolicy::ReuseExisting,
        target: ExecutionTargetSelector {
            kind: ExecutionTargetKind::Path,
            canonical_value: target,
        },
        publication: PublicationMapping {
            mapping_id: "publication.docs_freshness".to_string(),
            success_event_type: "execution.task.succeeded".to_string(),
            failure_event_type: "execution.task.failed".to_string(),
            content_source_kind: "content_written".to_string(),
        },
    }
}

fn input(target: String) -> ExecutionActivationInput {
    bind_builtin_execution_activation(selection(target), validation_context()).unwrap()
}

fn validation_context() -> ExecutionActivationValidationContext {
    ExecutionActivationValidationContext::from_provider_binding_refs([
        "docs-writer".to_string(),
        "review-provider".to_string(),
    ])
}

#[test]
fn real_builtin_assets_bind_and_validate_without_store_side_effects() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    let input = input(target);
    let receipt = validate_execution_activation(&input).unwrap();
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);

    assert_eq!(input.method_library.entries.len(), 1);
    assert!(input.method_library.invalid.is_empty());
    let method = &input.method_library.entries[0].method;
    assert_eq!(method.method_id, "refresh_docs_v1");
    assert!(method
        .composition
        .steps
        .iter()
        .any(|step| step.step_id == "scan_workspace"));
    assert_eq!(input.method_binding.package_step_id, "run_docs_writer");
    assert_eq!(input.task_package.package_id, "docs_writer");
    assert_eq!(input.task_package.workflow_id, "docs_writer_thread_v1");
    assert!(input.task_package.output_artifacts.iter().any(|mapping| {
        mapping.source_output_type == "readme_final"
            && mapping.artifact_type_id == "docs_patch"
            && mapping.schema_version == 1
    }));
    assert_eq!(receipt.method_binding_id, input.method_binding.binding_id);
    assert_eq!(receipt.workspace_scan_contract_digest.len(), 64);
    assert_eq!(receipt.execution_coordinates_digest.len(), 64);
}

#[test]
fn deterministic_receipt_replays_exactly_and_ignores_source_path() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();
    let first = input(target.clone());
    let mut second = input(target);
    second.method_library.entries[0].source_ref = MethodSourceRef::File {
        path: "/unrelated/source/refresh_docs_v1.json".to_string(),
    };

    let first_receipt = validate_execution_activation(&first).unwrap();
    let replay_receipt = validate_execution_activation(&first).unwrap();
    let alternate_source_receipt = validate_execution_activation(&second).unwrap();

    assert_eq!(first_receipt, replay_receipt);
    assert_eq!(first_receipt, alternate_source_receipt);
    assert_eq!(
        serde_json::to_vec(&first_receipt).unwrap(),
        serde_json::to_vec(&replay_receipt).unwrap()
    );
}

#[test]
fn divergent_selection_and_resolved_asset_identity_fail_closed() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    let mut method_drift = selection(target.clone());
    method_drift.method_id = "refresh_docs_v2".to_string();
    assert_eq!(
        bind_builtin_execution_activation(method_drift, validation_context())
            .unwrap_err()
            .field,
        "selection.method_id"
    );

    let mut package_drift = selection(target.clone());
    package_drift.task_package_id = "docs_writer_other".to_string();
    assert_eq!(
        bind_builtin_execution_activation(package_drift, validation_context())
            .unwrap_err()
            .field,
        "selection.task_package_id"
    );

    let mut scan_drift = selection(target.clone());
    scan_drift.workspace_scan_step_id = "decoy_scan".to_string();
    assert_eq!(
        bind_builtin_execution_activation(scan_drift, validation_context())
            .unwrap_err()
            .field,
        "selection.workspace_scan_step_id"
    );

    let selected = selection(target);
    let mut assets = BuiltInExecutionActivationRegistry::new()
        .resolve(&selected)
        .unwrap();
    assets.method_binding.package_step_id = "drifted_package_step".to_string();
    assert_eq!(
        bind_execution_activation(selected, assets, validation_context())
            .unwrap_err()
            .field,
        "assets.method_binding.package_step_id"
    );
}

#[test]
fn scan_artifact_and_publication_drift_fail_pure_validation() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    let mut scan = input(target.clone());
    scan.selection.workspace_scan.capability_version = 2;
    assert_eq!(
        validate_execution_activation(&scan).unwrap_err().field,
        "selection.workspace_scan.capability_version"
    );

    let mut artifact = input(target.clone());
    artifact.selection.required_artifact.artifact_type_id = "generic_patch".to_string();
    assert_eq!(
        validate_execution_activation(&artifact).unwrap_err().field,
        "selection.required_artifact.artifact_type_id"
    );

    let mut mapping = input(target.clone());
    mapping.task_package.output_artifacts[0].schema_version = 2;
    assert_eq!(
        validate_execution_activation(&mapping).unwrap_err().field,
        "task_package.output_artifacts"
    );

    let mut publication = input(target);
    publication.selection.publication.success_event_type = "execution.task.completed".to_string();
    assert_eq!(
        validate_execution_activation(&publication)
            .unwrap_err()
            .field,
        "selection.publication.success_event_type"
    );
}

#[test]
fn network_and_runtime_coordinates_are_validated_and_receipt_bound() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();
    let baseline = validate_execution_activation(&input(target.clone())).unwrap();

    let mut network = input(target.clone());
    network.selection.task_network_id = "network-docs-secondary".to_string();
    let network_receipt = validate_execution_activation(&network).unwrap();
    assert_ne!(baseline.input_hash, network_receipt.input_hash);
    assert_ne!(
        baseline.task_network_identity_digest,
        network_receipt.task_network_identity_digest
    );

    let mut invalid_network = input(target.clone());
    invalid_network.selection.task_network_id = "../network-docs".to_string();
    assert_eq!(
        validate_execution_activation(&invalid_network)
            .unwrap_err()
            .field,
        "selection.task_network_id"
    );

    let mut provider = input(target.clone());
    provider.selection.provider_binding_ref = "Docs Writer".to_string();
    assert_eq!(
        validate_execution_activation(&provider).unwrap_err().field,
        "selection.provider_binding_ref"
    );

    let mut relative_target = input(target.clone());
    relative_target.selection.target.canonical_value = "relative/path".to_string();
    assert_eq!(
        validate_execution_activation(&relative_target)
            .unwrap_err()
            .field,
        "selection.target.canonical_value"
    );

    let mut force = input(target);
    force.selection.force_policy = ExecutionForcePolicy::ReplaceExisting;
    let force_receipt = validate_execution_activation(&force).unwrap();
    assert_ne!(
        baseline.execution_coordinates_digest,
        force_receipt.execution_coordinates_digest
    );
    assert_ne!(baseline.input_hash, force_receipt.input_hash);
}

#[test]
fn disconnected_and_decoy_scan_steps_fail_closed() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    let mut disconnected = input(target.clone());
    disconnected.method_library.entries[0]
        .method
        .composition
        .edges
        .clear();
    assert_eq!(
        validate_execution_activation(&disconnected)
            .unwrap_err()
            .field,
        "selection.workspace_scan_step_id"
    );

    let mut decoy = input(target);
    let composition = &mut decoy.method_library.entries[0].method.composition;
    let mut decoy_step = composition.steps[1].clone();
    decoy_step.step_id = "scan_workspace".to_string();
    composition.steps[0].step_id = "actual_scan".to_string();
    composition.edges[0].from = "actual_scan".to_string();
    composition.steps.push(decoy_step);
    assert_eq!(
        validate_execution_activation(&decoy).unwrap_err().field,
        "selection.workspace_scan_step_id"
    );
}

#[test]
fn canonical_method_semantics_reject_trigger_effect_and_cost_drift() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    let mut trigger_drift = input(target.clone());
    trigger_drift.method_library.entries[0].method.trigger = Proposition::Holds {
        subject: Term::Variable("?node".to_string()),
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.8))),
    };
    assert_eq!(
        validate_execution_activation(&trigger_drift)
            .unwrap_err()
            .field,
        "method_library"
    );

    let mut effect_drift = input(target.clone());
    effect_drift.method_library.entries[0]
        .method
        .net_effects
        .clear();
    assert_eq!(
        validate_execution_activation(&effect_drift)
            .unwrap_err()
            .field,
        "method_library"
    );

    let mut cost_drift = input(target);
    cost_drift.method_library.entries[0].method.cost.time_ms += 1;
    assert_eq!(
        validate_execution_activation(&cost_drift)
            .unwrap_err()
            .field,
        "method_library"
    );
}

#[test]
fn canonical_package_semantics_reject_seed_and_expansion_drift() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    let mut seed_drift = input(target.clone());
    seed_drift.task_package.seed.artifacts[0].schema_version += 1;
    assert_eq!(
        validate_execution_activation(&seed_drift)
            .unwrap_err()
            .field,
        "task_package"
    );

    let mut expansion_drift = input(target);
    let PackageExpansionSpec::TraversalPrerequisite(expansion) =
        &mut expansion_drift.task_package.expansions[0];
    expansion.traversal_strategy = "directories_top_down".to_string();
    assert_eq!(
        validate_execution_activation(&expansion_drift)
            .unwrap_err()
            .field,
        "task_package"
    );
}

#[test]
fn valid_looking_unknown_provider_fails_binding_and_validation() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    let mut unknown_selection = selection(target.clone());
    unknown_selection.provider_binding_ref = "unknown-provider".to_string();
    assert_eq!(
        bind_builtin_execution_activation(unknown_selection, validation_context())
            .unwrap_err()
            .field,
        "selection.provider_binding_ref"
    );

    let mut unknown_bound_input = input(target);
    unknown_bound_input.selection.provider_binding_ref = "unknown-provider".to_string();
    assert_eq!(
        validate_execution_activation(&unknown_bound_input)
            .unwrap_err()
            .field,
        "selection.provider_binding_ref"
    );
}

#[test]
fn scan_to_writer_path_requires_workspace_snapshot_data_flow() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();

    let mut ordering = input(target.clone());
    ordering.method_library.entries[0].method.composition.edges[0].kind = EdgeKind::Ordering;
    let ordering_error = validate_execution_activation(&ordering).unwrap_err();
    assert_eq!(ordering_error.field, "selection.workspace_scan_step_id");
    assert!(ordering_error.message.contains("data-flow path"));

    let mut wrong_artifact = input(target);
    wrong_artifact.method_library.entries[0]
        .method
        .composition
        .edges[0]
        .kind = EdgeKind::DataFlow {
        artifact_type: Term::ArtifactType("unrelated_snapshot".to_string()),
    };
    let wrong_artifact_error = validate_execution_activation(&wrong_artifact).unwrap_err();
    assert_eq!(
        wrong_artifact_error.field,
        "selection.workspace_scan_step_id"
    );
    assert!(wrong_artifact_error.message.contains("data-flow path"));
}

#[test]
fn pure_output_mapper_resolves_docs_patch_and_rejects_missing_output() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().canonicalize().unwrap().display().to_string();
    let package = input(target).task_package;

    let mapping = resolve_task_package_output_mapping(&package, "docs_patch", 1).unwrap();
    assert_eq!(mapping.source_output_type, "readme_final");

    let completed_outputs = BTreeMap::from([(
        "readme_final".to_string(),
        "# Refreshed documentation".to_string(),
    )]);
    let artifact =
        map_task_package_output_artifact(&package, "docs_patch", 1, &completed_outputs).unwrap();
    assert_eq!(artifact.artifact_type_id, "docs_patch");
    assert_eq!(artifact.schema_version, 1);
    assert_eq!(artifact.source_output_type, "readme_final");
    assert_eq!(artifact.content, "# Refreshed documentation");

    let error =
        map_task_package_output_artifact(&package, "docs_patch", 1, &BTreeMap::new()).unwrap_err();
    assert_eq!(error.field, "completed_outputs");
}
