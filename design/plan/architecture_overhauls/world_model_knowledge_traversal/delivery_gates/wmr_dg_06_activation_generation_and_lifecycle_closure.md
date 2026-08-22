# WMR-DG-06 Activation Generation And Lifecycle Closure

Date: 2026-08-22

Gate identifier: `WMR-DG-06`

Revision: 1

Status: frozen and accepted

Gate owner: Codex separate Gate Acceptance lane with dedicated subagent recommendation

Exception authority: user

## Coherence Horizon

The gate begins at the accepted inert prepared activation closure from `WMR-DG-05` and ends at a current, truthfully live, interrupted and recoverable, replaced, or safely retired generation. It closes `WMR-H22` through `WMR-H25` and aggregates only already accepted native-owner positions.

Runtime schemas, persistence selection, service topology, migration, source implementation, and final cross-product inspection projection are outside the horizon.

## Deliverables

- `detailed_design/wmr_dd_06_worker_packet.md`
- `detailed_design/activation_generation_and_lifecycle_transition_ledger.md`
- `detailed_design/activation_generation_and_lifecycle_closure.md`
- affected entries in `world_model_reconciliation_handoff_ledger.md`
- accepted upstream `WMR-DG-01` through `WMR-DG-05` evidence
- integrated design-review receipt

## Criteria

| Criterion | Cross-deliverable claim | Acceptable evidence | Forbidden substitution | Blocking standard |
| --- | --- | --- | --- | --- |
| `WMR-DG-06-C01` | one lifecycle authority owns generation, assignment-head publication, and admission coherence | authority account and `WMR-H22` | two heads or unrelated current records | ambiguous authority blocks |
| `WMR-DG-06-C02` | prepared, intended, realizing, ready, current, and retired are independent positions | state ledger | adjacent state as proof | state collapse blocks |
| `WMR-DG-06-C03` | generation, participant specification, realization, incarnation, registration, lease, and owner receipt identities remain distinct | identity ledger | process handle as participant identity | identity collapse blocks |
| `WMR-DG-06-C04` | runtime registration is an exact projection of the accepted participant plan | parity account | fixed descriptor catalog | unproved participant closure blocks |
| `WMR-DG-06-C05` | supervisor owns process health while native owners own semantic readiness and progress | ownership account | heartbeat or clean tick as readiness | owner leakage blocks |
| `WMR-DG-06-C06` | every required participant publishes exact owner readiness bound to generation, incarnation, realization, and checkpoint | readiness account and `WMR-H23` | opaque receipt string or body resolution | unverifiable readiness blocks |
| `WMR-DG-06-C07` | current publication and admission expose one authoritative generation transition | publication account | head then separate uncorrelated open flag | split-brain window blocks |
| `WMR-DG-06-C08` | consumer-before-producer order is correct through durable positions and owner waits | order-independence account | actor tick order or same-pass coupling | timing-dependent correctness blocks |
| `WMR-DG-06-C09` | every owner wait has exact checkpoint and complete structurally resolvable wake references | `WMR-H24` account | polling or heartbeat | broken wake closure blocks |
| `WMR-DG-06-C10` | active work, active idle, quiescent, stalled, and interrupted remain truthful distinct projections | state table | all-no-work tick as quiescent | false liveness blocks |
| `WMR-DG-06-C11` | required participant recovery closes admission, creates a successor incarnation, reconstructs owner state, and reopens only after readiness | recovery trace | supervisor lease reacquisition alone | unsafe recovery blocks |
| `WMR-DG-06-C12` | successor generation becomes current conditionally and predecessor drains only prior accepted work | replacement trace | in-place mutation or simultaneous current heads | unsafe replacement blocks |
| `WMR-DG-06-C13` | late results retain old lineage and destination owners classify them | late-delivery trace | root semantic classification or relabel to current | cross-generation corruption blocks |
| `WMR-DG-06-C14` | uncertain external effects remain Execution-owned and block unsafe retirement through exact owner receipts | uncertain-effect trace | root operation semantics | owner collapse blocks |
| `WMR-DG-06-C15` | fenced quiescence requires closed admission, complete drains, passive fences, owner safe points, and unresolved summaries | `WMR-H25` account | ordinary quiescence or stopped handles | unsafe retirement blocks |
| `WMR-DG-06-C16` | retirement stops in reverse structural order, releases realization, and publishes immutable terminal evidence | retirement trace | process shutdown | incomplete retirement blocks |
| `WMR-DG-06-C17` | fresh, lagged, crash, missed-wake, replacement, late-delivery, and retirement traces cite exact identities and positions | lifecycle proof suite | narrative happy path only | incomplete lifecycle proof blocks |
| `WMR-DG-06-C18` | root remains structurally authoritative but semantically blind | negative authority account | lifecycle coordinator interpreting Plans, Tasks, EpiOps, Beliefs, or source meaning | semantic centralization blocks |
| `WMR-DG-06-C19` | candidate remains documentation-only and does not select implementation topology | worker packet and candidate diff | future implementation need | source work or architectural expansion blocks |

Every criterion is blocking. Acceptance requires every criterion to pass or carry a user-authorized exception.

## Acceptance Budget

- one initial acceptance pass
- one frozen violation set
- one program-owner disposition
- one bounded remediation cycle
- one verification pass limited to failed criteria and correction-caused regressions

The acceptance owner may judge only these criteria. It may not redesign lifecycle, choose persistence or topology, amend the gate, waive a criterion, or authorize `WMR-DD-07`.
