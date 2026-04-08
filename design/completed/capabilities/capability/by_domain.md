# Capabilities By Domain

Date: 2026-04-03  
Status: draft  
Scope: loose map from `src/<domain>/` to future domain-owned capability surfaces

See [Capability Model](README.md) for what a capability is and [Capability And Task Design](../README.md) for ownership rules.

## Purpose

Each substantive domain in `src/` may eventually expose a single module file, `capability.rs`, that names and wires **domain-owned** capability contracts. Functionality stays behind those contracts; `api` and `cli` remain thin adapters.

This document is **not** source of truth for Rust modules. It is a design sketch: section per domain, pseudo-module shape only. Names and boundaries will change as contracts land.

## Conventions in pseudo-modules

- `// capability: Name` marks one atomic surface a task compiler or control runtime could bind.
- Grouping is informational; real code may split across files under the domain.

---

## Runtime usefulness inventory

**Meaning:** how central each capability is to the **current runtime path** from trigger to generated frame and durable state updates.
This is an inventory aid, not a first-slice task-node recommendation.
Several high-score items below are still implementation helpers or compatibility surfaces rather than durable task-facing capability contracts.

**Out of scope for this score:** long-term plan compiler-only surfaces; weights assume today’s workflow-shaped runner remains the primary orchestration host.

### Scale

| Score | Label | Intent |
|------:|--------|--------|
| 5 | **Critical path** | Required for a normal workflow run or turn completion; omission breaks or hollows out execution |
| 4 | **Turn substrate** | Used every run or every turn through shared APIs; not always a distinct plan node but always nearby |
| 3 | **Session or queue** | Frequent for async or multi-step hosts; optional for minimal synchronous single-shot runs |
| 2 | **Preflight or enricher** | CI, validation, diagnostics, or optional telemetry sinks — useful around workflows, not the turn core |
| 1 | **Operator** | Human or tooling workflows; rare inside automated agent turn graphs |
| 0 | **Lifecycle admin** | Install-time or config editing; not part of steady workflow runtime |

### Catalog — all capabilities weighted

Sorted by score descending, then domain, then name.

| Score | Domain | Capability | Rationale |
|------:|--------|------------|-----------|
| 5 | context | ContextGenerate | Core LLM generation step for workflow turns |
| 5 | context | FrameWrite | Persists generated or merged frame outcome |
| 5 | heads | HeadIndexSave | Durability after head moves |
| 5 | heads | HeadResolveActive | Chooses current frame for continuation |
| 5 | heads | HeadSet | Commits new head after successful generation |
| 5 | metadata | FrameMetadataBuildGenerated | Built on every generation path in executor |
| 5 | prompt_context | PromptContextAssemble | Feeds rendered prompts and lineage inputs |
| 5 | provider | ProviderExecuteChat | Actual model call bound to workflow request |
| 5 | provider | ProviderResolveClient | Binds name and overrides before chat |
| 5 | store | StoreReadNode | Executor loads node record before turns |
| 5 | workflow | WorkflowExecuteTarget | Top-level entry that runs thread and gates |
| 5 | workflow | WorkflowStateRead | Turn loop reads thread and turn records |
| 5 | workflow | WorkflowStateWrite | Turn loop persists progress and gate outcomes |
| 4 | agent | AgentResolvePrompt | Turn prompts resolve through agent profile |
| 4 | config | ConfigLoadWorkspace | Registry and workflow profile loading on execute |
| 4 | context | ContextQueryGet | Direct reads feeding views and assembly |
| 4 | context | ContextQueryView | Composed node context for prompt construction |
| 4 | context | FrameRead | Loads basis frames and history for turns |
| 4 | heads | HeadIndexLoad | Restores head map when opening workspace |
| 4 | metadata | PromptLinkValidate | Gate and record path validates prompt links |
| 4 | prompt_context | PromptContextContractValidate | Guards assembled prompt context before use |
| 4 | store | StoreOpen | Attaches persistence before API use |
| 4 | store | StorePersistenceSync | Ensures coherency after writes on that path |
| 4 | telemetry | TelemetryEmitEvent | Turn and workflow events emitted during execution |
| 4 | workspace | WorkspaceResolveNodeId | Binds filesystem path to `NodeID` for targets |
| 3 | config | ConfigMerge | Produces effective config for loader consumers |
| 3 | config | ConfigResolvePaths | Roots for config and workflow discovery |
| 3 | context | ContextGenerateQueueDrain | Background queue processor, not every inline run |
| 3 | context | ContextGenerateQueueSubmit | Async handoff when generation is queued |
| 3 | heads | HeadTombstone | Cleanup or invalidation workflows |
| 3 | metadata | FrameMetadataDescribe | Supports validation and registry-driven behavior |
| 3 | telemetry | TelemetryRouteIngest | Internal path for events once emitted |
| 3 | telemetry | TelemetrySessionClose | Session teardown after a run |
| 3 | telemetry | TelemetrySessionOpen | Optional session wrapper around runs |
| 3 | workspace | WorkspaceScan | Refreshes tree state before many operations |
| 2 | agent | AgentStatus | Operator visibility, not turn logic |
| 2 | agent | AgentValidate | Preflight before scheduling runs |
| 2 | config | ConfigValidate | Preflight for bad config graphs |
| 2 | context | FrameTombstone | Maintenance or explicit invalidation flows |
| 2 | context | FrameCompact | Storage maintenance, not per-turn |
| 2 | context | FrameRestore | Recovery flows |
| 2 | provider | ProviderTest | Connectivity check before trusting a provider |
| 2 | provider | ProviderValidate | Profile sanity before bind |
| 2 | store | StoreWriteNode | Needed when workflows mutate node metadata |
| 2 | telemetry | TelemetrySinkOtel | Deployment wiring around the same events |
| 2 | telemetry | TelemetrySinkStore | Persists telemetry alongside business data |
| 2 | telemetry | TelemetrySinkTui | Human-facing sink for interactive runs |
| 2 | workflow | WorkflowRegistryValidate | CI or doctor-style validation |
| 2 | workspace | WorkspaceCiBatch | CI-shaped batch over workspace |
| 2 | workspace | WorkspaceValidate | Preflight invariants |
| 2 | workspace | WorkspaceStatus | Aggregated human status |
| 1 | agent | AgentList | CLI and tooling |
| 1 | agent | AgentShow | CLI and tooling |
| 1 | workspace | WorkspaceListDeleted | Reporting and cleanup tooling |
| 1 | workspace | WorkspaceWatchRun | Long-lived daemon, not a single plan step |
| 1 | workspace | WorkspaceDanger | Exceptional operator actions |
| 0 | agent | AgentCreate | Onboarding new agents |
| 0 | agent | AgentRemove | Removes agent config |
| 0 | agent | AgentSetRole | Role edits |
| 0 | init | InitBootstrapDefaults | First-time seed |
| 0 | init | InitIdempotentCheck | Dry-run bootstrap |
| 0 | provider | ProviderCreate | Adds provider profile |
| 0 | provider | ProviderRemove | Removes provider profile |
| 0 | provider | ProviderList | Listing profiles |
| 0 | provider | ProviderShow | Inspecting one profile |
| 0 | workflow | WorkflowInspect | Static inspection |
| 0 | workflow | WorkflowRegistryList | Listing registered workflows |

### Takeaways

- **Dense core:** scores **5** cluster around one vertical slice — execute workflow, read or write thread state, resolve node, load config, assemble prompt context, resolve and run provider, build metadata, write frame, update head, read store. That set is the first wave for capability contracts if plans must reproduce current workflow behavior.
- **Telemetry sinks** stay **2** while **emit** stays **4** — events are central; which sink is active is an environment concern.
- **Queue submit or drain** are **3** because inline workflow execution can bypass the queue; queued generation is a parallel hosting model.
- **Admin and list or show** capabilities remain **0** or **1**; they matter for operator experience and repository health, not for turn graphs.

### Inventory caveat

The weighted catalog above is useful for coverage analysis, but it is too fine-grained to serve directly as the first task-facing capability set.

Examples:

- `ProviderResolveClient` is a provider helper behind `ProviderExecuteChat`
- `FrameMetadataBuildGenerated` is part of context result shaping
- `PromptContextAssemble` is part of context preparation
- `HeadIndexSave` is a persistence detail behind higher state mutation
- `WorkflowExecuteTarget` is a compatibility trigger, not a durable task node

The first task-facing slice should publish durable atomic seams, not every helper step currently visible in code.

## First-Slice Task-Facing Capability Set

For the first task layer, the capability surface should be smaller and more intentional than the inventory above.

Recommended first-slice task-facing capability types:

| Domain | Capability | Why it is task-facing |
|--------|------------|-----------------------|
| merkle_traversal | MerkleTraversal | Structural ordering capability with reusable batch output |
| workspace | WorkspaceResolveNodeId | Resolves authored path or target into stable node scope |
| context | ContextGeneratePrepare | Produces provider-ready generation request from node scope, prompts, and policy |
| provider | ProviderExecuteChat | Executes ready provider work with batching and throttling hidden behind the domain |
| context | ContextGenerateFinalize | Validates provider result, builds metadata, persists frame effects, and emits reusable outputs |

Compatibility-only or hosting-only surfaces should remain outside that first task-facing set:

- `WorkflowExecuteTarget`
- `WorkflowStateRead`
- `WorkflowStateWrite`
- `ContextGenerateQueueSubmit`
- `ContextGenerateQueueDrain`

These may still be domain capabilities in a broad sense, but they should not be the initial building blocks for compiled tasks.

## Concrete Profiles For The First Slice

This section makes the first-slice task-facing set more concrete.
For each capability, it names:

- the internal function chain it should encompass
- the runtime initialization state that should live with the capability runtime object
- the external params that the sig adapter resolves into domain arguments
- the typed artifacts that come back out

The important boundary is this:

- `task` and `control` see slots, bindings, effects, and artifacts
- the domain-owned sig adapter resolves those external values into the internal function chain
- the underlying functionality stays behind the capability boundary

Everything listed below under runtime initialization, internal function chain, and internal arguments is domain-internal.
Those items are not durable task payload fields.
They may include process-local helpers that never cross the capability boundary.

### `WorkspaceResolveNodeId`

Purpose:

- resolve authored target input into one stable node scope that later capabilities can consume

Current implementation anchor:

- [commands.rs](/home/jerkytreats/meld/src/workspace/commands.rs)

Internal function chain:

- `workspace_lookup_path`
- store lookup through `find_by_path` or `get_by_path`
- `resolve_node_id_by_canonical_fallback`
- direct node id validation when the input is already a node id

Runtime initialization holds:

- capability instance identity
- workspace scope kind
- `workspace_root` binding
- input slot and output slot contracts
- execution policy metadata

Sig adapter resolves:

- one target selector input artifact
  - path-like target
  - or node-id-like target
- `include_tombstoned` policy binding when needed

Into internal arguments:

- `api`
- `workspace_root`
- resolved target selector
- `include_tombstoned`

Artifacts out:

- `resolved_node_ref`
- `target_resolution_summary`

### `MerkleTraversal`

Purpose:

- derive structural execution order from a root node scope and traversal strategy

Current implementation anchor:

- [merkle_traversal.rs](/home/jerkytreats/meld/src/merkle_traversal.rs)

Internal function chain:

- `traverse`
- level collection by Merkle child walk
- strategy-directed batch ordering
- `OrderedMerkleNodeBatches::new`

Runtime initialization holds:

- capability instance identity
- subtree or workspace scope
- chosen traversal bindings that are static for the bound instance
- input slot and output slot contracts

Sig adapter resolves:

- `resolved_node_ref` input artifact
- `traversal_strategy` binding
- optional traversal policy binding

Into internal arguments:

- `api`
- `target_node_id`
- `TraversalStrategy`

Artifacts out:

- `ordered_merkle_node_batches`
- `traversal_metadata`
- `traversal_observation_summary`

### `ContextGeneratePrepare`

Purpose:

- gather node, prompt, provider, and lineage inputs into one provider-ready generation request

Current implementation anchor:

- [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs)
- [prompt_collection.rs](/home/jerkytreats/meld/src/context/generation/prompt_collection.rs)

Internal function chain:

- `api.get_agent`
- node record load through `api.node_store().get`
- `PromptContract::from_agent`
- `build_prompt_messages`
- `prepare_provider_for_request`
- `prepare_generated_lineage`
- optional previous metadata snapshot load for downstream finalization

Runtime initialization holds:

- capability instance identity
- node scope kind
- bound agent, provider, and generation policy values when statically chosen
- input slot and output slot contracts
- execution policy metadata for this capability instance

Sig adapter resolves:

- `resolved_node_ref` input artifact
- optional upstream lineage or observation artifact
- explicit force or replay posture when relevant

Into internal arguments:

- context API handle
- node scope
- agent profile
- provider execution binding
- prompt contract inputs
- lineage preparation input

Artifacts out:

- `provider_execute_request`
- `preparation_summary`
- `prompt_context_lineage_summary`
- `preparation_observation_summary`

### `ProviderExecuteChat`

Purpose:

- execute ready provider work with batching, throttling, and response correlation hidden behind the provider boundary

Current implementation anchor:

- [executor.rs](/home/jerkytreats/meld/src/provider/executor.rs)

Internal function chain:

- execution admission
- provider compatibility grouping for batchable work
- `prepare_provider_for_request`
- `execute_completion`
- provider lifecycle event emission
- normalized result shaping

Runtime initialization holds:

- capability instance identity
- provider execution class
- any static provider lane or retry policy bindings
- input slot and output slot contracts
- batching and throttling policy metadata

Sig adapter resolves:

- `provider_execute_request` input artifact
- provider execution policy bindings such as retry class or lane hints when present

Into internal arguments:

- provider config
- runtime overrides
- provider client
- chat message payload
- completion options

Artifacts out:

- `provider_execute_result`
- `provider_usage_summary`
- `provider_timing_summary`
- `provider_observation_summary`

### `ContextGenerateFinalize`

Purpose:

- validate provider output, build durable frame metadata, persist frame effects, and emit downstream result artifacts

Current implementation anchor:

- [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs)
- [metadata_construction.rs](/home/jerkytreats/meld/src/context/generation/metadata_construction.rs)

Internal function chain:

- `build_and_validate_generated_metadata`
- metadata validation event emission
- `Frame::new`
- `api.put_frame`
- generation completion result shaping

Runtime initialization holds:

- capability instance identity
- frame family scope
- frame type and persistence policy bindings when statically chosen
- input slot and output slot contracts
- effect contract for frame and head writes

Sig adapter resolves:

- `provider_execute_result` input artifact
- `preparation_summary` input artifact

Into internal arguments:

- generated content bytes
- metadata validation inputs
- frame construction inputs
- frame persistence call

Artifacts out:

- `generation_result`
- `frame_ref`
- `frame_metadata_summary`
- `effect_summary`
- `finalization_observation_summary`

### Unified Caller Rule

Across the first-slice set, the caller-facing shape should stay uniform:

- initialize a capability runtime object from one bound capability instance
- invoke that runtime object with one invocation payload
- supply dynamic values only through named input slots and invocation context

That means `task` or `control` should never construct the internal `foo`, `string`, `bot` style argument set directly.
They should supply:

- bound static values during runtime initialization
- dynamic artifact or init values during invocation

The sig adapter then turns those two external layers into the internal call shape required by the domain functionality.

## Contract Shape For Score 5 And 4 Slice

The score 5 and 4 slice should share one execution-grade capability contract shape.
Those capabilities sit on the runtime path often enough that task compilation and later control execution need one uniform model for binding, validation, ordering, and dispatch.

The right abstraction level is above function signatures and below workflow-shaped orchestration.
The durable publication unit is the capability type contract.
Task compilation then combines that contract with compile-time scope and binding values to materialize a bound capability instance.

The contract should answer these questions:

- what scope kinds this capability supports
- what named bindings must be supplied before an instance is valid
- what typed artifacts may satisfy each input slot
- whether an input may be satisfied by init payload, artifact handoff, or either source
- what typed artifacts may be emitted after execution
- what effects require ordering even when no artifact changes hands
- what execution and retry class applies when the scheduler dispatches it

```rust
struct CapabilityTypeContract {
    capability_type_id: CapabilityTypeId,
    capability_version: CapabilityVersion,
    owning_domain: DomainId,
    scope_contract: ScopeContract,
    binding_contract: Vec<BindingSpec>,
    input_contract: Vec<InputSlotSpec>,
    output_contract: Vec<OutputSlotSpec>,
    effect_contract: Vec<EffectSpec>,
    execution_contract: ExecutionContract,
}
```

The contract above is enough for publication and compiler validation.
Task compilation should consume that type contract plus a small bound-instance projection:

```rust
struct BoundCapabilityInstance {
    capability_instance_id: CapabilityInstanceId,
    capability_type_id: CapabilityTypeId,
    capability_version: CapabilityVersion,
    scope_ref: ScopeRef,
    scope_kind: ScopeKind,
    binding_values: Vec<BindingValue>,
    input_wiring: Vec<InputWiring>,
    output_expectations: Vec<OutputExpectation>,
}
```

That keeps the abstraction split clean:

- domains publish reusable type contracts
- compiler binds those contracts into concrete instances
- execution sees bound instances plus validated edges, not domain internals

### Graph Consumer Relevant Details

For score 5 and 4 capabilities, a few details matter enough to be explicit in the contract:

- input slot source policy
  - init only
  - handoff only
  - init or handoff
- output emission policy
  - guaranteed
  - conditional with named condition
- effect ordering policy
  - shareable read
  - exclusive write
  - append-only
  - emit-only
- execution readiness policy
  - run when all required inputs resolve
  - run only after specific effect claims settle
  - queued or session-scoped admission when applicable

Those details stay declarative.
They do not expose adapters, storage structs, or transport handles.

### Why This Fits The 5 And 4 Band

- `MerkleTraversal`, `ContextGeneratePrepare`, `ProviderExecuteChat`, and `ContextGenerateFinalize` are artifact-driven and hand work cleanly from one domain to the next.
- `ContextGenerateFinalize` and later explicit state-mutation capabilities need effect metadata because they write durable state.
- `WorkspaceResolveNodeId` and `ConfigLoadWorkspace` often behave like prerequisite binders, so explicit scope and binding contracts matter more than complex outputs.
- `TelemetryEmitEvent` is runtime-adjacent but sink selection is environmental, so execution class and effect target are more important than downstream artifact fan-out.

### Example Readings

- `MerkleTraversal`
  - scope: workspace or subtree root
  - bindings: traversal strategy, traversal policy
  - inputs: target selection artifact from init or upstream handoff
  - outputs: ordered Merkle node batches, traversal metadata
  - effects: none

- `ContextGeneratePrepare`
  - scope: node
  - bindings: agent ref, provider ref, generation policy
  - inputs: target node ref, lineage input, optional upstream observations
  - outputs: provider execute request, preparation summary
  - effects: read-only access to context and prompt sources

- `ProviderExecuteChat`
  - scope: turn
  - bindings: provider ref, generation policy
  - inputs: provider execute request from upstream handoff
  - outputs: provider execute result, observation summary
  - effects: none beyond provider call accounting

- `ContextGenerateFinalize`
  - scope: node or frame family
  - bindings: frame type, persistence policy
  - inputs: provider execute result, preparation summary
  - outputs: frame ref
  - effects: exclusive write against frame store and active head targets

### Instance Projection Rule

Task compilation should consume capability contracts through a bound instance projection, not by reading domain function shapes.
That means:

- type contract owns static meaning
- bound instance owns chosen scope, bindings, and wiring
- dependency edges and artifact handoffs are derived from those two layers together

### Boundary Rule

This shape should be the default for score 5 and 4 capabilities only.
Score 3 queue or session capabilities may need wrappers because their value is more about hosting and lifecycle.
Score 2 through 0 capabilities can remain looser until they are proven useful as graph nodes.

---

## `agent`

Loose stand-in for `src/agent/capability.rs`.

```rust
// capability: AgentList — enumerate configured agents and roles
// capability: AgentShow — load one agent config and prompt path
// capability: AgentValidate — validate profile and prompt for one or all agents
// capability: AgentStatus — summarize validity and prompt presence
// capability: AgentCreate — add agent profile under storage layout
// capability: AgentRemove — remove agent profile
// capability: AgentSetRole — change reader or writer role
// capability: AgentResolvePrompt — resolve prompt path and optional content
```

---

## `config`

Loose stand-in for `src/config/capability.rs`.

```rust
// capability: ConfigResolvePaths — XDG and workspace path roots
// capability: ConfigLoadWorkspace — load workspace-scoped configuration
// capability: ConfigMerge — apply merge policy across sources
// capability: ConfigValidate — surface validation errors for config graphs
```

---

## `context`

Loose stand-in for `src/context/capability.rs`. Primary refactor target for task-ready generation.

```rust
// capability: ContextGeneratePrepare — assemble prompt context and provider-ready request
// capability: ContextGenerateFinalize — validate result, build metadata, and persist frame effects
// capability: ContextGenerateQueueSubmit — enqueue generation work with priority
// capability: ContextGenerateQueueDrain — process queue events and stats
// capability: ContextQueryView — build composed context view for a node
// capability: ContextQueryGet — direct reads used by view and callers
// capability: FrameRead — load frame and merkle set for a frame id
// capability: FrameWrite — persist new frame and update merkle state
// capability: FrameCompact — compact frame storage
// capability: FrameRestore — restore from compacted state
// capability: FrameTombstone — mark frames or lines as tombstoned per domain rules
```

---

## `heads`

Loose stand-in for `src/heads/capability.rs`. Today tightly coupled to store layout; capability boundaries may merge with `store` later.

```rust
// capability: HeadIndexLoad — load head index from disk
// capability: HeadIndexSave — persist head index
// capability: HeadResolveActive — latest non-tombstoned frame id for node and frame type
// capability: HeadSet — point head at a frame id
// capability: HeadTombstone — tombstone head entry while keeping history addressable
```

---

## `init`

Loose stand-in for `src/init/capability.rs`.

```rust
// capability: InitBootstrapDefaults — seed default agents, prompts, embedded workflows
// capability: InitIdempotentCheck — report what would change without writing
```

---

## `merkle_traversal`

Loose stand-in for `src/merkle_traversal.rs`.

```rust
// capability: MerkleTraversal — derive ordered Merkle node batches from scope and strategy
```

---

## `metadata`

Loose stand-in for `src/metadata/capability.rs`.

```rust
// capability: FrameMetadataDescribe — key descriptors and frame types registry
// capability: FrameMetadataBuildGenerated — build metadata for generated frames from inputs
// capability: PromptLinkValidate — validate prompt link contracts for writes
```

---

## `prompt_context`

Loose stand-in for `src/prompt_context/capability.rs`.

```rust
// capability: PromptContextAssemble — orchestrate storage reads into prompt context payload
// capability: PromptContextContractValidate — validate contracts before downstream use
```

---

## `provider`

Loose stand-in for `src/provider/capability.rs`.

```rust
// capability: ProviderList — enumerate provider profiles
// capability: ProviderShow — one profile plus optional key status
// capability: ProviderCreate — write new provider profile
// capability: ProviderRemove — remove profile and paths
// capability: ProviderValidate — validate profile fields and types
// capability: ProviderTest — connectivity and model listing
// capability: ProviderExecuteChat — bind and run chat completion for generation callers
```

---

## `store`

Loose stand-in for `src/store/capability.rs`.

```rust
// capability: StoreOpen — open or attach persistence for a workspace
// capability: StoreReadNode — read node record and metadata
// capability: StoreWriteNode — persist node updates
// capability: StorePersistenceSync — flush and consistency hooks as defined by store layer
```

---

## `telemetry`

Loose stand-in for `src/telemetry/capability.rs`.

```rust
// capability: TelemetryEmitEvent — record structured event
// capability: TelemetrySessionOpen — start session with policy
// capability: TelemetrySessionClose — end session and flush
// capability: TelemetryRouteIngest — accept events into internal bus
// capability: TelemetrySinkOtel — export to OpenTelemetry backend
// capability: TelemetrySinkTui — TUI-facing sink
// capability: TelemetrySinkStore — persist events to store-backed sink
```

---

## `workflow`

Loose stand-in for `src/workflow/capability.rs`. These are compatibility-facing surfaces, not recommended first-slice task nodes.

```rust
// capability: WorkflowRegistryList — list registered workflow ids and versions
// capability: WorkflowRegistryValidate — validate all registered workflows
// capability: WorkflowInspect — load profile for one workflow id
// capability: WorkflowExecuteTarget — run registered target through thread and gates
// capability: WorkflowStateRead — read durable workflow state as exposed by state store
// capability: WorkflowStateWrite — persist workflow progress and records
```

---

## `workspace`

Loose stand-in for `src/workspace/capability.rs`.

```rust
// capability: WorkspaceScan — scan tree and capture scan state
// capability: WorkspaceStatus — aggregate agent, provider, tree, context coverage
// capability: WorkspaceValidate — validate workspace invariants
// capability: WorkspaceListDeleted — list deleted or tombstoned paths per policy
// capability: WorkspaceWatchRun — run watch daemon and editor bridge
// capability: WorkspaceCiBatch — CI-oriented batch operations and reports
// capability: WorkspaceResolveNodeId — canonical node id resolution and fallbacks
// capability: WorkspaceDanger — privileged or destructive operations behind explicit service
```

---

## Modules without a domain `capability.rs`

`api` and `cli` parse and delegate; they should not grow capability catalogs.

`concurrency`, `error`, `ignore`, `logging`, `tree`, `types`, and `views` are shared infrastructure or types, not capability owners under the domain architecture rule.

---

## Relation to `agent::Capability`

`crate::agent::identity::Capability` is **Read** and **Write** flags on agent roles. That enum is unrelated to HTN-style capability contracts in this document. Rename or namespace separation is a follow-up if confusion persists.
