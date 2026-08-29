# WMR-VC-02 Evidence Remediation Record

Date: 2026-08-29

Gate: `WMR-VC-02-DG` frozen revision 1

Initial candidate digest: `ba37bbe63575f8316373adaaaf318767514d00a103f3fe80440d0f17b3383cdb`

Successor candidate digest: `01dc68b6f55bfaee115e102a973d8ca4595c5a5bf74f51733f069c251c2a6ce1`

Initial verdict: `not eligible`

Verification verdict: `accepted`

## Independent Audit Result

The delegated reviewer reproduced the initial candidate identity, confirmed a clean workspace, evaluated every frozen criterion, and reran formatting, lint, focused tests, the root proof, and the complete sequential workspace suite. It found no source defect, architecture violation, parallel runtime authority, or regression.

The audit identified two missing evidence classes. `C15` and edge `E08` lacked a restart test after terminal persistence with publication receipts still pending. `C17` lacked a satisfied Style Assurance basis for durable public Curation deserialization and replay because the property proof did not substitute for comparable fuzz or state-machine coverage.

## Frozen Violations And Dispositions

| Violation | Blocking claim | Disposition | Authorized correction |
| --- | --- | --- | --- |
| `WMR-VC-02-GA-V01` | `C15` and `E08` restart recovery | evidence correction | add reopen proof with both receipts missing and with only the terminal receipt missing |
| `WMR-VC-02-GA-V02` | `C17` Style Assurance completeness | evidence correction | add focused durable replay state-machine and Curation deserialization or replay fuzz evidence |

Runtime source development remained closed. The correction could change executable tests, fuzz registration, one focused fuzz target, and acceptance records only. It could not change runtime behavior, durable encoding, public semantics, authority, ownership, storage topology, the gate definition, or later-slice state.

## Correction Delivered

The restart proof interrupts publication immediately after the terminal result is flushed. It covers zero completed publication receipts and one completed semantic receipt. After store reopen, the actor reuses the exact result, performs no second traversal, appends only missing deterministic Events, and persists both final receipts.

The state-machine property runs thirty-two generated action sequences across operation, acceptance, result, receipt, query, JSON round trip, and repeated store reopen transitions. The focused fuzz target exercises public Curation rule, revision, operation, result, acceptance, and receipt deserialization together with ordered durable replay and reopen.

## Verification Evidence

```text
cargo test -p meld-world-model curation::tests -- --nocapture
cargo test -p meld standing_curation_settles_through_root_graph_events_and_belief -- --nocapture
cargo check --manifest-path crates/meld-world-model/fuzz/Cargo.toml --bin fuzz_curation_replay
cargo +nightly fuzz run fuzz_curation_replay -- -max_total_time=5
cargo clippy -p meld-world-model -p meld --all-targets -- -D warnings -A clippy::result-large-err
cargo test --workspace -- --test-threads=1
cargo fmt --all -- --check
git diff --check
```

All commands passed on the successor candidate. The fuzz campaign completed 365,964 executions in six seconds with no finding.

## Verification Limits

The verification assesses only the two frozen evidence violations and regressions caused by their correction. It adds no gate criterion, exception, runtime behavior, or authority for `SI-03`.
