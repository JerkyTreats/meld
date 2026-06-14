# Durable Runtime Pre Implementation Gaps

Date: 2026-06-14
Status: proposed
Scope: gaps to resolve before implementing the durable single process runtime host

## Purpose

This document captures review findings from [Durable Runtime First Slice](durable_runtime_first_slice.md) that should be resolved before runtime host implementation begins.

The durable host should assemble and tick existing domain runtimes. It should not invent artifact persistence, satisfaction decision persistence, worker status semantics, storage layout, or fixture identity rules while being implemented.

The design posture is production first. The first slice may be narrow in scenario count, actor count, and operational surface, but its storage contracts, idempotency rules, and ownership boundaries should be suitable as the foundation for expanded runtime work.

## Relationship To Runtime Assembly

These gaps are pre implementation blockers for the runtime host.

They are not permission to move domain semantics into root `meld`. Each fix should leave behind a contract or fixture that the host can consume through an owning domain boundary.

## In Scope

- durable artifact storage
- satisfaction curation persistence before execution goal mutation
- worker report semantics for bounded ticks
- concrete product storage layout
- first proof fixture and identity rules
- restart checkpoint requirements for the first proof

## Out Of Scope

- adding the runtime host
- adding a CLI command
- adding a background daemon
- adding multi process worker supervision
- changing goal judgment ownership
- changing planning or task network authority
- adding broad sensory, causal, or regime runtimes

## Gap Summary

| ID | Gap | Owner | Why It Blocks Runtime | Required Outcome | Evidence |
| --- | --- | --- | --- | --- | --- |
| `RTG-1` | Durable task artifact persistence | `execution` task domain | The runtime plan names a task artifact store, but the current task artifact repo is in memory. A host reopen cannot rely on live executor state for artifacts that later task steps, publications, or diagnostics may need. | Add a durable artifact store contract under execution task ownership. The first proof should use the production storage path even if it stores only the first docs writer artifacts. | [runtime storage shape](durable_runtime_first_slice.md), [artifact repo](../../../crates/meld-execution/src/task/artifact_repo.rs), [task executor](../../../crates/meld-execution/src/task/executor.rs) |
| `RTG-2` | Satisfaction curation decision persistence | `world_state` agent plus root adapter | Satisfaction curation can produce an agent mutation command, and execution can store the satisfied lifecycle state. The missing runtime contract is the persisted agent decision before applying the execution mutation. | Add a callable satisfaction curation worker or facade that persists `AgentCurationDecision`, dedupes by review identity, and only then exposes the goal mutation for the execution adapter. | [agent curation](../../../crates/meld-world-model/src/agent/curation.rs), [agent store](../../../crates/meld-world-model/src/agent/store.rs), [goal mutation adapter](../../../src/execution/goal_mutation.rs) |
| `RTG-3` | Bounded worker report contract | owning actor domains with root assembly | `Idle` and `MadeProgress` are too thin for restart-safe convergence tests. The host needs progress and failure diagnostics without treating them as semantic authority. | Define a report shape for each worker tick with item count, input cursor or revision, output cursor or revision, retryable errors, fatal errors, and budget exhaustion. | [worker contract](durable_runtime_first_slice.md), [graph runtime](../../../crates/meld-world-model/src/world_state/graph/runtime.rs), [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) |
| `RTG-4` | Product storage layout | root `meld` assembly plus owning stores | The plan requires one product storage root with named stores or trees, while implementation currently opens a mix of shared sled databases and per concern paths in tests. | Define exact production paths or tree names for event spine, graph reducer, belief, agent, execution goals, task network, task artifacts, context frames, and prompt artifacts. | [runtime storage shape](durable_runtime_first_slice.md), [minimal flywheel storage gap](minimal_runtime_flywheel.md), [task network store](../../../crates/meld-execution/src/task_network/store/sled.rs) |
| `RTG-5` | First proof fixture and identity rules | `integration` with `events`, `world_state`, and `execution` | The host must not choose semantic IDs, source kinds, method fixtures, or evidence mappings ad hoc while wiring the proof. | Specify the seed event, subject `DomainObjectRef`, branch and perspective, belief family config, docs freshness source kind, required artifact type, method fixture, task network id, session id, and stable record id derivations. | [first proof](durable_runtime_first_slice.md), [event ledger anchors](../../cognitive_architecture/events/multi_domain_spine.md), [outcome evidence mapper](../../../src/execution/outcome_evidence.rs) |
| `RTG-6` | Reopen checkpoint proof contract | `integration` plus root `meld` assembly | The durable plan requires reopening after at least one intermediate boundary, but it does not say which actors must be reloaded or which reports must prove resumed progress. | Define the required reopen checkpoints, expected durable state at each checkpoint, and forbidden host local state. At minimum, reopen after goal acceptance and after pending publication creation. | [first proof](durable_runtime_first_slice.md), [goal store reopen tests](../../../crates/meld-execution/tests/goals.rs), [task network reopen tests](../../../crates/meld-execution/tests/task_network_store.rs) |

## Design Posture

The runtime should avoid temporary shortcut contracts. A fixture may be small, a worker may process one item, and a proof may use one `docs_freshness` subject, but the persisted record shapes and boundary APIs should not need replacement when more subjects, agents, workers, or tasks are added.

When a gap has a fast path and a durable path, choose the durable path unless a separate assessment proves the durable path is larger than the first runtime foundation can responsibly carry.

## Required Fixes

### `RTG-1` Durable Task Artifact Persistence

The current `TaskArtifactRepo` is append only in behavior but lives inside the live task executor. That is enough for focused task tests, but it is not a durable product store.

Before host implementation, add a durable task artifact repository under the execution task domain. The store should be able to persist artifact records, artifact links, producer references, schema versions, and content references or inline content according to existing task contracts.

The first runtime proof should use this store even if the scenario writes only a small number of docs writer artifacts. That keeps the slice narrow without creating a throwaway persistence path.

### `RTG-2` Satisfaction Curation Decision Persistence

Goal creation curation has a durable delivery path through `AgentCuration::handle_delivery`. Satisfaction curation currently has a pure function and an execution adapter, but no named worker that persists the agent decision before the execution mutation is applied.

The fix should preserve the ownership split.

- world model agent owns satisfaction judgment and decision persistence
- root adapter maps the persisted mutation command into execution command shape
- execution goal API persists lifecycle state

The host should never call pure satisfaction curation and then mutate execution without first recording the agent decision.

### `RTG-3` Bounded Worker Report Contract

The host needs a uniform way to stop when the loop converges, detect retryable work, and report failure. It does not need a shared semantic state machine.

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

The runtime host needs a concrete production layout before code starts opening stores.

The layout should state whether each concern uses a separate sled database path or a shared database with named trees. It should also state which stores must flush before a host boundary is considered durable.

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

### `RTG-5` First Proof Fixture And Identity Rules

The end to end proof should be deterministic enough that the host cannot accidentally own semantic decisions. Fixture values should be small production examples, not ad hoc test only meanings.

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

The host proof should reopen from disk at planned boundaries and continue through domain stores only.

Required checkpoint coverage:

- after execution stores the active goal
- after task network creates a pending publication
- after publication append and before satisfaction curation when practical

At each checkpoint, the proof should drop the host value, reopen stores from the product root, run the next bounded turn, and assert that no correctness state was carried by local variables.

## Domain Impact

| Domain | Impact | Required Action |
| --- | --- | --- |
| `events` | own append and shared sequence | Provide stable record ids and replay cursors to the host through existing event contracts. |
| `world_state` | own graph, belief, planner projection, agent curation, and satisfaction judgment | Add or expose the satisfaction curation persistence worker and fixture inputs. |
| `execution` | own goals, planning, task network, task execution, publication, and task artifacts | Resolve artifact persistence scope and expose worker reports for planning, task network, task execution, and publication. |
| `api` | adapter | Wire existing context, provider, workspace, prompt, and event surfaces without becoming semantic authority. |
| `store` | product assembly concern | Provide the concrete root layout and flush expectations. |
| `config` | consume | Provide fixture or configuration values for seed agent, belief family, method library, task package, provider, and storage paths. |
| `context` | publish and consume | Persist final docs frame and prompt lineage through existing context contracts. |
| `task` | own task execution behavior | Preserve execution semantics and expose artifact persistence decision through execution task contracts. |

Other top level domains do not need new runtime authority for these gaps. They may remain fixed inputs, adapters, diagnostics, or non participants for the first durable proof.

## Exit Criteria

Runtime host implementation can begin when all statements below are true.

- `RTG-1` has a durable artifact store contract.
- `RTG-2` has a persisted satisfaction curation worker or facade.
- `RTG-3` has a bounded worker report shape used by every actor the host ticks.
- `RTG-4` has a concrete product storage map.
- `RTG-5` has a deterministic fixture and identity spec.
- `RTG-6` has explicit reopen checkpoint assertions for the end to end test.

## Evidence Date

Evidence was gathered on 2026-06-14 from the current source tree and the active cognitive architecture documents.
