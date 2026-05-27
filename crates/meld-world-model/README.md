# meld-world-model

`meld-world-model` is the durable world model crate for Meld. It materializes graph and claim views from the event spine, maintains belief state over public graph evidence, and projects planner-facing world state into `meld-lang`.

The crate owns world model knowledge. It does not own goal curation, method selection, task planning, capability choice, dispatch, retry, repair, or outcome publication.

## Domains

### Claims

Claims are settled facts about domain objects. When execution completes, fails, or produces an artifact, the world model records a `ClaimRecord` for the affected subject.

Core claim records:

- `ClaimRecord`
- `ClaimKind`
- `SettlementStatus`
- `EvidenceRecord`
- `ProvenanceRecord`

Claim state is append oriented. New claims supersede older claims over the same subject while preserving replacement history.

### Graph

Graph anchors are perspective-scoped pointers from a subject to a target. They form a traversable knowledge graph over domain objects.

Core graph records:

- `AnchorSelectionRecord`
- `PerspectiveKey`
- `TraversalFactRecord`
- `AnchorProvenanceRecord`
- `GraphWalkSpec`
- `GraphWalkResult`

Canonical perspective kinds include:

| Perspective kind | Perspective id | Meaning |
|---|---|---|
| `frame_type` | `analysis`, `summary`, or another frame type | Current frame head for a node |
| `snapshot` | `current` | Current snapshot for a source |
| `artifact_type` | Artifact type id | Current artifact for a task run |

Anchors supersede per subject and perspective. History remains queryable, while current indexes expose the latest active anchor.

### Belief

Belief consumes public graph state and runtime configuration, then produces planner-safe `BeliefView` records.

Core belief records and APIs:

- `BeliefKey`
- `BeliefView`
- `BeliefRevision`
- `BeliefStore`
- `BeliefQuery`
- `BeliefRuntime`
- `BeliefEvidenceNormalizer`
- `BeliefConfigLoader`
- `PlannerProjectionSummary`

Belief family semantics stay in configuration data. Core Rust code must not introduce family-specific modules, variants, or identifiers. The first configured slice uses runtime ids such as `docs_freshness`, but that value remains data rather than a Rust subsystem.

### Planner

Planner projection converts public belief and graph reads into a ground `meld_lang::WorldState`.

Core planner records and APIs:

- `PlannerProjectionContext`
- `PlannerFieldProjectionConfig`
- `PlannerProjectionInput`
- `PlannerGraphScope`
- `PlannerProjectionOutput`
- `PlannerSourceRef`
- `PlannerHydrationRefs`
- `PlannerProjectionWarning`
- `PlannerProjectionError`
- `PlannerQuery`
- `project_world_state`
- `PLANNER_PROJECTION_VERSION`

The first planner slice projects:

- belief confidence as `Proposition::Holds`
- stale state as a generic derived dimension
- observation-needed state as a generic derived dimension
- graph scope as `Proposition::Accessible`

Projection is deterministic, read only, and ground. Missing belief omits belief propositions so `meld-lang` can return `Indeterminate` for missing `Holds` dimensions.

## Storage

The crate uses embedded [sled](https://github.com/spacejam/sled) stores.

| Store | Domain | Main indexes |
|---|---|---|
| `WorldStateStore` | Claims | active claims by subject, claim history by subject, source fact, sequence |
| `TraversalStore` | Graph | current anchor by ref, anchor history, subject and perspective, relation indexes |
| `BeliefStore` | Belief | current views, revisions, evidence, assignments, dirty keys, observation opportunities |

Stores are append oriented where possible. Supersession and current-head movement preserve enough history for replay and explanation.

## Query API

### WorldStateQuery

`WorldStateQuery` is a read facade over `WorldStateStore`.

```rust
query.current_claims_for_object(subject);
query.claim_history_for_object(subject);
query.provenance_for_claim(claim_id);
query.supersession_chain_for_claim(claim_id);
```

### TraversalQuery

`TraversalQuery` is a read facade over `TraversalStore`.

```rust
query.current_anchor(anchor_ref);
query.current_anchors_for_subject(subject);
query.anchor_history(anchor_ref);
query.current_frame_head(node, frame_type);
query.current_frame_heads_for_node(node);
query.current_snapshot_for_source(source);
query.current_artifact_for_task_run(task_run, type_id);
query.neighbors(object, direction, relation_types, current_only);
query.walk(start, spec);
query.provenance_for_anchor(anchor_id);
```

### BeliefQuery

`BeliefQuery` is a planner-safe read facade over `BeliefStore`.

```rust
query.current_view(key);
query.current_views_for_subject(subject, perspective);
query.revision_history(key);
query.evidence_by_revision(revision_id);
query.provenance_by_revision(revision_id);
query.open_observation_opportunities(subject, perspective);
query.dirty_keys();
query.dirty_key_states();
query.rebuild_current_view_from_revision(key);
```

### PlannerQuery

`PlannerQuery` composes `BeliefQuery` and `TraversalQuery` into one read-only projection route.

```rust
let planner = PlannerQuery::new(belief_query, traversal_query);
let output = planner.project_current_world_state(
    &subject,
    "docs_freshness",
    None,
    None,
)?;
```

The route defaults to the `default/default` perspective and the `main` branch scope when the caller does not provide explicit values.

### WorldModelQueries

`WorldModelQueries` is an `Arc` friendly handle that exposes claim and graph query surfaces to long-lived runtime components and the API layer.

## Reduction And Runtime

`WorldStateReducer` handles execution events and writes claim records.

`TraversalReducer` handles domain events that imply structural relationships and writes anchor records.

`GraphRuntime` drives traversal replay by calling `catch_up` against the event spine.

`BeliefRuntime` assesses configured belief keys from graph and promoted evidence, then writes revisions, views, dirty-key state, and observation opportunities.

Planner projection has no reducer. It is a read-only projection over current graph and belief views.

## Position In The System

```text
meld-events
    -> meld-world-model
        -> claims
        -> graph
        -> belief
        -> planner projection
    -> meld-lang
        -> execution-readable WorldState
```

Downstream consumers include context, workspace, branch queries, API routes, agent curation, and execution evaluation. Execution planning remains outside this crate.

## Dependencies

This crate depends on:

- `meld-events` for event contracts, `DomainObjectRef`, and storage errors
- `meld-lang` for `WorldState`, `Proposition`, `Condition`, and `Term`
- `sled` for embedded persistence
- `serde` and `serde_json` for durable contracts and deterministic projection ordering
- `parking_lot` for shared runtime handles

## Layout Rules

Use domain-first modules.

Current domain entries:

- `src/world_state.rs`
- `src/world_state/*.rs`
- `src/world_state/graph.rs`
- `src/world_state/graph/*.rs`
- `src/belief.rs`
- `src/belief/*.rs`
- `src/planner.rs`
- `src/planner/*.rs`

Do not add `mod.rs`.

Adapters should remain thin. Public query facades should hide store layout and lower-layer internals.

## Harness And Quality Bar

The crate is expected to keep characterization, boundary, property, fuzz, and replay coverage for every world model domain.

### Required Test Families

- serde round-trip tests for public contracts
- validation tests for invalid ids, invalid probabilities, invalid field suffixes, and malformed query specs
- source ref and hydration ref preservation tests
- deterministic replay and reopen tests over persisted sled stores
- read facade tests that avoid direct lower-layer internals where public APIs exist
- source scans that reject `mod.rs` and family-specific Rust identifiers
- boundary scans that reject execution planning imports or values inside planner code
- grounding tests for every projected `meld_lang::WorldState`
- missing input tests that prove absence does not fabricate support
- property tests for probabilities, dimension ids, duplicate refs, stale state, observation state, and invalid field ids
- fuzz targets for serialized public contracts that accept untrusted input

### Planner-Specific Quality Bar

Planner projection must satisfy:

- consumes public graph and belief reads only
- creates no `Goal`, `Method`, `Composition`, `Operator`, `Effect`, task, or capability value
- emits only ground propositions
- emits no hardcoded runtime belief family Rust identifiers
- derives freshness and observation dimensions from runtime dimension ids and projection config
- preserves belief revision ids, evidence ids, source fact ids, and graph anchor ids
- returns deterministic proposition order, source ref order, hydration ref order, and warning order
- treats missing belief as omitted `Holds` propositions plus `MissingBelief`
- lets `meld-lang` evaluation return `Indeterminate` for missing belief dimensions
- emits `Accessible` only from available graph scope
- remains read only over graph and belief stores

## Verification

Run the focused crate checks before changing shared behavior:

```sh
cargo check -p meld-world-model
cargo test -p meld-world-model
cargo clippy -p meld-world-model -- -D warnings
```

Run the workspace gates before merging broad or public API changes:

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Run planner-focused checks after planner changes:

```sh
cargo test -p meld-world-model planner_module_boundary
cargo test -p meld-world-model planner_contracts
cargo test -p meld-world-model planner_belief_projection
cargo test -p meld-world-model planner_graph_projection
cargo test -p meld-world-model planner_world_state
cargo test -p meld-world-model planner_query
cargo test -p meld-world-model planner_typed_loop_handoff
```

Run fuzz checks for planner contract input with nightly:

```sh
cd crates/meld-world-model
cargo +nightly fuzz run fuzz_planner_projection_contract
```

Use a bounded smoke run when validating toolchain and target wiring:

```sh
cd crates/meld-world-model
cargo +nightly fuzz run fuzz_planner_projection_contract -- -runs=1
```

Run normal cargo syntax checks for the fuzz crate when nightly fuzzing is not available:

```sh
cargo check --manifest-path crates/meld-world-model/fuzz/Cargo.toml --bin fuzz_planner_projection_contract
```

Targeted mutation testing for planner projection should be attempted during handoff. If active filters report zero mutants, record that result with the command used rather than treating it as behavioral evidence.
