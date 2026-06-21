# RTG-3 Bounded Worker Report Contract Implementation Plan

Date: 2026-06-14
Status: implemented for first durable assembly
Scope: bounded worker diagnostics contract for the durable runtime first slice

## Purpose

This plan defines how to implement `RTG-3`, the bounded worker report contract required before durable flywheel assembly can be built.

The durable supervisor needs a uniform way to observe bounded worker operations, surface retryable and fatal failures, and prove restart safe progress. The supervisor must not become semantic authority for worker progress. Durable meaning remains in the owning domain stores.

## Required Outcome

Each actor that joins the first flywheel proof returns a bounded report with:

- actor id
- input cursor or input revision
- output cursor or output revision
- items attempted
- items committed
- retryable errors
- fatal errors
- budget exhausted flag

The report is diagnostic only. It may guide loop control, operator output, and integration test assertions, but it must not replace domain cursors, revisions, leases, command journals, or event records.

## Source Findings

[Durable Runtime Pre Implementation Gaps](durable_runtime_pre_implementation_gaps.md) names `RTG-3` as a blocker because `Idle` and `MadeProgress` do not carry enough restart proof data.

[Durable Runtime First Slice](durable_runtime_first_slice.md) requires bounded worker ticks that read durable input state, write through owner boundaries, advance cursors only after output is durable, tolerate duplicate delivery, and return diagnostics that are not semantic authority.

[GraphRuntime](../../../crates/meld-world-model/src/world_state/graph/runtime.rs) currently exposes `catch_up` as a count. The durable input and output cursor already exists as `TraversalStore::last_reduced_seq`, but the report does not expose the cursor before or after replay.

[TraversalReducer](../../../crates/meld-world-model/src/world_state/graph/reducer.rs) currently replays every event after the cursor through `EventStore::read_all_events_after`. A truly bounded graph worker needs a limited event read or a bounded reducer path.

[Publication Bridge](../../../crates/meld-execution/src/task_network/publication.rs) already has a bounded request through `limit` and a per item result enum. Its current report lacks actor identity, task network revisions before and after the tick, normalized retryable and fatal error buckets, and explicit budget exhaustion.

## Full Vision Alignment

The report contract must fit the full cognitive architecture, not only the first `docs_freshness` proof.

Meld is an open-world loop:

```text
observe
-> sensory
-> event ledger
-> world model
-> execution
-> event ledger
```

The contract therefore has to work for stream-shaped sensory workers, event replay reducers, layered world model inference, agent curation, execution planning, task network work, publication, and later synthesis.

Alignment rules:

- reports use the event spine sequence as the shared temporal clock whenever a worker consumes or produces canonical facts
- reports may use domain revisions where the owning aggregate has its own durable command stream
- reports must carry enough scope to distinguish agent, perspective, branch, subject, stream, and belief key where those values are material
- reports must treat raw sensory lanes as source-local diagnostics until a promoted semantic observation enters the event spine
- reports must support world model layers beyond graph and belief, including causal inference epochs, regime inference epochs, and planner-facing view snapshots
- reports must support execution's graphs-lower-graphs model across planning, task network, task, capability, and synthesis workers
- reports must remain diagnostics and must never replace typed event facts, belief revisions, agent decisions, goal records, task network revisions, or capability catalog records

This keeps `RTG-3` small enough to unblock the first durable assembly while ensuring the report vocabulary will not need replacement when Meld grows into continuous sensing, multi-agent belief ownership, causation, regimes, and online capability acquisition.

## Non Goals

- no durable supervisor implementation
- no CLI command
- no central worker state machine
- no new root owned cursor table
- no change to domain semantic ownership
- no multi process supervision
- no broad telemetry redesign

## Design Rules

Reports describe what a worker observed during one bounded tick. They do not define whether a goal exists, whether a goal is satisfied, whether a task outcome is valid, whether evidence should update a belief, or whether a task network command should be accepted.

Each owning domain keeps its own native report type when that avoids crate dependency cycles. Root runtime assembly may normalize those domain reports into one supervisor facing `WorkerTickReport`.

The root `meld` crate may define supervisor facing report contracts because root owns assembly and process diagnostics. Domain crates must not depend on root `meld`.

The first implementation should preserve existing public APIs with compatibility wrappers when practical. Existing callers of `GraphRuntime::catch_up` and `publish_pending_publications` should keep working while new report rich APIs are introduced.

## Contract Shape

Add the supervisor facing diagnostic contract under root runtime assembly, proposed path `src/runtime/contracts.rs`. If the runtime module does not exist yet, add only the contract module and export it from `src/runtime.rs` when assembly work begins.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkBudget {
    pub max_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerScope {
    pub domain_id: String,
    pub stream_id: Option<String>,
    pub work_key: Option<String>,
    pub agent_id: Option<String>,
    pub perspective_key: Option<String>,
    pub branch_id: Option<String>,
    pub subject_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCheckpoint {
    pub name: String,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTickIssue {
    pub item_id: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTickReport {
    pub actor_id: String,
    pub scope: WorkerScope,
    pub input_checkpoint: WorkerCheckpoint,
    pub output_checkpoint: WorkerCheckpoint,
    pub items_attempted: usize,
    pub items_committed: usize,
    pub retryable_errors: Vec<WorkerTickIssue>,
    pub fatal_errors: Vec<WorkerTickIssue>,
    pub budget_exhausted: bool,
}
```

`WorkBudget::max_items` must be greater than zero for normal bounded operations. Domain APIs may reject zero budgets or return a no work report, but runtime assembly should treat zero as a configuration error.

`WorkerScope` keeps the report usable across the full cognitive architecture. It should reuse event ledger vocabulary where possible. Use `domain_id` and `stream_id` for event-aligned work. Use `work_key` for the owning domain's stable unit of work, such as a belief key, causal variable, regime segment, task instance, publication, or catalog record. Use `agent_id`, `perspective_key`, `branch_id`, and `subject_key` for perspective-scoped world model and agent work.

`WorkerScope` is not a lock key or ownership claim. It is diagnostic scope copied from the owning domain contract.

`WorkerCheckpoint::name` should use stable labels such as `event_spine_seq`, `task_network_revision`, `belief_source_seq`, `agent_review_seq`, or `execution_goal_updated_seq`.

`items_attempted` counts durable input items selected for processing under the budget.

`items_committed` counts selected items whose intended durable output was committed. Durable retry markers may be useful recovery data, but they do not count as committed business output unless the owning domain considers them the successful output of that worker.

`budget_exhausted` means the worker stopped because the requested budget was consumed while more durable input may remain. Workers should detect this by reading or selecting one extra item when that is cheap and safe.

## Progress Interpretation

The supervisor derives progress diagnostics from the report instead of reading a shared `WorkStatus`.

Progress is present when any of these are true:

- output checkpoint value is greater than input checkpoint value
- items committed is greater than zero
- the owning domain report explicitly maps to a durable command or append that changed a store revision

The supervisor must also treat fatal errors, retryable errors, and budget exhaustion as separate diagnostic outcomes. A pass with no progress and retryable errors is not the same as convergence.

## Error Classification

Retryable errors include transient append failures, store contention, lease backpressure, unavailable provider or external process errors, and task network publication append failures that are durably marked for retry.

Fatal errors include invalid requests, corrupt durable records, malformed fixture data, impossible identity mismatches, validation failures that cannot be fixed by retry, and command rejections that indicate a violated precondition rather than stale duplicate delivery.

Duplicate delivery is not an error when the owning domain accepts it as idempotent replay. It should produce zero committed items unless the domain reports that a durable output was newly written.

## Phase 1 Shared Event Read Budget

Owner: `events`

Add a bounded read path to `EventStore`:

```rust
pub fn read_all_events_after_limit(
    &self,
    after_seq: u64,
    limit: usize,
) -> Result<Vec<EventRecord>, StorageError>
```

The method should return events in runtime sequence order, after the supplied cursor, capped to `limit`.

For budget exhaustion detection, graph replay can request `max_items + 1`, process only `max_items`, and set `budget_exhausted` when the extra event exists.

Verification:

- extend `crates/meld-events/tests/event_store_contracts.rs`
- prove ordered reads after a cursor
- prove capped result size
- prove `limit` zero returns an empty vector or is rejected consistently with chosen API policy

## Phase 2 Graph Runtime Report

Owner: `world_state` graph

Add a native graph report in `crates/meld-world-model/src/world_state/graph/runtime.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCatchUpReport {
    pub actor_id: String,
    pub input_event_seq: u64,
    pub output_event_seq: u64,
    pub events_attempted: usize,
    pub traversal_events_applied: usize,
    pub derived_events_appended: usize,
    pub retryable_errors: Vec<GraphWorkerIssue>,
    pub fatal_errors: Vec<GraphWorkerIssue>,
    pub budget_exhausted: bool,
}
```

Add a bounded method:

```rust
pub fn catch_up_bounded(
    &self,
    budget: GraphCatchUpBudget,
) -> Result<GraphCatchUpReport, StorageError>
```

Keep `catch_up` as a compatibility wrapper that calls the unbounded path or calls the bounded path with an effectively unlimited budget and returns `traversal_events_applied`.

Implementation notes:

- read `input_event_seq` from `TraversalStore::last_reduced_seq`
- replay at most `max_items` event records
- append derived graph events idempotently before advancing `last_reduced_seq`
- flush event spine before writing traversal cursor
- write `last_reduced_seq` to the highest durable output sequence seen by this tick
- set `budget_exhausted` only when more event records remain after the selected budget
- classify storage failures as fatal unless a specific store API identifies retryable backpressure

Reducer work:

- add a bounded reducer entrypoint alongside `TraversalReducer::replay_from_spine`
- pass selected event records into the reducer so event selection and budget handling stay outside semantic reduction
- preserve idempotent derived event append behavior

Verification:

- extend `tests/integration/traversal_graph.rs`
- prove report shows input cursor zero and output cursor after the first source event
- prove repeated catch up returns equal input and output cursors with zero attempted items
- prove derived anchor events remain idempotent after restart
- prove a budget of one over two source events sets `budget_exhausted`
- prove reopening after a bounded tick resumes from the persisted output cursor

## Phase 3 Publication Bridge Report

Owner: `execution` task network

Replace or extend `PublicationBridgeReport` with the required diagnostics while preserving the current `results` field for existing assertions.

Proposed shape:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationBridgeReport {
    pub actor_id: String,
    pub scope: PublicationBridgeScope,
    pub input_revision: u64,
    pub output_revision: u64,
    pub items_attempted: usize,
    pub items_committed: usize,
    pub retryable_errors: Vec<PublicationBridgeIssue>,
    pub fatal_errors: Vec<PublicationBridgeIssue>,
    pub budget_exhausted: bool,
    pub results: Vec<PublicationPublishResult>,
}
```

Request changes:

- keep `PublishPendingPublicationsRequest::limit`
- derive the effective `WorkBudget` from `limit`
- keep `worker_id` as audit input
- use actor id `execution.task_network.publication`
- include network id and session id in the domain report scope so the root report can map them to `WorkerScope`

Budget handling:

- select pending or failed publication ids in deterministic id order
- select `max_items + 1` ids when a limit is present
- process only `max_items`
- set `budget_exhausted` when the extra id exists

Report mapping:

- `Published` increments attempted and committed
- `AppendFailed` increments attempted and adds a retryable issue
- `MarkRejected` increments attempted and adds a fatal issue unless the rejection is a known stale duplicate case
- `AlreadyPublished` increments attempted only for direct single item calls

Revision mapping:

- `input_revision` is `store.state().revision` before selection
- `output_revision` is `store.state().revision` after all mark commands
- report progress can be derived from revision movement or committed items

Verification:

- extend `crates/meld-execution/tests/task_network_publication_bridge.rs`
- prove success report includes actor id, input revision, output revision, attempted count, committed count, and no errors
- prove append failure is retryable and records no committed item
- prove mark rejection is fatal
- prove limit one over two pending publications sets `budget_exhausted`
- prove retry after failed publication clears retryable status when publish succeeds

## Phase 4 Root Normalization

Owner: root `meld` assembly

Add conversion helpers near runtime assembly:

- `GraphCatchUpReport` to `WorkerTickReport`
- `PublicationBridgeReport` to `WorkerTickReport`
- future belief assessment report to `WorkerTickReport`
- future agent curation report to `WorkerTickReport`
- future goal mutation report to `WorkerTickReport`
- future task dispatch and task execution reports to `WorkerTickReport`

The conversion layer should be intentionally thin. It may rename domain fields into the host contract, but it must not recalculate semantic success from raw store state.

Verification:

- add narrow unit tests for conversion logic once `src/runtime` exists
- assert actor ids and checkpoint names are stable
- assert retryable and fatal issue order is preserved

## Phase 5 First Slice Actor Coverage

Owner: owning actor domains with root assembly

Before durable assembly is implemented, list every actor in the proof and require either a native report or an adapter report.

Initial required coverage:

| Actor | Owning Domain | Checkpoint Input | Checkpoint Output | Implementation Path |
| --- | --- | --- | --- | --- |
| graph reducer | `world_state` graph | `event_spine_seq` | `event_spine_seq` | `GraphRuntime::catch_up_bounded` |
| belief assessor | `world_state` belief | `belief_source_seq` | `belief_source_seq` | wrap `BeliefRuntime::assess_subject` and `assess_dirty_key` |
| agent goal curation | `world_state` agent | `belief_revision_seq` | `agent_decision_seq` | wrap `AgentCuration::handle_delivery` |
| goal acceptance | `execution` goals | `agent_decision_seq` | `execution_goal_updated_seq` | root adapter around `GoalSetApi` |
| planning loop | `execution` planning | `execution_goal_updated_seq` | `planning_request_seq` | root adapter around `PlanningRuntime::plan_goal` |
| task network reducer | `execution` task network | `task_network_revision` | `task_network_revision` | report around command submission batch |
| task worker | `execution` task plus root adapters | `task_network_revision` | `task_network_revision` | report around claim, execution, and outcome command |
| publication worker | `execution` task network | `task_network_revision` | `task_network_revision` | `publish_pending_publications` report |
| outcome evidence ingestion | `world_state` belief plus root adapter | `event_spine_seq` | `belief_source_seq` | report around configured evidence mapper and ingestion |
| satisfaction curation | `world_state` agent | `agent_review_seq` | `agent_decision_seq` | wrap `AgentCuration::handle_satisfaction_review` |
| goal satisfaction mutation | `execution` goals | `agent_decision_seq` | `execution_goal_updated_seq` | root adapter around satisfaction mutation |

Full vision coverage:

| Actor Family | Owning Domain | Durable Boundary | Report Scope Requirement |
| --- | --- | --- | --- |
| sensory modality worker | `sensory` | promoted observation event or source-local cursor | modality, source, stream, promotion rule |
| sensory lowering worker | `sensory` | lowered observation window or promoted event | modality, source, lowering window |
| graph reducer | `world_state` graph | event spine cursor and traversal cursor | branch, stream, subject when selected |
| belief comparator worker | `world_state` belief | belief lease and revision | belief key, agent, perspective, branch |
| belief recovery scanner | `world_state` belief | recovered lease or dirty key cursor | belief key, owner, config snapshot |
| causal inference worker | `world_state` causation | causal claim or inference epoch | variable family, intervention, outcome, regime |
| regime inference worker | `world_state` regime | regime posterior or segment epoch | segment, regime library, perspective |
| planner projection worker | `world_state` planner | view snapshot or projection version | decision context, agent lens, branch |
| agent activation worker | `world_state` agent | activation record and subscription cursor | agent, perspective, subject |
| agent curation worker | `world_state` agent | curation decision and subscription cursor | agent, belief key, branch, review |
| execution planning worker | `execution` planning | planning decision and task network command | goal, method, world state snapshot |
| task network reducer | `execution` task network | task network revision | network, command stream |
| task dispatch worker | `execution` task network | dispatch claim revision | network, task instance, claim |
| task executor worker | `execution` task | task outcome command and artifact records | task run, capability instance, artifact repo |
| publication worker | `execution` task network | publication mark and event append | network, publication, event session |
| synthesis worker | `execution` synthesis | synthesized capability catalog record | artifact type, catalog version, trust state |

This phase may land as documentation and adapter stubs if the full host is still blocked by other RTG items. The key requirement is that the eventual host has no unreported tick path.

The full vision table is not a mandate to implement every actor during `RTG-3`. It is a compatibility guard. New workers should fit the same bounded report vocabulary instead of introducing another progress model.

## Phase 6 Turn Report Assembly

Owner: root `meld` assembly

Define the eventual turn report as a list of normalized worker reports plus summary booleans:

```rust
pub struct RuntimeTurnReport {
    pub worker_reports: Vec<WorkerTickReport>,
    pub made_progress: bool,
    pub retryable_error_count: usize,
    pub fatal_error_count: usize,
    pub budget_exhausted: bool,
}
```

Summary values are derived from worker reports only.

Stop rules:

- stop as converged when every worker report has no progress, no retryable errors, no fatal errors, and no budget exhaustion
- stop as budget limited when any report has `budget_exhausted`
- stop as failed when any report has fatal errors
- stop as retryable blocked when no worker made progress and at least one report has retryable errors
- continue when any worker made progress and no fatal stop condition is present

Verification:

- add unit tests for convergence summary
- add integration assertions to the later `minimal_runtime_flywheel_turn_persists_and_satisfies_goal` proof
- assert reports after reopen show durable checkpoints, not root local counters

## Implementation Order

1. Add bounded event read support in `meld-events`.
2. Add bounded graph reducer and `GraphCatchUpReport`.
3. Preserve `GraphRuntime::catch_up` as a compatibility wrapper.
4. Enrich `PublicationBridgeReport` and preserve existing result behavior.
5. Add report field assertions to graph and publication tests.
6. Add root report contract and conversion helpers when runtime assembly begins.
7. Add coverage stubs or adapters for the remaining first slice actors.
8. Use normalized worker reports in the durable proof harness and supervisor diagnostics.
9. Require future sensory, causal, regime, planner, agent, and synthesis workers to map into the same report contract before they join the supervised flywheel.

## Test Commands

Run focused commands after each domain phase:

```sh
cargo test -p meld-events --test event_store_contracts
cargo test --test integration_tests graph_runtime
cargo test -p meld-execution --test task_network_publication_bridge
```

Run the broader checks before marking `RTG-3` complete:

```sh
cargo test -p meld-world-model
cargo test -p meld-execution
cargo test --test integration_tests minimal_runtime_flywheel_turn_persists_and_satisfies_goal
```

The final flywheel command will remain unavailable until durable flywheel assembly and the remaining RTG blockers are implemented.

## Acceptance Criteria

`RTG-3` is complete when all statements below are true.

- a stable worker report shape exists for root runtime assembly
- graph replay has a bounded report with input and output event sequence
- publication bridge has a bounded report with task network revisions and error buckets
- every first slice bounded operation has a native report or a root adapter report
- budget exhaustion is observable without root owned cursors
- retryable and fatal errors are distinguishable in reports
- compatibility wrappers keep existing focused tests meaningful
- restart tests prove output checkpoints come from durable stores
- report scope can distinguish many agents sharing one event and graph substrate
- deferred sensory, causal, regime, planner, and synthesis workers have an explicit compatibility path into the same report vocabulary

## Implementation Evidence

Implemented code paths:

- `EventStore::read_all_events_after_limit`
- `TraversalReducer::replay_events`
- `GraphRuntime::catch_up_bounded`
- `GraphCatchUpReport`
- `PublicationBridgeReport` with scope, revisions, item counts, error buckets, and budget exhaustion
- root `runtime::contracts::WorkerTickReport`
- root conversions from graph and publication reports into worker reports

Verification run:

```sh
cargo test -p meld-events --test event_store_contracts
cargo test -p meld-world-model
cargo test -p meld-execution
cargo test --test integration_tests graph_runtime
cargo test runtime::contracts
cargo check
git diff --check
```

## Risks

The main design risk is accidentally making root own semantic progress by adding root side cursors. The mitigation is to use checkpoints copied from domain stores after writes are durable.

The main implementation risk is graph replay budget behavior. If replay selects all events and only reports a cap afterward, the worker is not actually bounded. Add a bounded event read path first.

The publication bridge risk is treating retry failure marks as committed progress. The report must distinguish durable retry metadata from successful publication output.

The full vision risk is building a first slice report that cannot scale to multi-agent and layered world model work. The mitigation is `WorkerScope` plus explicit future actor coverage.

## Related Documents

- [Durable Runtime Pre Implementation Gaps](durable_runtime_pre_implementation_gaps.md)
- [Durable Runtime First Slice](durable_runtime_first_slice.md)
- [Minimal Runtime Flywheel](minimal_runtime_flywheel.md)
- [Cognitive Architecture](../../cognitive_architecture/README.md)
- [World Model Domain](../../cognitive_architecture/world_model/README.md)
- [World Model Vision](../../cognitive_architecture/world_model/VISION.md)
- [Sensory Domain](../../cognitive_architecture/sensory/README.md)
- [Events Domain](../../cognitive_architecture/events/README.md)
- [Execution Domain](../../cognitive_architecture/execution/README.md)
