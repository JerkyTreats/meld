# Event Observability PLAN

Date: 2026-07-08
Status: active
Workflow: complex change workflow active per [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md)
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

### Phase 4: status snapshot publication

Goal: health snapshots flow through the Wave 0 contract without crossing into the wiring workstream's write scope.

Implement `RuntimeStatusPublisher` publication of the health report from the supervisor tick, behind the existing Wave 0 trait; file cache persistence and console rendering remain the wiring workstream's waves. This is the coordination boundary and it is one bounded seam.

Exit criteria: supervisor tick publishes snapshots through the trait; a contract test proves shape stability; handoff note recorded for the wiring workstream.

### Phase 5: self-observation

Goal: the runtime emits promoted facts about its own health.

A threshold watcher over the health report emits `runtime.consumer_lag_exceeded`, `runtime.ingest_drops_burst`, `runtime.retention_gap_encountered`, and `runtime.restart_storm` facts through the durable class, each carrying the crossing evidence. The inclusion rule is enforced by design and by test: gauges and per-tick samples never enter the ledger, thresholds emit once per crossing with idempotent record ids, and a quiet runtime emits nothing.

Exit criteria: threshold facts appear in the ledger exactly once per crossing under storm tests; the world model graph reducer ignores them cleanly today; vocabulary recorded in the events domain docs.

### Phase 6: close

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
- `meld event tail` follow mode runs until interrupted; it is the one intentionally long-running command in the family.
- Observability reads may open the product database only when no runtime process holds it; against a running daemon, `meld event status` reads published snapshots once Phase 4 and the wiring waves land.

## Phase Completion Notes

### Phase 0

- Design artifact and rename sweep committed as the workstream's opening checkpoints; rename verified behavior-neutral by fresh review across all changed string literals.

## Related Documentation

- [Event Observability Design](event_observability_design.md)
- [Event Spine Overhaul PLAN](event_spine_overhaul_program.md)
- [Spine Compaction Design](spine_compaction_design.md)
- [Runtime Operator Visibility Program Ledger](../integration/runtime_operator_visibility_program_ledger.md)
- [Commit Policy](../../../governance/commit_policy.md)
