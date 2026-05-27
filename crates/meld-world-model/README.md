# meld-world-model

The world model is Meld's durable knowledge layer. It watches the event spine, builds up structured knowledge about domain objects, and projects that knowledge into a form the planner can reason over.

This crate owns *what Meld knows*. It does not own goals, task planning, dispatch, or execution — those live elsewhere and consume the world model as a read-only input.

## How knowledge flows

```text
events arrive
    -> claims settle facts about objects
    -> graph anchors link objects together
    -> belief assesses confidence from graph evidence
    -> planner projection grounds belief into meld-lang WorldState
```

Each layer builds on the one below it. Claims record what happened. The graph captures how things relate. Belief synthesizes those into assessable views. The planner projection flattens belief into propositions that `meld-lang` can evaluate.

## Domains

### Claims

When execution completes, fails, or produces an artifact, the world model records a **claim** — a settled fact about a domain object. Claims are append-only: new claims supersede older ones for the same subject, but the full history is preserved for replay and explanation.

### Graph

**Graph anchors** are perspective-scoped pointers from a subject to a target. They form a traversable knowledge graph over domain objects. A perspective might be a frame type (`analysis`, `summary`), a snapshot (`current`), or an artifact type — each scoping which anchor is "current" for a given subject.

Like claims, anchors supersede per subject and perspective while keeping history intact.

### Belief

Belief consumes graph state and runtime configuration, then produces **belief views** — planner-safe assessments of confidence, freshness, and observation opportunities. Belief families (like `docs_freshness`) are defined in configuration data, not in Rust code. The core engine is family-agnostic.

### Planner projection

The planner projection is a read-only, deterministic function that converts current belief and graph state into a ground `meld_lang::WorldState`. It emits propositions like:

- `Proposition::Holds` for belief confidence
- `Proposition::Accessible` for graph scope
- Derived dimensions for staleness and observation-needed state

Missing belief is represented as absent propositions, so `meld-lang` evaluation returns `Indeterminate` rather than fabricating support.

## Storage

The crate uses embedded [sled](https://github.com/spacejam/sled) stores, one per domain:

| Store | Holds | Key indexes |
|---|---|---|
| `WorldStateStore` | Claims | by subject (active + history), by source, by sequence |
| `TraversalStore` | Graph anchors | by ref (active + history), by subject + perspective, by relation |
| `BeliefStore` | Belief views | current views, revisions, evidence, dirty keys, observation opportunities |

All stores are append-oriented. Supersession and current-head tracking preserve enough history for replay and explanation.

## Query API

Each domain exposes a read facade. You rarely need to touch the stores directly.

**Claims** — `WorldStateQuery`

```rust
query.current_claims_for_object(subject);
query.claim_history_for_object(subject);
query.provenance_for_claim(claim_id);
query.supersession_chain_for_claim(claim_id);
```

**Graph** — `TraversalQuery`

```rust
query.current_anchor(anchor_ref);
query.current_frame_head(node, frame_type);
query.current_snapshot_for_source(source);
query.neighbors(object, direction, relation_types, current_only);
query.walk(start, spec);
```

**Belief** — `BeliefQuery`

```rust
query.current_view(key);
query.current_views_for_subject(subject, perspective);
query.revision_history(key);
query.dirty_keys();
query.open_observation_opportunities(subject, perspective);
```

**Planner** — `PlannerQuery` composes belief and graph queries into one projection route:

```rust
let planner = PlannerQuery::new(belief_query, traversal_query);
let output = planner.project_current_world_state(
    &subject,
    "docs_freshness",
    None, // defaults to default/default perspective
    None, // defaults to main branch scope
)?;
```

`WorldModelQueries` bundles claim and graph query surfaces behind an `Arc` handle for use in long-lived runtime components and the API layer.

## Reducers and runtimes

- `WorldStateReducer` — processes execution events into claim records
- `TraversalReducer` — processes domain events into anchor records
- `GraphRuntime` — drives traversal replay via `catch_up` against the event spine
- `BeliefRuntime` — assesses belief keys from graph evidence, writes revisions and dirty-key state

Planner projection has no reducer. It reads current state on demand.

## Dependencies

- `meld-events` — event contracts, `DomainObjectRef`, storage errors
- `meld-lang` — `WorldState`, `Proposition`, `Condition`, `Term`
- `sled` — embedded persistence
- `serde` / `serde_json` — durable contracts
- `parking_lot` — shared runtime handles

## Working on this crate

Run focused checks before changing shared behavior:

```sh
cargo check -p meld-world-model
cargo test -p meld-world-model
cargo clippy -p meld-world-model -- -D warnings
```

Run workspace gates before merging public API changes:

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Run planner-specific tests after planner changes:

```sh
cargo test -p meld-world-model planner_module_boundary
cargo test -p meld-world-model planner_contracts
cargo test -p meld-world-model planner_belief_projection
cargo test -p meld-world-model planner_graph_projection
cargo test -p meld-world-model planner_world_state
cargo test -p meld-world-model planner_query
cargo test -p meld-world-model planner_typed_loop_handoff
```

Fuzz the planner projection contract (requires nightly):

```sh
cd crates/meld-world-model
cargo +nightly fuzz run fuzz_planner_projection_contract
# Smoke run: append -- -runs=1
```

## Layout

Modules follow domain-first organization: `src/world_state.rs`, `src/belief.rs`, `src/planner.rs` and their sub-modules. No `mod.rs` files. Query facades hide store layout from callers.
