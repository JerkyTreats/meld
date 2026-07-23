# Runtime Operator Visibility Program Ledger

Date: 2026-07-04
Revised: 2026-07-12
Status: deferred and superseded as a current runtime-completion prerequisite
Program branch: `runtime-operator-visibility`

Authority: [Runtime Completion Ground Map](runtime_completion_ground_map.md) and [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) defer broad operator visibility until operational parity and bounded convergence pass. Completed evidence remains historical truth, but remaining packets are not current runtime-completion work.

## Objective

Implement runtime operator visibility so a user can tell whether `meld runtime run` started real work, stayed idle, stalled, retried, or moved the flywheel end to end.

The first objective is Wave 0 shared contract prework. Later waves must consume these shared contracts instead of inventing alternate cache, action, or status shapes.

## Historical Parent Program

The former [Production Cognitive Runtime Closure Program](production_cognitive_runtime_closure_program.md) no longer owns current delivery order. This ledger and its parent are deferred historical plans. Completed Wave 0 evidence remains historical truth, while current implementation authority stays with the operational-parity ground map and workstreams named above.

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

## Coordination Entries

### 2026-07-12 production runtime closure orchestration

The parent program adds the missing semantic activation and correctness waves around this visibility work.
Wave 1 in this ledger remains the immediate R1 implementation slice and maps into parent Wave 0.
Later visibility waves integrate only after their dependent domain actors and durability contracts pass the parent gates.

The runtime visibility program must not treat action reporting as evidence that an inert actor performs semantic work.

### 2026-07-08 event observability workstream

The event observability program additively extended the Wave 0 status contracts: `RuntimeStatusSnapshot` gains an optional, serde-defaulted `ledger` field carrying `RuntimeStatusLedgerSummary`, an operational projection of event ledger health whose authority remains the ledger watermark and cursor registry. Old cache JSON without the field still deserializes, proven by contract test. The `event.append` runtime id now builds a diagnostics-only semantic handle reporting the commit watermark and drop deltas through heartbeats; Wave 3's event runtime reports seam should consume rather than duplicate it.

Scope correction on 2026-07-10: events supplies `EventHealthReport` with stable mapping inputs. Wave 1 owns `RuntimeStatusLedgerSummary::from_health`, supervisor cadence, `RuntimeStatusPublisher` invocation, cache persistence, and staleness. Wave 3 owns heartbeat and runtime action mapping. None of those runtime tasks is an event closure gate.

### 2026-07-10 event foundation dependency correction

The canonical event architecture requires one logical ledger authority per product identity. Before E5, the CLI compatibility assembly and product runtime assembly could create separate writable event histories when `meld runtime run` passed through normal `RunContext` dispatch.

The [Event Foundation Closeout Program](../events/event_foundation_closeout_program.md) owned correctness repairs, identity-bearing authority, unified append and watermark truth, observability hardening, the remote contract seam, domain port migration, and direct product cutover. Its E5 [Product Event Authority Cutover](product_event_authority_cutover.md) owned CLI publication injection, product ledger selection, legacy history migration, and direct `meld event` routing.

Runtime visibility resumes after E6 closes the events foundation. Runtime then owns status publisher invocation and cache, real daemon transport, console and action publishers, and the full flywheel proof. The provisional `SelfObservationWatcher` does not assign promotion policy to events; a producer-owned runtime-health or sensory concern owns thresholds, hysteresis, process epochs, retries, outbox state, and promotion decisions.

### 2026-07-12 event foundation handoff

E5 product cutover and E6 event closure passed their implementation gates and fresh reviews through `9350a90`.
The main product cutover is `97cc225`; closure reliability corrections continue through `9350a90`.

The branch product identity now binds through durable `event_authority.json` `Preparing` and `Active` states to one external ledger path and identity.
The target ledger also persists the inverse branch claim, rejecting a second branch-local binding that names the same authority.
Cutover uses an `fs2` advisory lock, recoverable BLAKE3 source-to-target mappings, fail-closed path, identity, source, and marker validation, and an empty-source marker that prevents a later legacy semantic history.
Compatibility session data and configured non-event CLI storage paths remain in their existing locations, while semantic legacy event writes are rejected after cutover.
Dormant branches resolve their own configuration, source, product path, and ledger identity.

The real route tests in `tests/integration/product_event_authority_cutover.rs` prove that direct event commands and `runtime run` reuse one authority and sequence space, preserve identity across processes and reopen, leave legacy event rows unchanged, and reject a mismatched active binding without fallback.
The branch tests in `tests/integration/branches_runtime.rs` prove configured-source selection and authority isolation for dormant branches.

The event dependency is therefore satisfied and Wave 1 is ready to start.
No runtime visibility wave became complete as a side effect of event closure.
Runtime still owns R1 status publisher invocation, cache persistence and staleness; R2 daemon and real IPC; R3 console frames, runtime actions, and heartbeat mapping; and R4 the complete flywheel proof.

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

Status: ready

Summary:

- Implement status cache persistence.
- Make `runtime status` avoid full CLI context and product store locks.
- Make foreground `runtime run` publish live cache data.

Dependencies:

- `wave-0-shared-contracts`
- completed event foundation closeout E6 through `9350a90`

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
- event closure proves one direct local authority, not status-cache lock isolation

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
- `wave-1-runtime-visibility-core -> event-foundation-closeout-E6` is satisfied through `9350a90`; runtime hosting now consumes the closed direct product authority.
- `wave-2-run-console-and-lifecycle-events -> wave-1-runtime-visibility-core` because console frames should use the same cache and status row data.
- `wave-3-domain-action-publishers -> wave-0-shared-contracts` because domains must emit shared action records.
- `wave-3-domain-action-publishers -> wave-1-runtime-visibility-core` because domain actions need a cache writer and reader path.
- `wave-4-process-control -> wave-1-runtime-visibility-core` because readiness and stop behavior should publish stable status.
- `wave-5-end-to-end-proof -> wave-3-domain-action-publishers` because the proof needs domain action depth.
- `wave-5-end-to-end-proof -> event-foundation-closeout-E6` is satisfied through `9350a90`; Wave 5 must still prove the runtime-owned flywheel and operator surfaces.

## Wave Plan

Runtime resumption maps to the corrected dependency order as follows: Wave 1 is R1, Wave 4 supplies the daemon portion of R2, Waves 2 and 3 supply R3, and Wave 5 is R4. Real IPC work may extend Wave 4 without changing event contracts.

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

Event foundation dependency gate through `9350a90`:

- formatter, workspace build, workspace clippy with warnings denied, domain boundaries, and full workspace tests passed independently
- `cargo test -p meld-events --features test-support` passed in the clean CI no-lock branch
- execution and world-model package suites passed
- `product_event_authority_cutover`, `branches_runtime`, `progress_observability`, and `runtime_cli` integration suites passed
- repeated authority, cursor, concurrency, recovery, migration, and workspace reliability runs passed
- fresh migration, routing, recovery and concurrency, architecture and compatibility, and independent verification reviews passed

The repository intentionally ignores `Cargo.lock`. Clean-checkout verification therefore follows the same conditional branch as CI and omits `--locked`; the primary development worktree also passed the locked commands against its local ignored lockfile.

Route contracts handed to runtime:

- `real_cli_migrates_and_reuses_one_authority_for_event_and_runtime_routes`
- `binary_direct_commands_preserve_one_identity_across_processes`
- `real_route_rejects_a_mismatched_active_binding_without_fallback`
- `dormant_branch_migrations_keep_separate_product_authorities`
- `dormant_branch_migration_uses_its_configured_legacy_store`

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

- status: ready
- implementation evidence: none yet
- test evidence: none yet
- review status: not started
- dependency evidence: event foundation handoff passed through `9350a90`

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
- Event closure supplies reports and stable mapping inputs but does not invoke `RuntimeStatusPublisher`.
- Direct local authority routing is proven; daemon process ownership and real IPC remain unimplemented.

## Final Reconciliation

Program is active.

The event foundation dependency is reconciled and Wave 1 is unblocked.
No runtime visibility completion reconciliation has been performed because Waves 1 through 5 remain unimplemented.
