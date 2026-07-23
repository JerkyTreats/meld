# Durable Flywheel Vertical Proof Requirements

Date: 2026-06-17
Status: historical characterization, superseded for runtime completion
Scope: vertical proof requirements for Phase 5 Event Led Feedback, Phase 6 Deterministic Proof Harness, and Phase 7 Failure Path Proof

Authority: [Runtime Completion Ground Map](runtime_completion_ground_map.md) and [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) replace this document for current implementation. Retain it only as evidence of the earlier deterministic one-turn contracts. Its synthetic work result, direct proof driver, mandatory one-turn sequence, and failure-path scope are not current completion requirements.

## Purpose

This document defines the detailed requirements for the vertical durable flywheel proof.

The proof covers one deterministic `docs_freshness` turn. It exists to prove that direct domain handoff, checkpoint reopen, event led feedback, idempotent replay, and failure publication all work through durable product stores.

The vertical proof must show this path:

```text
world model runtime
-> execution runtime
-> event runtime
-> world model runtime
-> execution runtime
```

The final execution step is the satisfaction mutation for the success path only.

## Non Product Architecture Boundary

The deterministic proof driver is test support. It is not product runtime architecture, not a product scheduler, and not the center of the flywheel.

The proof driver may call bounded domain runtimes in a deterministic order. It may choose budgets, stop at named checkpoint labels, flush touched stores, drop opened runtime values, reopen from `ProductStorageLayout`, and collect diagnostic reports.

The proof driver must not own semantic progress. It must not carry goal records, belief views, planner projections, task network state, publication records, event records, promoted evidence, mutation commands, or domain cursors across checkpoints.

Root `meld` remains assembly and supervision. It opens stores, wires ports, supervises lifecycle, flushes storage, and reports diagnostics. Domain crates remain authoritative for graph replay, belief evidence, agent decisions, goal lifecycle, planning, task network state, task outcomes, publication state, event sequence, and event replay.

Deleting the proof harness must not require any product runtime semantic change.

## Source Contracts

The proof requirements are constrained by these source contracts:

- `design/plan/integration/durable_flywheel_runtime_phase_design.md`
- `design/plan/integration/runtime_requirements.md`
- `design/plan/integration/world_model_runtime_requirements.md`
- `design/plan/integration/execution_runtime_requirements.md`
- `design/plan/integration/event_runtime_requirements.md`
- `design/plan/integration/rtg_6_reopen_checkpoint_proof_contract.md`
- `design/plan/integration/durable_runtime_first_slice.md`
- `design/plan/integration/durable_runtime_pre_implementation_gaps.md`

If this document and an owning domain requirement conflict, the owning domain requirement wins for domain semantics. This document may only tighten the integration proof expectations.

## Fixture Constants

The physical seed config is detailed in [Docs Freshness Physical Configuration Requirements](docs_freshness_physical_configuration_requirements.md).

The vertical proof must use a single deterministic fixture object. The fixture may be small, but every semantic identity used by the proof must be fixed before runtime assembly runs.

Required one turn constants:

- product storage root
- `docs_freshness` subject object reference
- belief key for docs freshness
- branch scope
- perspective key
- belief family configuration id
- belief source kind for docs freshness evidence
- curation rule id
- curation threshold
- directive id
- directive text
- seed agent id
- subscription id
- fixture goal source identity
- fixture goal command id
- expected execution goal id
- method library identity
- docs method id
- capability catalog identity
- task package or workflow asset id
- planning request id
- composition id
- task network id `network-docs`
- task instance id
- task lifecycle epoch
- claim id
- claim worker id
- task outcome id for success
- task outcome id for failure
- task artifact repo id
- required success artifact type `docs_patch`
- publication id for success
- publication id for failure
- publication event type for success `execution.task.succeeded`
- publication event type for failure as a pinned fixture value
- event session id
- event worker id
- evidence mapping id for successful docs patch events
- evidence schema id for docs freshness support
- satisfaction review sequence
- satisfaction decision id
- satisfaction mutation command id
- expected final success lifecycle
- expected final failure lifecycle

Required derivation rules:

- Goal command id is deterministic from agent, subject, branch, dimension, target condition, source kind, and belief revision identity.
- Planning attempt identity includes goal id, goal updated sequence, projection frame id, method library identity, capability catalog identity, and planning request id.
- Task network command id is deterministic from composition id and planning attempt identity.
- Claim id is deterministic from task instance id, lifecycle epoch, worker identity, and attempt identity.
- Outcome id is deterministic from accepted claim and terminal task result.
- Event record id is deterministic from publication id.
- Publication mark command id is deterministic from publication id, result class, and event record id or failure hash.
- Evidence id includes event id or event sequence, source kind, mapping id, subject, and evidence schema id.
- Satisfaction decision id is deterministic from agent, subscription, goal id, review sequence, projection version, and dedupe key.
- Satisfaction mutation command id is deterministic from the same review identity.

The fixture constants and product root are the only correctness inputs allowed to cross checkpoint boundaries.

## Direct Handoff Sequence

The proof must exercise the same direct ports used by product runtime assembly.

The world model to execution handoff starts when graph and belief state create a low confidence docs freshness view. The world model agent curation runtime persists an `AgentCurationDecision`, derives the fixture goal command id, submits the command through the execution owned goal command sink, stores the sink result identity, and advances its delivery cursor only after the durable barriers are complete.

The execution runtime accepts the goal through the execution goal API. The goal set runtime persists one active goal and source identity dedupe record. Planning then reads active goals from execution storage and planner projection through a world model port. Planning may create task work only by lowering a composed docs method result into a task network command and submitting that command through the execution task network command runtime.

Task execution claims ready work through the task network command boundary before doing task work. The task runtime writes artifacts through the task artifact runtime before recording an outcome. Success records a succeeded outcome with a `docs_patch` artifact. Failure records a failed outcome and failure payload. Both outcomes create publishable task network publication state.

The execution to events handoff is the publication runtime. It scans pending and failed publications from task network state in deterministic publication id order, builds canonical event envelopes, calls `EventAppendSink.append_envelope_idempotent`, and marks publication state published only after append success. The event sequence returned by append must be stored on the published mark when available.

The events to world model handoff is bounded replay. World model graph replay and event evidence ingestion read only from the event replay source. Event runtime never advances world model cursors. World model graph and evidence cursors move only after world model output is durable.

The world model satisfaction handoff happens after replay and belief reassessment. Satisfaction curation reloads active goal summaries from execution, recomputes or verifies planner projection from current graph and belief stores, persists a satisfaction decision, submits the deterministic mutation command through execution goal mutation sink when and only when the projected world state supports satisfaction, stores the sink result identity, and advances the review cursor only after the required barriers are complete.

## Event Led Feedback Requirements

Execution outcome facts must reach the world model only through the canonical event spine.

The success path requires all statements below:

- The task outcome publication appends one canonical event through idempotent event append.
- Graph replay reads the appended event through bounded replay and advances the graph cursor only after traversal writes are durable.
- Event evidence ingestion reads the same event through bounded replay and applies the configured mapping for docs freshness.
- The success event produces promoted evidence and an idempotent assignment for the docs freshness belief key.
- Belief assessment produces a current belief view that crosses the configured satisfaction threshold.
- Satisfaction curation uses the updated projection and active goal summary from execution before emitting any mutation.
- The satisfaction decision is durable before execution goal mutation.
- The final satisfied lifecycle is read from execution storage.

The failure path requires all statements below:

- The failed task outcome creates publishable execution fact state.
- The publication runtime appends the failure fact through idempotent event append.
- Graph replay may index the failure fact as an event fact.
- Event evidence ingestion must not create support evidence for docs freshness from the failure fact.
- The failure fact must either create a durable rejection record, a neutral diagnostic, or a no assignment result according to the mapping policy.
- Belief assessment must not fabricate support confidence from the failure fact.
- Satisfaction curation must record an absorbed or indeterminate decision and must emit no satisfaction mutation.
- The active goal remains active in execution storage.

Root assembly must not hand task outcome objects, promoted evidence, belief views, or satisfaction commands directly across these boundaries.

## Proof Driver Responsibilities

The proof driver may:

- create or receive the product storage root
- load product runtime assembly through `ProductStorageLayout` and `OpenProductStores`
- install the deterministic fixture constants and deterministic adapters
- call bounded domain runtime operations in a deterministic sequence
- pass configured work budgets into bounded operations
- stop at required checkpoint labels
- flush always open product stores through the product boundary
- flush per network task stores before treating a checkpoint as durable
- drop all opened stores, runtime handles, adapters, and domain values at each checkpoint
- reopen stores from the same product root
- collect bounded diagnostic reports for later inspection
- assert correctness only through owning domain query APIs
- run success and failure variants with the same fixture identity family

The proof driver must not:

- store active goal progress
- store graph replay cursor
- store event evidence cursor
- store belief assessment cursor
- store belief view
- store planner projection
- store agent curation decision as authority
- store satisfaction decision as authority
- store task network revision as authority
- store dispatch claim as authority
- store task outcome as authority
- store task artifact records as authority
- store pending publication list
- store publication id selected before reopen as authority
- store event record or event sequence as authority
- store promoted evidence as authority
- decide whether a task outcome satisfies a goal
- map execution facts into world model evidence
- mutate execution goals outside the execution goal API
- mutate task network state outside the task network command boundary
- append execution facts outside the execution publication runtime
- advance world model cursors from event runtime diagnostics
- choose product runtime architecture based on harness call order

## Checkpoint Protocol

Each checkpoint has the same protocol:

1. A bounded domain operation writes durable state through the owning command or append boundary.
2. Every touched store is flushed.
3. Per network task stores are flushed before the product boundary flush is considered complete.
4. All opened stores, runtime values, adapters, and semantic local variables are dropped.
5. Stores are reopened from `ProductStorageLayout`.
6. Assertions read through domain query APIs.
7. The next bounded turn reconstructs all semantic inputs from stores and ports.

Reports may be retained across checkpoints only as diagnostics. They may prove that a bounded operation returned a report shape. They may not prove semantic correctness.

## Checkpoint Labels

The harness must support these exact checkpoint labels:

- `after_goal_acceptance`
- `after_pending_publication`
- `after_publication_append_before_satisfaction`

The success and failure variants use the same label names. The fixture variant determines whether the publication payload is success or failure.

## Checkpoint State

### `after_goal_acceptance`

State written before checkpoint:

- world model graph seed records exist
- directive record exists
- seed agent record exists and references the directive
- agent subscription exists
- low confidence docs freshness belief view exists
- goal curation decision is durable before execution receives the command
- execution goal store contains one active accepted docs freshness goal
- execution goal command outcome is durable
- execution source identity dedupe entry is durable

Stores flushed before drop:

- world model store
- agent store
- belief store
- execution goal store
- product stores opened by `OpenProductStores`

Assertions after reopen:

- expected goal id exists in execution storage
- lifecycle is `Active`
- source command id equals the fixture goal command id
- source identity equals the fixture goal source identity
- agent decision exists by dedupe key
- active goal query returns only the expected `docs_freshness` goal
- no graph cursor, belief view, goal object, curation output, or subscription cursor from before reopen is used by the next turn

Expected continuation from label:

- planning reloads the active goal from execution storage
- planner projection is queried through the world model port
- duplicate goal delivery returns the stored goal command outcome and creates no second goal

### `after_pending_publication`

Success state written before checkpoint:

- active goal was reloaded from execution storage
- planning composed the docs method
- task network command boundary accepted the task mutation
- task claim is recorded
- task artifacts are durable before outcome recording
- succeeded task outcome is recorded
- exactly one pending success publication exists in task network state
- pending publication event type is `execution.task.succeeded`
- pending publication payload includes artifact type `docs_patch`

Failure state written before checkpoint:

- active goal was reloaded from execution storage
- planning composed the docs method
- task network command boundary accepted the task mutation
- task claim is recorded
- failed task outcome is recorded
- exactly one pending failure publication exists in task network state
- failure publication event type equals the pinned failure fixture value
- failure publication payload carries failure facts and does not carry docs freshness support evidence

Stores flushed before drop:

- execution goal store
- task network store for `network-docs`
- task artifact store when task artifacts are written
- product stores opened by `OpenProductStores`

Assertions after reopen:

- task network state reloads from journal and latest snapshot
- task status is succeeded for the success variant or failed for the failure variant
- outcome exists in task network state
- exactly one pending publication exists
- publication id equals the deterministic fixture publication id for the variant
- no planning result, task network state clone, dispatch claim, task outcome, artifact record, or selected publication object from before reopen is used by the next turn

Expected continuation from label:

- publication runtime scans pending publications from task network state
- publication runtime processes selected publications in deterministic publication id order
- duplicate task network mutation command replays the stored response
- duplicate artifact append in the success variant is accepted only when byte identical
- divergent artifact duplicate is rejected

### `after_publication_append_before_satisfaction`

Success state written before checkpoint:

- publication runtime appended the success event idempotently
- event spine contains one canonical event record for the publication
- task network publication is marked published only after append success
- published state carries event sequence when available
- event record id matches the task network publication id derivation

Failure state written before checkpoint:

- publication runtime appended the failure event idempotently
- event spine contains one canonical event record for the failure publication
- task network failure publication is marked published only after append success
- published state carries event sequence when available
- event record id matches the task network publication id derivation

Stores flushed before drop:

- event spine
- task network store for `network-docs`
- world model store if graph replay has already touched it in the bounded turn
- execution goal store if a prior goal query has touched it in the bounded turn
- product stores opened by `OpenProductStores`

Assertions after reopen:

- event spine contains exactly one canonical event record for the publication record id
- duplicate append with the same record id returns the original sequence and writes no duplicate event record
- task network publication state is published and terminal for that publication id
- publication state is still the only execution publication cursor
- no event record, publication object, promoted evidence, updated belief view, satisfaction mutation command, or event cursor from before reopen is used by the next turn

Expected success continuation from label:

- world model graph replay reads the event through bounded replay
- event evidence ingestion rebuilds docs evidence from the reopened event record
- evidence assignment and dirty key scheduling are durable before cursor advancement
- belief reassessment updates docs freshness belief
- satisfaction curation persists its decision before execution mutation
- final execution goal lifecycle is satisfied at the fixture satisfaction review sequence

Expected failure continuation from label:

- world model graph replay reads the failure event through bounded replay
- event evidence ingestion records rejection, neutral result, or no assignment according to mapping policy
- no support evidence is created for docs freshness
- belief reassessment does not cross the satisfaction threshold due to failure facts
- satisfaction curation emits no mutation
- execution goal lifecycle remains active

## Success Path Acceptance

The success proof is accepted only when every statement below is true:

- product assembly opens all proof stores through the product layout
- world model agent curation persists the goal decision before goal sink submission is considered complete
- execution stores exactly one active docs freshness goal
- planning reads active goals from execution and projection from world model
- planning submits task network work only through execution command boundaries
- task dispatch records a fenced successful outcome
- task artifacts are durable before the successful outcome references them
- exactly one pending success publication is created
- publication appends exactly one success event across initial delivery and retries
- event append returns the original sequence on duplicate retry
- world model consumes the execution fact only through event replay
- evidence ingestion creates exactly one docs freshness support assignment for the success fact
- duplicate event replay creates no duplicate evidence, assignment, or belief revision meaning
- satisfaction decision is durable before execution mutation
- execution records the goal as satisfied
- final lifecycle is read from execution storage after the last reopen
- every checkpoint assertion uses domain query APIs
- reports are used only to check diagnostics, absence of fatal errors, and bounded progress shape

## Failure Path Acceptance

The failure proof is accepted only when every statement below is true:

- the same fixture family can drive a failure variant without changing product architecture
- task dispatch records a fenced failed outcome
- failed outcome creates pending publication state
- publication appends the failure fact through idempotent event append
- task network marks the failure publication published only after append success
- event replay exposes the failure fact to world model runtimes
- graph replay may index the failure event without treating it as docs freshness support
- event evidence ingestion creates no docs freshness support evidence
- failure mapping produces a durable rejection, neutral diagnostic, or no assignment according to policy
- satisfaction curation records absorbed or indeterminate decision when reviewed
- satisfaction curation emits no mutation command for the failure fact
- duplicate failure event replay creates no support evidence
- execution goal lifecycle remains active after failure review
- no diagnostic report is used as the reason the goal remains active

## Idempotency And Duplicate Replay Checks

Required duplicate checks:

- duplicate goal command delivery returns the stored goal command outcome and creates no second goal
- duplicate goal command id with different intent is rejected or reported as duplicate conflict
- duplicate planning attempt replays the prior task network command outcome or returns the prior diagnostic attempt record
- duplicate task network command request with matching hash replays the prior response
- duplicate task network command id with mismatched request hash is rejected
- duplicate task claim observes claim fence and does not execute stale work
- duplicate task outcome with the same outcome id replays or is absorbed according to task network command rules
- stale claim outcome is rejected
- duplicate exact artifact append is accepted only when persisted bytes match
- divergent artifact duplicate is rejected
- duplicate publication append returns the original event sequence and writes no second event record
- retry after append success and mark failure marks the existing publication using the original event sequence
- duplicate event replay over the same range creates no duplicate graph commit, evidence assignment, belief revision meaning, or cursor movement
- duplicate satisfaction review returns the stored decision and emits no second mutation command
- satisfaction sink retry after restart uses the same mutation command id
- failure fact duplicate replay never creates docs freshness support evidence

Every duplicate check must assert final state through owning stores, not through retained local objects.

## Diagnostics Requirements

Each bounded operation used by the proof must return or expose a bounded diagnostic report.

Required common report fields:

- actor id
- input cursor or input revision
- output cursor or output revision
- items attempted
- items committed
- retryable errors
- fatal errors
- budget exhausted flag

Additional required report fields by area:

- event append report includes attempted envelopes, committed records, duplicate idempotent hits, missing record id violations, retryable storage errors, and fatal validation errors
- event replay report includes caller id when available, input cursor, output cursor, records returned, limit, retention floor, and retention gap errors
- graph replay report includes input event sequence and output event sequence
- belief report includes dirty key count, committed revisions, rejected evidence count, and budget outcome
- goal curation report includes delivered subscriptions, committed decisions, sink submissions, sink result recoveries, and cursor movement
- planning report includes active goals scanned, projection warnings, composed results, lowered commands, and command replay hits
- task network report includes command attempts, accepted commands, rejected commands, latest revision, and replay validation errors
- task dispatch report includes claims attempted, claims accepted, terminal outcomes, provider retryable errors, and fatal execution errors
- artifact report includes repositories opened, artifact appends, duplicate exact replays, divergent duplicates, and schema validation failures
- publication report includes scanned publications, attempted appends, committed appends, duplicate event hits, published marks, failed marks, and budget exhaustion
- satisfaction report includes active goals reviewed, decisions persisted, mutation attempts, mutation replay hits, absorbed reviews, indeterminate reviews, and cursor movement

Diagnostics may be asserted for shape, item counts, absence of fatal errors, retryable error classification, and budget exhaustion. Diagnostics must not be used as correctness state for event sequence, graph cursor, belief state, active goal lifecycle, task network revision, publication state, evidence assignment, or satisfaction outcome.

## Verification Commands

Required proof commands:

```sh
cargo test --test integration_tests minimal_runtime_flywheel_turn_persists_and_satisfies_goal
cargo test --test integration_tests failure_outcome_does_not_satisfy_goal
cargo test --test integration_tests docs_freshness_reopens_after_goal_acceptance_from_product_stores
cargo test --test integration_tests docs_freshness_reopens_after_pending_publication_from_product_stores
cargo test --test integration_tests docs_freshness_reopens_after_publication_append_before_satisfaction
```

Required domain commands:

```sh
cargo test -p meld-world-model
cargo test -p meld-execution --test goals
cargo test -p meld-execution --test planning_runtime
cargo test -p meld-execution --test task_network_command
cargo test -p meld-execution --test task_network_dispatch
cargo test -p meld-execution --test task_network_publication_bridge
cargo test -p meld-events --test event_store_contracts
cargo test runtime::contracts
```

Required source scans:

```sh
rg -n "mod.rs" crates/meld-world-model/src crates/meld-execution/src crates/meld-events/src src design/plan/integration
rg -n "centralized semantic loop|convergence loop" design/plan/integration src crates/meld-world-model/src crates/meld-execution/src crates/meld-events/src
rg -n "supervisor-held semantic cursor|shared cursor table|supervisor-held source cursor" src design/plan/integration
```

The source scans must not report new production architecture that makes root a semantic flywheel scheduler. The phrase `convergence loop` may remain only in docs that reject root centered architecture.

## Required Test Names

The proof harness must include or preserve these test names:

- `minimal_runtime_flywheel_turn_persists_and_satisfies_goal`
- `failure_outcome_does_not_satisfy_goal`
- `docs_freshness_reopens_after_goal_acceptance_from_product_stores`
- `docs_freshness_reopens_after_pending_publication_from_product_stores`
- `docs_freshness_reopens_after_publication_append_before_satisfaction`

Recommended supporting test names:

- `vertical_proof_duplicate_goal_delivery_replays_goal_outcome`
- `vertical_proof_duplicate_publication_append_reuses_sequence`
- `vertical_proof_duplicate_event_replay_does_not_duplicate_evidence`
- `vertical_proof_duplicate_satisfaction_review_emits_no_second_mutation`
- `vertical_proof_failure_event_replay_creates_no_support_evidence`
- `vertical_proof_reports_are_not_correctness_state`

## Harness Implementation Phases

These phases are for the proof harness only. They must not create product scheduler architecture.

### Harness Phase 0 Boundary Lock

Create the fixture constants and assertion helpers. The helpers may know expected ids and labels. They must read state through owning query APIs and must not reach into domain internals.

Exit criteria:

- all fixture constants are defined in one place
- assertion helpers accept product root and fixture constants only
- no helper stores semantic progress across checkpoint calls

### Harness Phase 1 Assembly Wrapper

Add test support that opens product runtime assembly and returns direct domain ports and bounded runtime handles.

Exit criteria:

- assembly uses `ProductStorageLayout`
- ports match product direct handoff APIs
- no ordered semantic loop is added to production modules

### Harness Phase 2 Checkpoint Reopen Helper

Add a helper that flushes touched stores, drops all runtime values, reopens from the product root, and runs label specific assertions.

Exit criteria:

- `after_goal_acceptance` assertions pass from stores
- `after_pending_publication` assertions pass from stores
- `after_publication_append_before_satisfaction` assertions pass from stores
- retained inputs across helper calls are limited to product root, fixture constants, checkpoint label, and diagnostic reports

### Harness Phase 3 Success Path Driver

Drive the success path in deterministic bounded turns from goal curation through final satisfaction.

Exit criteria:

- success path test reaches satisfied lifecycle
- all required checkpoint labels are exercised
- every semantic transition uses the direct owning port or command boundary
- final state is read from execution storage after reopen

### Harness Phase 4 Idempotency Suite

Replay duplicate deliveries at each durable boundary and prove stable final state.

Exit criteria:

- duplicate command ids do not create duplicate domain state
- duplicate event appends do not create duplicate event records
- duplicate event replay does not duplicate world model state
- duplicate satisfaction review does not duplicate mutation commands

### Harness Phase 5 Failure Path Driver

Drive the failure variant from task failure through event publication and world model replay.

Exit criteria:

- failure fact publishes through event append
- failure fact reaches world model replay
- no docs freshness support evidence is created
- satisfaction curation emits no mutation
- execution goal remains active

### Harness Phase 6 Diagnostics And Scans

Assert bounded report shape and run source scans that guard the architecture boundary.

Exit criteria:

- reports include required fields
- reports are not used as correctness state
- source scans show no new root semantic scheduler
- proof harness can be identified as test support

## Final Acceptance

The vertical proof is complete when the success path satisfies the goal, the failure path publishes without satisfying the goal, every required checkpoint reopens from product stores, every duplicate replay check passes, diagnostics remain observational, and product runtime semantics live in the owning domains rather than the proof driver.
