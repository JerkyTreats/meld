# Tree browser specification

## 1. Purpose

The tree browser is a reusable navigation component for the TUI.

It should support two data sources:

- filesystem directories
- meld node records

The same component should work for content browsing, workspace selection, and future hierarchical pickers.

## 2. Current code constraints

The tree browser must fit the current storage model.

Important constraints:

- `NodeRecordStore` can list records, get a record by id, and resolve a record by canonical path
- `NodeRecord.children` stores child ids only
- `NodeRecord.path` stores canonical paths
- head coverage data exists through the head index, but subtree coverage is not precomputed per directory

This means the TUI provider must do a small amount of adaptation work:

- build labels for child rows from child record paths
- cache expanded node children
- compute optional decorations such as has frame or no frame

## 3. Recommended dependency

Use `tui-tree-widget` for the first implementation.

Why:

- mature enough for a first pass
- focused on tree rendering rather than application structure
- works with custom identifiers and external state

The TUI should own path input, focus, caching, and synchronization.

## 4. Provider contract

The browser should depend on a small provider trait.

```rust
pub trait TreeDataProvider {
    type NodeId: Clone + Eq + Hash;

    fn root(&self) -> Result<TreeEntry<Self::NodeId>, TreeError>;
    fn children(&self, id: &Self::NodeId) -> Result<Vec<TreeEntry<Self::NodeId>>, TreeError>;
    fn resolve_path(&self, path: &str) -> Result<Option<Self::NodeId>, TreeError>;
    fn display_path(&self, id: &Self::NodeId) -> Result<String, TreeError>;
    fn is_expandable(&self, id: &Self::NodeId) -> Result<bool, TreeError>;
}

pub struct TreeEntry<Id> {
    pub id: Id,
    pub label: String,
    pub kind: TreeEntryKind,
    pub status: TreeEntryStatus,
}
```

The contract should stay narrow. The browser owns focus and rendering state. Providers only answer data questions.

## 5. Filesystem provider

The filesystem provider is used for workspace selection and directory browsing.

Rules:

- root is the chosen start directory
- children list directory contents in stable sorted order
- the first release may show directories only for picker style flows
- path resolution should canonicalize before confirming a match
- unreadable directories should surface a visible error state rather than panic

## 6. Meld tree provider

The meld tree provider is used by content view.

Identifier choice:

- use `NodeID`

Lookup rules:

- root resolves from the workspace root node record
- children load through `NodeRecord.children` and record lookup by child id
- labels come from the last visible path segment of each child record path
- path resolution should use canonical workspace relative paths and delegate to existing store lookup where possible

Status decoration for the first release:

- normal for nodes with at least one active head
- dim or muted for nodes without active heads

More advanced directory coverage can come later.

## 7. Path handling

The browser needs a text path input that stays in sync with tree selection.

Display rules:

- filesystem provider shows absolute paths
- meld tree provider shows workspace relative paths

Sync rules:

- selecting a tree row updates the path input
- confirming a valid path updates tree selection
- confirming an invalid path keeps selection unchanged and shows an error state

## 8. Browser state

The browser state should include:

- selected node id
- expanded node set
- cached children by node id
- path input text
- current focus, either tree or path input
- optional completion list state

The cache should be invalidated by explicit refresh actions.

## 9. Performance guidance

- cache children after first expansion
- keep tree item construction limited to the visible expanded subtree
- avoid full store scans during every key press
- do not compute subtree frame coverage in the first release

The meld tree provider may preload a workspace snapshot once per content view load, but it should not rebuild that snapshot for every render tick.

## 10. File layout

Suggested files:

- `src/tui/tree.rs`
- `src/tui/tree/provider.rs`
- `src/tui/tree/filesystem.rs`
- `src/tui/tree/meld.rs`
- `src/tui/tree/state.rs`
- `src/tui/tree/input.rs`

## 11. Integration with content view

The content view should use the meld tree provider and load frame content for the selected node.

Loading order:

- resolve selected node id from the tree browser
- choose frame filter from loaded agent or explicit content view filter
- load context through existing `ContextApi` methods
- render frame text and metadata in the content pane

## 12. Required tests

### Unit tests

- filesystem provider returns stable sorted children
- filesystem provider handles unreadable directories without panic
- meld provider resolves children from child ids correctly
- meld provider derives visible labels from canonical paths
- path selection updates the input field
- valid path confirmation moves tree selection
- invalid path confirmation leaves selection unchanged

### Integration tests

- filesystem provider navigates a temp directory tree
- meld provider navigates a temp store built from a scanned workspace
- content view loads the selected node frame after tree selection changes

## 13. Deferred work

- subtree coverage counts per directory
- fuzzy search across tree labels
- richer completion menus
- incremental refresh driven by watch events
