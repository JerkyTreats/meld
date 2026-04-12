# Knowledge Graph ECS Decision Memo

Date: 2026-04-12
Status: active
Scope: ECS as an internal substrate for `curation` and the canonical knowledge graph

## Summary

This memo evaluates Entity Component System as the internal runtime and storage substrate for the canonical knowledge graph.
The scope is narrow by design.
This is not a proposal to replace the repo with an ECS runtime.
It is a test of whether ECS improves the `curation` domain enough to justify the lift.

The recommendation is `adopt hybrid`.
Use ECS concepts inside `curation` for belief state, evidence attachment, provenance, supersession, and projection execution.
Do not use ECS as the public storage shape, cross-domain contract, or repo-wide execution model.
Keep canonical persisted facts graph-shaped and spine-shaped.
Keep `task` and `capability` as the deliberate execution substrate.

This recommendation is exploratory, but not speculative.
It preserves long-term headroom for a living knowledge graph while avoiding a cross-domain rewrite before the repo has the sequencing, temporal, and storage guarantees that a wider ECS push would require.

## Narrow Thesis

The question is not "should the repo become an ECS system".

The actual question is:

- should the canonical knowledge graph use ECS as its internal substrate for mutable current belief and projector execution
- while the durable semantic ledger remains the spine
- while persisted domain records remain graph-shaped and typed
- while `task` and `capability` remain the execution substrate for deliberate side effects

That narrow framing matters.
Most of the upside comes from sparse attachment, projection flexibility, and parallel reducer execution inside `curation`.
Most of the cost appears when ECS is treated as a universal runtime model.

## Current Baseline

The current repo already contains the beginnings of a world-model architecture, but it is not yet organized around a domain-agnostic entity world.

### Concrete anchors

- `src/types.rs`
  `NodeID` and `FrameID` are the current root identifiers
- `src/context/frame.rs`
  `Frame` and `Basis` define context as immutable append-only records attached to nodes or prior frames
- `src/heads.rs`
  `HeadIndex` is a current-state projection keyed by node and frame type
- `src/task/contracts.rs`
  task artifacts, artifact links, and invocation records define the durable task data plane
- `src/task/executor.rs`
  task progression is task-local, artifact-driven, and contract-shaped
- `src/capability/contracts.rs`
  capability contracts define scope, slots, effects, and execution class
- `src/telemetry/events.rs`
  `ProgressEvent` is the current envelope for durable ordered event publication
- `src/telemetry/routing/bus.rs`
  the active bus is still in-process and channel-based
- `src/workspace/watch/events.rs`
  watch mode already batches and lowers raw filesystem churn into manageable change batches

### What already behaves like ECS

- append-only facts exist across frames, task events, and telemetry events
- reducers and projections already exist, such as `HeadIndex`, task state reduction, and prior reduction described in the design docs
- observation is already moving toward diff-first publication through `workspace_scan_batch` and watch batching
- execution already allows parallel work, especially in control-owned orchestration and task release of ready instances
- domain-owned projections over durable history already exist as a core pattern

### What is not yet ECS-like

- identity is still rooted in filesystem `NodeID` rather than a domain-agnostic anchor
- context attachment is node-centric, with `Basis` still assuming nodes and frames as the primary anchors
- execution state is task-artifact centric rather than entity-state centric
- the event schema is still coarse for curation needs and does not yet carry the full temporal and provenance fields needed for belief revision
- the current bus and store choices do not yet provide the multi-process or high-rate semantics that would let ECS systems scale cleanly across the whole architecture

## Decision Questions

This memo answers five questions.

### 1. Does ECS materially improve the knowledge graph over a purpose-built temporal graph projection

Yes, but only inside `curation`.
ECS materially improves the live mutable side of the knowledge graph where sparse state, many cross-cutting annotations, and many projector passes coexist.
It does not materially improve the durable ledger by itself.
It also does not replace the need for graph-shaped records, replay rules, or strong temporal semantics.

### 2. Which curation concerns naturally fit ECS

These concerns fit well:

- identity continuity
- componentized evidence
- mutable current belief state
- provenance attachment
- supersession tracking
- calibration state
- projection caches tuned for planner and operator views

These fit because each concern can be attached sparsely to the same entity without forcing one wide schema or one canonical record per use case.

### 3. Which concerns do not fit ECS well

These should remain outside ECS or only touch it through adapters:

- task orchestration
- provider execution
- raw sensory transport
- human-facing context retrieval APIs
- canonical durable event publication

These concerns either need stronger temporal guarantees, clearer contracts, or simpler query surfaces than an ECS-first model naturally provides.

### 4. What should happen to `ContextFrame`

`ContextFrame` should not become the primary ECS component.
It should become a projection-specific artifact attached through `DomainObjectRef`.
Some of what a frame carries today should be split into components such as evidence, belief, provenance, and annotation.
The frame itself should remain a durable shaped artifact for operator view, LLM view, and compatibility.

### 5. Does ECS buy real parallel headroom in this repo

Only partially today.
ECS gives a better execution shape for curation systems than the current task and artifact model, but the repo still has practical limits:

- the current event bus is in-process
- storage is centered on `sled`, filesystem blobs, and projection indexes
- task execution is still local and contract-driven
- multi-process ordering is still unresolved in the spine design

So ECS buys local parallelism and cleaner curation decomposition now.
It does not yet buy full-device distributed scale by itself.

## Alternatives Compared

### Alternative A

Purpose-built temporal knowledge graph with reducer-owned records and indexes, with no ECS vocabulary.

#### Strengths

- smallest conceptual lift from current design
- easiest fit with current spine and projection language
- easy to reason about in docs and storage schemas
- lower query surprise for operator-facing graph views

#### Weaknesses

- can become a rigid record taxonomy as new annotation types appear
- risks many special-case tables and indexes for each new curation concern
- weak fit for sparse cross-cutting state such as calibration, reliability, and transient merge state
- tends to push projection logic into custom reducers with duplicated traversal work

#### Judgment

Viable, but likely too rigid once the graph moves beyond thesis and evidence into dynamic belief maintenance.

### Alternative B

ECS-backed curation core with graph and query projections layered on top.

#### Strengths

- best fit for sparse attached state
- clear separation between entity identity and attached knowledge
- strong substrate for many curation systems that operate over overlapping state
- good fit for incremental projector execution and projection-specific caches

#### Weaknesses

- high conceptual load if exposed directly as the public graph model
- harder debugging if canonical persisted facts are not shaped separately
- query ergonomics degrade if operator-facing views must traverse raw components
- encourages runtime unification pressure far outside `curation`

#### Judgment

Strong internal substrate, weak public boundary.
Good only if layered behind graph-shaped records and explicit domain contracts.

### Alternative C

Hybrid model where ECS is an internal change-tracking and system execution substrate while canonical persisted facts remain graph-shaped records.

#### Strengths

- keeps the curation upside of sparse state and system execution
- preserves graph-shaped persistence, spine replay, and operator-friendly query surfaces
- reduces migration pressure on `task`, `capability`, and current context APIs
- lets the team defer crate and runtime selection until the curation model is proven

#### Weaknesses

- two conceptual layers must be maintained with care
- risk of drift between ECS state and persisted graph records
- requires explicit publication rules and replay discipline

#### Judgment

Best trade for this repo.
It buys the interesting part of ECS without forcing the rest of the architecture to pretend every domain is the same.

## Migration Inventory

The real cost is not "adding ECS".
The real cost is the set of lifts needed so that `curation` can stop inheriting assumptions from workspace context and task execution.

### Identity lift

Current state:

- identity roots in `NodeID`
- context basis still assumes node and frame anchors
- cross-domain identity is only sketched in `DomainObjectRef`

Lift required:

- define `DomainObjectRef` as the cross-domain public anchor
- introduce a curation-internal `EntityId`
- maintain identity continuity between durable references and live curation entities

Risk:

- high, because identity leaks through context, workspace, and planner assumptions today

### Context lift

Current state:

- frames attach to nodes or prior frames
- context query APIs assume node-centric retrieval

Lift required:

- split frame content into more precise concerns
- allow curation records to attach to heterogeneous anchors
- preserve frame projection compatibility for current consumers

Risk:

- high, because the current context engine is deeply shaped by node attachment

### Event lift

Current state:

- `ProgressEvent` has `ts`, `session`, `seq`, `event_type`, and `data`
- curation needs richer semantics for observed time, commit order, provenance, and supersession

Lift required:

- move toward the multi-domain envelope with `domain_id` and `content_hash`
- add curation-ready temporal fields and provenance references
- define promotion rules from fast sensory lanes into canonical curation facts

Risk:

- medium to high, because the spine design already anticipates the lift but the implementation is not there yet

### Storage lift

Current state:

- `HeadIndex` is a targeted projection over frames
- task repos and frame stores are domain-specific

Lift required:

- add belief, evidence, provenance, supersession, and calibration projections
- preserve replay from the spine into those projections
- decide what is transient ECS state versus what is persisted graph record

Risk:

- medium, because this is largely additive if the hybrid model is respected

### Execution lift

Current state:

- deliberate work flows through `task`, `capability`, and control orchestration
- continuous curation workers do not yet have a natural durable home

Lift required:

- define curation systems that subscribe, merge, supersede, calibrate, and project
- allow always-on reducer execution without forcing them into task semantics

Risk:

- medium, because `curation` is still mostly design today

### Compatibility surface

These can stay untouched in a first landing:

- `task` contracts
- `capability` contracts
- provider execution
- control program structure
- current frame query APIs

These need compatibility wrappers or dual publication during migration:

- context attachment
- event envelope
- planner-facing world-model reads

## Capability Analysis

### What ECS buys

#### Componentized belief state

Why it matters:

- `curation` needs to attach confidence, calibration, provenance, and supersession to the same conceptual thing without exploding one wide schema

What ECS buys:

- sparse attachment of those concerns
- clean separation between identity and attached belief records

Architecture need served:

- belief revision
- calibration
- planner-facing current state

#### Efficient sparse attachment of context

Why it matters:

- the graph must attach facts to workspace nodes, task outcomes, future domain objects, and eventually tracks or continuity objects

What ECS buys:

- one live entity can carry only the components relevant to that anchor

Architecture need served:

- cross-domain anchoring
- generalized context attachment

#### System parallelism for curation

Why it matters:

- curation will need ingest, merge, conflict handling, decay, calibration, and projection work that naturally cut across the same world state

What ECS buys:

- system-oriented execution over filtered state
- clearer decomposition than task artifacts for always-on reducers

Architecture need served:

- belief revision
- temporal transistor lowering downstream of sensory promotion

#### Separation of identity from attached knowledge

Why it matters:

- identity continuity, evidence history, and current belief are related but not identical

What ECS buys:

- identity components can remain stable while evidence and belief components churn

Architecture need served:

- continuity
- supersession
- planner-facing current state

#### Better projection-specific views

Why it matters:

- operator view, planner view, and calibration view want different slices of the same world model

What ECS buys:

- multiple projectors can read the same underlying components without forcing one canonical query shape

Architecture need served:

- planner-facing world state
- reflection and calibration

### What ECS does not buy

- sequencing correctness
- causal correctness
- temporal semantics
- good uncertainty math
- good planner behavior
- low replay cost by default
- readable operator queries by default

These still require explicit design in the spine, temporal schema, merge model, and projection contracts.

## Failure Modes

- runaway abstraction where every curation concern becomes a component with no durable shape discipline
- over-generalized components that hide domain meaning instead of clarifying it
- write amplification from publishing too much transient ECS churn into the canonical ledger
- hard queries when operator and planner views must reconstruct meaning from raw component sets
- replay cost if curation systems require expensive reconstruction of transient internal state
- poor debugging if supersession and calibration logic live only in opaque runtime state
- accidental runtime unification where sensory, curation, and execution are forced onto one model for aesthetic consistency

These risks are real.
They are the strongest reason not to choose Alternative B as the public architecture.

## Candidate Future Interfaces

This phase defines candidate interfaces only.
No Rust API changes are proposed here.

### Cross-domain public contracts

These should be public:

- `DomainObjectRef`
  stable cross-domain anchor for workspace objects, task outcomes, knowledge records, and future tracked objects
- curation fact envelope fields for domain, content hash, observed time, commit sequence, provenance refs, and supersession refs
- planner-facing query contracts that return shaped belief views rather than raw components

### Curation-internal interfaces

These should remain internal to `curation` unless evidence later proves they need to escape:

- `EntityId`
  live curation identity for ECS runtime use
- component families for identity, evidence, belief, provenance, temporal window, calibration, and projection cache state
- system families for ingest, merge, supersede, calibrate, and project

### Candidate component families

- identity
- anchor binding
- evidence item
- belief state
- provenance lineage
- temporal validity window
- supersession marker
- source reliability
- calibration state
- projector cache

### Candidate system families

- ingest
- merge
- supersede
- calibrate
- project

The boundary rule is simple:
components and systems are internal curation machinery.
facts, anchors, and shaped views are public contracts.

## Scenario Scorecard

Score legend:

- 1 poor
- 2 weak
- 3 workable
- 4 strong
- 5 best fit

| Scenario | Current path | Alternative A | Alternative B | Alternative C |
|---|---:|---:|---:|---:|
| Replay belief state from spine history | 2 | 4 | 3 | 4 |
| Attach one claim to a workspace node and a task outcome | 2 | 3 | 5 | 5 |
| Supersede stale evidence without losing provenance | 2 | 3 | 5 | 5 |
| Calibrate a prediction after later outcome | 3 | 3 | 5 | 5 |
| Serve a planner-facing current belief view | 2 | 3 | 4 | 5 |

## Cost Matrix

Lift legend:

- low
- medium
- high

| Module or concern | Alternative A | Alternative B | Alternative C | Notes |
|---|---|---|---|---|
| `src/types.rs` identity assumptions | medium | high | high | `DomainObjectRef` pressure appears in every serious path |
| `src/context/frame.rs` and frame basis | medium | high | high | context attachment must generalize in both B and C |
| `src/heads.rs` and head projections | low | medium | medium | new curation projections can coexist |
| `src/task/contracts.rs` | low | medium | low | keep task durable model intact in C |
| `src/task/executor.rs` | low | medium | low | task execution should not absorb curation systems |
| `src/capability/contracts.rs` | low | low | low | can stay stable in first landing |
| `src/telemetry/events.rs` | medium | high | high | curation-ready envelope still needed |
| `src/telemetry/routing/bus.rs` | low | medium | medium | local limitation remains in all serious curation growth paths |
| `src/workspace/watch/events.rs` | low | low | low | already resembles lowering input |
| new `curation` storage and projector layer | medium | high | medium | core landing area |

## Capability Matrix

| Capability area | Immediate value | Deferred value | Speculative value |
|---|---|---|---|
| Sparse belief attachment | high | high | medium |
| Cross-domain anchoring | medium | high | high |
| Projection-specific views | medium | high | medium |
| Parallel curation systems | medium | high | high |
| Planner-facing world state | low | high | high |
| Full-device multi-process scale | low | medium | high |
| Raw sensory handling | low | low | medium |

## Recommendation

Adopt the hybrid model.

### Why

- it captures the real ECS upside inside `curation`
- it avoids forcing ECS onto `task`, `capability`, or the public query surface
- it matches the repo’s actual maturity, where the spine and cross-domain identity model are still evolving
- it keeps graph-shaped persistence and replay semantics first-class

### Staged landing path

#### Stage 1

Define the public anchor and fact contracts.

- finalize `DomainObjectRef`
- define curation fact records for thesis, evidence, belief, provenance, supersession, and calibration
- define temporal fields for observed time and commit sequence

#### Stage 2

Build a curation-internal ECS prototype behind a projection boundary.

- add `EntityId`
- add internal components for identity, evidence, belief, provenance, supersession, and calibration
- add internal systems for ingest, merge, supersede, calibrate, and project
- publish graph-shaped records and planner-facing views from that core

#### Stage 3

Run replay and projector evaluation.

- replay from spine history into the curation core
- test supersession, late evidence, and calibration updates
- confirm that operator and planner views do not need raw ECS knowledge

#### Stage 4

Decide whether the ECS substrate is earning its keep.

- keep it if it reduces schema sprawl and enables multiple curation projections cleanly
- remove it if it becomes opaque, expensive, or redundant with a simpler reducer-owned graph model

### Stop conditions

Stop or narrow the ECS path if any of these happen:

- the team starts pushing ECS vocabulary into every domain contract
- planner and operator views become harder to query than the graph records they replaced
- replay requires hidden transient state that cannot be recovered from the spine
- the curation core gains complexity without reducing schema sprawl or merge duplication
- the event spine and temporal model remain under-specified, making ECS state look richer than the durable truth actually is

## Final Position

The intuition is directionally correct.
`curation` wants something more like an entity world with sparse attached state and many systems operating over it.
But the repo does not need an ECS religion.

The right move is to let the knowledge graph become its own system.
Use ECS internally where it sharpens curation.
Keep the spine, graph records, and shaped views as the durable truth seen by the rest of the architecture.
