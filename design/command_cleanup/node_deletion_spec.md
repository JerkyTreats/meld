# Node Deletion Specification (Tombstone-Based)

## Overview

This spec defines **node deletion** using **tombstone semantics**: marking a node and all its descendants as logically deleted without destroying the underlying data. Tombstoned records remain in storage for audit, recovery, and basis chain integrity. **Compaction** is a separate operation that purges tombstoned records after a configurable TTL.

**Rationale:** Users need to drop subtrees from the active index (e.g. after adding `node_modules` to ignore, or to prune stale nodes without a full rescan). Tombstoning provides this capability while preserving:

- **Audit trail**: What context existed before deletion
- **Basis chain integrity**: Frames referencing tombstoned nodes remain valid
- **Recovery capability**: Accidental deletions can be reversed
- **Append-only semantics**: No data is destroyed until explicit compaction

See node_deletion_and_append_only.md for the conceptual foundation.

---

## Goals

1. **Logical delete**: Mark a node and all descendants as tombstoned without destroying data.
2. **Cascade**: One delete operation tombstones the entire subtree; no orphan children in the active index.
3. **Consistency**: Active queries for nodes, heads, and frames exclude tombstoned entries.
4. **Recovery**: Tombstoned nodes can be restored via explicit command.
5. **Compaction**: Explicit command to purge tombstoned records older than TTL.

---

## Scope

### What is tombstoned (marked as logically deleted)

| Component | Action |
|-----------|--------|
| **Node store** | Set `tombstoned_at` timestamp on node record for the node and every descendant. Path-to-node_id mapping remains but queries for active nodes skip tombstoned entries. |
| **Head index** | Set `tombstoned_at` timestamp on every entry for the node and every descendant. Active head lookups skip tombstoned entries. |
| **Frame storage** | No change on tombstone. Frame blobs remain in storage. Compaction may remove them after TTL. |

### What is NOT changed on tombstone

| Component | Action |
|-----------|--------|
| **Frame blobs** | Preserved. Frames are not deleted or modified when their node is tombstoned. |
| **Basis references** | Preserved. Other frames can still reference tombstoned frames as basis. |
| **Node content** | No mutation of node content or NodeID semantics. |

---

## Tombstone Record Structure

### NodeRecord extension

Add to `NodeRecord`:

```text
pub struct NodeRecord {
    // ... existing fields ...
    
    /// Timestamp when this node was tombstoned, or None if active.
    pub tombstoned_at: Option<u64>,  // Unix timestamp in seconds
}
```

### HeadIndex extension

Modify head index entry storage to include tombstone state:

```text
/// Head entry with optional tombstone marker
struct HeadEntry {
    frame_id: FrameID,
    tombstoned_at: Option<u64>,  // Unix timestamp in seconds, or None if active
}
```

### Tombstone metadata

For audit purposes, tombstone operations should record:

- `tombstoned_at`: Unix timestamp when tombstoned
- `tombstoned_by`: Optional identifier (e.g. "user", "scan-prune", "cli")

---

## Cascade Semantics

- **Target**: One node, identified by path (positional) or `--node <node_id_hex>`. Path may be to a file or directory.
- **Scope**: That node plus **all descendants** — the entire branch. For a file, the branch is just that node. For a directory, the branch is the directory and every node in its subtree.
- **Order**: Collect all descendant node IDs via BFS/DFS, then tombstone every node in the set. Order does not matter since we are marking, not removing.
- **Root**: Tombstoning the workspace root node is equivalent to "tombstone entire tree"; allowed.

---

## API Design

### 1. NodeRecordStore

Add to the trait and implementations:

```text
/// Mark a node as tombstoned. Sets tombstoned_at to current timestamp.
/// Does not tombstone descendants; caller is responsible for cascade.
/// Returns the updated record, or error if node not found.
fn tombstone(&self, node_id: &NodeID) -> Result<NodeRecord, StorageError>;

/// Remove tombstone marker from a node (restore).
/// Returns the updated record, or error if node not found or not tombstoned.
fn restore(&self, node_id: &NodeID) -> Result<NodeRecord, StorageError>;

/// Permanently remove a tombstoned node record (compaction).
/// Only succeeds if node is tombstoned and tombstoned_at is older than cutoff.
/// Returns error if node is not tombstoned or is too recent.
fn purge(&self, node_id: &NodeID, cutoff: u64) -> Result<(), StorageError>;

/// List all tombstoned node IDs, optionally filtered by age.
fn list_tombstoned(&self, older_than: Option<u64>) -> Result<Vec<NodeID>, StorageError>;
```

**SledNodeRecordStore implementation:**

- `tombstone(node_id)`: Get record, set `tombstoned_at = now()`, put record back.
- `restore(node_id)`: Get record, clear `tombstoned_at`, put record back.
- `purge(node_id, cutoff)`: Get record, verify tombstoned and old enough, remove key and path key.
- `list_tombstoned(older_than)`: Iterate all records, filter by tombstoned_at.

**Query behavior changes:**

- `get(node_id)`: Returns record regardless of tombstone state. Caller decides whether to filter.
- `find_by_path(path)`: Returns record only if NOT tombstoned (active lookup).
- `list_all()`: Returns all records. Add `list_active()` that excludes tombstoned.

### 2. HeadIndex

Add:

```text
/// Tombstone all head entries for a node (all frame types).
/// Sets tombstoned_at on each entry.
pub fn tombstone_heads_for_node(&mut self, node_id: &NodeID);

/// Restore all head entries for a node (remove tombstone marker).
pub fn restore_heads_for_node(&mut self, node_id: &NodeID);

/// Purge tombstoned head entries older than cutoff.
pub fn purge_tombstoned(&mut self, cutoff: u64);

/// Get head for node, excluding tombstoned entries.
/// Existing get_head should skip tombstoned entries by default.
pub fn get_active_head(&self, node_id: &NodeID, frame_type: &str) -> Option<FrameID>;
```

**Storage format change:**

Current: `HashMap<(NodeID, String), FrameID>`

New: `HashMap<(NodeID, String), HeadEntry>` where `HeadEntry { frame_id, tombstoned_at }`

### 3. FrameStorage

No changes for tombstone operation. Frames remain in storage.

Add for compaction:

```text
/// Remove a frame blob from storage (compaction only).
/// Idempotent: no error if frame_id is not present.
fn purge(&self, frame_id: &FrameID) -> Result<(), StorageError>;
```

### 4. ContextApi (or equivalent)

Add:

```text
/// Tombstone a node and all descendants.
/// Marks records in node store and head index with tombstoned_at timestamp.
/// Frame blobs are not affected.
pub fn tombstone_node(
    &self,
    node_id: NodeID,
) -> Result<TombstoneResult, ApiError>;

/// Restore a tombstoned node and all descendants.
/// Clears tombstoned_at on records in node store and head index.
pub fn restore_node(
    &self,
    node_id: NodeID,
) -> Result<RestoreResult, ApiError>;

/// Compact tombstoned records older than TTL.
/// Purges node records, head index entries, and optionally frame blobs.
pub fn compact(
    &self,
    ttl_seconds: u64,
    purge_frames: bool,
) -> Result<CompactResult, ApiError>;
```

**TombstoneResult:** `{ nodes_tombstoned: u64, head_entries_tombstoned: u64 }`

**RestoreResult:** `{ nodes_restored: u64, head_entries_restored: u64 }`

**CompactResult:** `{ nodes_purged: u64, head_entries_purged: u64, frames_purged: u64 }`

**Tombstone algorithm:**

1. Get node record; if missing, return error (NodeNotFound).
2. If already tombstoned, return success with zero counts (idempotent).
3. Collect all descendant node IDs via BFS/DFS from `record.children`.
4. Build set `to_tombstone = { node_id } ∪ descendants`.
5. For each node_id in to_tombstone:
   - Node store: `tombstone(node_id)`
   - Head index: `tombstone_heads_for_node(node_id)`
6. Persist head index.
7. Return counts.

**Restore algorithm:**

1. Get node record; if missing, return error.
2. If not tombstoned, return success with zero counts (idempotent).
3. Collect all descendant node IDs (same as tombstone).
4. For each node_id in set:
   - Node store: `restore(node_id)`
   - Head index: `restore_heads_for_node(node_id)`
5. Persist head index.
6. Return counts.

**Compact algorithm:**

1. Calculate cutoff timestamp: `now() - ttl_seconds`.
2. Get list of tombstoned nodes older than cutoff.
3. For each node_id:
   - Collect head frame IDs if purge_frames is true.
   - Head index: purge tombstoned entries for this node.
   - If purge_frames: purge frame blobs for collected frame IDs.
   - Node store: purge(node_id, cutoff).
4. Persist head index.
5. Return counts.

**Concurrency:** Document that tombstone/restore/compact are not safe to run concurrently with scan or other structural changes. Use single-writer lock if needed.

---

## CLI

### Delete Command

Placement under **workspace** (tree lifecycle):

```text
merkle workspace delete <path>
merkle workspace delete --node <node_id_hex>
```

**Primary form:** `merkle workspace delete <path>` — path is a **positional argument**: workspace-relative or absolute path to a **file or directory**. The command resolves the path to the corresponding node, and tombstones that node and all descendants.

**Alternate form:** `merkle workspace delete --node <node_id_hex>` — when the node is identified by NodeID (hex) instead of path. Same cascade.

**Options:**

| Option | Description |
|--------|-------------|
| `<path>` | Positional: workspace-relative or absolute path to a file or directory. Mutually exclusive with `--node`. |
| `--node <id>` | Node ID (hex). Mutually exclusive with path. |
| `--dry-run` | Report how many nodes and head entries would be tombstoned, without performing the operation. |
| `--no-ignore` | Do not add the deleted path to the workspace ignore list. By default, the path is appended to the ignore list so the next scan skips it. See ignore_list_spec.md. |

Cascade is always on: the target node and all descendants are tombstoned.

**Behavior:**

1. Resolve path to node_id via node_store.find_by_path (or parse `--node` hex).
2. If node not found, error: "Node not found" / "Path not in tree."
3. If node already tombstoned, output "Already deleted" and exit success (idempotent).
4. If --dry-run: compute subtree size, output "Would delete N nodes, M head entries," exit.
5. Call api.tombstone_node(node_id).
6. **Unless `--no-ignore`:** Append the deleted path to the workspace ignore list.
7. Output: "Deleted N nodes, M head entries." Optionally: "Added <path> to ignore list."

### Restore Command

```text
merkle workspace restore <path>
merkle workspace restore --node <node_id_hex>
```

**Behavior:**

1. Resolve path to node_id. For restore, use a lookup that includes tombstoned nodes.
2. If node not found, error.
3. If node not tombstoned, output "Not deleted" and exit success.
4. If --dry-run: compute subtree size, output "Would restore N nodes, M head entries," exit.
5. Call api.restore_node(node_id).
6. **Remove path from ignore list** if present (inverse of delete behavior).
7. Output: "Restored N nodes, M head entries."

**Options:**

| Option | Description |
|--------|-------------|
| `<path>` | Positional: workspace-relative or absolute path. Mutually exclusive with `--node`. |
| `--node <id>` | Node ID (hex). Mutually exclusive with path. |
| `--dry-run` | Report counts without performing restore. |

### Compact Command

```text
merkle workspace compact
merkle workspace compact --ttl <days>
merkle workspace compact --all
```

**Behavior:**

1. Calculate cutoff based on --ttl (default: 90 days) or --all (cutoff = now, purge everything).
2. If --dry-run: count tombstoned records older than cutoff, output counts, exit.
3. Call api.compact(ttl_seconds, purge_frames=true).
4. Output: "Compacted N nodes, M head entries, F frames."

**Options:**

| Option | Description |
|--------|-------------|
| `--ttl <days>` | Tombstone age threshold in days. Default: 90. |
| `--all` | Purge all tombstoned records regardless of age. Mutually exclusive with `--ttl`. |
| `--keep-frames` | Do not purge frame blobs; only purge node and head index records. |
| `--dry-run` | Report counts without performing compaction. |

### List Tombstoned Command

```text
merkle workspace list-deleted
merkle workspace list-deleted --older-than <days>
```

**Behavior:**

1. Query tombstoned nodes, optionally filtered by age.
2. Output table: Path, Node ID (truncated), Tombstoned At, Age.
3. Support --format json.

---

## Edge Cases

| Case | Behavior |
|------|----------|
| **Node not in store** | Return error (NodeNotFound or PathNotInTree). |
| **Node already tombstoned** | Idempotent success; return zero counts. |
| **Restore non-tombstoned node** | Idempotent success; return zero counts. |
| **Delete root** | Allowed; tombstones entire tree. |
| **Delete file** | Tombstones only that node (no children). |
| **Delete directory** | Tombstones directory and entire subtree. |
| **Compact with no tombstones** | Success; return zero counts. |
| **Compact tombstone newer than TTL** | Skip that tombstone; not an error. |
| **Path vs node_id** | Same outcome; path is resolved to node_id once. |
| **Concurrent scan** | Document: avoid concurrent structural operations. |
| **Restore after partial cascade** | Restore uses same cascade logic; restores entire subtree. |

---

## Tests required

**Unit tests**

- NodeRecordStore tombstone: record marked with timestamp; get still returns it; find_by_path skips it.
- NodeRecordStore restore: tombstone cleared; find_by_path finds it again.
- NodeRecordStore purge: record removed; path key removed; fails if not tombstoned or too recent.
- HeadIndex tombstone_heads_for_node: entries marked; get_active_head skips them.
- HeadIndex restore_heads_for_node: entries unmarked; get_active_head finds them.
- HeadIndex purge_tombstoned: old entries removed; recent entries preserved.
- Path-to-workspace-relative normalization for ignore list append/remove.

**Integration tests (CLI)**

- Delete by path: file (single node tombstoned); directory (cascade: node and all descendants tombstoned).
- Delete by `--node <id>`: same outcomes as path.
- Delete already-deleted node: idempotent success.
- --dry-run: no store or head index changes; output "Would delete N nodes..."
- --no-ignore: deleted path not appended to ignore list.
- Default: path appended to ignore_list.
- Restore by path: tombstoned node and descendants restored.
- Restore non-deleted node: idempotent success.
- Restore removes path from ignore list.
- Compact with --ttl: only old tombstones purged.
- Compact with --all: all tombstones purged.
- Compact --keep-frames: frame blobs preserved.
- Compact default: frame blobs for purged nodes removed.
- list-deleted: shows tombstoned nodes with timestamps.

**Consistency / invariants**

- After delete: tombstoned nodes excluded from find_by_path and active listings.
- After delete: head index active queries skip tombstoned entries.
- After delete: frame blobs still exist; can be retrieved by FrameID.
- After restore: nodes and heads are active again; included in queries.
- After compact: purged nodes truly removed; purged frame blobs removed (unless --keep-frames).
- Basis chain: frames referencing tombstoned nodes remain valid until compaction.

**Edge cases**

- Delete root; delete only node in tree; restore root; compact empty tombstone list.
- Concurrent operations (document behavior or test single-writer lock).

---

## Implementation Checklist

- [ ] **NodeRecord**: Add `tombstoned_at: Option<u64>` field.
- [ ] **NodeRecordStore**: Add `tombstone`, `restore`, `purge`, `list_tombstoned` methods.
- [ ] **NodeRecordStore**: Update `find_by_path` to skip tombstoned nodes. Add `list_active`.
- [ ] **HeadIndex**: Change storage to include tombstone state per entry.
- [ ] **HeadIndex**: Add `tombstone_heads_for_node`, `restore_heads_for_node`, `purge_tombstoned`, `get_active_head`.
- [ ] **HeadIndex**: Update persistence format for new entry structure.
- [ ] **FrameStorage**: Add `purge` method for compaction.
- [ ] **ContextApi**: Add `tombstone_node`, `restore_node`, `compact` methods.
- [ ] **CLI**: Add `merkle workspace delete` with tombstone semantics.
- [ ] **CLI**: Add `merkle workspace restore` command.
- [ ] **CLI**: Add `merkle workspace compact` command.
- [ ] **CLI**: Add `merkle workspace list-deleted` command.
- [ ] **Ignore list**: Delete appends path; restore removes path.
- [ ] **Tests**: See Tests required section above.
- [ ] **Docs**: Update command_list.md with new commands.

---

## Migration

Existing node records do not have `tombstoned_at` field. On first load after upgrade:

- Treat missing `tombstoned_at` as `None` (node is active).
- No migration needed; new field is optional and defaults to active state.

Existing head index entries do not have tombstone state:

- On load, deserialize old format as active (no tombstone).
- On save, use new format with tombstone field.
- Include version field in persistence format for future compatibility.

---

## Summary

| Item | Design |
|------|--------|
| **Scope** | Node store + head index tombstoned for node and all descendants. Frame blobs preserved until compaction. |
| **Cascade** | Always on: tombstone target node and all descendants. |
| **Frames** | Preserved on delete. Purged on compact (default) or preserved with --keep-frames. |
| **Recovery** | `merkle workspace restore` clears tombstone markers. |
| **Compaction** | `merkle workspace compact` purges old tombstones. Default TTL: 90 days. |
| **API** | NodeRecordStore.tombstone/restore/purge; HeadIndex.tombstone_heads_for_node/restore/purge; FrameStorage.purge; ContextApi.tombstone_node/restore_node/compact. |
| **CLI** | `merkle workspace delete <path>` (tombstones); `merkle workspace restore <path>`; `merkle workspace compact`; `merkle workspace list-deleted`. |
