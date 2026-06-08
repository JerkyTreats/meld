#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task::{validate_task_initialization, ArtifactProducerRef, ArtifactRecord};
use meld_execution::task_network::dispatch::{Outcome, OutcomeStatus};
use meld_execution::task_network::initialization::{
    materialize_task_initialization, TaskInitializationDiagnosticCode,
};
use meld_execution::task_network::state::{ArtifactAvailability, NetworkState, TaskStatus};
use serde_json::json;

fn artifact(artifact_id: &str, artifact_type_id: &str, schema_version: u32) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: artifact_id.to_string(),
        artifact_type_id: artifact_type_id.to_string(),
        schema_version,
        content: json!({
            "artifact_id": artifact_id,
        }),
        producer: ArtifactProducerRef {
            task_id: "compiled-task-upstream".to_string(),
            capability_instance_id: "produce".to_string(),
            invocation_id: Some("invoke-upstream".to_string()),
            output_slot_id: Some(artifact_type_id.to_string()),
        },
    }
}

fn upstream_state(artifact_records: Vec<ArtifactRecord>) -> NetworkState {
    let mut state = NetworkState::empty("network-docs");
    state.tasks.insert(
        "task-upstream".to_string(),
        task_network_support::single_task_node("task-upstream"),
    );
    state.tasks.insert(
        "task-downstream".to_string(),
        task_network_support::task_node_with_upstream_source(
            "task-downstream",
            "task-upstream",
            "metadata",
            "metadata_doc",
            1,
        ),
    );
    state.statuses.insert(
        "task-upstream".to_string(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-upstream".to_string(),
        },
    );
    state
        .statuses
        .insert("task-downstream".to_string(), TaskStatus::Pending);
    state
        .artifact_availability
        .extend(
            artifact_records
                .iter()
                .map(|artifact| ArtifactAvailability {
                    task_instance_id: "task-upstream".to_string(),
                    artifact_type_id: artifact.artifact_type_id.clone(),
                    artifact_id: artifact.artifact_id.clone(),
                    schema_version: artifact.schema_version,
                }),
        );
    state.outcomes.insert(
        "outcome-upstream".to_string(),
        Outcome {
            outcome_id: "outcome-upstream".to_string(),
            task_instance_id: "task-upstream".to_string(),
            lifecycle_epoch: 1,
            claim_id: "claim-upstream".to_string(),
            claim_revision: 1,
            status: OutcomeStatus::Succeeded,
            error: None,
            artifact_records,
            task_events: vec![],
        },
    );
    state.set_revision_and_hash(1);
    state
}

#[test]
fn static_seed_materializes_into_init_artifact_value() {
    let mut state = NetworkState::empty("network-docs");
    let node = task_network_support::task_node_with_static_seed(
        "task-source",
        "metadata",
        "metadata_doc",
        1,
    );
    state.tasks.insert("task-source".to_string(), node);
    state
        .statuses
        .insert("task-source".to_string(), TaskStatus::Pending);
    state.set_revision_and_hash(1);

    let materialized = materialize_task_initialization(&state, "task-source").unwrap();

    assert_eq!(materialized.payload.init_artifacts.len(), 1);
    assert_eq!(
        materialized.payload.init_artifacts[0].artifact_type_id,
        "metadata_doc"
    );
    validate_task_initialization(
        &state.tasks["task-source"].compiled_task,
        &materialized.payload,
    )
    .unwrap();
}

#[test]
fn upstream_artifact_materializes_by_task_type_and_schema() {
    let state = upstream_state(vec![artifact("artifact-metadata", "metadata_doc", 1)]);

    let materialized = materialize_task_initialization(&state, "task-downstream").unwrap();

    assert_eq!(materialized.payload.init_artifacts.len(), 1);
    assert_eq!(
        materialized.payload.init_artifacts[0].content,
        json!({
            "artifact_id": "artifact-metadata",
        })
    );
    assert_eq!(materialized.provenance.len(), 1);
}

#[test]
fn missing_upstream_artifact_reports_typed_diagnostic() {
    let state = upstream_state(vec![]);

    let error = materialize_task_initialization(&state, "task-downstream").unwrap_err();

    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == TaskInitializationDiagnosticCode::MissingUpstreamArtifact
    }));
}

#[test]
fn ambiguous_upstream_artifacts_report_typed_diagnostic() {
    let state = upstream_state(vec![
        artifact("artifact-a", "metadata_doc", 1),
        artifact("artifact-b", "metadata_doc", 1),
    ]);

    let error = materialize_task_initialization(&state, "task-downstream").unwrap_err();

    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == TaskInitializationDiagnosticCode::AmbiguousUpstreamArtifact
    }));
}

#[test]
fn stale_upstream_availability_reports_typed_diagnostic() {
    let mut state = upstream_state(vec![artifact("artifact-stale", "metadata_doc", 1)]);
    state
        .outcomes
        .get_mut("outcome-upstream")
        .unwrap()
        .artifact_records
        .clear();
    state.set_revision_and_hash(2);

    let error = materialize_task_initialization(&state, "task-downstream").unwrap_err();

    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == TaskInitializationDiagnosticCode::StaleUpstreamArtifact
    }));
}
