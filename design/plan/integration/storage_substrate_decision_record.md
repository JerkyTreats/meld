# Storage Substrate Decision Record

Date: 2026-07-25
Status: active
Scope: standing record of the sled persistence decision — the affirmation at current scale, the recurring pressure against it, the observable triggers that would flip it, and the seam inventory that bounds an eventual substrate change

## Decision

Sled remains the correct persistence substrate at the current scale-set. It is embedded and zero-ops, its transactions cover every store the runtime owns, and its exclusive file lock enforces the single-writer invariants the domain stores depend on. This record is not a migration mandate. It exists so that on the day the decision flips, the migration surface is enumerated rather than archaeological.

## Pressure

The recurring cost has one root: sled holds an exclusive per-store OS file lock, so live state is readable only by the process that owns the store. Every consumer of live state must be mediated by the owning process, and direct opens work only after a run. Each workaround this forces is recorded in the seam inventory below.

## Trigger conditions

The decision flips when any of these becomes an observable requirement rather than speculation:

- Concurrent writers: more than one runtime process must write the same knowledge graph.
- Single-node ceiling: the knowledge graph must split across storage an order of magnitude beyond the current scale-set.
- Cross-machine agents: agents on separate hosts must share belief state without a mediating owner process.
- Substrate health: sled's own maintenance trajectory degrades to where an embedded dependency becomes a liability on its own clock, independent of scale.

## Migration posture

The served read surface chartered by the [Runtime Harness Plan](runtime_harness_plan.md) is the migration path in embryo. Every consumer moved behind a served contract shrinks the storage-coupled surface toward a small set of frozen traits: the runtime status reader and publisher, the event authority contract, and the domain store facades. A substrate change then happens behind those traits without touching consumers. The harness guarantee that live and playback surfaces are byte-consistent doubles as the storage-parity characterization test that the [Compatibility Policy](../../../governance/compatibility_policy.md) requires before an old path is removed.

## Maintenance rule

Every time the lock forces a workaround, the workaround must route through a seam, and the seam gets a line in the inventory below. A workaround that bypasses a seam — a direct store open during a live run, a copied store root, a lock-retry loop in product code — is rejected in review.

## Seam inventory

| Seam | Location | Form |
| --- | --- | --- |
| Per-store exclusive locks | `src/runtime/storage.rs` | `ProductStorageLayout` opens each store as a separate database; the lock is per store, and every store is exclusive to its owning process |
| Event tail follower | `src/events/tooling/tail.rs` | The follower warns the operator that it holds the single-process database lock while it watches |
| Test reopen-retry loops | `crates/meld-world-model/tests/` | Tests retry opening until the previous handle drops the lock |
| Cutover lock | `src/events/binding.rs` | `CutoverLock` takes an explicit exclusive file lock for the event authority migration path |
| Serve-through-foreground | [Runtime Harness Plan](runtime_harness_plan.md) | The harness reads stores only after a run or through surfaces served by the owning foreground process — the first architectural commitment forced by the lock rather than a local accommodation |

## Related documentation

- [Storage Policy](../../../governance/storage_policy.md) — placement rules for runtime state; orthogonal to substrate choice
- [Compatibility Policy](../../../governance/compatibility_policy.md) — the characterization and parity requirements a substrate migration would inherit
- [Runtime Harness Plan](runtime_harness_plan.md) — the served substrate whose boundary is the migration seam
