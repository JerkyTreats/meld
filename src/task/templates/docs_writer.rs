//! Docs-writer task package lowering and task run preparation.

use crate::api::ContextApi;
use crate::capability::{
    BoundBindingValue, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog,
};
use crate::error::ApiError;
use crate::task::compiler::compile_task_definition;
use crate::task::contracts::{TaskDefinition, TaskInitSlotSpec};
use crate::task::expansion::{
    TaskExpansionTemplate, TraversalPrerequisiteExpansionTemplate, TraversalPrerequisiteTemplate,
    WorkflowRegionTemplate, WorkflowTurnTemplate, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID, TRAVERSAL_PREREQUISITE_EXPANSION_KIND,
};
use crate::task::init::{InitArtifactValue, TaskInitializationPayload, TaskRunContext};
use crate::task::package::{PreparedTaskRun, WorkflowPackageTriggerRequest};
use crate::types::NodeID;
use crate::workflow::profile::{WorkflowGate, WorkflowProfile};
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::workflow::resolver::resolve_prompt_template;
use crate::workspace::resolve_workspace_node_id;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

const FORCE_POSTURE_INIT_SLOT_ID: &str = "force_posture";
const TARGET_NODE_REF_INIT_SLOT_ID: &str = "target_node_ref";
const TASK_EXPANSION_TEMPLATE_INIT_SLOT_ID: &str = "task_expansion_template";
const TRAVERSAL_INSTANCE_ID: &str = "capinst_traversal";

/// Prepares the docs-writer task package for one trigger request.
pub fn prepare_docs_writer_task_run(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
) -> Result<PreparedTaskRun, ApiError> {
    if request.package_id != "docs_writer" {
        return Err(ApiError::ConfigError(format!(
            "Unsupported workflow package '{}'",
            request.package_id
        )));
    }
    if request.workflow_id != registered_profile.profile.workflow_id {
        return Err(ApiError::ConfigError(format!(
            "Workflow package request '{}' does not match profile '{}'",
            request.workflow_id, registered_profile.profile.workflow_id
        )));
    }

    let target_node_id = resolve_target_node_id(api, workspace_root, request)?;
    let target_node_record = api
        .node_store()
        .get(&target_node_id)
        .map_err(ApiError::from)?
        .ok_or(ApiError::NodeNotFound(target_node_id))?;
    let profile = &registered_profile.profile;
    let prompts = prompt_map(api, registered_profile, profile)?;
    let gates = gate_map(profile);
    let ordered_turns = profile.ordered_turns();
    let region_template = WorkflowRegionTemplate {
        workflow_id: profile.workflow_id.clone(),
        agent_id: request.agent_id.clone(),
        provider: request.provider.clone(),
        frame_type: request.frame_type.clone(),
        force: request.force,
        force_init_slot_id: FORCE_POSTURE_INIT_SLOT_ID.to_string(),
        node_ref_slot_template: "node_ref::{node_id_prefix}".to_string(),
        existing_output_slot_template: "existing_readme::{node_id_prefix}".to_string(),
        existing_output_artifact_type_id: "readme_final".to_string(),
        turns: ordered_turns
            .iter()
            .map(|turn| {
                Ok(WorkflowTurnTemplate {
                    turn_id: turn.turn_id.clone(),
                    prompt_text: prompts.get(&turn.turn_id).cloned().ok_or_else(|| {
                        ApiError::ConfigError(format!(
                            "Workflow '{}' missing prompt text for turn '{}'",
                            profile.workflow_id, turn.turn_id
                        ))
                    })?,
                    output_type: turn.output_type.clone(),
                    gate: gates.get(&turn.gate_id).cloned().ok_or_else(|| {
                        ApiError::ConfigError(format!(
                            "Workflow '{}' missing gate '{}'",
                            profile.workflow_id, turn.gate_id
                        ))
                    })?,
                })
            })
            .collect::<Result<Vec<_>, ApiError>>()?,
    };
    let first_turn = ordered_turns.first().ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Workflow '{}' must declare at least one turn",
            profile.workflow_id
        ))
    })?;
    let last_turn = ordered_turns.last().ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Workflow '{}' must declare at least one turn",
            profile.workflow_id
        ))
    })?;
    let expansion_template = TaskExpansionTemplate {
        expansion_kind: TRAVERSAL_PREREQUISITE_EXPANSION_KIND.to_string(),
        content: serde_json::to_value(TraversalPrerequisiteExpansionTemplate {
            repeated_region: region_template,
            prerequisite_template: TraversalPrerequisiteTemplate {
                producer_turn_id: last_turn.turn_id.clone(),
                producer_stage_id: "finalize".to_string(),
                producer_output_slot_id: "generation_output".to_string(),
                producer_artifact_type_id: last_turn.output_type.clone(),
                consumer_turn_id: first_turn.turn_id.clone(),
                consumer_stage_id: "prepare".to_string(),
                consumer_input_slot_id: "upstream_artifact".to_string(),
            },
        })
        .map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to encode docs-writer expansion template: {}",
                err
            ))
        })?,
    };
    let task_definition = build_initial_definition(profile);
    let compiled_task = compile_task_definition(&task_definition, catalog)?;
    let init_payload = build_init_payload(
        profile,
        request,
        target_node_id,
        &target_node_record.path.to_string_lossy(),
        expansion_template,
    )?;

    Ok(PreparedTaskRun {
        compiled_task,
        init_payload,
        target_node_id,
    })
}

fn resolve_target_node_id(
    api: &ContextApi,
    workspace_root: &Path,
    request: &WorkflowPackageTriggerRequest,
) -> Result<NodeID, ApiError> {
    match (request.node_id, request.path.as_deref()) {
        (Some(node_id), None) => Ok(node_id),
        (None, Some(path)) => {
            resolve_workspace_node_id(api, workspace_root, Some(path), None, false)
        }
        (Some(_), Some(_)) => Err(ApiError::ConfigError(
            "Workflow package trigger cannot accept both node_id and path".to_string(),
        )),
        (None, None) => Err(ApiError::ConfigError(
            "Workflow package trigger requires node_id or path".to_string(),
        )),
    }
}

fn gate_map(profile: &WorkflowProfile) -> HashMap<String, WorkflowGate> {
    profile
        .gates
        .iter()
        .cloned()
        .map(|gate| (gate.gate_id.clone(), gate))
        .collect()
}

fn prompt_map(
    api: &ContextApi,
    registered_profile: &RegisteredWorkflowProfile,
    profile: &WorkflowProfile,
) -> Result<HashMap<String, String>, ApiError> {
    let mut prompts = HashMap::new();
    for turn in profile.ordered_turns() {
        let prompt = resolve_prompt_template(
            api,
            registered_profile.source_path.as_deref(),
            &turn.prompt_ref,
        )?;
        prompts.insert(turn.turn_id.clone(), prompt);
    }
    Ok(prompts)
}

fn build_initial_definition(profile: &WorkflowProfile) -> TaskDefinition {
    TaskDefinition {
        task_id: task_id(profile),
        task_version: profile.version,
        init_slots: vec![
            TaskInitSlotSpec {
                init_slot_id: FORCE_POSTURE_INIT_SLOT_ID.to_string(),
                artifact_type_id: "force_posture".to_string(),
                schema_version: 1,
                required: true,
            },
            TaskInitSlotSpec {
                init_slot_id: TARGET_NODE_REF_INIT_SLOT_ID.to_string(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                required: true,
            },
            TaskInitSlotSpec {
                init_slot_id: TASK_EXPANSION_TEMPLATE_INIT_SLOT_ID.to_string(),
                artifact_type_id: TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID.to_string(),
                schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                required: true,
            },
        ],
        capability_instances: vec![BoundCapabilityInstance {
            capability_instance_id: TRAVERSAL_INSTANCE_ID.to_string(),
            capability_type_id: "merkle_traversal".to_string(),
            capability_version: 1,
            scope_ref: "target".to_string(),
            scope_kind: "node".to_string(),
            binding_values: vec![binding("strategy", json!("bottom_up"))],
            input_wiring: vec![
                BoundInputWiring {
                    slot_id: "resolved_node_ref".to_string(),
                    sources: vec![BoundInputWiringSource::TaskInitSlot {
                        init_slot_id: TARGET_NODE_REF_INIT_SLOT_ID.to_string(),
                        artifact_type_id: "resolved_node_ref".to_string(),
                        schema_version: 1,
                    }],
                },
                BoundInputWiring {
                    slot_id: "task_expansion_template".to_string(),
                    sources: vec![BoundInputWiringSource::TaskInitSlot {
                        init_slot_id: TASK_EXPANSION_TEMPLATE_INIT_SLOT_ID.to_string(),
                        artifact_type_id: TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID.to_string(),
                        schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                    }],
                },
            ],
        }],
    }
}

fn build_init_payload(
    profile: &WorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    target_node_id: NodeID,
    target_path: &str,
    expansion_template: TaskExpansionTemplate,
) -> Result<TaskInitializationPayload, ApiError> {
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
                init_slot_id: FORCE_POSTURE_INIT_SLOT_ID.to_string(),
                artifact_type_id: "force_posture".to_string(),
                schema_version: 1,
                content: json!({
                    "force": request.force,
                    "replay": false,
                }),
            },
            InitArtifactValue {
                init_slot_id: TARGET_NODE_REF_INIT_SLOT_ID.to_string(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                content: json!({
                    "node_id": hex::encode(target_node_id),
                    "path": target_path,
                }),
            },
            InitArtifactValue {
                init_slot_id: TASK_EXPANSION_TEMPLATE_INIT_SLOT_ID.to_string(),
                artifact_type_id: TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID.to_string(),
                schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                content: serde_json::to_value(expansion_template).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Failed to encode task expansion template artifact: {}",
                        err
                    ))
                })?,
            },
        ],
        task_run_context: TaskRunContext {
            task_run_id: format!(
                "taskrun::{}::{}",
                profile.workflow_id,
                &hex::encode(target_node_id)[..16]
            ),
            session_id: request.session_id.clone(),
            trigger: "workflow.docs_writer.run".to_string(),
        },
    })
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
