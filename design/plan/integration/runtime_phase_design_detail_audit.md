# Runtime Phase Design Detail Audit

Date: 2026-06-17
Status: implemented
Scope: design detail readiness audit for durable flywheel runtime phases

## Purpose

This audit records which phases in [Durable Flywheel Runtime Phase Design](durable_flywheel_runtime_phase_design.md) are sufficiently specified for implementation.

The crate runtime requirements are now detailed enough to guide implementation:

- [World Model Runtime Requirements](world_model_runtime_requirements.md)
- [Execution Runtime Requirements](execution_runtime_requirements.md)
- [Event Runtime Requirements](event_runtime_requirements.md)
- [Supervisor Runtime Requirements](supervisor_runtime_requirements.md)

The remaining design risk was the vertical glue around product assembly and proof execution. That risk is now covered by [Product Runtime Assembly Requirements](product_runtime_assembly_requirements.md) and [Durable Flywheel Vertical Proof Requirements](durable_flywheel_vertical_proof_requirements.md).

## Phase Readiness

| Phase | Detail Status | Reason | Required Follow Up |
| --- | --- | --- | --- |
| Phase 0 | sufficient | Document realignment is bounded and already has stale language checks. | No added design required. |
| Phase 1 | sufficient | Product assembly requirements now bind root resolution, store opening order, supervisor store opening, port construction, runtime factory construction, supervisor handoff, error model, and forbidden semantic queries. | Use [Product Runtime Assembly Requirements](product_runtime_assembly_requirements.md). |
| Phase 2 | sufficient | Supervisor diagnostics are already covered by bounded worker report design and supervisor requirements. | Link where useful during implementation. |
| Phase 3 | sufficient | World model runtime requirements cover graph, belief, projection, goal curation, evidence ingestion, and satisfaction curation. | Keep direct phase link. |
| Phase 4 | sufficient | Execution runtime requirements cover goals, planning, task network, dispatch, artifacts, and publication. | Keep direct phase link. |
| Phase 5 | sufficient | Vertical proof requirements now bind event led feedback across event append, bounded replay, evidence ingestion, belief reassessment, projection, and satisfaction. | Use [Durable Flywheel Vertical Proof Requirements](durable_flywheel_vertical_proof_requirements.md). |
| Phase 6 | sufficient | Vertical proof requirements now separate deterministic harness duties from product runtime architecture and forbid retained semantic state. | Use [Durable Flywheel Vertical Proof Requirements](durable_flywheel_vertical_proof_requirements.md). |
| Phase 7 | sufficient | Vertical proof requirements now define failure outcome state, false satisfaction guards, idempotency, duplicate replay, and final active goal assertions. | Use [Durable Flywheel Vertical Proof Requirements](durable_flywheel_vertical_proof_requirements.md). |
| Phase 8 | sufficient | Supervisor domain plan and supervisor requirements define lifecycle, leases, health, shutdown, restart, and status. | Keep direct phase link. |
| Phase 9 | sufficiently small | CLI adapter should stay thin and wait until library proof is green. Detailed design is not needed before implementation. | Keep as a small adapter phase. |

## Implemented Design Details

### Product Runtime Assembly

Phase 1 needed a detailed design because assembly is the place where all stores, adapters, and ports meet. Without tighter requirements, implementation could accidentally add root reads of active goals, pending publications, belief views, or event payload meaning.

The added [Product Runtime Assembly Requirements](product_runtime_assembly_requirements.md) specify:

- product root resolution
- store opening order
- supervisor store opening
- port construction
- runtime factory registry construction
- runtime handle construction
- startup handoff to supervisor
- shutdown and flush handoff to supervisor
- assembly error model
- forbidden semantic queries

### Vertical Proof Path

Phases 5, 6, and 7 needed a single vertical proof design because they prove the flywheel shape rather than one crate runtime.

The added [Durable Flywheel Vertical Proof Requirements](durable_flywheel_vertical_proof_requirements.md) specify:

- docs freshness fixture constants
- success handoff sequence
- event led feedback sequence
- checkpoint labels
- durable state expected at each checkpoint
- proof driver responsibilities
- proof driver forbidden state
- failure outcome path
- false satisfaction guards
- idempotency and duplicate replay checks
- diagnostic use without correctness authority

## Small Phases

Phase 0, Phase 2, and Phase 9 are intentionally small.

Phase 0 is documentation cleanup with grep based verification.

Phase 2 is a diagnostic contract alignment phase and is already backed by [RTG-3 Bounded Worker Report Contract Implementation Plan](rtg_3_bounded_worker_report_contract.md).

Phase 9 should remain a thin adapter until library proof and supervisor start paths are stable. A detailed CLI design before that point would likely duplicate product assembly and supervisor details.

## Implemented Acceptance

The phase design is ready for implementation because:

- Phase 1 links to product runtime assembly requirements
- Phase 5 links to vertical proof requirements
- Phase 6 links to vertical proof requirements
- Phase 7 links to vertical proof requirements
- Phase 8 links to supervisor requirements
- The runtime requirements index remains the shared map for crate runtime and supervisor requirements
