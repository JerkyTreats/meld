# Node Deletion and Append-Only

## Current state

- **Nodes are not deletable today.** The `NodeRecordStore` trait has only `get`, `put`, and `find_by_path` — no `delete` or `tombstone`. Sled and the path-to-node_id mapping in `put` only add or overwrite; nothing removes or marks node records as deleted.
- **Scan does not prune.** On `merkle scan` or `--force`, we build a new tree from the filesystem and call `populate_store_from_tree`, which **puts** every node in the new tree. We never remove or tombstone nodes that are no longer in the tree. If the user deletes a directory on disk and rescans, the old node records and their path mappings remain in the store — stale entries accumulate.

## What "append-only" actually applies to

From the Phase 1 spec and README:

- **Frames**: Immutable once created; no deletion or modification of existing frames. History is preserved. "Frame Deletion: No deletion of frames (append-only)."
- **Nodes**: "Nodes are immutable (new state = new NodeID)" — meaning a given **NodeID** is an immutable identity for a content/structure snapshot; you don't mutate a node in place, you get a new NodeID when the filesystem state changes.

## Tombstones preserve append-only semantics

**Hard deletion** removes data permanently, violating the spirit of append-only. **Tombstoning** preserves data while marking it as logically deleted. This approach:

- **Preserves audit trail**: Tombstoned records remain queryable for history and debugging
- **Maintains basis chain integrity**: Frames can still reference tombstoned nodes/frames as basis
- **Enables recovery**: Accidental deletions can be reversed by removing the tombstone marker
- **Supports eventual compaction**: Old tombstones can be purged after a grace period, with user control

The **node store** is an **index** over "what is in the current tree." Tombstoning a node record marks it as "no longer in the active tree" without destroying the record. This is **updating the index to reflect current tree state** while preserving history.

So:

- **Append-only** = frames and node records are never destroyed; they may be tombstoned (marked as logically deleted)
- **Tombstone** = a marker indicating the record is no longer part of the active tree; the underlying data remains until compaction
- **Compaction** = optional, explicit removal of tombstoned records after a configurable grace period

## Tombstone design

**Conclusion: Tombstoning nodes and frames preserves append-only semantics while enabling logical deletion. Compaction is a separate, explicit operation.**

- **Nodes**: Tombstone the node record in the node store. The record remains with a `tombstoned_at` timestamp. Queries for "active" nodes skip tombstoned entries. Recovery clears the tombstone marker.
- **Head index**: Tombstone head entries for the tombstoned node. The mapping remains but is excluded from active queries.
- **Frames**: Frame blobs are preserved by default. Head index tombstoning logically removes them from "current" context without destroying the blob. Compaction can optionally purge frame blobs for tombstoned nodes after TTL.

### Cascade semantics

- **Tombstone node** = mark that node's record with `tombstoned_at`, and **recursively tombstone all descendants** so the whole subtree is logically removed from the active index.
- **Head index**: Tombstone head index entries for every tombstoned node ID.
- **Frame storage**: Frames remain in storage. They are not deleted on tombstone — only on explicit compaction after TTL.

### When tombstoning happens

Two ways to get "nodes tombstoned":

1. **Explicit delete** via `merkle workspace delete <path>` or `--node <id>`. Tombstones that node and all descendants. Use case: user wants to drop a subtree from the active index without rescanning (e.g. "stop tracking node_modules").
2. **Rescan with prune** (future): On `merkle scan --force`, optionally tombstone nodes that exist in the store but not in the new tree. Requires a "diff and tombstone" API.

Either way, cascade = "this node + all descendants" so the active index never has a child without a parent.

### When compaction happens

Compaction is a separate, explicit operation:

- `merkle workspace compact` removes tombstoned records older than TTL
- `merkle workspace compact --all` removes all tombstoned records regardless of age
- Compaction is optional; tombstoned data can remain indefinitely if desired

## Summary

| Layer           | Append-only | Tombstone | Compaction |
|-----------------|-------------|-----------|------------|
| **Frames**      | Yes         | Via head index | Optional after TTL |
| **Node store**  | Yes         | Mark with timestamp | Optional after TTL |
| **Head index**  | Yes         | Mark with timestamp | Optional after TTL |

Tombstoning nodes (with cascade to descendants) preserves append-only semantics. Frame blobs are preserved until explicit compaction. Use `merkle workspace compact` to purge old tombstones when storage bounds require it.
