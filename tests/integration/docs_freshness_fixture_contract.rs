use crate::integration::outcome_evidence_support::{
    build_docs_task_success_evidence, DocsTaskSuccessEvidenceRequest,
};
use meld_execution::task_network::store::network_storage_key;
use meld_lang::{Effect, GoalLifecycle, Proposition, StepKind, Term};
use meld_world_model::belief::BeliefConfigLoader;

use super::docs_freshness_fixture::{
    DocsFreshnessFirstProofFixture, CONTENT_SOURCE_KIND, DIMENSION_ID, EVIDENCE_POLICY_ID,
    FAILURE_EVENT_TYPE, GOAL_ACCEPTED_SEQ, GOAL_COMMAND_REVISION_ID, METHOD_ID, PREDICATE_ID,
    PUBLICATION_EVENT_SEQ, PUBLICATION_EVENT_TYPE, PUBLICATION_ID, REQUIRED_ARTIFACT_TYPE_ID,
    SATISFACTION_REVIEW_SEQ, SATISFIED_REVISION_ID, SEED_GRAPH_SEQ, SESSION_ID, SUBJECT_DOMAIN_ID,
    SUBJECT_OBJECT_ID, SUBJECT_OBJECT_KIND, TASK_ARTIFACT_REPO_ID, TASK_NETWORK_ID, THRESHOLD,
    WORKER_ID,
};

#[test]
fn docs_freshness_fixture_pins_first_proof_identities() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let subject = fixture.subject();
    let belief_key = fixture.belief_key();
    let agent = fixture.seed_agent_registration();
    let rule = fixture.curation_rule_config();

    subject.validate().unwrap();
    belief_key.validate().unwrap();
    agent.validate().unwrap();
    rule.validate().unwrap();

    assert_eq!(subject.domain_id, SUBJECT_DOMAIN_ID);
    assert_eq!(subject.object_kind, SUBJECT_OBJECT_KIND);
    assert_eq!(subject.object_id, SUBJECT_OBJECT_ID);
    assert_eq!(belief_key.subject, subject);
    assert_eq!(belief_key.dimension_id, DIMENSION_ID);
    assert_eq!(belief_key.predicate_id, PREDICATE_ID);
    assert_eq!(belief_key.evidence_policy_id, EVIDENCE_POLICY_ID);
    assert_eq!(belief_key.branch_scope.branch_id, "main");
    assert_eq!(agent.agent_id, "seed.docs_freshness");
    assert_eq!(agent.subject, fixture.subject());
    assert_eq!(rule.dimension_id, DIMENSION_ID);
    assert_eq!(rule.threshold, THRESHOLD);
}

#[test]
fn docs_freshness_fixture_aligns_belief_curation_and_evidence_mapping() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let config = BeliefConfigLoader::load_json(fixture.belief_config_json()).unwrap();
    let rule = fixture.curation_rule_config();
    let event = fixture.task_success_event(2);
    let request = DocsTaskSuccessEvidenceRequest {
        event,
        subject: fixture.subject(),
        stale_probability: 0.0,
        review_probability: 0.2,
        source_kind: CONTENT_SOURCE_KIND.to_string(),
        required_artifact_type_id: Some(REQUIRED_ARTIFACT_TYPE_ID.to_string()),
    };

    assert_eq!(config.config.family_id, DIMENSION_ID);
    assert_eq!(config.config.dimension_id, DIMENSION_ID);
    assert_eq!(config.config.predicate_id, PREDICATE_ID);
    assert_eq!(config.config.evidence_policy_id, EVIDENCE_POLICY_ID);
    assert_eq!(config.config.planner_projection.threshold, THRESHOLD);
    assert_eq!(rule.threshold, config.config.planner_projection.threshold);
    assert!(config
        .config
        .source_mappings
        .iter()
        .any(|mapping| mapping.source_kind == CONTENT_SOURCE_KIND
            && mapping.value_field == "stale_probability"));
    assert!(config
        .config
        .source_mappings
        .iter()
        .any(|mapping| mapping.source_kind == CONTENT_SOURCE_KIND
            && mapping.value_field == "review_probability"));

    let promoted = build_docs_task_success_evidence(request).unwrap().unwrap();

    assert_eq!(promoted.source_kind, CONTENT_SOURCE_KIND);
    assert_eq!(
        promoted.source_id,
        DocsFreshnessFirstProofFixture::publication_record_id(PUBLICATION_ID)
    );
    assert_eq!(promoted.subject, fixture.subject());
}

#[test]
fn docs_freshness_fixture_method_requires_docs_patch_output() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let method = fixture.docs_method();

    assert_eq!(method.method_id, METHOD_ID);
    assert_eq!(method.composition.steps.len(), 1);

    let StepKind::Op(operator) = &method.composition.steps[0].kind else {
        panic!("expected operator step");
    };
    assert!(operator.resolution.requires_outputs.iter().any(|slot| {
        slot.artifact_type == Term::ArtifactType(REQUIRED_ARTIFACT_TYPE_ID.to_string())
            && slot.required
    }));
    assert!(operator.effects.iter().any(|effect| matches!(
        effect,
        Effect::Assert(Proposition::Exists { artifact_type, .. })
            if artifact_type == &Term::ArtifactType(REQUIRED_ARTIFACT_TYPE_ID.to_string())
    )));
}

#[test]
fn docs_freshness_fixture_pins_runtime_safe_record_derivations() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let success = fixture.task_success_event(3);
    let failure = fixture.task_failure_event(4);
    let record_id = DocsFreshnessFirstProofFixture::publication_record_id(PUBLICATION_ID);

    assert_eq!(
        network_storage_key(TASK_NETWORK_ID).unwrap(),
        TASK_NETWORK_ID
    );
    assert_eq!(
        fixture.task_stream_id(),
        "task_network::network-docs::task::task-alpha"
    );
    assert_eq!(
        record_id,
        concat!(
            "execution::task_network_publication::",
            "task-network-publication-9ec007895dc9863796b15c44ce3d11cae6e7c4d19abeb95b797aa53c05747e11"
        )
    );
    assert_eq!(
        DocsFreshnessFirstProofFixture::publication_record_id(PUBLICATION_ID),
        record_id
    );
    assert_eq!(success.envelope.session, SESSION_ID);
    assert_eq!(success.envelope.event_type, PUBLICATION_EVENT_TYPE);
    assert_eq!(
        success.envelope.record_id.as_deref(),
        Some(record_id.as_str())
    );
    assert_eq!(failure.envelope.event_type, FAILURE_EVENT_TYPE);
    assert_eq!(
        fixture.expected_final_lifecycle(fixture.satisfaction_review_seq()),
        GoalLifecycle::Satisfied {
            at_seq: SATISFACTION_REVIEW_SEQ
        }
    );
    assert_eq!(WORKER_ID, "worker-docs");
    assert_eq!(fixture.goal_acceptance_seq(), GOAL_ACCEPTED_SEQ);
    assert_eq!(fixture.publication_event_seq(), PUBLICATION_EVENT_SEQ);
    assert_eq!(fixture.satisfaction_review_seq(), SATISFACTION_REVIEW_SEQ);
    assert_eq!(SEED_GRAPH_SEQ, 1);
    assert_eq!(GOAL_COMMAND_REVISION_ID, "revision-a");
    assert_eq!(SATISFIED_REVISION_ID, "revision-satisfied");
    assert_eq!(TASK_ARTIFACT_REPO_ID, "repo-docs");
}

#[test]
fn docs_freshness_fixture_pins_goal_and_subscription_derivations() {
    let fixture = DocsFreshnessFirstProofFixture::new();

    assert_eq!(
        fixture.expected_goal_source_identity(),
        concat!(
            "seed.docs_freshness::workspace_fs::node::node-a::main::",
            "docs_freshness::{\"Above\":{\"Literal\":{\"Number\":0.7}}}::",
            "belief_divergence"
        )
    );
    assert_eq!(
        fixture.expected_goal_command_id(),
        "goal-command-d229a6514f8dd7ea"
    );
    assert_eq!(fixture.expected_goal_id(), "goal-b3aa98202b995b4e");
    assert_eq!(
        fixture.expected_satisfaction_mutation_command_id(),
        "goal-mutation-command-bd5892fc7aa2fecb"
    );
    assert_eq!(
        fixture.expected_subscription_id(),
        "subscription-538131bbf2536cfa"
    );
}
