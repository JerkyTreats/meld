# Flywheel Parity Workstream

Date: 2026-07-31
Status: at the Strategy pause — every implementable slice landed or escalated with a decision brief; live validation awaits a reachable provider
Scope: everything up to Strategy — the bounded slices that bring the docs_freshness flywheel to truthful operation and loose output parity readiness, pausing at the Strategy boundary

## Goal

Long term, the flywheel reaches loose output parity with the static docs_freshness workflow: non-empty claims per actionable folder, children consumed into parent READMEs, structural fidelity to the workflow's output shape — without recreating static configuration inside PDS or theory. Medium term, this workstream delivers everything up to Strategy, then pauses for a sync. Strategy work — the settlement transform, learned ranking, efficacy — is explicitly not in this workstream. Parity finalization follows Strategy.

The hollow-README diagnosis and the [Silent Success Findings](silent_success_findings.md) define the work. The through-line: no layer may report success it cannot support, and no output may satisfy a goal without observable substance.

## Slices

### Slice One — Context Parity

Complete 2026-07-31. Frameless file children fall back to bounded on-disk source and empty directories prompt from the literal insufficient-context marker — resolver-sibling semantics restored on the capability path, with the old leaf-uniformity characterization inverted to assert distinct leaf content. The accomplices closed across the workstream: readiness validates declared artifact type and schema and narrates blocked tasks, F8 closed; the profile-versus-package cross-check landed at lowering, F1 closed; F7 was corrected on trace to a metadata-snapshot residual with no code change. The live-run half of acceptance — non-empty `evidence_map.claims` on a cold lab run — rides slice seven.

### Slice Two — Gate Signal

Complete 2026-07-31 per [Gate Signal First Slice](gate_signal_first_slice.md), under the canonical [Failure Signal](../../cognitive_architecture/execution/failure_signal.md) semantics: every verdict recorded before any failure return, gate failure retrying inside the execute invocation within the declared budget, exhausted budgets failing terminally through the installed failed-outcome interpretation, the vacuous-pass routes of F2 and F10 closed, and the shipped config enforcing with attempt budget three. The gate retry contract proves heal-within-budget at exactly the sabotaged cost and terminal failure naming the gate.

### Slice Three — Staleness And Completion Truthfulness

Complete 2026-07-31, with the diagnosis corrected on trace. Node identity is content-addressed, so head presence is a freshness check by construction and no digest comparison was needed. Landed: the three bare-equality completion predicates carry the known-greater-than-zero guard their aggregate sibling had; an all-fresh expansion refuses with a nothing-to-regenerate marker instead of expanding to zero instances; the claim route records that refusal terminally; and the production dispatch preparer stopped forcing whole-scope regeneration every turn, making runs incremental by identity.

The incremental staleness contract proves the machinery and measures the post-publication behavior in one test: with published READMEs removed, a fresh workspace refuses to fabricate work and one source mutation regenerates exactly its ancestor chain; while published READMEs sit in the tree, a rerun regenerates the whole scope because publication changed the world and identity truthfully reports it. Convergence is not this layer's job: **the regeneration loop is cut by belief** — the published outcome's yield becomes evidence, the freshness score passes, the goal satisfies, and no further dispatch occurs. That cut is slice four. Full-scope regeneration cost on a dispatch that follows a publication turn is recorded as an efficiency residual.

### Slice Four — Semantic Yield And Publication Truthfulness

Complete 2026-07-31 — the flywheel's epistemic cut. The package document declares which artifact and array field measure per-folder yield; assembly derives the declaration alongside folder classification; the aggregate counts yield from artifact content the ledger already carries and publishes a summary classing the run substantive or hollow. The installed theory splits its completed interpretation on that observable: substantive asserts freshness and satisfies the goal, cutting the regeneration loop; hollow is contradicting evidence at 0.9 and the goal stays truthfully open. No new engine machinery — the mapping vocabulary's content rules carry the whole discrimination. The publish filter's missing-head result became a terminal marked error recorded through the claim route, closing F9 primary. The yield declaration is package-authored data and one settlement route; the mapping value vocabulary is untouched, so settlement-by-acquisition slots in beside it later.

### Slice Five — Comparator Reconciliation

Escalated 2026-07-31 to a design decision for the Strategy sync, with a failed candidate measured. The residual: solo evidence windows cancel declared weight, reliability, and precision — `weighted / total_weight` — so one event swings the posterior at full strength regardless of theory declaration. The candidate reconciliation let the prior retain undeclared mass below full coverage: `weighted + (1 - total_weight) * prior`, preserving full-coverage behavior exactly. Result, measured against the installed theory: the satisfaction arc breaks. Damped genesis and folder posteriors leave the pre-completion revision near 0.70 stale, and the completed event's averaging step can only halve it — a substantive completion lands at confidence 0.659, below the 0.7 satisfaction threshold, and the loop never cuts. The shipped theory numbers are implicitly calibrated to cancel semantics; every intent-honoring damping variant reproduces the break.

The decision is therefore coupled: comparator weight semantics, theory factor weights, and the satisfaction threshold move together, and the sequential per-event averaging that gives the posterior its inertia is the deeper cause of order and count sensitivity. This is comparator-and-theory calibration — Strategy-sync material, not a parity patch. The candidate and its measurements are preserved in this section; current cancel semantics stay in force and are pinned by the solo-window characterization.

### Slice Six — Declared-Effect Characterization

Characterized 2026-07-31; the guard is deliberately not in scope. The pinned test `declared_effects_alone_currently_satisfy_goal` proves the hole exists: applying the method's declared update flips an evidence-free goal from Indeterminate to Satisfied. The existing evaluation-loop test asserts that same behavior as intended plan-time projection, so the guard is not a patch but a separation of plan-time projection from settlement satisfaction — the opening agenda item of the Strategy sync, where the settlement transform owns the seam.

### Slice Seven — Validation And Pause

Partially complete. The fixture half landed as contract tests in CI: structural parity assertions over the resurrected corpus with the hollow specimen pinned as a permanent must-fail, the gate retry contract, and the incremental staleness contract. The live half — one full cold lab-gateway run asserting non-hollow content through the new yield path, and one forced-failure run asserting truthful non-satisfaction — awaits a reachable provider. The workstream is otherwise at the pause.

## Baseline Corpus

Resurrected 2026-07-31 from the deleted eval harness at `0cf93df^:eval/readme/`: the synthetic nested case plus two github-docs cases live at `tests/fixtures/readme_parity/` with frozen input filesystems, golden expected READMEs, and provenance metadata, and the expected-properties assertions plus heading coverage at a 0.70 floor live in the readme parity assertion helper. The hollow-README specimen is pinned as a must-fail case. The Python runner, promptfoo integration, tuning loop, and optimization weights stay dead; the remaining three fixtures stay recoverable from history.

## Triage Before Sequencing

Resolved 2026-07-31: the F4 replay path proved legacy — its only callers were tests — and was killed as a breaking change, with the legacy mapper preserved as integration test support for the reopen contract.

## Pre-Strategy Exit Criteria

Scored at the pause, 2026-07-31:

- Cold run yields real content at every actionable directory — met at fixture level; live confirmation rides slice seven.
- Empty output cannot satisfy the goal — met: hollow completions map to contradicting evidence, pinned at the interpretation layer.
- Gate verdicts are durably recorded; terminal failure narrates truthfully — met, proven by the gate retry contract.
- Source change re-fires regeneration without force — met: content-addressed identity regenerates exactly the mutated ancestor chain.
- Zero-work and zero-file runs cannot report success — met: refusal markers, completion guards, and the terminal missing-head error.
- Solo-window comparator behavior reconciled with theory intent — escalated with a measured decision brief in slice five; cancel semantics remain pinned in force.
- Declared-effects characterization test green — pinned as the hole rather than the guard; the separation is the Strategy sync's opening item per slice six.

## Out Of Scope

Strategy construction, the settlement transform, learned ranking, cost record, failure-signal expansion beyond terminal mapping, the workflow's self-recursion flaw, CVE stewardship implementation, and full parity finalization — the last resumes after Strategy.

## Related Documentation

- [Silent Success Findings](silent_success_findings.md) — finding-to-slice dispositions
- [Gate Signal First Slice](gate_signal_first_slice.md) — slice two detail
- [Failure Signal](../../cognitive_architecture/execution/failure_signal.md) — canonical failure semantics
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) — the predecessor program this workstream follows
- [CVE Freshness Use Case](../../use_cases/cve_freshness.md) — the inverse use case parity choices must not foreclose
