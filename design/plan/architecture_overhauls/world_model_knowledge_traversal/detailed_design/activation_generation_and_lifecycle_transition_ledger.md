# Activation Generation And Lifecycle Transition Ledger

Date: 2026-08-22

Slice: `WMR-DD-06`

Status: active detailed-design product

Implementation authorization: none

## Identity Chain

| Identity | Owner | Meaning | Stable inputs | Must remain distinct from |
| --- | --- | --- | --- | --- |
| lifecycle acceptance | root lifecycle structure | accepted, duplicate, rejected, or conflicted decision over one inert prepared closure | assignment, prepared closure, request key, expected prior generation | preparation and current generation |
| activation generation | root lifecycle structure | one physically realized candidate or current instance of an assignment | lifecycle acceptance, assignment, prepared closure, generation ordinal, predecessor | assignment and participant incarnation |
| participant specification | activation preparation | one declared structural participant and lifecycle contract references | participant plan, participant id, owner, required status, mode, dependencies, readiness, wake, safe-point, stop refs | runtime registration and semantic owner body |
| runtime realization | runtime composition | one exact physical binding of a participant specification | generation, participant specification, selected implementation, binding revision, resources | readiness and process handle |
| participant incarnation | root lifecycle structure | one attempt to operate a realized participant in one generation | generation, participant, realization, incarnation ordinal, predecessor | supervisor lease and operation attempt |
| runtime registration projection | runtime composition | exact root operational projection of the participant set | generation, participant plan, realization set | independent fixed catalog |
| supervisor instance and lease | supervisor | process ownership and health for one participant incarnation | incarnation, process instance, lease epoch, timing policy | semantic readiness and liveness |
| owner readiness receipt | native owner | proof that one incarnation reconstructed its exact semantic and physical input position | generation, incarnation, realization, owner checkpoint, installed revisions, subscriptions, binding, readiness contract | body resolution and opaque reference |
| current-generation publication | root lifecycle structure | one authoritative assignment-head transition to a fully ready generation with admission open | generation, complete required readiness set, expected prior head, admission epoch | readiness and process health |
| owner checkpoint | native owner | exact durable progress frontier and accepted-obligation position | generation, incarnation, owner-native cursors, revisions, accepted work | supervisor report and queue length |
| owner wait receipt | native owner | truthful no-eligible-work account at one checkpoint | generation, incarnation, checkpoint, owner condition, complete structural wake references | polling and quiescence |
| wake reference resolution | structural wake owner or transport | proof that each declared wake resolves to a present owner, durable position, deadline, subscription, or operator channel | generation, participant, wake reference, owner or transport revision | semantic eligibility decision |
| owner safe-point receipt | native owner | fenced account of accepted work, drained work, unresolved operations, and restart position | generation, incarnation, closed admission epoch, checkpoint, unresolved summary | process stopped flag |
| fenced-quiescence receipt | root lifecycle structure | complete aggregation of required owner safe points and fenced passive paths | generation, closed admission, complete safe-point set, wake and passive-delivery fences | active-idle or ordinary quiescence |
| retirement receipt | root lifecycle structure | terminal proof that a non-current generation released its structural realization safely | generation, fenced quiescence, reverse-order stop receipts, lease releases, head proof | process shutdown |

## Lifecycle State Positions

| State | Required evidence | Explicitly not enough |
| --- | --- | --- |
| prepared | exact inert prepared activation closure with no lifecycle decision | package, assignment, or local owner receipts alone |
| intended | accepted lifecycle request over exact prepared closure | prepared closure alone |
| preparing | generation identity exists with admission closed | process construction |
| realizing | exact participant realizations, registrations, and incarnations are being established | fixed role catalog or handle count |
| ready | every required owner readiness receipt resolves under the exact generation and incarnation set | opaque strings or body resolution |
| current | conditional assignment-head publication and matching admission-open position are durably observable as one authority | ready record without head or open admission without head |
| operationally healthy | supervisor lease, heartbeat, and successful bounded process report for one incarnation | owner readiness, semantic progress, currentness, quiescence, or generation closure |
| active work | one or more owners report eligible or accepted work from exact checkpoints | successful tick alone |
| active idle | owners ran cleanly in the observed operational pass | quiescence |
| quiescent | every realized participant has a checkpoint, no eligible work, a complete wait, and viable wakes under current admission | empty queue or all-no-work tick |
| stalled | required wait, wake, participant, binding, or owner progress closure is missing or broken | operational health |
| interrupted | a required incarnation or lifecycle transition cannot currently preserve the generation contract and admission is fenced | expired lease alone without lifecycle action |
| draining | generation admission is closed and owners are completing or classifying already accepted obligations | stop request |
| fenced quiescent | every realized owner and passive path has a safe-point or unresolved-operation account under closed admission | ordinary quiescence |
| retired | generation is non-current, every realized participant is fenced quiescent, stopped in reverse structural order, and released | supervisor shutdown |

## Wait, Wake, Fence, And Restart

| Edge | Wait | Wake | Fence | Restart source |
| --- | --- | --- | --- | --- |
| `WMR-H22` | lifecycle decision absent or prepared closure stale, incomplete, unauthorized, or conflicted | exact closure, expected-prior head, authority, or binding successor | assignment, prepared closure, request key, expected prior | prepared closure, lifecycle decision journal, assignment-head authority |
| `WMR-H23` | realization, incarnation, registration parity, owner reconstruction, or readiness incomplete | exact binding, incarnation, owner checkpoint, subscription, or readiness receipt | generation, participant plan, realization, incarnation, installed revisions, bindings | generation account, participant plan, realization set, registrations, owner stores, readiness receipts |
| `WMR-H24` | owner has no eligible work or a structural wake does not resolve | exact owner revision, Event sequence, source subscription, deadline, operation completion, binding recovery, or operator signal | generation, incarnation, checkpoint, admission epoch, wake-owner revision | owner checkpoint, wait receipt, wake-resolution receipts, durable producer and consumer positions |
| `WMR-H25` | admission still open, accepted work undrained, unresolved effects unclassified, passive delivery unfenced, safe point absent, or generation still current | exact drain progress, owner reconciliation, passive fence, safe point, head successor, or stop receipt | generation, closed admission epoch, incarnation set, owner checkpoints, unresolved operations, current head | admission record, owner stores, operation accounts, passive subscriptions, safe points, head, stop and lease records |

## Order Independence

Consumer-before-producer execution produces an owner wait at an exact checkpoint. Producer-after-consumer progress advances its durable position and resolves a named wake. Correctness depends on those positions, not actor identifier order, same-pass execution, or polling frequency.

The participant dependency graph may constrain preparation, physical readiness, and reverse drain. Strategy Plans and native owner products alone define semantic causal order.

## Lifecycle Acceptance Outcomes

| Outcome | Durable result | Successor behavior |
| --- | --- | --- |
| accepted | decision and deterministic preparing generation identity | create or resolve that generation under closed admission |
| duplicate | exact prior identical outcome and its evidence | return the prior accepted generation only when the prior outcome was accepted; a prior rejected or conflicted outcome returns no generation |
| rejected | terminal rejection for the request key and exact closure | create no generation; changed input requires a successor request key |
| conflicted | terminal conflict naming mismatched closure, authority, or expected-prior head | create no generation; explicit reconciliation or successor request required |

## Restart And Replacement Matrix

| Interruption point | Reconstructed truth | Required next position |
| --- | --- | --- |
| after lifecycle acceptance before generation record | accepted decision with no candidate generation | idempotently create the same generation identity |
| after some realizations or incarnations | exact partial participant set under closed admission | resume first missing realization or owner readiness |
| after complete readiness before current publication | ready non-current generation | compare expected prior and publish conditionally |
| during current publication | resolve one assignment-head authority and matching admission epoch | complete or reconcile without two current truths |
| required participant lease loss | current generation plus failed incarnation and invalidated open admission epoch | close generation admission, mark interrupted, create successor incarnation, collect recovery readiness, then reopen under a successor admission epoch if the same generation is still current; submissions carrying the closed epoch are rejected or owner-classified outside replacement-incarnation work |
| new prepared closure ready beside old current | ready successor and still-current predecessor | conditional head replacement and new admission, old generation drains accepted work |
| late result for old generation | owner-native result with old generation and attempt lineage | owner classifies or reconciles without admitting it as new-generation work |
| during drain or stop | closed admission plus exact owner safe-point and stop subset | resume missing drain, safe point, reverse stop, or release position |
