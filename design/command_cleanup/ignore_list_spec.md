# Ignore List Specification

## Overview

Scan and workspace delete/restore use a consistent ignore model: by default scan respects .gitignore and a per-workspace ignore list. The ignore list is stored in the XDG data directory for that workspace. `merkle workspace delete` (tombstone) adds the deleted path to the ignore list unless `--no-ignore` is passed, so future scans never re-traverse that path. `merkle workspace restore` removes the path from the ignore list so the next scan can re-index it. See node_deletion_spec.md for tombstone semantics.

---

## Goals

1. **Default: respect .gitignore** — When scanning, ignore paths that match the workspace root’s .gitignore (if present).
2. **Per-workspace ignore list** — A persistent list of paths/patterns for the workspace, stored under XDG and read by scan.
3. **Scan reads and respects both** — Scan uses .gitignore plus the ignore list (and any built-in defaults) to drive the Walker.
4. **workspace delete (tombstone) updates the list** — After tombstoning a path, add it to the ignore list unless `--no-ignore` is passed, so the next scan does not re-add or re-traverse it.
5. **workspace restore removes from the list** — When restoring a tombstoned path, remove it from the ignore list so the next scan can re-index it.
6. **workspace ignore adds without tombstoning** — `merkle workspace ignore <path>` adds a path (file or directory) to the ignore list so future scans skip it; no node tombstoning.

---

## XDG location for ignore list

**Location:** Per-workspace, in the same XDG data directory used for store, frames, and indices:

- **Path:** `$XDG_DATA_HOME/merkle/<workspace_path>/ignore_list`

where `<workspace_path>` is the canonical workspace root path used by `workspace_data_dir(workspace_root)` in `src/config.rs` (e.g. `$XDG_DATA_HOME/merkle/home/user/projects/myproject/`). So the full path is:

- `workspace_data_dir(workspace_root).join("ignore_list")`

**Rationale:** The ignore list is workspace-specific data, not global config. Storing it next to the node store and frames keeps all workspace state under one directory and avoids touching the workspace root. XDG config home is used for global things (agents, providers, prompts); XDG data home is used for workspace-specific data (store, frames, head index, basis index, logs). The ignore list belongs with workspace data.

**Creation:** The file is created when first needed: when `merkle workspace ignore <path>` or `merkle workspace delete` (without `--no-ignore`) adds a path. The parent directory (`workspace_data_dir`) may already exist from a previous scan; if not, create it when writing the ignore list. Scan does not create the file; it only reads it if present. `merkle workspace restore` may remove lines or leave the file empty; it does not delete the file.

---

## .gitignore behavior

- **Default:** Scan treats the workspace root’s `.gitignore` as a source of ignore patterns. If `workspace_root/.gitignore` exists, read it and apply the same semantics as git (e.g. glob patterns, negation with `!`). If the file is missing, no .gitignore-based ignores.
- **Order / merge:** Effective ignores = built-in defaults (e.g. `.git`, `target`, `node_modules`, `.cargo`) + .gitignore patterns (if file exists) + ignore_list file contents (if file exists). Duplicates are harmless; later entries do not override earlier ones for the same path.

---

## Ignore list file format

- **Encoding:** UTF-8.
- **Content:** One path or pattern per line. Empty lines are skipped. Leading/trailing whitespace on a line is trimmed.
- **Path form:** Paths are stored relative to the workspace root when added by `workspace delete` or `workspace ignore` (e.g. `node_modules` or `src/generated`). Patterns may be glob-style if the Walker supports them (e.g. `**/node_modules`); otherwise treat as path prefix or exact path per Walker implementation.
- **Comments:** Optional: lines starting with `#` are ignored when reading. Not required for v1.

---

## Scan behavior

1. **Resolve workspace root** (CLI or config).
2. **Resolve ignore list path:** `workspace_data_dir(workspace_root).join("ignore_list")`. If the file exists, read it and parse into a list of patterns/paths.
3. **Resolve .gitignore:** If `workspace_root.join(".gitignore")` exists, read it and parse into ignore patterns (gitignore semantics or a subset).
4. **Build Walker config:** Merge built-in defaults, .gitignore patterns, and ignore_list patterns into `WalkerConfig::ignore_patterns` (or equivalent). Pass this config to TreeBuilder/Walker.
5. **Build tree and populate store** as in scan_command_spec.md.

Scan does not write the ignore list. It only reads it when present.

---

## workspace delete behavior (tombstone)

1. Perform the tombstone as in node_deletion_spec.md (resolve path, collect branch, tombstone nodes).
2. **Unless `--no-ignore` was passed:** Append the deleted path to the workspace ignore list.
   - **Path to append:** The path as given (positional) or the path corresponding to the tombstoned node, normalized to workspace-relative form (e.g. one line: `node_modules` or `src/foo/bar`). Use a single, consistent form so the next scan skips that path.
   - **File location:** `workspace_data_dir(workspace_root).join("ignore_list")`. Create the parent directory if it does not exist. Create the file if it does not exist; otherwise append a newline and the path.
   - **Deduplication:** Optional: before appending, check if the path or an equivalent is already in the file; if so, skip appending. Not required for v1.
3. Output: same as today; optionally mention "Added <path> to ignore list" when the path was appended.

**Why ignore list matters for tombstones:** Tombstoned nodes remain in storage until compaction, but the ignore list ensures that `merkle scan` will never re-traverse or re-add that path. This is critical for large deleted directories like `node_modules` — even though the tombstoned records exist, the ignore list prevents any scan from walking that filesystem subtree again.

---

## workspace restore behavior

When restoring a tombstoned node via `merkle workspace restore`:

1. Perform the restore as in node_deletion_spec.md (clear tombstone markers on node and descendants).
2. **Remove the path from the ignore list** if present.
   - **Path to remove:** The path corresponding to the restored node, normalized to workspace-relative form.
   - **File location:** Same as delete: `workspace_data_dir(workspace_root).join("ignore_list")`.
   - **Behavior:** Read the file, filter out lines matching the path (exact match or equivalent), write back. If the file becomes empty, it may be deleted or left empty.
3. Output: optionally mention "Removed <path> from ignore list" when the path was removed.

**Rationale:** If a node is restored, the user likely wants it to be re-scanned and re-indexed. Leaving it in the ignore list would cause confusion where the node appears active but gets skipped on the next scan.

---

## workspace ignore behavior

**Command:** `merkle workspace ignore [path]`

With **no path**: pretty-list the contents of the workspace ignore list (read the file and display in a readable format). With **a path**: add that path (file or directory) to the ignore list without tombstoning any nodes.

**Syntax**

```text
merkle workspace ignore [path]
```

**Options**

- `<path>`: Optional positional; workspace-relative or absolute path to a file or directory. If omitted, list the ignore list. If provided, normalized to workspace-relative form before appending.
- `--dry-run`: Optional; when adding a path, report the path that would be added without writing. Ignored when listing.

**Behavior**

**When no path is provided (list mode):**

1. Resolve workspace root (CLI or config).
2. Resolve ignore list path: `workspace_data_dir(workspace_root).join("ignore_list")`. If the file does not exist, output e.g. "Ignore list is empty." or show an empty list.
3. Read the file; parse lines (skip empty, trim; optionally skip comment lines starting with `#`).
4. Pretty-list the contents: one entry per line with clear formatting (e.g. numbered list or simple indented lines), so the user can see what is currently ignored. Optionally support `--format json` for machine-readable output (e.g. `{ "ignored": ["node_modules", "src/generated"] }`).

**When a path is provided (add mode):**

1. Resolve workspace root (CLI or config).
2. Normalize the given path to workspace-relative form (e.g. `node_modules`, `src/generated`). If the path is outside the workspace, error (e.g. "Path is outside workspace").
3. Resolve ignore list path: `workspace_data_dir(workspace_root).join("ignore_list")`. Create the parent directory if it does not exist. Create the file if it does not exist; otherwise append a newline and the path.
4. Optional deduplication: if the path or an equivalent is already in the file, skip appending (and optionally report "Already ignored").
5. Output: e.g. "Added <path> to ignore list." With `--dry-run`: "Would add <path> to ignore list."

**Guards**

- In add mode: path must be within the workspace (or exactly the workspace root if that is desired). Paths outside the workspace are an error.
- In add mode: no node store or tree mutation; this command only updates the ignore list file (no tombstoning).
- In list mode: missing file is not an error; show empty list or "Ignore list is empty."

---

## CLI summary

| Command | Effect on ignore list |
|--------|------------------------|
| **merkle scan** | Reads .gitignore and `ignore_list` if present; does not write. |
| **merkle workspace ignore** | Pretty-lists the contents of `ignore_list`. |
| **merkle workspace ignore \<path\>** | Appends path to `ignore_list` (workspace-relative). Does not tombstone nodes. |
| **merkle workspace delete \<path\>** | Tombstones nodes; appends path to `ignore_list` unless `--no-ignore`. |
| **merkle workspace delete --node \<id\>** | Tombstones nodes; when adding to ignore list, use the node’s path (from store) in workspace-relative form; append unless `--no-ignore`. |
| **merkle workspace restore \<path\>** | Clears tombstones; removes path from `ignore_list` if present. |
| **merkle workspace restore --node \<id\>** | Clears tombstones; when removing from ignore list, use the node's path (from store) in workspace-relative form. |
| **merkle workspace compact** | No effect on ignore list. |
| **merkle workspace list-deleted** | No effect on ignore list. |

---

## Implementation notes

- **Config / XDG:** Use `crate::config::xdg::workspace_data_dir(workspace_root)` to get the directory; then `join("ignore_list")` for the file. No new XDG function is required.
- **Walker:** Walker already has `WalkerConfig::ignore_patterns`. Scan (or TreeBuilder) must build that list from .gitignore + ignore_list file + defaults and pass it to `Walker::with_config`.
- **.gitignore parsing:** Use an existing crate (e.g. `ignore` or `gitignore`) for gitignore semantics, or a minimal subset (e.g. line-by-line globs). Specify the chosen approach in implementation.
- **watch:** Watch mode should use the same ignore sources when it triggers tree updates (e.g. load ignore list and .gitignore at daemon start and when processing events), so behavior stays consistent with scan.

---

## Tests required

- Unit: Parse ignore_list file (empty, one line, multiple lines, comments if supported).
- Unit: Path normalization when appending from workspace delete (workspace-relative form).
- Integration: Scan with no ignore_list file uses .gitignore only; scan with ignore_list file uses both; verify ignored paths are not in the tree.
- Integration: workspace delete (tombstone) without --no-ignore creates or appends to ignore_list; next scan excludes that path. workspace delete with --no-ignore does not add to ignore_list.
- Integration: workspace delete --node \<id\> adds the node’s path to ignore_list unless --no-ignore.
- Integration: workspace restore removes path from ignore_list when present; next scan can re-index that path. workspace restore --node \<id\> uses node's path when removing from ignore list.
- Integration: workspace ignore (no path) pretty-lists ignore_list; empty file or missing file shows empty list.
- Integration: workspace ignore \<path\> creates or appends to ignore_list; path is workspace-relative; next scan excludes that path. Path outside workspace errors. Optional: --dry-run does not write.
