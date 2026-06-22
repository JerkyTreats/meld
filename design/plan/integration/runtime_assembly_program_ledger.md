# Runtime Assembly Program Ledger

Date: 2026-06-21
Program branch: runtime-assembly-implementation
Status: in progress

## Objective

Finish the durable runtime assembly phased plan from root assembly through proof and supervisor foundation.

## Source plan

- `design/plan/integration/durable_flywheel_runtime_phase_design.md`
- `design/plan/integration/runtime_requirements.md`
- `design/plan/integration/world_model_runtime_requirements.md`
- `design/plan/integration/execution_runtime_requirements.md`
- `design/plan/integration/event_runtime_requirements.md`
- `design/plan/integration/supervisor_runtime_requirements.md`
- `design/plan/integration/durable_flywheel_vertical_proof_requirements.md`
- `design/plan/integration/runtime_implementation_PLAN.md`

## Program branch

`runtime-assembly-implementation`

## Phase inventory

| ID | Source phase | Summary | Status | Dependencies | Write scope | Owner | Implementation evidence | Test evidence | Review status | Risks |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| phase-0 | Durable flywheel Phase 0 | Document realignment | complete | none | design docs | prior work | design docs aligned to embedded runtimes | source scans in Phase 1 ledger | passed | none |
| phase-1 | Durable flywheel Phase 1 | Product runtime assembly | complete | phase-0 | `src/runtime` | prior work | `ProductRuntimeAssembly` and runtime ports | `runtime_implementation_PLAN.md` gate evidence | passed | none |
| phase-2 | Durable flywheel Phase 2 | Supervisor diagnostics contract | complete | phase-1 | `src/runtime/contracts.rs` | prior work | `WorkerTickReport` maps graph and publication reports | `cargo test runtime::contracts` in Phase 1 ledger | passed | none |
| phase-3 | Durable flywheel Phase 3 | World model embedded runtime | complete | phase-1 phase-2 | `crates/meld-world-model` | Copernicus | `AgentGoalCurationRuntime` and `AgentSatisfactionCurationRuntime` persist decisions, durable sink receipts, and active goal query reload boundaries | `cargo test -p meld-world-model --test agent` passed | review findings addressed | none |
| phase-4 | Durable flywheel Phase 4 | Execution embedded runtime | complete | phase-1 phase-2 phase-3 ports | `crates/meld-execution` | Lorentz | `PlanningRuntimeActor` and `PublicationRuntime` provide bounded actor facades with repeated tick and sustained outage idempotency coverage | `cargo test -p meld-execution --test planning_runtime`; `cargo test -p meld-execution --test task_network_publication_bridge` passed | review findings addressed | deterministic proof still uses pinned task mutation helper |
| phase-5 | Durable flywheel Phase 5 | Event led feedback | complete | phase-3 phase-4 | integration tests and `src/runtime/ports.rs` | orchestrator | success proof uses curation runtime, publication runtime, bounded docs task evidence replay port, satisfaction runtime, and mutation port | success, checkpoint, and failure proof gates passed | review findings addressed | planning proof keeps pinned fixture mutation helper because actor-lowered task ids differ from fixture ids |
| phase-6 | Durable flywheel Phase 6 | Deterministic proof harness | complete | phase-1 through phase-5 | `tests/integration` | orchestrator | `minimal_runtime_flywheel_turn_persists_and_satisfies_goal` exercises durable receipts, replay evidence port, and satisfaction mutation sink | success proof and checkpoint tests pass | review findings addressed | pinned task helper remains until fixture ids move to actor-lowered ids |
| phase-7 | Durable flywheel Phase 7 | Failure path proof | complete | phase-5 phase-6 | `tests/integration` | prior work | `failure_outcome_does_not_satisfy_goal` | focused test passes | review findings addressed | none |
| phase-8 | Durable flywheel Phase 8 | Runtime supervisor entrypoint | complete | phase-1 phase-2 | `src/runtime/supervisor*` | Feynman | supervisor contracts, store trees, leases, heartbeats, health snapshots, restart and shutdown records, explicit lifecycle entrypoint, status surface, shutdown, recovery, and restart evaluation | `cargo test runtime::supervisor --lib` passed | entrypoint reviewed by focused tests | none |
| phase-9 | Durable flywheel Phase 9 | CLI adapter | ready | phase-6 phase-8 | `src/cli/runtime_assembly.rs` | unassigned | thin assembly CLI exists | pending CLI adapter tests | pending | CLI must delegate to supervisor entrypoint |

## Dependency graph

- `phase-1 -> phase-0` because assembly requires realigned runtime boundaries.
- `phase-2 -> phase-1` because diagnostics use root runtime contracts.
- `phase-3 -> phase-1` because world model runtimes consume assembled ports and stores.
- `phase-4 -> phase-3` because execution planning consumes world model planner projection.
- `phase-5 -> phase-3` because event feedback updates world model graph, belief, and satisfaction.
- `phase-5 -> phase-4` because event feedback starts from execution publication.
- `phase-6 -> phase-5` because the harness proves feedback across reopen checkpoints.
- `phase-7 -> phase-6` because failure proof uses the same harness shape.
- `phase-8 -> phase-1` because supervisor consumes assembly startup resources.
- `phase-9 -> phase-6` because CLI must wait for the library proof.
- `phase-9 -> phase-8` because CLI should delegate to supervisor and assembly.

## Wave plan

Wave 1:

- Implement supervisor contracts and store operations under root runtime.
- Add the missing success vertical proof test under integration tests.
- Run focused runtime, integration, and source scan gates.

Wave 2:

- Deepen domain runtime facades only where Wave 1 proof exposes a real gap.
- Add CLI adapter only after library proof and supervisor status are stable.

## Shared contract decisions

- Root assembly and supervisor remain infrastructure only.
- Domain stores remain authoritative for semantic cursors, goals, beliefs, task networks, publications, and event sequence.
- The deterministic proof harness is test support and must not create product scheduler modules.
- Supervisor records may store bounded diagnostics as observations only.

## Wave execution log

Wave 1 started on 2026-06-21.

Ready items:

- phase-6
- phase-8

Parallelization decision:

- Root supervisor files and integration proof files are disjoint enough for parallel work.

Workers launched:

- Feynman for supervisor contracts and store operations
- orchestrator for vertical proof harness

Commands run:

- `cargo test --test integration_tests minimal_runtime_flywheel_turn_persists_and_satisfies_goal`
- `cargo test --test integration_tests docs_freshness_reopens_after`
- `cargo test --test integration_tests failure_outcome_does_not_satisfy_goal`
- `cargo test runtime::supervisor --lib`

Commits accepted:

- pending

Commits rejected:

- none

Conflicts:

- none

Gate results:

- success vertical proof passed
- three checkpoint reopen tests passed
- failure proof passed
- supervisor unit tests passed

Review results:

- pending

Next ready set:

- phase-3, phase-4, and phase-5 actor facade hardening before CLI

Wave 2 started on 2026-06-21.

Ready items:

- phase-3
- phase-4
- phase-5

Parallelization decision:

- World model agent runtime facade and execution actor facade writes are disjoint by crate.

Workers launched:

- Copernicus for world model agent runtime facade
- Lorentz for execution planning and publication runtime facades

Commands run:

- `cargo test -p meld-world-model --test agent`
- `cargo test -p meld-execution --test planning_runtime`
- `cargo test -p meld-execution --test task_network_publication_bridge`
- `cargo test --test integration_tests minimal_runtime_flywheel_turn_persists_and_satisfies_goal`
- `cargo test --test integration_tests docs_freshness_reopens_after`
- `cargo test --test integration_tests failure_outcome_does_not_satisfy_goal`
- `cargo check --workspace`
- `cargo fmt --check`

Commits accepted:

- pending

Commits rejected:

- none

Conflicts:

- none

Gate results:

- world model agent runtime facade tests passed
- execution planning actor tests passed
- execution publication runtime tests passed
- success proof and checkpoint proof tests passed
- workspace check passed

Review results:

- plan coverage review found event evidence, planning proof, sink receipt, and non-composed planning gaps
- architecture review found sink receipt durability, active goal query freshness, and satisfaction proof shortcut gaps
- regression review found post sink crash recovery, repeated planning tick, and sustained publication outage idempotency gaps
- all blocking world model, publication, evidence replay, and satisfaction proof gaps were patched
- planning proof pinned task helper remains as an explicit residual because actor-lowered task ids differ from fixture ids

Next ready set:

- phase-9 after CLI adapter ownership is assigned

## Gate evidence

| Gate | Command | Result | Evidence date | Notes |
| --- | --- | --- | --- | --- |
| initial status | `git status --short --branch` | passed | 2026-06-21 | branch `runtime-assembly-implementation` |
| success proof | `cargo test --test integration_tests minimal_runtime_flywheel_turn_persists_and_satisfies_goal` | passed | 2026-06-21 | 1 passed |
| checkpoint proof | `cargo test --test integration_tests docs_freshness_reopens_after` | passed | 2026-06-21 | 3 passed |
| failure proof | `cargo test --test integration_tests failure_outcome_does_not_satisfy_goal` | passed | 2026-06-21 | 1 passed |
| supervisor foundation | `cargo test runtime::supervisor --lib` | passed | 2026-06-21 | 8 passed |
| supervisor entrypoint | `cargo test runtime::supervisor --lib` | passed | 2026-06-21 | 18 passed |
| runtime assembly handle hooks | `cargo test runtime::assembly --lib` | passed | 2026-06-21 | 20 passed |
| runtime storage boundary | `cargo test runtime::storage --lib` | passed | 2026-06-21 | 1 passed |
| world model actor facade | `cargo test -p meld-world-model --test agent` | passed | 2026-06-21 | 40 passed |
| execution planning actor | `cargo test -p meld-execution --test planning_runtime` | passed | 2026-06-21 | 14 passed |
| execution publication actor | `cargo test -p meld-execution --test task_network_publication_bridge` | passed | 2026-06-21 | 9 passed |
| workspace check | `cargo check --workspace` | passed | 2026-06-21 | dev profile |
| workspace test | `cargo test --workspace` | passed | 2026-06-21 | full workspace and doctests passed |
| full local test | `cargo test -q` | passed | 2026-06-21 | 288 lib tests, 388 integration tests, and all focused binaries passed |
| format | `cargo fmt --check` | passed | 2026-06-21 | no output |
| runtime file layout scan | `find crates/meld-world-model/src crates/meld-execution/src crates/meld-events/src src/runtime src/cli -name mod.rs -print` | passed | 2026-06-21 | no output |
| supervisor cursor scan | `rg -n "supervisor-held semantic cursor|shared cursor table|supervisor-held source cursor" src/runtime crates/meld-world-model/src crates/meld-execution/src crates/meld-events/src` | passed | 2026-06-21 | no matches |
| semantic loop scan | `rg -n "centralized semantic loop|convergence loop" src/runtime crates/meld-world-model/src crates/meld-execution/src crates/meld-events/src` | passed | 2026-06-21 | no matches |

## Review findings

Addressed:

- Durable sink result identity is now persisted by the agent store and reported by runtime ticks.
- Agent goal curation and satisfaction runtime facades can reload active goals through an execution query port at the curation boundary.
- Post sink crash recovery is covered for goal curation when execution already exposes the accepted goal.
- Satisfaction replay with a durable receipt no longer requires a replayable mutation command.
- Repeated planning ticks skip already materialized lowered work instead of producing duplicate command rejection.
- Repeated publication append failures remain retryable across revision changes.
- The vertical proof now routes docs task event evidence through `DocsTaskEvidenceReplayPort` instead of carrying promoted evidence in the harness.
- The publication append before satisfaction proof now uses `AgentSatisfactionCurationRuntime` and the product goal mutation port.

Residual:

- The deterministic proof still applies the pinned task network mutation helper for the docs task because the actor lowerer derives different task ids than the first proof fixture constants.
- Phase 8 uses an explicit lifecycle entrypoint rather than a background scheduler loop.
- The CLI adapter remains unimplemented and should delegate to the supervisor entrypoint.

## Phase completion matrix

| Phase | Status | Evidence |
| --- | --- | --- |
| phase-0 | complete | prior Phase 1 ledger |
| phase-1 | complete | prior Phase 1 ledger |
| phase-2 | complete | prior Phase 1 ledger |
| phase-3 | complete | agent runtime facade gate passes |
| phase-4 | complete | planning and publication runtime facade gates pass |
| phase-5 | complete | proof uses actor facades for goal curation, publication, and satisfaction |
| phase-6 | complete | success proof and checkpoint tests pass |
| phase-7 | complete | failure proof passes |
| phase-8 | complete | supervisor entrypoint, status, shutdown, recovery, restart evaluation, and store tests pass |
| phase-9 | ready | pending CLI adapter over supervisor command surface |

## Risks and exceptions

- The deterministic proof has one explicit fixture shortcut for task network planning. Actor coverage exists in `meld-execution`, but the end to end proof still uses pinned task ids.
- CLI adapter work remains as the next staged implementation over the supervisor entrypoint.

## Final reconciliation

Library proof and actor facade hardening are complete for phases 3 through 7, with the pinned planning fixture shortcut recorded as residual. Phase 8 is complete through an explicit lifecycle entrypoint that starts, observes, stops, recovers expired leases, evaluates conservative restart policy, and exposes status without a background daemon. Phase 9 is ready for a thin CLI adapter over that supervisor surface.
