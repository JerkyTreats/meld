# Gate Signal First Slice

Date: 2026-07-31
Status: complete 2026-07-31 — all six items landed; retry rides the execute invocation loop, gate violations map to terminal failed outcomes on the claim route, and the shipped config enforces with attempt budget three
Scope: the bounded code work to make gate outcomes act as signals on the package dispatch route, per [Failure Signal](../../cognitive_architecture/execution/failure_signal.md); part of the docs_freshness flywheel parity workstream

## Problem

On the package route the sole content-reading point evaluates its gate and discards the result when `fail_on_violation` is false — `src/context/capability.rs:1069-1080`. The attempt route already persists a `ThreadTurnGateRecordV1` and emits a turn-failed event; the package route records nothing. All four shipped gates in `workflows/docs_writer_thread_v1.yaml` are non-blocking, the schema gate has a substring escape hatch that passes prose containing a field name, and `no_semantic_drift` degrades to an unconditional pass when its field lists are empty. The combined effect: a semantically empty artifact passes every check and is published as success.

## Work

1. Record gate verdicts on the package route, reusing the existing gate record shape so both routes produce the same durable fact.
2. Bound retry on gate failure at the execution tier: a failed gate re-attempts the turn within a declared budget; no belief-visible event is emitted from a non-terminal attempt.
3. Exhausted retries fail the task terminally so the outcome flows through the installed `docs_package_failed_v1` interpretation. No new evidence machinery.
4. Tighten gate evaluation against the confirmed vacuous-pass routes in [Silent Success Findings](silent_success_findings.md) F2 and F10: remove the substring escape hatch, support a non-empty-array requirement so `evidence_gate` can require at least one claim, replace the fence-naive JSON parse at `gates.rs:157` with the fence-aware extraction the finalize path already uses, reconcile the gate-input key space between the two routes, and make `no_semantic_drift` record what it checked so a pass with nothing checked is distinguishable and failing rather than silent.
5. Honor `stop_on_gate_fail` on the capability path, closing the F6 drift against the attempt route.
6. Flip `fail_on_violation` and `stop_on_gate_fail` in the shipped workflow config, sequenced after the context parity fix so cold runs do not burn retry budget against starved context.

## Out Of Scope

Settling gate design across the wider codebase and design corpus — gate vocabulary, gate ownership per route, and the full failure-signal channel into belief beyond the terminal outcome mapping. That expansion is expected and deliberately deferred; the deferral is recorded in [TODO](../../cognitive_architecture/TODO.md).

## Acceptance

- A forced-empty output retries within budget and then fails terminally; the ledger shows gate records for every attempt and exactly one belief-visible terminal outcome.
- The goal remains open after terminal failure, with truthful narration.
- Existing green paths are unchanged: a healthy run records passing gate verdicts and publishes as before.

## Related Documentation

- [Failure Signal](../../cognitive_architecture/execution/failure_signal.md) — the canonical semantics this slice implements
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) — the parity workstream this slice belongs to
