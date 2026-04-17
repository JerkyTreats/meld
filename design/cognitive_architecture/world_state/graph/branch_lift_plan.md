# Branch Lift Plan

Date: 2026-04-15
Status: active
Scope: full rename from `roots` to `branches` and the remaining implementation lift to federated workspace reads through the branch substrate

## Objective

Turn the current roots retrofit into the full branch substrate.

This plan starts from the current state:

- branch aware metadata exists inside `src/roots/`
- active workspace startup resolves an internal branch handle
- operator surfaces still say `roots`
- graph runtime is still single workspace and single store

The goal is:

- branch becomes the canonical runtime and code vocabulary
- `roots` remains only as a compatibility alias where needed
- workspace federation becomes a real read layer above many branch local graph stores

## Current State

The current repository now has:

- branch aware contract types in [contracts.rs](/home/jerkytreats/meld/src/roots/contracts.rs)
- branch resolution and branch handle seams in [locator.rs](/home/jerkytreats/meld/src/roots/locator.rs) and [runtime.rs](/home/jerkytreats/meld/src/roots/runtime.rs)
- active branch tracking in [route.rs](/home/jerkytreats/meld/src/cli/route.rs)
- branch neutral aliases behind the `roots` facade in [roots.rs](/home/jerkytreats/meld/src/roots.rs)

The remaining blockers are clear:

- the domain is still named `roots`
- CLI still exposes only `roots status`
- dormant branch workflows do not exist yet
- graph runtime still opens one local traversal store in [runtime.rs](/home/jerkytreats/meld/src/world_state/graph/runtime.rs)
- traversal query still assumes one store in [query.rs](/home/jerkytreats/meld/src/world_state/graph/query.rs)
- there is no federated branch query facade

## Workstreams

The full lift should be executed in two parallel design tracks with sequential implementation.

### Track 1

Canonical branch rename and compatibility isolation.

### Track 2

Federated workspace read runtime above branch local graph projections.

Track 1 must land before Track 2 becomes public API.
Track 2 can start internally before the final CLI rename is complete.

## Phase 5

Introduce the real `branches` domain while keeping `roots` as a facade.

Deliverables:

- add `src/branches.rs`
- add `src/branches/`
- move canonical implementations from `src/roots/` into `src/branches/`
- keep `src/roots.rs` as a compatibility facade that re exports branch contracts and tooling

Required rules:

- no behavior change
- keep root named file formats readable
- keep `meld roots status` working

Suggested file mapping:

- `src/branches/contracts.rs`
- `src/branches/locator.rs`
- `src/branches/manifest.rs`
- `src/branches/catalog.rs`
- `src/branches/ledger.rs`
- `src/branches/runtime.rs`
- `src/branches/query.rs`
- `src/branches/tooling.rs`
- `src/branches/kinds/workspace_fs.rs`

Exit criteria:

- internal imports prefer `crate::branches`
- `crate::roots` is a thin wrapper only
- parity tests prove no regression

## Phase 6

Flip canonical type names and public internal contracts to branch first naming.

Deliverables:

- canonical runtime type becomes `BranchRuntime`
- canonical resolved handle types become `ResolvedBranch` and `BranchHandle`
- canonical status row and output types become branch named
- graph and CLI route code stop importing root named types

Compatibility:

- keep root named type aliases during transition
- keep old JSON field names if needed for storage compatibility

Exit criteria:

- new code cannot add fresh root named primary contracts
- root named contracts exist only as aliases or compatibility wrappers

## Phase 7

Add branch operator flows beyond status.

Deliverables:

- `meld branches status`
- `meld branches discover`
- `meld branches migrate`
- `meld branches attach`

Compatibility:

- `meld roots status` remains as an alias
- later `meld roots discover` and `meld roots migrate` can also alias

Required implementation details:

- strict discovery filters for temp and test residue
- nested workspace handling
- ambiguous locator handling
- health and readiness state in the catalog

Exit criteria:

- dormant workspace branches can be inspected and migrated without opening them as the active workspace

## Phase 8

Add branch scoped spine identity and branch presence facts.

This is the semantic bridge from branch registration to real federation.

Deliverables:

- stable `spine_stream_id` for each branch in manifest and catalog
- branch presence records for `DomainObjectRef`
- reducers that can materialize presence facts from branch local graph facts

Why this matters:

- federation needs to know which branches contain an object
- future non-filesystem branch kinds need the same identity model

Required code seams:

- telemetry and spine event contracts
- graph reducer and store
- branch metadata contracts

Exit criteria:

- one semantic object can be present in many branches with branch local provenance preserved

## Phase 9

Build the federated branch graph runtime.

Deliverables:

- `src/branches/query.rs`
- `FederatedBranchQuery`
- branch scope selectors
- per branch runtime opening strategy
- merged traversal results

The runtime should:

- enumerate healthy branches from the catalog
- open branch local graph stores lazily
- fan out targeted traversal queries
- merge results with branch aware provenance

Required first operations:

- graph status by branch scope
- object lookup by branch scope
- neighbors by branch scope
- bounded walk by branch scope
- current anchors by branch scope
- provenance by branch scope

Exit criteria:

- federated single branch queries match current local graph behavior
- multi branch queries work without changing authoritative local stores

## Phase 10

Wire federation into explicit API and CLI surfaces.

Deliverables:

- branch scoped graph CLI entry points
- branch scope options in graph queries
- optional local service boundary above federated branch query

Recommended order:

1. internal branch query facade
2. CLI graph queries
3. optional HTTP read service

Exit criteria:

- external consumers no longer need to know about per workspace stores

## Phase 11

Promote `branches` to the primary user vocabulary.

Deliverables:

- help text and telemetry names prefer `branches`
- docs refer to `roots` only for compatibility and legacy storage
- `roots` commands become documented aliases

Exit criteria:

- all new docs and code use branch as the default term

## Code Seams That Still Need Lift

The remaining code lift is concentrated in these areas:

- top level module export in [lib.rs](/home/jerkytreats/meld/src/lib.rs)
- CLI parse and help in [parse.rs](/home/jerkytreats/meld/src/cli/parse.rs) and [help.rs](/home/jerkytreats/meld/src/cli/help.rs)
- binary routing in [meld.rs](/home/jerkytreats/meld/src/bin/meld.rs)
- compatibility facade and runtime in [roots.rs](/home/jerkytreats/meld/src/roots.rs) and [runtime.rs](/home/jerkytreats/meld/src/roots/runtime.rs)
- single store graph runtime in [runtime.rs](/home/jerkytreats/meld/src/world_state/graph/runtime.rs)
- single store traversal facade in [query.rs](/home/jerkytreats/meld/src/world_state/graph/query.rs)

## Verification Gates

Gate 1:

- branch rename parity
  `roots` and `branches` surfaces return identical results during the alias window

Gate 2:

- storage compatibility
  old root manifest and catalog files load under the branch runtime

Gate 3:

- federated query parity
  one branch federated queries match current one workspace graph queries

Gate 4:

- multi branch correctness
  two workspace branches produce deterministic merged traversal results

Gate 5:

- branch scope correctness
  active only, selected set, and all healthy branches all behave as declared

Gate 6:

- failure isolation
  one unhealthy branch cannot corrupt or block healthy branch reads

## Recommended Commit Sequence

1. add `branches` module and re export wrappers
2. move canonical runtime and contracts under `branches`
3. add `branches` CLI aliases
4. add dormant branch flows
5. add branch scoped spine identity and presence facts
6. add federated branch query facade
7. add graph CLI or service surface over federation
8. flip docs and help to branch first vocabulary

## Recommendation

Yes, the project can now investigate and execute the full lift.

The roots retrofit was the correct prerequisite.
The next decisive milestone is not another metadata slice.
It is Phase 5 and Phase 9:

- make `branches` the canonical module
- build the federated branch query runtime

Those two together turn the branch substrate from internal vocabulary into the actual workspace federation implementation.
