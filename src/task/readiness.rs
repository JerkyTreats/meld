//! Task readiness evaluation over compiled task structure and artifact state.

use crate::capability::BoundInputWiringSource;
use crate::task::{CompiledTaskRecord, TaskArtifactRepo};
use std::collections::HashSet;

/// Computes the currently ready capability instances inside one task.
pub fn compute_ready_capability_instances(
    compiled_task: &CompiledTaskRecord,
    artifact_repo: &TaskArtifactRepo,
    completed_instances: &HashSet<String>,
    in_flight_instances: &HashSet<String>,
) -> Vec<String> {
    let mut ready = Vec::new();

    for instance in &compiled_task.capability_instances {
        if completed_instances.contains(&instance.capability_instance_id)
            || in_flight_instances.contains(&instance.capability_instance_id)
        {
            continue;
        }

        let dependencies_satisfied = compiled_task
            .dependency_edges
            .iter()
            .filter(|edge| edge.to_capability_instance_id == instance.capability_instance_id)
            .all(|edge| completed_instances.contains(&edge.from_capability_instance_id));
        if !dependencies_satisfied {
            continue;
        }

        let inputs_satisfied = instance.input_wiring.iter().all(|wiring| {
            wiring
                .sources
                .iter()
                .all(|source| source_is_available(artifact_repo, source))
        });
        if inputs_satisfied {
            ready.push(instance.capability_instance_id.clone());
        }
    }

    ready
}

fn source_is_available(artifact_repo: &TaskArtifactRepo, source: &BoundInputWiringSource) -> bool {
    match source {
        BoundInputWiringSource::TaskInitSlot { init_slot_id, .. } => artifact_repo
            .artifacts_for_output_slot("__task_init__", init_slot_id)
            .last()
            .is_some(),
        BoundInputWiringSource::UpstreamOutput {
            capability_instance_id,
            output_slot_id,
            ..
        } => artifact_repo
            .artifacts_for_output_slot(capability_instance_id, output_slot_id)
            .last()
            .is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource};
    use crate::task::{
        ArtifactProducerRef, ArtifactRecord, CompiledTaskRecord, TaskArtifactRepo,
        TaskDependencyEdge, TaskDependencyKind,
    };
    use serde_json::json;

    #[test]
    fn readiness_requires_dependency_and_artifact_satisfaction() {
        let compiled_task = CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![
                BoundCapabilityInstance {
                    capability_instance_id: "capinst_parent".to_string(),
                    capability_type_id: "context_generate_finalize".to_string(),
                    capability_version: 1,
                    scope_ref: "node_parent".to_string(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![],
                    input_wiring: vec![BoundInputWiring {
                        slot_id: "child_summary".to_string(),
                        sources: vec![BoundInputWiringSource::UpstreamOutput {
                            capability_instance_id: "capinst_child".to_string(),
                            output_slot_id: "readme_summary".to_string(),
                            artifact_type_id: "readme_summary".to_string(),
                            schema_version: 1,
                        }],
                    }],
                },
                BoundCapabilityInstance {
                    capability_instance_id: "capinst_child".to_string(),
                    capability_type_id: "context_generate_finalize".to_string(),
                    capability_version: 1,
                    scope_ref: "node_child".to_string(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![],
                    input_wiring: vec![],
                },
            ],
            dependency_edges: vec![TaskDependencyEdge {
                from_capability_instance_id: "capinst_child".to_string(),
                to_capability_instance_id: "capinst_parent".to_string(),
                kind: TaskDependencyKind::Artifact,
                reason: "child before parent".to_string(),
            }],
        };
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        let mut completed = HashSet::new();
        let in_flight = HashSet::new();

        let ready =
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight);
        assert_eq!(ready, vec!["capinst_child".to_string()]);

        completed.insert("capinst_child".to_string());
        repo.append_artifact(ArtifactRecord {
            artifact_id: "artifact_child".to_string(),
            artifact_type_id: "readme_summary".to_string(),
            schema_version: 1,
            content: json!({ "summary": "child" }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: "capinst_child".to_string(),
                invocation_id: Some("invk_1".to_string()),
                output_slot_id: Some("readme_summary".to_string()),
            },
        })
        .unwrap();

        let ready =
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight);
        assert_eq!(ready, vec!["capinst_parent".to_string()]);
    }
}
