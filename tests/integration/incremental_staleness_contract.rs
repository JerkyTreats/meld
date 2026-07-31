//! Incremental staleness contract on the package route.
//!
//! Node ids are content-addressed, so staleness detection is inherent to
//! merkle identity once generation is unforced — with one blocker this
//! contract pins alongside the working machinery: publication writes
//! README.md files back into the tree it is keyed on, so every published
//! folder re-identifies on the next scan and an unforced rerun regenerates
//! everything. With published artifacts removed, identity reverts to its
//! source-scoped value: a fresh workspace refuses to fabricate work, and a
//! single source mutation regenerates exactly its ancestor chain.

use std::fs;

use crate::integration::parity_fixture::{
    run_incremental_workflow_scenario, ParityWorkspaceSpec, PARITY_TURNS_PER_FOLDER,
};

#[test]
fn staleness_is_source_scoped_and_publication_feedback_defeats_it() {
    let spec = ParityWorkspaceSpec::branching(2, 2);
    let actionable = spec.actionable_directories();

    let outcome = run_incremental_workflow_scenario(&spec, |workspace_root| {
        let mutated = workspace_root
            .join("tree/branch_root_0/branch_root_0_0")
            .join("module_root_0_0.rs");
        fs::write(
            mutated,
            "pub fn feature_root_0_0() -> &'static str { \"root_0_0 changed\" }\n",
        )
        .unwrap();
    });

    let full = PARITY_TURNS_PER_FOLDER * actionable.len();
    assert_eq!(outcome.full_run_requests, full);

    // The blocker, pinned: published READMEs re-identify every folder, so
    // an unforced rerun with no source change regenerates the whole scope.
    // Source-scoped node identity is the open work that removes this.
    assert_eq!(
        outcome.feedback_requests, full,
        "publication feedback expected to force full regeneration"
    );

    // With published artifacts removed, identity is source-scoped: every
    // head is found and the run refuses to fabricate work.
    assert!(
        outcome.zero_work_error.contains("Nothing to regenerate"),
        "fresh workspace must refuse loudly, got: {}",
        outcome.zero_work_error
    );

    // One mutated leaf re-identifies exactly its ancestor chain: leaf,
    // parent branch, target root.
    assert_eq!(
        outcome.mutation_requests,
        PARITY_TURNS_PER_FOLDER * 3,
        "unforced run must regenerate only the mutated ancestor chain"
    );
}
