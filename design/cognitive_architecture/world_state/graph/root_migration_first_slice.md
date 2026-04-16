# Root Migration First Slice

Date: 2026-04-15
Status: active
Scope: concrete contracts and rollout design for root registration, migration bookkeeping, and operator visible status

## Objective

This slice is the `workspace_fs` first landing under the broader branch substrate in [Branch Federation Substrate](branch_federation_substrate.md).

Turn the root migration architecture into a narrow design slice that can land safely in code.

This slice should solve the real blockers first:

- stable root identity
- manifest authority
- migration bookkeeping
- dormant root visibility
- startup integration for active roots

This slice should not attempt federation reads or broad authoritative rewrites.

## Domain Ownership

This work should land in a new `roots` domain.

Recommended landing zones:

- `src/roots.rs`
- `src/roots/contracts.rs`
- `src/roots/locator.rs`
- `src/roots/manifest.rs`
- `src/roots/catalog.rs`
- `src/roots/ledger.rs`
- `src/roots/runtime.rs`
- `src/roots/query.rs`
- `src/roots/tooling.rs`

Why a separate domain:

- root discovery is broader than `workspace`
- root migration bookkeeping is broader than `world_state`
- CLI routes should delegate to a clear root domain surface

`world_state/graph` should continue to own derived graph catch up only.

## First Slice Decisions

### Decision 1

Manifest is the local authority for one root.

The catalog is a global cache and index.

### Decision 2

Migration uses typed step records with append only history.

There is no generic `down` path in the first slice.

### Decision 3

The first slice supports metadata steps and derived replay steps only.

It does not perform destructive rewrites of node, frame, artifact, or workflow data.

### Decision 4

`roots status` lands before `roots discover` and `roots migrate`.

This gives operators visibility before bulk action.

### Decision 5

Root identity resolution happens before `RunContext` locks in storage paths.

Without that boundary, renamed roots will fork by path.

## Root Identity Contract

The first slice should separate two identifiers:

- `root_id`
  stable root identity used by federation and catalogs
- `workspace_path`
  current canonical path used to open the live workspace

The first slice may still derive `root_id` from existing path based state for legacy roots, but once assigned it becomes the durable root identity.

### Locator Inputs

The locator should inspect:

- requested workspace path
- canonical workspace path
- local data home path
- root manifest if present
- root catalog entry if present

### Locator Outcomes

The locator should return one of:

- `resolved`
- `relocated`
- `unregistered`
- `ambiguous`

`ambiguous` should stop automatic mutation and require explicit operator action.

## Manifest Contract

The manifest should live in the local data home as one serde round trippable record.

Recommended path:

- `<root_data_home>/root_manifest.json`

Recommended record:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootManifest {
    pub root_id: String,
    pub workspace_path: String,
    pub workspace_locator_version: u32,
    pub authoritative_state_version: u32,
    pub derived_state_version: u32,
    pub migration_runtime_version: u32,
    pub last_seen_at: String,
    pub last_successful_plan_id: Option<String>,
    pub last_successful_step_id: Option<String>,
}
```

Field meanings:

- `root_id`
  stable identity for this root
- `workspace_path`
  last known canonical workspace path
- `workspace_locator_version`
  version of root location semantics
- `authoritative_state_version`
  version marker for durable business state compatibility
- `derived_state_version`
  version marker for graph projection layout
- `migration_runtime_version`
  version of the migration planner and step semantics
- `last_seen_at`
  last successful runtime contact
- `last_successful_plan_id`
  most recent verified plan
- `last_successful_step_id`
  most recent verified step

First slice rule:

- manifest writes happen only after local inspection succeeds

## Catalog Contract

The catalog should live in the global Meld data home.

Recommended path:

- `<data_home>/meld/root_catalog.json`

Recommended record shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootCatalog {
    pub roots: Vec<RootCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootCatalogEntry {
    pub root_id: String,
    pub workspace_path: String,
    pub data_home_path: String,
    pub attachment_status: RootAttachmentStatus,
    pub inspection_status: RootInspectionStatus,
    pub migration_status: RootMigrationStatus,
    pub last_seen_at: Option<String>,
    pub last_inspected_at: Option<String>,
    pub last_migration_at: Option<String>,
}
```

Supporting enums:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RootAttachmentStatus {
    Active,
    Dormant,
    MissingWorkspacePath,
    Ambiguous,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RootInspectionStatus {
    Unknown,
    Registered,
    InspectionRequired,
    InvalidCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RootMigrationStatus {
    Unknown,
    NotNeeded,
    ReplayNeeded,
    InProgress,
    Failed,
    Succeeded,
}
```

First slice rule:

- the catalog may be rebuilt from manifests and inspections
- the manifest may not be rebuilt from the catalog alone

## Ledger Contract

The ledger should record each attempted step as durable history.

Recommended path:

- `<root_data_home>/migration_ledger.jsonl`

Recommended record:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootMigrationLedgerEntry {
    pub plan_id: String,
    pub step_id: String,
    pub lane: RootMigrationLane,
    pub status: RootMigrationStepStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error_summary: Option<String>,
    pub observed_inputs: Vec<String>,
    pub verification_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RootMigrationLane {
    Metadata,
    Derived,
    AuthoritativeCompatibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RootMigrationStepStatus {
    Started,
    Verified,
    Failed,
    Skipped,
}
```

First slice rule:

- each line is one immutable fact
- retries create new entries
- verification must be explicit before a step counts as successful

## Step Set For The First Slice

The first slice should support only these step ids:

- `register_root`
- `write_root_manifest`
- `refresh_catalog_entry`
- `initialize_traversal_projection`
- `replay_spine_into_traversal`
- `mark_derived_version`

### Step Preconditions

`register_root`

- canonical workspace path resolved
- local data home resolved

`write_root_manifest`

- inspection completed
- `root_id` assigned

`refresh_catalog_entry`

- manifest loaded or written

`initialize_traversal_projection`

- root store open succeeds

`replay_spine_into_traversal`

- progress store readable
- traversal store open succeeds

`mark_derived_version`

- derived replay verification passes

## Root Runtime Interface

The `roots` domain should expose one small runtime surface.

Recommended shape:

```rust
pub struct RootRuntime;

impl RootRuntime {
    pub fn resolve_active_root(
        &self,
        workspace_root: &Path,
    ) -> Result<ResolvedRoot, ApiError>;

    pub fn inspect_root(
        &self,
        resolved: &ResolvedRoot,
    ) -> Result<RootInspectionReport, ApiError>;

    pub fn plan_active_startup(
        &self,
        report: &RootInspectionReport,
    ) -> Result<RootMigrationPlan, ApiError>;

    pub fn apply_plan(
        &self,
        plan: &RootMigrationPlan,
    ) -> Result<RootMigrationReport, ApiError>;

    pub fn status(
        &self,
    ) -> Result<RootsStatusOutput, ApiError>;
}
```

### Supporting Reports

The first slice needs:

- `ResolvedRoot`
- `RootInspectionReport`
- `RootMigrationPlan`
- `RootMigrationReport`
- `RootsStatusOutput`

Those should live under `src/roots/contracts.rs`.

## `roots status` CLI Contract

This should be a new top level CLI command because dormant roots are global, not tied to one active workspace.

Recommended parse shape:

```rust
pub enum Commands {
    // existing variants
    Roots {
        command: RootsCommands,
    },
}

pub enum RootsCommands {
    Status {
        format: String,
    },
}
```

Text output should be operator focused.

Minimum fields per row:

- `root_id`
- `workspace_path`
- `attachment_status`
- `migration_status`
- `last_seen_at`

Json output should preserve explicit machine readable fields.

Recommended result:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootsStatusOutput {
    pub roots: Vec<RootStatusRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootStatusRow {
    pub root_id: String,
    pub workspace_path: String,
    pub data_home_path: String,
    pub attachment_status: String,
    pub inspection_status: String,
    pub migration_status: String,
    pub last_seen_at: Option<String>,
    pub last_migration_at: Option<String>,
}
```

## Startup Integration Design

### Existing Path

Today `RunContext::new` does these high level actions:

1. load config
2. resolve storage paths
3. open sled
4. create graph runtime
5. open frame and artifact stores
6. run graph catch up

### First Slice Change

Insert root resolution and root startup planning before storage paths are finalized.

Recommended high level flow:

1. load config
2. resolve active root through `RootRuntime`
3. finalize root aware storage paths
4. open sled and local stores
5. inspect root state
6. plan metadata and derived steps
7. apply plan
8. continue normal `RunContext` startup
9. run graph catch up as part of the applied plan or as a final startup action

### Boundary Rule

`RunContext` should orchestrate this path.

`world_state/graph` should not learn about manifests or catalogs.

## Discovery Policy For This Slice

`roots status` should read:

- catalog entries if present
- active root manifest if present

It should not perform broad disk discovery in the first slice.

Reason:

- the local install already shows many temp roots under `~/.local/share/meld/tmp`
- broad discovery without strict policy will create noisy operator output

`roots discover` should be a later slice after the candidate rules are implemented and tested.

## Rollout Plan

1. Contracts

- add `src/roots/contracts.rs`
- add serde tests for manifest, catalog, and ledger

2. Persistence

- add manifest reader and writer
- add catalog reader and writer
- add jsonl ledger append helper

3. Locator

- add active root resolution for one workspace path
- support `resolved` and `unregistered`
- defer `relocated` and `ambiguous` write behavior until status can expose them cleanly

4. Runtime

- add startup inspection and planning for active roots
- record metadata steps and derived replay steps

5. CLI

- add `Commands::Roots`
- add `RootsCommands::Status`
- add text and json formatters

6. Verification

- add integration tests for active root startup
- add integration tests for `roots status`
- add failure and retry ledger tests

## Test Plan

### Contract Tests

- manifest serde round trip
- catalog serde round trip
- ledger line parse and append behavior

### Runtime Tests

- unregistered active root creates manifest and catalog entry
- registered active root updates `last_seen_at`
- derived replay step writes verified ledger entries
- repeated startup is idempotent

### CLI Tests

- `roots status` text output for one active root
- `roots status --format json` shape stability
- status output with empty catalog

### Failure Tests

- manifest write failure records failed ledger step
- traversal replay failure leaves authoritative state readable
- retry appends new verified entries rather than mutating history

## Acceptance Gate

This slice is complete when all of these are true:

- one active root can self register on startup
- one active root gets a manifest and catalog entry
- derived replay records a plan and verified steps
- `roots status` exposes active and dormant root state clearly
- no graph code depends on manifest or catalog internals
- no authoritative business data is rewritten to land the slice
