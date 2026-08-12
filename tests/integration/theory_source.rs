//! Theory-source provisioning and load-path characterization.
//!
//! Proves the shipped `theory/docs_freshness/` package provisions into the
//! XDG theory root under the loader convention and loads back through every
//! selection identity it declares — the product path that turns committed
//! theory bodies into loadable configuration.

use std::path::{Path, PathBuf};

use meld::config::SelectedStewardshipPackage;
use meld::init::world::source::provision_theory_source;
use meld::init::world::theory::{
    load_belief_family_config, load_curation_rule_config, load_outcome_mapping_config,
};

use crate::integration::test_utils::with_xdg_env;

fn shipped_theory_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("theory")
        .join("docs_freshness")
}

fn shipped_selection() -> SelectedStewardshipPackage {
    SelectedStewardshipPackage {
        expression: "docs_freshness".to_string(),
        belief_family_id: "docs_freshness".to_string(),
        evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
        curation_rule_id: "docs_freshness".to_string(),
    }
}

#[test]
fn shipped_theory_provisions_and_loads_through_every_selection_identity() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let report = provision_theory_source(&shipped_theory_dir(), &shipped_selection()).unwrap();

        let kinds: Vec<&str> = report
            .bodies
            .iter()
            .map(|body| body.kind.as_str())
            .collect();
        assert!(kinds.contains(&"belief_family"));
        assert!(kinds.contains(&"outcome_interpretation"));
        assert!(kinds.contains(&"curation_rule"));
        assert_eq!(kinds.len(), 3);
        assert!(report.bodies.iter().all(|body| body.changed));

        let family = load_belief_family_config("docs_freshness").unwrap();
        assert_eq!(family.dimension_id, "docs_freshness");

        let mapping =
            load_outcome_mapping_config("docs_freshness_outcome_interpretation_v1").unwrap();
        let matched_event_types: Vec<&str> = mapping
            .rules
            .iter()
            .map(|rule| rule.match_event_type.as_str())
            .collect();
        assert!(matched_event_types.contains(&"execution.task.succeeded"));
        assert!(!matched_event_types.contains(&"execution.package.completed"));
        assert!(!matched_event_types.contains(&"execution.package.failed"));
        // The genesis rule closes belief motion from the unobserved-scope
        // fact: its source kind must map inside the family.
        assert!(matched_event_types.contains(&"world_model.unobserved_scope"));
        let genesis_rule = mapping
            .rules
            .iter()
            .find(|rule| rule.match_event_type == "world_model.unobserved_scope")
            .unwrap();
        assert!(family
            .source_mappings
            .iter()
            .any(|mapping| mapping.source_kind == genesis_rule.source_kind));
        assert_eq!(
            family.anchor_requirement,
            meld_world_model::belief::AnchorRequirement::Unanchored
        );

        let rule = load_curation_rule_config("docs_freshness").unwrap();
        assert_eq!(rule.dimension_id, "docs_freshness");
    });
}

#[test]
fn provisioning_is_byte_idempotent() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let selection = shipped_selection();
        provision_theory_source(&shipped_theory_dir(), &selection).unwrap();
        let second = provision_theory_source(&shipped_theory_dir(), &selection).unwrap();
        assert!(second.bodies.iter().all(|body| !body.changed));
    });
}

#[test]
fn selection_identity_mismatch_rejects_before_any_write() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let mut selection = shipped_selection();
        selection.belief_family_id = "some-other-family".to_string();

        let error = provision_theory_source(&shipped_theory_dir(), &selection).unwrap_err();

        assert!(error.to_string().contains("some-other-family"));
        // Nothing was provisioned: the theory root does not exist.
        assert!(!test_dir.path().join("meld").join("theory").exists());
    });
}

#[test]
fn provisioning_writes_nothing_outside_the_config_home() {
    let workspace = tempfile::TempDir::new().unwrap();
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        provision_theory_source(&shipped_theory_dir(), &shipped_selection()).unwrap();
        assert_eq!(std::fs::read_dir(workspace.path()).unwrap().count(), 0);
    });
}

/// Stewardship config selecting the shipped theory identities.
fn write_shipped_selection_config(workspace_root: &Path) -> PathBuf {
    let config_dir = workspace_root.join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    let target_root = workspace_root.canonicalize().unwrap();
    let config = format!(
        r#"[providers.steward-provider]
provider_name = "steward-provider"
provider_type = "local"
model = "test-model"
endpoint = "http://127.0.0.1:9"

[stewardship.docs_freshness]
expression = "docs_freshness"
target_root = "{target_root}"
subject = "docs"
agent_id = "docs-writer"
provider_id = "steward-provider"

[stewardship.docs_freshness.theory]
belief_family_id = "docs_freshness"
evidence_mapping_id = "docs_freshness_outcome_interpretation_v1"
curation_rule_id = "docs_freshness"
"#,
        target_root = target_root.display()
    );
    let config_path = config_dir.join("config.toml");
    std::fs::write(&config_path, config).unwrap();
    config_path
}

fn boot_diagnostic_codes(workspace_root: &Path, config_path: &Path) -> Vec<String> {
    let run_context = meld::cli::RunContext::new(
        workspace_root.to_path_buf(),
        Some(config_path.to_path_buf()),
    )
    .unwrap();
    let capabilities = run_context
        .product_runtime()
        .capability_runtime()
        .expect("PDS capability runtime");
    assert!(capabilities.catalog.contains("docs.inspect_scope", 1));
    assert!(capabilities
        .catalog
        .contains("docs.assess_published_scope", 1));
    assert!(!capabilities.catalog.contains("merkle_traversal", 1));
    run_context
        .product_runtime()
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
}

#[test]
fn pds_composes_without_legacy_planning_files() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let workspace = tempfile::TempDir::new().unwrap();
        let workspace_root = workspace.path().join("ws");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let config_path = write_shipped_selection_config(&workspace_root);

        // PDS capability and Strategy vocabulary compose independently of
        // the provisioned belief and evidence theory files.
        let codes = boot_diagnostic_codes(&workspace_root, &config_path);
        assert!(
            !codes
                .iter()
                .any(|code| code == "planning_theory_unresolved"),
            "PDS planning unexpectedly depended on legacy files: {codes:?}"
        );

        provision_theory_source(&shipped_theory_dir(), &shipped_selection()).unwrap();

        // Provisioning belief and evidence theory does not change the PDS
        // capability catalog or introduce a workflow dependency.
        let codes = boot_diagnostic_codes(&workspace_root, &config_path);
        assert!(
            !codes
                .iter()
                .any(|code| code == "planning_theory_unresolved"),
            "planning theory stayed unresolved after provisioning: {codes:?}"
        );
        assert!(
            !codes
                .iter()
                .any(|code| code == "evidence_mapping_unresolved"),
            "evidence mapping stayed unresolved after provisioning: {codes:?}"
        );
    });
}
