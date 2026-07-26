//! Shared survey-configuration fixture: the stewardship composition,
//! shipped theory body, and staged boot the stall specimen and the
//! served-surface tests reproduce the live survey with.

use std::fs;
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

/// The shipped theory body, byte for byte; no fixture-local mappings.
pub fn shipped_family_json() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/theory/docs_freshness/belief_family.docs_freshness.json"
    );
    fs::read_to_string(path).expect("shipped theory body exists")
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
            belief_family_id: FAMILY_ID.to_string(),
            evidence_mapping_id: "docs_freshness".to_string(),
            curation_rule_id: "docs_freshness".to_string(),
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
            family_config: serde_json::from_str(&shipped_family_json()).unwrap(),
            curation_rule: AgentCurationRuleConfig {
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
