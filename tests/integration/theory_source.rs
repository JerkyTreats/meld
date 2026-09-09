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
    load_authority_policy, load_belief_family_config, load_curation_rule_config,
    load_maintained_condition, load_outcome_mapping_config, load_strategy_theory_package,
};
use meld_events::DomainObjectRef;

use crate::integration::test_utils::with_xdg_env;

fn shipped_theory_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("theory")
        .join("docs_freshness")
}

fn shipped_selection() -> SelectedStewardshipPackage {
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

fn shipped_subject() -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", "docs").unwrap()
}

#[test]
fn shipped_theory_provisions_and_loads_through_every_selection_identity() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let report = provision_theory_source(
            &shipped_theory_dir(),
            &shipped_selection(),
            &shipped_subject(),
        )
        .unwrap();

        let kinds: Vec<&str> = report
            .bodies
            .iter()
            .map(|body| body.kind.as_str())
            .collect();
        assert!(kinds.contains(&"belief_family"));
        assert!(kinds.contains(&"outcome_interpretation"));
        assert!(kinds.contains(&"curation_rule"));
        assert!(kinds.contains(&"maintained_condition"));
        assert!(kinds.contains(&"strategy_theory"));
        assert!(kinds.contains(&"authority_policy"));
        assert_eq!(kinds.len(), 7);
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
        assert!(matched_event_types
            .iter()
            .all(|event| *event == "world_model.curation.result.v1"));
        assert!(!matched_event_types.contains(&"execution.task.succeeded"));
        assert!(!matched_event_types.contains(&"world_model.unobserved_scope"));
        for rule in &mapping.rules {
            assert_eq!(rule.source_kind, "docs_required_coverage");
            assert!(family
                .source_mappings
                .iter()
                .any(|source| source.source_kind == rule.source_kind));
        }
        assert_eq!(
            family.planner_projection.posterior_meaning,
            "coverage_probability"
        );
        assert_eq!(
            family.initial_assessment,
            meld_world_model::belief::InitialAssessmentPolicy::PriorAllowed
        );

        let rule = load_curation_rule_config("docs_freshness").unwrap();
        assert_eq!(rule.dimension_id, "docs_freshness");
        assert_eq!(
            rule.maintained_condition_id.as_deref(),
            Some("docs_freshness")
        );

        let condition = load_maintained_condition("docs_freshness").unwrap();
        assert_eq!(condition.condition_id, "docs_freshness");
        assert_eq!(condition.dimension_id, rule.dimension_id);

        let strategy = load_strategy_theory_package("docs_freshness").unwrap();
        assert_eq!(strategy.snapshot.theory_id, "docs_freshness");
        assert_eq!(strategy.capabilities.len(), 4);

        let authority = load_authority_policy("docs_workspace_local").unwrap();
        assert_eq!(authority.principal_id, "workspace-owner");
    });
}

#[test]
fn provisioning_is_byte_idempotent() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let selection = shipped_selection();
        provision_theory_source(&shipped_theory_dir(), &selection, &shipped_subject()).unwrap();
        let second =
            provision_theory_source(&shipped_theory_dir(), &selection, &shipped_subject()).unwrap();
        assert!(second.bodies.iter().all(|body| !body.changed));
    });
}

#[test]
fn selection_identity_mismatch_rejects_before_any_write() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let mut selection = shipped_selection();
        selection.belief_family_id = "some-other-family".to_string();

        let error = provision_theory_source(&shipped_theory_dir(), &selection, &shipped_subject())
            .unwrap_err();

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
        provision_theory_source(
            &shipped_theory_dir(),
            &shipped_selection(),
            &shipped_subject(),
        )
        .unwrap();
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
principal_id = "workspace-owner"
provider_id = "steward-provider"

[stewardship.docs_freshness.theory]
belief_family_id = "docs_freshness"
evidence_mapping_id = "docs_freshness_outcome_interpretation_v1"
curation_rule_id = "docs_freshness"
maintained_condition_id = "docs_freshness"
strategy_theory_id = "docs_freshness"
authority_policy_id = "docs_workspace_local"
claim_policy_id = "docs-claims-strict-v1"
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
    assert!(run_context.product_runtime().capability_runtime().is_none());
    run_context
        .product_runtime()
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
}

#[test]
fn authored_files_do_not_activate_runtime_without_a_durable_receipt() {
    let test_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&test_dir, || {
        let workspace = tempfile::TempDir::new().unwrap();
        let workspace_root = workspace.path().join("ws");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let config_path = write_shipped_selection_config(&workspace_root);

        let codes = boot_diagnostic_codes(&workspace_root, &config_path);
        assert!(
            codes
                .iter()
                .any(|code| code == "theory_image_not_installed"),
            "missing durable activation diagnostic: {codes:?}"
        );

        provision_theory_source(
            &shipped_theory_dir(),
            &shipped_selection(),
            &shipped_subject(),
        )
        .unwrap();

        // Source provisioning copies authored inputs only. World init owns
        // durable owner installation and commits the activation receipt.
        let codes = boot_diagnostic_codes(&workspace_root, &config_path);
        assert!(
            codes
                .iter()
                .any(|code| code == "theory_image_not_installed"),
            "source files activated without a receipt: {codes:?}"
        );
    });
}
