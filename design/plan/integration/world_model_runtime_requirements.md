# World Model Runtime Requirements

Date: 2026-06-17
Status: proposed
Scope: world model runtime actors only

## Purpose

This document defines detailed requirements for the world model runtimes that close the durable flywheel from events back to execution.

The world model side of the flywheel is:

```text
event spine
-> graph replay runtime
-> belief assessment runtime
-> planner projection runtime
-> agent goal curation runtime
-> execution goal API
-> execution outcome events
-> event evidence ingestion runtime
-> belief assessment runtime
-> planner projection runtime
-> satisfaction curation runtime
-> execution goal API
```

Root assembly opens stores, wires ports, starts runtime handles, supervises lifecycle, flushes checkpoints, and reports diagnostics. Root assembly does not own graph cursors, belief evidence cursors, belief views, planner decisions, agent delivery cursors, goal curation decisions, satisfaction decisions, active goals, or satisfaction mutation commands.

The required world model runtime actors are:

- graph replay runtime
- belief assessment runtime
- planner projection runtime
- agent goal curation runtime
- event evidence ingestion runtime
- satisfaction curation runtime

## Owned State

### Graph Replay Runtime

The graph replay runtime owns graph materialization from durable event records into traversal state.

Owned state:

- traversal graph objects, relations, anchors, selected anchors, and provenance indexes
- graph reducer cursor, stored as the last durably reduced event sequence
- graph reducer runtime metadata needed to resume replay
- idempotency keys for graph derived events when the reducer emits derived event envelopes
- bounded graph worker report for the latest tick as diagnostic output only

The graph reducer cursor is authoritative only inside the world model graph store. The supervisor may observe it through a report but must not store it as progress memory.

### Belief Assessment Runtime

The belief assessment runtime owns conversion from graph state and assigned evidence into current belief views.

Owned state:

- active runtime belief configuration snapshot hash and policy metadata
- external belief family configuration snapshots
- normalized evidence items
- evidence assignment edges from evidence item to belief key
- dirty belief key state
- assessment leases keyed by belief key and epoch
- belief revisions with prior links, comparator metadata, evidence ids, posterior summary, confidence, uncertainty, freshness, contradiction, status, and source cursor range
- current belief heads and current belief views
- evidence rejection records
- bounded belief worker report for diagnostic use

Belief views are the public output boundary for planner and agent code. Raw evidence, comparator drafts, and leases remain internal to belief.

### Planner Projection Runtime

The planner projection runtime owns projection from public graph and belief reads into execution readable world state.

Owned state:

- projection version
- projection rule configuration
- deterministic projection input selection for subject, dimension, perspective, and branch
- optional projection snapshot records when a runtime tick needs a durable handoff before agent use
- projection diagnostics, source refs, hydration refs, and warnings captured by downstream curation decisions

The runtime does not need to persist a global world state cache for correctness. When a projection is used to create a goal command or satisfaction mutation, the consuming agent decision must durably record projection version, source refs, hydration refs where available, and warnings.

### Agent Goal Curation Runtime

The agent goal curation runtime owns agent activation, subscription delivery, and goal command curation.

Owned state:

- directive records referenced by agents
- seed and activated agent records
- agent subscription records
- agent activation records
- subscription delivery cursor per subscription
- agent curation decisions
- goal command ids derived from deterministic curation input
- persisted execution goal sink result identity for emitted goal commands
- bounded goal curation worker report for diagnostic use

The runtime owns the decision to create a proposed goal command. Execution owns validation, acceptance, lifecycle state, and active goal storage.

### Event Evidence Ingestion Runtime

The event evidence ingestion runtime owns conversion from execution outcome event records into promoted evidence for belief.

Owned state:

- event evidence replay cursor per source mapping, belief family, perspective, and branch
- promoted evidence normalization rejections
- normalized promoted evidence items
- idempotent evidence assignments
- dirty belief key scheduling caused by promoted evidence
- ingestion report with selected event range, attempted records, accepted records, rejected records, newly assigned keys, committed belief revisions, retryable errors, fatal errors, and budget exhaustion

The event spine remains event owned. The ingestion cursor is world model owned because it records which event records have been reduced into world model evidence.

### Satisfaction Curation Runtime

The satisfaction curation runtime owns agent judgment that an active execution goal is satisfied by the current projected world state.

Owned state:

- satisfaction review cursor per agent, subscription, branch, and review source
- persisted agent curation decisions for satisfaction reviews
- deterministic goal mutation command ids
- persisted execution goal sink result identity for satisfaction mutations
- bounded satisfaction curation worker report for diagnostic use

Execution owns the final goal lifecycle mutation. The world model agent owns the satisfaction judgment and must persist that judgment before calling the execution goal API.

## Required Inputs

### Graph Replay Runtime

Required inputs:

- event replay source from the event domain
- optional event append sink for idempotent derived graph events
- traversal store opened by root assembly and owned by world model
- replay budget with maximum item count
- cancellation token or stop request from the supervisor
- reducer configuration and schema version

The runtime reads only event records after its graph reducer cursor. It must not consume execution stores directly.

### Belief Assessment Runtime

Required inputs:

- traversal query facade for graph anchors and provenance
- belief store
- runtime belief configuration snapshot
- perspective key
- branch scope
- dirty belief key list from belief storage
- promoted evidence already durably assigned by ingestion
- bounded assessment budget
- worker owner id for leases

The runtime may assess one requested subject, one dirty key, or a bounded batch of dirty keys. It must use public graph query surfaces and belief store contracts.

### Planner Projection Runtime

Required inputs:

- belief query facade
- traversal query facade
- projection rule configuration
- subject
- dimension id
- perspective key
- branch scope
- projection budget for bounded runtime operation

Projection consumes current public views. It does not read raw evidence, assessment leases, dirty key internals, execution plans, task networks, or task outcomes.

### Agent Goal Curation Runtime

Required inputs:

- agent store
- pending agent subscription deliveries
- belief query facade
- planner query facade or planner projection runtime port
- active goal summary from execution goal API
- goal curation rule configuration
- execution goal command sink
- bounded delivery budget
- cancellation token or stop request

Active goal summaries must be read through an explicit execution goal query port. The world model agent runtime must not read execution goal storage directly.

### Event Evidence Ingestion Runtime

Required inputs:

- event replay source from the event domain
- belief store
- belief assessment runtime port
- runtime belief configuration snapshot
- event to promoted evidence mapping configuration
- perspective key
- branch scope
- bounded event budget
- worker owner id for downstream assessment leases

The runtime consumes execution outcome facts only after they have been appended to the canonical event spine. It must not accept direct in memory outcome objects from execution as correctness input.

### Satisfaction Curation Runtime

Required inputs:

- agent store
- belief query facade
- planner query facade or planner projection runtime port
- active goal summary from execution goal API
- execution goal mutation sink
- satisfaction review trigger based on updated projection, updated belief revision, or event evidence cursor progress
- bounded review budget
- cancellation token or stop request

The runtime must reload active goals from execution before review. It must not carry active goal objects from earlier root assembly steps across a checkpoint.

## Durable Outputs

### Graph Replay Runtime

Durable outputs:

- graph object and relation records
- current anchor records
- anchor provenance records
- graph reducer cursor update after successful replay
- idempotently appended derived graph events where reducer policy requires them
- flushed event and traversal stores after each committed bounded tick

The reducer cursor must move only after traversal writes are durable and any derived event append requirements for the processed input range are handled.

### Belief Assessment Runtime

Durable outputs:

- configuration snapshots and active config metadata
- evidence and assignment records
- assessment lease records and terminal lease status
- belief revisions
- current head movement
- current belief view
- dirty key state updates
- rejection records
- flushed belief store after committed revisions

The current belief view is the durable handoff to planner and agent runtimes.

### Planner Projection Runtime

Durable outputs:

- optional projection snapshot keyed by subject, dimension, perspective, branch, projection version, and source refs
- projection report with input source refs, warning count, output proposition count, and budget outcome
- projection source refs and warnings embedded in any downstream agent curation decision that uses the projection

If no projection snapshot store is introduced, the durable output obligation is satisfied only when each goal or satisfaction decision persists the projection identity it used.

### Agent Goal Curation Runtime

Durable outputs:

- agent curation decision before external goal command submission is considered complete
- persisted execution sink result identity after the goal command is accepted, absorbed, or rejected by execution
- subscription cursor advancement after decision persistence and sink result handling meet the domain contract
- flushed agent store after decision, sink result identity, and cursor update

If the decision produces no goal command, the cursor may advance after the absorbed or indeterminate decision is durably stored.

### Event Evidence Ingestion Runtime

Durable outputs:

- normalized promoted evidence
- evidence assignment records
- rejection records for unmapped or invalid event facts
- dirty key state for affected belief keys
- event evidence cursor advancement after evidence, assignment, rejection, or durable dirty key scheduling is complete
- optional committed belief revisions when the ingestion tick also runs assessment for affected keys
- flushed belief store after cursor movement

The ingestion cursor must not depend on live event objects or root local state.

### Satisfaction Curation Runtime

Durable outputs:

- satisfaction curation decision before execution mutation submission
- deterministic goal mutation command id embedded in the decision when a mutation is required
- persisted execution sink result identity after execution accepts, absorbs, or rejects the mutation command
- review cursor advancement after decision persistence and sink result handling meet the domain contract
- flushed agent store after decision, sink result identity, and cursor update

The execution goal lifecycle update is durable in execution storage, not world model storage.

## Ports

### Event Replay Source

The event replay source is event owned and world model consumed.

Required behavior:

- read event records after a supplied sequence
- honor a bounded item limit
- return stable event records with sequence, id, type, payload, source identity, and transaction metadata
- never hide gaps or reorder records

Graph replay and event evidence ingestion both consume this port. Each runtime owns its own cursor.

### Event Append Sink

The event append sink is event owned.

Required behavior:

- append event envelopes idempotently
- return the canonical event sequence
- preserve stable event id identity
- report duplicate append as the existing canonical sequence when possible

Graph replay may use this port for derived graph events. Execution publication uses this port outside this document.

### Belief Assessment Port

The belief assessment port is world model owned.

Required behavior:

- assess one dirty key
- recover expired assessment leases
- return committed revision identity and confidence summary
- report backpressure when another live lease owns the key

Event evidence ingestion uses this port after it schedules or observes affected keys.

### Planner Projection Port

The planner projection port is world model owned.

Required behavior:

- project current world state for subject, dimension, perspective, and branch
- return ground world state, projection version, source refs, hydration refs, and warnings
- preserve deterministic ordering
- report missing belief as typed warning rather than fabricated support

Goal curation and satisfaction curation use this port.

### Execution Goal Query Port

The execution goal query port is execution owned.

Required behavior:

- return active goal summaries filtered by agent, subject, branch, dimension, and target where possible
- include goal id, agent id, lifecycle, target, source, and source identity needed for dedupe
- read from execution goal storage
- avoid exposing mutable execution internals

Goal curation and satisfaction curation use this port to avoid duplicate goal creation and to evaluate active goal satisfaction.

### Execution Goal Command Sink

The execution goal command sink is execution owned.

Required behavior:

- accept proposed goal commands idempotently by command id and dedupe key
- validate ground goal targets
- persist accepted goal lifecycle state in execution
- return a stable outcome with command id, goal id, lifecycle result, and duplicate or rejection status

Agent goal curation calls this sink directly through an injected port.

### Execution Goal Mutation Sink

The execution goal mutation sink is execution owned.

Required behavior:

- accept satisfaction mutation commands idempotently by command id and goal id
- validate that the target goal exists and is mutable
- persist lifecycle mutation in execution
- return a stable outcome with command id, goal id, resulting lifecycle, and duplicate or rejection status

Satisfaction curation calls this sink directly through an injected port.

## Idempotency And Cursor Rules

### Shared Rules

- Every runtime cursor belongs to the domain store that gives it meaning.
- A supervisor lease is never a semantic cursor.
- A bounded tick may return a diagnostic report, but report contents are not correctness state.
- Cursor movement must happen after all durable writes needed to make replay idempotent.
- Duplicate delivery must return the prior durable decision or durable sink result instead of creating a second command.
- Runtime ids and command ids must be stable enough for replay and duplicate submission.

### Graph Replay Runtime

- Read after the graph reducer cursor.
- Apply events in canonical event sequence order.
- Persist traversal writes before moving the reducer cursor.
- When a bounded tick appends derived events and source input remains after the budget, do not advance the reducer cursor past unprocessed source events.
- Append derived events with stable ids so replay does not duplicate them.
- A repeated tick over the same event range must produce the same traversal state and same derived event identities.

### Belief Assessment Runtime

- Assessment lease identity must include belief key and epoch.
- Evidence ids must be deterministic from source identity, schema mapping, key candidate, and source cursor range.
- Assignment insertion must be idempotent.
- Revision identity must be deterministic for the same prior revision, config snapshot, evidence set, comparator version, and cursor range.
- Dirty key state must not be cleared until the revision and view are durable.
- The assessment cursor range must be embedded in the revision.
- Duplicate assessment after recovery must observe existing evidence and either return the existing revision or commit the same revision identity.

### Planner Projection Runtime

- Projection output must be deterministic for the same graph state, belief view, perspective, branch, projection rules, and projection version.
- Projection snapshots, when used, must be keyed by projection identity and source refs.
- Missing input must produce typed warning or indeterminate state, not fabricated confidence.
- Downstream agent decisions must persist the projection version and source refs they used.

### Agent Goal Curation Runtime

- Subscription delivery cursor advances only after the curation decision is durable and any required goal command sink result is durable.
- Goal command id must be deterministic from agent, subject, branch, dimension, target condition, source kind, and belief revision identity.
- A duplicate delivery with an existing decision must not emit a second goal command.
- If a decision exists with no stored sink result, the runtime must retry the same command id through the execution sink.
- Active goal dedupe must use execution goal summaries, not world model memory.
- Cursor regression must be rejected.

### Event Evidence Ingestion Runtime

- Ingestion reads after its own event evidence cursor.
- The cursor is scoped by mapping id, belief family, perspective, and branch.
- Evidence ids must include event id or event sequence, source kind, mapping id, subject, and evidence schema id.
- Rejected records must be durably recorded before the cursor advances past them.
- Accepted records must write evidence, assignments, and dirty key scheduling before the cursor advances.
- If downstream assessment is backpressured, ingestion may advance after dirty key scheduling is durable.
- Replaying the same event range must not create duplicate evidence, duplicate assignments, or duplicate revisions.

### Satisfaction Curation Runtime

- Review cursor advances only after the satisfaction decision is durable and any required mutation sink result is durable.
- Satisfaction decision id must be deterministic from agent, subscription, goal id when present, review sequence, projection version, and dedupe key.
- Mutation command id must be deterministic from the same review identity.
- Duplicate reviews must return the stored decision and must not emit a second mutation command.
- If a decision exists with no stored sink result, the runtime must retry the same mutation command id.
- Active goals must be reloaded from execution for each review tick.

## Recovery Rules

### Shared Rules

- On startup, each runtime reads its domain store and resumes from domain owned cursors and pending work indexes.
- Expired supervisor leases may be marked by root supervisor, but semantic retry state remains in domain stores.
- Runtimes must tolerate process loss after any durable write barrier.
- Runtimes must flush their owned stores at safe points before reporting committed progress.
- Reports after recovery must describe resumed work without becoming proof inputs.

### Graph Replay Runtime

- Resume from the graph reducer cursor.
- Reprocess any event whose traversal writes were not followed by cursor movement.
- Treat duplicate derived event append as success.
- Preserve traversal cursor correctness when a prior bounded tick stopped before source input was exhausted.

### Belief Assessment Runtime

- Recover expired assessment leases and reschedule affected belief keys.
- Rebuild assessment input from evidence and assignments in the belief store.
- Do not require live normalized evidence objects from the failed process.
- Leave current belief head unchanged until a replacement revision and view are durable.
- Preserve rejection records for invalid input across restart.

### Planner Projection Runtime

- Recompute projection from graph and belief queries after restart.
- Reuse durable projection snapshots only as cache or audit records.
- Never treat an old projection snapshot as current unless source refs still match the current graph and belief heads.

### Agent Goal Curation Runtime

- Resume pending deliveries from subscription records and belief revision state.
- Reuse stored decisions for duplicate deliveries.
- Retry missing goal sink result with the same command id.
- Advance cursor only after sink result recovery succeeds or the existing decision needs no sink call.
- Rebuild active goal summaries through the execution query port.

### Event Evidence Ingestion Runtime

- Resume from the event evidence cursor.
- Reprocess the current event if the prior tick wrote evidence but did not advance the cursor.
- Use idempotent evidence and assignment writes to absorb replay.
- Assess dirty keys that were scheduled before failure.
- Never depend on root held promoted evidence objects.

### Satisfaction Curation Runtime

- Resume from satisfaction review cursor and pending satisfaction decisions.
- Recompute projection and active goal summaries after restart.
- Reuse stored satisfaction decisions for duplicate reviews.
- Retry missing mutation sink result with the same command id.
- Do not mark execution goals satisfied unless the execution mutation sink accepts or confirms the mutation.

## Supervisor Contract

The supervisor owns lifecycle only.

Allowed supervisor state:

- runtime ids
- desired runtime state
- runtime instance records
- runtime leases
- heartbeat records
- health snapshots
- restart counters
- cancellation tokens
- last bounded diagnostic report
- operator status views

Forbidden supervisor state:

- graph reducer cursor as progress memory
- event evidence cursor as progress memory
- belief evidence cursor as progress memory
- belief view as progress memory
- planner projection as progress memory
- active goals as progress memory
- curation decision as progress memory
- goal command or mutation command as progress memory
- promoted evidence as progress memory

Required world model runtime ids:

- `world_model.graph.replay`
- `world_model.belief.assessment`
- `world_model.planner.projection`
- `world_model.agent.goal_curation`
- `world_model.belief.event_evidence_ingestion`
- `world_model.agent.satisfaction_curation`

Each runtime handle must support:

- start after supervisor lease acquisition
- stop at a domain safe point
- heartbeat with bounded diagnostic report
- flush at checkpoint boundary
- retryable failure reporting
- fatal failure reporting
- restart from domain stores after replacement lease acquisition

The supervisor must not call pure curation functions and then apply execution mutations itself. It may start the satisfaction curation runtime, wire the execution mutation sink, and observe the resulting report.

## Forbidden Responsibilities

### Graph Replay Runtime

Forbidden responsibilities:

- accepting execution outcomes directly from execution storage
- creating goals
- assessing belief
- selecting methods or tasks
- mutating execution goal lifecycle
- storing supervisor lease state as graph progress

### Belief Assessment Runtime

Forbidden responsibilities:

- creating goals
- dispatching tasks
- invoking capabilities
- appending execution outcome events
- reading execution task internals
- exposing raw evidence or leases to planner and agent code
- hardcoding belief family names or evidence schemas as Rust branches

### Planner Projection Runtime

Forbidden responsibilities:

- creating or accepting goals
- selecting methods
- planning tasks
- dispatching work
- publishing outcomes
- deciding satisfaction
- reading raw belief internals
- fabricating support when inputs are missing

### Agent Goal Curation Runtime

Forbidden responsibilities:

- writing execution goal storage directly
- owning execution goal lifecycle
- dispatching tasks
- mutating task networks
- reading raw belief evidence or assessment leases
- advancing subscription cursor before required durable barriers
- using root local active goal state for dedupe

### Event Evidence Ingestion Runtime

Forbidden responsibilities:

- consuming direct in memory execution outcomes as correctness input
- appending canonical execution outcome events
- creating goals
- satisfying goals
- mutating task networks
- treating event append sequence as owned by world model
- advancing cursor before evidence or rejection durability

### Satisfaction Curation Runtime

Forbidden responsibilities:

- writing execution goal lifecycle state directly
- bypassing durable satisfaction decision storage
- accepting stale active goal objects from root local state
- mutating task networks
- appending execution publication events
- marking satisfaction on failed or indeterminate projection
- advancing review cursor before required durable barriers

## Implementation Phases

### Phase WMR 0 Requirements And Boundary Lock

Goal:

- freeze actor boundaries, ports, runtime ids, cursor ownership, and durable output rules

Required work:

- add this requirements document
- align future runtime design docs to root assembly as wiring and supervision only
- confirm no production centralized semantic loop owns progress
- confirm each actor has a bounded report shape

Exit criteria:

- every world model runtime has an owner, input set, output set, cursor rule, and forbidden responsibility list

### Phase WMR 1 Port Contracts

Goal:

- define stable ports needed by world model runtimes without exposing foreign domain internals

Required work:

- event replay source
- optional graph derived event append sink
- belief assessment port
- planner projection port
- execution goal query port
- execution goal command sink
- execution goal mutation sink
- report contracts shared by world model runtime handles

Exit criteria:

- root assembly can wire every port without owning runtime decisions
- world model code imports only public contracts from other domains

### Phase WMR 2 Graph Replay Runtime

Goal:

- run bounded event to graph replay through a supervised world model handle

Required work:

- use event replay source instead of root handed event lists
- persist traversal writes before reducer cursor movement
- preserve derived event idempotency
- expose bounded catch up report
- support cancellation at a safe point

Exit criteria:

- graph replay resumes correctly after process loss
- graph replay does not require root to remember the last event

### Phase WMR 3 Belief Assessment Runtime

Goal:

- run dirty key and subject assessment as a supervised runtime

Required work:

- preserve existing config snapshot, evidence, assignment, lease, revision, and view durability
- add bounded dirty key selection where needed
- recover expired leases at startup
- expose bounded assessment reports
- keep planner handoff through `BeliefView`

Exit criteria:

- belief assessment resumes from belief store only
- duplicate replay does not duplicate revisions or evidence

### Phase WMR 4 Planner Projection Runtime

Goal:

- provide a runtime projection surface for curation and satisfaction

Required work:

- wrap existing planner query behavior behind a runtime port
- define optional projection snapshot persistence if needed for bounded worker handoff
- ensure curation decisions persist projection identity
- expose projection reports

Exit criteria:

- curation can use projection without reading belief internals
- satisfaction can recompute projection after reopen

### Phase WMR 5 Agent Goal Curation Runtime

Goal:

- let world model agents submit curated goals through execution goal API

Required work:

- read pending subscription deliveries from durable agent state
- assemble inputs through belief and planner facades
- read active goal summaries through execution goal query port
- persist curation decision
- submit goal command through execution goal command sink
- persist sink result identity
- advance subscription cursor after durable barriers
- expose curation report

Exit criteria:

- root does not map each curated goal in the product path
- duplicate delivery cannot create duplicate goals

### Phase WMR 6 Event Evidence Ingestion Runtime

Goal:

- consume execution outcome events through event replay and turn them into belief evidence

Required work:

- add event evidence cursor records
- load event to evidence mappings from configuration
- normalize promoted evidence from event records
- persist evidence, assignments, rejections, and dirty key scheduling
- call belief assessment port for affected keys when budget allows
- expose ingestion report

Exit criteria:

- execution outcomes affect belief only after event append
- root does not hand execution outcomes directly to belief

### Phase WMR 7 Satisfaction Curation Runtime

Goal:

- let world model agents submit satisfaction mutations through execution goal API

Required work:

- create bounded satisfaction review selection
- reload active goals through execution goal query port
- recompute planner projection
- persist satisfaction decision before mutation
- submit mutation through execution goal mutation sink
- persist sink result identity
- advance review cursor after durable barriers
- expose satisfaction report

Exit criteria:

- root never calls pure satisfaction curation and then mutates execution as a shortcut
- satisfaction survives process loss between decision and execution mutation

### Phase WMR 8 Supervised Assembly Proof

Goal:

- prove the world model runtimes work as domain actors under root supervision

Required work:

- root assembly opens stores and wires ports
- supervisor starts enabled world model runtime handles after lease acquisition
- deterministic proof may drive bounded ticks for reproducibility
- proof reopens stores after goal acceptance, after pending publication, and after publication append before satisfaction
- final satisfaction reads execution lifecycle from execution storage

Exit criteria:

- domain runtimes carry the flywheel momentum
- root assembly can be dropped and recreated without losing correctness state

## Verification Requirements

### Source Scans

Required scans:

```sh
rg -n "mod.rs" crates/meld-world-model src design/plan
rg -n "supervisor-held semantic cursor|shared cursor table" src design/plan/integration
rg -n "centralized semantic loop|convergence loop" design/plan/integration
```

Expected result:

- no new `mod.rs`
- no supervisor-held semantic cursor language in production runtime design
- deterministic proof harness language remains clearly marked as test support

### Graph Replay Tests

Required tests:

- bounded replay applies events in sequence order
- cursor moves only after traversal writes are durable
- derived event append is idempotent
- budget exhaustion does not skip unprocessed source events
- reopen resumes from traversal store cursor
- report fields include actor id, input event sequence, output event sequence, attempted count, committed count, errors, and budget exhaustion

### Belief Assessment Tests

Required tests:

- config snapshot is persisted before assessment output
- evidence and assignments are idempotent
- lease recovery reschedules dirty keys
- revision and view persist before lease completion
- duplicate assessment does not duplicate revision meaning
- missing or invalid evidence records rejection instead of fabricated belief
- reopen rebuilds current view from belief store

### Planner Projection Tests

Required tests:

- projection uses only public graph and belief queries
- all emitted propositions are ground
- missing belief returns warning or indeterminate state
- deterministic inputs produce deterministic world state and source refs
- curation decisions persist projection identity
- optional projection snapshots are cache only and not a substitute for current source refs

### Agent Goal Curation Tests

Required tests:

- pending delivery produces one durable curation decision
- low confidence projection emits one goal command through execution sink
- high confidence projection records absorbed decision
- missing belief projection records indeterminate decision
- duplicate delivery returns stored decision and emits no duplicate command
- sink retry uses the same command id after restart
- subscription cursor advances only after decision and sink result durability
- active goal dedupe reads execution summary through port

### Event Evidence Ingestion Tests

Required tests:

- ingestion reads execution outcomes only from event replay
- mapped event produces promoted evidence and assignment
- unmapped event records rejection or no assignment according to policy
- cursor advances only after evidence, assignment, rejection, or dirty key scheduling is durable
- backpressured assessment leaves dirty key durable for later assessment
- duplicate event replay creates no duplicate evidence or assignment
- reopen resumes from ingestion cursor and dirty key state

### Satisfaction Curation Tests

Required tests:

- active satisfied goal produces durable satisfaction decision before mutation sink call
- unsatisfied goal records absorbed decision
- indeterminate goal records indeterminate decision
- duplicate review returns stored decision and emits no duplicate mutation
- sink retry uses the same mutation command id after restart
- review cursor advances only after decision and sink result durability
- final goal lifecycle is read from execution storage

### Integration Tests

Required tests:

- durable flywheel proof reopens after goal acceptance
- durable flywheel proof reopens after pending publication
- durable flywheel proof reopens after publication append before satisfaction
- failed execution outcome event does not create satisfaction support
- root assembly does not carry forbidden local state across reopen
- supervisor reports health without reading semantic internals

Recommended commands:

```sh
cargo test -p meld-world-model
cargo test --test integration_tests
cargo test runtime::supervisor
cargo test runtime::contracts
cargo clippy -p meld-world-model -- -D warnings
cargo clippy --workspace -- -D warnings
```

## Acceptance Criteria

The world model runtime requirements are satisfied when:

- graph replay consumes events through event replay and persists graph cursor in world model graph storage
- belief assessment consumes graph and promoted evidence through world model stores and persists current belief views
- planner projection consumes public graph and belief views and produces ground world state with durable source identity in downstream decisions
- agent goal curation submits goal commands through execution goal API after durable decision persistence
- event evidence ingestion consumes execution outcomes only through event replay and updates belief evidence through idempotent assignments
- satisfaction curation submits goal mutation commands through execution goal API after durable satisfaction decision persistence
- each runtime has a stable runtime id, bounded report, safe stop point, and recovery path from domain stores
- root assembly only opens stores, wires ports, starts and stops runtimes, flushes checkpoints, and reports diagnostics
- root supervisor never stores semantic progress as runtime correctness state
- deterministic proof can drop and reopen runtime values at required checkpoints and still converge to the satisfied or active goal lifecycle stored by execution
