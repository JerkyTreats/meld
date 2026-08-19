# Execution Task Network Phase 7 Implementation Plan

Date: 2026-06-01
Status: implemented
Scope: Phase 7 bridge from execution composition to task network mutation, one ready task, one dispatch, and outcome publication

## Purpose

This plan defines the implementation path for Phase 7.

Phase 7 starts from `planning::contracts::ExecutionComposition` and ends when one accepted task network mutation can dispatch one task through the existing task runtime and publish task outcome events through the execution event shape.

The phase builds the upper graph layer while preserving the existing task compiler, task package lowering, task executor, and capability runtime as lower execution machinery.

The next global execution phase after this plan was [Phase 8 Expanded Execution Slice](PHASE8.md), which is now implemented. Internal phase numbers in this file describe the Phase 7 implementation sequence only.

## Assessment Summary

The codebase already has strong lower execution machinery in `crates/meld-execution`.

Existing implementation evidence:

- `planning::contracts::ExecutionComposition` carries the concrete `meld_lang::Composition`, selected method, bindings, projected effects, validation report, operator resolution reports, and diagnostics.
- `task::compiler::compile_task_definition` compiles `TaskDefinition` into `CompiledTaskRecord`.
- `task::package` lowers workflow package authoring into `PreparedTaskRun`.
- `task::executor::TaskExecutor` runs one compiled task-local capability graph.
- `task::runtime::execute_task_to_completion` dispatches ready capability invocations inside one task.
- `task::events` already maps task events into canonical execution event envelopes.
- `task::expansion::TaskExpansionCompiler` already uses a small trait and registry when several expansion compiler implementations are expected.

Implementation to add:

- add the `task_network` domain module
- add the task network command mailbox contract
- add the single writer command acceptance facade
- add task network mutation contracts
- add the task network mutation log
- add reduced task network state
- add ready set computation over task nodes
- add fenced dispatch claims before task execution
- add the durable bridge from completed task output to outcome publication
- add durable publication outbox state
- add lowering from `ExecutionComposition` to task network commands carrying mutation sets

Existing quality patterns to preserve:

- `meld-execution` denies missing public docs at crate level.
- public contract modules include module Rustdoc and examples where the boundary benefits from a concrete use case.
- contract tests round trip public records through serde.
- source boundary tests scan for forbidden imports and forbidden runtime state.
- durable stores have reopen tests, duplicate command tests, corrupt record tests, and outside workspace path tests.
- deterministic behavior has focused tests and property tests.
- planning contracts already have fuzz targets for decode and runtime shape.
- task readiness already has property tests for completed and in flight exclusion.
- task package lowering has fixture style tests for embedded package documents and compatibility paths.

## Design Commitments

The current task compiler remains task-local.

It compiles:

```text
TaskDefinition
-> CompiledTaskRecord
```

Phase 7 adds a new upper lowering layer.

It lowers:

```text
ExecutionComposition
-> task_network::mutation::Set
```

The new lowering layer calls the existing task compiler for each executable task node. Task network mutation, task network state, ready set computation, and dispatch stay owned by the upper graph layer.

The implementation adds one level-specific lowerer and one narrow inner compiler contract for dependency injection and alternate test implementations.

## Boundary Statement

`planning::lowering` owns execution composition lowering.

It owns:

- reading one `ExecutionComposition`
- validating lowering readiness
- mapping `StepKind::Op` steps into task node build inputs
- preserving `StepKind::Goal` as a deferred recursive planning boundary
- mapping composition edges into task network dependency edge proposals
- producing deterministic lowering diagnostics
- producing a `task_network::mutation::Set`

Adjacent domains own:

- task network command acceptance and mutation reduction
- task network state reduction
- task network persistence
- task dispatch
- capability invocation
- task outcome publication
- goal lifecycle mutation
- world model belief revision

`task_network` owns command acceptance, mutation reduction, reduced graph state, ready set computation over task nodes, dispatch claims, task lifecycle event reduction, and outcome publication handoff.

`task` continues to own task-local compilation, task-local artifact storage, task-local capability readiness, task execution, task expansion, and task events.

No runtime caller mutates task network state directly.

Planning agents, task workers, publication workers, and recovery code submit commands to the task network command boundary. The task network reducer is the only writer to authoritative network state.

## Naming Rules

Use module ownership for internal names. Re-export explicit names only at public boundaries.

Internal names:

```text
planning::lowering::Request
planning::lowering::Plan
planning::lowering::Diagnostic
planning::lowering::Lowerer

task_network::mutation::Set
task_network::mutation::Mutation
task_network::mutation::Inject
task_network::mutation::CommitRequest
task_network::mutation::CommitResult
task_network::mutation::CommitRecord
task_network::command::Request
task_network::command::Command
task_network::command::Response

task_network::state::NetworkState
task_network::state::TaskNode
task_network::state::DependencyEdge
task_network::state::ReadySet
task_network::state::TaskStatus

task_network::dispatch::Request
task_network::dispatch::Claim
task_network::dispatch::Outcome

task_network::outcome::Publication
task_network::outcome::PublicationState
```

Public re-export names:

```rust
pub use planning::lowering::{
    Diagnostic as CompositionLoweringDiagnostic,
    Lowerer as ExecutionCompositionLowerer,
    Plan as CompositionLoweringPlan,
    Request as CompositionLoweringRequest,
};

pub use task_network::mutation::{
    CommitRecord as TaskNetworkCommitRecord,
    CommitRequest as TaskNetworkCommitRequest,
    CommitResult as TaskNetworkCommitResult,
    Inject as TaskNetworkInjectMutation,
    Mutation as TaskNetworkMutation,
    Set as TaskNetworkMutationSet,
};

pub use task_network::command::{
    Command as TaskNetworkCommand,
    Response as TaskNetworkCommandResponse,
    Request as TaskNetworkCommandRequest,
};
```

## First Slice Flow

The first implementation slice follows this flow:

```text
planning::contracts::ExecutionComposition
-> planning::lowering::Request
-> planning::lowering::Plan
-> task_network::mutation::Set
-> task_network::command::Request
-> task_network::mutation::CommitRequest
-> task_network::mutation::CommitRecord
-> task_network::command::Response
-> task_network::state::NetworkState
-> task_network::state::ReadySet
-> task_network::dispatch::Request
-> task_network::dispatch::Outcome
-> task_network::outcome::Publication
```

The first slice supports one `StepKind::Op` that resolves to one executable task node.

The first slice preserves composition identity, goal identity, method identity, step identity, world state frame identity, and operator resolution identity through every bridge record.

## Task Network Graph Strategy

The task network is a versioned directed acyclic graph projected from an append-only journal.

The task network is a single writer event sourced aggregate.

Planning agents submit task network commands. Task workers submit task outcome commands. Publication workers submit publication mark commands. Recovery code submits replay and repair commands. No caller writes the graph, claim state, outcome state, or publication state directly.

The command boundary serializes proposal acceptance and assigns one total order to accepted graph changes and lifecycle changes.

The first implementation keeps the reducer synchronous and deterministic. A later runtime actor wraps that reducer with a command mailbox, queues, worker pools, retry workers, cancellation tokens, and graceful shutdown. The actor is a serialization shell around pure replay and validation code.

Each graph edit proposal carries:

```text
command_id
base_revision
base_state_hash
read_preconditions
mutation_set
```

The command path revalidates every proposal against the latest committed state. A command commits when its declared read preconditions still hold against the current graph and lifecycle state. A command returns a typed conflict when the current state invalidates its read preconditions.

All accepted structural commits, dispatch claims, task outcomes, artifact availability updates, and publication marks share one monotonic revision stream. Separate durable trees serve as indexes over the journal, command outcomes, dispatch claims, outcomes, and publications.

The active graph is acyclic at every committed revision. Loops over time are represented as later mutations that create a new graph revision.

Task network task status is driven only by committed graph mutations, dispatch claims, task lifecycle outcome records, artifact availability records, and publication records. World model changes do not mutate task network state directly. World model changes cause planning commands when the planning loop decides the graph should change.

Readiness is derived from `NetworkState`. `ReadySet` is a query result over pending tasks, dependency edges, task statuses, and artifact availability. `ReadySet` is not authoritative persisted state.

Every task node carries a lifecycle epoch. Cancel, prune, preserve, and replacement operations advance or supersede epochs. Dispatch outcomes must name the task instance id, lifecycle epoch, claim id, and claim revision. A stale claim outcome cannot advance task state.

Outcome publication uses a durable outbox. Task completion creates an outcome record and a publication record in the task network revision stream before any external publication attempt. Publication retry workers mark publication state through commands.

The first precondition vocabulary is typed and narrow:

```text
RevisionIs
StateHashIs
NodeExists
NodeAbsent
NodeStatusIs
EdgeExists
EdgeAbsent
ArtifactAvailable
NoPath
ClaimCurrent
PublicationPending
```

The first conflict categories are:

```text
StaleBase
StateHashMismatch
FailedPrecondition
DuplicateCommand
InvalidGraph
InvalidLifecycleTransition
StaleClaim
PublicationAlreadyMarked
```

Durable state uses domain records with stable ids and stable serde fixtures. Graph libraries serve private validation and analysis only.

`task_network::readiness` and `task_network::store` build transient `petgraph` graphs from domain records for acyclicity checks, endpoint checks, affected subgraph analysis, and diagnostic reporting. Transient graph node indexes are local to validation.

Scaling does not come from multiple writers mutating one graph. It comes from read projections, worker pools, command batching, commutative command fast paths, and later sharding by network or lineage authority.

## Compiler Commitments

| Choice | Commitment | Reason |
|---|---|---|
| Keep `TaskCompiler` task-local | keep | It remains focused on task-local compilation. |
| Create new upper lowerer | accept | `ExecutionComposition` to task network mutation is a different level of graph lowering. |
| Add level-specific lowering contract | accept | The repo needs contracts named by graph level and domain ownership. |
| Add narrow inner compiler trait | accept | `planning::lowering::Lowerer` receives injectable task compilation behavior for tests and alternate implementations. |

Recommended narrow trait:

```rust
pub trait TaskDefinitionCompiler {
    fn compile_task_definition(
        &self,
        definition: &TaskDefinition,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskRecord, ApiError>;
}
```

`task::compiler::TaskCompiler` implements this trait. The trait compiles `TaskDefinition` into `CompiledTaskRecord` with a `CapabilityCatalog`.

## Module Shape

Add domain-first modules under `crates/meld-execution`.

```text
crates/meld-execution/src/planning/lowering.rs
crates/meld-execution/src/task_network.rs
crates/meld-execution/src/task_network/contracts.rs
crates/meld-execution/src/task_network/command.rs
crates/meld-execution/src/task_network/mutation.rs
crates/meld-execution/src/task_network/state.rs
crates/meld-execution/src/task_network/store.rs
crates/meld-execution/src/task_network/readiness.rs
crates/meld-execution/src/task_network/dispatch.rs
crates/meld-execution/src/task_network/outcome.rs
```

Module layout uses `task_network.rs` plus `task_network/*.rs` child files.

Keep existing modules in place:

- `task::compiler` remains the task-local compiler
- `task::package` remains workflow package compatibility lowering
- `task::executor` remains the task-local execution agent
- `task::runtime` remains task-local dispatch through capability invokers
- `task::events` remains the task event envelope surface

## Test Harness Shape

Add focused integration tests under `crates/meld-execution/tests`.

```text
crates/meld-execution/tests/composition_lowering.rs
crates/meld-execution/tests/task_network_command.rs
crates/meld-execution/tests/task_network_contracts.rs
crates/meld-execution/tests/task_network_store.rs
crates/meld-execution/tests/task_network_readiness.rs
crates/meld-execution/tests/task_network_dispatch.rs
crates/meld-execution/tests/task_network_outcome.rs
crates/meld-execution/tests/phase7_task_network_slice.rs
```

Add pinned fixtures under `crates/meld-execution/tests/fixtures/task_network`.

```text
mutation_set_v1.json
commit_record_v1.json
network_state_v1.json
dispatch_claim_v1.json
publication_v1.json
```

Add fuzz targets under `crates/meld-execution/fuzz`.

```text
fuzz_composition_lowering
fuzz_task_network_contracts
fuzz_task_network_command
fuzz_task_network_store_replay
fuzz_task_network_readiness
```

Fuzz targets exercise decode, identity derivation, pure lowering, command validation, mutation replay, and ready set computation with pure in memory inputs.

## Documentation Gates

Every new public module must have module Rustdoc that states owner, inputs, outputs, and explicit non ownership.

Every new public type, trait, enum, variant, public field, and public function must pass `#![deny(missing_docs)]`.

Public examples are required for these modules:

- `planning::lowering`
- `task_network::command`
- `task_network::mutation`
- `task_network::state`
- `task_network::dispatch`
- `task_network::outcome`

Examples compile through `cargo test -p meld-execution --doc`. Examples demonstrate contract construction with pure in memory values.

Inline comments are required for non-obvious ordering, idempotency, persistence, replay, and publication cursor rules. Inline comments explain invariants and sequencing rules.

Comment consistency review must scan the new files for placeholder phrasing before phase exit.

```sh
! rg -n "TODO|TBD|variant for this execution contract|owned by this execution contract|Execution helper for" \
  crates/meld-execution/src/planning/lowering.rs \
  crates/meld-execution/src/task_network.rs \
  crates/meld-execution/src/task_network
```

## Contract Shape

The first slice defines these records.

```text
pub mod planning::lowering {
    pub struct Request {
        pub request_id: String,
        pub network_id: String,
        pub composition: ExecutionComposition,
        pub idempotency_key: String,
    }

    pub struct Plan {
        pub request_id: String,
        pub network_id: String,
        pub composition_id: String,
        pub goal_id: String,
        pub method_id: String,
        pub mutations: task_network::mutation::Set,
        pub diagnostics: Vec<Diagnostic>,
    }
}
```

```text
pub mod task_network::command {
    pub struct Request {
        pub command_id: String,
        pub network_id: String,
        pub base_revision: u64,
        pub base_state_hash: String,
        pub read_preconditions: Vec<mutation::ReadPrecondition>,
        pub command: Command,
    }

    pub enum Command {
        ApplyMutationSet(mutation::Set),
        ClaimReadyTask(dispatch::Request),
        RecordTaskOutcome(dispatch::Outcome),
        MarkPublication(outcome::Publication),
    }

    pub enum Response {
        Accepted { revision: u64, state_hash: String },
        Duplicate { revision: u64, state_hash: String },
        Rejected(mutation::Rejection),
    }
}
```

```text
pub mod task_network::mutation {
    pub struct Set {
        pub schema_version: u32,
        pub set_id: String,
        pub network_id: String,
        pub source_composition_id: String,
        pub idempotency_key: String,
        pub mutations: Vec<Mutation>,
        pub diagnostics: Vec<PlanningDiagnostic>,
    }

    pub enum Mutation {
        Inject(Inject),
    }

    pub struct Inject {
        pub mutation_id: String,
        pub task_node: state::TaskNode,
        pub incoming_edges: Vec<state::DependencyEdge>,
        pub lineage: state::TaskLineage,
    }

    pub struct CommitRequest {
        pub command_id: String,
        pub base_revision: u64,
        pub base_state_hash: String,
        pub read_preconditions: Vec<ReadPrecondition>,
        pub mutation_set: Set,
    }

    pub enum ReadPrecondition {
        RevisionIs(u64),
        StateHashIs(String),
        NodeExists(String),
        NodeAbsent(String),
        NodeStatusIs { task_instance_id: String, status: state::TaskStatus },
        EdgeExists { from: String, to: String },
        EdgeAbsent { from: String, to: String },
        ArtifactAvailable { task_instance_id: String, artifact_type_id: String },
        NoPath { from: String, to: String },
        ClaimCurrent { task_instance_id: String, claim_id: String, claim_revision: u64 },
        PublicationPending(String),
    }

    pub enum CommitResult {
        Committed(CommitRecord),
        Duplicate(CommitRecord),
        Rejected(Rejection),
    }

    pub enum Rejection {
        StaleBase { expected: u64, actual: u64 },
        StateHashMismatch { expected: String, actual: String },
        FailedPrecondition(ReadPrecondition),
        DuplicateCommand(String),
        InvalidGraph(String),
        InvalidLifecycleTransition(String),
        StaleClaim { claim_id: String },
        PublicationAlreadyMarked(String),
    }

    pub struct CommitRecord {
        pub commit_id: String,
        pub command_id: String,
        pub network_id: String,
        pub revision: u64,
        pub previous_state_hash: String,
        pub state_hash: String,
        pub mutation_set: Set,
    }
}
```

```text
pub mod task_network::state {
    pub struct NetworkState {
        pub network_id: String,
        pub revision: u64,
        pub state_hash: String,
        pub tasks: BTreeMap<String, TaskNode>,
        pub edges: Vec<DependencyEdge>,
        pub statuses: BTreeMap<String, TaskStatus>,
        pub artifact_availability: Vec<ArtifactAvailability>,
    }

    pub struct TaskNode {
        pub task_instance_id: String,
        pub lifecycle_epoch: u64,
        pub compiled_task: CompiledTaskRecord,
        pub init_payload: TaskInitializationPayload,
        pub lineage: TaskLineage,
    }

    pub enum TaskStatus {
        Pending,
        Running { claim_id: String },
        Succeeded { outcome_id: String },
        Failed { outcome_id: String, error: String },
        Cancelled { reason: String },
    }

    pub struct ReadySet {
        pub network_id: String,
        pub revision: u64,
        pub state_hash: String,
        pub task_instance_ids: Vec<String>,
        pub diagnostics: Vec<ReadinessDiagnostic>,
    }
}
```

Later slices add `Cancel`, `Relink`, `Preserve`, and `Prune`.

## Development Phases

| Phase | Goal | Dependencies | State |
|---|---|---|---|
| 0 | Scope lock and implementation assessment | execution planning and task runtime | Complete in this plan |
| 1 | Module scaffold and public boundary | Phase 0 | Planned |
| 2 | Inner compiler boundary | Phase 1 | Planned |
| 3 | Task network command and mutation contracts | Phase 1 | Planned |
| 4 | Task network state contracts | Phase 3 | Planned |
| 5 | Composition lowering contracts | Phase 3 | Planned |
| 6 | One operator lowering implementation | Phase 5 and Phase 2 | Planned |
| 7 | Command commit store and replay | Phase 4 and Phase 6 | Planned |
| 8 | Ready set over task nodes | Phase 7 | Planned |
| 9 | Dispatch claim and task runtime bridge | Phase 8 | Planned |
| 10 | Outcome publication handoff | Phase 9 | Planned |
| 11 | End to end flywheel slice test | Phase 10 | Planned |

## Phase 0 -- Scope Lock And Implementation Assessment

| Field | Value |
|---|---|
| Goal | Fix the first Phase 7 boundary around execution composition lowering, task network command acceptance, and mutation reduction. |
| Dependencies | Phase 6 planning runtime, task compiler, task runtime, task events |
| State | Complete in this plan |

| Order | Task | State |
|---|---|---|
| 1 | Confirm existing lowering starts from workflow package authoring. | Complete |
| 2 | Confirm existing `TaskCompiler` is task-local. | Complete |
| 3 | Confirm new lowering starts from `ExecutionComposition`. | Complete |
| 4 | Confirm task network command acceptance and mutation reduction are Phase 7 additions. | Complete |
| 5 | Confirm `task_network` module creation is part of this phase. | Complete |
| 6 | Confirm public names should be module-scoped internally and explicit at re-export boundaries. | Complete |

Exit criteria:

- This plan records the reuse boundary for existing task code.
- This plan records a level-specific compiler boundary for Phase 7.
- This plan names the exact first slice object flow.

Verification:

```sh
rg -n "pub struct ExecutionComposition|pub struct TaskCompiler|pub struct PreparedTaskRun|pub struct TaskExecutor|TaskExpansionCompiler" crates/meld-execution/src
test ! -e crates/meld-execution/src/task_network.rs
```

## Phase 1 -- Module Scaffold And Public Boundary

| Field | Value |
|---|---|
| Goal | Add task network and lowering module entries while preserving dispatch behavior. |
| Dependencies | Phase 0 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Add `planning/lowering.rs`. | Planned |
| 2 | Add `task_network.rs`. | Planned |
| 3 | Add `task_network/contracts.rs`. | Planned |
| 4 | Add `task_network/command.rs`. | Planned |
| 5 | Add `task_network/mutation.rs`. | Planned |
| 6 | Add `task_network/state.rs`. | Planned |
| 7 | Add `task_network/store.rs`. | Planned |
| 8 | Add `task_network/readiness.rs`. | Planned |
| 9 | Add `task_network/dispatch.rs`. | Planned |
| 10 | Add `task_network/outcome.rs`. | Planned |
| 11 | Re-export only stable contracts and runtime facades. | Planned |
| 12 | Add module Rustdoc examples for each public boundary. | Planned |
| 13 | Add source boundary scan test for module shape and forbidden imports. | Planned |
| 14 | Add `petgraph` to `meld-execution` for private task network validation helpers. | Planned |

Exit criteria:

- The crate exposes Phase 7 public boundaries.
- Existing task package and task runtime behavior is unchanged.
- Module layout follows parent files plus child files.
- Public docs compile.
- New public modules explain ownership and non ownership.

Verification:

```sh
cargo check -p meld-execution
cargo test -p meld-execution --doc
test ! -e crates/meld-execution/src/task_network/mod.rs
test ! -e crates/meld-execution/src/planning/lowering/mod.rs
! rg -n "TODO|TBD|variant for this execution contract|owned by this execution contract|Execution helper for" \
  crates/meld-execution/src/planning/lowering.rs \
  crates/meld-execution/src/task_network.rs \
  crates/meld-execution/src/task_network
```

## Phase 2 -- Inner Compiler Boundary

| Field | Value |
|---|---|
| Goal | Preserve the existing task-local compiler while giving the new lowerer a narrow injectable compiler contract. |
| Dependencies | Phase 1 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Add `TaskDefinitionCompiler` trait in `task::compiler`. | Planned |
| 2 | Implement the trait for `TaskCompiler`. | Planned |
| 3 | Keep `compile_task_definition` as the direct helper. | Planned |
| 4 | Add tests proving the trait and direct helper produce equal output. | Planned |
| 5 | Add source scan proving the trait surface is limited to `TaskDefinition`, `CapabilityCatalog`, and `CompiledTaskRecord`. | Planned |
| 6 | Add Rustdoc example showing trait use through `TaskCompiler`. | Planned |
| 7 | Add test proving workflow package lowering still compiles through the direct helper. | Planned |

Exit criteria:

- `TaskCompiler` remains task-local.
- The trait can compile `TaskDefinition` into `CompiledTaskRecord`.
- Task-local compilation stays limited to task definition and compiled task records.
- Existing task package tests still pass unchanged.

Verification:

```sh
cargo test -p meld-execution task::compiler
cargo test -p meld-execution task::package
cargo test -p meld-execution --doc
! rg -n "ExecutionComposition|task_network" crates/meld-execution/src/task/compiler.rs
```

## Phase 3 -- Task Network Command And Mutation Contracts

| Field | Value |
|---|---|
| Goal | Define command and append-only mutation records for task network graph and lifecycle changes. |
| Dependencies | Phase 1 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Define `task_network::command::Request`. | Planned |
| 2 | Define `task_network::command::Command`. | Planned |
| 3 | Define `task_network::command::Response`. | Planned |
| 4 | Define `task_network::mutation::Set`. | Planned |
| 5 | Define `task_network::mutation::Mutation`. | Planned |
| 6 | Define first variant `task_network::mutation::Inject`. | Planned |
| 7 | Define `CommitRequest`, `CommitResult`, `CommitRecord`, and `Rejection`. | Planned |
| 8 | Define `ReadPrecondition`, base revision, base state hash, and conflict rejections. | Planned |
| 9 | Add deterministic identity rules for command requests, mutation sets, inject records, commits, and state hashes. | Planned |
| 10 | Add serde round trip tests. | Planned |
| 11 | Add duplicate command behavior for commit idempotency. | Planned |
| 12 | Add pinned JSON fixtures for command request, mutation set, and commit record. | Planned |
| 13 | Add property tests for stable command and mutation set identity under repeated construction. | Planned |
| 14 | Add fuzz target for command contract decode and identity derivation. | Planned |
| 15 | Add source scan proving command and mutation contracts import only contract and identity dependencies. | Planned |

Exit criteria:

- One inject mutation can carry one task node and incoming dependency edges.
- Command request carries base revision, base state hash, and read preconditions.
- Command response distinguishes accepted, duplicate, conflict, and validation rejection outcomes.
- Contract JSON stays stable through pinned fixtures.
- Invalid or random JSON has panic-free contract decoding.

Verification:

```sh
cargo test -p meld-execution task_network_command
cargo test -p meld-execution task_network_mutation
cargo test -p meld-execution task_network_contracts
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
! rg -n "TaskExecutor|execute_task_to_completion|CapabilityInvocationPayload" \
  crates/meld-execution/src/task_network/mutation.rs \
  crates/meld-execution/src/task_network/contracts.rs
```

## Phase 4 -- Task Network State Contracts

| Field | Value |
|---|---|
| Goal | Define reduced task network state derived from mutation records and task events. |
| Dependencies | Phase 3 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Define `NetworkState`. | Planned |
| 2 | Define `TaskNode`. | Planned |
| 3 | Define `DependencyEdge`. | Planned |
| 4 | Define `TaskStatus`. | Planned |
| 5 | Define `ArtifactAvailability`. | Planned |
| 6 | Define `TaskLineage` with composition, goal, method, and step references. | Planned |
| 7 | Define `state_hash` derivation from canonical reduced state. | Planned |
| 8 | Define `ReadySet` as a derived query record. | Planned |
| 9 | Add serde round trip tests. | Planned |
| 10 | Add pinned JSON fixture for one reduced state. | Planned |
| 11 | Add property tests for deterministic task and edge ordering. | Planned |
| 12 | Add boundary tests proving reduced state carries task node records and artifact availability only. | Planned |
| 13 | Add source scan proving state contracts stay at the task network boundary. | Planned |

Exit criteria:

- Reduced state can represent one pending task node.
- Reduced state preserves lineage back to `ExecutionComposition`.
- Reduced state carries task network task nodes, dependency edges, statuses, and artifact availability.
- Ready set stays derived from reduced state.
- Reduced state serialization is deterministic.

Verification:

```sh
cargo test -p meld-execution task_network_state
! rg -n "TaskArtifactRepo|CapabilityInvocationRecord" crates/meld-execution/src/task_network/state.rs
```

## Phase 5 -- Composition Lowering Contracts

| Field | Value |
|---|---|
| Goal | Define the execution planning side contract that turns an execution composition into proposed task network mutation sets. |
| Dependencies | Phase 3 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Define `planning::lowering::Request`. | Planned |
| 2 | Define `planning::lowering::Plan`. | Planned |
| 3 | Define `planning::lowering::Diagnostic`. | Planned |
| 4 | Define `planning::lowering::Lowerer`. | Planned |
| 5 | Add public re-exports with explicit type names. | Planned |
| 6 | Add contract tests proving lowering output is a mutation proposal. | Planned |
| 7 | Add Rustdoc example showing request to create a mutation plan. | Planned |
| 8 | Add property tests for deterministic lowering diagnostics. | Planned |
| 9 | Add fuzz target for lowering request decode and diagnostic paths. | Planned |
| 10 | Add source scan proving lowering depends on planning contracts, task compiler contracts, and task network mutation contracts. | Planned |

Exit criteria:

- Lowering request accepts `ExecutionComposition`.
- Lowering plan emits `task_network::mutation::Set`.
- Lowering diagnostics are deterministic and serializable.
- Lowering has public examples that compile.
- Lowering output carries a mutation proposal for task network acceptance.

Verification:

```sh
cargo test -p meld-execution planning_lowering_contracts
cargo test -p meld-execution --doc
! rg -n "TaskExecutor|execute_task_to_completion|CapabilityInvocationPayload|CapabilityInvoker" crates/meld-execution/src/planning/lowering.rs
```

## Phase 6 -- One Operator Lowering Implementation

| Field | Value |
|---|---|
| Goal | Lower one resolved `StepKind::Op` into one inject mutation. |
| Dependencies | Phase 5 and Phase 2 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Select one operator resolution report from `ExecutionComposition`. | Planned |
| 2 | Return typed lowering diagnostics for unresolved operators. | Planned |
| 3 | Build a `TaskDefinition` for the selected operator. | Planned |
| 4 | Compile the task through `TaskDefinitionCompiler`. | Planned |
| 5 | Build `TaskInitializationPayload` for the task node. | Planned |
| 6 | Build one `task_network::state::TaskNode`. | Planned |
| 7 | Build one `task_network::mutation::Inject`. | Planned |
| 8 | Preserve `StepKind::Goal` as a deferred recursive planning diagnostic. | Planned |
| 9 | Add fixture using the existing `refresh_docs_v1` method. | Planned |
| 10 | Add diagnostic tests for unresolved operator, missing resolution, recursive goal step, and unsupported edge kind. | Planned |
| 11 | Add property tests proving same composition produces same mutation set identity. | Planned |
| 12 | Add boundary test proving lowering consumes the supplied `ExecutionComposition`. | Planned |

Exit criteria:

- One composed docs freshness result lowers into one inject mutation.
- Lowering preserves the selected method from the composition.
- Lowering emits graph mutation proposals.
- Unsupported lowering paths return typed diagnostics.
- Lowered task node preserves composition, goal, method, and step lineage.

Verification:

```sh
cargo test -p meld-execution composition_lowering
cargo test -p meld-execution planning_runtime
! rg -n "PlanningRuntime::new|plan_goal" crates/meld-execution/src/planning/lowering.rs
```

## Phase 7 -- Command Commit Store And Replay

| Field | Value |
|---|---|
| Goal | Persist accepted command records and reduce them into task network state. |
| Dependencies | Phase 4 and Phase 6 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Add in memory task network store for deterministic tests. | Planned |
| 2 | Add durable task network store over caller-provided `sled::Db`. | Planned |
| 3 | Add one monotonic journal revision sequence for commits, claims, outcomes, artifacts, and publications. | Planned |
| 4 | Persist command records by revision and command id. | Planned |
| 5 | Persist command responses by command id. | Planned |
| 6 | Replay duplicate command outcomes. | Planned |
| 7 | Validate base revision, base state hash, and read preconditions at command time. | Planned |
| 8 | Reduce accepted inject mutations into `NetworkState`. | Planned |
| 9 | Compute and persist latest state hash after every accepted journal record. | Planned |
| 10 | Use `petgraph` transient graphs for acyclicity and endpoint validation. | Planned |
| 11 | Add reopen tests for durable state. | Planned |
| 12 | Add corrupt record tests for command log, mutation log, journal indexes, and command outcome trees. | Planned |
| 13 | Add outside workspace storage path test. | Planned |
| 14 | Add property tests for replay determinism over accepted command order. | Planned |
| 15 | Add property tests for interleaved proposal conflicts and successful rebases. | Planned |
| 16 | Add fuzz target for command log replay and conflict stability. | Planned |

Exit criteria:

- Accepted inject mutation survives reopen.
- Duplicate command id returns prior result.
- Reduced state and state hash are deterministic by revision.
- Acyclic graph validation runs before accepted structural commits.
- Proposal conflicts return typed results.
- Corrupt durable records return typed errors.
- Store can open on a caller supplied path outside the workspace.

Verification:

```sh
cargo test -p meld-execution task_network_store
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
```

## Phase 8 -- Ready Set Over Task Nodes

| Field | Value |
|---|---|
| Goal | Compute ready task nodes from reduced task network state. |
| Dependencies | Phase 7 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Define `task_network::state::ReadySet`. | Planned |
| 2 | Implement ready set for source nodes. | Planned |
| 3 | Implement ordering edge satisfaction from upstream success. | Planned |
| 4 | Implement data flow edge satisfaction from artifact availability. | Planned |
| 5 | Represent conditional edge guard evaluation as blocked diagnostics for this slice. | Planned |
| 6 | Build transient `petgraph` graphs for active dependency validation. | Planned |
| 7 | Add tests matching task-local readiness semantics at the upper level. | Planned |
| 8 | Add property tests proving completed, running, failed, and cancelled tasks stay absent from ready sets. | Planned |
| 9 | Add property tests proving repeated input state returns the same ordered ready set. | Planned |
| 10 | Add fuzz target for ready set computation over bounded generated graphs. | Planned |
| 11 | Add cycle and missing edge endpoint tests. | Planned |

Exit criteria:

- One injected task with satisfied dependencies becomes ready.
- Completed, running, failed, and cancelled tasks stay absent from ready sets.
- Ready set computation is deterministic.
- Cycles and missing dependencies produce typed diagnostics.

Verification:

```sh
cargo test -p meld-execution task_network_readiness
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
```

## Phase 9 -- Dispatch Claim And Task Runtime Bridge

| Field | Value |
|---|---|
| Goal | Claim one ready task before invoking the existing task runtime. |
| Dependencies | Phase 8 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Define `task_network::dispatch::Request`. | Planned |
| 2 | Define `task_network::dispatch::Claim`. | Planned |
| 3 | Define `task_network::dispatch::Outcome`. | Planned |
| 4 | Accept claim through `task_network::command::Command::ClaimReadyTask`. | Planned |
| 5 | Persist dispatch claim before task execution. | Planned |
| 6 | Append claim records through the same monotonic journal revision stream. | Planned |
| 7 | Include task lifecycle epoch, claim id, and claim revision in the claim record. | Planned |
| 8 | Build `TaskExecutor` from the claimed `TaskNode`. | Planned |
| 9 | Call `execute_task_to_completion` for the claimed task through in memory test invokers. | Planned |
| 10 | Accept task runtime result through `task_network::command::Command::RecordTaskOutcome`. | Planned |
| 11 | Reject stale task outcomes whose claim id, claim revision, or lifecycle epoch is no longer current. | Planned |
| 12 | Reduce task events into task network status and artifact availability. | Planned |
| 13 | Add failure path test that records failed status. | Planned |
| 14 | Add duplicate claim test proving claim idempotency and conflict behavior. | Planned |
| 15 | Add stale claim outcome test proving fenced outcomes cannot advance state. | Planned |
| 16 | Add restart test proving a claimed running task has explicit recovery state. | Planned |
| 17 | Add event ordering test for requested, started, progressed, succeeded, and failed events. | Planned |
| 18 | Add in memory provider-free fixture using test invokers only. | Planned |

Exit criteria:

- Ready task is marked running before task runtime invocation.
- Successful task runtime result marks task succeeded.
- Failed task runtime result marks task failed.
- Dispatch is idempotent before external capability work.
- Stale claim outcomes are rejected before state reduction.
- Dispatch tests use in memory invokers.

Verification:

```sh
cargo test -p meld-execution task_network_dispatch
! rg -n "OpenAI|Anthropic|Ollama|reqwest" crates/meld-execution/tests/task_network_dispatch.rs
```

## Phase 10 -- Outcome Publication Handoff

| Field | Value |
|---|---|
| Goal | Persist the handoff from task outcome to event publication. |
| Dependencies | Phase 9 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Define `task_network::outcome::Publication`. | Planned |
| 2 | Define `task_network::outcome::PublicationState`. | Planned |
| 3 | Record pending publication outbox entry after task completion. | Planned |
| 4 | Publish task events through existing execution task envelope helpers. | Planned |
| 5 | Mark publication complete through `task_network::command::Command::MarkPublication` after successful append. | Planned |
| 6 | Preserve replay cursor for failed publication. | Planned |
| 7 | Append publication state records through the same monotonic journal revision stream. | Planned |
| 8 | Add tests proving publication can resume after store reopen. | Planned |
| 9 | Add duplicate publication test proving published records replay idempotently. | Planned |
| 10 | Add crash before publication mark test. | Planned |
| 11 | Add crash after publication mark test. | Planned |
| 12 | Add corrupt publication record test. | Planned |
| 13 | Add fixture assertion for canonical task event envelope shape. | Planned |
| 14 | Add world model reducer compatibility assertion using event domain, stream, objects, and relations. | Planned |

Exit criteria:

- Task outcome has durable handoff coverage between task completion and event publication.
- Publication outbox entry exists before any external publication attempt.
- Published event payload remains compatible with existing task event envelopes.
- Publication replay is idempotent.
- Publication tests cover append failure and replay.
- Event graph objects and relations remain stable.

Verification:

```sh
cargo test -p meld-execution task_network_outcome
cargo test -p meld-execution task::events
```

## Phase 11 -- End To End Flywheel Slice Test

| Field | Value |
|---|---|
| Goal | Prove the Phase 7 bridge from one execution composition to one published task outcome event. |
| Dependencies | Phase 10 |
| State | Planned |

| Order | Task | State |
|---|---|---|
| 1 | Create one docs freshness goal and projected world state fixture. | Planned |
| 2 | Run `PlanningRuntime` to produce `PlanningResult::Composed`. | Planned |
| 3 | Lower the execution composition into task network mutation set. | Planned |
| 4 | Submit the mutation set through the task network command boundary. | Planned |
| 5 | Compute ready set. | Planned |
| 6 | Dispatch one ready task through test capability invoker. | Planned |
| 7 | Publish task outcome event. | Planned |
| 8 | Assert world model compatible execution event shape. | Planned |
| 9 | Reopen durable stores between commit and dispatch. | Planned |
| 10 | Reopen durable stores between dispatch and publication. | Planned |
| 11 | Assert idempotent event publication after replay. | Planned |
| 12 | Assert goal and composition lineage appear in task network records. | Planned |

Exit criteria:

- One `docs_freshness` execution composition produces one accepted task node.
- One ready task dispatches through the existing task runtime.
- One outcome event is available for world model reduction.
- The slice survives process boundary simulation through store reopen.
- Replayed publication is idempotent.

Verification:

```sh
cargo test -p meld-execution phase7_task_network_slice
```

## Next And Future Slices

The next global slice after this Phase 7 plan was [Phase 8 Expanded Execution Slice](PHASE8.md), which is now implemented.

Later slices extend Phase 8 with:

- recursive sub-goal planning
- conditional edge guard evaluation
- observation wait task generation
- cancel mutation execution
- relink mutation execution
- preserve mutation execution
- prune mutation execution
- plan diffing
- affected subtree selection
- switching cost model
- task equivalence across goals
- shared task reuse across goals
- resource capacity model
- external provider recovery after process restart
- synthesized capability catalog persistence
- dynamic agent creation tasks
- workflow executor migration
- Tokio runtime facade for proposal queues, worker pools, dispatch limits, publication retries, cancellation, and shutdown

## Quality Bar

Required verification families:

- serde round trip tests for all public task network contracts
- deterministic identity tests for mutation sets, commit records, task nodes, dispatch claims, and publication records
- duplicate command tests for mutation commit and dispatch claim
- duplicate outcome tests for task runtime result commands
- stale claim outcome tests for fenced dispatch
- proposal precondition tests for stale base revision and stale state hash conflicts
- global journal order tests for commits, claims, outcomes, artifacts, and publications
- reopen tests for durable task network store and publication handoff
- publication outbox crash tests before and after publication mark
- ready set tests for pending, running, succeeded, failed, and cancelled states
- transient `petgraph` validation tests for acyclicity and endpoint integrity
- boundary tests proving `TaskCompiler` stays task-local
- lowering tests proving `ExecutionComposition` is consumed as the selected planning artifact
- dispatch tests proving task events reduce into task network state
- event tests proving publication uses existing canonical task event envelopes
- source scan proving module layout uses parent files and child files
- source scan proving `planning::lowering` uses planning contracts, task compiler contracts, and task network mutation contracts
- public Rustdoc examples compile through doc tests
- placeholder and boilerplate comment scans for new Phase 7 files
- pinned JSON fixtures for every public task network record family
- fuzz targets for pure contract, lowering, replay, and readiness paths
- corrupt durable record tests for every sled tree introduced in Phase 7
- outside workspace storage path tests for every durable store introduced in Phase 7
- in memory invoker fixtures for Phase 7 dispatch tests

Required verification commands:

```sh
cargo fmt --check
cargo check -p meld-execution --all-targets
cargo test -p meld-execution --all-targets
cargo test -p meld-execution --doc
cargo clippy -p meld-execution --all-targets -- -D warnings
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
test ! -e crates/meld-execution/src/task_network/mod.rs
test ! -e crates/meld-execution/src/planning/lowering/mod.rs
! rg -n "CapabilityInvocationPayload|execute_task_to_completion|TaskExecutor" crates/meld-execution/src/planning/lowering.rs
! rg -n "TODO|TBD|variant for this execution contract|owned by this execution contract|Execution helper for" \
  crates/meld-execution/src/planning/lowering.rs \
  crates/meld-execution/src/task_network.rs \
  crates/meld-execution/src/task_network
```

High cost local or scheduled verification:

```sh
cargo llvm-cov -p meld-execution --all-targets --fail-under-lines 90
cargo mutants -p meld-execution --timeout 60 \
  --file crates/meld-execution/src/planning/lowering.rs \
  --file crates/meld-execution/src/task_network.rs \
  --file crates/meld-execution/src/task_network/contracts.rs \
  --file crates/meld-execution/src/task_network/command.rs \
  --file crates/meld-execution/src/task_network/mutation.rs \
  --file crates/meld-execution/src/task_network/state.rs \
  --file crates/meld-execution/src/task_network/store.rs \
  --file crates/meld-execution/src/task_network/readiness.rs \
  --file crates/meld-execution/src/task_network/dispatch.rs \
  --file crates/meld-execution/src/task_network/outcome.rs \
  -- --all-targets
cd crates/meld-execution
cargo +nightly fuzz run fuzz_composition_lowering -- -max_total_time=60
cargo +nightly fuzz run fuzz_task_network_contracts -- -max_total_time=60
cargo +nightly fuzz run fuzz_task_network_command -- -max_total_time=60
cargo +nightly fuzz run fuzz_task_network_store_replay -- -max_total_time=60
cargo +nightly fuzz run fuzz_task_network_readiness -- -max_total_time=60
```

## Acceptance Criteria

Phase 7 is complete when all statements are true:

- `ExecutionComposition` lowers into a deterministic `task_network::mutation::Set`
- unresolved operators produce typed lowering diagnostics
- one inject mutation commits to durable task network storage
- all graph and lifecycle writes pass through task network commands
- duplicate commit commands replay prior outcomes
- stale proposals return typed conflict outcomes
- accepted graph changes share one monotonic journal order
- committed network state has a deterministic state hash
- transient graph validation preserves an acyclic active graph
- reduced task network state survives reopen
- ready set computation over task nodes is deterministic
- one ready task is claimed before dispatch
- claimed task executes through the existing task runtime
- stale claim outcomes cannot advance task status
- task events reduce into task network status
- task outcome publication has a durable handoff record
- publication outbox state survives replay before and after publication mark
- published event shape remains compatible with existing execution task envelopes
- `TaskCompiler` remains task-local
- module layout uses parent files and child files
