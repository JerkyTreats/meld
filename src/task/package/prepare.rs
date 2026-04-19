//! Shared preparation helpers for workflow-backed task packages.

use crate::api::ContextApi;
use crate::capability::{
    BoundBindingValue, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog,
};
use crate::error::ApiError;
use crate::task::compiler::compile_task_definition;
use crate::task::contracts::{TaskDefinition, TaskInitSlotSpec};
use crate::task::expansion::{
    TaskExpansionTemplate, TASK_EXPANSION_SCHEMA_VERSION, TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID,
};
use crate::task::init::{InitArtifactValue, TaskInitializationPayload, TaskRunContext};
use crate::task::package::{
    PackageExpansionSpec, PreparedTaskRun, PreparedWorkflowPackageContext, SeedSourceSpec,
    TargetSelectorKind, TaskPackageSpec, TraversalPrerequisitePackageExpansionSpec,
    WorkflowPackageTriggerRequest,
};
use crate::types::NodeID;
use crate::workflow::profile::{WorkflowGate, WorkflowProfile};
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::workflow::resolver::resolve_prompt_template;
use crate::workspace::resolve_workspace_node_id;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

const TARGET_NODE_REF_ARTIFACT_TYPE_ID: &str = "resolved_node_ref";
const TRAVERSAL_INSTANCE_ID: &str = "capinst_traversal";

/// Returns the deterministic task run id used by workflow-backed task packages.
pub fn workflow_task_run_id(workflow_id: &str, target_node_id: NodeID) -> String {
    format!(
        "taskrun::{}::{}",
        workflow_id,
        &hex::encode(target_node_id)[..16]
    )
}

/// Resolves a workflow package request into shared package context.
pub fn prepare_workflow_package_context(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    package_spec: &TaskPackageSpec,
) -> Result<PreparedWorkflowPackageContext, ApiError> {
    validate_workflow_package_trigger(package_spec, registered_profile, request)?;

    let target_node_id = resolve_package_target_node_id(api, workspace_root, request)?;
    let target_node_record = api
        .node_store()
        .get(&target_node_id)
        .map_err(ApiError::from)?
        .ok_or(ApiError::NodeNotFound(target_node_id))?;
    let traversal_expansion = find_traversal_prerequisite_expansion(package_spec)?.clone();
    let prompts_by_turn_id = prompt_map(
        api,
        registered_profile,
        &traversal_expansion.repeated_region.turns,
    )?;
    let gates_by_id = gate_map(&registered_profile.profile);

    Ok(PreparedWorkflowPackageContext {
        target_node_id,
        target_path: target_node_record.path.to_string_lossy().into_owned(),
        prompts_by_turn_id,
        gates_by_id,
        traversal_expansion,
    })
}

/// Prepares one workflow-backed task run from a package spec and package-specific expansion lowering.
pub fn prepare_workflow_task_run<F>(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
    package_spec: &TaskPackageSpec,
    build_expansion_template: F,
) -> Result<PreparedTaskRun, ApiError>
where
    F: FnOnce(&PreparedWorkflowPackageContext) -> Result<TaskExpansionTemplate, ApiError>,
{
    let context = prepare_workflow_package_context(
        api,
        workspace_root,
        registered_profile,
        request,
        package_spec,
    )?;
    let expansion_template = build_expansion_template(&context)?;
    let task_definition = build_initial_task_definition(&registered_profile.profile, package_spec);
    let compiled_task = compile_task_definition(&task_definition, catalog)?;
    let init_payload = build_task_initialization_payload(
        &registered_profile.profile,
        package_spec,
        request,
        context.target_node_id,
        &context.target_path,
        expansion_template,
    )?;

    Ok(PreparedTaskRun {
        compiled_task,
        init_payload,
        target_node_id: context.target_node_id,
    })
}

/// Validates that a workflow package trigger request satisfies the authored trigger contract.
pub fn validate_workflow_package_trigger(
    package_spec: &TaskPackageSpec,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
) -> Result<(), ApiError> {
    if request.package_id != package_spec.package_id {
        return Err(ApiError::ConfigError(format!(
            "Unsupported workflow package '{}'",
            request.package_id
        )));
    }
    if registered_profile.profile.workflow_id != package_spec.workflow_id {
        return Err(ApiError::ConfigError(format!(
            "Workflow package '{}' is authored for workflow '{}' but profile is '{}'",
            package_spec.package_id,
            package_spec.workflow_id,
            registered_profile.profile.workflow_id
        )));
    }
    if request.workflow_id != registered_profile.profile.workflow_id {
        return Err(ApiError::ConfigError(format!(
            "Workflow package request '{}' does not match profile '{}'",
            request.workflow_id, registered_profile.profile.workflow_id
        )));
    }

    match [request.node_id.is_some(), request.path.is_some()] {
        [true, true] => {
            return Err(ApiError::ConfigError(
                "Workflow package trigger cannot accept both node_id and path".to_string(),
            ));
        }
        [false, false] => {
            return Err(ApiError::ConfigError(
                "Workflow package trigger requires node_id or path".to_string(),
            ));
        }
        [true, false] => {
            if !package_spec
                .trigger
                .accepted_targets
                .contains(&TargetSelectorKind::NodeId)
            {
                return Err(ApiError::ConfigError(format!(
                    "Package '{}' does not accept node_id triggers",
                    package_spec.package_id
                )));
            }
        }
        [false, true] => {
            if !package_spec
                .trigger
                .accepted_targets
                .contains(&TargetSelectorKind::Path)
            {
                return Err(ApiError::ConfigError(format!(
                    "Package '{}' does not accept path triggers",
                    package_spec.package_id
                )));
            }
        }
    }

    for field in &package_spec.trigger.required_runtime_fields {
        match field.as_str() {
            "agent_id" if request.agent_id.trim().is_empty() => {
                return Err(missing_runtime_field(package_spec, field));
            }
            "provider_binding" if request.provider.provider_name.trim().is_empty() => {
                return Err(missing_runtime_field(package_spec, field));
            }
            "frame_type" if request.frame_type.trim().is_empty() => {
                return Err(missing_runtime_field(package_spec, field));
            }
            "force" => {}
            "agent_id" | "provider_binding" | "frame_type" => {}
            other => {
                return Err(ApiError::ConfigError(format!(
                    "Package '{}' declares unsupported runtime field '{}'",
                    package_spec.package_id, other
                )));
            }
        }
    }

    Ok(())
}

/// Finds the authored traversal prerequisite expansion on a package.
pub fn find_traversal_prerequisite_expansion(
    package_spec: &TaskPackageSpec,
) -> Result<&TraversalPrerequisitePackageExpansionSpec, ApiError> {
    package_spec
        .expansions
        .iter()
        .map(|expansion| match expansion {
            PackageExpansionSpec::TraversalPrerequisite(spec) => spec,
        })
        .next()
        .ok_or_else(|| {
            ApiError::ConfigError(format!(
                "Package '{}' is missing traversal prerequisite expansion",
                package_spec.package_id
            ))
        })
}

/// Resolves the trigger target node for one package request.
pub fn resolve_package_target_node_id(
    api: &ContextApi,
    workspace_root: &Path,
    request: &WorkflowPackageTriggerRequest,
) -> Result<NodeID, ApiError> {
    match request.node_id {
        Some(node_id) => Ok(node_id),
        None => {
            resolve_workspace_node_id(api, workspace_root, request.path.as_deref(), None, false)
        }
    }
}

/// Resolves workflow prompt text for the authored package turns.
pub fn prompt_map(
    api: &ContextApi,
    registered_profile: &RegisteredWorkflowProfile,
    turns: &[crate::task::package::TurnSpec],
) -> Result<HashMap<String, String>, ApiError> {
    let mut prompts = HashMap::new();
    for turn in turns {
        let prompt = resolve_prompt_template(
            api,
            registered_profile.source_path.as_deref(),
            &turn.prompt_ref,
        )?;
        prompts.insert(turn.turn_id.clone(), prompt);
    }
    Ok(prompts)
}

/// Indexes workflow gates by gate id.
pub fn gate_map(profile: &WorkflowProfile) -> HashMap<String, WorkflowGate> {
    profile
        .gates
        .iter()
        .cloned()
        .map(|gate| (gate.gate_id.clone(), gate))
        .collect()
}

/// Builds the initial traversal-seeding task definition from package authroing data.
pub fn build_initial_task_definition(
    profile: &WorkflowProfile,
    package_spec: &TaskPackageSpec,
) -> TaskDefinition {
    let traversal_strategy = find_traversal_prerequisite_expansion(package_spec)
        .map(|spec| spec.traversal_strategy.clone())
        .unwrap_or_else(|_| "bottom_up".to_string());

    TaskDefinition {
        task_id: task_id(profile),
        task_version: profile.version,
        init_slots: package_spec
            .seed
            .artifacts
            .iter()
            .map(|artifact| TaskInitSlotSpec {
                init_slot_id: artifact.init_slot_id.clone(),
                artifact_type_id: artifact.artifact_type_id.clone(),
                schema_version: artifact.schema_version,
                required: true,
            })
            .collect(),
        capability_instances: vec![BoundCapabilityInstance {
            capability_instance_id: TRAVERSAL_INSTANCE_ID.to_string(),
            capability_type_id: "merkle_traversal".to_string(),
            capability_version: 1,
            scope_ref: "target".to_string(),
            scope_kind: "node".to_string(),
            binding_values: vec![binding("strategy", json!(traversal_strategy))],
            input_wiring: vec![
                BoundInputWiring {
                    slot_id: TARGET_NODE_REF_ARTIFACT_TYPE_ID.to_string(),
                    sources: vec![BoundInputWiringSource::TaskInitSlot {
                        init_slot_id: find_seed_artifact(package_spec, |source| {
                            matches!(source, SeedSourceSpec::TargetNodeRef)
                        })
                        .map(|artifact| artifact.init_slot_id.clone())
                        .unwrap_or_else(|| "target_node_ref".to_string()),
                        artifact_type_id: TARGET_NODE_REF_ARTIFACT_TYPE_ID.to_string(),
                        schema_version: 1,
                    }],
                },
                BoundInputWiring {
                    slot_id: "task_expansion_template".to_string(),
                    sources: vec![BoundInputWiringSource::TaskInitSlot {
                        init_slot_id: find_seed_artifact(package_spec, |source| {
                            matches!(source, SeedSourceSpec::ExpansionTemplate { .. })
                        })
                        .map(|artifact| artifact.init_slot_id.clone())
                        .unwrap_or_else(|| "task_expansion_template".to_string()),
                        artifact_type_id: TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID.to_string(),
                        schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                    }],
                },
            ],
        }],
    }
}

/// Builds the initialization payload from package seed contracts and one expansion template.
pub fn build_task_initialization_payload(
    profile: &WorkflowProfile,
    package_spec: &TaskPackageSpec,
    request: &WorkflowPackageTriggerRequest,
    target_node_id: NodeID,
    target_path: &str,
    expansion_template: TaskExpansionTemplate,
) -> Result<TaskInitializationPayload, ApiError> {
    let force_posture_spec = find_seed_artifact(package_spec, |source| {
        matches!(source, SeedSourceSpec::ForcePosture)
    })
    .ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Package '{}' is missing force posture seed",
            package_spec.package_id
        ))
    })?;
    let target_node_spec = find_seed_artifact(package_spec, |source| {
        matches!(source, SeedSourceSpec::TargetNodeRef)
    })
    .ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Package '{}' is missing target node seed",
            package_spec.package_id
        ))
    })?;
    let expansion_init_spec = find_seed_artifact(package_spec, |source| {
        matches!(source, SeedSourceSpec::ExpansionTemplate { .. })
    })
    .ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Package '{}' is missing expansion template seed",
            package_spec.package_id
        ))
    })?;

    Ok(TaskInitializationPayload {
        task_id: task_id(profile),
        compiled_task_ref: format!(
            "{}::{}::{}",
            task_id(profile),
            profile.version,
            &hex::encode(target_node_id)[..16]
        ),
        init_artifacts: vec![
            InitArtifactValue {
                init_slot_id: force_posture_spec.init_slot_id.clone(),
                artifact_type_id: force_posture_spec.artifact_type_id.clone(),
                schema_version: force_posture_spec.schema_version,
                content: json!({
                    "force": request.force,
                    "replay": false,
                }),
            },
            InitArtifactValue {
                init_slot_id: target_node_spec.init_slot_id.clone(),
                artifact_type_id: target_node_spec.artifact_type_id.clone(),
                schema_version: target_node_spec.schema_version,
                content: json!({
                    "node_id": hex::encode(target_node_id),
                    "path": target_path,
                }),
            },
            InitArtifactValue {
                init_slot_id: expansion_init_spec.init_slot_id.clone(),
                artifact_type_id: expansion_init_spec.artifact_type_id.clone(),
                schema_version: expansion_init_spec.schema_version,
                content: serde_json::to_value(expansion_template).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Failed to encode task expansion template artifact: {}",
                        err
                    ))
                })?,
            },
        ],
        task_run_context: TaskRunContext {
            task_run_id: workflow_task_run_id(&profile.workflow_id, target_node_id),
            session_id: request.session_id.clone(),
            trigger: "workflow.docs_writer.run".to_string(),
        },
    })
}

fn find_seed_artifact(
    package_spec: &TaskPackageSpec,
    predicate: impl Fn(&SeedSourceSpec) -> bool,
) -> Option<&crate::task::package::SeedArtifactSpec> {
    package_spec
        .seed
        .artifacts
        .iter()
        .find(|artifact| predicate(&artifact.source))
}

fn missing_runtime_field(package_spec: &TaskPackageSpec, field: &str) -> ApiError {
    ApiError::ConfigError(format!(
        "Package '{}' requires runtime field '{}'",
        package_spec.package_id, field
    ))
}

fn binding(binding_id: &str, value: Value) -> BoundBindingValue {
    BoundBindingValue {
        binding_id: binding_id.to_string(),
        value,
    }
}

fn task_id(profile: &WorkflowProfile) -> String {
    format!("task::{}", profile.workflow_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
    use crate::task::package::{
        InitialSeedSpec, PrerequisiteTemplateSpec, RepeatedRegionSpec, SeedArtifactSpec,
        StageChainSpec, StageSpec, TaskTriggerSpec, TurnOutputPolicySpec, TurnSpec,
    };
    use crate::workflow::profile::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowProfile, WorkflowThreadPolicy,
    };
    use crate::workflow::registry::RegisteredWorkflowProfile;
    use serde_json::json;

    fn package_spec() -> TaskPackageSpec {
        TaskPackageSpec {
            package_id: "docs_writer".to_string(),
            workflow_id: "docs_writer_thread_v1".to_string(),
            trigger: TaskTriggerSpec {
                accepted_targets: vec![TargetSelectorKind::Path],
                required_runtime_fields: vec![
                    "agent_id".to_string(),
                    "provider_binding".to_string(),
                    "frame_type".to_string(),
                    "force".to_string(),
                ],
            },
            seed: InitialSeedSpec {
                artifacts: vec![
                    SeedArtifactSpec {
                        init_slot_id: "force_posture".to_string(),
                        artifact_type_id: "force_posture".to_string(),
                        schema_version: 1,
                        source: SeedSourceSpec::ForcePosture,
                    },
                    SeedArtifactSpec {
                        init_slot_id: "target_node_ref".to_string(),
                        artifact_type_id: "resolved_node_ref".to_string(),
                        schema_version: 1,
                        source: SeedSourceSpec::TargetNodeRef,
                    },
                    SeedArtifactSpec {
                        init_slot_id: "task_expansion_template".to_string(),
                        artifact_type_id: TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID.to_string(),
                        schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                        source: SeedSourceSpec::ExpansionTemplate {
                            template_id: "bottom_up".to_string(),
                        },
                    },
                ],
            },
            expansions: vec![PackageExpansionSpec::TraversalPrerequisite(
                TraversalPrerequisitePackageExpansionSpec {
                    expansion_kind: "traversal_prerequisite_expansion".to_string(),
                    template_ref: "bottom_up".to_string(),
                    traversal_strategy: "bottom_up".to_string(),
                    publish: None,
                    repeated_region: RepeatedRegionSpec {
                        region_id: "docs_writer_node".to_string(),
                        force_init_slot_id: "force_posture".to_string(),
                        node_ref_slot_template: "node_ref::{node_id_prefix}".to_string(),
                        existing_output_slot_template: "existing_readme::{node_id_prefix}"
                            .to_string(),
                        existing_output_artifact_type_id: "frame_ref".to_string(),
                        stage_chain: StageChainSpec {
                            stages: vec![
                                StageSpec {
                                    stage_id: "prepare".to_string(),
                                    capability_type_id: "context_generate_prepare".to_string(),
                                    capability_version: 1,
                                },
                                StageSpec {
                                    stage_id: "execute".to_string(),
                                    capability_type_id: "provider_execute_chat".to_string(),
                                    capability_version: 1,
                                },
                                StageSpec {
                                    stage_id: "finalize".to_string(),
                                    capability_type_id: "context_generate_finalize".to_string(),
                                    capability_version: 1,
                                },
                            ],
                        },
                        turns: vec![TurnSpec {
                            turn_id: "style_refine".to_string(),
                            prompt_ref: "prompts/docs_writer/style_refine.md".to_string(),
                            output_type: "readme_final".to_string(),
                            gate_id: "style_gate".to_string(),
                            output_policy: TurnOutputPolicySpec {
                                persist_frame: true,
                            },
                            retry_limit: 1,
                            validate_json: false,
                        }],
                    },
                    prerequisite: PrerequisiteTemplateSpec {
                        producer_turn_id: "style_refine".to_string(),
                        producer_stage_id: "finalize".to_string(),
                        producer_output_slot_id: "generation_output".to_string(),
                        producer_artifact_type_id: "frame_ref".to_string(),
                        consumer_turn_id: "style_refine".to_string(),
                        consumer_stage_id: "prepare".to_string(),
                        consumer_input_slot_id: "upstream_artifact".to_string(),
                    },
                },
            )],
        }
    }

    fn registered_profile() -> RegisteredWorkflowProfile {
        RegisteredWorkflowProfile {
            profile: WorkflowProfile {
                workflow_id: "docs_writer_thread_v1".to_string(),
                version: 1,
                title: "Docs Writer".to_string(),
                description: "Writes docs".to_string(),
                thread_policy: WorkflowThreadPolicy {
                    start_conditions: json!({}),
                    dedupe_key_fields: Vec::new(),
                    max_turn_retries: 1,
                },
                turns: Vec::new(),
                gates: Vec::new(),
                artifact_policy: WorkflowArtifactPolicy {
                    store_output: true,
                    store_prompt_render: true,
                    store_context_payload: true,
                    max_output_bytes: 1024,
                },
                failure_policy: WorkflowFailurePolicy {
                    mode: "fail_fast".to_string(),
                    resume_from_failed_turn: false,
                    stop_on_gate_fail: true,
                },
                thread_profile: None,
                target_agent_id: None,
                target_frame_type: None,
                final_artifact_type: None,
            },
            source_path: None,
        }
    }

    fn request() -> WorkflowPackageTriggerRequest {
        WorkflowPackageTriggerRequest {
            package_id: "docs_writer".to_string(),
            workflow_id: "docs_writer_thread_v1".to_string(),
            node_id: None,
            path: Some("src".into()),
            agent_id: "docs-writer".to_string(),
            provider: ProviderExecutionBinding {
                provider_name: "test-provider".to_string(),
                runtime_overrides: ProviderRuntimeOverrides::default(),
            },
            frame_type: "context-docs-writer".to_string(),
            force: true,
            session_id: Some("session-1".to_string()),
        }
    }

    #[test]
    fn validate_trigger_rejects_unsupported_target_kind() {
        let mut request = request();
        request.node_id = Some([7u8; 32]);
        request.path = None;

        let error =
            validate_workflow_package_trigger(&package_spec(), &registered_profile(), &request)
                .unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error
            .to_string()
            .contains("does not accept node_id triggers"));
    }

    #[test]
    fn build_initial_task_definition_uses_seed_contracts() {
        let definition =
            build_initial_task_definition(&registered_profile().profile, &package_spec());

        assert_eq!(definition.task_id, "task::docs_writer_thread_v1");
        assert_eq!(definition.init_slots.len(), 3);
        assert_eq!(definition.capability_instances.len(), 1);
        let traversal = &definition.capability_instances[0];
        assert_eq!(traversal.capability_type_id, "merkle_traversal");
        assert_eq!(traversal.input_wiring.len(), 2);
        assert_eq!(
            traversal.input_wiring[0].sources[0],
            BoundInputWiringSource::TaskInitSlot {
                init_slot_id: "target_node_ref".to_string(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
            }
        );
    }

    #[test]
    fn build_initialization_payload_uses_seed_specs() {
        let payload = build_task_initialization_payload(
            &registered_profile().profile,
            &package_spec(),
            &request(),
            [9u8; 32],
            "src",
            TaskExpansionTemplate {
                expansion_kind: "traversal_prerequisite_expansion".to_string(),
                content: json!({ "template": "value" }),
            },
        )
        .unwrap();

        assert_eq!(payload.task_id, "task::docs_writer_thread_v1");
        assert_eq!(payload.init_artifacts.len(), 3);
        assert_eq!(payload.init_artifacts[0].init_slot_id, "force_posture");
        assert_eq!(payload.init_artifacts[1].init_slot_id, "target_node_ref");
        assert_eq!(
            payload.init_artifacts[2].artifact_type_id,
            TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID
        );
    }
}
