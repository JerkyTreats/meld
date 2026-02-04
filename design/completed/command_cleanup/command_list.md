# CLI Command Reference (Cleanup)

Commands to be clarified or reorganized (excluding the clear groups: `init`, `context`, `provider`, `agent`).

## Remove

Low-level, node-ID-based commands. No reasonable user should need these; removing rather than cluttering the `context` group.

| Command | Use |
|--------|-----|
| **get-node** | Get context by Node ID (hex); structured output. |
| **get-text** | Combined text of frames for a node ID. |
| **put-frame** | Attach a frame to a node from a file (no LLM). |
| **list-frames** | List frames/head types for a node. |
| **get-head** | Resolve head frame ID(s) for a node. |

**validate-providers** — Remove as top-level; use `merkle provider validate` per provider_validate_spec.md.

## Keep (reorganize)

**CLI placement:** Top-level: merkle scan. Workspace: merkle workspace status | validate | ignore | delete | restore | compact | list-deleted. Watch: merkle watch.

| Command | Grouping | Use |
|--------|----------|-----|
| **synthesize** | context | Create a directory’s “branch” frame from its children’s frames using a Synthesis agent. |
| **regenerate** | context | Regenerate frames for a node (and optionally descendants) when the basis has changed. |
| **scan** | top-level | Build or refresh the Merkle tree from the workspace filesystem (creates/refreshes workspace); required before context operations. |
| **status** | workspace | Workspace summary: tree (scanned/not scanned, root hash, node count, optional breakdown), context coverage per agent, top 5 paths by node count. |
| **validate** | workspace | Check workspace data integrity (store, head index, basis index consistency). |
| **ignore** | workspace | With no path: list the ignore list. With path: add path so future scans skip it; does not delete nodes. |
| **delete** | workspace | Tombstone a node and its descendants by path or --node id; cascade; optional --dry-run, --no-ignore. Data preserved until compact. |
| **restore** | workspace | Clear tombstone markers for a node and its descendants; reverses delete. |
| **compact** | workspace | Purge tombstoned records older than TTL (default 90 days); optional --all, --keep-frames, --dry-run. |
| **list-deleted** | workspace | List tombstoned nodes with timestamps; optional --older-than filter, --format json. |
| **watch** | top-level | Run the file-watcher daemon so the tree (and optionally context) stays updated on filesystem changes; uses same ignore sources as scan. |
