# Capability And Task Implementation Plan

Date: 2026-04-04
Status: proposed
Scope: ordered implementation of capability contracts, task runtime, first-slice capabilities, docs-writer task package, and workflow convergence onto task execution

## Overview

This plan defines the next implementation order after the ownership refactor.

The objective is to build the first real `capability` and `task` layer on top of the refactored runtime without reintroducing mixed ownership.

Planned outcome:

- `src/capability` defines the published contract model and catalog surface
- `src/task` owns compiled task records, artifact repo persistence, task initialization, and task-local execution
- the first high-value capability set is implemented against structured-data contracts
- docs-writer runs as a task package with durable artifact-driven dependency truth
- legacy workflow entry paths become compatibility triggers into task execution
- observability and test coverage are strong enough to make later control and repair work safe

## Related Specs

- [Capability And Task Design](README.md)
- [Capability Model](capability/README.md)
- [Capabilities By Domain](capability/by_domain.md)
- [Task Design](task/README.md)
- [Docs Writer Package](task/docs_writer_package.md)
- [Task Expansion Plan](task/task_expansion_plan.md)
- [Task Control Boundary](task_control_boundary.md)
- [Domain Architecture](domain_architecture.md)
- [Capability Refactor Completion](../completed/capability_refactor/README.md)
- [Control Design](../control/README.md)
- [Task Network](../control/task_network.md)
- [Complex Change Workflow Governance](../../governance/complex_change_workflow.md)
- [Commenting Policy](../../governance/commenting_policy.md)

## Guiding Rules

- keep all task to capability traffic structured-data-only
- keep artifact persistence task-owned
- keep task-local progression task-owned
- keep task-network ordering and repair intent control-owned
- preserve runnable workflow-triggered behavior until task-triggered paths fully replace it
- expand characterization, unit, and integration coverage before deleting compatibility paths
- require comments on new public contracts and non-obvious orchestration code using Rust idiomatic comment practice

## CLI Path Default Exception List

Project direction remains path-first targeting.
Current command surfaces that still include non-default path behavior:

- `meld context generate` accepts `--node` as an alternate selector
- `meld context regenerate` accepts `--node` as an alternate selector
- `meld context get` accepts `--node` as an alternate selector
- `meld workspace delete` accepts `--node` as an alternate selector
- `meld workspace restore` accepts `--node` as an alternate selector

This plan does not expand non-default selector behavior.

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 0 | Baseline lock and readiness gates | None | complete |
| 1 | Capability contract core and catalog | Phase 0 | complete |
| 2 | Task records, artifact repo, and compiler | Phase 0 and Phase 1 | complete |
| 3 | Task executor and invocation payload assembly | Phase 1 and Phase 2 | complete |
| 4 | First-slice capability implementation | Phase 1 through Phase 3 | complete |
| 5 | Docs-writer task package and task DAG execution | Phase 2 through Phase 4 | complete |
| 6 | Workflow convergence onto task execution | Phase 3 through Phase 5 | complete |
| 7 | Boundary seal, workflow retirement, and readiness signoff | Phase 0 through Phase 6 | proposed |

---

### Phase 0 - Baseline lock and readiness gates

**Goal**: freeze the refactored baseline and establish the stronger test and observability gates needed before new task and capability work lands.

**Source docs**:
- [Capability And Task Design](README.md)
- [Task Design](task/README.md)
- [Task Control Boundary](task_control_boundary.md)
- [Capability Refactor Completion](../completed/capability_refactor/README.md)

| Task | Completion |
|------|------------|
| Reconfirm current end-to-end docs-writer behavior through the workflow trigger path. | Proposed |
| Reconfirm provider lifecycle telemetry and progress reconstruction after the refactor. | Proposed |
| Define the new capability and task domain test matrix. | Proposed |
| Define comment gates for new public contracts and orchestrators. | Proposed |

**Exit criteria**:
- refactor baseline behavior is characterized well enough to detect regression
- observability expectations for task and capability work are explicit
- the new domain test matrix is documented before implementation begins

**Key files and seams**:
- `tests/integration/progress_observability.rs`
- `tests/integration/context_cli.rs`
- `tests/integration/workflow_cli.rs`
- `src/telemetry`
- `src/logging.rs`

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`
- integration gate: `cargo test --test integration_tests integration::context_cli::`
- integration gate: `cargo test --test integration_tests integration::workflow_cli::`

**Phase completion notes**:
- baseline `cargo check` passed on 2026-04-04
- workflow CLI integration gate passed on 2026-04-04
- progress observability integration gate passed on 2026-04-04
- commenting governance and stronger domain gate expectations are now part of the active plan

---

### Phase 1 - Capability contract core and catalog

**Goal**: introduce the shared capability contract layer and the first catalog surface in `src/capability`.

**Source docs**:
- [Capability Model](capability/README.md)
- [Capabilities By Domain](capability/by_domain.md)
- [Domain Architecture](domain_architecture.md)

| Task | Completion |
|------|------------|
| Add `src/capability` with typed contract records for identity, bindings, inputs, outputs, effects, and execution metadata. | Proposed |
| Add `BoundCapabilityInstance` and related compile-time instance projection types. | Proposed |
| Add `CapabilityCatalog` contract for registration and lookup by capability type id and version. | Proposed |
| Add validation rules for slot compatibility, binding completeness, and artifact schema agreement. | Proposed |
| Add structured invocation boundary types for runtime init and invocation payload. | Proposed |

**Exit criteria**:
- one shared capability contract model exists in code
- tasks can bind to published capability contracts without reaching into domain internals
- capability runtime init and invocation payload shapes are explicit and test covered

**Key files and seams**:
- new `src/capability.rs`
- new `src/capability/contracts.rs`
- new `src/capability/catalog.rs`
- new `src/capability/runtime.rs`
- `design/capabilities/capability/README.md`

**Expanded test work**:
- add unit coverage for capability contract validation and compatibility matching
- add unit coverage for stable identity and version resolution
- add integration coverage for catalog registration and lookup under multiple domains
- add negative tests for missing bindings, schema mismatches, and unsatisfied required slots

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- lint gate: `cargo clippy --all-targets -- -D warnings`
- unit gate: `cargo test capability::`
- integration gate: new `tests/integration/capability_contracts.rs`

**Comment gate**:
- all new public capability contract types and traits have Rustdoc comments
- validation code with non-obvious rules includes short why-focused comments

**Phase completion notes**:
- `src/capability` now provides published contract, catalog, and invocation payload types
- contract validation covers duplicate slots, required bindings, and artifact compatibility
- targeted capability unit and integration gates passed on 2026-04-04
- repo-wide clippy remains blocked by preexisting warnings outside the new capability files

---

### Phase 2 - Task records, artifact repo, and compiler

**Goal**: introduce `src/task` with durable task records, artifact repo behavior, task initialization, and compile-time graph creation.

**Source docs**:
- [Task Design](task/README.md)
- [Task Control Boundary](task_control_boundary.md)
- [Docs Writer Package](task/docs_writer_package.md)
- [Domain Architecture](domain_architecture.md)

| Task | Completion |
|------|------------|
| Add `CompiledTaskRecord`, `ArtifactRepoRecord`, and `CapabilityInvocationRecord` in `src/task`. | Proposed |
| Add task initialization payload contracts and seed artifact ingestion. | Proposed |
| Add task compiler that resolves bound capability instances from authored task definitions. | Proposed |
| Derive dependency edges from artifact requirements and effect ordering. | Proposed |
| Add compile-time validation for unsatisfied init slots, incompatible schemas, and invalid dependency structure. | Proposed |

**Exit criteria**:
- task graph creation happens at compile time
- artifact repo contract exists and is task-owned
- run creation can fail cleanly when required external structured inputs are missing
- compile-time errors and run-creation errors are distinct in code and tests

**Key files and seams**:
- new `src/task.rs`
- new `src/task/compiler.rs`
- new `src/task/artifact_repo.rs`
- new `src/task/contracts.rs`
- new `src/task/init.rs`
- `design/capabilities/task/README.md`

**Expanded test work**:
- add unit coverage for artifact repo append, lookup, supersession, and lineage
- add unit coverage for compiler graph derivation and effect ordering
- add integration coverage for compile success on valid docs-writer task definitions
- add integration coverage for compile failure on invalid wiring and missing init artifacts
- add integration coverage for deterministic compile output on repeated inputs

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- lint gate: `cargo clippy --all-targets -- -D warnings`
- unit gate: `cargo test task::artifact_repo`
- unit gate: `cargo test task::compiler`
- integration gate: new `tests/integration/task_compiler.rs`
- integration gate: new `tests/integration/task_artifact_repo.rs`

**Comment gate**:
- public task record and compiler contract types have Rustdoc comments
- graph derivation, effect ordering, and artifact supersession code includes invariant comments where the logic is not self-evident

**Phase completion notes**:
- `src/task` now provides durable task records, init payload validation, artifact repo behavior, and compiler entry points
- task compilation derives artifact edges from upstream slot wiring and serializes exclusive effects deterministically
- targeted task compiler and artifact repo unit and integration gates passed on 2026-04-04
- repo-wide clippy remains blocked by preexisting warnings outside the new task files

---

### Phase 3 - Task executor and invocation payload assembly

**Goal**: implement task-local execution progression and capability payload assembly without collapsing task back into domain execution details.

**Source docs**:
- [Task Design](task/README.md)
- [Capability Model](capability/README.md)
- [Task Control Boundary](task_control_boundary.md)
- [Task Network](../control/task_network.md)

| Task | Completion |
|------|------------|
| Add `TaskExecutor` in `src/task` as the single task-local execution agent. | Proposed |
| Implement ready-set evaluation over compiled task structure plus current artifact repo state. | Proposed |
| Implement invocation payload assembly from seed artifacts, upstream artifacts, and control-supplied execution context. | Proposed |
| Add invocation record persistence and task event emission. | Proposed |
| Add capability runtime acquisition from bound static data without leaking process-local objects across the task boundary. | Proposed |

**Exit criteria**:
- one task can advance itself through ready capability instances
- `CapabilityInvocationPayload` assembly is explicit and test covered
- task emits durable execution events and persists invocation records consistently
- task-local progression is clearly separate from task-network ordering and repair intent

**Key files and seams**:
- new `src/task/executor.rs`
- new `src/task/readiness.rs`
- new `src/task/events.rs`
- new `src/task/invocation.rs`
- `design/capabilities/task/README.md`
- `design/capabilities/capability/README.md`

**Expanded test work**:
- add unit coverage for ready-set calculation from artifact satisfaction
- add unit coverage for invocation payload assembly and ownership of `invocation_id`, `upstream_lineage`, and `execution_context`
- add integration coverage for parallel release of sibling capability instances
- add integration coverage for block then unblock behavior after artifact persistence
- add integration coverage for event emission and invocation record parity

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- lint gate: `cargo clippy --all-targets -- -D warnings`
- unit gate: `cargo test task::executor`
- unit gate: `cargo test task::readiness`
- integration gate: new `tests/integration/task_executor.rs`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`

**Comment gate**:
- task executor public entry points have Rustdoc comments
- readiness, barrier, and event-emission logic includes concise comments on invariants and ownership boundaries

**Phase completion notes**:
- `TaskExecutor` now seeds init artifacts, computes ready capability instances, assembles invocation payloads, and records task-local success or failure
- task-local event emission now covers request, start, progress, blocked, artifact emitted, success, and failure boundaries
- targeted readiness, executor, and task executor integration gates passed on 2026-04-04
- repo-wide clippy remains blocked by preexisting warnings outside the new task files

---

### Phase 4 - First-slice capability implementation

**Goal**: implement the initial high-value capability set against the shared contract model.

**Source docs**:
- [Capabilities By Domain](capability/by_domain.md)
- [Workspace Resolve Node Id](capability/workspace_resolve_node_id/README.md)
- [Merkle Traversal](../completed/capability_refactor/merkle_traversal/README.md)
- [Context Generate Prepare](capability/context_generate_prepare/README.md)
- [Provider Execute Chat](capability/provider_execute_chat/README.md)
- [Context Generate Finalize](capability/context_generate_finalize/README.md)

| Task | Completion |
|------|------------|
| Publish `WorkspaceResolveNodeId` from the workspace domain. | Proposed |
| Publish `MerkleTraversal` from the traversal domain with ordered batch outputs. | Proposed |
| Publish `ContextGeneratePrepare` from the context domain. | Proposed |
| Publish `ProviderExecuteChat` from the provider domain. | Proposed |
| Publish `ContextGenerateFinalize` from the context domain. | Proposed |
| Register the first-slice capability set in the capability catalog. | Proposed |

**Exit criteria**:
- the first-slice capability set exists in code behind published contracts
- each capability accepts structured invocation payloads and emits structured artifacts or failure summaries
- no first-slice capability requires task to pass process-local object references

**Key files and seams**:
- `src/workspace/capability.rs`
- `src/merkle_traversal.rs` or domain-local capability publication path
- `src/context/capability.rs`
- `src/provider/capability.rs`
- `src/provider/executor.rs`
- `design/capabilities/capability/by_domain.md`

**Expanded test work**:
- add unit coverage for sig adapter resolution in each first-slice capability
- add unit coverage for output shaping and failure artifact emission
- add integration coverage for each capability in task-driven invocation mode
- add integration coverage for provider execution correlation through the capability boundary
- add integration coverage for traversal batch determinism and workspace resolution stability

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- lint gate: `cargo clippy --all-targets -- -D warnings`
- unit gate: `cargo test workspace::capability`
- unit gate: `cargo test provider::capability`
- unit gate: `cargo test context::capability`
- integration gate: new `tests/integration/capability_invocation.rs`
- integration gate: new `tests/integration/merkle_traversal.rs`
- integration gate: new `tests/integration/provider_execution.rs`

**Comment gate**:
- public capability publication code has Rustdoc comments
- sig adapter code explains non-obvious slot-to-argument resolution and output shaping decisions

**Phase completion notes**:
- the first-slice invoker registry now exists in `src/capability/invocation.rs`
- workspace, Merkle traversal, context prepare, provider execute, and context finalize now publish real contracts from their owning domains
- the new capability invocation integration suite proves real structured-data invocation over a scanned tree and a mock provider on 2026-04-04
- repo-wide clippy remains blocked by preexisting warnings outside the new capability files

---

### Phase 5 - Docs-writer task package and task DAG execution

**Goal**: realize docs-writer as a task package that compiles into a task DAG and runs through task execution rather than workflow-owned orchestration.

**Source docs**:
- [Docs Writer Package](task/docs_writer_package.md)
- [Task Design](task/README.md)
- [Task Control Boundary](task_control_boundary.md)
- [Control Design](../control/README.md)

| Task | Completion |
|------|------------|
| Implement package loading and package-to-task-definition lowering for docs-writer. | Proposed |
| Compile docs-writer into a task DAG with bottom-up child-to-parent artifact requirements. | Proposed |
| Run docs-writer through task executor using the first-slice capabilities. | Proposed |
| Persist docs-writer artifacts and invocation records in the task artifact repo. | Proposed |
| Emit task and capability events that reconstruct progress for one docs-writer run. | Proposed |

**Exit criteria**:
- docs-writer no longer depends on workflow-owned execution order
- docs-writer task DAG compiles deterministically
- sibling nodes run in parallel when artifact requirements permit
- parent work does not release until required child artifacts are present

**Key files and seams**:
- new `src/task/package.rs`
- new `src/task/templates/docs_writer.rs`
- `src/task/compiler.rs`
- `src/task/executor.rs`
- `src/control`
- `design/capabilities/task/docs_writer_package.md`

**Expanded test work**:
- add integration coverage for docs-writer compile output on representative directory trees
- add integration coverage for bottom-up parallel sibling release
- add integration coverage for artifact repo state after each wave
- add integration coverage for task event reconstruction of docs-writer progress
- add parity coverage against current user-visible docs-writer output behavior

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- lint gate: `cargo clippy --all-targets -- -D warnings`
- integration gate: new `tests/integration/docs_writer_task.rs`
- integration gate: new `tests/integration/task_progress_events.rs`
- regression gate: `cargo test --test integration_tests integration::workflow_cli::`

**Comment gate**:
- package lowering and DAG expansion code documents the child-to-parent dependency invariant
- tests include clear fixtures and comments where tree shapes express execution intent

**Phase completion notes**:
- `src/task/package.rs`, `src/task/runtime.rs`, and `src/task/templates/docs_writer.rs` now lower docs-writer into a compiled task and execute it through the task runtime
- docs-writer task runs now persist task-scoped artifacts, invocation records, and final frames while preserving bottom-up child-to-parent dependencies
- the new docs-writer task integration suite proves deterministic compilation and end-to-end execution on 2026-04-04
- repo-wide clippy remains blocked by preexisting warnings outside the new task and capability files

---

### Phase 6 - Workflow convergence onto task execution

**Goal**: make workflow a compatibility trigger into task package and task execution instead of a separate execution runtime.

**Source docs**:
- [Capability Refactor Completion](../completed/capability_refactor/README.md)
- [Docs Writer Package](task/docs_writer_package.md)
- [Task Design](task/README.md)
- [Control Design](../control/README.md)

| Task | Completion |
|------|------------|
| Add workflow compatibility mapping from legacy workflow request shape to task package trigger shape. | Proposed |
| Route workflow-triggered docs-writer execution into task creation and task execution. | Proposed |
| Keep current CLI and watch entry paths working while execution ownership moves underneath. | Proposed |
| Remove workflow-owned sequencing and capability triggering from the active path. | Proposed |
| Keep user-visible behavior stable through compatibility adapters until workflow retirement is complete. | Proposed |

**Exit criteria**:
- workflow remains callable but no longer owns execution progression
- workflow-triggered docs-writer runs through task execution
- workflow compatibility behavior is covered well enough to retire legacy workflow internals later

**Key files and seams**:
- `src/workflow/facade.rs`
- `src/workflow/executor.rs`
- `src/workflow/resolver.rs`
- `src/cli/route.rs`
- `src/task`
- `src/control`

**Expanded test work**:
- extend workflow CLI coverage to assert task creation and task execution under the compatibility path
- add integration coverage for workflow-to-task trigger mapping errors
- add progress and telemetry coverage proving workflow session events join cleanly with task events
- add watch-mode compatibility coverage if watch still enters through workflow-shaped routes

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- lint gate: `cargo clippy --all-targets -- -D warnings`
- integration gate: `cargo test --test integration_tests integration::workflow_cli::`
- integration gate: new `tests/integration/workflow_task_compatibility.rs`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`

**Comment gate**:
- compatibility adapters include short comments that explain why the adapter still exists and what future task-native path will replace it

**Phase completion notes**:
- `src/workflow/executor.rs` now routes the active docs-writer execution path into task package preparation and task runtime execution while preserving the outer workflow trigger contract
- workflow-triggered docs-writer runs now emit workflow-compatible turn, lineage, provider, and summary events through the task-backed path
- workflow CLI, workflow-task compatibility, and progress observability gates passed on 2026-04-04
- repo-wide clippy remains blocked by preexisting warnings outside the new workflow, task, and capability files

---

### Phase 7 - Boundary seal, workflow retirement, and readiness signoff

**Goal**: remove the last mixed-ownership paths and seal the codebase around capability and task as the durable implementation layer.

**Source docs**:
- [Capability And Task Design](README.md)
- [Capability Refactor Completion](../completed/capability_refactor/README.md)
- [Task Design](task/README.md)
- [Control Design](../control/README.md)

| Task | Completion |
|------|------------|
| Remove dead workflow-owned execution code from active paths. | Proposed |
| Remove dead compatibility shims once all replacement gates are green. | Proposed |
| Publish the post-cutover `src` map for capability, task, control, context, provider, and workflow. | Proposed |
| Confirm that workflow now acts only as a package and trigger surface, or remove it when fully obsolete. | Proposed |
| Lock the final regression and observability suite. | Proposed |

**Exit criteria**:
- each runtime concern has one clear owner
- no active path bypasses task for docs-writer style execution
- workflow is no longer a hidden executor
- capability, task, and control boundaries are stable enough for later repair and task-network growth

**Key files and seams**:
- `src/task`
- `src/capability`
- `src/control`
- `src/workflow`
- `tests/integration`

**Implementation evidence**:
- format gate: `cargo fmt -- --check`
- compile gate: `cargo check`
- lint gate: `cargo clippy --all-targets -- -D warnings`
- full integration gate: `cargo test --test integration_tests`
- full suite gate: `cargo test`

**Comment gate**:
- remaining public domain contracts have Rustdoc coverage
- no compatibility adapter remains without an explanatory comment or an issue to remove it

## Cross-Phase Gates

### Boundary Gates

- task to capability traffic remains structured-data-only
- artifact persistence remains task-owned
- task-local readiness and triggering remain task-owned
- task-network ordering and repair intent remain control-owned
- provider execution remains provider-owned
- workflow does not regain hidden execution ownership

### Contract Gates

- every first-slice capability is published through the shared capability contract model
- every task payload field has a clear owner across compiler, task runtime, and control runtime
- no published contract requires process-local object references
- compile-time validation catches incompatible slot and artifact wiring before live execution

### Observability Gates

- task events and capability events use the shared telemetry envelope
- progress reducers can reconstruct task progression from emitted events
- provider lifecycle events remain correlated to stable request identity
- logs are diagnostic and never the sole source of durable execution truth

### Test Expansion Gates

- new domains land with unit coverage before broad integration coverage depends on them
- every phase adds at least one new negative-path test
- docs-writer receives end-to-end integration coverage as a task package
- workflow compatibility remains covered until workflow-owned execution is retired

### Commenting Gates

- public contracts, traits, and domain entry points added in this plan have Rustdoc comments
- non-obvious orchestration, graph, retry, and compatibility logic has short why-focused comments
- obvious statement-by-statement narration comments are avoided
- comments are updated or removed in the same change when surrounding behavior changes

## Implementation Order Summary

1. lock the refactored baseline with stronger gates
2. build the capability contract core and catalog
3. build task records, artifact repo, and compiler
4. build task executor and payload assembly
5. implement the first-slice capability set
6. run docs-writer as a task package
7. converge workflow onto task execution
8. retire workflow-owned execution and seal the boundaries

## Read With

- [Capability And Task Design](README.md)
- [Capability Model](capability/README.md)
- [Task Design](task/README.md)
- [Docs Writer Package](task/docs_writer_package.md)
- [Task Control Boundary](task_control_boundary.md)
- [Control Design](../control/README.md)
- [Task Network](../control/task_network.md)
- [Commenting Policy](../../governance/commenting_policy.md)
