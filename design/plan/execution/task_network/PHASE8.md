# Execution Task Network Phase 8 Expanded Execution Slice

Date: 2026-06-04
Status: ready after hardening
Scope: multi node execution composition lowering, task init materialization, real task dispatch, and replay

## Purpose

Phase 8 matures execution before sensory work.

The slice turns one execution composition into a real task network subgraph. It proves that the task network is not only a one node bridge. It commits a multi node graph atomically, computes ready sets over that graph, materializes task initialization payloads from static seeds and upstream artifacts, dispatches claimed tasks through the task runtime, records outcomes, and survives reopen.

Sensory remains deferred until execution can run and replay a real multi node graph.

## Entry Gate

Phase 8 starts only after the dedicated hardening task closes the current quality gaps.

Required hardening gates:

- `cargo fmt --check`
- `cargo check -p meld-execution --all-targets`
- `cargo test -p meld-execution --all-targets`
- `cargo test -p meld-execution --doc`
- `cargo clippy -p meld-execution --all-targets -- -D warnings`
- `cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml`
- task network store property tests pass without sled reopen lock failures
- mutation testing findings are reviewed and either fixed or recorded with rationale

## Design Inputs

Declarative architecture lives in:

- [Task Network](../../../cognitive_architecture/execution/task_network.md)
- [Task Initialization](../../../cognitive_architecture/execution/task_initialization.md)
- [Planning Pipeline](../../../cognitive_architecture/execution/planning/planning_pipeline.md)

The key design commitments are:

- The task network graph is the executable plan.
- Task initialization payloads are materialized task run envelopes, not the semantic authority.
- Artifact content remains an open JSON wire shape.
- Artifact type and schema version carry semantic contract identity.
- Data flow tasks are normal tasks with deferred init materialization.
- Lowering is all or nothing for executable operator steps in this slice.

## Scope

In scope:

- Lower every resolved operator step in one execution composition.
- Emit one task node per operator step.
- Preserve ordering and data flow edges between lowered task nodes.
- Commit the full mutation set atomically through the existing command boundary.
- Reject the whole lowering result when any executable operator step cannot be lowered.
- Preserve recursive goal steps as deferred diagnostics.
- Preserve conditional edges as deferred diagnostics.
- Define task init source records for static seeds and upstream artifacts.
- Materialize final `TaskInitializationPayload` before dispatch.
- Validate the materialized payload through existing task initialization validation.
- Dispatch claimed tasks through the existing task runtime with in memory test invokers.
- Record real task runtime outcomes through task network commands.
- Prove replay across graph commit, claims, outcomes, and publication marks.

Out of scope:

- Sensory runtime.
- Recursive sub-goal lowering.
- Conditional guard evaluation.
- Observation wait task generation.
- Cancel, relink, preserve, and prune mutations.
- Task equivalence across goals.
- Shared task reuse across goals.
- Plan diffing.
- Switching cost model.
- Provider process recovery.
- Workflow executor migration.

## All Or Nothing Lowering

Phase 8 lowering treats one execution composition as one atomic proposal.

The lowerer must inspect every `StepKind::Op` step. If any operator step lacks a resolved operator report, names a missing capability contract, or cannot compile into a task node, the lowerer returns diagnostics and no mutation set with executable graph changes.

The task network commit remains atomic. A command either accepts the full graph mutation set or rejects the proposal without mutating state.

This keeps partial graphs out of the authoritative network while the system lacks cancel, relink, preserve, and prune.

## Task Init Source Model

`TaskInitializationPayload` remains the final payload handed to the task executor.

Phase 8 adds an earlier task network owned source model for required init slots.

Source records:

```text
StaticSeed
  init_slot_id
  artifact_type_id
  schema_version
  content

UpstreamArtifact
  init_slot_id
  artifact_type_id
  schema_version
  upstream_task_instance_id
  upstream_artifact_type_id
```

Rules:

- Every required init slot has exactly one source.
- Static seed sources are materialized at task node injection.
- Upstream artifact sources are materialized after dependency satisfaction.
- Data flow materialization selects one matching artifact from the named upstream task.
- Matching requires task identity, artifact type, and schema version.
- Ambiguous or missing artifacts block dispatch with a typed diagnostic.
- The materialized payload must pass `validate_task_initialization`.

The payload content remains JSON. Phase 8 validates envelope fields and source resolution. Fuller artifact schema and semantic validation are later hardening.

## Execution Flow

The target flow:

```text
ExecutionComposition
-> CompositionLoweringRequest
-> CompositionLoweringPlan
-> TaskNetworkMutationSet with many Inject records
-> TaskNetworkCommand ApplyMutationSet
-> NetworkState with task nodes and dependency edges
-> ReadySet with independent source tasks
-> ClaimReadyTask
-> materialize TaskInitializationPayload
-> build TaskExecutor
-> execute task runtime with test invokers
-> RecordTaskOutcome
-> downstream ReadySet update
-> MarkPublication
-> reopen and replay identical state
```

## Implementation Phases

| Phase | Goal | State |
|---|---|---|
| 0 | Hardening gate | Required before start |
| 1 | Task init source contracts | Planned |
| 2 | All operator lowering | Planned |
| 3 | Edge preservation and graph commit | Planned |
| 4 | Init payload materialization | Planned |
| 5 | Real task runtime bridge | Planned |
| 6 | Expanded graph replay test | Planned |

## Phase 1 -- Task Init Source Contracts

Goal: Define the task network source plan for task init slots.

Tasks:

- Add source records for static seeds and upstream artifacts.
- Attach source records to task nodes or inject mutations.
- Keep `TaskInitializationPayload` as the materialized dispatch payload.
- Add contract tests for serde round trips and deterministic identity.
- Add validation that every required init slot has one source.
- Add validation that source artifact type and schema version match the compiled task init slot.

Exit criteria:

- Source task nodes can materialize payloads without upstream artifacts.
- Data flow task nodes carry deferred source records.
- Invalid or duplicate sources reject before commit.

## Phase 2 -- All Operator Lowering

Goal: Lower every executable operator step from one execution composition.

Tasks:

- Iterate all `StepKind::Op` steps instead of selecting the first.
- Resolve every operator report.
- Compile one `TaskDefinition` per operator step.
- Emit one `Inject` mutation per compiled task node.
- Preserve deterministic task ids, run ids, lineage, and mutation ordering.
- Return all-or-nothing diagnostics when any operator step fails.

Exit criteria:

- A three operator composition lowers into three inject mutations.
- One unresolved operator yields diagnostics and no executable graph mutation.
- Recursive goal steps remain deferred diagnostics.

## Phase 3 -- Edge Preservation And Graph Commit

Goal: Commit real task graph structure through existing task network commands.

Tasks:

- Lower every ordering edge between known operator steps.
- Lower every data flow edge between known operator steps.
- Reject edges that reference missing executable endpoints.
- Keep conditional edges deferred with typed diagnostics.
- Commit the full graph mutation set through `ApplyMutationSet`.
- Prove endpoint validation and cycle validation run before commit.

Exit criteria:

- Independent source tasks appear together in the ready set.
- Downstream join tasks wait for all incoming dependencies.
- Invalid graph proposals leave network state unchanged.

## Phase 4 -- Init Payload Materialization

Goal: Build final task init payloads immediately before dispatch.

Tasks:

- Add materialization over reduced network state and task node source records.
- Materialize static seed sources into `InitArtifactValue`.
- Materialize upstream artifact sources from accepted outcome artifacts.
- Reject missing, ambiguous, mismatched, or stale artifacts with typed diagnostics.
- Preserve source provenance in the materialization result.
- Validate the final payload against the compiled task.

Exit criteria:

- Source task payloads validate before execution.
- Data flow task payloads validate only after upstream artifacts exist.
- A downstream task cannot be claimed or executed with incomplete init materialization.

## Phase 5 -- Real Task Runtime Bridge

Goal: Replace synthetic outcomes in the expanded slice with task runtime output.

Tasks:

- Build a task executor from a fenced claim and materialized init payload.
- Run the task through existing task runtime helpers using in memory test invokers.
- Convert task runtime success into `RecordTaskOutcome`.
- Convert task runtime failure into `RecordTaskOutcome`.
- Preserve task events and artifact availability in the outcome.
- Reject stale outcomes through existing claim fencing.

Exit criteria:

- Claimed tasks execute through the task runtime.
- Outcome artifacts can unblock downstream data flow tasks.
- Failure status and error text reduce into task network state.

## Phase 6 -- Expanded Graph Replay Test

Goal: Prove the full expanded execution slice.

Fixture graph:

```text
prepare_metadata
collect_context
write_summary

prepare_metadata -> write_summary
collect_context -> write_summary
```

Expected behavior:

- Commit creates three task nodes and two dependency edges.
- `prepare_metadata` and `collect_context` are ready together.
- `write_summary` is blocked until both upstream tasks succeed.
- Upstream outcomes materialize the downstream init payload.
- `write_summary` executes through the task runtime.
- Publication mark survives reopen.
- Replayed state hash matches the pre-reopen state hash.

Verification:

```sh
cargo test -p meld-execution --test composition_lowering
cargo test -p meld-execution --test task_network_command
cargo test -p meld-execution --test task_network_readiness
cargo test -p meld-execution --test task_network_dispatch
cargo test -p meld-execution --test task_network_execution_bridge
cargo test -p meld-execution --test task_network_store
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
```

## Acceptance Criteria

Phase 8 is complete when all statements are true:

- One execution composition lowers every resolved operator step.
- Lowering is all or nothing for executable operator steps.
- One mutation set can contain many inject mutations.
- Ordering and data flow edges preserve the composition graph.
- Static seed tasks materialize init payloads before dispatch.
- Data flow tasks materialize init payloads from upstream artifacts.
- Required init slots are never satisfied by artifact type alone without schema version.
- Ready set behavior proves parallel source tasks and downstream joins.
- Claimed tasks run through the existing task runtime.
- Real task runtime outcomes unblock downstream tasks.
- The expanded slice survives reopen after commit, claim, outcome, and publication.
- Sensory runtime remains deferred.
