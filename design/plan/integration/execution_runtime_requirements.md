# Execution Runtime Requirements

Date: 2026-06-17
Status: superseded
Scope: execution runtime actors only

Note 2026-08-12: superseded. [Runtime Requirements Index](runtime_requirements.md) recorded the replacement of this document by the runtime completion authority chain.

## Purpose

This document defines design requirements for the execution runtime actors that carry the flywheel from accepted goals to published execution facts.

The target flywheel shape is:

```text
world model agent
-> execution public goal API
-> execution goal lifecycle store
-> planning over world model planner projection
-> task network command state
-> task dispatch
-> task artifact persistence
-> task outcome recording
-> publication outbox
-> event append API
```

Execution owns the operational commitment. It accepts goal commands through its public API, stores goal lifecycle state, reads planner projection through an injected port, decomposes active goals into task network state, dispatches tasks, persists artifacts, records task outcomes, and publishes pending execution facts through the event append API.

Root assembly only opens stores, wires ports, starts runtime actors, supervises lifecycle, handles shutdown, and reports diagnostics. Root assembly must not become a semantic scheduler for the execution flywheel.

This is design work only. It does not request code implementation.

## Source Material

Primary source material:

- `design/plan/integration/durable_flywheel_runtime_phase_design.md`
- `design/plan/integration/flywheel_runtime_code_assessment.md`
- `design/plan/execution/assessment.md`
- `design/plan/execution/goals/assessment.md`
- `design/plan/execution/planning/PLAN.md`
- `design/plan/execution/task_network/PLAN.md`
- `design/plan/execution/task_network/PHASE8.md`
- `crates/meld-execution/src/goals/api.rs`
- `crates/meld-execution/src/planning/runtime.rs`
- `crates/meld-execution/src/task_network/publication.rs`
- `crates/meld-execution/src/task/runtime.rs`

Additional execution source surfaces used for requirements:

- `crates/meld-execution/src/goals/persistent_store.rs`
- `crates/meld-execution/src/planning/world_state.rs`
- `crates/meld-execution/src/planning/lowering.rs`
- `crates/meld-execution/src/task_network/command.rs`
- `crates/meld-execution/src/task_network/dispatch.rs`
- `crates/meld-execution/src/task_network/outcome.rs`
- `crates/meld-execution/src/task_network/store/sled.rs`
- `crates/meld-execution/src/task_network/initialization.rs`
- `crates/meld-execution/src/task/artifact_repo.rs`
- `crates/meld-execution/src/task/artifact_repo/factory.rs`

## Runtime Boundary

Execution runtime actors are domain actors inside `meld-execution`. They own execution state and execution transitions. They may call injected ports for world model projection, capability invocation, artifact storage, task network command acceptance, and event append.

Execution runtime actors must not import world model internals or event store internals for semantic decisions. Stable contracts and ports carry every cross domain handoff.

Root `meld` owns concrete store location, port implementation construction, actor handles, cancellation tokens, health reports, restart policy, and operator diagnostics.

Root `meld` must not store execution progress memory such as active goal cursors, task network revisions, pending publication ids, claim fences, planner projection frames, or artifact state.

## Runtime Actors

### Goal Set Runtime

Purpose:

The goal set runtime is the public execution ingress for goal lifecycle commands. It accepts producer neutral goal requests through `GoalSetApi`, converts valid accepted goals into execution owned lifecycle records, and exposes deterministic active goal reads to planning.

Required behavior:

- Accept goal commands through the public execution API.
- Validate shared execution invariants such as non empty command id, non empty goal id, non empty agent id, ground target, and permitted lifecycle policy.
- Convert proposed producer goals to active execution goals when the acceptance lifecycle requires activation.
- Store execution lifecycle records in `PersistentGoalSetStore` or an equivalent durable store.
- Persist command outcomes for applied, duplicate, and not found results.
- Dedupe producer retries through command id and optional source identity.
- Expose active goals and goal records in deterministic goal id order.
- Accept lifecycle mutation commands for modify, remove, satisfy, suspend, and resume.
- Flush durable goal state before reporting committed progress.

It owns:

- execution goal records
- goal lifecycle state
- goal command outcomes
- source identity dedupe index
- goal created and updated sequence metadata
- execution side validation of neutral goal acceptance

It does not own:

- agent curation policy
- goal construction judgment
- belief confidence
- planner projection semantics
- task decomposition
- task execution
- event append

### Planning Runtime

Purpose:

The planning runtime turns active execution goals into task network proposals. It reads active goals from execution goal storage, requests a goal scoped planner projection through a world model port, runs `PlanningRuntime`, lowers composed results through execution lowering, and submits task network mutation commands through the task network command runtime.

Required behavior:

- Read active goals from the goal set runtime or goal query store.
- Request a goal scoped `PlanningWorldStateRequest` through an injected planner projection port.
- Use projection frame provenance from `PlanningWorldStateFrameRef` in every planning attempt record.
- Run deterministic method selection against a verified method library and capability catalog.
- Treat `PlanningResult::Satisfied` as an execution lifecycle candidate and submit satisfaction through the goal set API with a stable command id. Correction 2026-07-25: superseded by the [Goal Ownership Boundary Audit](goal_ownership_boundary_audit.md) and the implemented code — planning reports satisfaction as a diagnostic and never mutates goal lifecycle; the world-model agent owns satisfaction through `AgentGoalMutationCommand`.
- Treat `PlanningResult::Composed` as the only path that may create task network work.
- Lower `ExecutionComposition` into a task network mutation set without reselecting a method.
- Submit the mutation set as `Command::ApplyMutationSet` through the task network command boundary.
- Persist append only planning attempt records for audit and recovery.
- Keep planning attempt records diagnostic. They must not replace goal state or task network state.

It owns:

- planning attempt audit records
- planning dedupe cursor by goal id, goal update sequence, projection frame id, method library identity, and capability catalog identity
- method library snapshot identity used for replay diagnostics
- planning result diagnostics
- deterministic task network command id derivation for lowered compositions

It does not own:

- belief settlement
- world model projection rules
- task network state mutation
- task dispatch
- artifact persistence
- provider execution
- event append
- continuous root driven scheduling

### Task Network Command Runtime

Purpose:

The task network command runtime is the single writer for task network state. It serializes graph mutations, dispatch claims, task outcomes, and publication marks through the `task_network::command::Request` boundary and persists the resulting journal and reduced state.

Required behavior:

- Accept `Command::ApplyMutationSet`, `Command::ClaimReadyTask`, `Command::RecordTaskOutcome`, and `Command::MarkPublication`.
- Validate `network_id`, `base_revision`, `base_state_hash`, and read preconditions before any state change.
- Persist accepted commands, rejected commands, request hashes, command responses, journal records, and latest state snapshots.
- Replay prior command responses by command id when the request hash matches.
- Reject a reused command id when the request hash differs.
- Maintain one monotonic revision stream for graph commits, claims, outcomes, artifact availability, and publication marks.
- Keep `ReadySet` as a derived query over reduced state, not as authority.
- Create pending publication outbox records when task outcomes are accepted.
- Reject stale claim outcomes through lifecycle epoch, claim id, and claim revision checks.
- Rebuild identical reduced state on reopen by replaying the journal and validating the latest snapshot hash.

It owns:

- task network command mailbox semantics
- command request and response identity
- task network journal
- latest reduced network state
- task node lifecycle state
- dispatch claim records
- accepted outcome records
- artifact availability projection derived from accepted successful outcomes
- durable publication outbox records

It does not own:

- capability invocation
- provider calls
- task artifact repository writes
- world model updates
- event spine append
- evidence mapping
- goal lifecycle state

### Task Dispatch Runtime

Purpose:

The task dispatch runtime performs executable work for ready task nodes. It claims one ready task through the task network command runtime, materializes final task initialization, opens a task artifact repository, runs the task runtime with injected capability invokers, and records the terminal outcome back through the task network command runtime.

Required behavior:

- Query ready tasks from task network state through a read side query.
- Submit `Command::ClaimReadyTask` before any task execution.
- Use the accepted claim as the only authority to execute a task instance.
- Materialize `TaskInitializationPayload` after claim and before building a `TaskExecutor`.
- Block dispatch when initialization is missing, ambiguous, stale, or invalid.
- Open a task scoped artifact repository before task execution begins.
- Persist task init artifacts and emitted artifacts through the task artifact runtime.
- Persist or journal capability invocation attempts before non-idempotent external work.
- Invoke capabilities through injected capability and provider ports.
- Convert task runtime success into `dispatch::OutcomeStatus::Succeeded`.
- Convert task runtime failure into `dispatch::OutcomeStatus::Failed`.
- Include emitted artifacts and task events in `dispatch::Outcome`.
- Submit `Command::RecordTaskOutcome` with claim id, claim revision, lifecycle epoch, and deterministic outcome id.
- Leave canonical execution fact publication to the publication runtime.

It owns:

- worker local dispatch loop state
- task claim request construction
- capability invocation ordering inside one task run
- task runtime bridge from `TaskExecutor` to `dispatch::Outcome`
- recovery policy for in flight claimed tasks
- dispatch diagnostics

It does not own:

- direct task network mutation
- direct goal lifecycle mutation
- planner projection
- event spine append for canonical outcome facts
- publication mark commands except through the publication runtime
- artifact repository schema authority beyond validating handoff to the artifact runtime

### Task Artifact Runtime

Purpose:

The task artifact runtime persists task scoped artifacts and artifact links. It provides durable repositories to task dispatch and exposes artifact snapshots for outcome recording and data flow materialization.

Required behavior:

- Open task scoped repositories through `TaskArtifactRepoFactory`.
- Scope each repository by stable repo id.
- Persist init artifacts, emitted artifacts, failure artifacts, expansion artifacts, and artifact links.
- Reject duplicate artifact ids unless the caller can prove exact idempotent replay.
- Record supersession or lineage links through append only artifact link records.
- Validate link endpoints in the same durable transaction as link writes.
- Load repository snapshots from durable records on reopen.
- Validate artifact record schema version on open.
- Validate link metadata sequence on open.
- Flush durable writes before a task outcome includes those artifact records.
- Expose artifacts by id, capability instance, and output slot for materialization and outcome construction.

It owns:

- task artifact records
- artifact link records
- repository metadata
- artifact repository schema validation
- artifact append identity
- durable repo flush

It does not own:

- task network artifact availability authority
- outcome publication
- event append
- provider retry policy
- belief evidence mapping
- goal satisfaction

### Publication Runtime

Purpose:

The publication runtime publishes durable execution facts from task network outbox records into the event append API. It scans pending and failed publication records, builds canonical event envelopes, appends them idempotently, and marks publication state through the task network command runtime.

Required behavior:

- Read pending and failed `Publication` records from task network state.
- Process selected publications in deterministic publication id order.
- Build canonical event envelopes from publication records.
- Derive deterministic event record ids from publication ids.
- Call `EventAppendSink.append_envelope_idempotent` for each selected publication.
- Mark a publication as published only after append succeeds.
- Store returned event sequence on the publication mark when available.
- Mark append failure as retryable failed publication state.
- Keep failure diagnostics in the bridge report.
- Report budget exhaustion when the selected limit leaves more retryable publications.
- Never map execution facts into world model evidence.

It owns:

- publication scan order
- publication tick reports
- event envelope construction for execution task outcomes
- deterministic event id derivation
- append failure classification
- publication mark command construction

It does not own:

- event sequence allocation
- world model replay cursor
- belief evidence promotion
- goal satisfaction review
- task outcome state mutation except through `Command::MarkPublication`
- root supervisor retry memory

## Owned State

| Actor | Authoritative State | Diagnostic State |
| --- | --- | --- |
| goal set runtime | goal records, lifecycle state, command outcomes, source identity index | validation errors and acceptance reports |
| planning runtime | none beyond append only attempt and dedupe cursor records | candidate reports, projection warnings, method diagnostics, lowering diagnostics |
| task network command runtime | journal, reduced state, command outcomes, claims, outcomes, publications | rejection summaries and replay validation reports |
| task dispatch runtime | durable invocation journal when external work is attempted | worker leases, in flight diagnostics, task run summaries |
| task artifact runtime | artifact records, artifact links, repo metadata | schema validation reports and repair diagnostics |
| publication runtime | publication state through task network mark commands and appended event records through the event append API | publication bridge reports and append failure summaries |

Planning attempt records are not execution commitment. The task network journal is the durable plan after composition lowering has been accepted.

Worker local memory is never authoritative after restart. Every external side effect must be protected by a durable command, durable invocation record, idempotent external key, or explicit recovery state.

## Required Inputs

| Actor | Inputs |
| --- | --- |
| goal set runtime | `GoalAcceptanceRequest`, execution native lifecycle commands, opened goal store, sequence metadata, tick budget |
| planning runtime | active goals, planner projection port, method library, capability catalog, lowerer, task network command port, optional planning attempt store |
| task network command runtime | command requests, opened task network store, cancellation signal, bounded command budget |
| task dispatch runtime | ready task query, task network command port, task artifact repo factory, capability catalog, capability invokers, provider and context ports, expansion compiler, cancellation signal |
| task artifact runtime | repository open requests, artifact append requests, link append requests, repo flush requests |
| publication runtime | task network state query, task network command port, event append sink, session id, worker id, publication limit, cancellation signal |

The planner projection port is read only from execution. Execution requests a projection frame and consumes its returned `WorldState` plus frame provenance. Execution must not reach into world model stores.

Capability invocation ports are execution dependencies, not execution ownership. Provider implementations remain outside execution.

## Durable Outputs

| Actor | Durable Outputs |
| --- | --- |
| goal set runtime | `ExecutionGoalRecord`, `GoalCommandOutcome`, source identity index entries |
| planning runtime | planning attempt records, optional satisfy command outcomes through goal API, task network command requests through command runtime |
| task network command runtime | command request records, command response records, accepted journal records, latest state snapshot, claim records, outcome records, publication records |
| task dispatch runtime | claim commands, outcome commands, invocation journal records, task run summaries when configured |
| task artifact runtime | `ArtifactRepoRecord`, artifact records, artifact links, repo metadata |
| publication runtime | event spine records through idempotent append, publication published marks, publication failed marks |

Task success and task failure both produce durable outcomes. A failure outcome is a fact. It must not imply goal satisfaction.

Task outcome publication state is the durable cursor for publication. There must not be a separate supervisor-held list of pending publications.

## Ports

Required port families:

- goal command ingress port
- goal query and mutation port
- planner projection source port
- task network command port
- task network state query port
- task artifact repository factory port
- capability invocation port
- context and provider adapter ports
- event append sink port
- supervisor diagnostics port

Port rules:

- All cross domain calls use explicit contracts.
- Root assembly may build port implementations.
- Runtime actors call ports directly after root wiring.
- Ports must not expose another domain internal module as execution authority.
- Ports must support deterministic tests with in memory implementations.
- Ports that trigger external side effects must support idempotency keys or durable replay.

`EventAppendSink` remains an execution callable publication port. It is not a root mediated handoff step.

`PlanningWorldStateRequest` remains the execution side projection request shape. The world model owns how that request becomes a projected `WorldState`.

## Idempotency And Cursor Rules

### Goal Set Runtime

- `metadata.command_id` is the primary command replay key.
- `metadata.source_identity` is the producer dedupe key when present.
- Duplicate delivery of the same command id must return the same persisted outcome.
- Reusing a command id for different intent must be rejected or reported as duplicate conflict by the store boundary.
- Goal records preserve created and updated sequence metadata from command metadata.
- The goal runtime does not own event replay cursors or planner projection cursors.

### Planning Runtime

- A planning attempt identity must include goal id, goal updated sequence, projection frame id, method library identity, capability catalog identity, and planning request id.
- A composed planning result must derive a stable task network command id from composition id and goal planning attempt identity.
- Repeating the same planning attempt must either return the same diagnostic attempt record or replay the same task network command outcome.
- A new projection frame can trigger a new planning attempt for the same active goal.
- A changed goal updated sequence can trigger a new planning attempt.
- Planning attempt cursors are local to planning and must not replace active goal queries.
- Planning must not own a world model replay cursor.

### Task Network Command Runtime

- Every command request requires a caller supplied `command_id`.
- The store persists request hash and response by command id for accepted and rejected commands.
- A matching duplicate request replays the prior response.
- A mismatched duplicate command id is rejected.
- `base_revision`, `base_state_hash`, and read preconditions are checked against latest state.
- Revisions are the only authoritative task network cursor.
- Claims must include task instance id, lifecycle epoch, claim id, claim revision, worker id, and idempotency key.
- Outcomes must name the current claim fence or be rejected as stale.

### Task Dispatch Runtime

- Dispatch claim ids must be stable for the selected task instance, lifecycle epoch, worker identity, and attempt identity.
- Outcome ids must be stable for the accepted claim and terminal task result.
- Non-idempotent capability invocation must not start before an invocation journal record or equivalent durable barrier exists.
- External provider calls must carry idempotency keys when the provider supports them.
- Retrying a task after restart must observe existing claim and invocation state before doing more external work.
- Worker local task run cursors are advisory only.

### Task Artifact Runtime

- Repo id scopes all artifact and link identity.
- Artifact id is unique within a repo.
- Artifact ids should be derived from task instance id, lifecycle epoch, capability instance id, invocation id, and output slot where possible.
- Exact duplicate artifact append can be treated as replay only when the persisted record matches byte for byte.
- Divergent duplicate artifact append must be rejected.
- Link append order is durable repo metadata, not process memory.
- Artifact repos do not own event cursors.

### Publication Runtime

- Publication state is the durable publication cursor.
- Pending and failed states are retryable.
- Published state is terminal for that publication id.
- Event record id is deterministic from publication id.
- Event append uses idempotent append.
- Publication mark command id is deterministic from publication id, result class, and event record id or failure hash.
- If append succeeds and marking fails, retry must append idempotently and attempt the mark again.
- The publication runtime does not own world model replay cursors.

## Recovery Rules

### Goal Set Runtime

- On start, open the durable goal store supplied by assembly.
- Reopen must preserve goal records, command outcomes, and source identity dedupe.
- If a command outcome exists without required record context, actor start must fail with a fatal diagnostic.
- Active goal reads after reopen must be deterministic.
- Satisfied, abandoned, and suspended goals must not be reactivated by replay.

### Planning Runtime

- On start, load the last planning attempt cursor and method library identity.
- If no planning attempt store exists yet, planning may recompute from active goals and rely on task network command idempotency.
- If a prior attempt produced a task network command but no recorded command response, resubmit the same command id.
- If method library identity changes, record a new planning attempt instead of overwriting prior diagnostics.
- If projection is unavailable, record an indeterminate or port failure diagnostic and leave goal lifecycle unchanged.
- If lowering fails, record diagnostics and do not submit a task network mutation command.

### Task Network Command Runtime

- On start, replay journal records in revision order.
- Load command request and response identity before accepting new commands.
- Validate latest snapshot network id, revision, and state hash against replayed state.
- Treat snapshot mismatch, missing command response, or journal decode failure as fatal actor start errors.
- Do not attempt external side effects during replay.
- Rejected command outcomes must replay after reopen.

### Task Dispatch Runtime

- On start, inspect task network state for in flight claims owned by this runtime identity and claims whose lease has expired under configured policy.
- If an invocation journal has a terminal result but no task outcome command, reconstruct and submit the outcome.
- If artifacts were persisted but the outcome command was not accepted, reopen the artifact repo and submit the outcome with the same outcome id.
- If external work may still be running, query the provider when a query contract exists.
- If external work cannot be queried and was non-idempotent, record an unknown or failed outcome according to configured execution class policy rather than blindly rerunning.
- If the task node lifecycle epoch changed, do not record the old outcome.
- If initialization materialization no longer validates, release no new claim and report a retryable diagnostic.

### Task Artifact Runtime

- On start, open repositories lazily by repo id through the factory.
- Reopen must load artifacts and links in deterministic order.
- Unsupported artifact record schema version is fatal for that repo.
- Stored link endpoints must reference existing artifacts.
- Repo metadata sequence must not lag stored link sequence.
- Durable writes must be flushed before the dispatch runtime records an outcome containing those artifacts.

### Publication Runtime

- On start, scan task network publication records rather than using a root cursor.
- Retry pending and failed publications in deterministic id order.
- If a publication is already published, skip it.
- If event append returns the same sequence for an idempotent retry, mark the publication published with that sequence.
- If append fails, record failed state and keep the publication retryable.
- If marking is rejected due to current state already being published, treat the item as complete after refetch.
- If marking is rejected for any other reason, report a fatal publication issue and leave state unchanged beyond any persisted failed mark.

## Supervisor Contract

Root assembly and supervisor may:

- open product stores
- construct concrete port implementations
- pass stores and ports into runtime actors
- start actors
- request bounded ticks
- provide cancellation tokens
- flush product stores
- restart failed actors according to policy
- collect health and diagnostic reports
- report actor input cursor, output cursor, item counts, retryable errors, fatal errors, and budget exhaustion

Root assembly and supervisor must not:

- sequence each semantic flywheel handoff as product logic
- store active goal progress
- store planner projection cursors
- store task network revisions as authority
- store dispatch claim state
- store pending publication lists
- map task outcomes into world model evidence
- mark goals satisfied
- rewrite domain command outcomes

Every runtime actor must expose a bounded tick or service loop contract with:

- stable actor id
- startup validation
- graceful shutdown
- flush on stop when the actor owns durable writes
- deterministic report shape
- retryable and fatal error separation
- no hidden supervisor-held cursor requirement

Supervisor reports are diagnostics. They are not correctness state.

## Forbidden Responsibilities

Global forbidden responsibilities:

- No production root ordered convergence loop.
- No root mediated goal handoff after ports are wired.
- No root mediated outcome handoff after publication is wired; the root replay port is removed and ingestion consumes published events through the world-model mapping.
- No runtime actor may reach into another domain internal modules.
- No actor may bypass the task network command boundary for graph, claim, outcome, or publication state.
- No actor may publish canonical execution facts except the publication runtime.
- No actor may write runtime state inside the target workspace path.
- No new production module may use `mod.rs`.

Actor specific forbidden responsibilities:

| Actor | Forbidden Responsibilities |
| --- | --- |
| goal set runtime | world model imports, agent judgment, planning, task dispatch, publication |
| planning runtime | belief semantics, task network direct writes, provider calls, event append |
| task network command runtime | capability invocation, artifact repo writes, world model evidence, event append |
| task dispatch runtime | graph mutation outside commands, goal mutation, canonical event append, publication marking |
| task artifact runtime | task lifecycle decisions, event append, publication state, goal satisfaction |
| publication runtime | world model replay, evidence promotion, goal satisfaction, task execution |

## Implementation Phases

### Phase 0 Requirements Alignment

Goal:

Keep execution runtime design aligned with the flywheel architecture before code work.

Requirements:

- Adopt this document as the execution runtime requirements source.
- Keep root assembly language limited to store opening, port wiring, lifecycle, and diagnostics.
- Keep execution runtime actor names domain specific.
- Preserve the modern module layout rule and do not introduce `mod.rs`.

Exit criteria:

- Runtime design docs no longer imply root owns execution progress memory.
- Execution runtime actors are named and scoped by domain behavior.

### Phase 1 Goal Set Runtime

Goal:

Wrap the existing public goal API and durable goal store in a runtime actor contract.

Requirements:

- Add a goal command ingress runtime around `GoalSetApi`.
- Use `PersistentGoalSetStore` for authoritative state.
- Expose active goal reads for planning.
- Return bounded diagnostics to supervisor.
- Prove command id replay and source identity dedupe across reopen.

Exit criteria:

- Agent delivered neutral goal requests store active execution goals.
- Duplicate goal command delivery does not create duplicate goals.
- Reopen preserves active and satisfied lifecycle state.

### Phase 2 Planning Runtime

Goal:

Turn active goals into task network mutation commands without root mediation.

Requirements:

- Read active goals from goal runtime state.
- Request world state projection through a port.
- Run `PlanningRuntime`.
- Persist planning attempt diagnostics.
- Lower composed results into task network mutation sets.
- Submit mutation commands through the task network command runtime.
- Use stable command ids so restart can resubmit safely.

Exit criteria:

- An active docs freshness goal can produce a task network mutation command.
- Planning does not write task network state directly.
- Projection failure and no applicable method leave goal lifecycle unchanged.

### Phase 3 Task Network Command Runtime

Goal:

Wrap `SledTaskNetworkStore` behind a serialized command actor.

Requirements:

- Accept command requests from planning, dispatch, and publication actors.
- Persist accepted and rejected command outcomes.
- Expose read side queries for state, ready tasks, and publication records.
- Preserve monotonic revision ordering.
- Validate snapshot state hash on reopen.

Exit criteria:

- Graph commits, claims, outcomes, and publication marks share one durable revision stream.
- Duplicate command replay is stable across restart.
- Stale claims and stale publication marks are rejected.

### Phase 4 Task Artifact Runtime

Goal:

Make task artifacts durable before outcome recording becomes authoritative.

Requirements:

- Open task artifact repositories through `TaskArtifactRepoFactory`.
- Persist emitted artifacts and links.
- Validate artifact and link identity on reopen.
- Flush repository writes before outcome command submission.
- Define exact duplicate artifact replay behavior.

Exit criteria:

- Emitted artifacts survive reopen.
- Artifact links survive reopen in deterministic order.
- A task outcome never references artifacts that failed durable persistence.

### Phase 5 Task Dispatch Runtime

Goal:

Claim ready tasks, run task runtime work, and record outcomes through the command boundary.

Requirements:

- Claim ready tasks before execution.
- Materialize init payloads after claim.
- Build `TaskExecutor` from accepted claim and durable artifact repo.
- Persist invocation attempts before non-idempotent external work.
- Convert success and failure into fenced outcomes.
- Submit outcome commands through task network command runtime.

Exit criteria:

- Claimed tasks execute through the existing task runtime.
- Successful outcomes unblock downstream data flow tasks.
- Failed outcomes are recorded and publishable without satisfying goals.
- Restart does not blindly rerun non-idempotent external work.

### Phase 6 Publication Runtime

Goal:

Publish pending execution facts from the task network outbox into the event append API.

Requirements:

- Scan pending and failed publications from task network state.
- Build deterministic event envelopes.
- Append through `EventAppendSink`.
- Mark publication state through task network commands.
- Record retryable append failures.
- Avoid supervisor-held publication cursors.

Exit criteria:

- One pending task success publication appends exactly one event across retries.
- One pending task failure publication appends a failure fact across retries.
- Publication state is marked published only after append success.

### Phase 7 Recovery And Supervisor Integration

Goal:

Connect actors to root lifecycle without moving domain semantics into root.

Requirements:

- Add actor startup validation.
- Add bounded tick reports.
- Add graceful shutdown and flush contracts.
- Add restart policy hooks.
- Keep all progress cursors in execution stores.

Exit criteria:

- Root can start, stop, and restart execution actors.
- Actors resume from durable state after restart.
- Root reports diagnostics without storing execution authority.

## Verification Requirements

Required static checks:

```sh
cargo fmt --check
cargo check -p meld-execution --all-targets
cargo clippy -p meld-execution --all-targets -- -D warnings
rg -n "mod.rs" crates/meld-execution/src design/plan/integration
rg -n "centralized semantic loop|convergence loop" design/plan/integration crates/meld-execution/src
```

Required focused tests:

```sh
cargo test -p meld-execution --test goals
cargo test -p meld-execution --test planning_runtime
cargo test -p meld-execution --test composition_lowering
cargo test -p meld-execution --test task_network_command
cargo test -p meld-execution --test task_network_store
cargo test -p meld-execution --test task_network_readiness
cargo test -p meld-execution --test task_network_initialization
cargo test -p meld-execution --test task_network_dispatch
cargo test -p meld-execution --test task_network_execution_bridge
cargo test -p meld-execution --test task_network_publication_bridge
cargo test -p meld-execution --all-targets
```

Required integration proof:

- accept one producer neutral goal command through execution public API
- reopen and read the active goal
- request a planner projection through a port
- produce a composed planning result
- lower to a task network mutation command
- commit the mutation command through the task network command runtime
- claim a ready task
- materialize task initialization
- execute through task runtime with injected invokers
- persist artifacts before outcome recording
- record success outcome and pending publication
- publish pending success fact idempotently to the event append API
- record failure outcome and pending publication
- publish pending failure fact idempotently to the event append API
- restart between every durable boundary and prove replay reaches the same state

Required negative proof:

- duplicate goal command does not create a second goal
- no applicable method does not create task network work
- invalid lowering does not mutate task network state
- stale claim outcome is rejected
- missing artifact blocks data flow dispatch
- append failure does not mark a publication published
- task failure does not satisfy a goal
- supervisor-held pending publication cursor is absent

## Acceptance Criteria

Execution runtime requirements are satisfied when all statements below are true.

- Goal commands enter execution only through the public goal API or an execution owned goal command sink.
- Goal lifecycle state is authoritative in execution storage.
- Planning reads active goals from execution and planner projection through a port.
- Planning does not own belief semantics or world model cursors.
- Composed planning results become task network mutation commands through execution lowering.
- The task network command runtime is the only writer for graph, claim, outcome, and publication state.
- Task dispatch claims tasks before execution and records fenced outcomes after execution.
- Task artifacts are durably persisted before they appear in task outcomes.
- Task success and task failure both create publishable execution facts.
- Publication appends pending execution facts through the event append API using idempotent event identities.
- Publication marks happen only after append success.
- Restart after any accepted durable boundary can resume without duplicate goals, duplicate graph commits, duplicate task outcomes, duplicate artifacts, or duplicate event facts.
- Root assembly wires ports and supervises lifecycle only.
- No execution runtime implementation requires `mod.rs`.
- Verification includes focused unit tests, reopen tests, duplicate delivery tests, failure tests, publication retry tests, and one end to end flywheel proof.
