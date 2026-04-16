# Root Federation Runtime

Date: 2026-04-15
Status: active
Scope: runtime discovery, safe migration, and user trigger flow for pre existing workspace roots that need to participate in one graph

## Thesis

This document is the `workspace_fs` first slice under the broader branch substrate in [Branch Federation Substrate](branch_federation_substrate.md).

Workspace root remains the local operator anchor.

The unified graph does not replace roots.
It federates them.

That means runtime must own three things:

- discovery of durable roots
- safe migration of each root in place
- trigger flow that makes migration natural for users without coupling it to `meld init`

The migration goal is not to rebuild old roots from scratch.
The goal is to make old roots legible to current graph runtime while keeping source truth intact.

## Core Position

The durable unit is one workspace root.
Each root owns one local data home with:

- one local spine
- one local graph projection store
- one local frame store
- one local artifact store
- one local workspace history

The unified graph is the semantic surface above those roots.

So migration must be root scoped.
It is never scan scoped.
It is never `init` scoped.

## Runtime Ownership

Migration belongs to runtime because:

- root stores are opened during normal startup
- graph reducers already run during normal startup and command completion
- store evolution is a data lifecycle concern, not a config bootstrap concern
- `meld init` owns user config assets, not workspace data evolution

## Required Runtime Concepts

### Root Manifest

Each root should have one durable manifest stored in its local data home.

It should declare:

- `root_id`
- `workspace_path`
- `root_state_version`
- `graph_state_version`
- `last_migrated_version`
- `last_reduced_seq`
- `discovered_at`
- `last_seen_at`

This manifest is the local truth for one root.

### Root Catalog

There should be one global catalog of known roots.

It should declare:

- known `root_id` values
- canonical workspace paths
- local data home paths
- migration status
- attachment status
- last migration attempt
- last successful migration

This catalog is the runtime registry for federation.

### Migration Runtime

This is a runtime service, not a one off script.

It owns:

- discovery
- inspection
- migration planning
- execution of safe additive migrations
- durable migration status recording

### Federation Runtime

This is the later runtime that reads many roots as one graph surface.

It is not required to merge all roots into one physical store on day one.
The first requirement is simply that runtime knows which roots exist and can migrate them into graph readable shape.

## Discovery

Discovery should happen in two modes.

### Passive Discovery

Passive discovery happens on normal startup for the current workspace root.

Runtime should:

- resolve the canonical workspace path
- resolve the local data home
- ensure a root manifest exists
- ensure the root is registered in the root catalog
- update `last_seen_at`

This makes active roots self registering.

### Active Discovery

Active discovery scans the global Meld data home for candidate roots.

Candidate signals include:

- local `store`
- local `frames`
- local `artifacts`
- local head index persistence
- local workflow state
- legacy ignore list or publish state

For each candidate:

- recover the implied workspace path from XDG root layout
- validate that the candidate looks like a real root
- assign or recover `root_id`
- write a root manifest if missing
- add the root to the root catalog

Discovery must not mutate business data beyond manifest and catalog registration.

## Migration

Migration should be additive, root local, and retryable.

### Migration Phases Per Root

1. Inspect

- open the root store
- inspect available trees
- inspect manifest presence
- inspect graph runtime metadata
- inspect legacy persistence such as head index
- derive the current root state version

2. Register

- ensure root manifest
- ensure root catalog entry
- persist canonical workspace path and local data home

3. Normalize

- create missing additive trees
- create missing runtime metadata
- add missing version markers
- preserve old trees and source artifacts

4. Catch Up

- run graph reducer catch up for the root
- persist `last_reduced_seq`
- verify restart safe idempotent replay

5. Finalize

- persist `last_migrated_version`
- mark migration status successful
- record migration time

### Safety Rules

Migration must obey these rules:

- never require `meld init`
- never require a new scan before migration
- never rewrite frames or artifacts for graph migration
- never delete old trees during first migration
- never advance version markers before successful completion
- always allow retry after failure
- always leave source truth readable even if graph migration fails

### Failure Model

Failure in graph migration should leave the system in one of two safe states:

- root is readable but graph catch up is incomplete
- root is readable and graph catch up can resume from last reduced `seq`

Failure must never make stored frames, artifacts, or node records unusable.

## Trigger Flow

### Automatic Trigger For Active Root

On `RunContext` startup:

- discover current root
- ensure manifest
- ensure catalog entry
- run lightweight additive migration if needed
- run graph catch up

On command completion:

- run graph catch up
- update `last_seen_at`

This keeps the active root healthy with no special user action.

### Explicit Trigger For Dormant Roots

Dormant roots need an operator visible path.

Recommended commands:

- `meld roots discover`
  scan XDG data and register candidate roots
- `meld roots status`
  show known roots, versions, and migration state
- `meld roots migrate`
  migrate all registered roots safely
- `meld roots attach <path>`
  explicitly register one root that discovery missed

This keeps migration visible and auditable.

## User Flow

### Normal User Flow

For a root the user is actively working in:

- user runs any normal command
- runtime discovers the root
- runtime migrates additive store state if needed
- runtime catches graph up
- user continues work

No `init` step is required.

### Bulk Upgrade Flow

For old roots that are not currently active:

1. user runs `meld roots discover`
2. runtime registers dormant roots
3. user runs `meld roots status`
4. user runs `meld roots migrate`
5. runtime migrates each root and records status

### Recovery Flow

If migration fails for one root:

- root remains registered
- root remains readable
- runtime records failure details
- user can retry with `meld roots migrate`

## Versioning

Versioning should be explicit and local.

Minimum version markers:

- root storage version
- graph projection version
- migration runtime version

The migration runtime should use these to decide:

- no action needed
- additive migration needed
- replay only needed
- operator intervention needed

## Relationship To Unified Graph

The unified graph should not dissolve roots into one anonymous store.

Instead:

- each root remains an operator anchor
- each root gets a stable `root_id`
- each root publishes graph readable state locally
- federation reads those roots as one connected semantic surface

This preserves local operational clarity while allowing cross root meaning.

## Acceptance Criteria

This runtime design is successful when all of these are true:

- a pre existing root can be discovered without a fresh scan
- a discovered root can be registered without mutating business data
- migration can add graph readable runtime state without rewriting source artifacts
- graph catch up can resume after interruption
- active roots self register during normal runtime startup
- dormant roots can be discovered and migrated through explicit user flow
- root local operator behavior remains unchanged after migration

## Next Implementation Slice

The next implementation slice should land:

- root manifest contract
- root catalog contract
- discovery runtime
- migration status reporting
- one explicit `roots status` command

After that:

- add `roots discover`
- add `roots migrate`
- add federation level graph read planning
