# Context generate by path specification

## 1. Overview

This spec describes updates to `merkle context generate` so that generation can be driven by path (file or directory), with optional recursive generation over a subtree and checks for missing or out-of-date child context when the path is a directory. It assumes the LLM payload is built per `llm_payload_spec.md` (file content or that agent's child context in the payload).

## 2. Path and target

- **Path input:** The user may pass a path via `--path <path>` or as a positional argument, e.g. `merkle context generate ./foo`. Path is canonicalized relative to the workspace root and resolved to a node via `NodeRecordStore.find_by_path`. If not in the tree, return `PathNotInTree` and suggest `merkle scan` or `merkle watch`.
- **Target node:** The resolved node may be a file or a directory. Behavior differs by node type and by flags below.

## 3. Single-node generation (no --recursive)

**File path:**  
Generate one frame for that file. No descendant check. Existing flow: resolve path to node_id, resolve agent and provider, frame type default `context-{agent_id}`, head check unless `--force`, then enqueue (sync or async). The queue builds the payload using current file content per `llm_payload_spec.md`.

**Directory path:**  
Generate one frame for that directory node only.

- **Descendant check:** Before generating, collect all descendant node IDs in the subtree (see Subtree collection below). For each descendant and the chosen `frame_type`, check whether a head exists: `get_head(descendant_node_id, frame_type)`. If any descendant has no head, treat as missing (out of date: file or tree changed, context not regenerated).
- **When check fails:** If any descendant is missing a head and `--force` is not set, do not generate. Return an error that lists the paths (or node IDs) that are missing context and tell the user to run `merkle scan` if needed, regenerate those nodes, or use `--force` to generate anyway.
- **When check passes or --force:** Proceed to generate for the directory node only. The queue builds the payload using that agent's child context (child head frames for the same frame_type) per `llm_payload_spec.md`.

**--force semantics (single-node):**  
(1) Generate even if the target node already has a head for this frame type. (2) For a directory, proceed even when descendants have missing context.

## 4. Recursive generation (--recursive)

- **Flag:** Add `--recursive` to `context generate`. When set, the target is the subtree rooted at the given path, not just the single node.
- **Ordering:** Process nodes by **folder level (depth), lowest (deepest) to highest (root)**. Example: level 1 = `dir/foo/wow.txt`, `dir/foo/baz.txt`; level 2 = `dir/foo/`, `dir/bot.txt`; level 3 = `dir/`. So directory nodes are generated only after all their descendants have frames, matching the payload rule that directory context uses that agent's child context.
- **Enqueued only:** All LLM calls go through the generation queue. Recursive does not call the provider directly; it enqueues one or more batches.
- **Sync:** Enqueue the deepest level; wait until every request in that level completes; then enqueue the next level; repeat until the root level. Result: after each level, all nodes at that depth have frames before any parent is enqueued.
- **Async:** Enqueue level 1, then level 2, … then root, in order. No wait between levels. The queue processes requests; level order ensures children are available before parents when the queue runs.
- **Descendant check:** If `--force` is not set, before enqueueing any level run the same missing-head check over the entire subtree. If any node in the subtree is missing a head for the chosen frame type, error and list them; do not enqueue. If `--force` is set, skip this check and enqueue all levels.
- **Scope:** If the path is a file, `--recursive` still means only that single node. If the path is a directory, recursive means all nodes in the subtree (files and directories), in level order.

## 5. Subtree collection and level grouping

- **Subtree:** For a given node_id, the subtree is that node plus all descendants. Use `NodeRecord.children` recursively (or iteratively) via `node_store.get(node_id)`; guard against cycles (e.g. visited set).
- **By level:** Group subtree node IDs by depth. Depth 0 = root of the subtree, depth 1 = its children, etc. For recursive generation, process levels from **max depth to 0** (deepest first). So either return a structure keyed by depth (e.g. `Vec<Vec<NodeID>>` with index 0 = deepest) or compute a post-order list and group by depth before enqueueing.
- **Shared helper:** Subtree collection (and optionally level grouping) should live in a shared place (e.g. tree or store layer) so both the directory missing-context check and the recursive generator use it.

## 6. CLI surface

- **Existing:** `--path`, positional path, `--node`, `--agent`, `--provider`, `--frame-type`, `--force`, `--sync`, `--async`.
- **Add:** `--recursive` (bool). When true, generate for the whole subtree by level as above.
- **Conflict / precedence:** Same as today: path (or positional) vs `--node` mutually exclusive; `--sync` vs `--async` mutually exclusive. `--force` applies to both “overwrite existing head” and “proceed despite missing descendant context.”

## 7. Error messages

- **Path not in tree:** Existing. Suggest `merkle scan` or `merkle watch`.
- **Missing descendant context (directory, no --recursive):** List paths or node IDs that have no head for the chosen frame type. State that the directory needs child context first; suggest regenerating those paths or using `--force` to generate anyway.
- **Missing context in subtree (--recursive, no --force):** Same idea over the full subtree; list missing paths or summarize count and example paths, and suggest `--force` or regenerating.

## 8. Summary

| Path type   | Without --recursive                          | With --recursive                                        |
|-------------|----------------------------------------------|---------------------------------------------------------|
| File        | Generate for that file only; no check        | Generate for that file only (single node)               |
| Directory   | Check descendants; if ok or --force, generate for dir only | Check subtree; if ok or --force, generate by level (deepest first) |

- **--force:** Overwrite existing head; for directories, proceed even when descendants (or subtree) have missing context.
- **Recursive:** One batch per level, deepest to root; enqueue only; sync = wait per level, async = enqueue all levels in order.
- **Payload:** Per `llm_payload_spec.md`: file nodes get file content; directory nodes get that agent's child context. Context is retrieved per agent; directory generation uses only that agent's child heads.

## 9. Related docs

- **llm_payload_spec.md** — What is sent to the LLM (current file content or that agent's child context, prompt, optional response template).
- Original command spec: `completed/design/context/context_generate_command.md`.
