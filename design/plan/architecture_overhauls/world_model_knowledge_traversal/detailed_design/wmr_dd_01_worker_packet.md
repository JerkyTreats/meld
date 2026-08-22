# WMR-DD-01 Detailed Design Worker Packet

Date: 2026-08-21

Status: historical worker packet, delivery complete

## Objective

Close the first owner-to-owner design vertical from owner observation through one immutable, occurrence-rich `TraversalCut` without claiming later Curation, Belief, or complete `PlannerCut` behavior.

Direct product proof: docs freshness and dependency security observations can be traced through exact owner publications, Event positions, graph occurrences, hydration, and cut identity. The output supplies later consumers with exact graph input while keeping their acceptance positions deferred.

## Accepted Maturity Envelope

Posture: `exploratory`

Obligation floor: `operational durability for incumbent records and recovery seams`

Hard limits:

- zero source-code changes
- zero new crates, stores, services, runtimes, protocols, migrations, or compatibility systems
- Events and `meld-lang` retain current semantic ownership
- D0 vocabulary remains bounded to this vertical
- Curation, Belief settlement, complete `PlannerCut`, Strategy, Agent progression, Execution, PDS, and lifecycle detail remain later-phase work

Tripwires:

- any source-code edit
- any new top-level domain
- any new semantic authority in Events, Graph, Traversal, Planner, lifecycle, or adapters
- any claim of absence, correctness, belief, readiness, or quiescence from graph reachability or a deferred consumer
- any required deliverable outside the exact write scope

## Frozen Affected Set

The complete affected package set remains `meld`, `meld-events`, `meld-execution`, `meld-lang`, and `meld-world-model`. Active design ownership is limited to workspace, docs, dependency security, Events, Graph, Traversal, the later Planner input boundary, and declared Curation and Belief consumer edges.

## Exact Write Scope

- this packet
- `semantic_transition_ledger.md`
- `owner_publication_to_traversal_cut.md`
- `../world_model_reconciliation_handoff_ledger.md`
- `../delivery_gates/wmr_dg_01_owner_publication_to_frozen_cut.md`
- `../reviews/wmr_dd_01_integrated_design_review_receipt.md`
- `../delivery_gates/wmr_dg_01_acceptance_receipt.md`
- `../world_model_reconciliation_delivery_program_ledger.md`
- `../README.md`

Permitted read scope is the named evidence in the program ledger, canonical Graph, Traversal, Planner, Curation, and docs freshness architecture, plus current source anchors already frozen in the evidence maps.

## Required Deliverables

- bounded semantic transition ledger
- owner-publication-to-`TraversalCut` detailed design
- exact `WMR-H01` through `WMR-H03` obligations
- declared deferred relationships `WMR-H04` through `WMR-H06`
- already-correct docs trace
- missing-or-incorrect observed-scope trace with Curation judgment deferred
- dependency security dissimilarity trace
- complexity delta and validation evidence

## Existing Seams

- `DomainObjectRef`, `EventRelation`, and `EventEnvelope`
- idempotent Event append and bounded replay
- workspace snapshot and node observation Events
- `TraversalFactRecord`, graph cursor, projection indexes, and derived outbox
- canonical `GraphObjectPublication`, `RelationOccurrencePublication`, `TraversalQuery`, `TraversalResult`, and `TraversalCut`

## Forbidden Changes

- Rust types, fields, schemas, API signatures, migrations, estimates, or implementation sequencing
- Curation operation design
- Belief eligibility or settlement design
- complete `PlannerCut` assembly
- shared runtime ledger or universal ontology
- later-slice Gate criteria

## Proof And Review

Frozen Delivery Gate: [WMR-DG-01 revision 3](../delivery_gates/wmr_dg_01_owner_publication_to_frozen_cut.md)

Implementation-review role: one integrated design review over the exact documentation candidate

Gate Acceptance role: one separate criterion-bound pass after design review

Commit expectation: no commit unless separately requested

Stop before any unauthorized expansion. Return changed artifacts, direct design proof, complexity delta, validation, and unresolved risks.
