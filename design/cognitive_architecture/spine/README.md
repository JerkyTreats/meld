# Spine Concern

Date: 2026-04-12
Status: active
Scope: shared temporal substrate for sensory, world state, execution, and attached object domains

## Thesis

The spine is the shared temporal ledger for the cognitive loop.
It is the durable boundary between domain work and cross-domain truth.

No domain should require direct correctness calls into another domain.
Instead, promoted semantic facts should be published into the spine, then reduced into domain projections.

The spine must provide:

- durable append for promoted semantic facts
- one runtime-wide order for replay and temporal queries
- subscription and catch-up for reducers, planners, and diagnostics
- cross-domain anchors for objects, beliefs, outcomes, and provenance

## Boundary

`spine` owns:

- the canonical event envelope
- runtime-wide sequence assignment
- append, replay, and subscription behavior
- compatibility rules for stored records during migration
- cross-domain reference contracts such as `DomainObjectRef`

`sensory` owns observation lowering and promotion.
`world_state` owns belief update and materialized world state.
`execution` owns planning, control, and action outcomes.
`telemetry` consumes the spine downstream for observability and export.

## Immediate Position

The first implementation slice should land in `execution`.
That does not make the spine part of execution.
It means the current telemetry and task code provide the cheapest path to the first real spine.

The immediate design constraints are:

- sequence must become runtime wide rather than session local
- the canonical envelope must add `domain_id`, `stream_id`, and optional `content_hash`
- raw sensory lanes stay outside the spine until semantic promotion
- `execution` becomes the first domain to publish canonical facts into the spine
- `world_state` and `sensory` constrain the envelope now, even if they do not land in the first refactor

## Open Choice

The first landing should use one local sequencer and one local append path.
Do not block the near-term refactor on distributed log design.

The unresolved longer-term question is how multi-process sequencing should evolve after the single-runtime spine is proven.

## Read Order

1. [Events Design](../events/README.md)
2. [Event Spine Requirements](../events/event_manager_requirements.md)
3. [Event Spine Refactor](../events/telemetry_refactor.md)
4. [Multi-Domain Spine](../events/multi_domain_spine.md)

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Sensory Domain](../sensory/README.md)
- [World State Domain](../world_state/README.md)
- [Execution Domain](../execution/README.md)
