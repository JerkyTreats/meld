# Flywheel Parity Workstream

Date: 2026-07-31
Status: active
Scope: everything up to Strategy — the bounded slices that bring the docs_freshness flywheel to truthful operation and loose output parity readiness, pausing at the Strategy boundary

## Goal

Long term, the flywheel reaches loose output parity with the static docs_freshness workflow: non-empty claims per actionable folder, children consumed into parent READMEs, structural fidelity to the workflow's output shape — without recreating static configuration inside PDS or theory. Medium term, this workstream delivers everything up to Strategy, then pauses for a sync. Strategy work — the settlement transform, learned ranking, efficacy — is explicitly not in this workstream. Parity finalization follows Strategy.

The hollow-README diagnosis and the [Silent Success Findings](silent_success_findings.md) define the work. The through-line: no layer may report success it cannot support, and no output may satisfy a goal without observable substance.

## Slices

### Slice One — Context Parity

The root-cause fix. Restore file-content context on the capability path, mirroring the resolver semantics the task-package path silently dropped — filesystem fallback for frameless directory children with bounded excerpts, and the explicit insufficient-context marker for genuinely empty directories. Close the structural accomplices: the empty-wiring filter, the non-required upstream slot, readiness discarding schema versions and diagnostics per F8, force-tombstone ordering per F7, and the missing `input_refs` cross-check between profile and package per F1.

Acceptance: a cold run on the fixture corpus produces prompts containing file-derived context at the deepest directories, and live `evidence_map.claims` is non-empty.

### Slice Two — Gate Signal

Per [Gate Signal First Slice](gate_signal_first_slice.md), under the canonical [Failure Signal](../../cognitive_architecture/execution/failure_signal.md) semantics: gate verdicts recorded on the package route, bounded retry at the execution tier, terminal failure through the installed failed-outcome interpretation, gates tightened against the vacuous-pass routes in F2 and F10, config flipped after slice one lands.

### Slice Three — Staleness And Completion Truthfulness

New, from F3 and F5. Traversal compares the stored context digest against the freshly computed digest instead of treating head presence as done; emptied batches are flagged rather than dropped; the three bare-equality completion predicates gain the `known > 0` guard their aggregate sibling already carries. This slice revives the re-fire half of docs_freshness — staleness detection is what makes freshness a maintained condition rather than a one-shot generation.

Acceptance: changing source under a documented directory produces regeneration work without force; a zero-work expansion is reported truthfully, not as success.

### Slice Four — Semantic Yield And Publication Truthfulness

Content becomes observable to belief, and publication stops impersonating success. Finalize stamps a verified-claims cardinality on the outcome; the per-folder aggregate carries it through `FolderPublicationResult` into the package-completed payload; the completed-outcome interpretation swaps its constant for a `DataScalar` over the yield pointer — theory declares which observable, code provides it, no new engine machinery. The publish filter's missing-head result becomes an acted-upon signal per F9 rather than a written-and-never-read artifact.

Acceptance: an empty-yield run produces contradicting evidence and the goal stays open; a run that writes zero files cannot report success.

### Slice Five — Comparator Reconciliation

The recorded residual: factor weights cancel in per-event solo windows, so a single event swings the posterior regardless of declared weight. Characterize with a comparator test, reconcile comparator behavior with theory-author intent. Load-bearing once slice four makes evidence values vary.

### Slice Six — Declared-Effect Characterization

A test proving declared effects alone cannot satisfy: run the method with dispatch producing nothing and assert the dimension never crosses the satisfaction threshold from the declared update value. If it fails, the minimal guard enters scope and observationality gains its first reader early; the settlement transform proper remains Strategy work past the pause.

### Slice Seven — Validation And Pause

Fixture-based structural parity assertions in CI over the resurrected baseline corpus, one full cold lab run asserting non-hollow content, one forced-failure run asserting truthful non-satisfaction. Then the workstream pauses for the Strategy sync.

## Baseline Corpus

Resurrected from the deleted eval harness at `0cf93df^:eval/readme/` — frozen input filesystems, golden expected READMEs, and per-fixture expected properties, including pinned github-docs subtrees researched for good-README shape. Take the synthetic nested case plus two github-docs cases into a Rust-test-owned fixtures directory; port the expected-properties assertions and the heading-coverage idea into a Rust assertion helper. The Python runner, promptfoo integration, tuning loop, and optimization weights stay dead.

## Triage Before Sequencing

The F4 replay path — evidence promotion filtered on an artifact type nothing produces, with its cursor advanced past skips — needs a liveness verdict before slice work begins: kill it if legacy, fix the cursor advance immediately if live.

## Pre-Strategy Exit Criteria

- Cold run yields real content at every actionable directory.
- Empty output cannot satisfy the goal.
- Gate verdicts are durably recorded; terminal failure narrates truthfully.
- Source change re-fires regeneration without force.
- Zero-work and zero-file runs cannot report success.
- Solo-window comparator behavior reconciled with theory intent.
- Declared-effects characterization test green.

## Out Of Scope

Strategy construction, the settlement transform, learned ranking, cost record, failure-signal expansion beyond terminal mapping, the workflow's self-recursion flaw, CVE stewardship implementation, and full parity finalization — the last resumes after Strategy.

## Related Documentation

- [Silent Success Findings](silent_success_findings.md) — finding-to-slice dispositions
- [Gate Signal First Slice](gate_signal_first_slice.md) — slice two detail
- [Failure Signal](../../cognitive_architecture/execution/failure_signal.md) — canonical failure semantics
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) — the predecessor program this workstream follows
- [CVE Freshness Use Case](../../use_cases/cve_freshness.md) — the inverse use case parity choices must not foreclose
