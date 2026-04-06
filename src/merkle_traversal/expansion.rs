//! Merkle traversal-owned task expansion content and compilation.

use crate::api::ContextApi;
use crate::capability::{
    BoundBindingValue, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog,
};
use crate::error::ApiError;
use crate::provider::ProviderExecutionBinding;
use crate::task::compiler::compile_task_definition;
use crate::task::contracts::{
    ArtifactProducerRef, ArtifactRecord, CompiledTaskRecord, TaskDefinition, TaskInitSlotSpec,
};
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use crate::types::NodeID;
use crate::workflow::profile::WorkflowGate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Traversed node ref carried into expansion compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalExpansionNode {
    pub node_id: String,
    pub path: String,
}

/// Relation between two traversed nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalExpansionRelation {
    pub upstream_node_id: String,
    pub downstream_node_id: String,
}

/// Workflow-backed repeated region template authored by the package layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRegionTemplate {
    pub workflow_id: String,
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub force_init_slot_id: String,
    pub node_ref_slot_template: String,
    pub existing_output_slot_template: String,
    pub existing_output_artifact_type_id: String,
    pub turns: Vec<WorkflowTurnTemplate>,
}

/// One authored workflow turn inside a repeated region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTurnTemplate {
    pub turn_id: String,
    pub prompt_text: String,
    pub output_type: String,
    pub gate: WorkflowGate,
    pub persist_frame: bool,
    pub retry_limit: usize,
    pub validate_json: bool,
}

/// Cross-node prerequisite mapping applied over traversal relations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteTemplate {
    pub producer_turn_id: String,
    pub producer_stage_id: String,
    pub producer_output_slot_id: String,
    pub producer_artifact_type_id: String,
    pub consumer_turn_id: String,
    pub consumer_stage_id: String,
    pub consumer_input_slot_id: String,
}

/// Template content authored ahead of traversal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteExpansionTemplate {
    pub repeated_region: WorkflowRegionTemplate,
    pub prerequisite_template: TraversalPrerequisiteTemplate,
}

/// Fully resolved expansion content emitted after traversal succeeds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalPrerequisiteExpansionContent {
    pub traversal_strategy: String,
    pub node_batches: Vec<Vec<TraversalExpansionNode>>,
    pub relations: Vec<TraversalExpansionRelation>,
    pub repeated_region: WorkflowRegionTemplate,
    pub prerequisite_template: TraversalPrerequisiteTemplate,
}

/// Compiles a traversal prerequisite expansion into a task delta.
pub fn compile_traversal_prerequisite_expansion(
    api: &ContextApi,
    compiled_task: &CompiledTaskRecord,
    expansion: &TaskExpansionRequest,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ApiError> {
    let content: TraversalPrerequisiteExpansionContent =
        serde_json::from_value(expansion.content.clone()).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to decode traversal prerequisite expansion '{}': {}",
                expansion.expansion_id, err
            ))
        })?;

    if content.repeated_region.turns.is_empty() {
        return Err(ApiError::ConfigError(format!(
            "Traversal prerequisite expansion '{}' must declare at least one turn",
            expansion.expansion_id
        )));
    }

    let nodes_by_id = index_nodes(&content.node_batches)?;
    let existing_readmes = collect_existing_readmes(
        api,
        &nodes_by_id,
        &content.repeated_region.frame_type,
        content.repeated_region.force,
    )?;
    let active_batches = content
        .node_batches
        .iter()
        .map(|batch| {
            batch
                .iter()
                .filter(|node| !existing_readmes.contains_key(&node.node_id))
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|batch| !batch.is_empty())
        .collect::<Vec<_>>();
    let active_ids = active_batches
        .iter()
        .flat_map(|batch| batch.iter().map(|node| node.node_id.clone()))
        .collect::<HashSet<_>>();
    let upstream_by_downstream = relations_by_downstream(&content.relations)?;

    let mut init_slots = Vec::new();
    let mut init_artifacts = Vec::new();
    let mut existing_node_ids = existing_readmes.keys().cloned().collect::<Vec<_>>();
    existing_node_ids.sort_unstable();
    for node_id in existing_node_ids {
        let slot_id = instantiate_template(
            &content.repeated_region.existing_output_slot_template,
            &node_id,
        )?;
        init_slots.push(TaskInitSlotSpec {
            init_slot_id: slot_id.clone(),
            artifact_type_id: content
                .repeated_region
                .existing_output_artifact_type_id
                .clone(),
            schema_version: 1,
            required: true,
        });
        init_artifacts.push(expansion_init_artifact(
            compiled_task,
            &expansion.expansion_id,
            &slot_id,
            &content.repeated_region.existing_output_artifact_type_id,
            existing_readmes
                .get(&node_id)
                .expect("existing node id collected above")
                .clone(),
        ));
    }

    for batch in &active_batches {
        for node in batch {
            let slot_id = instantiate_template(
                &content.repeated_region.node_ref_slot_template,
                &node.node_id,
            )?;
            init_slots.push(TaskInitSlotSpec {
                init_slot_id: slot_id.clone(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                required: true,
            });
            init_artifacts.push(expansion_init_artifact(
                compiled_task,
                &expansion.expansion_id,
                &slot_id,
                "resolved_node_ref",
                json!({
                    "node_id": node.node_id,
                    "path": node.path,
                }),
            ));
        }
    }

    let mut capability_instances = Vec::new();
    for batch in &active_batches {
        for node in batch {
            let mut previous_finalize_instance_id: Option<String> = None;
            let mut previous_turn_output_type: Option<String> = None;
            for turn in &content.repeated_region.turns {
                let prepare_instance_id =
                    stage_instance_id(&node.node_id, &turn.turn_id, "prepare")?;
                let execute_instance_id =
                    stage_instance_id(&node.node_id, &turn.turn_id, "execute")?;
                let finalize_instance_id =
                    stage_instance_id(&node.node_id, &turn.turn_id, "finalize")?;
                let node_ref_slot_id = instantiate_template(
                    &content.repeated_region.node_ref_slot_template,
                    &node.node_id,
                )?;

                let mut upstream_sources = Vec::new();
                if let (Some(previous_finalize), Some(previous_output_type)) = (
                    previous_finalize_instance_id.as_ref(),
                    previous_turn_output_type.as_ref(),
                ) {
                    upstream_sources.push(BoundInputWiringSource::UpstreamOutput {
                        capability_instance_id: previous_finalize.clone(),
                        output_slot_id: "generation_output".to_string(),
                        artifact_type_id: previous_output_type.clone(),
                        schema_version: 1,
                    });
                } else if turn.turn_id == content.prerequisite_template.consumer_turn_id {
                    let mut related_nodes = upstream_by_downstream
                        .get(&node.node_id)
                        .cloned()
                        .unwrap_or_default();
                    related_nodes.sort_by(|left, right| {
                        let left_path = nodes_by_id
                            .get(left)
                            .map(|node| node.path.as_str())
                            .unwrap_or_default();
                        let right_path = nodes_by_id
                            .get(right)
                            .map(|node| node.path.as_str())
                            .unwrap_or_default();
                        left_path.cmp(right_path).then(left.cmp(right))
                    });
                    for related_node_id in related_nodes {
                        if active_ids.contains(&related_node_id) {
                            upstream_sources.push(BoundInputWiringSource::UpstreamOutput {
                                capability_instance_id: stage_instance_id(
                                    &related_node_id,
                                    &content.prerequisite_template.producer_turn_id,
                                    &content.prerequisite_template.producer_stage_id,
                                )?,
                                output_slot_id: content
                                    .prerequisite_template
                                    .producer_output_slot_id
                                    .clone(),
                                artifact_type_id: content
                                    .prerequisite_template
                                    .producer_artifact_type_id
                                    .clone(),
                                schema_version: 1,
                            });
                        } else if existing_readmes.contains_key(&related_node_id) {
                            upstream_sources.push(BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: instantiate_template(
                                    &content.repeated_region.existing_output_slot_template,
                                    &related_node_id,
                                )?,
                                artifact_type_id: content
                                    .repeated_region
                                    .existing_output_artifact_type_id
                                    .clone(),
                                schema_version: 1,
                            });
                        }
                    }
                }

                capability_instances.push(BoundCapabilityInstance {
                    capability_instance_id: prepare_instance_id.clone(),
                    capability_type_id: "context_generate_prepare".to_string(),
                    capability_version: 1,
                    scope_ref: node.node_id.clone(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![
                        binding("agent_id", json!(content.repeated_region.agent_id)),
                        binding(
                            "provider_binding",
                            serde_json::to_value(&content.repeated_region.provider).map_err(
                                |err| {
                                    ApiError::ConfigError(format!(
                                        "Failed to encode provider binding for expansion '{}': {}",
                                        expansion.expansion_id, err
                                    ))
                                },
                            )?,
                        ),
                        binding("frame_type", json!(content.repeated_region.frame_type)),
                        binding("prompt_text", json!(turn.prompt_text)),
                        binding("turn_id", json!(turn.turn_id)),
                        binding("workflow_id", json!(content.repeated_region.workflow_id)),
                        binding("output_type", json!(turn.output_type)),
                        binding(
                            "gate",
                            serde_json::to_value(&turn.gate).map_err(|err| {
                                ApiError::ConfigError(format!(
                                    "Failed to encode workflow gate '{}' for expansion '{}': {}",
                                    turn.gate.gate_id, expansion.expansion_id, err
                                ))
                            })?,
                        ),
                    ],
                    input_wiring: vec![
                        BoundInputWiring {
                            slot_id: "resolved_node_ref".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: node_ref_slot_id,
                                artifact_type_id: "resolved_node_ref".to_string(),
                                schema_version: 1,
                            }],
                        },
                        BoundInputWiring {
                            slot_id: "force_posture".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: content.repeated_region.force_init_slot_id.clone(),
                                artifact_type_id: "force_posture".to_string(),
                                schema_version: 1,
                            }],
                        },
                        BoundInputWiring {
                            slot_id: content.prerequisite_template.consumer_input_slot_id.clone(),
                            sources: upstream_sources,
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
                    scope_ref: node.node_id.clone(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![
                        binding("max_attempts", json!(turn.retry_limit)),
                        binding("validate_json", json!(turn.validate_json)),
                    ],
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
                    scope_ref: node.node_id.clone(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![
                        binding("persist_frame", json!(turn.persist_frame)),
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
                previous_turn_output_type = Some(turn.output_type.clone());
            }
        }
    }

    let mut full_init_slots = compiled_task.init_slots.clone();
    full_init_slots.extend(init_slots.clone());
    let mut full_instances = compiled_task.capability_instances.clone();
    full_instances.extend(capability_instances.clone());
    let compiled_definition = TaskDefinition {
        task_id: compiled_task.task_id.clone(),
        task_version: compiled_task.task_version,
        init_slots: full_init_slots,
        capability_instances: full_instances,
    };
    let compiled = compile_task_definition(&compiled_definition, catalog)?;
    let base_edges = compiled_task
        .dependency_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let dependency_edges = compiled
        .dependency_edges
        .into_iter()
        .filter(|edge| !base_edges.contains(edge))
        .collect::<Vec<_>>();

    Ok(CompiledTaskDelta {
        init_slots,
        init_artifacts,
        capability_instances,
        dependency_edges,
    })
}

fn collect_existing_readmes(
    api: &ContextApi,
    nodes_by_id: &BTreeMap<String, TraversalExpansionNode>,
    frame_type: &str,
    force: bool,
) -> Result<HashMap<String, Value>, ApiError> {
    let mut existing = HashMap::new();
    if force {
        return Ok(existing);
    }

    for node in nodes_by_id.values() {
        let node_id = decode_node_id(&node.node_id)?;
        let Some(frame_id) = api.get_head(&node_id, frame_type)? else {
            continue;
        };
        let frame = api
            .frame_storage()
            .get(&frame_id)
            .map_err(ApiError::from)?
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Missing frame '{}' for node '{}'",
                    hex::encode(frame_id),
                    node.node_id
                ))
            })?;
        existing.insert(
            node.node_id.clone(),
            json!({
                "content": String::from_utf8_lossy(&frame.content).to_string(),
                "format": "markdown",
            }),
        );
    }

    Ok(existing)
}

fn index_nodes(
    batches: &[Vec<TraversalExpansionNode>],
) -> Result<BTreeMap<String, TraversalExpansionNode>, ApiError> {
    let mut nodes = BTreeMap::new();
    for batch in batches {
        for node in batch {
            decode_node_id(&node.node_id)?;
            if nodes.insert(node.node_id.clone(), node.clone()).is_some() {
                return Err(ApiError::ConfigError(format!(
                    "Traversal expansion contains duplicate node '{}'",
                    node.node_id
                )));
            }
        }
    }
    Ok(nodes)
}

fn relations_by_downstream(
    relations: &[TraversalExpansionRelation],
) -> Result<HashMap<String, Vec<String>>, ApiError> {
    let mut by_downstream = HashMap::<String, Vec<String>>::new();
    for relation in relations {
        if relation.upstream_node_id == relation.downstream_node_id {
            return Err(ApiError::ConfigError(format!(
                "Traversal expansion relation must not be reflexive for node '{}'",
                relation.upstream_node_id
            )));
        }
        by_downstream
            .entry(relation.downstream_node_id.clone())
            .or_default()
            .push(relation.upstream_node_id.clone());
    }
    Ok(by_downstream)
}

fn expansion_init_artifact(
    compiled_task: &CompiledTaskRecord,
    expansion_id: &str,
    slot_id: &str,
    artifact_type_id: &str,
    content: Value,
) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("expansion::{}::init::{}", expansion_id, slot_id),
        artifact_type_id: artifact_type_id.to_string(),
        schema_version: 1,
        content,
        producer: ArtifactProducerRef {
            task_id: compiled_task.task_id.clone(),
            capability_instance_id: "__task_init__".to_string(),
            invocation_id: None,
            output_slot_id: Some(slot_id.to_string()),
        },
    }
}

fn stage_instance_id(node_id_hex: &str, turn_id: &str, stage: &str) -> Result<String, ApiError> {
    Ok(format!(
        "node::{}::turn::{}::{}",
        node_id_prefix(node_id_hex)?,
        turn_id,
        stage
    ))
}

fn instantiate_template(template: &str, node_id_hex: &str) -> Result<String, ApiError> {
    Ok(template
        .replace("{node_id_hex}", node_id_hex)
        .replace("{node_id_prefix}", &node_id_prefix(node_id_hex)?))
}

fn node_id_prefix(node_id_hex: &str) -> Result<String, ApiError> {
    if node_id_hex.len() < 16 {
        return Err(ApiError::ConfigError(format!(
            "Node id '{}' is too short for deterministic prefix generation",
            node_id_hex
        )));
    }
    Ok(node_id_hex[..16].to_string())
}

fn decode_node_id(value: &str) -> Result<NodeID, ApiError> {
    let bytes = hex::decode(value).map_err(|err| {
        ApiError::ConfigError(format!("Invalid node id hex '{}': {}", value, err))
    })?;
    if bytes.len() != 32 {
        return Err(ApiError::ConfigError(format!(
            "Invalid node id hex '{}' length '{}'",
            value,
            bytes.len()
        )));
    }
    let mut node_id = [0u8; 32];
    node_id.copy_from_slice(&bytes);
    Ok(node_id)
}

fn binding(binding_id: &str, value: Value) -> BoundBindingValue {
    BoundBindingValue {
        binding_id: binding_id.to_string(),
        value,
    }
}
