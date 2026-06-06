# Execution Task Network Phase 8 Expanded Execution Slice

Date: 2026-06-04
Status: implemented
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

Hardening evidence:

- 2026-06-04 command gates passed for format, check, tests, doc tests, clippy, and fuzz crate check.
- 2026-06-04 task network store tests passed under normal parallel execution without sled reopen lock failures.
- 2026-06-04 focused task network mutation run passed with 38 caught mutants, 13 unviable mutants, and zero survivors.

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
| 0 | Hardening gate | Complete |
| 1 | Task init source contracts | Complete |
| 2 | All operator lowering | Complete |
| 3 | Edge preservation and graph commit | Complete |
| 4 | Init payload materialization | Complete |
| 5 | Real task runtime bridge | Complete |
| 6 | Expanded graph replay test | Complete |

## Implementation Progress

Completed on 2026-06-05.

Implemented code paths:

- Added task network owned init source records for static seeds and upstream artifacts.
- Added source validation before task graph commit.
- Added materialization over reduced network state into `TaskInitializationPayload`.
- Updated dispatch to build `TaskExecutor` from a fenced claim plus materialized init payload.
- Updated runtime bridge helpers to convert task runtime success and failure into `RecordTaskOutcome`.
- Updated composition lowering to lower every resolved operator step in deterministic order.
- Preserved ordering and data flow edges between lowered operator steps.
- Kept recursive goal steps and conditional edges as deferred diagnostics.
- Preserved all or nothing lowering when an executable operator cannot be lowered.
- Updated task network fixture contracts and fuzz targets for source records.

Implemented tests:

- `composition_lowering` covers multi operator lowering, all or nothing rejection, missing capability contracts, data flow source planning, static seed planning, conditional edge deferral, endpoint rejection, and repeatable mutation identity.
- `task_network_command` covers source validation rejection, duplicate source rejection, contract mismatch rejection, endpoint rejection, and cycle rejection before state mutation.
- `task_network_readiness` covers parallel source tasks, downstream joins, artifact availability, and schema aware data flow readiness.
- `task_network_initialization` covers static seed materialization, upstream artifact materialization, missing artifacts, ambiguous artifacts, stale artifacts, and final payload validation.
- `task_network_dispatch` covers materialization blocking before claim and executor construction from materialized init payloads.
- `task_network_execution_bridge` covers the three node expanded graph, real task runtime execution, runtime outcome conversion, downstream unblocking, publication marking, reopen, replayed state hash, and failure outcome conversion.
- `task_network_contracts` fixtures now round trip source records and publication payloads with artifact records.

Verification completed on 2026-06-05:

```sh
cargo fmt --check
cargo check -p meld-execution --all-targets
cargo test -p meld-execution --all-targets
cargo test -p meld-execution --doc
cargo clippy -p meld-execution --all-targets -- -D warnings
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
cargo mutants -p meld-execution --timeout 60 --file crates/meld-execution/src/planning/lowering.rs --file crates/meld-execution/src/task_network.rs --file crates/meld-execution/src/task_network/contracts.rs --file crates/meld-execution/src/task_network/command.rs --file crates/meld-execution/src/task_network/mutation.rs --file crates/meld-execution/src/task_network/state.rs --file crates/meld-execution/src/task_network/store.rs --file crates/meld-execution/src/task_network/readiness.rs --file crates/meld-execution/src/task_network/dispatch.rs --file crates/meld-execution/src/task_network/outcome.rs -- --all-targets
```

Verification evidence:

- Format check passed.
- All target check passed.
- All target tests passed.
- Doc tests passed.
- Clippy passed with warnings denied.
- Fuzz crate check passed.
- Focused task network mutation run passed with 86 mutants tested, 66 caught mutants, 20 unviable mutants, and zero missed mutants.

## Implementation Gate Standard

Each implementation phase must add or update tests that prove its exit criteria before the phase is considered complete.

Required command gate after each implementation phase:

```sh
cargo fmt --check
cargo check -p meld-execution --all-targets
cargo test -p meld-execution --all-targets
cargo test -p meld-execution --doc
cargo clippy -p meld-execution --all-targets -- -D warnings
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
```

Focused mutation testing remains a high cost verification gate at the end of Phase 6, or earlier when a phase changes shared reducer or lowering behavior.

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

Verification:

- Update `task_network_contracts` fixtures to round trip source records.
- Add deterministic identity tests proving equivalent source records produce stable task node and mutation set identity.
- Add `task_network_command` tests proving missing, duplicate, and contract mismatched sources reject before commit.
- Extend `fuzz_task_network_contracts` so decoded source records preserve required envelope fields.

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

Verification:

- Update `composition_lowering` so a three operator fixture emits three inject mutations in deterministic order.
- Add a test where one unresolved operator among several operators returns diagnostics and no executable graph mutation.
- Add a missing capability contract test that proves all operator lowering is rejected as one proposal.
- Add a repeatability test proving the same composition produces the same mutation set identity.

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

Verification:

- Update `composition_lowering` to prove ordering and data flow edges lower between all known operator steps.
- Update `task_network_command` to prove endpoint validation and cycle validation reject before state mutation.
- Update `task_network_readiness` to prove independent source tasks are ready together.
- Add a downstream join readiness test proving all incoming dependencies must succeed or provide required artifacts before dispatch.
- Add a conditional edge diagnostic test proving guarded edges remain deferred and do not become executable dependencies.

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

Verification:

- Add `task_network_initialization` tests for static seed materialization into `InitArtifactValue`.
- Add upstream artifact materialization tests for task identity, artifact type, and schema version matching.
- Add diagnostics tests for missing, ambiguous, mismatched, and stale upstream artifacts.
- Add a validation test proving the materialized `TaskInitializationPayload` passes `validate_task_initialization`.
- Add dispatch blocking coverage proving incomplete materialization prevents claim or executor creation.
- Extend `fuzz_task_network_readiness` or add a materialization fuzz target for source resolution invariants.

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

Verification:

- Update `task_network_dispatch` so executor construction consumes a materialized init payload, not an eager node payload.
- Update `task_network_execution_bridge` to run claimed tasks through task runtime helpers with in memory test invokers.
- Add success conversion coverage from task runtime output to `RecordTaskOutcome`.
- Add failure conversion coverage that preserves status, error text, and emitted task events.
- Add stale claim coverage proving runtime outcomes cannot advance task network state after claim fencing fails.

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

Implementation gate tests:

- Add a three node fixture in `task_network_execution_bridge` using `prepare_metadata`, `collect_context`, and `write_summary`.
- Prove source tasks are claimed and executed independently before the join task.
- Prove upstream artifacts materialize the `write_summary` init payload only after both upstream outcomes are accepted.
- Prove the final outcome creates publication state and `MarkPublication` survives reopen.
- Prove replay reconstructs the same task statuses, artifacts, publications, journal length, and state hash.
- Add or update store replay coverage for multi node commit, claim, outcome, and publication records.

Verification:

```sh
cargo test -p meld-execution --test composition_lowering
cargo test -p meld-execution --test task_network_command
cargo test -p meld-execution --test task_network_readiness
cargo test -p meld-execution --test task_network_dispatch
cargo test -p meld-execution --test task_network_execution_bridge
cargo test -p meld-execution --test task_network_store
cargo check --manifest-path crates/meld-execution/fuzz/Cargo.toml
cargo mutants -p meld-execution --timeout 60 --file crates/meld-execution/src/planning/lowering.rs --file crates/meld-execution/src/task_network.rs --file crates/meld-execution/src/task_network/contracts.rs --file crates/meld-execution/src/task_network/command.rs --file crates/meld-execution/src/task_network/mutation.rs --file crates/meld-execution/src/task_network/state.rs --file crates/meld-execution/src/task_network/store.rs --file crates/meld-execution/src/task_network/readiness.rs --file crates/meld-execution/src/task_network/dispatch.rs --file crates/meld-execution/src/task_network/outcome.rs -- --all-targets
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
