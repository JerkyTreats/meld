# Spine Graph Completion Plan

Date: 2026-04-18
Status: active
Scope: completion contract, domain matrix, layer matrix, and delivery plan for fully landing the first spine plus graph traversal feature

## Intent

This document turns the current branch review into an explicit completion target.

The branch has already grown beyond a narrow graph only implementation.
It now spans the shared event spine, graph materialization, branch federation, and the first publisher domains.

That broader scope can be valid.
It must be made explicit.

## Branch Goal

Fully land the first real event spine plus graph traversal substrate for the application, with branch aware federation, so that promoted cross domain facts can be appended once, replayed deterministically, materialized into current anchors and relations, and queried coherently within one branch or across many branches.

## Definition Of Done

This branch is done only when all of these are true.

- the spine is a real shared substrate with one canonical envelope, runtime wide sequence, replay safety, and legacy read compatibility
- required publisher domains emit canonical facts into the spine through explicit object refs and relations
- `world_state/graph` can rebuild current anchors, lineage, provenance, and graph walk from replayable facts and indexed materialization
- branch federation preserves branch provenance and branch presence rather than flattening many branch local stores into one ambiguous read result
- at least one production consumer outside branch tooling uses traversal data deliberately
- verification covers spine durability, graph replay parity, workspace publication, branch federation correctness, and consumer level integration
- scope limits are explicit, especially belief, curation, and sensory work remaining out of scope

## Scope In

- canonical event spine contracts and storage semantics
- `workspace` publication for promoted structural facts
- `context`, `control`, `task`, and `workflow` publication into the spine
- `world_state/graph` materialization, query, lineage, provenance, and walk
- branch aware federation over many traversal stores
- production runtime catch up and one meaningful consumer integration path

## Scope Out

- `world_state/belief`
- curation
- sensory raw lanes
- distributed sequencing
- generic graph query language
- non filesystem branch kinds as first class runtime implementations

## Current Branch Delta By Domain

The committed delta from `origin/main...HEAD` touches these domains.

| Domain | Changed files | Branch role |
| --- | ---: | --- |
| `context` | 29 | major publisher into spine and graph |
| `task` | 25 | major execution publisher and runtime refactor |
| `workflow` | 19 | major execution publisher and runtime refactor |
| `world_state` | 17 | graph materialization and legacy claim compatibility |
| `workspace` | 16 | workspace fact publication and scan integration |
| `telemetry` | 13 | downstream compatibility refactor |
| `cli` | 11 | runtime wiring and graph catch up |
| `provider` | 11 | supporting runtime changes |
| `branches` | 10 | federation runtime and query layer |
| `events` | 8 | canonical spine substrate |
| `session` | 6 | session lifecycle split from spine concerns |
| `control` | 5 | execution outcome publisher |
| `capability` | 5 | supporting execution changes |
| `store` | 3 | compatibility support |
| supporting domains | remaining | supportive refactor and wiring |

## Domain Matrix

### Core Domains

| Domain | Needed integration | Current state | Completion gate |
| --- | --- | --- | --- |
| `events` | canonical envelope, runtime wide sequence, replay, compatibility | high | spine history is durable, append only in practice, and not undermined by session cleanup semantics |
| `session` | lifecycle only, never the authority for canonical facts | medium | session pruning no longer deletes or weakens canonical spine history |
| `workspace` | publish promoted structural facts from scan and watch | medium | scan and watch both publish canonical `workspace_fs` facts at promoted structural boundaries |
| `context` | publish frame and head facts with explicit refs and relations | high | publication coverage is complete and parity stays green |
| `control` | publish execution outcome facts with graph usable refs | high | control outcomes remain replayable and graph readable |
| `task` | publish task run and artifact facts with target and artifact refs | high | task events remain canonical and graph usable across live and replay |
| `workflow` | publish workflow turn facts with node, frame, and plan linkage | high | workflow turn publication remains canonical and graph usable |
| `world_state/graph` | materialize current anchors, lineage, provenance, and walk | medium | derived graph records become explicit durable facts and queries stay index backed |
| `branches` | branch identity, branch scope resolution, federated read, branch presence | medium low | federated reads preserve branch provenance, branch presence, and fact identity across many branches |
| `cli` | runtime catch up and external query surface | high | startup and command paths keep graph current and expose correct branch scoped reads |

### Supporting Domains

| Domain | Needed integration | Current state | Completion gate |
| --- | --- | --- | --- |
| `telemetry` | downstream summaries and compatibility only | medium | telemetry does not own business fact meaning or correctness paths |
| `store` | keep local source truth intact during graph lift | medium | no graph path reaches into another domain internals for authority |
| `provider` | no direct graph ownership | low | only support execution publishers and consumers where needed |
| `agent` | no direct graph ownership | low | only support execution and context flows where needed |
| `capability` | no direct graph ownership | low | only support execution publication and one consumer path if required |

## Layer Matrix

Layers are useful planning strata.
They are not yet governance entities like domains.

| Layer | Owning domains | Purpose | Current state | Completion gate |
| --- | --- | --- | --- | --- |
| spine substrate | `events`, part of `session`, compatibility in `telemetry` | one temporal contract for promoted facts | mostly real | durable append, replay, and compatibility are complete |
| publisher layer | `workspace`, `context`, `control`, `task`, `workflow` | publish graph usable facts into the spine | mostly real | all required first slice publishers are complete |
| graph materialization layer | `world_state/graph` | build current anchors, lineage, provenance, and walks | real but partial | derived graph facts are explicit and durable |
| federation layer | `branches` plus branch local traversal stores | branch scoped read across many stores | partial | branch presence and branch provenance are preserved |
| application consumer layer | `cli` today, later execution consumers | use traversal as application input rather than inspection only | minimal | one meaningful production consumer path exists |
| belief layer | future `world_state/belief` | confidence, contradiction, curation, settlement | not started here | explicitly out of scope for this branch |

## Current Gaps

### Gap 1

Spine durability is still weaker than the design target.

Current issue:

- session pruning can still delete event records through the current store lifecycle path

Why it blocks done:

- canonical shared history cannot depend on session retention policy

### Gap 2

Workspace publication is not complete.

Current issue:

- scan emits canonical workspace facts
- watch still emits mostly trace style events and does not publish promoted structural workspace facts

Why it blocks done:

- long lived watch driven work can move real workspace state without keeping the graph current

### Gap 3

`world_state/graph` does not yet durably publish its own explicit derived fact family.

Current issue:

- reducers build in memory `world_state.*` envelopes
- those envelopes are not appended as durable facts

Why it blocks done:

- the graph is materialized, but its own derived facts are not yet a first class replayable contract

### Gap 4

Branch federation is semantically incomplete.

Current issue:

- federated reads merge branch local traversal results without a first class branch presence model
- branch local fact identity can collide or lose provenance when merged too loosely

Why it blocks done:

- full branch federation is in scope for this branch
- federation cannot be called done until branch provenance survives merging

### Gap 5

Production consumers are still too thin.

Current issue:

- traversal is caught up in runtime
- branch tooling can inspect it
- production code outside branch tooling does not yet rely on traversal in a meaningful way

Why it blocks done:

- the layer is still more inspectable than operational

## Completion Plan

### Phase 0

Freeze the completion contract for this branch.

Tasks:

- adopt this branch goal and definition of done as the working completion contract
- mark belief and curation as explicitly not started in this feature
- mark full branch federation and matured spine as explicitly in scope

Exit gate:

- branch scope is stated consistently in design docs and review notes

### Phase 1

Harden the spine substrate.

Tasks:

- separate canonical history retention from session retention
- ensure session pruning and interruption handling cannot delete canonical spine facts
- keep legacy record compatibility and runtime wide sequence behavior
- tighten tests around append only spine semantics

Primary code areas:

- `src/events/store.rs`
- `src/session/storage.rs`
- `src/session/runtime.rs`
- `src/telemetry/sessions/service.rs`
- `tests/integration/event_spine.rs`

Exit gate:

- canonical spine history survives session cleanup policies

### Phase 2

Complete required publisher domain integration.

Tasks:

- keep existing `context`, `control`, `task`, and `workflow` publication coverage stable
- add canonical workspace publication for watch promoted outcomes
- ensure watch emits canonical structural facts at batch or snapshot boundaries
- preserve current local `NodeID` workflows while publishing `DomainObjectRef`

Primary code areas:

- `src/workspace/watch/runtime.rs`
- `src/workspace/commands.rs`
- `src/workspace/events.rs`
- `tests/integration/workspace_traversal.rs`
- new watch focused integration coverage

Exit gate:

- scan and watch both keep graph relevant workspace facts current

### Phase 3

Make graph derived facts explicit and durable.

Tasks:

- decide the first durable `world_state/graph` derived fact family contract
- append derived `world_state.anchor_selected`
- append derived `world_state.anchor_superseded`
- append any required provenance or lineage records through explicit durable paths
- ensure replay remains deterministic and idempotent

Primary code areas:

- `src/world_state/graph/events.rs`
- `src/world_state/graph/reducer.rs`
- `src/world_state/graph/runtime.rs`
- `src/world_state/graph/store.rs`
- `tests/integration/traversal_graph.rs`

Exit gate:

- graph derived facts are explicit durable contracts rather than reducer local byproducts

### Phase 4

Finish branch federation semantics.

Tasks:

- add first class branch presence semantics
- make federated merges preserve branch provenance
- remove branch local fact identity ambiguity in merged reads
- add multi branch verification that proves provenance survives merging
- keep one branch parity with local traversal queries

Primary code areas:

- `src/branches/contracts.rs`
- `src/branches/query.rs`
- `src/branches/runtime.rs`
- `src/world_state/graph/store.rs`
- any required branch scoped traversal support
- `tests/integration/branches_query.rs`

Exit gate:

- federated reads across many branches are semantically sound, not only convenient

### Phase 5

Land one real production consumer path.

Tasks:

- choose one meaningful production consumer outside branch tooling
- make that path read traversal deliberately before acting or deciding
- keep branch tooling as inspection surface, not the only surface

Recommended first candidates:

- execution planning gate for current frame head lookup
- execution path that resolves current artifact anchor
- operator status surface that reads traversal rather than raw local stores

Primary code areas:

- `src/control/`
- `src/task/`
- `src/workflow/`
- `src/cli/route.rs`
- new focused integration coverage

Exit gate:

- one production path depends on traversal projection in a way that replay parity can verify

### Phase 6

Verification and closeout.

Tasks:

- expand test coverage for spine durability
- expand test coverage for watch driven workspace publication
- expand test coverage for branch federation over many branches
- verify live and replay parity for at least one consumer path
- document what remains out of scope after branch closeout

Exit gate:

- feature verification covers the actual definition of done rather than only local happy paths

## Verification Matrix

| Area | Required proof |
| --- | --- |
| spine | runtime wide monotonic sequence, legacy compatibility, append safety after session cleanup |
| workspace | scan publication, watch promotion, stable source identity, snapshot selection correctness |
| graph materialization | replay parity, current anchor lookup, lineage, provenance, bounded walk |
| federation | one branch parity, many branch provenance preservation, unhealthy branch isolation |
| consumers | at least one live versus replay parity path using traversal deliberately |

## Exit Review Questions

Before this branch is called done, the review should answer yes to each of these.

- is canonical spine history durable independently of session retention
- do all required first slice publisher domains publish canonical facts
- does watch keep workspace facts current in the graph
- are graph derived facts explicit durable contracts
- does branch federation preserve branch provenance and branch presence
- is traversal consumed by production code outside branch tooling
- are belief and curation still clearly out of scope

## Read With

- [Graph](README.md)
- [Graph Implementation Plan](implementation_plan.md)
- [Workspace FS Graph Transition Requirements](workspace_fs_transition_requirements.md)
- [Branch Federation Substrate](branch_federation_substrate.md)
- [Spine Concern](../../spine/README.md)
