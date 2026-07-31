//! Gate retry and terminal-failure contract on the package route.
//!
//! Failure-signal semantics under test: a gate-failing completion retries
//! inside the execute invocation within the declared budget and heals on a
//! later attempt with no belief-visible trace; a completion that fails the
//! gate on every attempt exhausts the budget and fails the run terminally
//! with the gate named in the error.

use crate::integration::parity_fixture::{
    try_run_workflow_route, ParityWorkspaceSpec, PARITY_TURNS_PER_FOLDER,
};

/// Attempt budget bound for every turn in `docs_writer_thread_v1`.
const TURN_ATTEMPT_BUDGET: usize = 3;

#[test]
fn gate_failure_retries_within_budget_and_heals() {
    let spec = ParityWorkspaceSpec::branching(2, 2);
    let actionable = spec.actionable_directories();
    let sabotage_first = TURN_ATTEMPT_BUDGET - 1;

    let run = try_run_workflow_route(&spec, sabotage_first)
        .expect("run with transient gate failures must heal within the retry budget");

    assert_eq!(run.completed_instances, run.capability_instances);
    assert_eq!(run.baseline.readme_paths().len(), actionable.len());
    // The sabotaged attempts cost exactly their count in extra provider
    // requests; nothing retried beyond the healing attempt.
    assert_eq!(
        run.provider_requests,
        PARITY_TURNS_PER_FOLDER * actionable.len() + sabotage_first
    );
}

#[test]
fn gate_failure_exhausting_budget_fails_terminally_and_bounded() {
    let spec = ParityWorkspaceSpec::branching(2, 2);

    let error = match try_run_workflow_route(&spec, usize::MAX) {
        Ok(_) => panic!("permanently gate-failing output must fail the run"),
        Err(error) => error,
    };

    assert!(
        error.contains("Workflow gate 'evidence_gate'"),
        "terminal error must name the failed gate: {error}"
    );
    assert!(
        error.contains("not a decodable JSON object"),
        "terminal error must carry the gate reasons: {error}"
    );
}
