# Spine Compaction Design

Date: 2026-07-08
Status: proposed
Scope: epoch compaction for the event spine, designed now, built when the activation trigger fires

## Intent

Define how the spine prunes history without breaking replay determinism, so the compactor lands as a non-breaking change against contracts that already exist: the retained lower boundary, the typed retention gap error, and genesis facts.

## Activation Trigger

Compaction is not built until one of these holds on a real deployment:

- the spine tree exceeds 1,000,000 records, or
- rebuild-from-zero replay of any projection exceeds 30 seconds on the deployment machine, or
- spine disk footprint exceeds 2 GB.

Until then this document is the contract and the implementation is deliberately absent. Building an epoch compactor for a spine holding thousands of events is the pre-extension trap the overhaul PLAN names.

## Already-Landed Contract

The retention phase shipped the pieces consumers are written against:

- `EventStore::retained_lower_boundary` returns the first retained sequence, one meaning full history.
- `EventStore::set_retained_lower_boundary` raises the boundary monotonically; it never lowers.
- Every replay path returns `StorageError::RetentionGap` when a cursor predates the boundary, so no consumer can silently skip pruned history.
- The graph reducer surfaces a gap as a fatal `retention_gap` diagnostic without moving its cursor.
- `EventEnvelope::genesis_domain` records rebuilt-from-snapshot at a basis sequence with an idempotent record id.

## Compaction Procedure

When the trigger fires, the compactor runs as a supervised maintenance task:

1. Choose a target boundary `B`, at most the minimum durable cursor across all registered consumers; the runtime wiring workstream owns consumer registration.
2. Quiesce readers through the supervisor before raising the boundary, so no reader that passed the gap check can scan across a concurrent prune and return a silently gapped batch. Readers additionally re-check the boundary after scanning as defense.
3. For each projection whose durable state summarizes history below `B`, record a genesis fact through `genesis_domain` with `basis_seq` at least `B - 1`.
4. Raise the retained lower boundary to `B` and flush before deleting anything, so a crash mid-prune fails closed: readers see a gap error, never partial history.
5. Delete spine records, session index entries, and record index entries below `B`, in that order, in bounded batches.
6. Emit one compaction event recording the old and new boundaries for audit.

Replaying genesis plus retained deltas reconstructs every projection; rebuild-from-zero stops being a supported path once the boundary rises, which is exactly what the gap error communicates.

## Session Consumers

Session reads share the global boundary, so once it rises, a session cursor of zero errors even for sessions created after compaction. Session-scoped readers such as the live progress panel must handle the gap by re-anchoring their cursor at `B - 1` rather than discarding the error class; the panel's error handling is updated when the compactor lands, and until then the default boundary of one makes every session cursor valid.

## Invariants

- The boundary raise is flushed before any deletion, always.
- No consumer cursor below `B` may exist when the boundary rises; the compactor aborts rather than strand a consumer.
- Idempotency entries for pruned records are deleted with them. Genesis facts dedupe by identity, so snapshots at one basis sequence must be deterministic, and the `.genesis` event type suffix is reserved in every domain vocabulary.
- The compactor never touches the legacy tree, which is already migration-frozen.

## Read With

- [Event Spine Overhaul History](../../completed/events/event_spine_overhaul_program.md)
- [Event Runtime Requirements History](../../completed/integration/event_runtime_requirements.md)
