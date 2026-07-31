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

Complete 2026-07-31, with the diagnosis corrected on trace. Node identity is content-addressed, so head presence is a freshness check by construction and no digest comparison was needed. Landed: the three bare-equality completion predicates carry the known-greater-than-zero guard their aggregate sibling had; an all-fresh expansion refuses with a nothing-to-regenerate marker instead of expanding to zero instances; the claim route records that refusal terminally; and the production dispatch preparer stopped forcing whole-scope regeneration every turn, making runs incremental by identity.

The incremental staleness contract proves the machinery and measures the post-publication behavior in one test: with published READMEs removed, a fresh workspace refuses to fabricate work and one source mutation regenerates exactly its ancestor chain; while published READMEs sit in the tree, a rerun regenerates the whole scope because publication changed the world and identity truthfully reports it. Convergence is not this layer's job: **the regeneration loop is cut by belief** — the published outcome's yield becomes evidence, the freshness score passes, the goal satisfies, and no further dispatch occurs. That cut is slice four. Full-scope regeneration cost on a dispatch that follows a publication turn is recorded as an efficiency residual.

### Slice Four — Semantic Yield And Publication Truthfulness

Complete 2026-07-31 — the flywheel's epistemic cut. The package document declares which artifact and array field measure per-folder yield; assembly derives the declaration alongside folder classification; the aggregate counts yield from artifact content the ledger already carries and publishes a summary classing the run substantive or hollow. The installed theory splits its completed interpretation on that observable: substantive asserts freshness and satisfies the goal, cutting the regeneration loop; hollow is contradicting evidence at 0.9 and the goal stays truthfully open. No new engine machinery — the mapping vocabulary's content rules carry the whole discrimination. The publish filter's missing-head result became a terminal marked error recorded through the claim route, closing F9 primary. The yield declaration is package-authored data and one settlement route; the mapping value vocabulary is untouched, so settlement-by-acquisition slots in beside it later.

### Slice Five — Comparator Reconciliation

Escalated 2026-07-31 to a design decision for the Strategy sync, with a failed candidate measured. The residual: solo evidence windows cancel declared weight, reliability, and precision — `weighted / total_weight` — so one event swings the posterior at full strength regardless of theory declaration. The candidate reconciliation let the prior retain undeclared mass below full coverage: `weighted + (1 - total_weight) * prior`, preserving full-coverage behavior exactly. Result, measured against the installed theory: the satisfaction arc breaks. Damped genesis and folder posteriors leave the pre-completion revision near 0.70 stale, and the completed event's averaging step can only halve it — a substantive completion lands at confidence 0.659, below the 0.7 satisfaction threshold, and the loop never cuts. The shipped theory numbers are implicitly calibrated to cancel semantics; every intent-honoring damping variant reproduces the break.

The decision is therefore coupled: comparator weight semantics, theory factor weights, and the satisfaction threshold move together, and the sequential per-event averaging that gives the posterior its inertia is the deeper cause of order and count sensitivity. This is comparator-and-theory calibration — Strategy-sync material, not a parity patch. The candidate and its measurements are preserved in this section; current cancel semantics stay in force and are pinned by the solo-window characterization.

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
