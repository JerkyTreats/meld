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

/// Narrow compiler boundary used by higher-level task network lowering.
///
/// The trait stays task-local. It accepts an authored task definition and a
/// capability catalog, then returns the compiled task graph record used by the
/// existing task executor.
///
/// # Example
///
/// ```rust
/// use meld_execution::capability::CapabilityCatalog;
/// use meld_execution::task::{
///     TaskCompiler, TaskDefinition, TaskDefinitionCompiler,
/// };
///
/// let compiler = TaskCompiler::new();
/// let catalog = CapabilityCatalog::new();
/// let definition = TaskDefinition {
///     task_id: "task-empty".to_string(),
///     task_version: 1,
///     init_slots: vec![],
///     capability_instances: vec![],
/// };
///
/// let compiled = compiler
///     .compile_task_definition(&definition, &catalog)
///     .expect("empty task definition is valid");
/// assert_eq!(compiled.task_id, "task-empty");
/// ```
pub trait TaskDefinitionCompiler {
    /// Compiles one task definition into a validated task graph record.
    fn compile_task_definition(
        &self,
        definition: &TaskDefinition,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskRecord, ApiError>;
}

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

impl TaskDefinitionCompiler for TaskCompiler {
    fn compile_task_definition(
        &self,
        definition: &TaskDefinition,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskRecord, ApiError> {
        self.compile(definition, catalog)
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

    fn target_selector_slot() -> TaskInitSlotSpec {
        TaskInitSlotSpec {
            init_slot_id: "target_selector".to_string(),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
            required: true,
        }
    }

    fn resolve_instance() -> BoundCapabilityInstance {
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
        }
    }

    fn simple_definition() -> TaskDefinition {
        TaskDefinition {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![target_selector_slot()],
            capability_instances: vec![resolve_instance()],
        }
    }

    fn assert_compile_error(definition: TaskDefinition, expected: &str) {
        let error = compile_task_definition(&definition, &catalog()).unwrap_err();

        assert!(
            error.to_string().contains(expected),
            "expected error containing '{expected}', got '{error}'"
        );
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
    fn compiler_rejects_task_definition_identity_violations() {
        let mut empty_task_id = simple_definition();
        empty_task_id.task_id.clear();
        assert_compile_error(empty_task_id, "task_id must not be empty");

        let mut zero_version = simple_definition();
        zero_version.task_version = 0;
        assert_compile_error(zero_version, "version must be greater than zero");

        let mut duplicate_init = simple_definition();
        duplicate_init.init_slots.push(target_selector_slot());
        assert_compile_error(duplicate_init, "duplicate init slot");

        let mut empty_init_artifact_type = simple_definition();
        empty_init_artifact_type.init_slots[0]
            .artifact_type_id
            .clear();
        assert_compile_error(
            empty_init_artifact_type,
            "must declare artifact type and schema version",
        );

        let mut zero_init_schema_version = simple_definition();
        zero_init_schema_version.init_slots[0].schema_version = 0;
        assert_compile_error(
            zero_init_schema_version,
            "must declare artifact type and schema version",
        );

        let mut duplicate_instance = simple_definition();
        duplicate_instance
            .capability_instances
            .push(resolve_instance());
        assert_compile_error(duplicate_instance, "duplicate capability instance");
    }

    #[test]
    fn compiler_rejects_unknown_or_mismatched_wiring() {
        let mut unknown_contract = simple_definition();
        unknown_contract.capability_instances[0].capability_type_id = "missing".to_string();
        assert_compile_error(unknown_contract, "references unknown capability");

        let mut unknown_producer = simple_definition();
        unknown_producer
            .capability_instances
            .push(BoundCapabilityInstance {
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
                        capability_instance_id: "missing_capability".to_string(),
                        output_slot_id: "resolved_node_ref".to_string(),
                        artifact_type_id: "resolved_node_ref".to_string(),
                        schema_version: 1,
                    }],
                }],
            });
        assert_compile_error(unknown_producer, "unknown producer capability");

        let mut mismatched_init = simple_definition();
        let BoundInputWiringSource::TaskInitSlot {
            artifact_type_id, ..
        } = &mut mismatched_init.capability_instances[0].input_wiring[0].sources[0]
        else {
            unreachable!("fixture uses task init wiring");
        };
        *artifact_type_id = "wrong_type".to_string();
        assert_compile_error(mismatched_init, "rejects artifact type");

        let mut mismatched_init_schema = simple_definition();
        mismatched_init_schema.init_slots[0].schema_version = 2;
        assert_compile_error(mismatched_init_schema, "does not match the wiring used by");
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

    #[test]
    fn compiler_orders_three_exclusive_effects_within_scope() {
        let mut definition = TaskDefinition {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: Vec::new(),
            capability_instances: Vec::new(),
        };
        for suffix in ["c", "a", "b"] {
            definition.init_slots.push(TaskInitSlotSpec {
                init_slot_id: format!("provider_result_{suffix}"),
                artifact_type_id: "provider_execute_result".to_string(),
                schema_version: 1,
                required: true,
            });
            definition
                .capability_instances
                .push(BoundCapabilityInstance {
                    capability_instance_id: format!("capinst_finalize_{suffix}"),
                    capability_type_id: "context_generate_finalize".to_string(),
                    capability_version: 1,
                    scope_ref: "node_a".to_string(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![],
                    input_wiring: vec![BoundInputWiring {
                        slot_id: "provider_result".to_string(),
                        sources: vec![BoundInputWiringSource::TaskInitSlot {
                            init_slot_id: format!("provider_result_{suffix}"),
                            artifact_type_id: "provider_execute_result".to_string(),
                            schema_version: 1,
                        }],
                    }],
                });
        }

        let compiled = compile_task_definition(&definition, &catalog()).unwrap();
        let effect_edges = compiled
            .dependency_edges
            .iter()
            .filter(|edge| edge.kind == TaskDependencyKind::Effect)
            .map(|edge| {
                (
                    edge.from_capability_instance_id.as_str(),
                    edge.to_capability_instance_id.as_str(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            effect_edges,
            vec![
                ("capinst_finalize_a", "capinst_finalize_b"),
                ("capinst_finalize_b", "capinst_finalize_c"),
            ]
        );
    }

    #[test]
    fn compiler_keeps_exclusive_effect_ordering_per_scope() {
        let mut definition = TaskDefinition {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: Vec::new(),
            capability_instances: Vec::new(),
        };
        for (suffix, scope_ref) in [("a", "node_a"), ("b", "node_b")] {
            definition.init_slots.push(TaskInitSlotSpec {
                init_slot_id: format!("provider_result_{suffix}"),
                artifact_type_id: "provider_execute_result".to_string(),
                schema_version: 1,
                required: true,
            });
            definition
                .capability_instances
                .push(BoundCapabilityInstance {
                    capability_instance_id: format!("capinst_finalize_{suffix}"),
                    capability_type_id: "context_generate_finalize".to_string(),
                    capability_version: 1,
                    scope_ref: scope_ref.to_string(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![],
                    input_wiring: vec![BoundInputWiring {
                        slot_id: "provider_result".to_string(),
                        sources: vec![BoundInputWiringSource::TaskInitSlot {
                            init_slot_id: format!("provider_result_{suffix}"),
                            artifact_type_id: "provider_execute_result".to_string(),
                            schema_version: 1,
                        }],
                    }],
                });
        }

        let compiled = compile_task_definition(&definition, &catalog()).unwrap();

        assert!(compiled
            .dependency_edges
            .iter()
            .all(|edge| edge.kind != TaskDependencyKind::Effect));
    }
}
