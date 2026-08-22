# WMR-DG-04 Execution Admission And Observation Return

Date: 2026-08-22

Gate identifier: `WMR-DG-04`

Revision: 1

Status: frozen and accepted

Gate owner: Codex separate Gate Acceptance lane with dedicated subagent recommendation

Exception authority: user

## Active Slice And Handoff

Active slice: `WMR-DD-04`, Executable Admission And Observation Return.

Intended handoff: establish exact Execution consumption and returned owner evidence so `WMR-DD-05` can rely on stable executable and observation owner contracts during product compilation and Agent genesis design.

## Coherence Horizon

The gate begins at the accepted `WMR-DD-03` Task authorization envelope and ends at exact Agent absorption of a declared returned owner milestone. It includes `WMR-H13` through `WMR-H17` and `WMR-H32` through `WMR-H36`.

PDS compilation, Agent genesis, activation-wide readiness, aggregate quiescence, replacement, retirement, runtime schemas, and source implementation are outside the horizon.

## Deliverables

- `detailed_design/wmr_dd_04_worker_packet.md`
- `detailed_design/execution_admission_and_observation_transition_ledger.md`
- `detailed_design/execution_admission_and_observation_return.md`
- affected entries in `world_model_reconciliation_handoff_ledger.md`
- accepted upstream `WMR-DG-01` through `WMR-DG-03` evidence
- integrated design-review receipt

## Criteria

| Criterion | Cross-deliverable claim | Acceptable evidence | Forbidden substitution | Blocking standard |
| --- | --- | --- | --- | --- |
| `WMR-DG-04-C01` | Execution receives one complete authorized Task with exact Plan, Goal, context, authority, generation, and idempotency lineage | admission account and identity ledger | Goal alone or heterogeneous Plan transport | incomplete consumer envelope blocks |
| `WMR-DG-04-C02` | Execution validates its consumer seam without Method search, Strategy reconstruction, Goal revision, or world-model replanning | admission decision table and negative authority account | current legacy planning behavior | semantic re-planning in Execution blocks |
| `WMR-DG-04-C03` | accepted, duplicate, rejected, and conflicted admission outcomes are durable and replay-safe | admission outcomes and `WMR-H13` | call success or process memory | any non-observable intake path blocks |
| `WMR-DG-04-C04` | several Tasks in one Plan remain independently admitted and authorized | cardinality and lowering account | one aggregate Goal authorization | authorization collapse blocks |
| `WMR-DG-04-C05` | lowering and Task Network mutation preserve exact Task and Plan lineage without becoming semantic authority | identity ledger and `WMR-H14` | compiled node as Strategy product | lineage loss or owner collapse blocks |
| `WMR-DG-04-C06` | dispatch route state is durable or mandatorily reconstructible from exact durable inputs | `WMR-H15` restart account | in-process map or same-pass ordering | process-only handoff blocks |
| `WMR-DG-04-C07` | claims, attempts, external operation identity, authority, and generation fence uncertain effects | `WMR-H16` and uncertain-effect account | worker call completion | unbounded duplicate-effect risk blocks |
| `WMR-DG-04-C08` | succeeded, failed, cancelled, and uncertain outcomes are durable Execution products | outcome table and transition ledger | timeout or missing callback | any silent terminal path blocks |
| `WMR-DG-04-C09` | outcome durability and neutral Event append are independent positions | `WMR-H17` and publication account | append receipt as execution proof | conflated positions block |
| `WMR-DG-04-C10` | semantic-owner observation remains distinct from Task outcome and carries completeness | `WMR-H32`, owner-return account, and product traces | Task success as truth | owner-authority leakage blocks |
| `WMR-DG-04-C11` | returned Event, Graph, Belief, Agent, and Goal positions remain independent | `WMR-H33` through `WMR-H36` and milestone table | any earlier milestone as universal completion | false visibility or satisfaction blocks |
| `WMR-DG-04-C12` | missing README proof reaches exact owner evidence after executable work | direct product trace | Task success alone | missing returned product proof blocks |
| `WMR-DG-04-C13` | partial, failed, and uncertain README work cannot erase possible semantic effects | failure trace and reconciliation rule | failure as proof of no change | unsafe effect inference blocks |
| `WMR-DG-04-C14` | dependency security retains mitigation, inventory, advisory, assessment, and verification meaning | dissimilarity trace | generic success relation | security owner collapse blocks |
| `WMR-DG-04-C15` | waits, wakes, fences, restart, and late results cite exact owner positions | lifecycle table | polling, queue emptiness, or actor order | false lifecycle closure blocks |
| `WMR-DG-04-C16` | candidate remains documentation-only and does not pull PDS or activation lifecycle into scope | worker packet and candidate diff | future implementation need | source work or architectural expansion blocks |

Every criterion is blocking. Acceptance requires every criterion to pass or carry a user-authorized exception.

## Acceptance Budget

- one initial acceptance pass
- one frozen violation set
- one program-owner disposition
- one bounded remediation cycle
- one verification pass limited to failed criteria and correction-caused regressions

The acceptance owner may judge only these criteria. It may not redesign Execution, choose remediation, amend this gate, waive a criterion, or authorize `WMR-DD-05`.
