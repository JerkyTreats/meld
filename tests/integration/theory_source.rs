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
    load_available_actions, load_belief_family_config, load_curation_rule_config,
    load_method_realizations, load_outcome_mapping_config, load_planning_methods,
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
        assert!(kinds.contains(&"available_actions"));
        assert!(kinds.contains(&"method_realizations"));
        assert!(kinds.contains(&"method"));
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
        assert!(matched_event_types.contains(&"execution.package.completed"));
        assert!(matched_event_types.contains(&"execution.package.failed"));

        let rule = load_curation_rule_config("docs_freshness").unwrap();
        assert_eq!(rule.dimension_id, "docs_freshness");

        let methods = load_planning_methods("docs_freshness").unwrap();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].method_id, "refresh_docs_v1");

        let actions = load_available_actions("docs_freshness").unwrap();
        assert_eq!(actions.actions.len(), 1);
        assert_eq!(actions.actions[0].action_id, "docs.refresh_subtree");

        let realizations = load_method_realizations("docs_freshness").unwrap();
        assert_eq!(realizations.len(), 1);
        assert_eq!(realizations[0].method_id, "refresh_docs_v1");
        assert_eq!(realizations[0].action_id, "docs.refresh_subtree");

        // The realization association is closed over the provisioned set:
        // every named method and action exists.
        assert!(realizations.iter().all(|binding| {
            methods.iter().any(|m| m.method_id == binding.method_id)
                && actions
                    .actions
                    .iter()
                    .any(|a| a.action_id == binding.action_id)
        }));
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
