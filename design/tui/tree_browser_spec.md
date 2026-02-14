# Tree browser specification

## 1. Purpose

The tree browser is a reusable, generic tree navigation component for the TUI. It is not tied to filesystem semantics. The same component renders:

- Filesystem directory trees (workspace selection, initialization flow)
- Merkle node trees (content browser)
- Any future hierarchical data

It pairs a tree panel with a path input and keeps both synchronized. The user can navigate the tree visually or type a path directly.

## 2. Crate evaluation

Three options were researched. One does not exist.

### tui-tree-widget (recommended)

- **Version:** 0.24.0
- **Downloads/maturity:** 114 GitHub stars, actively maintained, on version 0.24
- **Core API:** `TreeItem<'text, Identifier>` (generic typed identifiers), `Tree` (renderable widget), `TreeState` (selection, expand/collapse state)
- **Identifier model:** Generic. Each item has an identifier unique among siblings. A path through the tree is a `Vec<Identifier>`. Example: `vec!["src", "main.rs"]` uniquely identifies a nested item.
- **What it gives us:** Pure tree rendering with expand/collapse, scrollbar, selection. No opinions on data source.
- **What it does not give us:** No filesystem awareness, no lazy loading, no path input, no focus management. Those are ours to build.
- **Why recommended:** The generic identifier model maps directly to "path traversal not tied to a filesystem." Both filesystem paths and merkle node paths are slash-separated hierarchies. `TreeItem<String>` works for both. The crate is mature and stable.

### ratatui-toolkit (monitor, do not adopt yet)

- **Version:** 0.1.16
- **Downloads/maturity:** 138 total downloads, first published 11 days ago. Extremely new.
- **Components:** TreeView (TreeNode-based, string identifiers), FileSystemTree (feature `file-tree`), MarkdownRenderer, ResizableSplit, Toast, Dialog, TermTui
- **Appeal:** All-in-one. TreeView + FileSystemTree + MarkdownRenderer could serve the tree browser, content browser, and markdown rendering in one dependency.
- **Risk:** 138 downloads across all versions. No adoption signal. API may change. Single maintainer.
- **Recommendation:** Do not depend on it for initial implementation. Re-evaluate after it reaches 1.0 or gains meaningful adoption. If it stabilizes, it could replace tui-tree-widget and the markdown rendering decision (termimad vs pulldown-cmark) in one move.

### ratatui-interact (does not exist)

This crate does not appear on crates.io or docs.rs. Likely a hallucinated suggestion. Discard.

## 3. Architecture

```
TreeDataProvider (trait)
  |
  +-- FileSystemProvider (reads dirs from filesystem)
  +-- MerkleTreeProvider (reads nodes from NodeRecordStore)
  |
  v
TreeBrowser (our component)
  |
  +-- Tree panel (renders tui-tree-widget::Tree)
  +-- Path input (custom text field with tab completion)
  +-- Synchronization logic
```

The `TreeDataProvider` trait is the abstraction boundary. The `TreeBrowser` widget knows nothing about where nodes come from. Providers translate their data source into the interface the browser expects.

## 4. TreeDataProvider trait

```
pub trait TreeDataProvider {
    /// The type used to identify nodes. Must be displayable and convertible to/from path segments.
    type NodeId: Clone + Eq + Hash + Display;

    /// Return the root node.
    fn root(&self) -> TreeNodeInfo<Self::NodeId>;

    /// Return the children of a node. Called lazily when the user expands a node.
    /// Returns Ok(vec) on success, Err on permission/access error.
    fn children(&self, id: &Self::NodeId) -> Result<Vec<TreeNodeInfo<Self::NodeId>>, TreeError>;

    /// Resolve a path (slash-separated segments) to a node ID. Used when the user types a path.
    /// Returns None if the path does not resolve to a valid node.
    fn resolve_path(&self, segments: &[String]) -> Option<Self::NodeId>;

    /// Return the path segments for a node ID. Used to populate the path input when the user selects a node in the tree.
    fn path_segments(&self, id: &Self::NodeId) -> Vec<String>;

    /// Return display text for a node (may differ from its ID).
    fn display_text(&self, id: &Self::NodeId) -> Text;

    /// Return whether a node can be expanded (has or may have children).
    fn is_expandable(&self, id: &Self::NodeId) -> bool;

    /// Return a style hint for the node (normal, dimmed, highlighted, error).
    fn style_hint(&self, id: &Self::NodeId) -> NodeStyle;

    /// Optional: called when the path input changes to provide tab completions.
    /// Returns candidate completions for the last segment.
    fn completions(&self, partial_segments: &[String]) -> Vec<String>;
}
```

**TreeNodeInfo:**

```
pub struct TreeNodeInfo<Id> {
    pub id: Id,
    pub display: String,
    pub expandable: bool,
    pub style: NodeStyle,
}
```

**NodeStyle:**

```
pub enum NodeStyle {
    Normal,
    Dimmed,       // unreadable dir, node without context
    Highlighted,  // currently active/selected
    Error,        // broken symlink, cycle
    Custom(Style), // ratatui Style for provider-specific rendering
}
```

## 5. FileSystemProvider

Implements `TreeDataProvider` for filesystem directory browsing.

- **NodeId:** `PathBuf`
- **root():** Returns the starting directory (user home or `/`)
- **children():** Reads directory entries. Directories only (no files). Sorted alphabetically. Hidden directories (dotfiles) included but visually distinct.
- **Symlinks:** Followed. A visited set (based on canonical path) guards against cycles. If a symlink target was already visited, the node renders with `NodeStyle::Error` and text "(cycle)" appended. It cannot be expanded.
- **Permissions:** If `read_dir()` fails with a permission error, the node renders as `NodeStyle::Dimmed`. It cannot be expanded. No crash.
- **resolve_path():** Canonicalizes the segments joined with `/`, checks that the path exists and is a directory.
- **completions():** Lists entries in the parent directory that start with the partial last segment. Directories only.

**Lazy loading:** children() is called only when the user expands a node. Previously loaded children are cached in a `HashMap<PathBuf, Vec<TreeNodeInfo>>`. Cache is invalidated when the user presses a refresh key or when the provider is told to refresh a subtree.

## 6. MerkleTreeProvider

Implements `TreeDataProvider` for merkle node tree browsing.

- **NodeId:** `NodeID` (the existing merkle node ID type)
- **root():** Returns the workspace root node from `NodeRecordStore`
- **children():** Returns child nodes from `NodeRecord.children`. Includes both files and directories. Sorted: directories first, then files, alphabetically within each group.
- **resolve_path():** Walks the tree from root using `NodeRecordStore.find_by_path()`.
- **style_hint():** Based on context coverage:
  - `Normal` -- node has a head frame for the loaded agent
  - `Dimmed` -- node has no head frame (no context generated)
  - `Highlighted` -- node is the currently selected node
- **display_text():** Shows the node's filename (last path segment). Directories get a trailing `/`. Appends a context indicator (e.g. a checkmark or dot) based on head existence for the loaded agent.

**No lazy loading needed:** The merkle node tree is already fully in memory (NodeRecordStore). children() returns immediately.

## 7. TreeBrowser widget

The `TreeBrowser` is the assembled component that combines tree panel, path input, and synchronization.

### Layout

```
+-------------------------------------------+
| /home/user/projects/myapp                 |  <- Path input (1 row)
+-------------------------------------------+
| > projects/                               |  <- Tree panel (remaining height)
|   > myapp/              <-- selected      |
|     > src/                                |
|       main.rs                             |
|       lib.rs                              |
|     Cargo.toml                            |
|   > other-project/                        |
| > documents/                              |
+-------------------------------------------+
```

Path input is at the top. Tree panel fills the remaining space.

### Focus model

Two focus targets: path input and tree panel. Tab switches focus between them. Visual indicator: the focused panel has a highlighted border or cursor.

- **Tree focused (default):** Arrow keys navigate the tree. Enter expands/collapses or selects. Typing starts filtering visible nodes.
- **Path focused:** Text editing in the path input. Tab triggers completion. Enter confirms the path (navigates tree to it). Escape returns focus to tree.

### Synchronization

- **Tree selection changes path input:** When the user navigates the tree and selects a node, the path input updates to show the full path of the selected node (via `provider.path_segments()`).
- **Path input changes tree selection:** When the user edits the path and presses Enter, the tree navigates to the resolved node (via `provider.resolve_path()`). If the path is invalid, the path input shows an error indicator (red border or text) and the tree does not move.
- **Partial sync while typing:** As the user types in the path input, no tree navigation occurs until Enter is pressed. This avoids flickering and performance issues from resolving every keystroke.

### Tab completion

When the path input is focused and the user presses Tab:

1. Split the current input into path segments
2. Call `provider.completions()` with the segments
3. If one completion: auto-fill it and append `/`
4. If multiple completions: show a dropdown list below the path input. Arrow keys select, Enter confirms, Escape dismisses.
5. If no completions: no action

### Scrolling

The tree panel scrolls via `tui-tree-widget`'s built-in scrollbar. When navigating programmatically (path input confirms a deep path), the tree scrolls to make the selected node visible.

## 8. Selection and confirmation

The tree browser supports two modes of use:

**Picker mode (workspace selection):** The user is choosing a target. Enter on a node confirms the selection and closes the overlay. Returns the selected node ID. Cancel (Escape) returns None. Used by the workspace directory browser.

**Browse mode (content browser):** The user is exploring. Enter on a directory expands/collapses it. Enter on a file selects it and triggers a callback (e.g. display its content in an adjacent panel). The browser stays open. Used by the content browser view.

The mode is set by the caller when constructing the `TreeBrowser`.

## 9. Key bindings

### Tree panel focused

- `j`/`k` or Up/Down -- navigate
- `h` or Left -- collapse directory or go to parent
- `l` or Right -- expand directory
- Enter -- expand/collapse (directories), select/confirm (files or picker mode)
- Tab -- switch focus to path input
- `/` -- start inline search (filter visible nodes by typing)
- `r` -- refresh current node's children (re-call provider.children())
- `g` -- go to root
- Escape -- cancel (picker mode) or close search

### Path input focused

- Characters -- type path
- Backspace -- delete character
- Tab -- trigger completion
- Enter -- navigate tree to typed path
- Escape -- return focus to tree panel
- Ctrl+U -- clear path input
- Ctrl+A / Home -- cursor to start
- Ctrl+E / End -- cursor to end

## 10. Implementation location

All behind `#[cfg(feature = "tui")]`:

- `src/tui/tree_browser/mod.rs` -- module root, TreeBrowser widget, re-exports
- `src/tui/tree_browser/provider.rs` -- TreeDataProvider trait, TreeNodeInfo, NodeStyle
- `src/tui/tree_browser/filesystem.rs` -- FileSystemProvider implementation
- `src/tui/tree_browser/merkle.rs` -- MerkleTreeProvider implementation
- `src/tui/tree_browser/path_input.rs` -- path input widget, tab completion, cursor management
- `src/tui/tree_browser/sync.rs` -- synchronization logic between tree and path input
- `src/tui/tree_browser/state.rs` -- TreeBrowserState (wraps tui-tree-widget TreeState + path input state + focus + cache)

### Files updated

- `src/tui/views/content.rs` -- replace inline tree with TreeBrowser using MerkleTreeProvider
- `src/tui/views/init.rs` -- use TreeBrowser with FileSystemProvider for workspace selection
- `src/tui/overlays/directory_browser.rs` -- use TreeBrowser with FileSystemProvider in picker mode
- `Cargo.toml` -- add `tui-tree-widget` as optional dependency under the `tui` feature

### Dependency addition

```
tui-tree-widget = { version = "0.24", optional = true }
```

Under the tui feature:

```
tui = ["dep:ratatui", "dep:crossterm", "dep:tui-tree-widget"]
```

## 11. Required tests

### Unit tests

**TreeDataProvider trait (mock provider):**
- root() returns a valid root node
- children() returns children for an expandable node
- children() returns empty vec for a leaf node
- resolve_path() with valid segments returns the correct node ID
- resolve_path() with invalid segments returns None
- path_segments() returns segments that round-trip through resolve_path()
- completions() for a partial segment returns matching children
- completions() for a non-matching segment returns empty vec

**FileSystemProvider:**
- root() returns the configured starting directory
- children() lists only directories (no files) in sorted order
- children() on a directory with a symlink cycle marks the cyclic entry with NodeStyle::Error
- children() on an unreadable directory returns Err (provider does not panic)
- style_hint() returns Dimmed for unreadable directories
- resolve_path() with a valid directory path returns Some
- resolve_path() with a file path returns None (directories only for filesystem provider)
- resolve_path() with a nonexistent path returns None
- completions() returns directory names matching the partial segment
- Cache: calling children() twice returns the same result without re-reading the filesystem
- Cache invalidation: after refresh(), children() re-reads the filesystem

**MerkleTreeProvider:**
- root() returns the workspace root node
- children() returns child nodes sorted directories-first then alphabetically
- children() includes both files and directories
- style_hint() returns Normal for nodes with head frames, Dimmed for nodes without
- resolve_path() delegates to NodeRecordStore.find_by_path()
- display_text() for a directory appends `/`

**Path input:**
- Typing characters appends to the input
- Backspace removes the last character
- Tab with one completion auto-fills and appends `/`
- Tab with multiple completions opens completion list
- Tab with no completions does nothing
- Enter on a valid path returns the resolved segments
- Enter on an invalid path returns error indicator
- Ctrl+U clears the input

**Synchronization:**
- Selecting a node in the tree updates the path input to match
- Confirming a path in the input navigates the tree to that node
- Confirming an invalid path does not move the tree selection
- Switching focus between tree and path input preserves both states

**TreeBrowser modes:**
- Picker mode: Enter on a leaf returns Some(node_id) and signals close
- Picker mode: Escape returns None
- Browse mode: Enter on a directory toggles expand/collapse
- Browse mode: Enter on a file triggers the selection callback without closing

### Integration tests (headless)

**FileSystemProvider with real filesystem:**
- Create a temp directory structure, verify children() returns correct entries
- Create a symlink cycle, verify it is detected and marked
- Create an unreadable directory (chmod 000), verify Dimmed style and no panic
- Verify tab completions against real directory contents

**MerkleTreeProvider with real store:**
- Build a node tree in store, verify children() matches expected structure
- Verify style_hint() reflects actual head existence
- Verify resolve_path() works for nested paths

**TreeBrowser round-trip:**
- Open browser with FileSystemProvider, navigate to a directory via tree, verify path input matches
- Open browser with FileSystemProvider, type a path, verify tree navigates to it
- Open browser with MerkleTreeProvider, navigate the tree, verify path input shows merkle paths

## 12. Design nuances

### Visible depth limit

Very deep filesystem hierarchies (e.g. `node_modules`) can produce enormous trees. The FileSystemProvider should support a configurable max depth. Nodes at the max depth render as expandable but show "(depth limit)" when expanded. The path input can still navigate beyond the visible depth.

### Performance of large directories

Directories with thousands of entries (e.g. a flat directory with 10,000 files) need consideration. The tree widget itself handles this well (it only renders visible rows). The provider's children() call should not block the event loop. For the filesystem provider, consider spawning a blocking task for `read_dir()` on directories expected to be large, returning a loading indicator while waiting. For the merkle provider, the store is in memory so this is not a concern.

### Path input ambiguity

For the filesystem provider, paths are absolute (`/home/user/...`). For the merkle provider, paths are relative to workspace root (`src/main.rs`). The path input should show a prefix indicating the context:
- Filesystem: shows the absolute path
- Merkle: shows the path relative to workspace root, optionally prefixed with `./`

The provider's `path_segments()` and `resolve_path()` handle this distinction. The TreeBrowser itself does not interpret paths.

### Inline search

When the user presses `/` in the tree panel, a small search input appears at the bottom of the tree panel. Typing filters visible nodes to those matching the search string (substring match). The first match is selected. Enter confirms and closes search. Escape cancels search and restores the previous selection. This is implemented in the TreeBrowser, not in the provider.

### tui-tree-widget TreeItem construction

`tui-tree-widget` expects all visible `TreeItem`s to be constructed upfront. With lazy loading, we build TreeItems for only the expanded subtree:
1. Start with root TreeItem
2. When a node is expanded, call `provider.children()` and insert child TreeItems
3. Collapsed subtrees have their TreeItems removed (or kept in cache)
4. On each render, rebuild the `Tree` widget from the current expanded state

This means the TreeBrowser's state maintains a parallel structure: the expanded node set (which providers have been queried) alongside `tui-tree-widget`'s `TreeState` (which tracks selection and visual open/close state).

## 13. Related docs

- **tui_session_state_spec.md** -- Workspace selection flow and content browser integration that use this component.
- **tui_spec.md** -- The parent TUI spec. Section 4.3 (content browser) uses MerkleTreeProvider. The directory browser overlay uses FileSystemProvider.
- **design/observability/observability_spec.md** -- Not directly related, but session events drive what the content browser shows.
