# Activation Generation And Lifecycle Closure Detailed Design

Date: 2026-08-22

Slice: `WMR-DD-06`

Status: active-slice design candidate

Implementation authorization: none

## Decision

One assignment has one lifecycle authority for generation identity, assignment-head publication, and admission fencing. Root lifecycle consumes exact structural identities and native-owner receipts. It never interprets semantic bodies, decides owner eligibility, owns executable or epistemic outcomes, or turns participant order into causal scheduling.

The design converges the currently disconnected startup publication, portable lifecycle, participant plan, runtime registration, and supervisor lease concepts into one authoritative account without choosing a Rust type, store, transaction mechanism, service topology, or migration.

## Activation Path

```text
inert prepared activation closure
-> durable lifecycle acceptance
-> preparing generation with admission closed
-> participant-plan realization
-> exact registration parity and participant incarnations
-> native-owner reconstruction and readiness receipts
-> complete structural readiness barrier
-> conditional current publication with matching admission-open epoch
-> order-independent owner work or durable wait
```

Prepared does not mean intended. Intended does not mean realized. Body resolution does not mean ready. Ready does not mean current. Current publication is the only boundary after which ordinary reconciliation work may be admitted for that generation.

## One Lifecycle Authority

The assignment head, current-generation record, and admission epoch form one externally consistent authority. A reader cannot observe a new head with missing generation identity or open admission for a non-current generation. Implementation may satisfy this by transaction, journaled state machine, indirection, or another proven mechanism. This design chooses none.

Lifecycle acceptance is idempotent by request key and exact prepared closure. `accepted` creates or resolves one deterministic preparing generation. `duplicate` returns the exact prior decision without new work and returns a generation only when that prior decision was accepted. `rejected` and `conflicted` are terminal for that request key and create no generation. A different closure or expected-prior head under the same request is conflicted. Generation identity is immutable and separate from participant incarnation and process lease.

## Participant Realization And Registration Parity

The accepted participant plan is the structural source of the runtime set. Runtime registration is its exact root operational projection, not an independently inferred fixed catalog. Every registration maps to exactly one declared participant specification. An optional specification that is not realized is absent from the active registration set and cannot satisfy a required position. Once any optional specification is realized, it receives the same readiness, checkpoint, wait, wake, recovery, safe-point, stop, release, and retirement closure as every other realized participant.

The parity receipt binds generation, participant-plan identity, complete realization set, registration projection, selected bindings, and implementation revisions. It proves structural correspondence only.

The supervisor retains process ownership. Its instances, leases, heartbeats, reports, and restart policies must cite the participant incarnation. Operational health never substitutes for semantic owner readiness or progress.

## Native-Owner Readiness

Each native owner reconstructs its exact accepted design position under one generation and incarnation. A readiness receipt names the readiness contract, realization, installed revisions, binding, required subscriptions, durable checkpoint, and owner-specific proof position.

Examples include:

- source owner subscription and durable observation cursor
- Events ledger and append authority position
- Traversal replay and projection checkpoint
- configured Belief view and assessment position
- Agent record, accepted subscriptions, genesis publication, and readable required inputs
- Planner source hydration and projection position
- Curation intake and publication positions
- Execution admission, Task Network, provider binding, and outcome publication positions

Root verifies identity, completeness, owner, generation, incarnation, realization, and required contract reference. It does not judge the semantic content of a readiness receipt.

## Current Publication

Every required specification must be realized before publication. Every realized participant, including any optional participant selected into the active registration set, must have a present, resolvable, non-conflicted readiness receipt bound to the exact current incarnation set before it may participate. Optional participants cannot stand in for required ones, and an optional participant cannot become active before its own readiness. Undeclared extra receipts do not satisfy the barrier.

Publication conditionally compares the expected prior assignment head, records the new current generation, and opens its admission under one matching epoch. A conflict leaves the prepared generation non-current and admission closed. No process-local Capability catalog or executor registry can be smuggled into the durable current identity without the exact Capability preparation and realization lineage.

## Work, Waiting, And Wake Closure

Native owners retain durable checkpoints and eligibility. When an owner has no eligible work, it emits a wait receipt under its current checkpoint and names every structural condition that can change eligibility. Root resolves only whether each wake reference has a present owner or transport and a durable address.

Permitted wake classes include exact Event sequence or watermark, owner revision successor, source subscription delivery, deadline, durable operation completion, binding recovery, and operator signal. Polling and heartbeat may reduce latency but are not wake evidence.

Liveness projection is structural:

- active work when an owner has eligible or accepted work
- active idle for a clean observed operational pass
- quiescent only with complete checkpoints for every realized participant, no eligible work, complete waits, and viable wakes
- stalled when required participants, waits, wake owners, bindings, or progress closure are missing
- interrupted when lifecycle fencing is required to reconstruct a required incarnation or incomplete authoritative transition

Quiescence is not Goal satisfaction, product completion, or retirement.

## Recovery Within A Current Generation

When a realized participant incarnation is lost or its binding becomes untrustworthy, lifecycle invalidates the open admission epoch, closes generation admission, and marks the generation interrupted before replacement work can enter. The supervisor may recover process ownership, but lifecycle creates a successor incarnation and the native owner reconstructs from its durable checkpoint and accepted obligations. Any submission carrying the closed epoch is rejected or classified by its native owner without entering replacement-incarnation work.

The generation may reopen only under a successor admission epoch when the replacement incarnation publishes owner recovery readiness, every realized registration still has parity and lifecycle closure, the assignment head still names the generation, and no fence conflict exists. Existing semantic work retains its owner identity and original generation lineage.

## Generation Replacement

A successor prepared closure may be realized and made ready while the predecessor remains current. The successor cannot admit ordinary work before a conditional head transition. Replacement publishes the ready successor as current with a new admission epoch and closes predecessor admission as one authoritative transition.

The old generation then drains only work accepted under its prior epoch. Late producer deliveries and external results retain old generation, incarnation, operation, and attempt lineage. Their semantic owner classifies, admits, rejects, or reconciles them. Lifecycle only aggregates the resulting receipt and never re-labels them as new-generation work.

## Fenced Quiescence And Retirement

Retirement requires:

1. prove the generation is no longer current and close admission
2. fence passive sources and deliveries
3. drain every already accepted owner obligation
4. collect exact owner safe-point receipts and unresolved-operation summaries
5. require Execution to classify uncertain effects and Curation to close accepted Epistemic Operations under their own semantics
6. prove every realized participant and passive path is fenced quiescent
7. stop participants in reverse structural dependency order
8. release supervisor leases and physical bindings
9. publish the immutable retirement receipt

An unresolved operation may be compatible with a safe point only when its owner receipt says how it remains durably attributable and reconcilable after retirement. Root cannot decide that classification.

## Lifecycle Proof Trace Matrix

| Scenario | Exact identities | Initial durable position | Transition | Resulting durable position | Restart source | Forbidden interpretation |
| --- | --- | --- | --- | --- | --- | --- |
| fresh activation | assignment, prepared closure, generation, participant plan, realization set, incarnation set, readiness set, expected prior head, admission epoch | accepted lifecycle decision and preparing generation under closed admission | realize every declared active participant, prove registration parity, collect owner readiness, conditionally publish head | current generation under matching open admission epoch | lifecycle decision, generation account, plan, realizations, registrations, owner readiness, head | ready or healthy as current before publication |
| lagged consumer | generation, producer position, consumer incarnation, checkpoint, wait, wake reference | producer behind or ahead of consumer checkpoint with complete wait | producer commits exact successor and wake owner resolves it | consumer remains durably eligible at its prior cursor until later consumption advances checkpoint | producer store, consumer cursor, wait, wake resolution | actor order or same-pass delivery as correctness |
| missed or broken wake | generation, participant, checkpoint, wait, wake owner or transport revision | complete owner wait with unresolved wake reference | resolve transport or prove it absent | viable wake receipt or stalled liveness projection | owner wait and wake-owner registry | polling as wake proof or broken wake as quiescent |
| crash between commit and consumption | assignment, generation, producer identity and position, consumer incarnation and cursor, admission epoch | producer commit durable, consumer cursor not advanced | process restarts and reconstructs both positions | same work remains eligible without replaying producer semantic decision | producer store, consumer cursor, wait, generation and admission records | call success or process delivery as handoff |
| uncertain external effect | generation, Execution participant and incarnation, Task, claim, attempt, operation, binding, target, checkpoint | durable uncertain operation under accepted admission epoch | Execution reconciles or retains attributable unresolved operation | owner safe point cites classified outcome or durable unresolved summary | Execution network, claim, attempt, operation and outcome stores | lifecycle assigning Execution semantic status |
| participant interruption | generation, old incarnation, closed admission epoch, new incarnation, checkpoint, readiness | current generation with failed realized incarnation | invalidate epoch, close admission, reconstruct successor incarnation, collect readiness | same current head may reopen only under successor admission epoch | head, admission history, participant and supervisor records, owner checkpoint | stale-epoch submission entering replacement work |
| generation replacement | assignment, predecessor and successor generations, prepared closure, readiness sets, head epochs | predecessor current and successor ready but non-current | conditional head and admission transition | successor current and open, predecessor non-current and draining prior accepted work | both generation accounts, readiness, head and admission history | simultaneous current heads or new work entering predecessor |
| late delivery after replacement | old generation, old incarnation, source or operation identity, destination owner position, successor generation | old addressed delivery arrives after successor current | destination owner classifies under old lineage | accepted, rejected, or reconciled owner receipt still cites old generation | source or operation store, destination cursor and owner receipt | relabeling delivery as successor work |
| retirement | non-current generation, closed epoch, incarnation set, checkpoints, unresolved summaries, passive fences, safe points, stop and lease identities | generation draining under closed admission | complete drains, collect safe points, fence passive paths, reverse stop, release | fenced-quiescence receipt and immutable retirement receipt | admission, owner stores, operations, subscriptions, safe points, head, stop and lease records | clean tick, queue emptiness, or stopped process as retirement |

## Product Boundary

PDS leaves the live path after preparation. World Model Reconciliation owners operate the product. Root lifecycle maintains only structural generation coherence. `WMR-DD-07` may compose these accepted positions for proof and inspection but cannot redefine them.
