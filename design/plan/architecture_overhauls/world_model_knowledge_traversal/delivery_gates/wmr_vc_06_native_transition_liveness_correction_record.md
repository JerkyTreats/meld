# WMR-VC-06 Native Transition And Liveness Correction Record

Date: 2026-09-05

Slice: `WMR-VC-06`

Status: rejected by fresh Logical Review; historical evidence only

Rejected predecessor: `WMR-VC-06::0dc00fd22ddf54951bd18fa016a58cac1cd2eab8c1b0966daef73b023dbab241`

Successor candidate: `WMR-VC-06::871f0be8b14d49adc808250b7c6f28f9849eb408e8d7e5762bcd301cd48e7033`

Exact successor manifest: [native-transition and production-liveness successor](../reviews/wmr_vc_06_native_transition_liveness_successor_candidate.sha256)

Fresh Logical Review rejected this candidate because root still authored typed wake meaning, failure and predecessor action records could project active work, and shared Graph lifecycle state could reuse predecessor-incarnation readiness. The preserved aggregate, transition, retirement, and source-retirement evidence remains valuable but does not establish acceptance.

## Correction Boundary

The correction remains wholly inside VC-06. It adds no Startup proof, final native inspection, product migration, VC-07 behavior, commit, push, or deployment.

## Active Owner Transitions

- the generic lifecycle trait has no proof-generating defaults
- every concrete semantic handle must implement readiness, wait, safe point, stop, release, and wake resolution
- world-model and Execution actors retain their own native lifecycle phase and author transition proofs from their durable checkpoint
- native stop fails before native safe-point completion
- native release fails before native stop completion
- root binds owner transition proofs to exact generation receipts but does not create the owner transition
- the legacy Event diagnostics handle retains an explicit local transition because it is not a realized active participant in the accepted plan

## Native Wait And Resolver Path

- each active owner returns its typed wait receipt through the concrete owner implementation
- Graph, Evidence, and Curation expose Event-position wakes
- Belief, Agent, and Task admission expose owner-revision wakes
- Dispatch and Publication expose durable-operation wakes
- the workspace passive source retains its passive-subscription wake
- the production supervisor asks native handles and passive sources whether each recorded wake has a structural resolver
- root performs no condition-name conversion and does not force wakes into one generic variant

## Production Liveness

`RuntimeSupervisor::status_snapshot` now consumes `ActivationLifecycleStore::project_liveness`. The projection uses durable owner waits, live native resolvers, current working participants, and current idle participants.

The composed product proof observes activation-wide active work, converges through native waits to resolved quiescence, and observes retired state after shutdown through the same production status API. The store proof separately pins active idle, stalled, and quiescent classification. Durable generation status continues to dominate interrupted, draining, and retired projections.

## Verification

- formatting passes
- all-target compilation passes
- strict Clippy passes with the established repository allowance for `clippy::result_large_err`
- the complete serial workspace library, binary, example, and test set passes
- 548 root library tests pass
- world-model and Execution native lifecycle tests pass
- focused lifecycle and composed product tests pass
- benchmark compilation passes
- strict rustdoc remains at the ten established baseline diagnostics
- manifest rehash and deleted-path checks pass
- diff hygiene passes

## Review Boundary

This record establishes implementation and mechanical evidence only. It does not perform or claim the fresh Logical Review, Style Assurance, or Gate Acceptance required for the successor candidate.
