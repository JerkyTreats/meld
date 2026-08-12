//! Docs freshness PDS evidence contract.
//!
//! Only the final exact-byte assessment artifact is admissible as fresh
//! evidence. Intermediate task success and publication are operational facts,
//! not freshness facts.

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
                    "verified_readmes": 4
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
        record: EventRecord::from_envelope(envelope, 7),
        mapping_id: MAPPING_ID.to_string(),
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
fn exact_byte_assessment_promotes_its_measured_probability() {
    let OutcomeMappingDisposition::Applicable { record, .. } =
        mapping().map_outcome(&outcome("docs_freshness_assessment", 0.0))
    else {
        panic!("assessment should be applicable");
    };
    assert_eq!(record.source_kind, "docs_freshness_assessment");
    assert_eq!(record.subject.object_id, "root");
    assert_eq!(
        record.fields.get("stale_probability"),
        Some(&EvidenceValue::Scalar(0.0))
    );
    assert_eq!(
        record.fields.get("source_fingerprint"),
        Some(&EvidenceValue::Text("source-hash".to_string()))
    );
}

#[test]
fn intermediate_task_success_is_not_freshness_evidence() {
    let disposition = mapping().map_outcome(&outcome("docs_publication_receipt", 0.0));
    assert!(matches!(
        disposition,
        OutcomeMappingDisposition::NotApplicable { .. }
    ));
}

#[test]
fn duplicate_selected_artifacts_are_rejected_as_ambiguous() {
    let mut input = outcome("docs_freshness_assessment", 0.0);
    let duplicate = input.record.envelope().data["artifact_records"][1].clone();
    input.record.envelope_mut().data["artifact_records"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);

    assert!(matches!(
        mapping().map_outcome(&input),
        OutcomeMappingDisposition::Invalid { .. }
    ));
}

#[test]
fn shipped_probability_math_crosses_only_after_assessment() {
    let prior = 0.0;
    let after_unobserved = (prior + 1.0) / 2.0;
    let after_fresh_assessment = (after_unobserved + 0.0) / 2.0;
    let unobserved_confidence = 1.0 - after_unobserved;
    let assessed_confidence = 1.0 - after_fresh_assessment;

    assert!(unobserved_confidence < 0.7);
    assert!(assessed_confidence >= 0.7);
    assert_eq!(after_fresh_assessment, 0.25);
}
