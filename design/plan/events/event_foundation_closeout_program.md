# Event Foundation Closeout Program

Date: 2026-07-12
Status: closed 2026-07-12
Scope: close event correctness, authority, observability, compatibility, and direct product routing before runtime hosting resumes
Workflow: complex change workflow deactivated at close per [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md)
Branch: `event-foundation-closeout`

## Objective

Finish `meld-events` as a correct, observable, identity-bearing product foundation that does not require a supervisor.

Runtime consumes and hosts event capabilities after this program closes.
Runtime scheduling, cache publication, daemon process ownership, and semantic runtime visibility do not define event closure.

## Dependency Direction

```mermaid
flowchart LR
    EH[Event authority and observability hardening] --> PC[Product CLI authority cutover]
    PC --> EC[Events foundation closed]
    EC --> RV[Runtime visibility and daemon work resumes]
    RV --> SF[Semantic flywheel proof]
```

The product cutover is part of event foundation closure because it proves that the authority is the product truth rather than an isolated crate abstraction.
Out-of-process access follows event closure because a real remote client needs a process host.

## Predecessors

This program succeeds the completed [Event Spine Overhaul PLAN](event_spine_overhaul_program.md) and [Event Observability PLAN](event_observability_program.md).
Those programs established the ledger mechanics and first observability surfaces.
Their completion evidence remains valid, and their remaining event-owned gaps were closed here.

The [Product Event Authority Cutover](../integration/product_event_authority_cutover.md) is the detailed E5 migration and root composition plan.
The [Product Event Authority Assessment](../integration/product_event_authority_domain_assessment.md) records the cross-domain integration gaps.

## Ownership Boundary

| Concern | Event closeout owner | Runtime owner |
| --- | --- | --- |
| durable ledger identity | define and validate | consume |
| append, replay, subscription, watermark, and observability capabilities | define and implement | host and route |
| health, flow, trace, session, and paging truth | compute over one authority | publish or display |
| remote access | transport-neutral contract and loopback conformance | process host, IPC framing, reconnects, endpoints, and authentication |
| health promotion | durably append a supplied promoted fact | supply raw health and restart signals |
| promotion policy | no canonical policy | producer-owned runtime-health or sensory concern |
| operator status | stable health report and mapping inputs | status mapping, tick cadence, publisher invocation, cache, staleness, console, and action records |

## Closure Invariants

- One product identity resolves to one persisted `LedgerIdentity` and one `EventAuthority`.
- Every canonical append, including graph-derived and execution events, advances one durable sequence and one notification source.
- Append, replay, subscription, watermark, cursor lag, and observability reject identity mismatch instead of combining sequence spaces.
- Production domains cannot open an alternate writable canonical event store.
- Every bounded read reports the range it covered or that its answer was truncated.
- Provenance follows structural event contracts and never depends on arbitrary payload string inspection.
- Direct `meld event` commands read the configured product authority.
- A real command route proves one ledger identity and one event sequence across CLI and product assembly.

## Orchestration

The root integrator owns shared contracts, module exports, Cargo manifests, plan evidence, staging, commits, and phase gates.
Builder agents receive disjoint file ownership, run focused tests, and do not commit.
Fresh reviewers must not have authored the reviewed package and inspect the staged diff against the phase requirements and repository policies before each checkpoint.

At most three builders run beside the integrator.
Parallel lanes freeze shared contracts before fan-out, and Cargo gates run serially after integration.
The integrator stages only named phase files so unrelated work cannot enter a checkpoint.

Every source checkpoint runs the formatter before build, lint, boundary, focused test, and workspace test evidence.
Behavior-changing phases receive two independent review lenses.
E6 adds an independent verification agent that executes the documented acceptance commands from a clean working tree.

Commits use declarative conventional subjects under [Commit Policy](../../../governance/commit_policy.md).
Breaking changes use a `!` marker and a `BREAKING CHANGE:` footer.
No push occurs without separate user verification.

## Development Phases

### E0 — Scope Correction — complete 2026-07-10

Correct the event and runtime plans so runtime hosting is downstream of event closure.

Tasks:

- Remove `RuntimeStatusPublisher` invocation from event observability closure.
- Keep `EventHealthReport` and stable status mapping inputs on the event side.
- Move supervisor cadence, status cache persistence, staleness, console rendering, action records, and heartbeat mapping to runtime visibility.
- Move daemon lifecycle and real remote transport to runtime.
- Mark threshold, hysteresis, process epoch, retry, outbox, and promotion decisions as producer-owned policy.
- Record `SelfObservationWatcher` as a provisional compatibility implementation.

Exit criteria: no event phase or gate depends on supervisor scheduling, status cache publication, or a daemon host.

Evidence: branch `event-foundation-closeout` created from the local event-observability head with the existing design work preserved. Formatter, Markdown link, Markdown parentheses, and diff checks passed. The pre-closeout benchmark baseline recorded replay at tip at 0.61 microseconds for one hundred thousand events, idle catch-up at 0.72 microseconds, health at 0.66 milliseconds, page at tip at 1.48 microseconds, eight durable writers at 12.64 milliseconds per thousand, flywheel throughput at 37.63 milliseconds per two thousand events, flywheel latency at 54.68 microseconds, and storage at 1153 bytes per event.

### E1 — Correctness Regressions — complete 2026-07-12

Repair and pin the remaining correctness defects before authority construction hides them behind a larger surface.

Tasks:

- Enforce exact session partitioning and add delimiter-collision regressions.
- Make consumer cursor advancement atomic and prove it under concurrent writers and reopen.
- Enforce bounded page, flow, and trace limits, including malformed and extreme limits.
- Correct relation-only trace discovery.
- Define trace coverage and truncation semantics explicitly in report contracts.
- Add direct regressions for each repaired behavior.

Exit criteria: focused concurrency, reopen, malformed-limit, session-isolation, and trace suites pass without ignored known defects.

Evidence: commits `b9ba7a3` and `c09c2ae` enforce exact session isolation, atomically advance durable cursors, bound every observability read, correct relation-only traces, and expose trace coverage. The cursor suite passed 25 consecutive normal runs and 25 consecutive serial runs as part of the closure reliability gate.

### E2 — Authority Core — complete 2026-07-12

Make identity and construction authority structural.

Tasks:

- Persist `LedgerIdentity` in canonical ledger metadata and recover the same identity after reopen.
- Introduce one `EventAuthority` aggregate that owns writable construction.
- Derive identity-bearing append, replay, subscription, watermark, and observability capabilities from that aggregate.
- Reject identity mismatch, duplicate binding, and split-brain writable open attempts.
- Route every canonical append through the authority writer and its notification source.
- Initialize the committed watermark from durable state after reopen.
- Seal production constructors so domains cannot build writable `EventStore` or writer instances directly.
- Keep explicit narrow constructors for tests, migration, and read-only compatibility only.

Exit criteria: capability identity agrees across reopen, alternate writable construction is unavailable to production domains, and every append path advances one sequence and watermark.

Evidence: commits `a20390d` and `215ef32` persist and validate ledger identity, derive all capabilities from one authority, recover the durable watermark, reject mismatches and duplicate process-local bindings, and return identity-bearing append and replay results. The authority suite passed 25 consecutive normal runs and 25 consecutive serial runs with 16 tests per run. Production raw event constructors are sealed behind event-owned internals and explicit test support.

### E3 — Observability Hardening And Remote Contract — complete 2026-07-12

Make every event read honest about authority, durability, and coverage.

Tasks:

- Run health, flow, trace, session, and paging over one supplied authority.
- Carry `LedgerIdentity` on every report and reject mismatched requests or cursors.
- Report scanned range and truncation state for every bounded result.
- Compute durable consumer lag against durable authority state.
- Add reopen, retention, direct append, notification, and malformed-limit tests.
- Replace payload string provenance inference with explicit object references, relations, and record provenance.
- Define transport-neutral requests, responses, durability outcomes, identity validation, and error semantics.
- Provide one reusable conformance suite for local and remote authority implementations.
- Prove the remote contract with a loopback adapter that does not require a daemon.

Exit criteria: local and loopback implementations pass the same conformance suite, and every report names its ledger and coverage honestly.

Evidence: commits `303d9c1`, `d19d742`, and `01e4f59` add ledger identity and coverage to every report, replace payload-string inference with structural source-record provenance, and define the transport-neutral authority contract. The local and serde loopback clients passed the same reusable conformance suite. The feature-enabled event suite passed 172 tests and 3 doctests.

### E4 — Domain Port Migration — complete 2026-07-12

Move production callers onto authority capabilities without moving domain meaning into events.

Tasks:

- Preserve execution's publication sink contract and satisfy it with a thin root adapter over the authority append capability.
- Preserve world model replay requirements and satisfy them with a thin root adapter over the authority replay capability.
- Route graph-derived appends through the authority capability.
- Remove production domain calls that append directly to `EventStore`.
- Carry ledger identity through replay cursors and append receipts where cross-authority mixing could occur.

Exit criteria: execution and world model retain their domain contracts, all canonical production writes use the authority, and boundary checks find no cross-domain reach into event internals.

Evidence: commits `3c6b26d`, `e73dfa0`, and `b7781a2` bind execution publication receipts to ledger identity, move world model replay and derived publication behind domain-owned ports, and adapt root product ports to authority capabilities. Domain boundary checks reject production construction of raw event writers, stores, and graph runtimes.

### E5 — Product Cutover — complete 2026-07-12

Make the configured product authority the single-process product truth.

Tasks:

- Characterize and migrate existing CLI event history under compatibility policy.
- Inject the authority appender into `ProgressRuntime`.
- Stop `CliRuntimeAssembly` from constructing canonical event storage.
- Make `ProductRuntimeAssembly` consume the resolved authority.
- Make legacy event trees read-only after successful cutover.
- Route direct `meld event` commands to the configured product authority.
- Start `meld runtime run` through the real command route and prove one ledger identity and one monotonic sequence.

Exit criteria: the [Product Event Authority Cutover](../integration/product_event_authority_cutover.md) gates pass for direct single-process product routing and no compatibility event tree receives new semantic writes.

Evidence: commit `97cc225` delivers external product storage, recoverable legacy migration, durable product binding, injected telemetry capabilities, shared product assembly, direct CLI authority routing, and raw-constructor sealing. Commits `1d79028`, `9a18276`, `b6e4e55`, `f1021a2`, and `bdd5119` remove global XDG state from absolute-root resolution and harden bounded sled reopen behavior in world-model and migration verification. Commit `172166e` persists the inverse branch product claim in the target ledger and rejects two branch-local bindings that name one physical authority. Commit `95d917c` keeps the observability wire-contract suite valid with and without explicit test support. Commits `2a6243e` and `b86d26e` remove full-suite belief and agent store reopen races with narrowly bounded test-only lock-release retries. Commit `6f06d93` makes the task-network factory itself tolerate the same bounded lock-release window with deterministic retry classification tests. Commit `9350a90` preserves the head-index complexity threshold while using median samples to reject scheduler outliers. Migration stress passed 50 consecutive local runs and 25 consecutive independent runs. Fresh migration, routing, constructor-sealing, product-binding, reopen, and timing reviews found no remaining blocker or should-fix finding.

### E6 — Closure — complete 2026-07-12

Run the complete event gate ladder against the integrated authority and product cutover.

Tasks:

- Run formatter, lint, domain boundary, focused crate, and full workspace gates.
- Repeat concurrency and recovery suites across clean reopen cycles.
- Compare event and observability benchmarks with the overhaul baselines.
- Run the reusable authority conformance suite against local and loopback adapters.
- Obtain a fresh adversarial review focused on identity, split brain, durability, bounded reads, migration parity, and test honesty.
- Reconcile event readiness and cross-domain assessments with the implemented state.

Exit criteria: all E1 through E5 gates pass, no event closure gate names runtime scheduling or real daemon transport, and the events foundation is marked closed.

Evidence: formatter, workspace build, warning-denying workspace clippy, domain boundaries, focused crate suites, route-level integration suites, and the full workspace all-targets suite passed. The full workspace suite passed three consecutive times. Authority, cursor, concurrency, and recovery suites passed 25 consecutive normal runs and 25 consecutive serial runs, with 16 authority integration tests, 3 cursor tests, 5 concurrency tests, and 9 recovery tests per run. Five authority unit tests additionally prove durable same-product reopen, cross-product rejection, and successful retry after an indeterminate first claim flush.

An independent verifier reproduced the clean CI no-lock ladder at `9350a90` on its first final-checkpoint run, including the full workspace all-targets suite, static constructor and module audits, and 25-run task-network reopen and head-index timing stress.

Fresh recovery, concurrency, architecture, compatibility, migration, routing, and constructor-sealing reviews found no unresolved blocker or should-fix finding. The reusable local and serde loopback conformance clients passed identical authority assertions.

Benchmark comparison against the pre-closeout baseline:

| Measurement | Baseline | Closing result | Change |
| --- | ---: | ---: | ---: |
| replay at tip | 0.610 µs | 0.625 µs | +2.5 percent |
| idle catch-up | 0.720 µs | 0.603 µs | -16.3 percent |
| health | 0.660 ms | 0.662 ms | +0.3 percent |
| page at tip | 1.480 µs | 0.715 µs | -51.7 percent |
| eight durable writers | 12.640 ms | 13.170 ms | +4.2 percent |
| flywheel throughput | 37.630 ms | 37.349 ms | -0.7 percent |
| flywheel latency | 54.680 µs | 52.701 µs | -3.6 percent |
| storage per event | 1153 bytes | 1101 bytes | -4.5 percent |

Every protected measurement remained within the ten percent closure threshold.

## Self-Observation Rule

Events owns durable append of a promoted health fact presented through the authority append capability.
Events does not decide when raw runtime health becomes a semantic fact.

A producer-owned runtime-health or sensory concern owns:

- thresholds and hysteresis
- process epochs
- retry and outbox state
- promotion decisions

Runtime supplies raw health and restart signals.
The existing `SelfObservationWatcher` may be hardened as a compatibility bridge, but it remains provisional and is not canonical event behavior or an event closure gate.

## Runtime Resumption

With E6 closed, runtime work resumes in this order:

- R1 — invoke the status publisher and persist the status cache
- R2 — host the authority behind the daemon and implement real IPC
- R3 — publish console frames and runtime action records
- R4 — prove the complete semantic flywheel and operator visibility path

Runtime owns supervisor tick cadence, status staleness, `runtime status`, console frames, `event.append` heartbeat and action mapping, daemon lifecycle, process ownership, reconnect behavior, endpoint lifecycle, authentication, and direct-versus-daemon process tests.

## Non Goals

- Implement supervisor scheduling to satisfy a historical observability phase.
- Persist or render the runtime status cache.
- Build the daemon or select an IPC framing protocol.
- Define threshold or hysteresis policy in `meld-events`.
- Complete sensory implementation or the full semantic flywheel.
- Treat the loopback remote adapter as a process integration test.

## Verification Policy

Compatibility work follows [Compatibility Policy](../../../governance/compatibility_policy.md) and [Compatibility Shim Policy](../../../governance/compatibility_shim_policy.md).
Migration preserves semantic units under [Semantic Unit Preservation Policy](../../../governance/semantic_unit_preservation_policy.md).
Every checkpoint ran the relevant focused tests before the full workspace gate. The phase evidence above records the final integrated result.

## Read With

- [Events Readiness Assessment](assessment.md)
- [Event Observability Design](event_observability_design.md)
- [Event Runtime Requirements](../integration/event_runtime_requirements.md)
- [Product Runtime Assembly Requirements](../integration/product_runtime_assembly_requirements.md)
- [Runtime Operator Visibility Program Ledger](../integration/runtime_operator_visibility_program_ledger.md)
