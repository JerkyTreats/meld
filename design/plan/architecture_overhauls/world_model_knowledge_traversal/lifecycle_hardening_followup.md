# Runtime lifecycle hardening follow-up

Status: recorded and deferred by user direction. Graph architecture assessment and optimization remain the current priority. This note preserves the need for a broad lifecycle pass; it is not an implementation program or an approved takeover design.

## Reason to retain this work

The Graph engine spike exposed the same immediate-restart failure in the baseline, indexed Sled and SQLite arms. A crashed runtime leaves `code-semantics.observation` leased to its previous instance for approximately 15 minutes. A replacement instance fails before readiness, preventing native Graph replay qualification. The [retained evidence](../../../../../meld-eval/evidence/graph-engine-fit-v1/CLOSEOUT.md#recovery-did-not-qualify) distinguishes the observed stop and lease rejection from unproven data loss or successful crash replay.

The timing rationale is explicit in [runtime assembly](../../../../src/runtime/assembly.rs): long provider work must not permit another supervisor to take ownership. [Startup ownership checks](../../../../src/runtime/supervisor/entrypoint.rs) recover expired leases before refusing remaining predecessor ownership. [The supervisor store](../../../../src/runtime/supervisor/store.rs) owns acquisition, renewal, release and expiry. This is a lifecycle concern across components, not a Graph database-specific defect.

## Breadth of the future pass

Assess ownership from startup through normal work, interruption, replacement and termination. The lease failure is the entry point, not the entire scope. Investigate how process liveness, permission to execute work, long-running operations, persisted generation identity and component health relate. Include partial startup cleanup, owner-child shutdown, cancellation and draining, stale-owner exclusion, interrupted publication and replay, and the public evidence available to explain recovery. These are investigation concerns; this record does not assert that each is defective or prescribe a new component for each.

A useful result would make ownership and recovery coherent through the existing runtime authority, while keeping long work safe and avoiding duplicate effects. A shorter timeout alone is not the selected solution. Local evidence that an owner has died and the possibility that surviving work can still publish must be reconciled explicitly.

## Effect on current Graph work

The shared lease failure does not currently differentiate the engine candidates or overturn the observed read-amplification results. It is therefore not a prerequisite for architectural examination or bounded Graph optimization. Immediate crash recovery remains unqualified; deferral does not turn that finding into a pass or authorize claims of crash-safe production operation.

Bring lifecycle work forward if evidence shows that it changes the storage recommendation, prevents necessary comparison of a proposed consistency boundary, permits stale effects under normal optimization work, or reveals a material correctness failure. Revisit the preserved crash probes when lifecycle hardening is undertaken. No private lease edits, forced takeover protocol, timeout change or new lifecycle subsystem is authorized by this note.

The [native agdb delivery](agdb_native_delivery.md) exposes a normal-stop race: a completed shutdown receipt and an available supervisor store can precede release of the world-model store. Its bounded correction waits for the verified process to exit on Linux before reporting stop completion. This addresses the observed command handoff; interrupted-owner recovery, descendant ownership, stale-owner exclusion and the broader lifecycle account remain open.
