use meld::task::{ArtifactLinkRelation, ArtifactProducerRef, ArtifactRecord, TaskArtifactRepo};
use serde_json::json;

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

#[test]
fn artifact_repo_returns_artifacts_for_output_slot() {
    let mut repo = TaskArtifactRepo::new("repo_docs_writer");
    repo.append_artifact(artifact(
        "artifact_a",
        "capinst_finalize_a",
        "readme_summary",
    ))
    .unwrap();
    repo.append_artifact(artifact(
        "artifact_b",
        "capinst_finalize_a",
        "readme_summary",
    ))
    .unwrap();
    repo.append_artifact(artifact(
        "artifact_c",
        "capinst_finalize_b",
        "readme_summary",
    ))
    .unwrap();

    let artifacts = repo.artifacts_for_output_slot("capinst_finalize_a", "readme_summary");

    assert_eq!(artifacts.len(), 2);
}

#[test]
fn artifact_repo_tracks_supersession_explicitly() {
    let mut repo = TaskArtifactRepo::new("repo_docs_writer");
    repo.append_artifact(artifact(
        "artifact_old",
        "capinst_finalize_a",
        "readme_summary",
    ))
    .unwrap();
    repo.append_artifact(artifact(
        "artifact_new",
        "capinst_finalize_a",
        "readme_summary",
    ))
    .unwrap();

    repo.mark_superseded("artifact_old", "artifact_new", "retry replacement")
        .unwrap();

    assert_eq!(repo.record().artifact_links.len(), 1);
    assert_eq!(
        repo.record().artifact_links[0].relation,
        ArtifactLinkRelation::Supersedes
    );
}
