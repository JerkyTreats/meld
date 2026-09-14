//! Shared survey-configuration fixture: the stewardship composition and
//! frozen specimen theory body that the stall and served-surface tests use
//! to reproduce the live survey.

use std::path::Path;

use meld::config::{PhysicalBinding, SelectedStewardshipPackage};
use meld::harness::boot::{HarnessBootRequest, HarnessRootSelection};
use meld::runtime::assembly::StewardshipComposition;
use meld::runtime::contracts::{
    RuntimeActionRecord, RuntimeStatusPublisher, WaitingOnDeclaration, WorkerCheckpoint,
    WorkerScope, WorkerTickReport,
};
use meld::runtime::registration::{RegistrationKind, RegistrationSet, RuntimeRegistration};
use meld::runtime::storage::{OpenProductStores, ProductStorageLayout};
use meld::runtime::supervisor::SupervisorReportStore;
use meld_world_model::belief::{BeliefFamilyRegistry, BeliefFamilyRegistryStore};

pub const SUBJECT_ID: &str = "docs";
pub const AGENT_ID: &str = "seed.docs_freshness";
pub const FAMILY_ID: &str = "docs_freshness";

/// Publish a retained historical wait for report presentation tests.
///
/// This fixture exercises the report reader and its projections. It is not
/// evidence that the current runtime composed or invoked a Belief actor.
pub fn publish_historical_belief_wait_fixture(
    supervisor_store: &meld::runtime::supervisor::SupervisorStore,
    observed_at_ms: u64,
) {
    let report = WorkerTickReport {
        actor_id: "world_model.belief_assessment".to_string(),
        scope: WorkerScope {
            domain_id: "world_model".to_string(),
            stream_id: None,
            work_key: Some("belief_assessment".to_string()),
            agent_id: None,
            perspective_key: None,
            branch_id: None,
            subject_key: Some(format!("workspace_fs::node::{SUBJECT_ID}")),
        },
        input_checkpoint: WorkerCheckpoint {
            name: "belief_assessment_checkpoint".to_string(),
            value: 0,
        },
        output_checkpoint: WorkerCheckpoint {
            name: "belief_assessment_checkpoint".to_string(),
            value: 0,
        },
        items_attempted: 0,
        items_committed: 0,
        retryable_errors: Vec::new(),
        fatal_errors: Vec::new(),
        budget_exhausted: false,
        waiting_on: vec![WaitingOnDeclaration {
            condition: "belief_work_ineligible".to_string(),
            subject_key: Some(format!("workspace_fs::node::{SUBJECT_ID}")),
            detail: "required evidence family has no admitted observation".to_string(),
            wake_refs: Vec::new(),
        }],
    };
    let action = RuntimeActionRecord::from_worker_tick(
        "survey-belief-wait",
        "world_model.belief_assessment",
        observed_at_ms,
        report,
    );
    SupervisorReportStore::open(supervisor_store)
        .unwrap()
        .publish_action(&action)
        .unwrap();
}

/// The frozen specimen theory body.
///
/// This is the shipped docs freshness family as it stood at the 2026-07-25
/// live survey — anchor-required with no graph_anchor mapping — preserved
/// verbatim so the anchor stall remains reproducible as the harness
/// validation specimen. The product body has since ignited and diverged;
/// the specimen deliberately has not.
pub fn specimen_family_json() -> &'static str {
    r#"{
  "family_id": "docs_freshness",
  "dimension_id": "docs_freshness",
  "predicate_id": "confidence",
  "evidence_policy_id": "default_policy",
  "evidence_schemas": [
    {
      "schema_id": "folder_task_success",
      "required": false,
      "role": "Support",
      "reliability": 1.0,
      "precision": 1.0
    },
    {
      "schema_id": "aggregate_completion",
      "required": true,
      "role": "Support",
      "reliability": 1.0,
      "precision": 1.0
    }
  ],
  "source_mappings": [
    {
      "mapping_id": "folder_task_success_to_selected_tree",
      "source_kind": "execution_folder_task_outcome",
      "evidence_schema_id": "folder_task_success",
      "subject_from": "field:selected_scope",
      "value_field": "tree_stale_signal",
      "factor_id": "folder_task_success"
    },
    {
      "mapping_id": "package_aggregate_to_selected_tree",
      "source_kind": "execution_package_aggregate",
      "evidence_schema_id": "aggregate_completion",
      "subject_from": "record.subject",
      "value_field": "tree_stale_signal",
      "factor_id": "aggregate_completion"
    }
  ],
  "comparator": {
    "engine_id": "weighted_bayesian",
    "engine_version": "1",
    "factors": [
      {
        "factor_id": "folder_task_success",
        "evidence_schema_id": "folder_task_success",
        "weight": 0.2,
        "polarity": "Supports"
      },
      {
        "factor_id": "aggregate_completion",
        "evidence_schema_id": "aggregate_completion",
        "weight": 1.0,
        "polarity": "Supports"
      }
    ],
    "missing_evidence_uncertainty": 0.9
  },
  "default_prior": 0.75,
  "planner_projection": {
    "confidence_field": "confidence",
    "threshold": 0.6,
    "posterior_meaning": "stale_probability"
  },
  "config_version": "1",
  "observationality": "Observational"
}"#
}

pub fn survey_binding(
    workspace_root: std::path::PathBuf,
    storage_root: std::path::PathBuf,
) -> PhysicalBinding {
    PhysicalBinding {
        agent_positions: Default::default(),
        bindings: Default::default(),
        workspace_root: Some(workspace_root),
        subject: meld_events::DomainObjectRef::new("workspace_fs", "node", SUBJECT_ID).unwrap(),
        agent_id: AGENT_ID.to_string(),
        provider_id: Some("specimen-provider".to_string()),
        package: SelectedStewardshipPackage {
            expression: "docs_freshness".to_string(),
            principal_id: "workspace-owner".to_string(),
            belief_family_id: FAMILY_ID.to_string(),
            evidence_mapping_id: "docs_freshness".to_string(),
            curation_rule_id: "docs_freshness".to_string(),
            maintained_condition_id: "docs_freshness".to_string(),
            strategy_theory_id: "docs_freshness".to_string(),
            authority_policy_id: "docs_workspace_local".to_string(),
            claim_policy_id: "docs-claims-strict-v1".to_string(),
        },
        storage_root,
    }
}

/// Build one survey boot request over an explicit session directory.
///
/// The fixture installs its historical Belief body through the owning
/// registry before runtime assembly. Production initialization is not
/// reproduced by this test helper.
pub fn survey_boot_request(
    session_dir: &Path,
    binding: &PhysicalBinding,
    manifest_id: &str,
    booted_at_ms: u64,
) -> HarnessBootRequest {
    let layout = ProductStorageLayout::from_root(session_dir.join("root"));
    let stores = OpenProductStores::open(&layout).unwrap();
    let mut registry = BeliefFamilyRegistryStore::new(stores.traversal_store.db().clone()).unwrap();
    registry
        .install(serde_json::from_str(specimen_family_json()).unwrap(), 0)
        .unwrap();
    drop(stores);

    let mut request = HarnessBootRequest::temporary(manifest_id, booted_at_ms);
    request.root = HarnessRootSelection::ExistingDataRoot {
        product_root: session_dir.join("root"),
        branch_home: session_dir.join("branch-home"),
        legacy_store_path: session_dir.join("legacy-compat"),
        manifest_path: session_dir.join(format!("{manifest_id}.json")),
    };
    request.unsafe_existing_root = true;
    request.stewardship = Some(StewardshipComposition {
        binding: binding.clone(),
    });
    request.registration_set = Some(RegistrationSet {
        registrations: ["world_model.graph_replay", "world_model.belief_assessment"]
            .into_iter()
            .map(|runtime_id| RuntimeRegistration {
                registration_id: format!("survey::{runtime_id}"),
                runtime_id: runtime_id.to_string(),
                kind: RegistrationKind::ActiveActor,
                required_resources: Vec::new(),
            })
            .collect(),
    });
    request.enabled_runtime_ids = vec![
        "world_model.graph_replay".to_string(),
        "world_model.belief_assessment".to_string(),
    ];
    request.disabled_runtime_ids = Some(Vec::new());
    request
}
