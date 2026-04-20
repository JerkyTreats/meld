# Event Extraction Plan

Date: 2026-04-17
Status: completed
Scope: phased implementation plan for extracting canonical event ownership into `events`

## Overview

Objective:

- move canonical event ownership out of `telemetry`
- establish minimal `session` ownership outside canonical append
- leave `telemetry` as a temporary downstream compatibility surface

Outcome:

- `events` owns append, replay, sequencing, compatibility, and canonical contracts
- `session` owns minimal runtime lifecycle records for command runs
- correctness code depends on `events` rather than telemetry owned storage and runtime
- `telemetry` becomes a downstream compatibility layer only

## Development Phases

### Phase 0

Goal:

- freeze the design baseline and workflow artifact before code changes

Tasks:

- add this `PLAN.md`
- commit the current extraction design docs

Exit criteria:

- only planned design files are changed
- design docs cover events, session, reducer taxonomy, and downstream compatibility posture

Verification:

- `git status --short`

### Phase 1

Goal:

- add canonical `events` types and compatibility shims with no behavior change

Tasks:

- add `src/events.rs`
- add `src/events/contracts.rs`
- add `src/events/compat.rs`
- add `src/events/runtime.rs`
- add `src/events/store.rs`
- add `src/events/ingress.rs`
- add `src/events/subscription.rs`
- add `src/events/query.rs`
- move canonical event contracts into `events`
- convert telemetry event and contract modules into reexport shims
- retarget contract only imports to `crate::events`

Exit criteria:

- canonical event types exist under `events`
- telemetry still compiles through compatibility shims
- no behavior change in append or replay

Verification:

- `cargo test telemetry --lib`
- `cargo test world_state --lib`
- `cargo test branches_query --tests`
- `cargo test test_build_logging_config_default --bin meld`
- `rg "use crate::telemetry::contracts|use crate::telemetry::events" src`

### Phase 2

Goal:

- extract minimal session ownership from telemetry

Tasks:

- add `src/session.rs`
- add `src/session/contracts.rs`
- add `src/session/runtime.rs`
- add `src/session/storage.rs`
- add `src/session/policy.rs`
- add `src/session/events.rs`
- add `src/cli/session.rs`
- move session contracts and storage out of telemetry owned modules
- introduce `SessionRuntime`
- keep telemetry reexports during migration

Exit criteria:

- session lifecycle persists separately from canonical event storage
- canonical append no longer requires session owned types for its basic shape

Verification:

- `cargo test telemetry --lib`
- `cargo test cli --lib`
- `cargo test session_lifecycle_round_trips --lib`
- `cargo test interrupted_sessions_are_marked --lib`
- `rg "SessionStatus|PrunePolicy" src/telemetry`

### Phase 3

Goal:

- extract canonical append, sequencing, and replay into `EventRuntime` and `EventStore`

Tasks:

- move event bus logic into `events`
- move ingestor logic into `events`
- move canonical store logic into `events`
- convert `ProgressRuntime` into a compatibility facade over `EventRuntime` and `SessionRuntime`
- remove session metadata access from the append path

Exit criteria:

- canonical append and replay are owned by `events`
- session lifecycle is no longer fused with canonical append

Verification:

- `cargo test telemetry --lib`
- `cargo test runtime_wide_sequence_is_monotonic --lib`
- `cargo test legacy_events_remain_readable --lib`
- `cargo test slow_or_missing_consumer_does_not_break_append --lib`
- `rg "allocate_next_seq|get_meta\\(&envelope.session\\)|put_meta\\(&event.session" src`

### Phase 4

Goal:

- cut producers over to canonical `events`

Tasks:

- convert event builders to return `EventEnvelope`
- switch correctness relevant emit sites to canonical envelope append
- keep compatibility helpers only where still needed

Exit criteria:

- correctness producers publish through canonical `events`
- task local cache is no longer authoritative

Verification:

- `cargo test task_executor_publishes_canonical_events --lib`
- `cargo test projection_matches_live_execution --lib`
- `cargo test branches_query --tests`
- `cargo test roots_runtime --tests`
- `rg "emit_event_best_effort\\(" src`
- `rg "\"scan_started\"|\"queue_stats\"|\"file_changed\"" src`

### Phase 5

Goal:

- move correctness replay consumers off telemetry owned storage

Tasks:

- retarget execution projection to `EventStore`
- retarget world state reducers to `EventStore`
- retarget graph catch up to `EventStore`

Exit criteria:

- correctness replay consumers import from `events`

Verification:

- `cargo test replay_rebuilds_execution_projection --lib`
- `cargo test world_state_graph --tests`
- `cargo test branches_query --tests`
- `rg "telemetry::sinks::store::ProgressStore|telemetry::events::ProgressEvent" src`

### Phase 6

Goal:

- shrink telemetry to a downstream compatibility surface

Tasks:

- remove canonical storage, append, and type ownership from telemetry
- keep only compatibility facade and downstream summary helpers

Exit criteria:

- telemetry no longer owns correctness behavior

Verification:

- `cargo test telemetry_is_downstream_only --lib`
- `cargo test cli --lib`
- `rg "pub mod events|pub mod contracts|pub mod routing|pub mod sinks" src/telemetry.rs`
- `rg "crate::telemetry::DomainObjectRef|crate::telemetry::EventRelation" src`

### Phase 7

Goal:

- finalize extraction and clean up transitional correctness dependencies

Tasks:

- remove leftover telemetry owned logic that still exceeds compatibility needs
- update this plan with gate evidence
- update module comments and final notes

Exit criteria:

- correctness code depends on `events`
- compatibility shims remain only where intentionally retained

Verification:

- `cargo test`
- `cargo test --bin meld`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `rg "telemetry::sinks::store|telemetry::events::ProgressEnvelope|telemetry::events::ProgressEvent" src`
- `rg "DomainObjectRef|EventRelation" src | rg "telemetry"`

## Gate Definitions

- Gate A: behavior preserving compile and targeted tests pass
- Gate B: storage ownership split is verified by tests
- Gate C: canonical producer cutover is verified by parity tests
- Gate D: replay consumers depend on `events`
- Gate E: final workspace wide test and lint gates pass

## Implementation Order Summary

1. land design baseline
2. introduce `events`
3. introduce `session`
4. move append and replay ownership
5. cut producers over
6. cut replay consumers over
7. shrink telemetry
8. finalize cleanup

## Related Documentation

- [Event Domain Extraction Spec](event_domain_extraction_spec.md)
- [Event Spine Refactor](telemetry_refactor.md)
- [Event Spine Requirements](../../cognitive_architecture/events/event_manager_requirements.md)
- [Multi-Domain Spine](../../cognitive_architecture/events/multi_domain_spine.md)

## Exception List

- Phase 4 parity verification used integration targets instead of `--lib` because
  `task_executor_publishes_canonical_events` and `projection_matches_live_execution`
  live in `tests/integration`
- Phase 5 replay verification used the integration target for
  `replay_rebuilds_execution_projection` for the same reason
- Phase 6 downstream verification used the integration target for
  `telemetry_is_downstream_only`

## Gate Evidence

Completed commits:

1. `84ac3cf` `design(events): define event extraction execution plan`
2. `0d6a85e` `refactor(events): add canonical event contracts and compatibility shims`
3. `6e35861` `refactor(session): split minimal session lifecycle from telemetry`
4. `8f62b12` `refactor(events): extract canonical append and replay runtime`
5. `963b9c9` `refactor(events): cut producers over to canonical event runtime`
6. `d390500` `refactor(events): retarget replay consumers to canonical store`
7. `9146b12` `refactor(telemetry): reduce telemetry to downstream compatibility`

Final verification completed after Phase 6:

- full `cargo test`
- `cargo test --bin meld`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- final telemetry search gates in `src`

- telemetry compatibility names may remain temporarily during migration
- downstream renaming such as `observability` is out of scope for this plan
- no public API contract is defined in this implementation
