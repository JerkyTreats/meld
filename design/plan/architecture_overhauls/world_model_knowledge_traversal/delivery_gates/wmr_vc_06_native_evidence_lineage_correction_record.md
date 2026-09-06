# WMR-VC-06 Native Evidence And Activation Lineage Correction Record

Date: 2026-09-05

Slice: `WMR-VC-06`

Status: third bounded correction implemented; fresh Logical Review required

Rejected predecessor: `WMR-VC-06::871f0be8b14d49adc808250b7c6f28f9849eb408e8d7e5762bcd301cd48e7033`

Successor candidate: `WMR-VC-06::e4bf8a9578aa44594411607bd1c5a117aabb9dd37beca322c1f1346d6ddff77f`

Exact successor manifest: [native-evidence and activation-lineage successor](../reviews/wmr_vc_06_native_evidence_lineage_successor_candidate.sha256)

## Correction Boundary

This correction remains wholly inside VC-06. It preserves the durable lifecycle aggregate, atomic publication, recovery mechanics, reverse stop order, typed retirement evidence, and workspace source retirement. It adds no Startup proof, final native inspection, product migration, VC-07 behavior, commit, push, or deployment.

## Expected Outcomes

| Finding | Required outcome | Implemented evidence |
| --- | --- | --- |
| root-authored wake meaning | each owner emits condition-specific typed wake addresses and native domain code owns resolver meaning | world-model and Execution waiting contracts carry typed addresses, every quiet producer supplies them, native resolver functions live in the owning crates, and root only translates and aggregates |
| failure and predecessor action reuse | activation liveness uses only exact current-generation and current-incarnation reports, with failure distinct from work | durable action records carry generation and incarnation identity, production filters exact lineage, failures dominate active work and project stalled, and legacy records remain readable without invented lineage |
| predecessor Graph readiness reuse | each native transition is keyed by exact generation, incarnation, phase, and checkpoint | native lifecycle sessions are identity-indexed, changed-checkpoint retries conflict, a shared Graph runtime authors a distinct successor readiness proof, and root rejects a transition from another lifecycle position |

## Native Wake Path

World-model and Execution owners now publish `EventPosition`, `OwnerRevision`, `DurableOperation`, `DurableDeadline`, `BindingRecovery`, or `OperatorAction` addresses directly from the condition that found work ineligible. Agent conditions have no generic fallback. Unknown conditions fail closed. Multi-cause conditions retain every address.

Resolver namespace decisions now live in the native domain waiting modules. Root performs lossless enum translation, asks each active owner and passive source whether it resolves the address, and passes the aggregate resolver set to the lifecycle store. Root no longer maps condition strings or owns active-owner namespace meaning.

Publication with no composed Task Network now fails as an unresolved native binding. It cannot claim an empty outbox wait that no owner inspected.

## Production Liveness

Every durable action produced under an activation is bound to its exact generation and incarnation before publication. Production status ignores unbound, predecessor-generation, and predecessor-incarnation actions for activation classification.

Only successful or started current-owner actions count as active work. Current no-work and duplicate outcomes count as active idle. Rejected, blocked, retryable failure, fatal failure, and cancelled outcomes count as failed. Any failed realized participant forces the assignment projection to stalled even when another participant reports work.

Released report-store shapes remain byte-exact readable. Legacy waits decode with no fabricated wake references, and legacy actions decode with no fabricated lifecycle lineage, so they cannot establish current activation liveness.

## Native Transition Identity

World-model and Execution lifecycle state is stored per exact native identity rather than as one shared phase. Each start creates or reuses only the requested generation and incarnation session. Idempotent retry requires the same phase and checkpoint. Another incarnation always performs its own constructed-to-running transition and authors a distinct proof.

The composed Graph factory test builds two handles over the same shared Graph runtime and proves that the successor readiness receipt contains the successor incarnation and a different native proof. Stop before safe point and release before stop remain rejected.

## Verification

- formatting passes
- all-target compilation passes
- strict Clippy passes with the established `clippy::result_large_err` allowance
- the complete serial workspace test and doc-test suite passes
- 551 root library tests pass
- 437 root integration tests pass with three environment-gated tests ignored
- world-model and Execution lifecycle, waiting, and integration suites pass
- benchmark compilation passes
- strict rustdoc reaches only the ten established baseline diagnostics
- the exact 36-path manifest rehashes and both retired paths remain absent
- diff hygiene passes

## Review Boundary

This record establishes implementation and mechanical evidence only. It does not perform or claim the fresh Logical Review, Style Assurance, or Gate Acceptance required for the exact successor candidate.
