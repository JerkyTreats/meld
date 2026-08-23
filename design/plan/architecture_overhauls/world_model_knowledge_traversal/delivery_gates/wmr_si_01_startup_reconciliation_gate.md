# WMR-SI-01 Startup Reconciliation Delivery Gate

Date: 2026-08-23

Gate identifier: `WMR-SI-01-DG`

Revision: 1

Status: frozen for the authorized source slice

Active slice: `WMR-SI-01`

Intended handoff: non-blocking Startup runtime proof to later product migration

Gate owner: dedicated acceptance reviewer

Exception authority: user

## Coherence Horizon

The horizon begins with one compiled and activated `meld_startup` product under a fresh open admission epoch. It closes with one Agent-owned Startup Goal satisfaction receipt, a complete `startup_nonce_account`, and truthful native owner positions for any still-unresolved Execution or lifecycle obligation.

The horizon includes the exact accepted Startup path, native restart and replay positions exercised by that path, and the minimum root composition needed to run it. Docs Freshness, Dependency Security, fail-closed dependent-product activation, legacy Workflow, and unrelated runtime cleanup are outside the horizon.

## Required Deliverables

| Deliverable | Owning domain | Required product |
| --- | --- | --- |
| Startup product and assignment | PDS | exact package, assignment, compilation, and genesis lineage |
| standing and planned epistemic work | Curation | deterministic expectation, non-realization, confirmation, and terminal result positions |
| reasoning context | Graph, Traversal, Belief, Causation, Regime, and Planner | complete cuts and exact admitted revisions |
| causal construction | Strategy | immutable heterogeneous Plan with complete Task and confirmation operation |
| authority and progression | Agent | Goal, Plan judgment, separate product authorizations, milestones, and satisfaction |
| executable realization | Execution and nonce | Goal-attributed admission, unified network node, deterministic nonce effect, and outcome account |
| durable return | Events, Graph, Curation, and Belief | append, visibility, confirmation, settlement, and return lineage |
| runtime continuity | root lifecycle and native owners | generation, epoch, readiness, waits, wakes, restart, and fencing evidence |
| product inspection | inspection projection | read-only native position correlation and first missing successor |

## Producer And Consumer Edges

The gate evaluates `SPDS-H01` through `SPDS-H23` against their accepted `WMR-H` owner edges. No transport receipt may substitute for the downstream owner's accepted product.

## Criteria

| Criterion | Required claim | Acceptable evidence | Blocking condition |
| --- | --- | --- | --- |
| `WMR-SI-01-DG-C01` | exact `meld_startup` product compilation, assignment, Agent genesis, generation, and open epoch form one lineage | durable owner records plus inspection | missing or mixed lineage blocks |
| `WMR-SI-01-DG-C02` | a fresh epoch completes the full standing mismatch to Agent Goal satisfaction path | live harness trace plus durable records | any skipped native owner position blocks |
| `WMR-SI-01-DG-C03` | Strategy produces one immutable Plan with separate complete Task and Epistemic Operation products | Plan record and Agent authorizations | Task Network awareness or bundled authorization blocks |
| `WMR-SI-01-DG-C04` | Execution receives only the complete Goal-attributed Task and lowers it through the unified Task Network | admission, lowering, node, attempt, and attribution records | epistemic or Startup-specific Execution semantics block |
| `WMR-SI-01-DG-C05` | `nonce.emit.v1` publishes only the deterministic owner-issued Event kind `nonce` | Capability contract, append record, and negative tests | arbitrary Event authority or duplicate identity blocks |
| `WMR-SI-01-DG-C06` | Event append, Graph visibility, confirmation, Belief settlement, Agent acceptance, Goal satisfaction, and Execution closure remain independent | ordered and adverse-order traces | collapsed positions or callback inference block |
| `WMR-SI-01-DG-C07` | retry, replay, and restart preserve one nonce, Goal, Event, and owner decision per equal epoch input | restart and idempotency tests | duplicate semantic products or lost progress block |
| `WMR-SI-01-DG-C08` | a successor admission epoch creates a successor nonce and fences old evidence | epoch replacement trace and stale-input tests | old evidence satisfying the successor blocks |
| `WMR-SI-01-DG-C09` | structural readiness precedes the nonce while nonce satisfaction remains non-gating | lifecycle records and failed-nonce startup trace | cyclic readiness or dependent-product gating blocks |
| `WMR-SI-01-DG-C10` | inspection names the deepest available position, first missing successor, and unresolved parallel obligations without writing owner state | inspection specimens for success, lag, conflict, and uncertainty | synthesized truth or hidden uncertainty blocks |
| `WMR-SI-01-DG-C11` | changed source follows repository structure, commenting, lint, property, fuzz, durability, and test-quality policy | satisfied `WMR-SAO-01` receipt | absent or failed Style Assurance blocks |
| `WMR-SI-01-DG-C12` | implementation stays inside the activated boundary | exact changed surface and architecture review | unauthorized domain expansion blocks |

## Required Scenario Evidence

The fresh epoch is the direct product proof. Gate evidence must also cover Event already durable after restart, crash before append, crash after append before provider return, Graph lag, missing Belief route, confirmation rejection, stale epoch Task, participant replacement, duplicate Task admission, mismatched Goal lineage, satisfied nonce with uncertain Execution callback, and whole-runtime idle after nonce.

Equivalent deterministic fault injection is acceptable. A unit-only simulation cannot replace the required live fresh-epoch harness proof.

## Forbidden Substitutions

Process health cannot substitute for structural readiness. Structural readiness cannot substitute for nonce satisfaction. Event append cannot substitute for Graph visibility. Graph visibility cannot substitute for Belief settlement. Belief settlement cannot substitute for Agent milestone acceptance. Task success cannot substitute for semantic return. Agent satisfaction cannot substitute for Execution uncertainty closure or global quiescence.

## Acceptance Inputs

Gate Acceptance requires the exact source candidate, direct product proof, logical implementation-review receipt, satisfied Style Assurance Receipt, criterion evidence, changed-surface account, authorized exceptions, and affected handoff entries.

The final verdict is `accepted`, `rejected`, or `not eligible`. Acceptance makes `WMR-SI-01` handoff eligible only.
