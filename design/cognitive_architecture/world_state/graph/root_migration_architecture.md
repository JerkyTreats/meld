# Root Migration Architecture

Date: 2026-04-15
Status: active
Scope: architectural migration model for root scoped state evolution, graph projection catch up, and federation readiness

## Thesis

This document is the `workspace_fs` compatibility and migration model inside the broader branch substrate in [Branch Federation Substrate](branch_federation_substrate.md).

Meld should not treat migration as one generic schema upgrade flow.

It has three different state lanes:

- authoritative root state
- derived graph state
- discovery and operations metadata

Those lanes need different migration strategies.

The right model is a root scoped migration runtime that combines versioned steps with live inspection and reconciliation.

## Why A Database Style Model Is Not Enough

A classic database migration model assumes:

- one authoritative schema
- one ordered history of `up` steps
- optional reverse `down` steps

Meld does not fit that shape cleanly.

The local root contains durable source truth such as node records and frames, but it also contains rebuildable graph projections and future federation metadata.

That means migration has to answer two different questions:

- what source truth must remain readable
- what derived state can be rebuilt or replayed

The migration architecture should borrow governance discipline from database migrations without copying their storage assumptions.

## Root Scoped Lanes

### Authoritative Lane

This lane holds source truth that operators depend on directly.

Examples:

- node record store
- frame storage
- artifacts
- workflow state
- head index persistence
- ignore and publish state

Rules:

- prefer additive changes
- prefer compatibility readers before rewrites
- use one way transforms only when a compatibility path is impossible
- never make the root unreadable to land graph support

### Derived Lane

This lane exists to accelerate reads and materialize graph state.

Examples:

- event spine indexes
- traversal facts
- current anchor indexes
- provenance indexes
- future federation read caches

Rules:

- prefer replay
- prefer rebuild over in place repair
- treat projection state as disposable if source truth is intact
- record cursors and versions, but do not confuse them with source truth

### Metadata Lane

This lane records runtime knowledge about roots and migration work.

Examples:

- root manifest
- root catalog
- migration ledger
- last seen and last attempted timestamps
- operator attachment status

Rules:

- metadata records what runtime observed and attempted
- metadata may be repaired from inspection
- metadata should never be the only copy of source truth

## Core Contracts

### Root Manifest

The root manifest should be the authoritative local metadata record for one root.

Minimum fields:

- `root_id`
- `workspace_path`
- `workspace_locator_version`
- `authoritative_state_version`
- `derived_state_version`
- `migration_runtime_version`
- `last_seen_at`
- `last_successful_plan_id`
- `last_successful_step_id`

The manifest lives in the local data home and travels with the root runtime state.

### Root Catalog

The root catalog should be a federation registry, not a second authority for local root state.

It should cache:

- `root_id`
- last known workspace path
- local data home path
- attachment status
- last inspection summary
- last migration summary

Authority rule:

- manifest is authoritative for one root
- catalog is a global index and cache derived from manifests and inspections

That avoids dual writes where both records pretend to be source truth.

### Migration Ledger

Each root should have a durable migration ledger.

The ledger records each attempted step with:

- `plan_id`
- `step_id`
- `lane`
- `status`
- `started_at`
- `finished_at`
- `error_summary`
- `observed_inputs`
- `verification_summary`

The ledger is append only.

The ledger is not a replacement for the manifest.
It is the execution history that explains why the root is in its current state.

## Step Model

Migration should use typed steps rather than one coarse root version jump.

Each step should declare:

- stable step id
- lane ownership
- preconditions
- inspection inputs
- apply behavior
- verification behavior
- resume behavior

Recommended execution flow:

1. inspect
2. plan
3. apply
4. verify
5. record

The key rule is that planning uses both declared versions and live inspection.

Versions tell runtime what the root claims.
Inspection tells runtime what is actually present.

## Step Families

The first migration architecture should support a narrow set of step families.

### Registration Steps

Examples:

- `register_root`
- `backfill_manifest`
- `refresh_catalog_entry`
- `reconcile_workspace_path`

These steps touch the metadata lane only.

### Authoritative Compatibility Steps

Examples:

- `upgrade_head_index_reader`
- `backfill_authoritative_version_marker`
- `record_legacy_layout_signals`

These steps should be rare and conservative.

They should usually add compatibility or markers, not rewrite business data.

### Derived Projection Steps

Examples:

- `initialize_traversal_projection`
- `replay_spine_into_traversal`
- `rebuild_anchor_indexes`
- `mark_derived_version`

These steps should be easy to rerun.

### Federation Preparation Steps

Examples:

- `assign_stable_root_id`
- `attach_root_to_catalog`
- `mark_federation_ready`

These steps make multi root reads possible without forcing physical store unification.

## Failure And Recovery Model

The recovery model should prefer resume and forward repair.

It should not optimize for generic `down` steps.

### Why Not General `down`

`down` is the wrong default for this system because:

- source truth must stay readable
- derived state can be rebuilt
- rollback across mixed local stores is hard to trust
- most failures are better handled by resume from the last verified point

### Required Recovery Guarantees

- interruption during derived replay leaves source truth intact
- interruption during metadata updates leaves the root inspectable
- retry can continue from the last verified step
- a failed plan can be superseded by a later corrective plan

### Safe States

After any failure the root should remain in one of these states:

- source truth readable and derived state stale
- source truth readable and derived state partially caught up with a recorded cursor
- metadata inconsistent but repairable from local inspection

## Runtime Insertion Points

### Pre Open Root Resolution

There needs to be a root locator layer before `RunContext` commits to a storage path.

This layer is responsible for:

- canonical workspace path resolution
- root id lookup
- moved root reconciliation
- local data home selection

Without that layer, path derived identity will fork roots after a rename or move.

### Startup Path

On active root startup the runtime should:

1. resolve root identity and data home
2. inspect manifest and local stores
3. plan metadata and derived steps if needed
4. apply safe additive steps
5. run derived catch up
6. verify and record results

This extends the current startup path rather than replacing it.

### Command Completion Path

After command execution the runtime should:

- update `last_seen_at`
- run derived catch up if the spine advanced
- record any cursor changes

### Explicit Operator Path

Dormant roots need explicit commands because they are not opened through normal work.

Recommended first commands:

- `meld roots status`
- `meld roots discover`
- `meld roots migrate`

`meld roots attach <path>` can land later if discovery misses valid roots.

## Planning Rules

The planner should classify each root into one of these outcomes:

- no action
- metadata registration only
- derived replay only
- compatibility plus replay
- operator intervention required

The planner should not rely on one version integer alone.

It should use:

- manifest versions
- observed files and trees
- known legacy signals
- workspace path and locator state
- last successful ledger entry

## Discovery Rules

Discovery must be strict enough to avoid registering disposable roots.

Candidate signals should be weighted, not treated as independently sufficient.

Suggested rule:

- require `store` plus at least one other durable signal
- exclude temp roots by policy
- detect nested roots and record parent relationship instead of flattening them
- mark ambiguous candidates as `inspection_required`

This matters because a live local install can contain:

- real long lived roots
- nested roots
- temporary test roots
- partially initialized roots

## Verification Model

Every applied step needs a verification contract.

Examples:

- manifest write round trips through serde
- expected sled trees exist
- `last_reduced_seq` advanced or remained stable by design
- repeated catch up is idempotent
- catalog entry matches manifest identity

Verification should be small and local.
It should run immediately after each step or step batch.

## Minimal First Slice

The first implementation slice should stay narrow.

Land:

- root manifest contract
- root catalog contract with manifest as authority
- append only migration ledger
- root locator interface
- `roots status`
- active root startup inspection plus derived catch up recording

Do not land yet:

- generic rollback framework
- destructive authoritative rewrites
- multi root physical store merge
- federation read cache compaction

## Relationship To Current Runtime

The current repo already has the right seed behavior for derived migration:

- root scoped storage resolution in [src/config/paths/xdg_root.rs](/home/jerkytreats/meld/src/config/paths/xdg_root.rs)
- active root startup in [src/cli/route.rs](/home/jerkytreats/meld/src/cli/route.rs)
- idempotent graph catch up in [src/world_state/graph/runtime.rs](/home/jerkytreats/meld/src/world_state/graph/runtime.rs)
- traversal store creation with additive tree open in [src/world_state/graph/store.rs](/home/jerkytreats/meld/src/world_state/graph/store.rs)

So the architectural gap is not a missing generic migration engine.

The real gaps are:

- stable root identity above path derived storage
- durable migration bookkeeping
- strict discovery
- operator visible status for dormant roots

## Acceptance Criteria

This architecture is successful when all of these are true:

- one root can be inspected without mutating business data
- one root can replay derived graph state without a fresh scan
- one failed step leaves source truth readable
- one resumed plan records a coherent ledger history
- a moved root can be reconciled without silently forking identity
- active roots stay healthy through normal startup
- dormant roots become visible through explicit status and discovery flows
