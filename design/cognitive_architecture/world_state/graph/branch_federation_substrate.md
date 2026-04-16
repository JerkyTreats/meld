# Branch Federation Substrate

Date: 2026-04-15
Status: active
Scope: canonical branch abstraction, workspace federation plan, and extensible substrate for non-filesystem branches

## Thesis

`branch` is the durable abstraction.

`workspace_fs` is the first branch kind.

Federation should not be designed as many filesystem roots glued together.
It should be designed as one temporal graph that composes facts from many durable branches over one shared event spine.

That position satisfies the current workspace problem and leaves room for future branch kinds such as thesis, evidence, and other attached domains without redefining the substrate.

## Why Branch Is The Right Boundary

The system needs one durable unit that can answer all of these questions:

- where local source truth lives
- how local current state is named
- which event stream participates in the spine
- which derived projections are available for traversal
- how branch health and migration are tracked

Filesystem roots satisfy those constraints today, but they are not the general case.
A thesis with supporting evidence artifacts has the same durability shape even if it has no backing filesystem tree.

So the model should be:

- one canonical `branch` contract
- many concrete `branch_kind` implementations
- `workspace_fs` as the first implementation

## Design Goals

The branch substrate must satisfy these goals:

- many filesystem roots with stable branch identity
- later non-filesystem branches without substrate reset
- spine first durability and replay semantics
- scoped and performant traversal
- clear provenance from branch local facts back to spine history
- compatibility with current root migration work

## Core Model

Every branch has four durable concerns:

### 1. Identity

- `branch_id`
- `branch_kind`
- display metadata
- locator metadata

Identity must be durable even if locator metadata changes.

### 2. Source Truth

Branch local durable objects and branch local mutable refs.

Examples:

- `workspace_fs`
  node records, frames, artifacts, workflow state, head index
- `thesis`
  thesis objects, evidence artifacts, assertion sets, current thesis refs

### 3. Spine Participation

Each branch publishes promoted facts into one stable spine stream.

The spine remains the only canonical cross-branch temporal history.

### 4. Derived Projections

Branch local traversal indexes, current anchor indexes, provenance indexes, and future caches.
These are rebuildable from source truth plus spine history.

## Branch Contract

The canonical branch contract should be the durable runtime surface.

Recommended manifest shape:

```rust
pub struct BranchManifest {
    pub branch_id: String,
    pub branch_kind: String,
    pub display_name: String,
    pub canonical_locator: String,
    pub locator_version: u32,
    pub authoritative_state_version: u32,
    pub derived_state_version: u32,
    pub migration_runtime_version: u32,
    pub spine_stream_id: String,
    pub branch_status: BranchStatus,
    pub capabilities: Vec<BranchCapability>,
    pub created_at: String,
    pub last_seen_at: String,
    pub last_successful_plan_id: Option<String>,
    pub last_successful_step_id: Option<String>,
    pub branch_kind_state: serde_json::Value,
}
```

Notes:

- `branch_id` is authoritative
- `canonical_locator` is mutable metadata
- `branch_kind_state` carries kind specific details without leaking them into federation contracts
- `spine_stream_id` gives each branch one durable temporal anchor

Recommended catalog role:

- global index of known branches
- attachment and health cache
- last known locator and storage paths
- not the authority for one branch

Recommended ledger role:

- append only execution history for migration and repair steps
- branch scoped and lane aware

## Workspace As The First Branch Kind

`workspace_fs` should be canonicalized as `branch_kind = "workspace_fs"`.

Its kind specific state should include:

- canonical workspace path
- local data home path
- workspace path history when relocations are reconciled
- compatibility markers for legacy root layout

This keeps the current root migration work useful while making it clear that root is a legacy operator term, not the enduring substrate abstraction.

## Semantic Identity And Branch Presence

Federation needs one distinction that the current root model does not capture explicitly:

- semantic object identity
- branch presence of that object

That means one object can exist as itself across many branches while still preserving branch local provenance.

Recommended model:

```rust
pub struct BranchObjectPresence {
    pub branch_id: String,
    pub object_ref: DomainObjectRef,
    pub introduced_at_seq: u64,
    pub retired_at_seq: Option<u64>,
}
```

This allows the federated layer to answer:

- where an object is present now
- where it came from
- which branches should participate in a walk

Without this distinction, a merged graph will blur semantic identity and local provenance.

## Spine Invariants

The branch substrate must preserve these spine rules:

- spine append remains the only canonical cross-branch history
- `seq` remains stable and replayable
- branch facts are published, not inferred by hidden cross-domain calls
- reducers may rebuild projections from spine plus branch local source truth
- projection failure must never corrupt authoritative branch state

This keeps federation aligned with the current spine design rather than turning the graph layer into the authority.

## Traversal Model

Traversal should be branch aware from the start.

The runtime should support:

- one branch scope
- active branch scope
- selected branch set
- all attached healthy branches

Every federated query should accept:

- branch scope
- depth cap
- relation filter
- current only mode
- provenance hydration mode

The planner should resolve scope before opening stores widely.
Traversal should remain bounded and selective.

## Substrate Architecture

The canonical runtime stack should look like this:

1. Branch local source stores
2. Branch local derived projections
3. Federated branch read runtime
4. External query surfaces

The external surfaces may be:

- CLI
- local HTTP service
- visualization feed
- future planner adapters

Those surfaces should never read raw branch stores directly.

## Domain Layout

The current `roots` domain should evolve into a `branches` domain with compatibility wrappers.

Target landing zones:

- `src/branches.rs`
- `src/branches/contracts.rs`
- `src/branches/locator.rs`
- `src/branches/manifest.rs`
- `src/branches/catalog.rs`
- `src/branches/ledger.rs`
- `src/branches/runtime.rs`
- `src/branches/query.rs`
- `src/branches/tooling.rs`
- `src/branches/kinds/workspace_fs.rs`

Compatibility path:

- keep `roots` as a thin compatibility facade during migration
- preserve existing root manifest and catalog readers
- write forward compatible records that can be loaded as branches

## Detailed Implementation Plan

### Phase 0

Freeze terminology and contracts in design.

Deliverables:

- branch vocabulary becomes canonical in design docs
- root is explicitly scoped to `workspace_fs` compatibility work
- branch contract, catalog role, ledger role, and spine invariants are frozen

### Phase 1

Generalize runtime metadata from root to branch.

Deliverables:

- introduce `BranchManifest`, `BranchCatalog`, and `BranchMigrationLedger`
- keep compatibility readers for `RootManifest`, `RootCatalogEntry`, and root ledger records
- add `branch_kind = "workspace_fs"` for existing workspace registrations
- add stable `branch_id` field to all new records

Code path:

- start in the current `src/roots/` code
- add shared conversion types
- rename internal concepts before renaming CLI terms

Exit criteria:

- existing active workspace startup still self registers
- `meld roots status` can render branch aware records
- no existing workspace data becomes unreadable

### Phase 2

Introduce branch runtime boundaries.

Deliverables:

- `BranchLocator`
- `BranchRuntime`
- `BranchHandle`
- `BranchKindAdapter`

Responsibilities:

- resolve branch identity before storage path finalization
- open one branch by `branch_id`
- expose branch kind capabilities
- hide filesystem specific assumptions from callers

`workspace_fs` is the only real adapter in this phase.

Exit criteria:

- `RunContext` resolves an active workspace through `BranchRuntime`
- graph catch up uses branch handles rather than path first root assumptions

### Phase 3

Assign branch scoped spine identity and object presence facts.

Deliverables:

- stable `spine_stream_id` per branch
- branch presence materialization for `DomainObjectRef`
- reducers that persist presence facts alongside traversal facts

Why this phase matters:

- cross-branch identity remains ambiguous without branch presence
- future thesis and evidence branches need the same identity model

Exit criteria:

- one object can be present in more than one branch without losing provenance
- federated planning can determine which branches contain a queried object

### Phase 4

Build the federated branch read runtime.

Deliverables:

- `FederatedBranchQuery`
- branch scope selectors
- per branch store opening strategy
- merged traversal results with branch aware provenance

Supported first operations:

- graph status by scope
- object lookup by scope
- neighbor expansion by scope
- bounded walk by scope
- current anchors by scope
- provenance by scope

Exit criteria:

- one query surface can read many workspace branches
- single branch queries match current local behavior
- unhealthy branches are excluded cleanly with surfaced status

### Phase 5

Add operator flows for dormant branches.

Deliverables:

- `meld branches status`
- `meld branches discover`
- `meld branches migrate`
- `meld branches attach`

Compatibility:

- keep `meld roots status` as an alias until branch terminology is fully adopted

Exit criteria:

- dormant workspace branches can be discovered, migrated, and inspected without opening them as the active workspace

### Phase 6

Land the second branch kind.

Recommended second kind:

- `thesis`

Reason:

- it proves the substrate is not filesystem specific
- it exercises semantic identity, provenance, and branch local refs in a non-tree domain

Deliverables:

- `src/branches/kinds/thesis.rs`
- thesis manifest state
- thesis spine publication rules
- thesis traversal projection rules

Exit criteria:

- one federated query can traverse across `workspace_fs` and `thesis` branches using the same branch substrate

## Performance Plan

Federation must not degrade into open everything and scan everything.

The performance plan should be:

- resolve branch scope first
- resolve candidate branches for an object before walk fanout
- keep branch local adjacency indexes
- use bounded depth always
- hydrate provenance after the walk when possible
- allow read snapshots for stable visualization sessions
- cache branch health and readiness in the catalog

If a later optimization is needed, add federated caches as derived projections.
Do not make them authoritative.

## Reliability Plan

The reliability model should remain simple:

- append facts once to the spine
- rebuild projections as needed
- never rewrite authoritative branch state for traversal convenience
- keep migration step history append only
- surface ambiguous locator states instead of guessing

This is the same durability discipline already used in the root migration architecture, promoted to the branch level.

## Compatibility Strategy

The branch shift should preserve current operator expectations.

Rules:

- active workspace migration remains invisible when safe
- root local data homes remain valid
- root terms may remain in operator surfaces during transition
- filesystem paths remain locators, not authorities
- all record format changes require compatibility readers before cleanup

## Test Plan

Required characterization and parity tests:

- existing single workspace behavior remains unchanged
- renamed workspace does not fork branch identity after reconciliation
- federated single branch queries match local branch queries
- multi workspace federation returns stable merged results
- unhealthy branch exclusion is surfaced and deterministic
- branch presence facts survive replay
- dormant branch discovery does not register temp residue as real branches
- thesis branch traversal shares the same query surface as workspace traversal

## Non Goals

This plan does not require:

- one physical merged graph store
- external graph database adoption as core authority
- distributed spine sequencing in the first federation slice
- destructive rewrites of legacy workspace data

## Recommendation

Canonicalize `branch` now.

Keep `workspace_fs` as the first branch kind and the proving ground for the substrate.
Finish federation through a branch runtime, branch presence facts, and a federated read layer.
Then add the second branch kind to prove the model is truly general.
