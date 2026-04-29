//! Task compilation from authored task definitions to validated graph records.

use crate::capability::{CapabilityCatalog, CapabilityTypeContract, EffectSpec};
use crate::error::ApiError;
use crate::task::contracts::{
    CompiledTaskRecord, TaskDefinition, TaskDependencyEdge, TaskDependencyKind,
};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Task compiler for structured task definitions.
#[derive(Debug, Clone, Default)]
pub struct TaskCompiler;

impl TaskCompiler {
    /// Creates a new task compiler.
    pub fn new() -> Self {
        Self
    }

    /// Compiles one task definition into a validated task graph record.
    pub fn compile(
        &self,
        definition: &TaskDefinition,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskRecord, ApiError> {
        compile_task_definition(definition, catalog)
    }
}

/// Compiles one task definition into a validated task graph record.
pub fn compile_task_definition(
    definition: &TaskDefinition,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskRecord, ApiError> {
    if definition.task_id.trim().is_empty() {
        return Err(ApiError::ConfigError(
            "Task definition task_id must not be empty".to_string(),
        ));
    }
    if definition.task_version == 0 {
        return Err(ApiError::ConfigError(
            "Task definition version must be greater than zero".to_string(),
        ));
    }

    let mut init_slot_ids = HashSet::new();
    for init_slot in &definition.init_slots {
        if init_slot.init_slot_id.trim().is_empty() {
            return Err(ApiError::ConfigError(
                "Task init slot id must not be empty".to_string(),
            ));
        }
        if !init_slot_ids.insert(init_slot.init_slot_id.as_str()) {
            return Err(ApiError::ConfigError(format!(
                "Task definition '{}' has duplicate init slot '{}'",
                definition.task_id, init_slot.init_slot_id
            )));
        }
        if init_slot.artifact_type_id.trim().is_empty() || init_slot.schema_version == 0 {
            return Err(ApiError::ConfigError(format!(
                "Task definition '{}' init slot '{}' must declare artifact type and schema version",
                definition.task_id, init_slot.init_slot_id
            )));
        }
    }

    let mut instance_ids = HashSet::new();
    let mut contracts: HashMap<&str, &CapabilityTypeContract> = HashMap::new();
    for instance in &definition.capability_instances {
        if !instance_ids.insert(instance.capability_instance_id.as_str()) {
            return Err(ApiError::ConfigError(format!(
                "Task definition '{}' has duplicate capability instance '{}'",
                definition.task_id, instance.capability_instance_id
            )));
        }
        let contract = catalog
            .get(&instance.capability_type_id, instance.capability_version)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Task definition '{}' references unknown capability '{}' version '{}'",
                    definition.task_id, instance.capability_type_id, instance.capability_version
                ))
            })?;
        instance.validate_against(contract)?;
        contracts.insert(instance.capability_instance_id.as_str(), contract);
    }

    validate_init_slot_sources(definition)?;

    let mut dependency_edges = BTreeSet::new();
    for instance in &definition.capability_instances {
        for wiring in &instance.input_wiring {
            for source in &wiring.sources {
                if let crate::capability::BoundInputWiringSource::UpstreamOutput {
                    capability_instance_id,
                    output_slot_id,
                    ..
                } = source
                {
                    if !instance_ids.contains(capability_instance_id.as_str()) {
                        return Err(ApiError::ConfigError(format!(
                            "Task definition '{}' wires unknown producer capability '{}'",
                            definition.task_id, capability_instance_id
                        )));
                    }
                    dependency_edges.insert(TaskDependencyEdge {
                        from_capability_instance_id: capability_instance_id.clone(),
                        to_capability_instance_id: instance.capability_instance_id.clone(),
                        kind: TaskDependencyKind::Artifact,
                        reason: format!(
                            "output '{}' satisfies input '{}'",
                            output_slot_id, wiring.slot_id
                        ),
                    });
                }
            }
        }
    }

    let mut exclusive_effects: HashMap<(String, String), Vec<&str>> = HashMap::new();
    for instance in &definition.capability_instances {
        let contract = contracts
            .get(instance.capability_instance_id.as_str())
            .expect("contract resolved above");
        for effect in exclusive_effect_specs(contract) {
            exclusive_effects
                .entry((instance.scope_ref.clone(), effect.target.clone()))
                .or_default()
                .push(instance.capability_instance_id.as_str());
        }
    }

    for ((_scope_ref, effect_target), mut capability_ids) in exclusive_effects {
        capability_ids.sort_unstable();
        for pair in capability_ids.windows(2) {
            dependency_edges.insert(TaskDependencyEdge {
                from_capability_instance_id: pair[0].to_string(),
                to_capability_instance_id: pair[1].to_string(),
                kind: TaskDependencyKind::Effect,
                reason: format!("exclusive effect target '{}'", effect_target),
            });
        }
    }

    Ok(CompiledTaskRecord {
        task_id: definition.task_id.clone(),
        task_version: definition.task_version,
        init_slots: definition.init_slots.clone(),
        capability_instances: definition.capability_instances.clone(),
        dependency_edges: dependency_edges.into_iter().collect(),
    })
}

fn validate_init_slot_sources(definition: &TaskDefinition) -> Result<(), ApiError> {
    for instance in &definition.capability_instances {
        for wiring in &instance.input_wiring {
            for source in &wiring.sources {
                if let crate::capability::BoundInputWiringSource::TaskInitSlot {
                    init_slot_id,
                    artifact_type_id,
                    schema_version,
                } = source
                {
                    let init_slot = definition
                        .init_slots
                        .iter()
                        .find(|slot| slot.init_slot_id == *init_slot_id)
                        .ok_or_else(|| {
                            ApiError::ConfigError(format!(
                                "Task definition '{}' is missing init slot '{}' used by '{}'",
                                definition.task_id, init_slot_id, instance.capability_instance_id
                            ))
                        })?;
                    if init_slot.artifact_type_id != *artifact_type_id
                        || init_slot.schema_version != *schema_version
                    {
                        return Err(ApiError::ConfigError(format!(
                            "Task definition '{}' init slot '{}' does not match the wiring used by '{}'",
                            definition.task_id, init_slot_id, instance.capability_instance_id
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

fn exclusive_effect_specs(contract: &CapabilityTypeContract) -> impl Iterator<Item = &EffectSpec> {
    contract
        .effect_contract
        .iter()
        .filter(|effect| effect.exclusive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{
        ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, BoundBindingValue,
        BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityCatalog,
        CapabilityTypeContract, EffectKind, EffectSpec, ExecutionClass, ExecutionContract,
        InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
    };
    use crate::task::contracts::TaskInitSlotSpec;
    use serde_json::json;

    fn resolve_contract() -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: "workspace_resolve_node_id".to_string(),
            capability_version: 1,
            owning_domain: "workspace".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "workspace".to_string(),
                scope_ref_kind: "workspace_root".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![InputSlotSpec {
                slot_id: "target_selector".to_string(),
                accepted_artifact_type_ids: vec!["target_selector".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "resolved_node_ref".to_string(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifact".to_string(),
                retry_class: "none".to_string(),
                cancellation_supported: false,
            },
        }
    }

    fn traversal_contract() -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: "merkle_traversal".to_string(),
            capability_version: 1,
            owning_domain: "merkle_traversal".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: true,
            },
            binding_contract: vec![BindingSpec {
                binding_id: "strategy".to_string(),
                value_kind: BindingValueKind::Literal,
                required: true,
                affects_deterministic_identity: true,
            }],
            input_contract: vec![InputSlotSpec {
                slot_id: "resolved_node_ref".to_string(),
                accepted_artifact_type_ids: vec!["resolved_node_ref".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "ordered_merkle_node_batches".to_string(),
                artifact_type_id: "ordered_merkle_node_batches".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![EffectSpec {
                effect_id: "read_tree".to_string(),
                kind: EffectKind::Read,
                target: "workspace_tree".to_string(),
                exclusive: false,
            }],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifact".to_string(),
                retry_class: "none".to_string(),
                cancellation_supported: false,
            },
        }
    }

    fn finalize_contract() -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: "context_generate_finalize".to_string(),
            capability_version: 1,
            owning_domain: "context".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![InputSlotSpec {
                slot_id: "provider_result".to_string(),
                accepted_artifact_type_ids: vec!["provider_execute_result".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "readme_summary".to_string(),
                artifact_type_id: "readme_summary".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![EffectSpec {
                effect_id: "write_head".to_string(),
                kind: EffectKind::Write,
                target: "frame_head".to_string(),
                exclusive: true,
            }],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifact".to_string(),
                retry_class: "context".to_string(),
                cancellation_supported: false,
            },
        }
    }

    fn catalog() -> CapabilityCatalog {
        let mut catalog = CapabilityCatalog::new();
        catalog.register(resolve_contract()).unwrap();
        catalog.register(traversal_contract()).unwrap();
        catalog.register(finalize_contract()).unwrap();
        catalog
    }

    #[test]
    fn compiler_derives_artifact_edges_from_input_wiring() {
        let compiled = compile_task_definition(
            &TaskDefinition {
                task_id: "task_docs_writer".to_string(),
                task_version: 1,
                init_slots: vec![TaskInitSlotSpec {
                    init_slot_id: "target_selector".to_string(),
                    artifact_type_id: "target_selector".to_string(),
                    schema_version: 1,
                    required: true,
                }],
                capability_instances: vec![
                    BoundCapabilityInstance {
                        capability_instance_id: "capinst_resolve".to_string(),
                        capability_type_id: "workspace_resolve_node_id".to_string(),
                        capability_version: 1,
                        scope_ref: "workspace".to_string(),
                        scope_kind: "workspace".to_string(),
                        binding_values: vec![],
                        input_wiring: vec![BoundInputWiring {
                            slot_id: "target_selector".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: "target_selector".to_string(),
                                artifact_type_id: "target_selector".to_string(),
                                schema_version: 1,
                            }],
                        }],
                    },
                    BoundCapabilityInstance {
                        capability_instance_id: "capinst_traversal".to_string(),
                        capability_type_id: "merkle_traversal".to_string(),
                        capability_version: 1,
                        scope_ref: "node_a".to_string(),
                        scope_kind: "node".to_string(),
                        binding_values: vec![BoundBindingValue {
                            binding_id: "strategy".to_string(),
                            value: json!("bottom_up"),
                        }],
                        input_wiring: vec![BoundInputWiring {
                            slot_id: "resolved_node_ref".to_string(),
                            sources: vec![BoundInputWiringSource::UpstreamOutput {
                                capability_instance_id: "capinst_resolve".to_string(),
                                output_slot_id: "resolved_node_ref".to_string(),
                                artifact_type_id: "resolved_node_ref".to_string(),
                                schema_version: 1,
                            }],
                        }],
                    },
                ],
            },
            &catalog(),
        )
        .unwrap();

        assert_eq!(compiled.dependency_edges.len(), 1);
        assert_eq!(
            compiled.dependency_edges[0].from_capability_instance_id,
            "capinst_resolve"
        );
        assert_eq!(
            compiled.dependency_edges[0].to_capability_instance_id,
            "capinst_traversal"
        );
        assert_eq!(
            compiled.dependency_edges[0].kind,
            TaskDependencyKind::Artifact
        );
    }

    #[test]
    fn compiler_rejects_missing_init_slot() {
        let error = compile_task_definition(
            &TaskDefinition {
                task_id: "task_docs_writer".to_string(),
                task_version: 1,
                init_slots: vec![],
                capability_instances: vec![BoundCapabilityInstance {
                    capability_instance_id: "capinst_resolve".to_string(),
                    capability_type_id: "workspace_resolve_node_id".to_string(),
                    capability_version: 1,
                    scope_ref: "workspace".to_string(),
                    scope_kind: "workspace".to_string(),
                    binding_values: vec![],
                    input_wiring: vec![BoundInputWiring {
                        slot_id: "target_selector".to_string(),
                        sources: vec![BoundInputWiringSource::TaskInitSlot {
                            init_slot_id: "target_selector".to_string(),
                            artifact_type_id: "target_selector".to_string(),
                            schema_version: 1,
                        }],
                    }],
                }],
            },
            &catalog(),
        )
        .unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("missing init slot"));
    }

    #[test]
    fn compiler_derives_effect_edges_for_exclusive_effects() {
        let compiled = compile_task_definition(
            &TaskDefinition {
                task_id: "task_docs_writer".to_string(),
                task_version: 1,
                init_slots: vec![
                    TaskInitSlotSpec {
                        init_slot_id: "provider_result_a".to_string(),
                        artifact_type_id: "provider_execute_result".to_string(),
                        schema_version: 1,
                        required: true,
                    },
                    TaskInitSlotSpec {
                        init_slot_id: "provider_result_b".to_string(),
                        artifact_type_id: "provider_execute_result".to_string(),
                        schema_version: 1,
                        required: true,
                    },
                ],
                capability_instances: vec![
                    BoundCapabilityInstance {
                        capability_instance_id: "capinst_finalize_a".to_string(),
                        capability_type_id: "context_generate_finalize".to_string(),
                        capability_version: 1,
                        scope_ref: "node_a".to_string(),
                        scope_kind: "node".to_string(),
                        binding_values: vec![],
                        input_wiring: vec![BoundInputWiring {
                            slot_id: "provider_result".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: "provider_result_a".to_string(),
                                artifact_type_id: "provider_execute_result".to_string(),
                                schema_version: 1,
                            }],
                        }],
                    },
                    BoundCapabilityInstance {
                        capability_instance_id: "capinst_finalize_b".to_string(),
                        capability_type_id: "context_generate_finalize".to_string(),
                        capability_version: 1,
                        scope_ref: "node_a".to_string(),
                        scope_kind: "node".to_string(),
                        binding_values: vec![],
                        input_wiring: vec![BoundInputWiring {
                            slot_id: "provider_result".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: "provider_result_b".to_string(),
                                artifact_type_id: "provider_execute_result".to_string(),
                                schema_version: 1,
                            }],
                        }],
                    },
                ],
            },
            &catalog(),
        )
        .unwrap();

        assert_eq!(compiled.dependency_edges.len(), 1);
        assert_eq!(
            compiled.dependency_edges[0].kind,
            TaskDependencyKind::Effect
        );
        assert_eq!(
            compiled.dependency_edges[0].from_capability_instance_id,
            "capinst_finalize_a"
        );
        assert_eq!(
            compiled.dependency_edges[0].to_capability_instance_id,
            "capinst_finalize_b"
        );
    }
}
