//! Contract tests for the convergence-proof parity fixtures.
//!
//! These prove the fixture surface itself before the proof consumes it:
//! the workspace builder is byte-deterministic, the workflow-route
//! baseline is reproducible across fully isolated runs, and the
//! expected-filesystem assertion helpers report readable diffs.

use crate::integration::parity_fixture::{
    assert_baselines_match, assert_workspace_matches_baseline, capture_file_snapshot,
    run_workflow_route_baseline, ParityWorkspaceSpec, ReadmeBaseline, PARITY_TURNS_PER_FOLDER,
};
use tempfile::TempDir;

#[test]
fn parity_workspace_builder_writes_deterministic_trees() {
    for spec in [
        ParityWorkspaceSpec::default_shape(),
        ParityWorkspaceSpec::branching(3, 3),
    ] {
        let first_dir = TempDir::new().unwrap();
        let second_dir = TempDir::new().unwrap();
        spec.write_to(first_dir.path());
        spec.write_to(second_dir.path());
        assert_eq!(
            capture_file_snapshot(first_dir.path()),
            capture_file_snapshot(second_dir.path()),
            "workspace builder produced different trees for the same spec"
        );
    }
}

#[test]
fn parity_default_shape_has_proof_required_structure() {
    let spec = ParityWorkspaceSpec::default_shape();
    let directories = spec.actionable_directories();

    // Three folder levels below the workspace root.
    assert!(directories.contains("tree"));
    assert!(directories.contains("tree/alpha"));
    assert!(directories.contains("tree/alpha/core"));
    // Sibling fan-out of three actionable folders at one level.
    let top_level_fanout = directories
        .iter()
        .filter(|directory| {
            directory.starts_with("tree/") && !directory["tree/".len()..].contains('/')
        })
        .count();
    assert_eq!(top_level_fanout, 3);
    // The walker-ignored folder is excluded from the actionable set.
    assert!(!directories.contains("tree/target"));
    assert!(spec
        .expected_readme_paths()
        .iter()
        .all(|path| !path.starts_with("tree/target/")));
}

#[test]
fn parity_branching_spec_scales_with_depth_and_breadth() {
    let spec = ParityWorkspaceSpec::branching(3, 3);
    // 1 target folder + 3 + 9 + 27 branch folders, all actionable.
    assert_eq!(spec.actionable_directories().len(), 40);
    assert_eq!(spec.expected_readme_paths().len(), 40);

    let narrow = ParityWorkspaceSpec::branching(2, 2);
    assert_eq!(narrow.actionable_directories().len(), 7);
}

#[test]
fn workflow_route_baseline_is_reproducible() {
    let spec = ParityWorkspaceSpec::default_shape();
    let expected_paths = spec.expected_readme_paths();
    let actionable = spec.actionable_directories();

    let first = run_workflow_route_baseline(&spec);
    let second = run_workflow_route_baseline(&spec);

    // The route published exactly one README per actionable folder and
    // nothing inside the non-actionable folder.
    assert_eq!(first.baseline.readme_paths(), expected_paths);
    assert!(first
        .baseline
        .readme_paths()
        .iter()
        .all(|path| !path.starts_with("tree/target/")));

    // Both runs drove the full four-turn route over every folder.
    assert_eq!(
        first.provider_requests,
        PARITY_TURNS_PER_FOLDER * actionable.len()
    );
    assert_eq!(second.provider_requests, first.provider_requests);
    assert_eq!(first.completed_instances, first.capability_instances);
    assert_eq!(second.completed_instances, second.capability_instances);

    // Byte-identical README path set and contents across isolated runs.
    assert_baselines_match(&first.baseline, &second.baseline);

    // Folders with distinct child contexts converge to distinct contents:
    // parent READMEs incorporate their children's published READMEs.
    let root_readme = first.baseline.content("tree/README.md").unwrap();
    let alpha_readme = first.baseline.content("tree/alpha/README.md").unwrap();
    let beta_readme = first.baseline.content("tree/beta/README.md").unwrap();
    assert_ne!(root_readme, alpha_readme);
    assert_ne!(alpha_readme, beta_readme);
    // Route characterization: leaf-folder turn prompts carry folder
    // identity through on-disk file-child source (the cold-run fallback),
    // so leaves with distinct files produce distinct README content, and
    // an empty leaf prompts from the insufficient-context marker instead
    // of an absent context block.
    let core = first.baseline.content("tree/alpha/core/README.md").unwrap();
    let util = first.baseline.content("tree/alpha/util/README.md").unwrap();
    let hollow = first
        .baseline
        .content("tree/gamma/hollow/README.md")
        .unwrap();
    assert_ne!(core, util);
    assert_ne!(core, hollow);
    assert_ne!(util, hollow);
}

#[test]
fn baseline_assertions_report_readable_diffs() {
    let expected = ReadmeBaseline::from_entries([
        ("tree/README.md", "# root\n"),
        ("tree/alpha/README.md", "# alpha\n"),
        ("tree/beta/README.md", "# beta\n"),
    ]);
    let actual = ReadmeBaseline::from_entries([
        ("tree/README.md", "# root\n"),
        ("tree/alpha/README.md", "# drifted\n"),
        ("tree/gamma/README.md", "# gamma\n"),
    ]);

    assert!(!expected.is_empty());
    assert_eq!(expected.len(), 3);

    let differences = expected.diff(&actual);
    assert_eq!(differences.len(), 3);
    assert!(differences
        .iter()
        .any(|difference| difference.contains("missing README 'tree/beta/README.md'")));
    assert!(differences
        .iter()
        .any(|difference| difference.contains("unexpected README 'tree/gamma/README.md'")));
    assert!(differences.iter().any(|difference| {
        difference.contains("content mismatch at 'tree/alpha/README.md'")
            && difference.contains("# alpha")
            && difference.contains("# drifted")
    }));

    // Matching baselines assert cleanly, both against a captured baseline
    // and against a real filesystem tree.
    assert_baselines_match(&expected, &expected.clone());
    let workspace = TempDir::new().unwrap();
    for (path, content) in [
        ("tree/README.md", "# root\n"),
        ("tree/alpha/README.md", "# alpha\n"),
        ("tree/beta/README.md", "# beta\n"),
    ] {
        let full = workspace.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    assert_workspace_matches_baseline(workspace.path(), &expected);
}
