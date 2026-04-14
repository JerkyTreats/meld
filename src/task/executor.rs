//! Task-local execution agent for compiled tasks.

use crate::capability::{CapabilityExecutionContext, CapabilityInvocationPayload, UpstreamLineage};
use crate::error::ApiError;
use crate::task::contracts::{
    ArtifactProducerRef, ArtifactRecord, CapabilityInvocationRecord, CompiledTaskRecord,
};
use crate::task::events::{target_node_id_from_init_payload, TaskEvent};
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRecord};
use crate::task::init::{validate_task_initialization, TaskInitializationPayload};
use crate::task::invocation::assemble_invocation_payload;
use crate::task::readiness::compute_ready_capability_instances;
use crate::task::TaskArtifactRepo;
use std::collections::{BTreeSet, HashSet};

/// Single task-local execution agent for one live task instance.
#[derive(Debug, Clone)]
pub struct TaskExecutor {
    compiled_task: CompiledTaskRecord,
    init_payload: TaskInitializationPayload,
    artifact_repo: TaskArtifactRepo,
    invocation_records: Vec<CapabilityInvocationRecord>,
    events: Vec<TaskEvent>,
    expansion_records: Vec<TaskExpansionRecord>,
    applied_expansion_ids: HashSet<String>,
    completed_instances: HashSet<String>,
    in_flight_instances: HashSet<String>,
    started: bool,
}

impl TaskExecutor {
    fn build_task_event(
        task_id: impl Into<String>,
        task_run_id: impl Into<String>,
        target_node_id: Option<String>,
        event_type: &str,
    ) -> TaskEvent {
        let mut event = TaskEvent::new(event_type, task_id, task_run_id);
        event.target_node_id = target_node_id;
        event
    }

    fn new_task_event(&self, event_type: &str) -> TaskEvent {
        Self::build_task_event(
            self.compiled_task.task_id.clone(),
            self.init_payload.task_run_context.task_run_id.clone(),
            target_node_id_from_init_payload(&self.init_payload),
            event_type,
        )
    }

    /// Creates one live task executor and seeds init artifacts into the repo.
    pub fn new(
        compiled_task: CompiledTaskRecord,
        init_payload: TaskInitializationPayload,
        repo_id: impl Into<String>,
    ) -> Result<Self, ApiError> {
        validate_task_initialization(&compiled_task, &init_payload)?;

        let mut artifact_repo = TaskArtifactRepo::new(repo_id);
        for init_artifact in &init_payload.init_artifacts {
            artifact_repo.append_artifact(ArtifactRecord {
                artifact_id: format!(
                    "init::{}::{}",
                    init_payload.task_run_context.task_run_id, init_artifact.init_slot_id
                ),
                artifact_type_id: init_artifact.artifact_type_id.clone(),
                schema_version: init_artifact.schema_version,
                content: init_artifact.content.clone(),
                producer: ArtifactProducerRef {
                    task_id: compiled_task.task_id.clone(),
                    capability_instance_id: "__task_init__".to_string(),
                    invocation_id: None,
                    output_slot_id: Some(init_artifact.init_slot_id.clone()),
                },
            })?;
        }

        let mut requested = TaskEvent::new(
            "task_requested",
            compiled_task.task_id.clone(),
            init_payload.task_run_context.task_run_id.clone(),
        );
        requested.target_node_id = target_node_id_from_init_payload(&init_payload);
        let events = vec![requested];

        Ok(Self {
            compiled_task,
            init_payload,
            artifact_repo,
            invocation_records: Vec::new(),
            events,
            expansion_records: Vec::new(),
            applied_expansion_ids: HashSet::new(),
            completed_instances: HashSet::new(),
            in_flight_instances: HashSet::new(),
            started: false,
        })
    }

    /// Returns the current task artifact repo.
    pub fn artifact_repo(&self) -> &TaskArtifactRepo {
        &self.artifact_repo
    }

    /// Returns the compiled task backing this live executor.
    pub fn compiled_task(&self) -> &CompiledTaskRecord {
        &self.compiled_task
    }

    /// Returns the task initialization payload for this live executor.
    pub fn init_payload(&self) -> &TaskInitializationPayload {
        &self.init_payload
    }

    /// Returns the persisted invocation records.
    pub fn invocation_records(&self) -> &[CapabilityInvocationRecord] {
        &self.invocation_records
    }

    /// Returns emitted task events.
    pub fn events(&self) -> &[TaskEvent] {
        &self.events
    }

    /// Returns applied task expansion records.
    pub fn expansion_records(&self) -> &[TaskExpansionRecord] {
        &self.expansion_records
    }

    /// Returns the currently ready capability instances.
    pub fn ready_capability_instances(&self) -> Vec<String> {
        compute_ready_capability_instances(
            &self.compiled_task,
            &self.artifact_repo,
            &self.completed_instances,
            &self.in_flight_instances,
        )
    }

    /// Returns true when all compiled capability instances completed successfully.
    pub fn is_complete(&self) -> bool {
        self.completed_instances.len() == self.compiled_task.capability_instances.len()
    }

    /// Returns the current completed capability instance count.
    pub fn completed_count(&self) -> usize {
        self.completed_instances.len()
    }

    /// Releases all currently ready capability invocations.
    pub fn release_ready_invocations(
        &mut self,
        execution_context: CapabilityExecutionContext,
    ) -> Result<Vec<CapabilityInvocationPayload>, ApiError> {
        let ready = self.ready_capability_instances();
        if !self.started {
            self.started = true;
            self.events.push(self.new_task_event("task_started"));
        }

        if ready.is_empty() {
            let mut event = self.new_task_event("task_blocked");
            event.blocked_reason = Some("no_ready_capability_instances".to_string());
            event.ready_count = Some(0);
            event.running_count = Some(self.in_flight_instances.len());
            self.events.push(event);
            return Ok(Vec::new());
        }

        let mut payloads = Vec::new();
        for capability_instance_id in ready {
            let instance = self
                .compiled_task
                .capability_instances
                .iter()
                .find(|instance| instance.capability_instance_id == capability_instance_id)
                .expect("ready instance id must exist in compiled task");
            let attempt_index = self
                .invocation_records
                .iter()
                .filter(|record| record.capability_instance_id == capability_instance_id)
                .count() as u32
                + 1;
            let invocation_id = format!("{}::attempt::{}", capability_instance_id, attempt_index);
            let payload = assemble_invocation_payload(
                &self.compiled_task,
                &self.init_payload,
                &self.artifact_repo,
                instance,
                invocation_id.clone(),
                CapabilityExecutionContext {
                    attempt: attempt_index,
                    ..execution_context.clone()
                },
                Some(UpstreamLineage {
                    task_id: self.compiled_task.task_id.clone(),
                    task_run_id: self.init_payload.task_run_context.task_run_id.clone(),
                    capability_path: vec![capability_instance_id.clone()],
                    batch_index: None,
                    node_index: None,
                    repair_scope: None,
                }),
            )?;

            self.in_flight_instances
                .insert(capability_instance_id.clone());
            self.invocation_records.push(CapabilityInvocationRecord {
                invocation_id: invocation_id.clone(),
                capability_instance_id: capability_instance_id.clone(),
                supplied_inputs: payload
                    .supplied_inputs
                    .iter()
                    .filter_map(|input| match &input.value {
                        crate::capability::SuppliedValueRef::Artifact(artifact) => {
                            Some(ArtifactRecord {
                                artifact_id: artifact.artifact_id.clone(),
                                artifact_type_id: artifact.artifact_type_id.clone(),
                                schema_version: artifact.schema_version,
                                content: artifact.content.clone(),
                                producer: ArtifactProducerRef {
                                    task_id: self.compiled_task.task_id.clone(),
                                    capability_instance_id: "__payload_copy__".to_string(),
                                    invocation_id: None,
                                    output_slot_id: Some(input.slot_id.clone()),
                                },
                            })
                        }
                        crate::capability::SuppliedValueRef::StructuredValue(_) => None,
                    })
                    .collect(),
                emitted_artifacts: Vec::new(),
                failure_summary: None,
                attempt_index,
            });

            let mut event = self.new_task_event("task_progressed");
            event.capability_instance_id = Some(capability_instance_id.clone());
            event.invocation_id = Some(invocation_id);
            event.attempt_index = Some(attempt_index);
            event.ready_count = Some(self.ready_capability_instances().len());
            event.running_count = Some(self.in_flight_instances.len());
            self.events.push(event);

            payloads.push(payload);
        }

        Ok(payloads)
    }

    /// Records successful invocation completion and emitted artifacts.
    pub fn record_success(
        &mut self,
        invocation_id: &str,
        emitted_artifacts: Vec<ArtifactRecord>,
    ) -> Result<(), ApiError> {
        let task_id = self.compiled_task.task_id.clone();
        let task_run_id = self.init_payload.task_run_context.task_run_id.clone();
        let target_node_id = target_node_id_from_init_payload(&self.init_payload);
        let record = self
            .invocation_records
            .iter_mut()
            .find(|record| record.invocation_id == invocation_id)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Task executor does not know invocation '{}'",
                    invocation_id
                ))
            })?;

        for artifact in emitted_artifacts {
            record.emitted_artifacts.push(artifact.artifact_id.clone());
            self.artifact_repo.append_artifact(artifact.clone())?;

            let mut event = Self::build_task_event(
                task_id.clone(),
                task_run_id.clone(),
                target_node_id.clone(),
                "task_artifact_emitted",
            );
            event.capability_instance_id = Some(record.capability_instance_id.clone());
            event.invocation_id = Some(record.invocation_id.clone());
            event.artifact_id = Some(artifact.artifact_id.clone());
            event.artifact_type_id = Some(artifact.artifact_type_id.clone());
            event.attempt_index = Some(record.attempt_index);
            self.events.push(event);
        }

        self.in_flight_instances
            .remove(&record.capability_instance_id);
        self.completed_instances
            .insert(record.capability_instance_id.clone());

        let mut event = Self::build_task_event(
            task_id,
            task_run_id,
            target_node_id,
            "task_succeeded",
        );
        event.capability_instance_id = Some(record.capability_instance_id.clone());
        event.invocation_id = Some(record.invocation_id.clone());
        event.attempt_index = Some(record.attempt_index);
        self.events.push(event);

        Ok(())
    }

    /// Records failed invocation completion.
    pub fn record_failure(
        &mut self,
        invocation_id: &str,
        failure_summary: ArtifactRecord,
        error: impl Into<String>,
    ) -> Result<(), ApiError> {
        let task_id = self.compiled_task.task_id.clone();
        let task_run_id = self.init_payload.task_run_context.task_run_id.clone();
        let target_node_id = target_node_id_from_init_payload(&self.init_payload);
        let record = self
            .invocation_records
            .iter_mut()
            .find(|record| record.invocation_id == invocation_id)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Task executor does not know invocation '{}'",
                    invocation_id
                ))
            })?;
        record.failure_summary = Some(failure_summary.clone());
        self.artifact_repo.append_artifact(failure_summary)?;
        self.in_flight_instances
            .remove(&record.capability_instance_id);

        let mut event = Self::build_task_event(
            task_id,
            task_run_id,
            target_node_id,
            "task_failed",
        );
        event.capability_instance_id = Some(record.capability_instance_id.clone());
        event.invocation_id = Some(record.invocation_id.clone());
        event.attempt_index = Some(record.attempt_index);
        event.error = Some(error.into());
        self.events.push(event);
        Ok(())
    }

    /// Applies one append-only compiled task delta if the expansion id is new.
    pub fn apply_task_expansion(
        &mut self,
        expansion_id: &str,
        expansion_kind: &str,
        source_artifact_id: &str,
        delta: CompiledTaskDelta,
    ) -> Result<bool, ApiError> {
        if self.applied_expansion_ids.contains(expansion_id) {
            return Ok(false);
        }

        let existing_init_slots = self
            .compiled_task
            .init_slots
            .iter()
            .map(|slot| slot.init_slot_id.as_str())
            .collect::<HashSet<_>>();
        for slot in &delta.init_slots {
            if existing_init_slots.contains(slot.init_slot_id.as_str()) {
                return Err(ApiError::ConfigError(format!(
                    "Task '{}' already contains init slot '{}' from expansion '{}'",
                    self.compiled_task.task_id, slot.init_slot_id, expansion_id
                )));
            }
        }

        let existing_instances = self
            .compiled_task
            .capability_instances
            .iter()
            .map(|instance| instance.capability_instance_id.as_str())
            .collect::<HashSet<_>>();
        for instance in &delta.capability_instances {
            if existing_instances.contains(instance.capability_instance_id.as_str()) {
                return Err(ApiError::ConfigError(format!(
                    "Task '{}' already contains capability instance '{}' from expansion '{}'",
                    self.compiled_task.task_id, instance.capability_instance_id, expansion_id
                )));
            }
        }

        let existing_edges = self
            .compiled_task
            .dependency_edges
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        for edge in &delta.dependency_edges {
            if existing_edges.contains(edge) {
                return Err(ApiError::ConfigError(format!(
                    "Task '{}' already contains dependency edge '{} -> {}' from expansion '{}'",
                    self.compiled_task.task_id,
                    edge.from_capability_instance_id,
                    edge.to_capability_instance_id,
                    expansion_id
                )));
            }
        }

        for artifact in &delta.init_artifacts {
            if self
                .artifact_repo
                .get_artifact(&artifact.artifact_id)
                .is_some()
            {
                return Err(ApiError::ConfigError(format!(
                    "Task '{}' already contains init artifact '{}' from expansion '{}'",
                    self.compiled_task.task_id, artifact.artifact_id, expansion_id
                )));
            }
        }

        for slot in delta.init_slots {
            self.compiled_task.init_slots.push(slot);
        }
        for artifact in delta.init_artifacts {
            self.artifact_repo.append_artifact(artifact.clone())?;

            let mut event = self.new_task_event("task_artifact_emitted");
            event.capability_instance_id = Some("__task_init__".to_string());
            event.artifact_id = Some(artifact.artifact_id.clone());
            event.artifact_type_id = Some(artifact.artifact_type_id.clone());
            self.events.push(event);
        }
        for instance in delta.capability_instances {
            self.compiled_task.capability_instances.push(instance);
        }
        for edge in delta.dependency_edges {
            self.compiled_task.dependency_edges.push(edge);
        }

        self.applied_expansion_ids.insert(expansion_id.to_string());
        self.expansion_records.push(TaskExpansionRecord {
            expansion_id: expansion_id.to_string(),
            expansion_kind: expansion_kind.to_string(),
            source_artifact_id: source_artifact_id.to_string(),
        });

        let mut event = self.new_task_event("task_expansion_applied");
        event.artifact_id = Some(source_artifact_id.to_string());
        event.artifact_type_id = Some(expansion_kind.to_string());
        event.ready_count = Some(self.ready_capability_instances().len());
        event.running_count = Some(self.in_flight_instances.len());
        self.events.push(event);

        Ok(true)
    }
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
    use crate::task::{
        compile_task_definition, InitArtifactValue, TaskDefinition, TaskInitSlotSpec,
        TaskRunContext,
    };
    use serde_json::json;

    fn catalog() -> CapabilityCatalog {
        let mut catalog = CapabilityCatalog::new();
        catalog
            .register(CapabilityTypeContract {
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
            })
            .unwrap();
        catalog
            .register(CapabilityTypeContract {
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
            })
            .unwrap();
        catalog
    }

    fn compiled_task() -> CompiledTaskRecord {
        compile_task_definition(
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
                        scope_ref: "node_root".to_string(),
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
        .unwrap()
    }

    fn init_payload() -> TaskInitializationPayload {
        TaskInitializationPayload {
            task_id: "task_docs_writer".to_string(),
            compiled_task_ref: "compiled_task_docs_writer".to_string(),
            init_artifacts: vec![InitArtifactValue {
                init_slot_id: "target_selector".to_string(),
                artifact_type_id: "target_selector".to_string(),
                schema_version: 1,
                content: json!({ "path": "docs" }),
            }],
            task_run_context: TaskRunContext {
                task_run_id: "taskrun_1".to_string(),
                session_id: Some("session_1".to_string()),
                trigger: "workflow.execute".to_string(),
            },
        }
    }

    #[test]
    fn executor_releases_first_ready_invocation() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();

        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].capability_instance_id, "capinst_resolve");
        assert_eq!(executor.invocation_records().len(), 1);
    }

    #[test]
    fn executor_emits_blocked_event_when_nothing_is_ready() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let _ = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        let blocked = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        assert!(blocked.is_empty());
        assert!(executor
            .events()
            .iter()
            .any(|event| event.event_type == "task_blocked"));
    }
}
