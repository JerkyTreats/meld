//! Shared survey-configuration fixture: the stewardship composition,
//! frozen specimen theory body, and staged boot the stall specimen and
//! the served-surface tests reproduce the live survey with.

use std::path::Path;

use meld::config::{PhysicalBinding, SelectedStewardshipPackage};
use meld::harness::boot::{HarnessBootRequest, HarnessRootSelection, HarnessWorldInit};
use meld::init::world::pipeline::WorldInitContent;
use meld::init::world::{WorldInitRequest, WorldInitStage};
use meld::runtime::assembly::{StewardshipComposition, StewardshipTheoryBindings};
use meld_events::DomainObjectRef;
use meld_world_model::agent::AgentCurationRuleConfig;
use meld_world_model::belief::BranchScope;
use meld_world_model::PerspectiveKey;

pub const SUBJECT_ID: &str = "docs";
pub const AGENT_ID: &str = "seed.docs_freshness";
pub const FAMILY_ID: &str = "docs_freshness";

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
        workspace_root,
        subject: SUBJECT_ID.to_string(),
        agent_id: AGENT_ID.to_string(),
        provider_id: "specimen-provider".to_string(),
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

pub fn world_init() -> HarnessWorldInit {
    HarnessWorldInit {
        request: WorldInitRequest {
            stages: vec![
                WorldInitStage::InstallTheory,
                WorldInitStage::GenesisIdentities,
                WorldInitStage::SeedEpistemicFacts,
            ],
        },
        content: WorldInitContent {
            family_config: serde_json::from_str(specimen_family_json()).unwrap(),
            curation_rule: AgentCurationRuleConfig {
                maintained_condition_id: None,
                dimension_id: FAMILY_ID.to_string(),
                threshold: 0.7,
                priority_urgency: 50,
                desired_summary: "confidence>0.7".to_string(),
                source_kind: "belief_divergence".to_string(),
            },
            agent_id: AGENT_ID.to_string(),
            subject: DomainObjectRef::new("workspace_fs", "node", SUBJECT_ID).unwrap(),
            perspective: PerspectiveKey::new("default", "default").unwrap(),
            branch_scope: BranchScope::main(),
            observation_scope: FAMILY_ID.to_string(),
            directive: format!("steward 'docs_freshness' for subject '{SUBJECT_ID}'"),
            provenance: "meld world init".to_string(),
            session_id: "stall-specimen".to_string(),
            observed_seq: 0,
        },
    }
}

/// Build one survey boot request over an explicit session directory.
///
/// The two-boot structure mirrors the product: the first boot installs
/// the world after assembly, a later boot binds the actors against it.
pub fn survey_boot_request(
    session_dir: &Path,
    binding: &PhysicalBinding,
    manifest_id: &str,
    booted_at_ms: u64,
) -> HarnessBootRequest {
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
        theory: StewardshipTheoryBindings::default(),
    });
    request.world_init = Some(world_init());
    request
}
