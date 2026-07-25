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
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Durable progress snapshot for one live task executor.
///
/// The task domain owns this record. It carries the expanded compiled task,
/// the run initialization payload, invocation records, expansion records, and
/// completed instance ids intact so a reopened executor resumes exactly where
/// the last quiesced step stopped. Artifact truth is not duplicated here: it
/// stays in the durable task artifact repository the snapshot is reopened
/// with, and readiness is recomputed from that repository plus this record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExecutorSnapshot {
    /// Compiled task graph including every applied expansion delta.
    pub compiled_task: CompiledTaskRecord,
    /// Initialization payload that created the task run.
    pub init_payload: TaskInitializationPayload,
    /// Capability invocation records accumulated by the run.
    pub invocation_records: Vec<CapabilityInvocationRecord>,
    /// Expansion records applied by the run, in application order.
    pub expansion_records: Vec<TaskExpansionRecord>,
    /// Capability instance ids completed durably, in sorted order.
    pub completed_instance_ids: Vec<String>,
    /// True when the run already emitted its started transition.
    pub started: bool,
}

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
        Self::new_with_artifact_repo(compiled_task, init_payload, TaskArtifactRepo::new(repo_id))
    }

    /// Creates one live task executor with a caller supplied artifact repository.
    ///
    /// A reopened durable repo may already contain identical init artifacts. The
    /// constructor treats those as replay and rejects drift under the same id.
    pub fn new_with_artifact_repo(
        compiled_task: CompiledTaskRecord,
        init_payload: TaskInitializationPayload,
        mut artifact_repo: TaskArtifactRepo,
    ) -> Result<Self, ApiError> {
        validate_task_initialization(&compiled_task, &init_payload)?;

        for init_artifact in &init_payload.init_artifacts {
            let artifact = ArtifactRecord {
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
            };
            if let Some(existing) = artifact_repo.get_artifact(&artifact.artifact_id) {
                // Durable replay is allowed only when the seed record remains
                // contract equivalent across process restarts.
                if existing == &artifact {
                    continue;
                }
                return Err(ApiError::ConfigError(format!(
                    "Task executor init artifact '{}' already exists with different content",
                    artifact.artifact_id
                )));
            }
            artifact_repo.append_artifact(artifact)?;
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

    /// Captures the durable progress snapshot at a quiesced step boundary.
    ///
    /// Snapshots are only meaningful when no invocation is in flight, because
    /// released-but-unresolved work has no durable resume semantics. Callers
    /// must resolve every released invocation before persisting progress.
    pub fn snapshot(&self) -> Result<TaskExecutorSnapshot, ApiError> {
        if !self.in_flight_instances.is_empty() {
            return Err(ApiError::ConfigError(format!(
                "Task executor for '{}' cannot snapshot with {} invocations in flight",
                self.compiled_task.task_id,
                self.in_flight_instances.len()
            )));
        }

        let mut completed_instance_ids =
            self.completed_instances.iter().cloned().collect::<Vec<_>>();
        completed_instance_ids.sort();

        Ok(TaskExecutorSnapshot {
            compiled_task: self.compiled_task.clone(),
            init_payload: self.init_payload.clone(),
            invocation_records: self.invocation_records.clone(),
            expansion_records: self.expansion_records.clone(),
            completed_instance_ids,
            started: self.started,
        })
    }

    /// Reopens a live task executor from a durable snapshot and its artifact repo.
    ///
    /// The repo must be the same durable repository the snapshot progressed
    /// against: init artifacts are not re-seeded and every emitted artifact
    /// referenced by an invocation record must already exist. Resumed runs do
    /// not repeat the requested or started transitions.
    pub fn from_snapshot(
        snapshot: TaskExecutorSnapshot,
        artifact_repo: TaskArtifactRepo,
    ) -> Result<Self, ApiError> {
        if snapshot.init_payload.task_id != snapshot.compiled_task.task_id {
            return Err(ApiError::ConfigError(format!(
                "Task executor snapshot init payload targets '{}' but compiled task is '{}'",
                snapshot.init_payload.task_id, snapshot.compiled_task.task_id
            )));
        }

        let instance_ids = snapshot
            .compiled_task
            .capability_instances
            .iter()
            .map(|instance| instance.capability_instance_id.as_str())
            .collect::<HashSet<_>>();
        for completed_id in &snapshot.completed_instance_ids {
            if !instance_ids.contains(completed_id.as_str()) {
                return Err(ApiError::ConfigError(format!(
                    "Task executor snapshot completed unknown capability instance '{}'",
                    completed_id
                )));
            }
        }
        for record in &snapshot.invocation_records {
            if !instance_ids.contains(record.capability_instance_id.as_str()) {
                return Err(ApiError::ConfigError(format!(
                    "Task executor snapshot invocation '{}' names unknown capability instance '{}'",
                    record.invocation_id, record.capability_instance_id
                )));
            }
            for artifact_id in &record.emitted_artifacts {
                // Progress and artifact stores must describe the same run;
                // a missing emitted artifact means the stores diverged.
                if artifact_repo.get_artifact(artifact_id).is_none() {
                    return Err(ApiError::ConfigError(format!(
                        "Task executor snapshot invocation '{}' references artifact '{}' missing from repo '{}'",
                        record.invocation_id,
                        artifact_id,
                        artifact_repo.record().repo_id
                    )));
                }
            }
        }

        let mut applied_expansion_ids = HashSet::new();
        for record in &snapshot.expansion_records {
            if !applied_expansion_ids.insert(record.expansion_id.clone()) {
                return Err(ApiError::ConfigError(format!(
                    "Task executor snapshot contains duplicate expansion '{}'",
                    record.expansion_id
                )));
            }
        }

        Ok(Self {
            compiled_task: snapshot.compiled_task,
            init_payload: snapshot.init_payload,
            artifact_repo,
            invocation_records: snapshot.invocation_records,
            events: Vec::new(),
            expansion_records: snapshot.expansion_records,
            applied_expansion_ids,
            completed_instances: snapshot.completed_instance_ids.into_iter().collect(),
            in_flight_instances: HashSet::new(),
            started: snapshot.started,
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
        self.release_ready_invocations_bounded(usize::MAX, execution_context)
    }

    /// Releases at most `max_invocations` currently ready capability invocations.
    ///
    /// Ready order follows compiled instance order, so truncation under a
    /// budget is deterministic. A zero budget releases nothing and records no
    /// lifecycle events, leaving state untouched for the caller's report.
    pub fn release_ready_invocations_bounded(
        &mut self,
        max_invocations: usize,
        execution_context: CapabilityExecutionContext,
    ) -> Result<Vec<CapabilityInvocationPayload>, ApiError> {
        if max_invocations == 0 {
            return Ok(Vec::new());
        }

        let mut ready = self.ready_capability_instances();
        ready.truncate(max_invocations);
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
        let mut emitted_ids = BTreeSet::new();
        for artifact in &emitted_artifacts {
            if !emitted_ids.insert(artifact.artifact_id.as_str())
                || self
                    .artifact_repo
                    .get_artifact(&artifact.artifact_id)
                    .is_some()
            {
                return Err(ApiError::ConfigError(format!(
                    "Task executor invocation '{}' emitted duplicate artifact '{}'",
                    invocation_id, artifact.artifact_id
                )));
            }
        }
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
        if !self
            .in_flight_instances
            .contains(&record.capability_instance_id)
        {
            return Err(ApiError::ConfigError(format!(
                "Task executor invocation '{}' is not in flight",
                invocation_id
            )));
        }

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

        let mut event =
            Self::build_task_event(task_id, task_run_id, target_node_id, "task_succeeded");
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
        if self
            .artifact_repo
            .get_artifact(&failure_summary.artifact_id)
            .is_some()
        {
            return Err(ApiError::ConfigError(format!(
                "Task executor invocation '{}' emitted duplicate failure artifact '{}'",
                invocation_id, failure_summary.artifact_id
            )));
        }
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
        if !self
            .in_flight_instances
            .contains(&record.capability_instance_id)
        {
            return Err(ApiError::ConfigError(format!(
                "Task executor invocation '{}' is not in flight",
                invocation_id
            )));
        }
        record.failure_summary = Some(failure_summary.clone());
        self.artifact_repo.append_artifact(failure_summary)?;
        self.in_flight_instances
            .remove(&record.capability_instance_id);

        let mut event = Self::build_task_event(task_id, task_run_id, target_node_id, "task_failed");
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
        compile_task_definition, InitArtifactValue, TaskDefinition, TaskDependencyEdge,
        TaskDependencyKind, TaskInitSlotSpec, TaskRunContext,
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

    fn emitted_artifact(
        artifact_id: &str,
        capability_instance_id: &str,
        invocation_id: &str,
        output_slot_id: &str,
    ) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.to_string(),
            artifact_type_id: "resolved_node_ref".to_string(),
            schema_version: 1,
            content: json!({ "node_id": "node_root", "path": "docs" }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: capability_instance_id.to_string(),
                invocation_id: Some(invocation_id.to_string()),
                output_slot_id: Some(output_slot_id.to_string()),
            },
        }
    }

    fn failure_artifact(
        artifact_id: &str,
        capability_instance_id: &str,
        invocation_id: &str,
    ) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.to_string(),
            artifact_type_id: "capability_failure".to_string(),
            schema_version: 1,
            content: json!({ "message": "failed" }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: capability_instance_id.to_string(),
                invocation_id: Some(invocation_id.to_string()),
                output_slot_id: Some("failure".to_string()),
            },
        }
    }

    #[test]
    fn executor_releases_first_ready_invocation() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        assert_eq!(executor.completed_count(), 0);

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

    #[test]
    fn executor_records_success_events_in_order_and_releases_dependents() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        executor
            .record_success(
                &payloads[0].invocation_id,
                vec![emitted_artifact(
                    "artifact_resolved_node",
                    "capinst_resolve",
                    &payloads[0].invocation_id,
                    "resolved_node_ref",
                )],
            )
            .unwrap();
        let next_payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        let event_types = executor
            .events()
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>();

        assert_eq!(next_payloads[0].capability_instance_id, "capinst_traversal");
        assert_eq!(
            event_types,
            vec![
                "task_requested",
                "task_started",
                "task_progressed",
                "task_artifact_emitted",
                "task_succeeded",
                "task_progressed",
            ]
        );
    }

    #[test]
    fn executor_rejects_unknown_and_duplicate_completion_records() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        let unknown = executor
            .record_failure(
                "missing",
                failure_artifact("artifact_failure", "capinst_resolve", "missing"),
                "failed",
            )
            .unwrap_err();
        assert!(unknown.to_string().contains("does not know invocation"));

        executor
            .record_success(
                &payloads[0].invocation_id,
                vec![emitted_artifact(
                    "artifact_resolved_node",
                    "capinst_resolve",
                    &payloads[0].invocation_id,
                    "resolved_node_ref",
                )],
            )
            .unwrap();
        let duplicate = executor
            .record_success(&payloads[0].invocation_id, Vec::new())
            .unwrap_err();

        assert!(duplicate.to_string().contains("is not in flight"));
    }

    #[test]
    fn executor_rejects_duplicate_emitted_artifact_ids_without_mutating_record() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        let artifact = emitted_artifact(
            "artifact_resolved_node",
            "capinst_resolve",
            &payloads[0].invocation_id,
            "resolved_node_ref",
        );
        executor
            .artifact_repo
            .append_artifact(artifact.clone())
            .unwrap();

        let error = executor
            .record_success(&payloads[0].invocation_id, vec![artifact])
            .unwrap_err();

        assert!(error.to_string().contains("duplicate artifact"));
        assert!(executor.invocation_records()[0]
            .emitted_artifacts
            .is_empty());
    }

    #[test]
    fn executor_rejects_duplicate_emitted_artifact_ids_in_same_completion_batch() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        let artifact = emitted_artifact(
            "artifact_resolved_node",
            "capinst_resolve",
            &payloads[0].invocation_id,
            "resolved_node_ref",
        );

        let error = executor
            .record_success(&payloads[0].invocation_id, vec![artifact.clone(), artifact])
            .unwrap_err();

        assert!(error.to_string().contains("duplicate artifact"));
        assert!(executor.invocation_records()[0]
            .emitted_artifacts
            .is_empty());
        assert!(executor
            .artifact_repo()
            .get_artifact("artifact_resolved_node")
            .is_none());
    }

    #[test]
    fn executor_allows_retry_after_recorded_failure() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        executor
            .record_failure(
                &payloads[0].invocation_id,
                failure_artifact(
                    "artifact_failure",
                    "capinst_resolve",
                    &payloads[0].invocation_id,
                ),
                "failed",
            )
            .unwrap();

        let retry_payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        assert_eq!(retry_payloads.len(), 1);
        assert_eq!(retry_payloads[0].execution_context.attempt, 2);
        assert_eq!(
            retry_payloads[0].invocation_id,
            "capinst_resolve::attempt::2"
        );
    }

    #[test]
    fn executor_rejects_duplicate_failure_artifact_without_mutating_record() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        let failure = failure_artifact(
            "artifact_failure",
            "capinst_resolve",
            &payloads[0].invocation_id,
        );
        executor
            .artifact_repo
            .append_artifact(failure.clone())
            .unwrap();

        let error = executor
            .record_failure(&payloads[0].invocation_id, failure, "failed")
            .unwrap_err();

        assert!(error.to_string().contains("duplicate failure artifact"));
        assert!(executor.invocation_records()[0].failure_summary.is_none());
        assert!(!executor
            .events()
            .iter()
            .any(|event| event.event_type == "task_failed"));
    }

    #[test]
    fn executor_records_success_with_no_outputs() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        executor
            .record_success(&payloads[0].invocation_id, Vec::new())
            .unwrap();

        assert_eq!(executor.completed_count(), 1);
        assert!(executor
            .events()
            .iter()
            .any(|event| event.event_type == "task_succeeded"));
    }

    #[test]
    fn executor_does_not_rerelease_in_flight_invocation() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let first = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        let second = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(executor.invocation_records().len(), 1);
    }

    #[test]
    fn executor_applies_expansion_delta_once_and_preserves_existing_graph() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let before_instances = executor.compiled_task().capability_instances.len();
        let applied = executor
            .apply_task_expansion(
                "expansion_1",
                "discover_children",
                "artifact_expansion_request",
                CompiledTaskDelta {
                    init_slots: vec![TaskInitSlotSpec {
                        init_slot_id: "child_selector".to_string(),
                        artifact_type_id: "target_selector".to_string(),
                        schema_version: 1,
                        required: true,
                    }],
                    init_artifacts: vec![ArtifactRecord {
                        artifact_id: "artifact_child_selector".to_string(),
                        artifact_type_id: "target_selector".to_string(),
                        schema_version: 1,
                        content: json!({ "node_id": "node_child" }),
                        producer: ArtifactProducerRef {
                            task_id: "task_docs_writer".to_string(),
                            capability_instance_id: "__task_init__".to_string(),
                            invocation_id: None,
                            output_slot_id: Some("child_selector".to_string()),
                        },
                    }],
                    capability_instances: vec![BoundCapabilityInstance {
                        capability_instance_id: "capinst_child_resolve".to_string(),
                        capability_type_id: "workspace_resolve_node_id".to_string(),
                        capability_version: 1,
                        scope_ref: "workspace".to_string(),
                        scope_kind: "workspace".to_string(),
                        binding_values: vec![],
                        input_wiring: vec![BoundInputWiring {
                            slot_id: "target_selector".to_string(),
                            sources: vec![BoundInputWiringSource::TaskInitSlot {
                                init_slot_id: "child_selector".to_string(),
                                artifact_type_id: "target_selector".to_string(),
                                schema_version: 1,
                            }],
                        }],
                    }],
                    dependency_edges: vec![TaskDependencyEdge {
                        from_capability_instance_id: "capinst_resolve".to_string(),
                        to_capability_instance_id: "capinst_child_resolve".to_string(),
                        kind: TaskDependencyKind::Effect,
                        reason: "expansion ordering".to_string(),
                    }],
                },
            )
            .unwrap();
        let duplicate = executor
            .apply_task_expansion(
                "expansion_1",
                "discover_children",
                "artifact_expansion_request",
                CompiledTaskDelta::default(),
            )
            .unwrap();

        assert!(applied);
        assert!(!duplicate);
        assert_eq!(executor.expansion_records().len(), 1);
        assert_eq!(executor.expansion_records()[0].expansion_id, "expansion_1");
        assert_eq!(
            executor.compiled_task().capability_instances.len(),
            before_instances + 1
        );
        assert!(executor
            .events()
            .iter()
            .any(|event| event.event_type == "task_expansion_applied"));
    }

    fn bare_instance(capability_instance_id: &str) -> BoundCapabilityInstance {
        BoundCapabilityInstance {
            capability_instance_id: capability_instance_id.to_string(),
            capability_type_id: "test_capability".to_string(),
            capability_version: 1,
            scope_ref: "node_a".to_string(),
            scope_kind: "node".to_string(),
            binding_values: vec![],
            input_wiring: vec![],
        }
    }

    fn sibling_task() -> CompiledTaskRecord {
        CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![
                bare_instance("capinst_s1"),
                bare_instance("capinst_s2"),
                bare_instance("capinst_s3"),
            ],
            dependency_edges: vec![],
        }
    }

    fn empty_init_payload() -> TaskInitializationPayload {
        TaskInitializationPayload {
            init_artifacts: vec![],
            ..init_payload()
        }
    }

    #[test]
    fn executor_bounded_release_truncates_ready_order_deterministically() {
        let mut executor =
            TaskExecutor::new(sibling_task(), empty_init_payload(), "repo_docs_writer").unwrap();

        let zero = executor
            .release_ready_invocations_bounded(0, CapabilityExecutionContext::default())
            .unwrap();
        assert!(zero.is_empty());
        assert_eq!(executor.events().len(), 1, "zero budget records no events");

        let first = executor
            .release_ready_invocations_bounded(2, CapabilityExecutionContext::default())
            .unwrap();
        let second = executor
            .release_ready_invocations_bounded(2, CapabilityExecutionContext::default())
            .unwrap();

        assert_eq!(
            first
                .iter()
                .map(|payload| payload.capability_instance_id.as_str())
                .collect::<Vec<_>>(),
            vec!["capinst_s1", "capinst_s2"]
        );
        assert_eq!(second[0].capability_instance_id, "capinst_s3");
        assert_eq!(executor.invocation_records().len(), 3);
    }

    #[test]
    fn executor_snapshot_rejects_in_flight_invocations() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let _ = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();

        let error = executor.snapshot().unwrap_err();

        assert!(error.to_string().contains("in flight"));
    }

    #[test]
    fn executor_snapshot_round_trip_resumes_exactly() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        executor
            .record_success(
                &payloads[0].invocation_id,
                vec![emitted_artifact(
                    "artifact_resolved_node",
                    "capinst_resolve",
                    &payloads[0].invocation_id,
                    "resolved_node_ref",
                )],
            )
            .unwrap();

        let snapshot = executor.snapshot().unwrap();
        let mut resumed =
            TaskExecutor::from_snapshot(snapshot, executor.artifact_repo().clone()).unwrap();

        assert_eq!(resumed.completed_count(), 1);
        assert!(resumed.events().is_empty());
        assert_eq!(
            resumed.ready_capability_instances(),
            vec!["capinst_traversal".to_string()]
        );

        let next = resumed
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        assert_eq!(next[0].invocation_id, "capinst_traversal::attempt::1");
        // The resumed run must not repeat requested or started transitions.
        assert!(resumed
            .events()
            .iter()
            .all(|event| event.event_type == "task_progressed"));
    }

    #[test]
    fn executor_from_snapshot_rejects_artifact_repo_divergence() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let payloads = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap();
        executor
            .record_success(
                &payloads[0].invocation_id,
                vec![emitted_artifact(
                    "artifact_resolved_node",
                    "capinst_resolve",
                    &payloads[0].invocation_id,
                    "resolved_node_ref",
                )],
            )
            .unwrap();
        let snapshot = executor.snapshot().unwrap();

        let error =
            TaskExecutor::from_snapshot(snapshot, TaskArtifactRepo::new("repo_other")).unwrap_err();

        assert!(error.to_string().contains("missing from repo"));
    }

    #[test]
    fn executor_rejects_expansion_delta_collisions() {
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();

        let duplicate_instance = executor
            .apply_task_expansion(
                "expansion_1",
                "discover_children",
                "artifact_expansion_request",
                CompiledTaskDelta {
                    capability_instances: vec![
                        executor.compiled_task().capability_instances[0].clone()
                    ],
                    ..CompiledTaskDelta::default()
                },
            )
            .unwrap_err();

        assert!(duplicate_instance
            .to_string()
            .contains("already contains capability instance"));

        let duplicate_edge = executor
            .apply_task_expansion(
                "expansion_2",
                "discover_children",
                "artifact_expansion_request",
                CompiledTaskDelta {
                    dependency_edges: vec![executor.compiled_task().dependency_edges[0].clone()],
                    ..CompiledTaskDelta::default()
                },
            )
            .unwrap_err();

        assert!(duplicate_edge
            .to_string()
            .contains("already contains dependency edge"));
    }
}
