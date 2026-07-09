# Event Observability PLAN

Date: 2026-07-08
Status: closed 2026-07-09; the Phase 4 publisher call is carried by the wiring workstream ledger
Workflow: complex change workflow deactivated at close per [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md)
Design source: [Event Observability Design](event_observability_design.md)

## Overview

### Objective

Complete the event observability workstream: every read model, port, command, and promoted self-observation fact the design defines, so an operator can tell what the cognitive layer is doing, whether it is progressing, and why something happened — through surfaces a future TUI or dashboard consumes as thin adapters.

### Outcome

`EventObservabilityPort` fully implemented over an in-process backing; `meld event status`, `tail`, `trace`, and `session` commands rendering text and JSON from serializable reports; a named consumer cursor registry shared with compaction; health snapshots publishable through the Wave 0 status contracts; and promoted runtime-health facts flowing into the ledger so the world model can form beliefs about the runtime's own health.

### In Scope

- `EventCursorRegistry` and registration of the existing graph reducer cursor
- the port trait, all five query surfaces, and their DTOs
- the `meld event` command family with text and JSON rendering
- health snapshot publication behind the Wave 0 `RuntimeStatusPublisher` contract
- promoted threshold facts in a `runtime` domain vocabulary with inclusion-rule discipline
- bench coverage for observability read costs

### Out of Scope

- TUI and HTTP dashboard adapters; the port is their contract, their construction is gated on want
- the daemon IPC or HTTP edge; out-of-process backings implement the same port later
- status cache file persistence and console rendering, owned by the runtime wiring workstream waves
- compaction, the eleven inert runtime handles, and anything else handed to other workstreams

## Development Phases

### Phase 0: design and naming — complete 2026-07-08

The design artifact and the rename sweep landed as the workstream's opening commits.

### Phase 1: contracts foundation

Goal: freeze the contracts every parallel unit builds against.

One integrator lands, in one checkpoint: the `EventCursorRegistry` over a well-known tree; the `EventObservabilityPort` trait with all five method signatures; every report DTO with serde derives and contract tests pinning their JSON field names; the in-process backing struct wired to store, watermark, and registry; and the `meld event` CLI skeleton — subcommand enum, dispatch, format validation — with stub handlers, so later units never touch shared parse or route files.

Key seams: new `crates/meld-events/src/events/observability.rs` for port and DTOs; `src/events_cli` or the existing CLI domain for the command skeleton per the thin adapter rule.

Exit criteria: contract tests pin every DTO's serialized shape; the CLI skeleton dispatches stubs; workspace green.

### Phase 2: surface fan-out

Goal: build the four query surfaces in parallel against the frozen contracts.

Four builder units with strictly disjoint file ownership, each delivering the port method implementation, its command handler and text formatter, and its tests:

- unit health: `EventHealthReport` computation including per-consumer lag from the registry and trailing append rates, plus `meld event status`
- unit tail: `EventPage` paging over `EventSubscription` plus `meld event tail` with follow mode
- unit trace: `EventTraceReport` causal walk over object references, relations, and stored fact id provenance, plus `meld event trace`
- unit session: `SessionTimelineReport` plus `meld event session`, and the flow view folded in: `EventFlowReport` counts by domain and type with silent-domain detection

Exit criteria: each unit's tests green; combined workspace green; every command renders text and JSON.

### Phase 3: integration and end-to-end proof

Goal: the surfaces observe real flywheel activity, not fixtures.

The integrator registers the graph reducer cursor in the registry as an alias of its traversal store position, then drives a real workspace scan plus generation flow and proves each command shows it: status reports the consumer lag closing, tail follows the events live, trace walks a real object from scan to anchor, session reconstructs the command timeline. Fixes whatever the proof exposes.

Exit criteria: a recorded end-to-end evidence note in this PLAN with real command output; workspace green; bench additions for status query and tail wake latency recorded.

### Phase 4: status snapshot publication — partial, deviation recorded 2026-07-08

Goal: health snapshots flow through the Wave 0 contract without crossing into the wiring workstream's write scope.

Implement `RuntimeStatusPublisher` publication of the health report from the supervisor tick, behind the existing Wave 0 trait; file cache persistence and console rendering remain the wiring workstream's waves. This is the coordination boundary and it is one bounded seam.

Exit criteria: supervisor tick publishes snapshots through the trait; a contract test proves shape stability; handoff note recorded for the wiring workstream.

### Phase 5: self-observation — complete 2026-07-08

Goal: the runtime emits promoted facts about its own health.

A threshold watcher over the health report emits `runtime.consumer_lag_exceeded`, `runtime.ingest_drops_burst`, `runtime.retention_gap_encountered`, and `runtime.restart_storm` facts through the durable class, each carrying the crossing evidence. The inclusion rule is enforced by design and by test: gauges and per-tick samples never enter the ledger, thresholds emit once per crossing with idempotent record ids, and a quiet runtime emits nothing.

Exit criteria: threshold facts appear in the ledger exactly once per crossing under storm tests; the world model graph reducer ignores them cleanly today; vocabulary recorded in the events domain docs.

### Phase 6: close — complete 2026-07-08

Rerun benches, record evidence, update the observability design status, close the PLAN, and write the handoff list for the wiring workstream and future adapter work.

## Orchestration

How agents and workflows execute this PLAN. Roles:

- The integrator owns contracts, shared files, integration, commits, and PLAN evidence. Phases 1, 3, 4, and 6 are integrator-led because they touch shared seams or judge end-to-end truth.
- Builder agents execute Phase 2's four units concurrently in the shared tree with disjoint file ownership, the pattern proven in the overhaul's harness phase: each agent owns exactly its module files and test files, never Cargo manifests or shared CLI files, which Phase 1 pre-wires. Each returns its diff summary and test evidence.
- Fresh-context reviewers gate every checkpoint commit, receiving the staged diff and the relevant PLAN phase with no implementation context. Single reviewer for additive read-side phases; a two-lens adversarial pair for behavior-changing phases, which here means Phase 4, because it writes from the supervisor tick, and Phase 5, because it produces ledger facts. Review findings are fixed or waived with recorded reasons before landing, and reviewers are instructed to attack test honesty and inclusion-rule discipline specifically.
- A verification agent in Phase 3 independently reproduces the end-to-end proof from the PLAN's instructions alone, so the evidence note reflects what an operator would actually see rather than what the integrator expected.

Concurrency rules: builder agents share the tree and build cache; cargo serializes compilation; file ownership is stated in every agent brief; agents run tests scoped to their units and the integrator runs the full ladder. Stability gates rerun the workspace suite at least three times at phase exits that touched concurrency-adjacent code.

Checkpoint gates, identical ladder to the overhaul: formatter first, clippy zero warnings, boundary script, scoped then full workspace tests, docs style scan on changed Markdown, DTO shape contract tests, fresh review, atomic conventional commits, no push without explicit verification.

## Verification Strategy

- Formatter check precedes all test gates per workflow governance.
- DTO stability: contract tests serialize every report and compare field names, so adapters can rely on the JSON shape; changes to shapes are called out as breaking.
- Inclusion rule: Phase 5 storm tests prove threshold facts are once-per-crossing and quiet runtimes emit nothing.
- End-to-end: Phase 3's proof drives real flywheel activity and captures real command output into this PLAN.
- Bench: observability reads must not regress ledger performance; status query cost and tail wake latency are recorded against the overhaul baselines.

## Implementation Order Summary

Contracts foundation lands first and freezes the seams. The four surface units build concurrently and integrate one by one. The end-to-end proof validates against reality. Publication and self-observation land as reviewed behavior-changing checkpoints. The close records evidence and handoffs.

## Exceptions

- The `meld event` command family is workspace-scoped read-only diagnostics; no command takes `--path` targeting, called out per [CLI Targeting Policy](../../../governance/cli_targeting_policy.md).
- `meld event tail` follow mode runs until interrupted or its output pipe closes; it is the one intentionally long-running command in the family. An interrupted follow leaves its command session without a session-ended record in the ledger, a recorded consequence of interruption-based exit; a closed pipe ends the command cleanly and completes the session. Follow holds the single-process database lock, so no concurrent meld command can produce events while it watches; the command says so on startup, and cross-process live following arrives with the daemon edge.
- Observability reads may open the product database only when no runtime process holds it; against a running daemon, `meld event status` reads published snapshots once Phase 4 and the wiring waves land.

## Phase Completion Notes

### Final polish pass — closed 2026-07-09

Four fixes landed to retire the recorded nits and close the program, with no TUI work by decision:

- The trace scan window now anchors at the ledger tip minus the scan limit, clamped to the first readable cursor, so traces favor recent activity instead of the oldest hundred thousand records. The Phase 3 disposition that accepted the old direction is superseded here.
- The restart storm threshold is now the supervisor's configured restart attempt limit passed at construction, so a storm is exactly restarts exhausting the limit; the Phase 5 nit about an unreachable storm under a lowered limit is retired. A limit of zero disables restarts entirely and correctly emits no storm fact.
- `meld event session` with an unknown id now fails with a not-found error naming both checks — no session record and no ledger events — instead of rendering an empty timeline with a success exit. A stored session with zero ledger events still renders.
- Watcher fired-state for consumers absent from the health report is dropped each observation, keeping the maps bounded by live registry names.

Gate evidence: formatter clean; clippy zero warnings; boundary script passed; full workspace green; docs scan clean. Fresh-context review approved the diff with five minor findings: the two doc amendments in this note, a rustdoc overclaim on the zero-limit storm case now corrected, an unexercised explicit-thresholds constructor now pinned by a watcher test, and an over-assertive not-found message now naming both checks honestly. Accepted without code change: the zero-event stored-session render branch and the tip-anchored trace window lack dedicated pinning tests because constructing them needs either a session store seam or a hundred-thousand-event fixture, and the retention clamp expression appears in three read surfaces — a pre-existing duplication left for whichever workstream next touches the observability module.

### Phase 6 and program close — 2026-07-08

Gate evidence: formatter clean; clippy zero warnings; boundary script passed; full workspace green at 1466 tests; the closing bench run shows every ledger number within noise of the overhaul baselines — replay at tip 0.60 microseconds and idle catch-up 0.70 microseconds at one hundred thousand events, flywheel latency 43 microseconds, eight durable producers at 12.8 milliseconds per thousand, disk at 1101 bytes per event — so the observability layer cost the ledger nothing, and the observability reads themselves hold flat at 617 microseconds for health and 1.4 microseconds for a page at tip.

Program outcome against the overview commitments: the port serves all five query surfaces over the in-process backing with every wire shape pinned; `meld event status`, `tail`, `trace`, `session`, and `flow` render text and JSON and were proven against real flywheel activity by an independent verification agent; the consumer cursor registry enumerates lag and doubles as compaction's consumer registration; the Wave 0 snapshot contract carries an optional ledger summary and the heartbeat path carries the watermark and drop diagnostics today; promoted runtime facts flow once per crossing under proven inclusion-rule discipline. The orchestration held: four concurrent builders with zero collisions, fresh reviews on every checkpoint including one caught blocker per behavior-changing phase, and an independent operator-perspective verification whose follow-mode finding shipped as an honest warning.

Handoffs: the runtime wiring workstream owns the Phase 4 publisher call — the supervisor accepting an injected `RuntimeStatusPublisher` and copying `RuntimeStatusLedgerSummary::from_health` into tick snapshots, per the coordination entry in its ledger — plus the `event.append` heartbeat surface it should consume rather than duplicate. Future adapter work builds on the port: a TUI hosts in-process today, and the browser dashboard waits on the daemon edge, both pure presentation loops. The compaction workstream inherits the registry as its consumer registration. Open nits recorded in phase notes — the unlinked storm threshold and the trace scan bound anchored at the retained boundary — were both resolved by the final polish pass noted above.

The complex change workflow deactivates for this program with the Phase 4 publisher call explicitly carried by the wiring workstream's ledger.

### Phase 5 — complete 2026-07-08

Gate evidence: formatter clean; clippy zero warnings; boundary script passed; full workspace green; six watcher tests prove the inclusion rule under storms — one thousand quiet observations emit nothing, each condition fires exactly once per crossing with an idempotent record id and re-arms only on recovery; a live supervised run confirmed three quiet ticks emit zero runtime facts while the consumer lag row stays live.

What changed: `SelfObservationWatcher` in the runtime domain observes the health report and supervisor restart counts each tick and promotes threshold crossings into the ledger durably through the append port — consumer lag exceeded, ingest drops burst, retention gap encountered per stranded consumer, and restart storm per runtime. The runtime domain vocabulary is recorded in the canonical multi-domain ledger doc. The graph reducer ignores runtime-domain events by construction since they are not traversal source events.

Semantics recorded per condition, from review: consumer lag ids embed the crossing watermark, so every genuine re-crossing within or across runs is a distinct fact. Retention gap ids embed the boundary, so a restart re-observation dedups into the original fact until compaction moves the boundary — the condition never cleared, so one fact is the truth. Restart storm ids embed the supervisor instance epoch, because restart counts reset every run and epoch-free ids would dedup every storm after the first into invisibility, the blocker the review caught. Drops ids embed the monotonic total. Emission failures leave the condition fired rather than retrying every tick, because the idempotent id makes later replay safe and a runtime that cannot append is already loud on the heartbeat path. Recorded nit: the storm threshold equals the default restart attempt limit by coincidence, and a lower configured limit makes storms unreachable; linking them is future work.

### Phase 4 — partial, deviation recorded 2026-07-08

Phase 4 as written asked the supervisor tick to publish health snapshots through the Wave 0 `RuntimeStatusPublisher` trait. This checkpoint deliberately lands the publishable shape, not the publication. `RuntimeStatusSnapshot` gains an optional, serde-defaulted ledger summary mapped by `RuntimeStatusLedgerSummary::from_health`, with its wire shape pinned by contract test and pre-field cache JSON proven to still deserialize. The `event.append` inert handle becomes a diagnostics-only semantic handle, so the existing heartbeat path carries the commit watermark as its checkpoint and drop deltas as retryable issues, visible through `meld runtime status` today.

The trait plumbing was deferred by choice, not impossibility — a test-double publisher could prove the call today — because the only real implementor is the wiring workstream's blocked Wave 1, and dead plumbing was judged worse than a recorded gap. The exit criterion "supervisor tick publishes snapshots through the trait" is not met; the phase stays open until the supervisor accepts an injected publisher and a test proves the call, here or at Wave 1 integration.

Coordination: this additively extends Wave 0's reviewed contract with an optional field that leaves old cache files readable, and it touches the event runtime reports seam that Wave 3 reserves; a coordination entry is recorded in the visibility program ledger. Content-rule check against the visibility skeleton: the ledger summary is an operational projection — sequence authority remains the ledger watermark and the cursor registry, and append rates are excluded as windowed computations — satisfying the cache's operational-projections-only rule and its prohibition on holding event sequence authority.

Review dispositions from the two-lens pair: the handle now samples its baseline on the first tick, so an existing ledger is never reported as fresh work and all-time drops are never reported as a fresh burst after a restart, with items pinned at zero because the observer commits nothing itself — checkpoint movement alone reports ledger progress. Waived with reasons: sustained real drops under an on-retryable-failure restart policy would cycle the diagnostics handle, accepted because the baseline resample on rebuild breaks the historical-drop loop the reviewers traced, the default policy restarts only on heartbeat expiry, and a genuine restart storm would itself surface through Phase 5's facts; the work budget is unused by a diagnostics-only tick; the watermark checkpoint is process-scoped and reads zero after a process restart until the first commit, recorded here for the wiring workstream's action-metric consumers; the drop-issue branch is untested because forcing a full writer queue deterministically requires counter injection, and the branch is four lines guarded by the baseline. Pinned for the future: the handle writes nothing to the ledger — per-tick drop observations are heartbeat-path diagnostics under the inclusion rule, and threshold-crossing `runtime` facts with idempotent record ids remain Phase 5's exclusive channel.

### Phase 3 — complete 2026-07-08

Gate evidence: formatter clean; clippy zero warnings; boundary script passed; full workspace green; the graph reducer mirrors its cursor into the registry after each durable advance, with the mirror opened on the ledger database in both topologies and failures degrading to a stale lag row, never a failed tick.

End-to-end evidence, real command output from a scratch workspace after one `meld scan`:

```
tip_seq:             23
committed_watermark: 23
retained_from:       1
dropped_events:      0
consumers:
  world_state.graph.reducer  cursor=22        lag=1
append_rates:
  telemetry     events=13        window=38s
  workspace_fs  events=9         window=38s
```

Flow showed all three domains with plausible counts. Tracing the snapshot object walked the promised chain: snapshot materialized, snapshot selected, node observations, and the derived anchor linked by stored fact provenance. The session timeline reconstructed the scan command completely with millisecond gaps. Tail showed the most recent records by default and the oldest with an explicit cursor.

Independent verification: a verification agent reproduced the proof from this PLAN's instructions alone and passed every surface, judging the chains coherent and operator-useful with clear error messages and stable JSON. Its one significant finding: follow mode's live-following claim was not reproducible across processes, because the follower holds the single-process database lock and starves any producer — fixed by saying so on startup and in the help text, recorded in the exception list, with cross-process following arriving with the daemon edge. Accepted with reasons: an unknown session id returns an empty timeline rather than a not-found error, since the ledger cannot distinguish a typo from a session that emitted nothing; observer commands append their own telemetry, so diagnostics sessions shift flow counts, the honest cost of the observer living in the observed ledger. The steady consumer lag of one is correct behavior: each command's final session-ended record lands after the reducer's last catch-up.

Bench evidence: health query 595 to 618 microseconds and page-at-tip 1.3 to 1.4 microseconds, both flat from one thousand to one hundred thousand events — the status command's cost is its bounded window decode, and the tail loop's wake floor is one page read. The cross-process wake latency bench is deferred to the daemon edge, where a cross-process wake first exists.

### Phase 2 — complete 2026-07-08

Gate evidence: formatter clean; clippy zero warnings; boundary script passed; full workspace green with twenty-six new surface tests across three meld-events suites plus twelve tail unit tests over a scripted port fake; all five commands live-verified against a scratch workspace including JSON rendering and subject-validation errors.

Orchestration outcome: four builder agents ran concurrently with disjoint file ownership and zero collisions; every agent reported contract friction instead of working around it, and all friction resolved at integration — the dead stub helpers retired on schedule, and the tail unit's tip-accessor friction dissolved once integration routed its defaults through the health surface.

Review dispositions, fixed at integration: the session timeline and both tail default cursors now start at the first readable cursor so no default path can trip a retention gap once a compactor raises the boundary, matching the degrade-not-fail rule the other surfaces implemented; one-shot tail defaults to the most recent records, matching its help text and its name; follow mode ends cleanly when its output pipe closes, which also completes the command session; trace text lines carry recorded time and domain so hops correlate against tail captures by eye. Accepted with reasons: clock-skew renders as a zero span in status and an absent gap in flow and session, both commented, unified rendering deferred until an operator complaint proves the inconsistency matters; the trace scan bound truncates from the retained boundary forward, so very old chains win over very new ones past one hundred thousand events — recorded for the end-to-end phase to weigh, with the bound documented on its constant; domains silent beyond the census window are invisible by documented bounded-cost design; an interrupted follow leaves its session without an ended record, recorded in the exception list.

### Phase 1 — complete 2026-07-08

Gate evidence: formatter clean; clippy zero warnings; boundary script passed; full workspace green; nine wire-shape contract tests pin every report DTO including all trace link variants; the CLI skeleton smoke-tested live with format validation preceding stub failures.

Review dispositions, fixed in-phase: registry reports are atomic through a merge so concurrent reporters can never persist a regression; the flattened export set is complete so fan-out units never touch the shared module file; every trace link variant's wire shape is pinned; the stub error for wired surfaces renders as a friendly not-implemented message rather than a storage failure; report contracts use `session_id` uniformly. Accepted with reasons: consumers that have never reported are absent from snapshots, registration-on-first-report is the convention and the docs say so; zero-limit pages are rejected rather than blocked on; `SilentDomain` carries `last_recorded_at` rather than a computed age, a presentation-free deviation from the design sketch recorded here.

### Phase 0

- Design artifact and rename sweep committed as the workstream's opening checkpoints; rename verified behavior-neutral by fresh review across all changed string literals.

## Related Documentation

- [Event Observability Design](event_observability_design.md)
- [Event Spine Overhaul PLAN](event_spine_overhaul_program.md)
- [Spine Compaction Design](spine_compaction_design.md)
- [Runtime Operator Visibility Program Ledger](../integration/runtime_operator_visibility_program_ledger.md)
- [Commit Policy](../../../governance/commit_policy.md)
