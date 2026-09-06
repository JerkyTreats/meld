//! Docs freshness PDS evidence contract.
//!
//! Native Curation coverage is admitted under the installed Docs policy.
//! Task artifacts remain operational evidence and cannot establish freshness.

use meld_world_model::belief::outcome::interpretation::{
    ConfiguredOutcomeMappingSet, OutcomeMappingSetConfig,
};
use meld_world_model::belief::{
    BeliefConfigLoader, EvidenceValue, OutcomeEvidenceMapping, OutcomeMappingDisposition,
    OutcomeMappingInput,
};
use meld_world_model::events::{EventEnvelope, EventRecord};
use serde_json::{json, Value};

const FAMILY_JSON: &str =
    include_str!("../../../theory/docs_freshness/belief_family.docs_freshness.json");
const INTERPRETATION_JSON: &str =
    include_str!("../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json");
const MAPPING_ID: &str = "docs_freshness_outcome_interpretation_v1";

fn mapping() -> ConfiguredOutcomeMappingSet {
    let config: OutcomeMappingSetConfig = serde_json::from_str(INTERPRETATION_JSON).unwrap();
    ConfiguredOutcomeMappingSet::new(config).unwrap()
}

fn outcome(artifact_type: &str, stale_probability: f64) -> OutcomeMappingInput {
    let payload = json!({
        "outcome_id": "outcome-assessment",
        "task_instance_id": "task-assessment",
        "lifecycle_epoch": 0,
        "claim_id": "claim-assessment",
        "claim_revision": 1,
        "status": "Succeeded",
        "error": Value::Null,
        "artifact_records": [
            {
                "artifact_id": "artifact-publication-receipt",
                "artifact_type_id": "docs_publication_receipt",
                "schema_version": 1,
                "content": {
                    "published_readmes": 4
                }
            },
            {
                "artifact_id": "artifact-assessment",
                "artifact_type_id": artifact_type,
                "schema_version": 1,
                "content": {
                    "subject_id": "root",
                    "source_fingerprint": "source-hash",
                    "stale_probability": stale_probability,
                    "expected_readmes": 4,
                    "verified_readmes": 4,
                    "policy_identity": "policy-hash",
                    "validation_fingerprint": "validation-hash",
                    "weighted_groundedness": 0.96,
                    "unsupported_claim_mass": 0.0,
                    "contradiction_claim_mass": 0.0
                },
                "producer": {
                    "task_id": "task-assessment",
                    "capability_instance_id": "assess-published-docs-scope",
                    "invocation_id": "invocation-assessment",
                    "output_slot_id": "docs_freshness_assessment"
                }
            }
        ],
        "task_events": []
    });
    let envelope = EventEnvelope::with_now_domain(
        "session",
        "execution",
        "task-assessment",
        "execution.task.succeeded",
        None,
        payload,
    )
    .with_record_id("publication-assessment");
    OutcomeMappingInput {
        mapping_revision: None,
        record: EventRecord::from_envelope(envelope, 7),
        mapping_id: MAPPING_ID.to_string(),
    }
}

fn coverage(status: &str, seq: u64) -> OutcomeMappingInput {
    let envelope = EventEnvelope::with_now_domain("session", "curation", "coverage-operation", "world_model.curation.result.v1", None,
        json!({"disposition":"applied", "semantic_publication":{"batch":{"objects":[{"qualifications":{
            "coverage":status, "output_policy_revision":"docs-required-coverage-v1", "judgment_subject_domain":"workspace_fs",
            "judgment_subject_kind":"node", "judgment_subject_id":"root"
        }}]}}}))
        .with_record_id(format!("coverage-{status}-{seq}"));
    OutcomeMappingInput {
        mapping_revision: None,
        record: EventRecord::from_envelope(envelope, seq),
        mapping_id: MAPPING_ID.into(),
    }
}

#[test]
fn committed_pds_theory_parses_and_round_trips() {
    let family = BeliefConfigLoader::load_json(FAMILY_JSON).unwrap();
    assert_eq!(family.config.family_id, "docs_freshness");
    assert_eq!(family.config.default_prior, 0.0);
    assert_eq!(family.config.planner_projection.threshold, 0.7);
    let family_raw: Value = serde_json::from_str(FAMILY_JSON).unwrap();
    assert_eq!(serde_json::to_value(&family.config).unwrap(), family_raw);

    let config: OutcomeMappingSetConfig = serde_json::from_str(INTERPRETATION_JSON).unwrap();
    let mapping_raw: Value = serde_json::from_str(INTERPRETATION_JSON).unwrap();
    assert_eq!(serde_json::to_value(&config).unwrap(), mapping_raw);
    assert_eq!(mapping().mapping_id(), MAPPING_ID);
}

#[test]
fn native_coverage_promotes_its_exact_subject_and_disposition() {
    for (status, probability) in [("satisfied", 1.0), ("unsatisfied", 0.0)] {
        let OutcomeMappingDisposition::Applicable { record, .. } =
            mapping().map_outcome(&coverage(status, 7))
        else {
            panic!("native coverage should be applicable")
        };
        assert_eq!(record.source_kind, "docs_required_coverage");
        assert_eq!(record.subject.object_id, "root");
        assert_eq!(record.subject.domain_id, "workspace_fs");
        assert_eq!(
            record.fields.get("coverage_probability"),
            Some(&EvidenceValue::Scalar(probability))
        );
    }
}

#[test]
fn task_success_is_not_freshness_evidence() {
    for artifact_type in [
        "docs_validated_patch_set",
        "docs_publication_receipt",
        "docs_freshness_assessment",
    ] {
        let disposition = mapping().map_outcome(&outcome(artifact_type, 0.0));
        assert!(matches!(
            disposition,
            OutcomeMappingDisposition::NotApplicable { .. }
        ));
    }
}

#[test]
fn repeated_legacy_assessments_do_not_change_applicability() {
    let mut input = outcome("docs_freshness_assessment", 0.0);
    let duplicate = input.record.envelope().data["artifact_records"][1].clone();
    input.record.envelope_mut().data["artifact_records"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);

    assert!(matches!(
        mapping().map_outcome(&input),
        OutcomeMappingDisposition::NotApplicable { .. }
    ));
}

#[test]
fn shipped_comparator_replaces_coverage_without_accumulating_obsolete_confidence() {
    use meld_world_model::belief::{
        BayesianComparator, BeliefEvidenceNormalizer, BranchScope, ComparatorInput,
    };
    let snapshot = BeliefConfigLoader::load_json(FAMILY_JSON).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        meld_world_model::PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let mut prior = None;
    for (status, seq, expected) in [
        ("unsatisfied", 7, 0.0),
        ("satisfied", 8, 1.0),
        ("unsatisfied", 9, 0.0),
    ] {
        let OutcomeMappingDisposition::Applicable { record, .. } =
            mapping().map_outcome(&coverage(status, seq))
        else {
            panic!("native coverage should map")
        };
        let evidence = normalizer.normalize_promoted(&record).unwrap();
        let revision = BayesianComparator::assess(ComparatorInput {
            config: snapshot.config.clone(),
            config_snapshot_hash: snapshot.hash.clone(),
            prior_revision: prior,
            evidence,
            subject_key: None,
            source_cursor_start: seq,
            source_cursor_end: seq,
        })
        .unwrap()
        .revision;
        assert_eq!(revision.planner_projection.confidence, expected);
        prior = Some(revision);
    }
}
