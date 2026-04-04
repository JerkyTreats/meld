//! Docs-writer task package lowering and task run preparation.

use crate::api::ContextApi;
use crate::capability::{
    BoundBindingValue, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog,
};
use crate::error::ApiError;
use crate::merkle_traversal::{traverse, TraversalStrategy};
use crate::store::NodeRecord;
use crate::task::compiler::compile_task_definition;
use crate::task::contracts::{TaskDefinition, TaskInitSlotSpec};
use crate::task::init::{InitArtifactValue, TaskInitializationPayload, TaskRunContext};
use crate::task::package::{PreparedTaskRun, WorkflowPackageTriggerRequest};
use crate::types::NodeID;
use crate::workflow::profile::{WorkflowGate, WorkflowProfile};
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::workflow::resolver::resolve_prompt_template;
use crate::workspace::resolve_workspace_node_id;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

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
    let traversal = traverse(api, target_node_id, TraversalStrategy::BottomUp)?;
    let node_records = collect_node_records(api, traversal.as_slice())?;
    let profile = &registered_profile.profile;
    let gates = gate_map(profile);
    let prompts = prompt_map(api, registered_profile, profile)?;
    let task_definition = build_docs_writer_definition(
        profile,
        request,
        traversal.as_slice(),
        &node_records,
        &gates,
        &prompts,
    )?;
    let compiled_task = compile_task_definition(&task_definition, catalog)?;
    let init_payload = build_init_payload(
        profile,
        request,
        traversal.as_slice(),
        &node_records,
        target_node_id,
    );

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
            resolve_workspace_node_id(api, &workspace_root.to_path_buf(), Some(path), None, false)
        }
        (Some(_), Some(_)) => Err(ApiError::ConfigError(
            "Workflow package trigger cannot accept both node_id and path".to_string(),
        )),
        (None, None) => Err(ApiError::ConfigError(
            "Workflow package trigger requires node_id or path".to_string(),
        )),
    }
}

fn collect_node_records(
    api: &ContextApi,
    batches: &[Vec<NodeID>],
) -> Result<HashMap<NodeID, NodeRecord>, ApiError> {
    let mut records = HashMap::new();
    for batch in batches {
        for node_id in batch {
            let record = api
                .node_store()
                .get(node_id)
                .map_err(ApiError::from)?
                .ok_or_else(|| ApiError::NodeNotFound(*node_id))?;
            records.insert(*node_id, record);
        }
    }
    Ok(records)
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

fn build_docs_writer_definition(
    profile: &WorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    batches: &[Vec<NodeID>],
    node_records: &HashMap<NodeID, NodeRecord>,
    gates: &HashMap<String, WorkflowGate>,
    prompts: &HashMap<String, String>,
) -> Result<TaskDefinition, ApiError> {
    let mut init_slots = vec![TaskInitSlotSpec {
        init_slot_id: "force_posture".to_string(),
        artifact_type_id: "force_posture".to_string(),
        schema_version: 1,
        required: true,
    }];
    let mut capability_instances = Vec::new();
    let ordered_turns = profile.ordered_turns();

    for batch in batches {
        for node_id in batch {
            let record = node_records
                .get(node_id)
                .expect("node record collected above");
            init_slots.push(TaskInitSlotSpec {
                init_slot_id: node_ref_slot_id(*node_id),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                required: true,
            });

            let mut previous_finalize_instance_id: Option<String> = None;
            for turn in &ordered_turns {
                let prepare_instance_id = stage_instance_id(*node_id, &turn.turn_id, "prepare");
                let execute_instance_id = stage_instance_id(*node_id, &turn.turn_id, "execute");
                let finalize_instance_id = stage_instance_id(*node_id, &turn.turn_id, "finalize");
                let gate = gates.get(&turn.gate_id).cloned().ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Workflow '{}' missing gate '{}'",
                        profile.workflow_id, turn.gate_id
                    ))
                })?;
                let prompt_text = prompts.get(&turn.turn_id).cloned().ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Workflow '{}' missing prompt text for turn '{}'",
                        profile.workflow_id, turn.turn_id
                    ))
                })?;
                let mut prepare_sources = vec![BoundInputWiringSource::TaskInitSlot {
                    init_slot_id: node_ref_slot_id(*node_id),
                    artifact_type_id: "resolved_node_ref".to_string(),
                    schema_version: 1,
                }];
                if let Some(previous_finalize) = previous_finalize_instance_id.as_ref() {
                    let upstream_type = previous_turn_output_type(profile, &turn.turn_id)?;
                    prepare_sources.push(BoundInputWiringSource::UpstreamOutput {
                        capability_instance_id: previous_finalize.clone(),
                        output_slot_id: "generation_output".to_string(),
                        artifact_type_id: upstream_type.to_string(),
                        schema_version: 1,
                    });
                } else {
                    for child in &record.children {
                        let child_finalize = stage_instance_id(*child, "style_refine", "finalize");
                        if node_records.contains_key(child) {
                            prepare_sources.push(BoundInputWiringSource::UpstreamOutput {
                                capability_instance_id: child_finalize,
                                output_slot_id: "generation_output".to_string(),
                                artifact_type_id: "readme_final".to_string(),
                                schema_version: 1,
                            });
                        }
                    }
                }

                capability_instances.push(BoundCapabilityInstance {
                    capability_instance_id: prepare_instance_id.clone(),
                    capability_type_id: "context_generate_prepare".to_string(),
                    capability_version: 1,
                    scope_ref: hex::encode(node_id),
                    scope_kind: "node".to_string(),
                    binding_values: vec![
                        binding("agent_id", json!(request.agent_id)),
                        binding(
                            "provider_binding",
                            serde_json::to_value(&request.provider).map_err(|err| {
                                ApiError::ConfigError(format!(
                                    "Failed to encode provider binding: {}",
                                    err
                                ))
                            })?,
                        ),
                        binding("frame_type", json!(request.frame_type)),
                        binding("prompt_text", json!(prompt_text)),
                        binding("turn_id", json!(turn.turn_id)),
                        binding("workflow_id", json!(profile.workflow_id)),
                        binding("output_type", json!(turn.output_type)),
                        binding(
                            "gate",
                            serde_json::to_value(&gate).map_err(|err| {
                                ApiError::ConfigError(format!(
                                    "Failed to encode workflow gate '{}': {}",
                                    gate.gate_id, err
                                ))
                            })?,
                        ),
                    ],
                    input_wiring: vec![
                        BoundInputWiring {
                            slot_id: "resolved_node_ref".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: node_ref_slot_id(*node_id),
                                artifact_type_id: "resolved_node_ref".to_string(),
                                schema_version: 1,
                            }],
                        },
                        BoundInputWiring {
                            slot_id: "force_posture".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: "force_posture".to_string(),
                                artifact_type_id: "force_posture".to_string(),
                                schema_version: 1,
                            }],
                        },
                        BoundInputWiring {
                            slot_id: "upstream_artifact".to_string(),
                            sources: prepare_sources
                                .into_iter()
                                .filter(|source| {
                                    !matches!(source, BoundInputWiringSource::TaskInitSlot { .. })
                                })
                                .collect(),
                        },
                    ]
                    .into_iter()
                    .filter(|wiring| !wiring.sources.is_empty())
                    .collect(),
                });

                capability_instances.push(BoundCapabilityInstance {
                    capability_instance_id: execute_instance_id.clone(),
                    capability_type_id: "provider_execute_chat".to_string(),
                    capability_version: 1,
                    scope_ref: hex::encode(node_id),
                    scope_kind: "node".to_string(),
                    binding_values: Vec::new(),
                    input_wiring: vec![BoundInputWiring {
                        slot_id: "provider_execute_request".to_string(),
                        sources: vec![BoundInputWiringSource::UpstreamOutput {
                            capability_instance_id: prepare_instance_id.clone(),
                            output_slot_id: "provider_execute_request".to_string(),
                            artifact_type_id: "provider_execute_request".to_string(),
                            schema_version: 1,
                        }],
                    }],
                });

                capability_instances.push(BoundCapabilityInstance {
                    capability_instance_id: finalize_instance_id.clone(),
                    capability_type_id: "context_generate_finalize".to_string(),
                    capability_version: 1,
                    scope_ref: hex::encode(node_id),
                    scope_kind: "node".to_string(),
                    binding_values: vec![
                        binding("persist_frame", json!(turn.turn_id == "style_refine")),
                        binding("output_type", json!(turn.output_type)),
                    ],
                    input_wiring: vec![
                        BoundInputWiring {
                            slot_id: "provider_execute_result".to_string(),
                            sources: vec![BoundInputWiringSource::UpstreamOutput {
                                capability_instance_id: execute_instance_id,
                                output_slot_id: "provider_execute_result".to_string(),
                                artifact_type_id: "provider_execute_result".to_string(),
                                schema_version: 1,
                            }],
                        },
                        BoundInputWiring {
                            slot_id: "preparation_summary".to_string(),
                            sources: vec![BoundInputWiringSource::UpstreamOutput {
                                capability_instance_id: prepare_instance_id,
                                output_slot_id: "preparation_summary".to_string(),
                                artifact_type_id: "preparation_summary".to_string(),
                                schema_version: 1,
                            }],
                        },
                    ],
                });

                previous_finalize_instance_id = Some(finalize_instance_id);
            }
        }
    }

    Ok(TaskDefinition {
        task_id: task_id(profile),
        task_version: profile.version,
        init_slots,
        capability_instances,
    })
}

fn build_init_payload(
    profile: &WorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    batches: &[Vec<NodeID>],
    node_records: &HashMap<NodeID, NodeRecord>,
    target_node_id: NodeID,
) -> TaskInitializationPayload {
    let mut init_artifacts = vec![InitArtifactValue {
        init_slot_id: "force_posture".to_string(),
        artifact_type_id: "force_posture".to_string(),
        schema_version: 1,
        content: json!({
            "force": request.force,
            "replay": false,
        }),
    }];

    for batch in batches {
        for node_id in batch {
            let record = node_records
                .get(node_id)
                .expect("node record collected above");
            init_artifacts.push(InitArtifactValue {
                init_slot_id: node_ref_slot_id(*node_id),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                content: json!({
                    "node_id": hex::encode(node_id),
                    "path": record.path.to_string_lossy(),
                }),
            });
        }
    }

    TaskInitializationPayload {
        task_id: task_id(profile),
        compiled_task_ref: format!(
            "{}::{}::{}",
            task_id(profile),
            profile.version,
            &hex::encode(target_node_id)[..16]
        ),
        init_artifacts,
        task_run_context: TaskRunContext {
            task_run_id: format!(
                "taskrun::{}::{}",
                profile.workflow_id,
                &hex::encode(target_node_id)[..16]
            ),
            session_id: request.session_id.clone(),
            trigger: "workflow.docs_writer.run".to_string(),
        },
    }
}

fn previous_turn_output_type(
    profile: &WorkflowProfile,
    current_turn_id: &str,
) -> Result<String, ApiError> {
    let ordered = profile.ordered_turns();
    let current_index = ordered
        .iter()
        .position(|turn| turn.turn_id == current_turn_id)
        .ok_or_else(|| {
            ApiError::ConfigError(format!(
                "Workflow '{}' is missing turn '{}'",
                profile.workflow_id, current_turn_id
            ))
        })?;
    if current_index == 0 {
        return Err(ApiError::ConfigError(format!(
            "Workflow '{}' turn '{}' does not have a previous output",
            profile.workflow_id, current_turn_id
        )));
    }
    Ok(ordered[current_index - 1].output_type.clone())
}

fn stage_instance_id(node_id: NodeID, turn_id: &str, stage: &str) -> String {
    format!(
        "node::{}::turn::{}::{}",
        &hex::encode(node_id)[..16],
        turn_id,
        stage
    )
}

fn node_ref_slot_id(node_id: NodeID) -> String {
    format!("node_ref::{}", &hex::encode(node_id)[..16])
}

fn task_id(profile: &WorkflowProfile) -> String {
    format!("task::{}", profile.workflow_id)
}

fn binding(binding_id: &str, value: serde_json::Value) -> BoundBindingValue {
    BoundBindingValue {
        binding_id: binding_id.to_string(),
        value,
    }
}
