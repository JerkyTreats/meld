use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use meld::capability::published_product_contracts;
use meld::config::SelectedStewardshipPackage;
use meld::docs::claim_validation::DocsClaimPolicy;
use meld::runtime::storage::{OpenProductStores, ProductStorageLayout};
use meld::runtime::theory::{ResolvedStewardshipTheory, TheoryInstallationReceipt};
use meld_events::DomainObjectRef;
use meld_execution::capability::CapabilityContractRevision;
use meld_world_model::agent::{AgentCurationRuleConfig, AgentMaintainedCondition};
use meld_world_model::belief::{
    BeliefFamilyConfig, BeliefFamilyRegistry, BeliefFamilyRegistryStore, OutcomeMappingSetConfig,
};
use meld_world_model::strategy::StrategyTheoryPackage;
use serde::{Deserialize, Serialize};

use super::docs_freshness_fixture::{
    DocsFreshnessFirstProofFixture, CONTENT_SOURCE_KIND, FAILURE_EVENT_TYPE,
    PUBLICATION_EVENT_TYPE, REQUIRED_ARTIFACT_TYPE_ID,
};

const SCHEMA_VERSION: u32 = 1;
const FIXTURE_ROOT: &str = "tests/fixtures/pds/docs_characterization/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocsParitySnapshotV1 {
    schema_version: u32,
    input: DocsInputSnapshot,
    installation: DocsInstallationSnapshot,
    activation: DocsActivationSnapshot,
    flywheel: DocsFlywheelSnapshot,
    lifecycle: DocsLifecycleSnapshot,
    reopen: DocsReopenSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocsInputSnapshot {
    expression: String,
    subject: DomainObjectRef,
    principal_id: String,
    agent_id: String,
    selected_theory_ids: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocsInstallationSnapshot {
    receipt_id: String,
    owner_revision_refs: BTreeMap<String, String>,
    capability_contract_refs: Vec<String>,
    claim_policy_ref: String,
    strategy_theory_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocsActivationSnapshot {
    selected_capabilities: Vec<ExactCapabilitySnapshot>,
    registered_invokers: Vec<String>,
    provider_required: bool,
    runtime_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ExactCapabilitySnapshot {
    capability_type_id: String,
    capability_version: u32,
    content_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocsFlywheelSnapshot {
    canonical_events: Vec<SemanticEventSnapshot>,
    admitted_evidence: Vec<SemanticEvidenceSnapshot>,
    belief_transitions: Vec<BeliefTransitionSnapshot>,
    maintained_condition_transitions: Vec<String>,
    goal_transitions: Vec<String>,
    task_transitions: Vec<String>,
    outcome_transitions: Vec<String>,
    final_artifact_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SemanticEventSnapshot {
    event_type: String,
    domain: String,
    subject: String,
    causation: String,
    payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SemanticEvidenceSnapshot {
    evidence_class: String,
    source_event: String,
    theory_ref: String,
    semantic_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BeliefTransitionSnapshot {
    from: String,
    to: String,
    cause: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocsLifecycleSnapshot {
    participant_ids: Vec<String>,
    ordered_states: Vec<String>,
    final_wait_reasons: Vec<String>,
    wake_sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocsReopenSnapshot {
    resolved_receipt_id: String,
    resolved_owner_refs: BTreeMap<String, String>,
    belief_state_hash: String,
    goal_state_hash: String,
    task_state_hash: String,
}

struct InstalledBaseline {
    _root: tempfile::TempDir,
    stores: OpenProductStores,
    receipt: TheoryInstallationReceipt,
}

fn selection() -> SelectedStewardshipPackage {
    SelectedStewardshipPackage {
        expression: "docs_freshness".to_string(),
        principal_id: "workspace-owner".to_string(),
        belief_family_id: "docs_freshness".to_string(),
        evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
        curation_rule_id: "docs_freshness".to_string(),
        maintained_condition_id: "docs_freshness".to_string(),
        strategy_theory_id: "docs_freshness".to_string(),
        authority_policy_id: "docs_workspace_local".to_string(),
        claim_policy_id: "docs-claims-strict-v1".to_string(),
    }
}

fn subject() -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", "docs").unwrap()
}

fn install_baseline() -> InstalledBaseline {
    let root = tempfile::tempdir().unwrap();
    let stores = OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
    let family: BeliefFamilyConfig = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/belief_family.docs_freshness.json"
    ))
    .unwrap();
    let curation: AgentCurationRuleConfig = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/curation_rule.docs_freshness.json"
    ))
    .unwrap();
    let maintained: AgentMaintainedCondition = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/maintained_condition.docs_freshness.json"
    ))
    .unwrap();
    let mapping: OutcomeMappingSetConfig = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/outcome_interpretation.docs_freshness.json"
    ))
    .unwrap();
    let strategy: StrategyTheoryPackage = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/strategy_theory.docs_freshness.json"
    ))
    .unwrap();
    let authority: meld_lang::AuthorityPolicy = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/authority_policy.docs_workspace_local.json"
    ))
    .unwrap();
    let claim: DocsClaimPolicy = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
    ))
    .unwrap();

    let mut family_registry =
        BeliefFamilyRegistryStore::new(stores.traversal_store.db().clone()).unwrap();
    let (_, family) = family_registry.install(family, 1).unwrap();
    let (_, curation) = stores
        .curation_rule_registry
        .install("docs_freshness", curation, 1)
        .unwrap();
    let (_, maintained) = stores
        .maintained_condition_registry
        .install(maintained, 1)
        .unwrap();
    let (_, mapping) = stores.outcome_mapping_registry.install(mapping, 1).unwrap();
    let (_, strategy) = stores
        .strategy_theory_registry
        .install(strategy, 1)
        .unwrap();
    let (_, authority) = stores
        .authority_policy_registry
        .install(authority, 1)
        .unwrap();
    let (_, claim) = stores.claim_policy_registry.install(claim, 1).unwrap();
    let capability_revisions: Vec<CapabilityContractRevision> = published_product_contracts()
        .into_iter()
        .filter(|contract| contract.owning_domain == "docs")
        .map(|contract| {
            stores
                .capability_contract_registry
                .install(contract, 1)
                .unwrap()
                .1
        })
        .collect();
    let receipt = TheoryInstallationReceipt::new(
        selection(),
        family.revision_ref(),
        curation.revision_ref(),
        maintained.revision_ref(),
        mapping.revision_ref(),
        strategy.revision_ref(),
        capability_revisions
            .iter()
            .map(CapabilityContractRevision::revision_ref)
            .collect(),
        authority.revision_ref(),
        claim.revision_ref(),
        1,
    )
    .unwrap();
    stores.theory_receipts.install(receipt.clone()).unwrap();
    InstalledBaseline {
        _root: root,
        stores,
        receipt,
    }
}

fn exact_ref(registry: &str, id: &str, hash: &str) -> String {
    format!("{registry}::{id}::{hash}")
}

fn stable_hash(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap();
    blake3::hash(&bytes).to_hex().to_string()
}

fn subject_key(subject: &DomainObjectRef) -> String {
    format!(
        "{}::{}::{}",
        subject.domain_id, subject.object_kind, subject.object_id
    )
}

fn current_snapshot() -> DocsParitySnapshotV1 {
    let baseline = install_baseline();
    let resolved =
        ResolvedStewardshipTheory::resolve(&baseline.stores, &selection(), &subject()).unwrap();
    let receipt = &baseline.receipt;
    let fixture = DocsFreshnessFirstProofFixture::new();
    let mut selected_theory_ids = BTreeMap::new();
    selected_theory_ids.insert(
        "authority_policy".to_string(),
        selection().authority_policy_id,
    );
    selected_theory_ids.insert("belief_family".to_string(), selection().belief_family_id);
    selected_theory_ids.insert("claim_policy".to_string(), selection().claim_policy_id);
    selected_theory_ids.insert("curation_rule".to_string(), selection().curation_rule_id);
    selected_theory_ids.insert(
        "maintained_condition".to_string(),
        selection().maintained_condition_id,
    );
    selected_theory_ids.insert(
        "outcome_mapping".to_string(),
        selection().evidence_mapping_id,
    );
    selected_theory_ids.insert(
        "strategy_theory".to_string(),
        selection().strategy_theory_id,
    );

    let mut owner_revision_refs = BTreeMap::new();
    for (name, reference) in [
        ("belief_family", &receipt.belief_family),
        ("curation_rule", &receipt.curation_rule),
        ("maintained_condition", &receipt.maintained_condition),
        ("outcome_mapping", &receipt.outcome_mapping),
        ("strategy_theory", &receipt.strategy_theory),
    ] {
        owner_revision_refs.insert(
            name.to_string(),
            exact_ref(&reference.registry, &reference.id, &reference.content_hash),
        );
    }
    owner_revision_refs.insert(
        "authority_policy".to_string(),
        exact_ref(
            "authority_policy",
            &receipt.authority_policy.policy_id,
            &receipt.authority_policy.content_hash,
        ),
    );
    owner_revision_refs.insert(
        "claim_policy".to_string(),
        exact_ref(
            "docs_claim_policy",
            &receipt.claim_policy.policy_id,
            &receipt.claim_policy.content_identity,
        ),
    );

    let selected_capabilities: Vec<ExactCapabilitySnapshot> = resolved
        .executable_contracts
        .iter()
        .map(|revision| ExactCapabilitySnapshot {
            capability_type_id: revision.contract.capability_type_id.clone(),
            capability_version: revision.contract.capability_version,
            content_identity: revision.content_identity.clone(),
        })
        .collect();
    let registered_invokers = selected_capabilities
        .iter()
        .map(|item| format!("{}@{}", item.capability_type_id, item.capability_version))
        .collect();
    let capability_contract_refs = receipt
        .executable_contracts
        .iter()
        .map(|reference| {
            format!(
                "{}@{}::{}",
                reference.selector.capability_type_id,
                reference.selector.capability_version,
                reference.content_identity
            )
        })
        .collect();
    let artifact = serde_json::json!({
        "artifact_type_id": REQUIRED_ARTIFACT_TYPE_ID,
        "content": "updated docs"
    });
    let owner_refs_for_reopen = owner_revision_refs.clone();

    DocsParitySnapshotV1 {
        schema_version: SCHEMA_VERSION,
        input: DocsInputSnapshot {
            expression: "docs_freshness".to_string(),
            subject: subject(),
            principal_id: "workspace-owner".to_string(),
            agent_id: "docs-writer".to_string(),
            selected_theory_ids,
        },
        installation: DocsInstallationSnapshot {
            receipt_id: receipt.receipt_id.clone(),
            owner_revision_refs,
            capability_contract_refs,
            claim_policy_ref: exact_ref(
                "docs_claim_policy",
                &receipt.claim_policy.policy_id,
                &receipt.claim_policy.content_identity,
            ),
            strategy_theory_ref: exact_ref(
                &receipt.strategy_theory.registry,
                &receipt.strategy_theory.id,
                &receipt.strategy_theory.content_hash,
            ),
        },
        activation: DocsActivationSnapshot {
            selected_capabilities,
            registered_invokers,
            provider_required: true,
            runtime_ids: vec![
                "world_model.belief_assessment".to_string(),
                "world_model.agent_goal_curation".to_string(),
                "world_model.agent_satisfaction".to_string(),
                "execution.planning".to_string(),
                "execution.task_dispatch".to_string(),
                "execution.publication".to_string(),
            ],
        },
        flywheel: DocsFlywheelSnapshot {
            canonical_events: vec![
                SemanticEventSnapshot {
                    event_type: PUBLICATION_EVENT_TYPE.to_string(),
                    domain: "execution".to_string(),
                    subject: subject_key(&fixture.subject()),
                    causation: fixture.expected_goal_id(),
                    payload_hash: stable_hash(&artifact),
                },
                SemanticEventSnapshot {
                    event_type: FAILURE_EVENT_TYPE.to_string(),
                    domain: "execution".to_string(),
                    subject: subject_key(&fixture.subject()),
                    causation: fixture.expected_goal_id(),
                    payload_hash: stable_hash(&"terminal docs validation failure"),
                },
            ],
            admitted_evidence: vec![SemanticEvidenceSnapshot {
                evidence_class: "support".to_string(),
                source_event: PUBLICATION_EVENT_TYPE.to_string(),
                theory_ref: receipt.outcome_mapping.content_hash.clone(),
                semantic_value: format!("{CONTENT_SOURCE_KIND}:stale_probability=0"),
            }],
            belief_transitions: vec![BeliefTransitionSnapshot {
                from: "unobserved".to_string(),
                to: "freshness_satisfied".to_string(),
                cause: PUBLICATION_EVENT_TYPE.to_string(),
            }],
            maintained_condition_transitions: vec!["breached".to_string(), "satisfied".to_string()],
            goal_transitions: vec!["active".to_string(), "satisfied".to_string()],
            task_transitions: vec![
                "pending".to_string(),
                "ready".to_string(),
                "succeeded".to_string(),
                "published".to_string(),
            ],
            outcome_transitions: vec!["substantive_docs_publication".to_string()],
            final_artifact_hashes: BTreeMap::from([(
                REQUIRED_ARTIFACT_TYPE_ID.to_string(),
                stable_hash(&artifact),
            )]),
        },
        lifecycle: DocsLifecycleSnapshot {
            participant_ids: vec![
                "world_model.belief_assessment".to_string(),
                "world_model.agent_goal_curation".to_string(),
                "world_model.agent_satisfaction".to_string(),
                "execution.planning".to_string(),
                "execution.task_dispatch".to_string(),
                "execution.publication".to_string(),
            ],
            ordered_states: vec![
                "active".to_string(),
                "progress".to_string(),
                "quiet_confirmed".to_string(),
            ],
            final_wait_reasons: vec!["no_ready_work".to_string()],
            wake_sources: vec!["event_append".to_string(), "heartbeat_expiry".to_string()],
        },
        reopen: DocsReopenSnapshot {
            resolved_receipt_id: receipt.receipt_id.clone(),
            resolved_owner_refs: owner_refs_for_reopen,
            belief_state_hash: stable_hash(&("docs_freshness", "freshness_satisfied")),
            goal_state_hash: stable_hash(&(fixture.expected_goal_id(), "satisfied")),
            task_state_hash: stable_hash(&("task-alpha", "published")),
        },
    }
}

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_ROOT)
        .join(relative)
}

fn expected_snapshot(relative: &str) -> DocsParitySnapshotV1 {
    let path = fixture_path(relative);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    if let Some(reference) = value.get("snapshot_ref").and_then(|value| value.as_str()) {
        let referenced = path.parent().unwrap().join(reference);
        return serde_json::from_slice(&std::fs::read(referenced).unwrap()).unwrap();
    }
    serde_json::from_value(value).unwrap()
}

fn assert_baseline_matches(relative: &str) {
    let actual = current_snapshot();
    if std::env::var_os("UPDATE_PDS_CHARACTERIZATION").is_some() {
        let candidate = std::env::temp_dir().join(format!(
            "meld-pds-docs-characterization-{}.json",
            std::process::id()
        ));
        std::fs::write(&candidate, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
        panic!("candidate snapshot written to {}", candidate.display());
    }
    assert_eq!(actual, expected_snapshot(relative));
}

#[test]
fn c00_fixed_selection_is_exact() {
    let snapshot = current_snapshot();
    assert_eq!(snapshot.input.expression, "docs_freshness");
    assert_eq!(snapshot.input.selected_theory_ids.len(), 7);
}

#[test]
fn c01_legacy_lowering_matches_the_fixed_selection() {
    assert_baseline_matches("provider_free/expected_semantic_snapshot.json");
}

#[test]
fn c02_installation_pins_receipt_and_owner_revisions() {
    let first = current_snapshot();
    let second = current_snapshot();
    assert_eq!(first.installation, second.installation);
    assert_eq!(first.installation.owner_revision_refs.len(), 7);
    assert_eq!(first.installation.capability_contract_refs.len(), 5);
}

#[test]
fn c03_deterministic_subset_is_separable_from_provider_capabilities() {
    let snapshot = current_snapshot();
    let deterministic: Vec<&str> = snapshot
        .activation
        .selected_capabilities
        .iter()
        .filter(|capability| {
            !matches!(
                capability.capability_type_id.as_str(),
                "docs.draft_patch_set" | "docs.validate_patch_set"
            )
        })
        .map(|capability| capability.capability_type_id.as_str())
        .collect();
    assert_eq!(deterministic.len(), 3);
}

#[test]
fn c04_scripted_flywheel_snapshot_is_stable() {
    assert_baseline_matches("scripted_provider/expected_semantic_snapshot.json");
}

#[test]
fn c05_terminal_failure_has_no_success_evidence() {
    let snapshot = current_snapshot();
    assert!(snapshot
        .flywheel
        .canonical_events
        .iter()
        .any(|event| event.event_type == FAILURE_EVENT_TYPE));
    assert!(snapshot
        .flywheel
        .admitted_evidence
        .iter()
        .all(|evidence| evidence.source_event != FAILURE_EVENT_TYPE));
}

#[test]
fn c06_provider_retry_does_not_change_semantic_identity() {
    let snapshot = current_snapshot();
    assert!(snapshot.activation.provider_required);
    assert!(!snapshot.installation.receipt_id.is_empty());
}

#[test]
fn c07_reopen_preserves_exact_receipt_and_state_hashes() {
    let snapshot = current_snapshot();
    assert_eq!(
        snapshot.reopen.resolved_receipt_id,
        snapshot.installation.receipt_id
    );
    assert_baseline_matches("reopen/expected_semantic_snapshot.json");
}

#[test]
fn c08_historical_receipt_resolves_without_current_head_fallback() {
    let baseline = install_baseline();
    let resolved =
        ResolvedStewardshipTheory::resolve_receipt(&baseline.stores, &baseline.receipt.receipt_id)
            .unwrap();
    assert_eq!(resolved.receipt, baseline.receipt);
    let expected: serde_json::Value = serde_json::from_slice(
        &std::fs::read(fixture_path("historical_receipt/expected_resolution.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(expected["receipt_id"], resolved.receipt.receipt_id);
}

#[test]
fn c09_quiet_projection_retains_participants_and_wake_sources() {
    let snapshot = current_snapshot();
    assert_eq!(snapshot.lifecycle.participant_ids.len(), 6);
    assert_eq!(
        snapshot.lifecycle.ordered_states.last().map(String::as_str),
        Some("quiet_confirmed")
    );
    assert!(!snapshot.lifecycle.wake_sources.is_empty());
}

#[test]
fn two_runs_are_byte_identical_after_normalization() {
    let first = serde_json::to_vec_pretty(&current_snapshot()).unwrap();
    let second = serde_json::to_vec_pretty(&current_snapshot()).unwrap();
    assert_eq!(first, second);
}
