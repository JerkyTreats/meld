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
    source_unavailability(artifact_repo, source).is_none()
}

/// Why a wiring source is not satisfiable, or `None` when it is. The declared
/// artifact type and schema version are part of the contract: a present
/// artifact of the wrong shape must read as unavailable, matching the
/// task-network readiness sibling, not as satisfied.
fn source_unavailability(
    artifact_repo: &TaskArtifactRepo,
    source: &BoundInputWiringSource,
) -> Option<String> {
    let (producer, slot, expected_type, expected_schema) = match source {
        BoundInputWiringSource::TaskInitSlot {
            init_slot_id,
            artifact_type_id,
            schema_version,
        } => (
            "__task_init__",
            init_slot_id.as_str(),
            artifact_type_id,
            *schema_version,
        ),
        BoundInputWiringSource::UpstreamOutput {
            capability_instance_id,
            output_slot_id,
            artifact_type_id,
            schema_version,
        } => (
            capability_instance_id.as_str(),
            output_slot_id.as_str(),
            artifact_type_id,
            *schema_version,
        ),
    };
    let artifacts = artifact_repo.artifacts_for_output_slot(producer, slot);
    let Some(artifact) = artifacts.last() else {
        return Some(format!("no artifact from '{producer}' slot '{slot}'"));
    };
    if &artifact.artifact_type_id != expected_type {
        return Some(format!(
            "artifact from '{producer}' slot '{slot}' has type '{}', expected '{expected_type}'",
            artifact.artifact_type_id
        ));
    }
    if artifact.schema_version != expected_schema {
        return Some(format!(
            "artifact from '{producer}' slot '{slot}' has schema version {}, expected {expected_schema}",
            artifact.schema_version
        ));
    }
    None
}

/// Human-readable reasons each non-terminal instance is blocked. Empty for
/// instances that are ready; used to make a blocked task narrate its cause
/// instead of reporting only that no instance is ready.
pub fn blocked_instance_diagnostics(
    compiled_task: &CompiledTaskRecord,
    artifact_repo: &TaskArtifactRepo,
    completed_instances: &HashSet<String>,
    in_flight_instances: &HashSet<String>,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    for instance in &compiled_task.capability_instances {
        if completed_instances.contains(&instance.capability_instance_id)
            || in_flight_instances.contains(&instance.capability_instance_id)
        {
            continue;
        }
        let unmet_dependencies: Vec<&str> = compiled_task
            .dependency_edges
            .iter()
            .filter(|edge| edge.to_capability_instance_id == instance.capability_instance_id)
            .filter(|edge| !completed_instances.contains(&edge.from_capability_instance_id))
            .map(|edge| edge.from_capability_instance_id.as_str())
            .collect();
        for dependency in unmet_dependencies {
            diagnostics.push(format!(
                "'{}' waits on incomplete dependency '{dependency}'",
                instance.capability_instance_id
            ));
        }
        for wiring in &instance.input_wiring {
            for source in &wiring.sources {
                if let Some(reason) = source_unavailability(artifact_repo, source) {
                    diagnostics.push(format!(
                        "'{}' slot '{}': {reason}",
                        instance.capability_instance_id, wiring.slot_id
                    ));
                }
            }
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource};
    use crate::task::{
        ArtifactProducerRef, ArtifactRecord, CompiledTaskRecord, TaskArtifactRepo,
        TaskDependencyEdge, TaskDependencyKind,
    };
    use proptest::prelude::*;
    use serde_json::json;

    fn instance(
        capability_instance_id: &str,
        input_wiring: Vec<BoundInputWiring>,
    ) -> BoundCapabilityInstance {
        BoundCapabilityInstance {
            capability_instance_id: capability_instance_id.to_string(),
            capability_type_id: "context_generate_finalize".to_string(),
            capability_version: 1,
            scope_ref: format!("node_{capability_instance_id}"),
            scope_kind: "node".to_string(),
            binding_values: vec![],
            input_wiring,
        }
    }

    fn output_source(capability_instance_id: &str, output_slot_id: &str) -> BoundInputWiringSource {
        BoundInputWiringSource::UpstreamOutput {
            capability_instance_id: capability_instance_id.to_string(),
            output_slot_id: output_slot_id.to_string(),
            artifact_type_id: "readme_summary".to_string(),
            schema_version: 1,
        }
    }

    fn init_source(init_slot_id: &str) -> BoundInputWiringSource {
        BoundInputWiringSource::TaskInitSlot {
            init_slot_id: init_slot_id.to_string(),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
        }
    }

    fn artifact(
        artifact_id: &str,
        capability_instance_id: &str,
        output_slot_id: &str,
    ) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.to_string(),
            artifact_type_id: "readme_summary".to_string(),
            schema_version: 1,
            content: json!({ "summary": artifact_id }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: capability_instance_id.to_string(),
                invocation_id: Some("invk_1".to_string()),
                output_slot_id: Some(output_slot_id.to_string()),
            },
        }
    }

    fn init_artifact(init_slot_id: &str) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: format!("init::{init_slot_id}"),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
            content: json!({ "node_id": "node_root" }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: "__task_init__".to_string(),
                invocation_id: None,
                output_slot_id: Some(init_slot_id.to_string()),
            },
        }
    }

    fn chain_task() -> CompiledTaskRecord {
        CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![
                instance(
                    "capinst_parent",
                    vec![BoundInputWiring {
                        slot_id: "child_summary".to_string(),
                        sources: vec![output_source("capinst_child", "readme_summary")],
                    }],
                ),
                instance("capinst_child", vec![]),
            ],
            dependency_edges: vec![TaskDependencyEdge {
                from_capability_instance_id: "capinst_child".to_string(),
                to_capability_instance_id: "capinst_parent".to_string(),
                kind: TaskDependencyKind::Artifact,
                reason: "child before parent".to_string(),
            }],
        }
    }

    #[test]
    fn readiness_requires_dependency_and_artifact_satisfaction() {
        let compiled_task = chain_task();
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        let mut completed = HashSet::new();
        let in_flight = HashSet::new();

        let ready =
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight);
        assert_eq!(ready, vec!["capinst_child".to_string()]);

        completed.insert("capinst_child".to_string());
        repo.append_artifact(artifact(
            "artifact_child",
            "capinst_child",
            "readme_summary",
        ))
        .unwrap();

        let ready =
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight);
        assert_eq!(ready, vec!["capinst_parent".to_string()]);
    }

    #[test]
    fn readiness_requires_mixed_init_and_upstream_inputs() {
        let compiled_task = CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![instance(
                "capinst_parent",
                vec![
                    BoundInputWiring {
                        slot_id: "selector".to_string(),
                        sources: vec![init_source("target_selector")],
                    },
                    BoundInputWiring {
                        slot_id: "summary".to_string(),
                        sources: vec![output_source("capinst_child", "readme_summary")],
                    },
                ],
            )],
            dependency_edges: vec![TaskDependencyEdge {
                from_capability_instance_id: "capinst_child".to_string(),
                to_capability_instance_id: "capinst_parent".to_string(),
                kind: TaskDependencyKind::Artifact,
                reason: "child before parent".to_string(),
            }],
        };
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        let mut completed = HashSet::from(["capinst_child".to_string()]);
        let in_flight = HashSet::new();

        repo.append_artifact(artifact(
            "artifact_child",
            "capinst_child",
            "readme_summary",
        ))
        .unwrap();
        assert!(
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight)
                .is_empty()
        );

        repo.append_artifact(init_artifact("target_selector"))
            .unwrap();
        assert_eq!(
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight),
            vec!["capinst_parent".to_string()]
        );

        completed.insert("capinst_parent".to_string());
        assert!(
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight)
                .is_empty()
        );
    }

    #[test]
    fn readiness_excludes_in_flight_and_fans_out_completed_artifacts() {
        let compiled_task = CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![
                instance(
                    "capinst_a",
                    vec![BoundInputWiring {
                        slot_id: "summary".to_string(),
                        sources: vec![output_source("capinst_source", "readme_summary")],
                    }],
                ),
                instance(
                    "capinst_b",
                    vec![BoundInputWiring {
                        slot_id: "summary".to_string(),
                        sources: vec![output_source("capinst_source", "readme_summary")],
                    }],
                ),
            ],
            dependency_edges: vec![
                TaskDependencyEdge {
                    from_capability_instance_id: "capinst_source".to_string(),
                    to_capability_instance_id: "capinst_a".to_string(),
                    kind: TaskDependencyKind::Artifact,
                    reason: "source before a".to_string(),
                },
                TaskDependencyEdge {
                    from_capability_instance_id: "capinst_source".to_string(),
                    to_capability_instance_id: "capinst_b".to_string(),
                    kind: TaskDependencyKind::Artifact,
                    reason: "source before b".to_string(),
                },
            ],
        };
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        repo.append_artifact(artifact(
            "artifact_source",
            "capinst_source",
            "readme_summary",
        ))
        .unwrap();
        let completed = HashSet::from(["capinst_source".to_string()]);
        let in_flight = HashSet::from(["capinst_b".to_string()]);

        let ready =
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight);

        assert_eq!(ready, vec!["capinst_a".to_string()]);
    }

    #[test]
    fn readiness_honors_effect_only_edges_and_missing_artifacts() {
        let compiled_task = CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![
                instance("capinst_first", vec![]),
                instance("capinst_second", vec![]),
                instance(
                    "capinst_needs_artifact",
                    vec![BoundInputWiring {
                        slot_id: "summary".to_string(),
                        sources: vec![output_source("capinst_first", "readme_summary")],
                    }],
                ),
            ],
            dependency_edges: vec![
                TaskDependencyEdge {
                    from_capability_instance_id: "capinst_first".to_string(),
                    to_capability_instance_id: "capinst_second".to_string(),
                    kind: TaskDependencyKind::Effect,
                    reason: "exclusive effect".to_string(),
                },
                TaskDependencyEdge {
                    from_capability_instance_id: "capinst_first".to_string(),
                    to_capability_instance_id: "capinst_needs_artifact".to_string(),
                    kind: TaskDependencyKind::Artifact,
                    reason: "artifact handoff".to_string(),
                },
            ],
        };
        let repo = TaskArtifactRepo::new("repo_docs_writer");
        let completed = HashSet::from(["capinst_first".to_string()]);
        let in_flight = HashSet::new();

        let ready =
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight);

        assert_eq!(ready, vec!["capinst_second".to_string()]);
    }

    #[test]
    fn readiness_rejects_type_and_schema_mismatched_artifacts() {
        let compiled_task = chain_task();
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        let completed = HashSet::from(["capinst_child".to_string()]);
        let in_flight = HashSet::new();

        let mut wrong_type = artifact("artifact_wrong_type", "capinst_child", "readme_summary");
        wrong_type.artifact_type_id = "frame_ref".to_string();
        repo.append_artifact(wrong_type).unwrap();
        assert!(
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight)
                .is_empty(),
            "type-mismatched artifact must not satisfy readiness"
        );
        let diagnostics =
            blocked_instance_diagnostics(&compiled_task, &repo, &completed, &in_flight);
        assert!(diagnostics
            .iter()
            .any(|reason| reason.contains("has type 'frame_ref'")));

        let mut wrong_schema = artifact("artifact_wrong_schema", "capinst_child", "readme_summary");
        wrong_schema.schema_version = 2;
        repo.append_artifact(wrong_schema).unwrap();
        assert!(
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight)
                .is_empty(),
            "schema-mismatched artifact must not satisfy readiness"
        );

        repo.append_artifact(artifact("artifact_ok", "capinst_child", "readme_summary"))
            .unwrap();
        assert_eq!(
            compute_ready_capability_instances(&compiled_task, &repo, &completed, &in_flight),
            vec!["capinst_parent".to_string()]
        );
    }

    #[test]
    fn readiness_keeps_cycles_blocked() {
        let compiled_task = CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![
                instance("capinst_a", vec![]),
                instance("capinst_b", vec![]),
            ],
            dependency_edges: vec![
                TaskDependencyEdge {
                    from_capability_instance_id: "capinst_a".to_string(),
                    to_capability_instance_id: "capinst_b".to_string(),
                    kind: TaskDependencyKind::Effect,
                    reason: "cycle".to_string(),
                },
                TaskDependencyEdge {
                    from_capability_instance_id: "capinst_b".to_string(),
                    to_capability_instance_id: "capinst_a".to_string(),
                    kind: TaskDependencyKind::Effect,
                    reason: "cycle".to_string(),
                },
            ],
        };

        assert!(compute_ready_capability_instances(
            &compiled_task,
            &TaskArtifactRepo::new("repo_docs_writer"),
            &HashSet::new(),
            &HashSet::new(),
        )
        .is_empty());
    }

    proptest! {
        #[test]
        fn readiness_never_returns_completed_or_in_flight_instances(
            complete_a in any::<bool>(),
            complete_b in any::<bool>(),
            in_flight_a in any::<bool>(),
            in_flight_b in any::<bool>(),
        ) {
            let compiled_task = CompiledTaskRecord {
                task_id: "task_docs_writer".to_string(),
                task_version: 1,
                init_slots: vec![],
                capability_instances: vec![instance("capinst_a", vec![]), instance("capinst_b", vec![])],
                dependency_edges: vec![],
            };
            let mut completed = HashSet::new();
            let mut in_flight = HashSet::new();
            if complete_a {
                completed.insert("capinst_a".to_string());
            }
            if complete_b {
                completed.insert("capinst_b".to_string());
            }
            if in_flight_a {
                in_flight.insert("capinst_a".to_string());
            }
            if in_flight_b {
                in_flight.insert("capinst_b".to_string());
            }

            let ready = compute_ready_capability_instances(
                &compiled_task,
                &TaskArtifactRepo::new("repo_docs_writer"),
                &completed,
                &in_flight,
            );

            prop_assert!(ready.iter().all(|id| !completed.contains(id)));
            prop_assert!(ready.iter().all(|id| !in_flight.contains(id)));
        }
    }
}
