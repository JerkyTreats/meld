# Runtime Operator Visibility Program Ledger

Date: 2026-07-04
Status: active
Program branch: `runtime-operator-visibility`

## Objective

Implement runtime operator visibility so a user can tell whether `meld runtime run` started real work, stayed idle, stalled, retried, or moved the flywheel end to end.

The first objective is Wave 0 shared contract prework. Later waves must consume these shared contracts instead of inventing alternate cache, action, or status shapes.

## Source Plan

Primary plan artifact:

- `design/plan/integration/runtime_operator_visibility_plan_skeleton.md`

Supporting requirement artifacts:

- `design/plan/integration/runtime_requirements.md`
- `design/plan/integration/supervisor_runtime_requirements.md`
- `design/plan/integration/event_runtime_requirements.md`
- `design/plan/integration/execution_runtime_requirements.md`
- `design/plan/integration/world_model_runtime_requirements.md`

## Program Branch

Branch:

- `runtime-operator-visibility`

Branch note:

- The branch was created from `master` while existing dirty runtime and design changes were present.
- Those existing changes were carried forward and were not reverted.

## Phase Inventory

### wave-0-shared-contracts

Status: complete

Summary:

- Define shared runtime status cache records.
- Define runtime action records.
- Define object refs, causes, outcomes, metrics, checkpoints, issue summaries, and redaction state.
- Define publisher and reader traits for later cache persistence.
- Add focused contract tests.

Dependencies:

- Source plan skeleton.

Write scope:

- `src/runtime/contracts.rs`
- `design/plan/integration/runtime_operator_visibility_program_ledger.md`

Owner:

- central orchestrator

Implementation evidence:

- `src/runtime/contracts.rs` adds status cache records with product root, supervisor store path, status cache path, writer identity, snapshots, recent action window, and write time.
- `src/runtime/contracts.rs` adds read request and read result contracts with missing, fresh, and stale cache state derivation.
- `src/runtime/contracts.rs` adds runtime action records with object refs, causes, outcomes, metrics, checkpoint observations, issue summaries, and redaction state.
- `src/runtime/contracts.rs` adds cache publisher and reader traits for Wave 1 persistence and route work.

Test evidence:

- `cargo test runtime::contracts --lib` passed with 10 tests.
- Tests cover graph, publication bridge, publication runtime, planning runtime, agent runtime, docs evidence replay, worker action conversion, outcome issue classification, status cache JSON round trip, and cache stale state derivation.

Review status:

- first fresh review found contract and coverage gaps
- all first review findings were fixed
- follow-up fresh review passed with no remaining findings

Unresolved risks:

- concrete cache persistence is deferred to Wave 1
- CLI route isolation is deferred to Wave 1
- domain action publishers are deferred to Wave 3
- full runtime status lock behavior is not fixed until Wave 1

### wave-1-runtime-visibility-core

Status: blocked

Summary:

- Implement status cache persistence.
- Make `runtime status` avoid full CLI context and product store locks.
- Make foreground `runtime run` publish live cache data.

Dependencies:

- `wave-0-shared-contracts`

Write scope:

- `src/runtime/supervisor`
- `src/runtime/tooling.rs`
- `src/runtime/presentation.rs`
- `src/cli/route.rs`
- runtime CLI tests

Owner:

- future worker set

Implementation evidence:

- none yet

Test evidence:

- none yet

Review status:

- not started

Unresolved risks:

- lock safety must be proven with an integration test

### wave-2-run-console-and-lifecycle-events

Status: blocked

Summary:

- Print startup, tick, idle, issue, progress, and shutdown frames.
- Map supervisor lifecycle records into runtime actions.
- Bridge runtime lifecycle actions to the event spine.

Dependencies:

- `wave-1-runtime-visibility-core`

Write scope:

- runtime presentation
- runtime supervisor lifecycle mapping
- runtime event mapping
- event bridge tests

Owner:

- future worker set

Implementation evidence:

- none yet

Test evidence:

- none yet

Review status:

- not started

Unresolved risks:

- lifecycle writes must not depend on event append success

### wave-3-domain-action-publishers

Status: blocked

Summary:

- Add domain action publishers for world model, execution, event runtime, provider, workflow, publication, and docs freshness flows.

Dependencies:

- `wave-0-shared-contracts`
- `wave-1-runtime-visibility-core`

Write scope:

- world model runtime reports
- execution runtime reports
- event runtime reports
- provider and workflow progress adapters

Owner:

- future worker set

Implementation evidence:

- none yet

Test evidence:

- none yet

Review status:

- not started

Unresolved risks:

- publisher depth must not move semantic progress into the supervisor

### wave-4-process-control

Status: blocked

Summary:

- Add process records, readiness handshake, logs, shutdown request records, `runtime stop`, and optional detached launch.

Dependencies:

- `wave-1-runtime-visibility-core`

Write scope:

- runtime process contracts
- runtime tooling
- CLI parse and route
- supervisor control records
- integration tests

Owner:

- future central worker

Implementation evidence:

- none yet

Test evidence:

- none yet

Review status:

- not started

Unresolved risks:

- detached default should wait until foreground visibility and status cache are stable

### wave-5-end-to-end-proof

Status: blocked

Summary:

- Prove docs freshness and durable flywheel progress are visible from startup through execution publication and world model replay.

Dependencies:

- `wave-1-runtime-visibility-core`
- `wave-3-domain-action-publishers`

Write scope:

- integration tests
- runtime proof fixtures
- docs freshness proof artifacts

Owner:

- future worker set

Implementation evidence:

- none yet

Test evidence:

- none yet

Review status:

- not started

Unresolved risks:

- proof must distinguish idle, stalled, retrying, and progressing states

## Dependency Graph

- `wave-1-runtime-visibility-core -> wave-0-shared-contracts` because cache persistence and route isolation consume shared snapshot and action shapes.
- `wave-2-run-console-and-lifecycle-events -> wave-1-runtime-visibility-core` because console frames should use the same cache and status row data.
- `wave-3-domain-action-publishers -> wave-0-shared-contracts` because domains must emit shared action records.
- `wave-3-domain-action-publishers -> wave-1-runtime-visibility-core` because domain actions need a cache writer and reader path.
- `wave-4-process-control -> wave-1-runtime-visibility-core` because readiness and stop behavior should publish stable status.
- `wave-5-end-to-end-proof -> wave-3-domain-action-publishers` because the proof needs domain action depth.

## Wave Plan

Wave 0:

- central only
- no parallel workers
- define shared contracts before fan out

Wave 1:

- two workers after Wave 0 review passes
- supervisor cache writer
- CLI status route isolation

Wave 2:

- up to three workers with tight ownership
- console frames
- lifecycle action mapper
- runtime event bridge

Wave 3:

- domain workers in isolated scopes
- world model actions
- execution actions
- event runtime actions

Wave 4:

- central process control slice

Wave 5:

- proof worker plus fresh reviewers

## Shared Contract Decisions

- Status cache records are observation data only.
- Runtime action records are observation data only.
- Worker checkpoints may be displayed but must not become supervisor resume cursors.
- Cache publisher and reader are traits in `src/runtime/contracts.rs`.
- Concrete file persistence is deferred to Wave 1.
- Worker reports are serializable so cache and action writers can persist bounded summaries.
- Runtime object refs are compact ids, not borrowed domain internals.
- Provider and workflow action records must use redaction state before any payload is surfaced.

## Wave Execution Log

### Wave 0

Ready items:

- shared runtime status and action contracts
- shared program ledger

Parallelization decision:

- central only because later workers must consume one contract shape

Workers launched:

- none

Commands run:

- `cargo fmt --check`
- `cargo test runtime::contracts --lib`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Commits accepted:

- none yet

Commits rejected:

- none

Conflicts:

- none

Gate results:

- `cargo fmt --check` passed
- `cargo test runtime::contracts --lib` passed with 10 tests
- `cargo check --workspace` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

Review results:

- first fresh review found five findings
- all findings fixed in `src/runtime/contracts.rs`
- follow-up review passed with no remaining findings

Next ready set:

- `wave-1-runtime-visibility-core`

## Gate Evidence

Wave 0 focused gates:

- `cargo fmt --check` passed after applying rustfmt
- `cargo test runtime::contracts --lib` passed with 10 tests

Broader gates:

- `cargo check --workspace` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Review Findings

Wave 0 first fresh review findings:

- fixed: cache record omitted `supervisor_store_path`
- fixed: reader contract lacked shared stale derivation
- accepted with evidence: root runtime maps public domain report contracts into supervisor-facing diagnostics, and these mappings predated Wave 0
- fixed: worker action checkpoints collapsed input and output checkpoint names
- fixed: worker action object ids could fall back to only `work_key`
- fixed: focused tests were missing for publication runtime, planning runtime, agent runtime, and docs evidence replay
- fixed: outcome classification paths were undercovered

Wave 0 follow-up fresh review passed with no remaining findings.

## Phase Completion Matrix

Wave 0:

- status: complete
- implementation evidence: `src/runtime/contracts.rs`
- test evidence: `cargo test runtime::contracts --lib`
- review status: passed

Wave 1:

- status: blocked
- implementation evidence: none yet
- test evidence: none yet
- review status: not started

Wave 2:

- status: blocked
- implementation evidence: none yet
- test evidence: none yet
- review status: not started

Wave 3:

- status: blocked
- implementation evidence: none yet
- test evidence: none yet
- review status: not started

Wave 4:

- status: blocked
- implementation evidence: none yet
- test evidence: none yet
- review status: not started

Wave 5:

- status: blocked
- implementation evidence: none yet
- test evidence: none yet
- review status: not started

## Risks And Exceptions

- Existing dirty runtime and design files predated this Wave 0 task and were not reverted.
- The ledger is bootstrapped before Wave 1, so later workers must update evidence as they land work.
- Wave 0 is complete, but later waves still need their own review lanes.
- Runtime status still uses the old blocking route until Wave 1.

## Final Reconciliation

Program is active.

No final reconciliation has been performed.
