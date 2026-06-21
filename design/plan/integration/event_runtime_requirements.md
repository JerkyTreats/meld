# Event Runtime Requirements

Date: 2026-06-17
Status: proposed
Scope: event runtime actors for the first durable flywheel

## Purpose

This document defines detailed requirements for the event runtime only.

The event runtime is the durable event spine for the flywheel. Execution publishes prepared domain event envelopes into events. World model runtimes read sequenced event records from events. The event runtime owns append, runtime wide sequence assignment, idempotency, bounded replay, subscription semantics, retention boundaries, and event runtime diagnostics.

Root assembly wires concrete ports, opens stores, starts or wraps runtime handles, supervises lifecycle, flushes storage, and reports health. Root assembly does not mediate each semantic handoff after ports are wired.

The first durable flywheel target is:

```text
world model runtime
-> execution runtime
-> event runtime
-> world model runtime
```

The event runtime must make this loop restart safe without taking over execution meaning or world model meaning.

## Runtime Actors

The first durable flywheel treats these event owned actors as distinct responsibilities even when they share one process and one `EventStore`.

- idempotent append runtime
- sequence assignment runtime
- subscription and bounded replay runtime
- event retention and diagnostics runtime

These actors may be implemented as one synchronous facade at first. Their requirements remain separate so later supervised handles can split them without changing cross domain semantics.

## Owned State

The event runtime owns only event domain state.

- canonical event records keyed by runtime wide sequence
- sequence metadata used to assign the next event sequence
- record id index used for idempotent append lookup
- session index used for compatibility and diagnostic reads
- legacy event compatibility reads for existing telemetry records
- in process bounded event queue state when the queue based ingress path is used
- event runtime diagnostic counters and last observed append or replay summaries
- retention floor metadata when retention is introduced
- storage flush state exposed only as operational health

The event runtime does not own consumer projection cursors. Graph, belief, planner, and agent cursors remain in world model stores. Publication retry state remains in execution task network stores. Lifecycle leases and heartbeats remain in the root supervisor store.

## Required Inputs

Append inputs:

- prepared `EventEnvelope` values from execution publication and other producer domains
- stable `record_id` for every cross domain durable flywheel publication
- producer owned `domain_id`, `stream_id`, `event_type`, payload, object refs, relation refs, and optional content hash
- append mode selected by the port, with durable flywheel publication using the idempotent append path
- cancellation or shutdown signal supplied by the supervised handle

Replay inputs:

- caller supplied `after_seq` cursor boundary
- caller supplied batch limit
- optional caller identity for diagnostics and future retention safety checks
- optional filters only when the event runtime can apply them without interpreting payload meaning

Supervisor inputs:

- opened event store handle or database handle
- runtime id
- lease or lifecycle context
- configured queue capacity and replay batch limits
- flush request during shutdown

## Durable Outputs

Append outputs:

- one persisted `EventRecord` for each accepted new envelope
- one runtime wide `seq` assigned by the event runtime
- record id index entry for idempotent lookup when `record_id` is present
- session index entry for session scoped compatibility reads
- updated sequence metadata that never moves behind persisted records
- append result containing the existing or newly assigned sequence

Replay outputs:

- ordered batches of persisted `EventRecord` values
- a deterministic upper sequence boundary that the caller may persist only after caller owned output is durable
- typed replay errors when the requested cursor is outside retained history
- diagnostics describing batch size, lower cursor, upper cursor, and retention boundary

Diagnostic outputs:

- bounded runtime reports for supervisor visibility
- retryable and fatal diagnostic classifications
- no product fact appended to the canonical event spine unless an explicit producer policy later promotes it

## Ports

The first durable flywheel needs narrow event ports.

Append sink:

- accepts one prepared envelope through an idempotent append operation
- returns the existing or newly assigned event sequence
- rejects or diagnoses malformed durable flywheel publication envelopes before reporting semantic success
- is callable directly by execution publication runtime
- does not require root to inspect the publication payload

Batch append sink:

- accepts a bounded batch of prepared envelopes
- applies the same idempotency and sequence rules as single append
- reports per item outcome when partial retry is possible
- preserves producer order only as input order for append attempts, while canonical order is always assigned sequence order

Replay source:

- accepts `after_seq` and `limit`
- returns records with `seq` greater than `after_seq`
- returns at most `limit` records
- returns records in ascending runtime sequence order
- is callable directly by world model graph, belief, and evidence ingestion runtimes

Subscription source:

- is a durable pull subscription for the first flywheel
- uses bounded replay as the delivery mechanism
- treats the caller supplied cursor as the delivery boundary
- never advances a consumer cursor on behalf of the consumer
- may later add push wakeups, but push wakeups must not become correctness state

Flush and health port:

- flushes event storage before shutdown is acknowledged
- exposes last assigned sequence, last flush status, queue health, replay health, and retention floor
- exposes no event payload interpretation

## Idempotency And Sequence Rules

The event runtime must provide idempotent append for durable flywheel publications.

- `record_id` is the idempotency key.
- If a record id already exists, idempotent append returns the original sequence and writes no duplicate event record.
- If a record id is absent on a durable flywheel publication, the append actor must report a producer contract violation or an equivalent fatal diagnostic.
- Compatibility append paths may continue to accept envelopes without `record_id`, but they must not be used for execution publication in the first durable flywheel.
- The execution publication bridge must derive deterministic record ids from publication identity.
- A retry after execution crashes between append and publication mark must return the original event sequence.
- Idempotency lookup must survive process restart.
- Idempotency lookup must repair a missing record id index from canonical event records when possible.

Sequence assignment is event runtime authority.

- Only the event runtime assigns canonical runtime wide event sequence values.
- Persisted event records must never share a sequence.
- Persisted event records must be readable in ascending sequence order across sessions.
- Sequence assignment must be monotonic across restart.
- Sequence metadata must never lag behind the greatest persisted sequence plus one after recovery or append repair.
- Duplicate idempotent append must not create a second persisted record.
- Consumer correctness must not depend on a producer local clock, session local clock, or root scheduling order.
- Consumers use sequence as the shared temporal clock for cross domain questions.

The event runtime must not reinterpret producer payloads while assigning sequence. The payload, objects, relations, and content hash remain producer owned data carried by the envelope.

## Replay And Cursor Boundary

The replay boundary is exclusive.

- A request with `after_seq` equal to `N` returns only records with sequence greater than `N`.
- A request with limit zero returns an empty batch.
- A successful bounded replay batch is deterministic for the same retained event spine state.
- The returned records are already committed event facts.
- The caller may advance its durable cursor only after caller owned derived state is durable.
- World model graph reduction advances graph replay cursors in the world model store.
- World model belief evidence ingestion advances evidence source cursors in the world model store.
- Event runtime diagnostics may report replay boundaries but must not store world model cursor truth.
- Session scoped reads are compatibility and diagnostic reads, not the primary world model catch up contract.

Retention must preserve cursor safety.

- The first durable flywheel retains the canonical event spine without deleting records.
- Any future retention policy must publish a retained lower sequence boundary.
- If a caller asks for a cursor below the retained lower boundary, replay must return a typed retention gap error instead of silently skipping history.
- Retention may prune session summaries or operational diagnostics only when canonical replay safety is preserved.

## Recovery Rules

Startup recovery:

- open all event store trees before exposing append or replay ports
- load sequence metadata
- scan canonical event records when metadata is missing or stale
- repair sequence metadata so the next assigned sequence is greater than every persisted sequence
- preserve legacy readable records without moving their meaning into new domain semantics
- expose recovery diagnostics to the supervisor

Append recovery:

- append record, sequence metadata, session index, and record id index must commit as one logical durable append
- if a record exists but the record id index is missing, idempotent lookup must repair the index and return the existing sequence
- if append fails before a durable record exists, the producer may retry with the same record id
- if append succeeds and downstream producer marking fails, retry with the same record id must return the existing sequence
- append failure must not let execution mark a publication as published

Replay recovery:

- replay after restart reads from durable event records only
- in process queue state is not semantic truth
- queued but unappended envelopes are producer responsibility unless the producer owns a durable outbox
- world model replay resumes from world model cursors and event records

Shutdown recovery:

- supervised shutdown must stop accepting new append work or fence new work before flush
- flush event store after in flight accepted appends reach a safe point
- report final sequence and flush status
- release supervisor lease only after durable flush attempt has completed

## Supervisor Contract

The root supervisor may wrap the event runtime in a lifecycle handle.

Allowed supervisor behavior:

- acquire a lease for the event runtime id before starting a long lived event handle
- start or expose the concrete append, replay, subscription, flush, and health ports
- monitor queue health, append health, replay health, last sequence, and flush status
- restart the event handle after retryable failure
- flush the event store during shutdown
- include event runtime reports in operator status

Forbidden supervisor behavior:

- assign event sequence
- store event consumer cursors as semantic progress
- choose which execution publication should become world model evidence
- inspect event payloads to decide goal satisfaction
- duplicate the event record id index
- delete canonical event history through an operational retention shortcut
- run a root ordered event loop that mediates execution to world model handoff

Suggested runtime id:

```text
events.ledger
```

Suggested actor ids:

```text
events.ledger.append
events.ledger.sequence
events.ledger.replay
events.ledger.retention
```

## Forbidden Responsibilities

The event runtime must not own these responsibilities:

- execution goal lifecycle
- planning policy
- task dispatch
- task outcome meaning
- task publication outbox state
- world model graph materialization meaning
- belief evidence mapping
- belief settlement
- agent curation
- goal satisfaction review
- root lifecycle leases
- CLI routing
- telemetry sink routing outside event store compatibility
- broad sensory raw stream retention

The event runtime may carry object refs and relation refs, but it must not decide what those refs mean beyond validation required by event contracts.

## Event Retention And Diagnostics

The first durable flywheel must prefer retention safety over storage reduction.

Retention requirements:

- keep canonical event records for the product root
- keep record id index entries as long as the referenced canonical records are retained
- keep sequence metadata durably consistent with retained canonical records
- keep session index entries or rebuild them from canonical records when needed
- never prune canonical records as part of session summary pruning
- define a future retention floor before any event deletion feature is enabled
- require every consumer domain to expose durable cursor checkpoints before retention can use consumer progress

Diagnostics requirements:

- append diagnostics include attempted envelopes, committed records, duplicate idempotent hits, missing record id violations, retryable storage errors, fatal validation errors, and budget exhaustion
- sequence diagnostics include last assigned sequence, highest persisted sequence, metadata repair count, and duplicate sequence detection
- replay diagnostics include caller id when available, input cursor, output cursor, records returned, limit, retention floor, and retention gap errors
- queue diagnostics include capacity, enqueue failures, drain count, and disconnected receiver state
- flush diagnostics include last flush time, flush result, and storage error summary
- diagnostics must be bounded and serializable for supervisor reports
- diagnostics must not become product world facts by default

## Implementation Phases

### Phase ER-0 Contract Alignment

Clarify that `meld-events` is the owner of append, sequence, idempotency, replay, and subscription semantics.

Exit criteria:

- design docs describe root as assembly and supervisor only
- execution publication still calls an event append sink directly
- world model replay still calls an event replay source directly

### Phase ER-1 Append Runtime

Harden the durable flywheel append path around idempotent envelopes.

Requirements:

- expose one append sink for execution publication runtime
- require stable record ids for execution publication events
- return the event sequence from append
- report duplicate record id hits as successful idempotent outcomes
- report missing record id for durable publication as a producer contract violation

Exit criteria:

- retrying the same execution publication appends one event record
- append success gives execution enough sequence identity to mark the publication published
- append failure leaves execution publication retryable

### Phase ER-2 Sequence Recovery

Make sequence assignment recovery explicit.

Requirements:

- recover next sequence from metadata and canonical records
- repair stale sequence metadata before accepting new appends
- prevent duplicate persisted sequence keys
- expose repair diagnostics

Exit criteria:

- reopened event runtime assigns a sequence greater than existing records
- stale metadata cannot cause overwrite or duplicate sequence
- idempotent duplicate after reopen returns the original sequence

### Phase ER-3 Bounded Replay And Subscription

Define the first subscription contract as pull based bounded replay.

Requirements:

- expose replay after sequence with limit
- return ascending runtime sequence order
- keep consumer cursor advancement outside event runtime
- surface retention gaps as typed errors
- support world model catch up without root mediation

Exit criteria:

- world model can catch up from event sequence zero to the latest retained record
- repeated replay from the same cursor returns the same batch when the spine is unchanged
- cursor advancement remains in world model state

### Phase ER-4 Retention And Diagnostics

Add safe retention defaults and supervisor visible diagnostics.

Requirements:

- retain canonical spine records for the first durable flywheel
- expose retention floor even when the floor is zero
- expose bounded append, replay, sequence, queue, and flush diagnostics
- keep supervisor lifecycle records outside the canonical event spine by default

Exit criteria:

- session pruning cannot remove canonical event records
- operator status can explain append and replay health
- diagnostics do not duplicate semantic domain state

### Phase ER-5 Supervisor Wiring

Wire concrete event ports through root assembly and supervisor lifecycle.

Requirements:

- root assembly opens event store
- root assembly passes append sink to execution runtime
- root assembly passes replay source to world model runtime
- supervisor starts or wraps event runtime with `events.ledger`
- supervisor flushes event store on shutdown

Exit criteria:

- root assembly does not inspect event payloads
- root supervisor does not assign sequence
- flywheel handoff works through direct ports

## Verification Requirements

Existing focused verification:

```sh
cargo test -p meld-events --test event_store_contracts
cargo test --test integration_tests runtime_wide_sequence_is_monotonic
cargo test --test integration_tests idempotent_append_reuses_existing_record_id
cargo test --test integration_tests session_prune_does_not_delete_canonical_spine_history
cargo test -p meld-execution --test task_network_publication_bridge publication_bridge_appends_pending_task_outcome_once
```

New or strengthened verification:

```text
event_runtime_reopens_with_monotonic_sequence
event_runtime_rejects_durable_publication_without_record_id
event_runtime_repairs_stale_sequence_metadata
event_runtime_replay_reports_retention_gap
event_runtime_subscription_does_not_advance_consumer_cursor
event_runtime_shutdown_flushes_before_lease_release
minimal_runtime_flywheel_turn_persists_and_satisfies_goal
failure_outcome_does_not_satisfy_goal
```

Static checks:

```sh
rg -n "supervisor-held source cursor|shared cursor table" src design/plan/integration
rg -n "mod.rs" crates/meld-events src
```

The implementation must preserve the modern Rust module layout and must not add `mod.rs`.

## Acceptance Criteria

The event runtime requirements are satisfied when these statements are true.

- Execution publication appends through an event owned idempotent append sink.
- Each execution publication event has a stable record id and appends at most one canonical event record.
- Append returns the original sequence on duplicate retry across process restart.
- Event sequence is runtime wide, monotonic, durable, and assigned only by the event runtime.
- World model consumes event records through bounded replay or subscription source ports.
- World model owns and advances its replay cursors only after world model output is durable.
- Root assembly wires concrete event ports but does not mediate semantic event handoff.
- Root supervisor manages lifecycle, leases, health, restart, and flush without owning event meaning.
- Canonical event history is retained for the first durable flywheel.
- Retention gaps are explicit errors before any deletion based retention is enabled.
- Diagnostics are bounded, operational, and not product world facts by default.
- The minimal durable flywheel proof can reopen after event append and continue from stores.
- A failed execution outcome can publish a failure event without creating false goal satisfaction.
