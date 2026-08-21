# WMR-DD-02 Worker Packet

Date: 2026-08-21

Slice: `WMR-DD-02`, Epistemic Authorship And Settlement Loop

Status: active

Implementor: Codex detailed-design implementor

## Product Increment

Define one owner-to-owner vertical from an exact `TraversalCut` and valid initiating authority through a terminal Curation result, neutral Event carriage, independent Graph visibility, and configured Belief settlement.

The direct design proof must show that standing and Strategy-planned Curation use one semantic owner and one result grammar without confusing their initiating authority. It must also keep Event append, Graph visibility, Belief revision, and later Agent acceptance as separate milestones.

## Maturity Envelope

Posture: exploratory

Obligation floor: operational durability for incumbent Events, Graph, Belief, and Agent records

Hard limits:

- design artifacts only
- zero source-code changes
- zero new crates, dependencies, durable stores, services, background runtimes, protocols, migrations, or compatibility systems
- Events remains neutral carriage
- Traversal remains graph materialization and bounded read authority
- Belief remains configured evidence admission and settlement authority
- Agent Plan progression and complete `PlannerCut` assembly remain deferred to `WMR-DD-03`
- one Curation semantic authority across both invocation contexts

Tripwires:

- a result can be called visible without a named consumer position
- a missing edge is used as proof of nonexistence
- replay of a historical Event grants fresh operation authority
- a generic graph relation is treated as perspective-free truth
- Belief interprets every Curation Event without an installed mapping or dependency contract
- the design needs a new persistence placement to be coherent
- downstream Agent or Planner behavior must be designed to satisfy this slice

## Direct Product Proof

Prove three traces.

### Already-correct docs

An installed Agent rule and exact complete workspace and docs cut allow standing Curation to establish or reuse the expected README and coverage relationships, emit an `unchanged` or `applied` terminal result, and become independently visible without creating a Goal or Task.

### Missing or incorrect docs

Standing Curation authors the expected README and a bounded realization assessment against exact complete observation scope. Non-realization is a positive Curation-owned assessment, not an absent edge. The result becomes visible to Graph and, when configured, Belief. Strategy and Task construction remain deferred.

### Dependency security dissimilarity

Curation may connect inventory, advisory, assessment, and verification products only inside Curation-owned vocabulary and Agent perspective. It cannot claim inventory presence, advisory authority, or verification truth on behalf of those owners.

## Exact Write Scope

- this worker packet
- `epistemic_operation_transition_ledger.md`
- `epistemic_authorship_and_settlement.md`
- `../world_model_reconciliation_handoff_ledger.md`
- `../delivery_gates/wmr_dg_02_epistemic_authorship_and_settlement.md`
- `../reviews/wmr_dd_02_integrated_design_review_receipt.md`
- `../delivery_gates/wmr_dg_02_acceptance_receipt.md`
- `../world_model_reconciliation_delivery_program_ledger.md`
- `../README.md`

## Existing Seams

- accepted `TraversalCut` and `TraversalResult` design from `WMR-DD-01`
- idempotent Event append, ledger identity, sequence, and bounded replay
- graph projection cursor and occurrence-preserving materialization design
- installed Belief evidence mappings, dependency selection, immutable revisions, and Agent subscription positions
- canonical Curation ownership and bounded operation contract
- canonical Agent authorization and Plan progression boundaries

## Required Deliverables

- exact operation, acceptance, terminal result, publication, Graph, Belief, Agent, and Planner positions
- shared Curation semantics across standing and planned invocation
- terminal outcome and unchanged-result meaning
- perspective, scope, currentness, supersession, idempotency, and feedback controls
- wait, wake, fence, restart, and bounded quiescence claims
- exact deferred fields for `WMR-DD-03`
- direct product traces and dissimilarity proof

## Forbidden Changes

- source or runtime implementation requirements
- Rust types, fields, APIs, schemas, and persistence placement
- Strategy Plan shape or Agent progression design
- universal Event-to-Belief interpretation
- direct mutation of Graph or Belief storage by Curation
- a shared ontology or generic epistemic edge API
- activation-wide readiness, liveness, or retirement claims

## Review And Acceptance

Integrated review owner: Codex integrated architecture review lane

Review budget: one initial pass and one verification pass only after accepted corrections

Gate: `WMR-DG-02` revision 1 frozen

Gate owner: Codex separate cross-deliverable acceptance lane

Acceptance budget: one initial pass, one program-owner disposition, one bounded remediation cycle, and one verification pass

Commit expectation: an accepted Gate Receipt closes through a delivery commit before another slice can activate.

## Stop Conditions

Stop and return to the program owner if any tripwire is crossed, if planned Curation cannot remain producer-deferred, if a terminal result cannot be replay-safe without selecting new storage architecture, or if a gate criterion requires `WMR-DD-03` behavior.
