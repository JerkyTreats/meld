use std::ffi::OsString;
use std::fs;
use std::path::Path;

use meld::agent::profile::prompt_contract::PromptContract;
use meld::cli::RunContext;
use meld::config::MerkleConfig;
use meld::context::belief_context::{hydrate_belief_context_bundle, BELIEF_CONTEXT_FAMILY_ID};
use meld::context::generation::contracts::GenerationOrchestrationRequest;
use meld::context::generation::prompt_collection::build_prompt_messages_with_belief;
use meld::events::DomainObjectRef;
use meld::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
use meld::runtime::storage::ProductStorageLayout;
use meld::store::{NodeRecord, NodeRecordStore, NodeType, SledNodeRecordStore};
use meld::types::NodeID;
use meld::world_state::belief::{
    BeliefKey, BeliefProvenanceSummary, BeliefStatus, BeliefStore, BeliefView, BranchScope,
    ContradictionState, FreshnessState, HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld::world_state::PerspectiveKey;
use meld_execution::{BeliefContextReadPort, BeliefStatusLabel};

struct EnvironmentRestore {
    values: Vec<(OsString, Option<OsString>)>,
}

impl EnvironmentRestore {
    fn isolate(root: &Path) -> Self {
        let mut keys: Vec<OsString> = ["HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME"]
            .into_iter()
            .map(OsString::from)
            .collect();
        keys.extend(
            std::env::vars_os()
                .map(|(key, _)| key)
                .filter(|key| key.to_string_lossy().starts_with("MERKLE")),
        );
        keys.sort();
        keys.dedup();
        let values = keys
            .iter()
            .map(|key| (key.clone(), std::env::var_os(key)))
            .collect();
        for key in keys
            .iter()
            .filter(|key| key.to_string_lossy().starts_with("MERKLE"))
        {
            std::env::remove_var(key);
        }
        let home = root.join("home");
        let config = root.join("config");
        let data = root.join("data");
        for path in [&home, &config, &data] {
            fs::create_dir_all(path).unwrap();
        }
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", config);
        std::env::set_var("XDG_DATA_HOME", data);
        Self { values }
    }
}

impl Drop for EnvironmentRestore {
    fn drop(&mut self) {
        for (key, value) in self.values.drain(..) {
            if let Some(value) = value {
                std::env::set_var(&key, value);
            } else {
                std::env::remove_var(&key);
            }
        }
    }
}

#[test]
fn cli_assembly_cuts_over_legacy_belief_for_context_and_generation_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let _environment = EnvironmentRestore::isolate(temp.path());
    let workspace = temp.path().join("workspace");
    let external = temp.path().join("external");
    let legacy_path = external.join("legacy.sled");
    let product_root = external.join("product");
    let frames_path = external.join("frames");
    let artifacts_path = external.join("artifacts");
    let workflows_path = external.join("workflows");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&external).unwrap();
    fs::write(workspace.join("README.md"), "content").unwrap();

    let mut config = MerkleConfig::default();
    config.system.storage.store_path = legacy_path.clone();
    config.system.storage.frames_path = frames_path;
    config.system.storage.artifacts_path = artifacts_path;
    config.system.storage.product_root = Some(product_root.clone());
    config.workflows.user_profile_dir = Some(workflows_path);
    let config_path = external.join("meld.toml");
    fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();

    let node_id: NodeID = [0x2a; 32];
    let subject_id = hex::encode(node_id);
    let legacy_view = belief_view(
        &subject_id,
        "legacy-revision",
        0.82,
        41,
        BeliefStatus::Stale,
        true,
        true,
    );
    seed_legacy_state(&legacy_path, &workspace, node_id, &legacy_view);

    let first = RunContext::new(workspace.clone(), Some(config_path.clone())).unwrap();
    let signal = first
        .api()
        .current_belief_signal(&subject_id, BELIEF_CONTEXT_FAMILY_ID)
        .unwrap()
        .unwrap();
    assert_eq!(signal.status, BeliefStatusLabel::Stale);
    assert_eq!(signal.confidence, 0.82);
    assert!(signal.stale);
    assert!(signal.contradicted);
    assert_eq!(
        signal.contradicted_evidence_ids,
        vec!["contradicted-legacy-revision"]
    );
    assert_eq!(signal.as_of_seq, 41);
    assert_eq!(signal.revision_id.as_deref(), Some("legacy-revision"));
    assert_eq!(signal.evidence_ids, vec!["hydrated-legacy-revision"]);
    assert_eq!(signal.source_fact_ids, vec!["fact-41"]);

    let bundle = hydrate_belief_context_bundle(first.api(), node_id, "README.md").unwrap();
    let assertion = bundle.assertion_for_subject(&subject_id).unwrap();
    assert_eq!(bundle.family_id, BELIEF_CONTEXT_FAMILY_ID);
    assert_eq!(bundle.as_of_seq, 41);
    assert_eq!(assertion.revision_id.as_deref(), Some("legacy-revision"));
    assert_eq!(assertion.status, BeliefStatusLabel::Stale);
    assert_eq!(assertion.confidence, 0.82);
    assert!(assertion.stale);
    assert!(assertion.contradicted);
    assert_eq!(
        assertion.contradicted_claims,
        vec!["contradicted-legacy-revision"]
    );
    assert_eq!(assertion.evidence_refs, vec!["hydrated-legacy-revision"]);
    assert_eq!(assertion.source_fact_refs, vec!["fact-41"]);
    assert_eq!(assertion.as_of_seq, 41);

    let node_record = first.api().node_store().get(&node_id).unwrap().unwrap();
    let prompt = build_prompt_messages_with_belief(
        first.api(),
        &generation_request(node_id),
        &node_record,
        &prompt_contract(),
        Some(&bundle),
    )
    .unwrap();
    assert!(prompt.context_payload.contains("Belief Context"));
    assert!(prompt
        .context_payload
        .contains("status=Stale, confidence=0.82 (as of sequence 41), stale"));
    assert!(prompt
        .context_payload
        .contains("contradicted-legacy-revision (unresolved)"));
    assert!(prompt.context_payload.contains("legacy-revision"));
    assert!(prompt.context_payload.contains("hydrated-legacy-revision"));
    assert!(prompt.context_payload.contains("fact-41"));
    drop(first);

    let legacy = open_belief(&legacy_path);
    let fenced_error = legacy
        .put_view(&belief_view(
            &subject_id,
            "legacy-after-cutover",
            0.1,
            42,
            BeliefStatus::Settled,
            false,
            false,
        ))
        .unwrap_err();
    assert!(fenced_error
        .to_string()
        .contains("legacy belief authority is frozen"));
    drop(legacy);

    let product_layout = ProductStorageLayout::from_root(&product_root);
    let product = open_belief(&product_layout.world_model_db);
    product
        .put_view(&belief_view(
            &subject_id,
            "product-revision",
            0.93,
            52,
            BeliefStatus::Settled,
            false,
            false,
        ))
        .unwrap();
    product.flush().unwrap();
    drop(product);

    let reopened = RunContext::new(workspace, Some(config_path)).unwrap();
    let reopened_signal = reopened
        .api()
        .current_belief_signal(&subject_id, BELIEF_CONTEXT_FAMILY_ID)
        .unwrap()
        .unwrap();
    assert_eq!(reopened_signal.confidence, 0.93);
    assert_eq!(reopened_signal.as_of_seq, 52);
    assert_eq!(
        reopened_signal.revision_id.as_deref(),
        Some("product-revision")
    );
    let reopened_bundle =
        hydrate_belief_context_bundle(reopened.api(), node_id, "README.md").unwrap();
    let reopened_assertion = reopened_bundle.assertion_for_subject(&subject_id).unwrap();
    assert_eq!(reopened_bundle.as_of_seq, 52);
    assert_eq!(
        reopened_assertion.revision_id.as_deref(),
        Some("product-revision")
    );
    assert_eq!(reopened_assertion.confidence, 0.93);
}

fn seed_legacy_state(legacy_path: &Path, workspace: &Path, node_id: NodeID, view: &BeliefView) {
    fs::create_dir_all(legacy_path).unwrap();
    let database = sled::open(legacy_path).unwrap();
    let nodes = SledNodeRecordStore::from_db(database.clone());
    nodes
        .put(&NodeRecord {
            node_id,
            path: workspace.join("README.md"),
            node_type: NodeType::File {
                size: 7,
                content_hash: [0x11; 32],
            },
            children: Vec::new(),
            parent: None,
            frame_set_root: None,
            metadata: Default::default(),
            tombstoned_at: None,
        })
        .unwrap();
    let beliefs = BeliefStore::new(database).unwrap();
    beliefs.put_view(view).unwrap();
    nodes.flush().unwrap();
    beliefs.flush().unwrap();
}

fn open_belief(path: &Path) -> BeliefStore {
    BeliefStore::new(sled::open(path).unwrap()).unwrap()
}

fn belief_view(
    subject_id: &str,
    revision_id: &str,
    confidence: f64,
    sequence: u64,
    status: BeliefStatus,
    stale: bool,
    contradicted: bool,
) -> BeliefView {
    let subject = DomainObjectRef::new("workspace_fs", "node", subject_id).unwrap();
    BeliefView {
        view_id: format!("view-{revision_id}"),
        key: BeliefKey {
            subject: subject.clone(),
            dimension_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
            predicate_id: "confidence".to_string(),
            perspective: PerspectiveKey::new("default", "default").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "default_policy".to_string(),
        },
        current_revision_id: Some(revision_id.to_string()),
        status,
        posterior: PosteriorSummary {
            probability: confidence,
            meaning: "stale_probability".to_string(),
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: "confidence".to_string(),
            confidence,
            threshold: 0.7,
        },
        uncertainty: 1.0 - confidence,
        precision: 1.0,
        freshness: FreshnessState {
            stale,
            reasons: Vec::new(),
            high_water_seq: sequence,
        },
        contradiction: ContradictionState {
            contradicted,
            reasons: Vec::new(),
            supporting_evidence_ids: vec![format!("supporting-{revision_id}")],
            contradicted_evidence_ids: if contradicted {
                vec![format!("contradicted-{revision_id}")]
            } else {
                Vec::new()
            },
        },
        observation: None,
        assessment_state: "complete".to_string(),
        advisory_posture: "ready".to_string(),
        provenance: BeliefProvenanceSummary::empty(),
        hydration: HydrationRefs {
            evidence_ids: vec![format!("hydrated-{revision_id}")],
            source_fact_ids: vec![format!("fact-{sequence}")],
            graph_anchor_ids: Vec::new(),
            revision_id: Some(revision_id.to_string()),
        },
    }
}

fn generation_request(node_id: NodeID) -> GenerationOrchestrationRequest {
    GenerationOrchestrationRequest {
        request_id: 1,
        node_id,
        agent_id: "test-agent".to_string(),
        provider: ProviderExecutionBinding::new(
            "test-provider",
            ProviderRuntimeOverrides::default(),
        )
        .unwrap(),
        frame_type: "context-test".to_string(),
        retry_count: 0,
        force: true,
    }
}

fn prompt_contract() -> PromptContract {
    PromptContract {
        system_prompt: "System prompt".to_string(),
        user_prompt_file: "Summarize {path}".to_string(),
        user_prompt_directory: "Summarize {path}".to_string(),
    }
}
