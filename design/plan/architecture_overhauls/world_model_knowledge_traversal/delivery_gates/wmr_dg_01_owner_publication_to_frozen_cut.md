# WMR-DG-01 Owner Publication To Frozen Traversal Cut

Gate identifier: `WMR-DG-01`

Revision: 3 frozen

Status: active and frozen on 2026-08-21

Gate type: cross-deliverable detailed-design coherence

Implementation authorization: none

## Intended Handoff

This gate decides whether the first detailed-design slice has established a coherent path from owner observation through an immutable `TraversalCut` and has made the next Curation, Belief, and later Planner verticals meaningful.

An accepted gate establishes design handoff eligibility. It does not prove runtime implementation and does not authorize `WMR-DD-02`.

## Coherence Horizon

Inside the horizon:

- bounded D0 identity and transition vocabulary used by the first vertical
- workspace and docs owner publication
- dependency security as a dissimilar publication check
- neutral Event append and replay
- Graph admission and occurrence-preserving materialization
- bounded Traversal query and exact `TraversalCut`
- exact graph input and downstream contract for later `PlannerCut` assembly
- handoff edges `WMR-H01` through `WMR-H06`
- active lifecycle facts meaningful for `WMR-H01` through `WMR-H03`

Outside the horizon:

- Curation operation internals and storage
- Strategy Plan construction
- Agent Plan progression
- Execution intake and Task realization
- PDS package compilation
- activation-wide lifecycle coordination
- Rust types, schemas, APIs, migrations, and implementation commits

`WMR-H04` through `WMR-H06` are included only as declared later-consumer relationships. Their consumer contracts remain deferred.

## Deliverables Under Acceptance

- semantic transition ledger bounded to identities and positions used by the first vertical
- owner-publication detailed design
- reconciled Graph and Traversal detailed design
- explicit boundary between completed `TraversalCut`, later multi-source `PlannerCut` assembly, and older `WorldModelView` vocabulary
- [handoff and lifecycle ledger](../world_model_reconciliation_handoff_ledger.md) entries for `WMR-H01` through `WMR-H06`
- docs freshness already-correct and missing-or-incorrect traces
- dependency security dissimilarity trace
- integrated design-review receipt for the exact design candidate

Named upstream evidence:

- [program ledger](../world_model_reconciliation_delivery_program_ledger.md)
- [detailed design workstream framing](../detailed_design_workstream_framing.md)
- [current code ground map](../current_code_groundmap.md)
- [impact assessment](../impact_assessment/impact_assessment.md)
- [producer-consumer connectivity review](../runtime_initialization_lifecycle/reviews/producer_consumer_connectivity_review.md)
- [canonical Graph and Traversal specification](../../../../cognitive_architecture/world_model/graph/spec.md)
- [canonical Planner architecture](../../../../cognitive_architecture/world_model/planner/README.md)

## Criteria

| Criterion | Cross-deliverable claim | Required evidence | Blocking failure |
| --- | --- | --- | --- |
| `WMR-DG-01-C01` | object address, owner publication, Event record, graph fact, relation occurrence, and `TraversalCut` identities remain distinct and linked | transition ledger plus both product traces | any identity collapse makes replay, supersession, or consumer visibility ambiguous |
| `WMR-DG-01-C02` | product owners define semantic meaning, Events carries neutrally, Graph materializes structurally, and Traversal reads without claiming belief or complete reasoning context | owner matrix and contract-boundary trace | Events, adapters, lifecycle, Graph, or Traversal gains foreign semantic authority |
| `WMR-DG-01-C03` | every active edge names a durable producer position and an independent consumer acceptance position | complete `WMR-H01` through `WMR-H03` entries | call completion, Event append, or another consumer cursor substitutes for the required position |
| `WMR-DG-01-C04` | equal relation endpoints do not erase distinct relation occurrences or their provenance | Graph and Traversal result examples across workspace and dependency security | relation occurrence identity or qualification is lost before the cut |
| `WMR-DG-01-C05` | presence, currentness, temporal scope, branch, perspective, owner revision, and cut completeness are explicit policies or facts | traversal query and cut rules plus incomplete-cut scenario | generic `current_only`, missing data, or graph absence silently becomes semantic absence or universal currentness |
| `WMR-DG-01-C06` | the cut retains enough owner-correct hydration to distinguish a reached README, source claim, manifest, lockfile, advisory, assessment, or verification record without parsing opaque identity | hydration ownership table and both product traces | consumers must parse object keys, interpret arbitrary payloads, or query foreign storage internals without a declared port |
| `WMR-DG-01-C07` | equal graph owner revisions, query policy, scope, and projection version produce equal `TraversalCut` identity, while changed inputs create a successor cut without rewriting history | deterministic identity and replay account | restart cannot reproduce or validate the cut from exact source positions |
| `WMR-DG-01-C08` | active lifecycle statements name waits, wakes, fences, and restart sources without claiming activation-wide readiness or quiescence | lifecycle projection for `WMR-H01` through `WMR-H03` | clean ticks, empty queues, Event append, or deferred consumers are treated as readiness or quiescence |
| `WMR-DG-01-C09` | the already-correct docs trace reaches an immutable `TraversalCut` that exposes observed README and source claims without requiring a draft, publication receipt, or Task | exact trace with owner products, occurrences, and positions | executable work is required merely to establish existing correctness inputs |
| `WMR-DG-01-C10` | the missing-or-incorrect docs trace exposes complete observed scope, provenance, and hydration sufficient for later Curation to assess non-realization or mismatch without Traversal making that judgment | exact observed-scope trace and deferred-authority account | missing graph reachability becomes truth, or D1 claims Curation-owned expectation, non-realization, or correctness |
| `WMR-DG-01-C11` | dependency security uses the same publication and cut protocol while retaining its distinct inventory, advisory, assessment, and verification ownership | dissimilarity trace | docs-specific grammar leaks into the shared boundary or security semantics move into Curation, Events, or Traversal |
| `WMR-DG-01-C12` | deferred Planner, Curation, and Belief consumer obligations are explicit and cannot create false completion | `WMR-H04` through `WMR-H06` states plus downstream precondition list | the gate requires later-phase work or claims those consumers are ready |
| `WMR-DG-01-C13` | `TraversalCut`, complete `PlannerCut`, and current `WorldModelView` vocabulary have explicit owners and phase boundaries without prematurely assembling the reasoning cut | terminology decision with supersession and downstream assembly disposition | two competing root reasoning-cut products remain or `WMR-DD-01` claims source revisions owned by later phases |
| `WMR-DG-01-C14` | the design stays within the accepted maturity envelope and sacred seams | expansion decision record and complexity delta | new crate, store, service, protocol, Events grammar, language grammar, or implementation work enters without approval |

## Acceptable Evidence

Acceptable evidence is limited to:

- exact owner-to-owner trace tables
- identity and lineage examples derived from current product scenarios
- contract invariants tied to current code or canonical architecture
- explicit before-and-after terminology reconciliation
- edge-level producer and consumer positions
- bounded restart, lag, incomplete-cut, and deferred-consumer scenarios
- integrated design-review receipt over the exact candidate

## Forbidden Substitutions

The following do not satisfy this gate:

- a list of future Rust types or fields
- unit tests without a cross-deliverable trace
- Event append as proof of graph, Belief, Planner, or Agent visibility
- graph reachability as belief or correctness
- generic currentness without owner policy and scope
- a clean actor pass as quiescence
- a future consumer placeholder reported as ready
- implementation feasibility as proof of semantic ownership
- roadmap detail as proof that the current vertical closes
- reviewer confidence without cited evidence

## Blocking Standard

A violation blocks only when it demonstrates:

- missing direct design-product proof
- incorrect active-path ownership or behavior
- concrete current data or security risk
- applicable policy violation
- maturity-envelope violation
- a failed declared cross-deliverable coherence claim

Future scaling, speculative durability, generalized hardening, implementation ergonomics, and undeclared later consumers do not block this gate.

## Acceptance Ownership And Budget

Gate owner: one separate cross-deliverable acceptance owner

Program owner: delivery-program owner

Exception authority: user

Budget:

- one initial acceptance pass
- criterion-bound verdicts only
- frozen violation set after the initial pass
- one program-owner disposition
- one bounded remediation cycle
- one gate-verification pass limited to failed criteria and correction-caused regressions

The gate owner may not prescribe redesign, implement corrections, waive a criterion, amend this definition, expand the slice, or authorize the next phase.

## Violation Disposition

Every violation must cite one criterion identifier, concrete evidence, affected deliverables or edge identifiers, and the reason the handoff is blocked.

The program owner assigns exactly one disposition:

- `evidence correction`
- `active-slice defect`
- `gate-definition defect`
- `unauthorized expansion`
- `later-phase dependency`
- `authorized exception`

A later-phase dependency never expands `WMR-DD-01`. It either returns this definition to design authority or leaves the gate rejected pending user direction.

## Acceptance Record

The Gate Acceptance Record must contain:

- gate identifier and frozen revision
- exact candidate commit or tree identity
- integrated design-review receipt
- one verdict and evidence set for every criterion
- frozen violations
- program-owner dispositions
- verification result when used
- authorized exceptions with scope and retirement condition
- downstream preconditions established
- overall verdict of `accepted`, `rejected`, or `not eligible`

An accepted record becomes the Gate Acceptance Receipt and changes `WMR-DD-01` to `handoff eligible`.

## Downstream Preconditions Established By Acceptance

Acceptance establishes only that:

- Curation and Belief can name an exact bounded graph source cut without inventing Graph or Planner ownership
- Planner can later combine that `TraversalCut` with exact Belief, Causation, Regime, directive, Capability catalog, scope, authority, and projection revisions
- owner publication and downstream visibility positions are distinct
- deferred Curation and Belief obligations are named in the handoff ledger
- lifecycle design can later bind exact products and positions without interpreting their semantics

Acceptance does not establish that Curation, Belief settlement, complete `PlannerCut` assembly, heterogeneous Plans, Agent progression, Execution changes, PDS compilation, or lifecycle closure are designed or implemented.
