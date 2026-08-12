# Sensory Readiness Assessment

Status: conditionally ready
Depends on: `design/plan/events/assessment.md`, `design/plan/world_model/graph/assessment.md`
Design source: `design/cognitive_architecture/sensory/README.md`, `design/cognitive_architecture/sensory/substrate.md`
Evidence date: 2026-07-10

## Verdict Summary

Sensory is conditionally ready for one promoted observation contract.

The typed loop can be implemented without sensory runtime. Runtime flywheel requires sensory or execution outcome facts.

## Conceptual Correctness

Sensory solves raw environmental observation and promotion into compact semantic facts.

The boundary is correct because raw high-rate signal remains outside canonical events until promoted.

## Completeness

The first runtime slice needs one workspace delta observation or one git change observation, local lowering, duplicate suppression, one promotion contract, and event publication.

Sensory does not depend on `meld-lang` for raw observation.

Sensory output must produce event facts that graph and belief can consume.

## Boundary Clarity

Sensory owns modality workers, raw-to-typed lowering, promotion gates, duplicate suppression, source-local backpressure, provenance, and observation fact publication.

Sensory does not own belief updates, conflict resolution, event durability, replay, task-triggered observation mechanics, planner-facing truth, provider execution, or human-facing context views.

## Dependency Readiness

The event mechanics and graph contracts required for sensory design are ready.
Production publication through one authority remains part of the active event foundation closeout.

Typed-loop readiness does not require sensory runtime.

Runtime flywheel readiness requires a promoted observation record or an execution outcome record.

## First-Slice Feasibility

The sensory first slice is one promoted observation fact for one or two concrete modalities.

The typed loop can use a constructed `WorldState` and does not need the sensory worker lifecycle.

## Current Implementation Evidence

- `src/workspace.rs`
- `src/workspace/`
- `src/ignore.rs`
- `tests/integration/workspace_traversal.rs`
- `tests/integration/workspace_commands.rs`
- `tests/integration/workspace_isolation.rs`
- `tests/integration/context_traversal.rs`
- `design/cognitive_architecture/sensory/README.md`
- `design/cognitive_architecture/sensory/substrate.md`
- `design/cognitive_architecture/execution/planning/observation_wait_semantics.md`

## Gaps

- First promoted observation record shape is not specified.
- Worker lifecycle and local buffer policy are not specified.
- Promotion thresholds and hysteresis are not chosen.
- Git modality details are not specified.

## Open Questions

- Whether the first runtime modality is workspace delta, git change, or both.
- Which object refs and relations the first promoted observation must carry.

## Recommendation

Proceed with typed loop independent of sensory runtime. Specify sensory promotion before runtime flywheel.

## Evidence note 2026-08-12

Moved from `design/cognitive_architecture/sensory/README.md`. The system is stronger at task-triggered observation than continuous background sensing. The required first slice:

- always-on workers for workspace, git, and other high-value modalities
- typed observation contracts that can survive replay and cross-domain reuse
- source-local throttling so high-volume sensors do not dominate events
- a clean handoff from sensory publication to curation reducers
