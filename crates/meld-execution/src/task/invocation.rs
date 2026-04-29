//! Capability payload assembly from task-owned state.

use crate::capability::{
    ArtifactValueRef, BoundCapabilityInstance, CapabilityExecutionContext,
    CapabilityInvocationPayload, InputValueSource, SuppliedInputValue, SuppliedValueRef,
    UpstreamLineage,
};
use crate::error::ApiError;
use crate::task::{CompiledTaskRecord, TaskArtifactRepo, TaskInitializationPayload};

/// Assembles one capability invocation payload from task-owned state.
pub fn assemble_invocation_payload(
    compiled_task: &CompiledTaskRecord,
    init_payload: &TaskInitializationPayload,
    artifact_repo: &TaskArtifactRepo,
    instance: &BoundCapabilityInstance,
    invocation_id: impl Into<String>,
    execution_context: CapabilityExecutionContext,
    lineage: Option<UpstreamLineage>,
) -> Result<CapabilityInvocationPayload, ApiError> {
    let mut supplied_inputs = Vec::new();
    for wiring in &instance.input_wiring {
        for source in &wiring.sources {
            match source {
                crate::capability::BoundInputWiringSource::TaskInitSlot {
                    init_slot_id, ..
                } => {
                    let artifacts =
                        artifact_repo.artifacts_for_output_slot("__task_init__", init_slot_id);
                    let artifact = artifacts.last().ok_or_else(|| {
                        ApiError::ConfigError(format!(
                            "Task '{}' is missing init artifact for slot '{}'",
                            compiled_task.task_id, init_slot_id
                        ))
                    })?;
                    supplied_inputs.push(SuppliedInputValue {
                        slot_id: wiring.slot_id.clone(),
                        source: InputValueSource::InitPayload,
                        value: SuppliedValueRef::Artifact(ArtifactValueRef {
                            artifact_id: artifact.artifact_id.clone(),
                            artifact_type_id: artifact.artifact_type_id.clone(),
                            schema_version: artifact.schema_version,
                            content: artifact.content.clone(),
                        }),
                    });
                }
                crate::capability::BoundInputWiringSource::UpstreamOutput {
                    capability_instance_id,
                    output_slot_id,
                    ..
                } => {
                    let artifacts = artifact_repo
                        .artifacts_for_output_slot(capability_instance_id, output_slot_id);
                    let artifact = artifacts.last().ok_or_else(|| {
                        ApiError::ConfigError(format!(
                            "Task '{}' is missing upstream artifact from '{}' output '{}'",
                            compiled_task.task_id, capability_instance_id, output_slot_id
                        ))
                    })?;
                    supplied_inputs.push(SuppliedInputValue {
                        slot_id: wiring.slot_id.clone(),
                        source: InputValueSource::ArtifactHandoff,
                        value: SuppliedValueRef::Artifact(ArtifactValueRef {
                            artifact_id: artifact.artifact_id.clone(),
                            artifact_type_id: artifact.artifact_type_id.clone(),
                            schema_version: artifact.schema_version,
                            content: artifact.content.clone(),
                        }),
                    });
                }
            }
        }
    }

    Ok(CapabilityInvocationPayload {
        invocation_id: invocation_id.into(),
        capability_instance_id: instance.capability_instance_id.clone(),
        supplied_inputs,
        upstream_lineage: lineage.or_else(|| {
            Some(UpstreamLineage {
                task_id: compiled_task.task_id.clone(),
                task_run_id: init_payload.task_run_context.task_run_id.clone(),
                capability_path: vec![instance.capability_instance_id.clone()],
                batch_index: None,
                node_index: None,
                repair_scope: None,
            })
        }),
        execution_context,
    })
}
