# Durable Runtime Pre Implementation Gaps

Date: 2026-06-14
Status: historical pre-assembly gap evidence
Scope: gaps to resolve before implementing durable single process runtime assembly

Current runtime-completion authority is [Runtime Completion Ground Map](runtime_completion_ground_map.md), followed by [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md). This document preserves earlier gap analysis and must not restore the superseded one-turn or activation-heavy sequence.

## Purpose

This document captures review findings from [Durable Runtime First Slice](durable_runtime_first_slice.md) that should be resolved before runtime assembly implementation begins.

Durable assembly should wire existing domain runtimes. It should not invent artifact persistence, satisfaction decision persistence, worker status semantics, storage layout, or fixture identity rules while being implemented.

The design posture is production first. The first slice may be narrow in scenario count, actor count, and operational surface, but its storage contracts, idempotency rules, and ownership boundaries should be suitable as the foundation for expanded runtime work.

## Relationship To Runtime Assembly

These gaps are pre implementation blockers for runtime assembly.

They are not permission to move domain semantics into root `meld`. Each fix should leave behind a contract or fixture that runtime assembly can consume through an owning domain boundary.

## In Scope

- durable artifact storage
- satisfaction curation persistence before execution goal mutation
- worker report semantics for bounded ticks
- concrete product storage layout
- root supervisor domain contracts
- first proof fixture and identity rules
- restart checkpoint requirements for the first proof

## Out Of Scope

- adding runtime assembly
- adding a CLI command
- adding a background daemon
- adding multi process worker supervision
- changing goal judgment ownership
- changing planning or task network authority
- adding broad sensory, causal, or regime runtimes

## Gap Summary

| ID | Gap | Owner | Why It Blocks Runtime | Required Outcome | Evidence |
| --- | --- | --- | --- | --- | --- |
| `RTG-1` | Durable task artifact persistence | `execution` task domain | The runtime plan names a task artifact store, but the current task artifact repo is in memory. A process reopen cannot rely on live executor state for artifacts that later task steps, publications, or diagnostics may need. | Add a durable artifact store contract under execution task ownership. The first proof should use the production storage path even if it stores only the first docs writer artifacts. | [runtime storage shape](durable_runtime_first_slice.md), [artifact repo](../../../crates/meld-execution/src/task/artifact_repo.rs), [task executor](../../../crates/meld-execution/src/task/executor.rs) |
| `RTG-2` | Satisfaction curation decision persistence | `world_state` agent plus root adapter | Satisfaction curation can produce an agent mutation command, and execution can store the satisfied lifecycle state. The missing runtime contract is the persisted agent decision before applying the execution mutation. | Add a callable satisfaction curation worker or facade that persists `AgentCurationDecision`, dedupes by review identity, and only then exposes the goal mutation for the execution adapter. | [agent curation](../../../crates/meld-world-model/src/agent/curation.rs), [agent store](../../../crates/meld-world-model/src/agent/store.rs), [goal mutation adapter](../../../src/execution/goal_mutation.rs) |
| `RTG-3` | Bounded worker report contract | owning actor domains with root assembly | `Idle` and `MadeProgress` are too thin for restart-safe convergence tests. The supervisor needs progress and failure diagnostics without treating them as semantic authority. | Define a report shape for each worker operation with item count, input cursor or revision, output cursor or revision, retryable errors, fatal errors, and budget exhaustion. | [worker contract](durable_runtime_first_slice.md), [graph runtime](../../../crates/meld-world-model/src/world_state/graph/runtime.rs), [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) |
| `RTG-4` | Product storage layout | root `meld` assembly plus owning stores | The plan requires one product storage root with named stores or trees, while implementation currently opens a mix of shared sled databases and per concern paths in tests. | Define exact production paths or tree names for event spine, graph reducer, belief, agent, execution goals, task network, task artifacts, context frames, and prompt artifacts. | [runtime storage shape](durable_runtime_first_slice.md), [minimal flywheel storage gap](minimal_runtime_flywheel.md), [task network store](../../../crates/meld-execution/src/task_network/store/sled.rs) |
| `RTG-5` | First proof fixture and identity rules | `integration` with `events`, `world_state`, and `execution` | Root assembly must not choose semantic IDs, source kinds, method fixtures, or evidence mappings ad hoc while wiring the proof. | Specify the seed event, subject `DomainObjectRef`, branch and perspective, belief family config, docs freshness source kind, required artifact type, method fixture, task network id, session id, and stable record id derivations. | [first proof](durable_runtime_first_slice.md), [event ledger anchors](../../cognitive_architecture/events/multi_domain_spine.md), [outcome evidence mapper](../../../src/execution/outcome_evidence.rs) |
| `RTG-6` | Reopen checkpoint proof contract | `integration` plus root `meld` assembly | The durable plan requires reopening after at least one intermediate boundary, but it does not say which actors must be reloaded or which reports must prove resumed progress. | Define the required reopen checkpoints, expected durable state at each checkpoint, and forbidden root local state. At minimum, reopen after goal acceptance and after pending publication creation. | [first proof](durable_runtime_first_slice.md), [goal store reopen tests](../../../crates/meld-execution/tests/goals.rs), [task network reopen tests](../../../crates/meld-execution/tests/task_network_store.rs) |
| `RTG-7` | Runtime supervisor domain | root `meld` supervisor | Root assembly needs a legitimate lifecycle domain or it can lose track of runtime handles, duplicate work, and hide failed processes. | Add supervisor contracts and storage for runtime ids, desired state, leases, heartbeats, health, shutdown, and restart policy while keeping semantic progress in domain stores. | [supervisor domain plan](runtime_supervisor_domain_plan.md), [runtime contracts](../../../src/runtime/contracts.rs), [runtime storage](../../../src/runtime/storage.rs) |

## Design Posture

The runtime should avoid temporary shortcut contracts. A fixture may be small, a worker may process one item, and a proof may use one `docs_freshness` subject, but the persisted record shapes and boundary APIs should not need replacement when more subjects, agents, workers, or tasks are added.

When a gap has a fast path and a durable path, choose the durable path unless a separate assessment proves the durable path is larger than the first runtime foundation can responsibly carry.

## Required Fixes

### `RTG-1` Durable Task Artifact Persistence

The current `TaskArtifactRepo` is append only in behavior but lives inside the live task executor. That is enough for focused task tests, but it is not a durable product store.

Before runtime assembly implementation, add a durable task artifact repository under the execution task domain. The store should be able to persist artifact records, artifact links, producer references, schema versions, and content references or inline content according to existing task contracts.

The first runtime proof should use this store even if the scenario writes only a small number of docs writer artifacts. That keeps the slice narrow without creating a throwaway persistence path.

### `RTG-2` Satisfaction Curation Decision Persistence

Goal creation curation has a durable delivery path through `AgentCuration::handle_delivery`. Satisfaction curation currently has a pure function and an execution adapter, but no named worker that persists the agent decision before the execution mutation is applied.

The fix should preserve the ownership split.

- world model agent owns satisfaction judgment and decision persistence
- root adapter maps the persisted mutation command into execution command shape
- execution goal API persists lifecycle state

Root assembly should never call pure satisfaction curation and then mutate execution without first recording the agent decision.

### `RTG-3` Bounded Worker Report Contract

Detailed implementation plan: [RTG-3 Bounded Worker Report Contract Implementation Plan](rtg_3_bounded_worker_report_contract.md).

The supervisor needs a uniform way to detect retryable work, report failure, and observe proof progress. It does not need a shared semantic state machine.

This should be a stable runtime diagnostics contract rather than a test helper.

Each worker tick should report at least:

- actor id
- input cursor or input revision
- output cursor or output revision
- items attempted
- items committed
- retryable errors
- fatal errors
- budget exhausted flag

The report is diagnostic. The durable state remains in the owning domain store.

### `RTG-4` Product Storage Layout

Runtime assembly needs a concrete production layout before code starts opening stores.

The layout should state whether each concern uses a separate sled database path or a shared database with named trees. It should also state which stores must flush before an assembly boundary is considered durable.

Required entries:

- event spine
- graph reducer state
- belief store
- agent store
- execution goal store
- task network store
- task artifact store
- context frame store
- prompt artifact store

Implementation surface:

- root `meld` owns `ProductStorageLayout`, `OpenProductStores`, path selection, database opening, and checkpoint flush ordering
- owning crates keep record meaning, schema validation, replay rules, idempotency, and durable cursor semantics
- execution-owned factories open task-scoped artifact repositories and task network stores without exposing raw `sled::Db` handles to root assembly

Concrete product layout:

```text
<product_root>/
  ledger.sled/
  workspace.sled/
  world_model.sled/
  execution/
    goals.sled/
    task_artifacts.sled/
    task_networks/
      <network_storage_key>.sled/
  context/
    frames/
    prompt_artifacts/
```

| Concern | Backing location | Owner |
| --- | --- | --- |
| event spine and session compatibility | `ledger.sled` | `meld-events` |
| workspace node records | `workspace.sled` | root `meld` workspace store |
| graph reducer state | `world_model.sled` | `meld-world-model` |
| belief store | `world_model.sled` | `meld-world-model` |
| agent store | `world_model.sled` | `meld-world-model` |
| legacy world state claim store | `world_model.sled` | `meld-world-model` |
| execution goals | `execution/goals.sled` | `meld-execution` |
| task network stores | `execution/task_networks/<network_storage_key>.sled` | `meld-execution` |
| task artifacts | `execution/task_artifacts.sled` | `meld-execution` |
| context frame blobs | `context/frames` | root `meld` context |
| prompt artifact blobs | `context/prompt_artifacts` | root `meld` prompt context |

Task network stores use one sled database per network id because the current task network trees are scoped to one reduced network state. `TaskNetworkStoreFactory` validates network ids before deriving storage keys and does not silently rewrite ids.

A proof checkpoint is durable only after all stores touched by the bounded turn have flushed. `OpenProductStores::flush_boundary` flushes always-open product stores, including the event ledger, workspace store, world model stores, execution goal store, and task artifact factory. Task network stores are opened per network and must be flushed by the caller before the boundary is treated as a checkpoint.

Legacy workflow JSON migration is out of scope for `RTG-4`. A later migration may move workflow thread and turn records into the execution storage root after characterization and compatibility tests exist.

### `RTG-5` First Proof Fixture And Identity Rules

The end to end proof should be deterministic enough that root assembly cannot accidentally own semantic decisions. Fixture values should be small production examples, not ad hoc test only meanings.

The fixture should specify:

- subject object reference
- seed event or observed fact shape
- branch and perspective
- belief family configuration
- curation threshold
- method library entry
- task package or workflow asset
- required artifact type for docs freshness evidence
- publication event type
- stable record id derivations
- expected final goal lifecycle

### `RTG-6` Reopen Checkpoint Proof Contract

The flywheel proof should reopen from disk at planned boundaries and continue through domain stores only.

Required checkpoint coverage:

- after execution stores the active goal
- after task network creates a pending publication
- after publication append and before satisfaction curation when practical

At each checkpoint, the proof should drop the assembly value, reopen stores from the product root, run the next bounded turn, and assert that no correctness state was carried by local variables.

### `RTG-7` Runtime Supervisor Domain

The root supervisor must be a focused runtime domain with its own durable lifecycle records. Without that boundary, root assembly can lose track of runtime handles, start duplicate workers, hide stale leases, or report a healthy system while domain runtimes are stopped.

The supervisor fix should add:

- runtime id contracts
- desired runtime state records
- runtime instance records
- lease acquisition and renewal
- heartbeat records
- health snapshots
- shutdown records
- conservative restart policy
- operator status query surface

The supervisor must not store semantic progress. It may store the last diagnostic report from a domain runtime, but the report is observation only. Domain stores remain authoritative for cursors, goals, beliefs, task networks, publications, and event sequence.

Detailed implementation plan: [Runtime Supervisor Domain Plan](runtime_supervisor_domain_plan.md).

## Domain Impact

| Domain | Impact | Required Action |
| --- | --- | --- |
| `events` | own append and shared sequence | Provide stable record ids and replay cursors to runtime assembly through existing event contracts. |
| `world_state` | own graph, belief, planner projection, agent curation, and satisfaction judgment | Add or expose the satisfaction curation persistence worker and fixture inputs. |
| `execution` | own goals, planning, task network, task execution, publication, and task artifacts | Resolve artifact persistence scope and expose worker reports for planning, task network, task execution, and publication. |
| `runtime` | own lifecycle supervision | Add supervisor contracts, storage, leases, heartbeat, health, shutdown, and restart policy. |
| `api` | adapter | Wire existing context, provider, workspace, prompt, and event surfaces without becoming semantic authority. |
| `store` | product assembly concern | Provide the concrete root layout and flush expectations. |
| `config` | consume | Provide fixture or configuration values for seed agent, belief family, method library, task package, provider, and storage paths. |
| `context` | publish and consume | Persist final docs frame and prompt lineage through existing context contracts. |
| `task` | own task execution behavior | Preserve execution semantics and expose artifact persistence decision through execution task contracts. |

Other top level domains do not need new runtime authority for these gaps. They may remain fixed inputs, adapters, diagnostics, or non participants for the first durable proof.

## Exit Criteria

Runtime assembly implementation can begin when all statements below are true.

- `RTG-1` has a durable artifact store contract.
- `RTG-2` has a persisted satisfaction curation worker or facade.
- `RTG-3` has a bounded worker report shape used by every actor in the proof.
- `RTG-4` has a concrete product storage map.
- `RTG-5` has a deterministic fixture and identity spec.
- `RTG-6` has explicit reopen checkpoint assertions for the end to end test.
- `RTG-7` has a supervisor domain contract and storage plan.

## Evidence Date

Evidence was gathered on 2026-06-14 from the current source tree and the active cognitive architecture documents.
