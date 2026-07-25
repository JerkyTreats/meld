//! Bounded durable stepping over one compiled task package.
//!
//! Owner: task domain. This module makes the existing task package executor
//! the first implementor of the task-network bounded package-step contract.
//! It is compatibility-scoped: bounded stepping is built beside
//! `execute_task_to_completion` and changes only how much work runs per step,
//! never what gets executed or its dependency order.
//!
//! One step releases at most one bounded ready wave through the same executor
//! transitions the completion path uses, resolves every released invocation,
//! and persists executor, expansion, artifact, readiness, and completion
//! progress before returning. A fresh instance opened over the same database
//! resumes exactly where the last step stopped.

use crate::capability::{
    BoundCapabilityInstance, CapabilityExecutionContext, CapabilityInvocationPayload,
    CapabilityInvocationResult,
};
use crate::error::ApiError;
use crate::task::executor::TaskExecutor;
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use crate::task::progress::TaskProgressStore;
use crate::task::runtime::failure_artifact;
use crate::task::{
    parse_task_expansion_request_artifact, CompiledTaskRecord, TaskArtifactRepo,
    TaskExecutorSnapshot, TaskInitializationPayload,
};
use crate::task_network::package_step::{
    PackageStep, PackageStepProgress, PackageStepReport, PackageStepRequest,
};
use async_trait::async_trait;
use std::collections::HashSet;

/// Capability invocation and expansion compilation port for bounded steps.
///
/// The stepper owns wave sequencing and durability. The invoker owns how one
/// released capability invocation executes and how expansion request
/// artifacts compile into append-only deltas.
#[async_trait]
pub trait PackageStepInvoker: Send + Sync {
    /// Executes one released capability invocation.
    async fn invoke_capability(
        &self,
        instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ApiError>;

    /// Compiles one task expansion request into an append-only delta.
    fn compile_expansion(
        &self,
        compiled_task: &CompiledTaskRecord,
        request: &TaskExpansionRequest,
    ) -> Result<CompiledTaskDelta, ApiError>;
}

/// Returns the artifact repo id used for one bounded package run.
///
/// Hosts that need the run's artifacts after the stepper closes reopen the
/// task artifact repository under this id in the same database.
pub fn package_step_repo_id(task_run_id: &str) -> String {
    format!("package_step::{task_run_id}")
}

/// Durable bounded package execution over the existing task executor.
///
/// First implementor of the task-network `PackageStep` contract. Opening over
/// a database that already holds progress for the same task run resumes that
/// run: completed invocations are never repeated and applied expansions are
/// never reapplied.
pub struct DurablePackageExecution<I> {
    executor: TaskExecutor,
    progress: TaskProgressStore,
    task_run_id: String,
    invoker: I,
}

impl<I: PackageStepInvoker> DurablePackageExecution<I> {
    /// Opens a durable bounded package execution within a caller-owned database.
    ///
    /// A first open seeds the executor and persists a baseline snapshot so a
    /// reopen before the first step already resumes from durable state. A
    /// later open with drifted compiled task identity or initialization
    /// payload is rejected instead of silently forking the run.
    pub fn open(
        db: sled::Db,
        compiled_task: CompiledTaskRecord,
        init_payload: TaskInitializationPayload,
        invoker: I,
    ) -> Result<Self, ApiError> {
        let task_run_id = init_payload.task_run_context.task_run_id.clone();
        let artifact_repo =
            TaskArtifactRepo::open_sled(db.clone(), package_step_repo_id(&task_run_id))
                .map_err(to_api)?;
        let progress = TaskProgressStore::open(db).map_err(to_api)?;

        let executor = match progress.load(&task_run_id).map_err(to_api)? {
            Some(snapshot) => {
                validate_resume_identity(&snapshot, &compiled_task, &init_payload)?;
                TaskExecutor::from_snapshot(snapshot, artifact_repo)?
            }
            None => {
                let executor = TaskExecutor::new_with_artifact_repo(
                    compiled_task,
                    init_payload,
                    artifact_repo,
                )?;
                progress
                    .save(&task_run_id, &executor.snapshot()?)
                    .map_err(to_api)?;
                executor
            }
        };

        Ok(Self {
            executor,
            progress,
            task_run_id,
            invoker,
        })
    }

    /// Returns the live task executor backing this bounded run.
    pub fn executor(&self) -> &TaskExecutor {
        &self.executor
    }

    fn progress_counters(&self) -> PackageStepProgress {
        // Readiness is recomputed from durable state on demand rather than
        // stored separately, so it can never drift from the graph and repo.
        PackageStepProgress {
            known_units: self.executor.compiled_task().capability_instances.len(),
            completed_units: self.executor.completed_count(),
            ready_units: self.executor.ready_capability_instances().len(),
            applied_expansions: self.executor.expansion_records().len(),
            persisted_artifacts: self.executor.artifact_repo().record().artifacts.len(),
        }
    }

    fn persist_progress(&self) -> Result<(), ApiError> {
        // Artifact appends were already durable per write; the snapshot commits
        // executor, expansion, and completion progress for reopen.
        let snapshot = self.executor.snapshot()?;
        self.progress
            .save(&self.task_run_id, &snapshot)
            .map_err(to_api)?;
        self.executor.artifact_repo().flush().map_err(to_api)?;
        Ok(())
    }

    fn no_work_report(
        input_progress: PackageStepProgress,
        budget_exhausted: bool,
        package_complete: bool,
    ) -> PackageStepReport {
        PackageStepReport {
            items_attempted: 0,
            items_committed: 0,
            output_progress: input_progress.clone(),
            input_progress,
            budget_exhausted,
            package_complete,
        }
    }
}

#[async_trait]
impl<I: PackageStepInvoker> PackageStep for DurablePackageExecution<I> {
    type Error = ApiError;

    fn progress(&self) -> PackageStepProgress {
        self.progress_counters()
    }

    async fn step(&mut self, request: &PackageStepRequest) -> Result<PackageStepReport, ApiError> {
        let input_progress = self.progress_counters();
        if self.executor.is_complete() {
            // Completed runs attempt no work: repeated steps are durable no-ops.
            return Ok(Self::no_work_report(input_progress, false, true));
        }
        if request.max_ready_invocations == 0 {
            let pending = input_progress.ready_units > 0;
            return Ok(Self::no_work_report(input_progress, pending, false));
        }

        let released = self.executor.release_ready_invocations_bounded(
            request.max_ready_invocations,
            CapabilityExecutionContext::default(),
        )?;
        if released.is_empty() {
            // Step boundaries hold no in-flight work, so an incomplete run
            // with nothing ready is permanently blocked. Mirror the
            // completion path error instead of reporting empty progress.
            return Err(ApiError::GenerationFailed(format!(
                "Task '{}' is blocked with no ready capability instances",
                self.executor.compiled_task().task_id
            )));
        }
        let budget_exhausted = input_progress.ready_units > released.len();

        // Sibling payloads were assembled together at release, exactly like
        // the completion path, so concurrent resolution cannot leak one
        // sibling's outputs into another's inputs.
        let mut instances = Vec::with_capacity(released.len());
        for payload in &released {
            let instance = self
                .executor
                .compiled_task()
                .capability_instances
                .iter()
                .find(|instance| instance.capability_instance_id == payload.capability_instance_id)
                .cloned()
                .ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Task '{}' is missing capability instance '{}'",
                        self.executor.compiled_task().task_id,
                        payload.capability_instance_id
                    ))
                })?;
            instances.push(instance);
        }
        let invoker = &self.invoker;
        let outcomes = futures::future::join_all(
            released
                .iter()
                .zip(instances.iter())
                .map(|(payload, instance)| invoker.invoke_capability(instance, payload)),
        )
        .await;

        // Every released invocation resolves before progress persists, so the
        // snapshot is always quiesced and failed work is durable, not repeated
        // blindly on reopen.
        let mut items_committed = 0usize;
        let mut first_error = None;
        for (payload, outcome) in released.iter().zip(outcomes) {
            match outcome {
                Ok(result) => {
                    let mut expansion_requests = Vec::new();
                    for artifact in &result.emitted_artifacts {
                        if let Some(expansion) = parse_task_expansion_request_artifact(artifact)? {
                            expansion_requests.push((artifact.artifact_id.clone(), expansion));
                        }
                    }
                    self.executor
                        .record_success(&payload.invocation_id, result.emitted_artifacts)?;
                    for (source_artifact_id, expansion_request) in expansion_requests {
                        let delta = self
                            .invoker
                            .compile_expansion(self.executor.compiled_task(), &expansion_request)?;
                        // Duplicate expansion ids stay durable no-ops, matching
                        // the completion path's idempotent expansion behavior.
                        let _ = self.executor.apply_task_expansion(
                            &expansion_request.expansion_id,
                            &expansion_request.expansion_kind,
                            &source_artifact_id,
                            delta,
                        )?;
                    }
                    items_committed += 1;
                }
                Err(error) => {
                    self.executor.record_failure(
                        &payload.invocation_id,
                        failure_artifact(
                            self.executor.compiled_task().task_id.clone(),
                            payload.capability_instance_id.clone(),
                            payload.invocation_id.clone(),
                            error.to_string(),
                        ),
                        error.to_string(),
                    )?;
                    first_error.get_or_insert(error);
                }
            }
        }

        self.persist_progress()?;
        if let Some(error) = first_error {
            return Err(error);
        }

        Ok(PackageStepReport {
            items_attempted: released.len(),
            items_committed,
            input_progress,
            output_progress: self.progress_counters(),
            budget_exhausted,
            package_complete: self.executor.is_complete(),
        })
    }
}

fn validate_resume_identity(
    snapshot: &TaskExecutorSnapshot,
    compiled_task: &CompiledTaskRecord,
    init_payload: &TaskInitializationPayload,
) -> Result<(), ApiError> {
    if snapshot.init_payload != *init_payload {
        return Err(ApiError::ConfigError(format!(
            "Task run '{}' already has durable progress with a different initialization payload",
            init_payload.task_run_context.task_run_id
        )));
    }
    if snapshot.compiled_task.task_id != compiled_task.task_id
        || snapshot.compiled_task.task_version != compiled_task.task_version
    {
        return Err(ApiError::ConfigError(format!(
            "Task run '{}' progress belongs to task '{}' v{} but caller supplied '{}' v{}",
            init_payload.task_run_context.task_run_id,
            snapshot.compiled_task.task_id,
            snapshot.compiled_task.task_version,
            compiled_task.task_id,
            compiled_task.task_version
        )));
    }

    // Expansion only appends, so every base instance must survive in the
    // expanded snapshot graph; a missing base instance means graph drift.
    let snapshot_instances = snapshot
        .compiled_task
        .capability_instances
        .iter()
        .map(|instance| instance.capability_instance_id.as_str())
        .collect::<HashSet<_>>();
    for instance in &compiled_task.capability_instances {
        if !snapshot_instances.contains(instance.capability_instance_id.as_str()) {
            return Err(ApiError::ConfigError(format!(
                "Task run '{}' progress is missing base capability instance '{}'",
                init_payload.task_run_context.task_run_id, instance.capability_instance_id
            )));
        }
    }
    Ok(())
}

fn to_api(error: impl std::fmt::Display) -> ApiError {
    ApiError::ConfigError(error.to_string())
}
