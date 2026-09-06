# WMR-VC-06 Activation Generation And Lifecycle Source Assessment

Date: 2026-09-05

Slice: `WMR-VC-06`

Status: third corrected successor implemented; fresh Logical Review required

Source baseline: `85f4b85d`

Initial rejected candidate: `WMR-VC-06::ad41bce802f5b99226d25de41eb9da4d39b005a34f7c9bca599abbf666470f38`

Rejected native-owner successor: `WMR-VC-06::0dc00fd22ddf54951bd18fa016a58cac1cd2eab8c1b0966daef73b023dbab241`

Rejected transition-liveness successor: `WMR-VC-06::871f0be8b14d49adc808250b7c6f28f9849eb408e8d7e5762bcd301cd48e7033`

Current candidate identity: `WMR-VC-06::e4bf8a9578aa44594411607bd1c5a117aabb9dd37beca322c1f1346d6ddff77f`

## Product Boundary

The source cut begins with the exact inert prepared activation closure accepted by `WMR-VC-05`. It ends with one current generation under coherent admission, or one interrupted, draining, fenced-quiescent, or retired generation with durable evidence.

```text
prepared closure
-> accepted lifecycle decision
-> exact plan projection and realization
-> participant incarnations and native readiness
-> atomic current head and admission epoch
-> bounded work or owner wait with structural wakes
-> recovery or replacement fence
-> owner safe points and passive fences
-> immutable retirement
```

Startup nonce proof, native inspection, product migration, commit, push, and deployment remain outside this slice.

## Current Entrypoint And Successor

The production entrypoint is `RuntimeSupervisor::start` through `ProductRuntimeAssembly::supervisor_startup_package`. It receives the exact prepared closure, its exact registration projection, and the sole lifecycle store. The supervisor accepts or recovers a generation before handle creation, supplies each exact lifecycle context to its native owner, records only the returned native readiness receipt, then publishes one current head and open admission epoch only after the complete barrier passes.

The descriptor catalog remains implementation inventory. It cannot add participants. A required planned participant without a body fails closed, and an optional absent participant is not realized.

## Canonical Authority

`ActivationLifecycleStore` owns one assignment aggregate in the existing theory database. The aggregate contains the sole current generation head, all generation records, admission epochs, realizations, incarnations, readiness, waits, safe points, fenced quiescence, retirement, and an optimistic concurrency revision.

Lifecycle request decisions and transition snapshots are durable sibling indexes of that aggregate. Exact request retries reuse the original decision. Changed inputs under the same request key conflict. Invalid requests become terminal rejected decisions.

Current publication and admission opening occur in one optimistic transaction. Replacement closes predecessor admission and moves it to draining in the same transaction that installs the successor head and epoch.

## Ownership Boundary

Root owns structural closure and process supervision. World-model and Execution actors retain native lifecycle phase and author their own readiness, safe-point, stop, and release transition proofs from durable checkpoints. Concrete owners author condition-specific typed waits, and native domain modules own resolver meaning. Root binds, translates, and aggregates those outputs against the exact generation and incarnation without interpreting Plan, Task, epistemic, Belief, source, or external-effect meaning.

Supervisor health remains operational. A clean bounded tick is only active idle. Activation-wide quiescence requires a current wait for every realized participant and a resolver for every structural wake. Durable action reports are bound to exact generation and incarnation identity. Only matching current-incarnation success counts as active work, while failures and missing current evidence project stalled. The production supervisor obtains resolvers from native active handles and passive sources, then consumes the durable assignment-wide projection in its status API. Interrupted, draining, and retired remain distinct durable positions.

## Recovery And Retirement

Process recovery interrupts the current generation, closes admission, creates a successor incarnation, clears stale owner evidence for that participant, and reopens only after exact readiness is restored. Process restart reuses the non-terminal generation for the same prepared closure. Replacement creates a new generation and fences its predecessor atomically at publication.

Graceful shutdown closes admission, installs the passive-source fence, records active-owner safe points while owners remain live, and commits aggregate fenced quiescence. Native stop hooks then complete in durable reverse participant dependency order before exact leases or passive bindings are released. One immutable terminal receipt retains every typed stop and release receipt. Exact retirement retry is idempotent. Empty, altered, incomplete, out-of-order, or conflicting terminal evidence is rejected.

Late work remains attributed to its original activation lineage. Root-owned portable operation and passive-delivery semantics are deleted. Destination and Execution owners retain semantic classification authority.

## Persistence Decision

One new lifecycle tree family is added inside the existing theory database. No database, crate, dependency, service, actor, or second supervisor is added. Immutable prepared products and existing native-owner stores require no migration or compatibility writer. Released supervisor action and cache shapes retain byte-exact compatibility readers and never acquire fabricated wake or lineage evidence.

## Affected Domain Set

- runtime assembly, registration, lifecycle, supervision, ports, and operator tick presentation
- theory as the unchanged prepared closure and participant-plan publisher
- native owner actors extended with durable checkpoint reads and explicit native lifecycle transitions
- Dependency Security only to retire its exclusive root portable-operation fixture
- harness and runtime CLI proof surfaces

Events, Graph, Belief, Curation, Agent, and Execution stores are reused without a new semantic writer. Startup and product migration remain later work.

## Retirement Inventory

- delete the disconnected portable lifecycle actor and intent queue
- delete root-owned durable external-operation and passive-delivery semantics
- remove the inferred stewardship registration catalog
- remove the lifecycle actor registration and actor tick
- stop selecting unrelated latest Agent activation records
- replace tick-level `quiescent` presentation with truthful `active_idle`

## Scope Account

The current candidate changes thirty-three production Rust paths, including two retired paths, and three executable integration-test paths. No numeric budget was frozen before direct implementation authorization, so this is an evidence account rather than a retroactive limit.

## Direct Proof

- exact request retry and conflict behavior
- terminal rejected decisions
- atomic current head and open epoch
- replacement head switch and predecessor fence
- interrupted recovery through successor incarnation readiness
- complete waits, broken wakes, quiescence, and stalled projection
- native Event-position, owner-revision, durable-operation, durable-deadline, binding-recovery, operator-action, and passive-subscription wakes
- native domain resolver ownership with root limited to lossless translation and aggregation
- exact action generation and incarnation lineage with failure and predecessor isolation
- production supervisor projection of active work, resolved quiescence, and retired state
- safe-point, passive-fence, and immutable retirement evidence
- native transition rejection for stop before safe point and release before stop
- exact nine-participant prepared product activation
- fail-closed native readiness and fresh shared-Graph successor readiness
- exact prepared Agent authority despite newer loose owner heads
- full all-target regression suite

## Advancement Boundary

The current candidate has implementation and mechanical evidence only. Fresh Logical Review is required. Style Assurance and Gate Acceptance remain ineligible until that review passes over the exact candidate. This work does not close the slice, authorize commit or push, activate `WMR-VC-07`, or claim Startup proof.
