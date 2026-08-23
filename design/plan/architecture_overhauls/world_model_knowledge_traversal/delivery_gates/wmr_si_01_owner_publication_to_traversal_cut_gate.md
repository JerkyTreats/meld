# WMR-SI-01 Owner Publication To TraversalCut Delivery Gate

Date: 2026-08-23

Gate identifier: `WMR-SI-01-DG`

Revision: 1

Status: frozen for the authorized source slice

Active slice: `WMR-SI-01`

Intended handoff: accepted occurrence-rich graph input to future Curation, Belief, and Planner consumers

Gate owner: dedicated acceptance reviewer

Exception authority: user

## Coherence Horizon

The horizon begins with one owner observation and its bounded completeness account. It closes with a reproducible `TraversalResult` over one immutable `TraversalCut` after durable Event carriage and Graph projection.

The horizon includes owner publication durability, neutral Event identity, Graph admission, projection progress, relation occurrence preservation, owner revision selection, bounded Traversal, hydration lineage, lag, replay, and restart. It excludes downstream Curation acceptance, Belief settlement, complete `PlannerCut`, Strategy, Agent progression, Execution, PDS activation, lifecycle aggregation, and Startup.

## Required Deliverables

| Deliverable | Owner | Required product |
| --- | --- | --- |
| observation and completeness | source owner | exact source revision, observations, exclusions, failures, and terminal completeness receipt |
| publication operation | source owner | durable retryable operation or deterministic reconstruction source |
| durable carriage | Events | stable record identity, typed owner payload, and append position |
| graph admission | Graph | owner-route validation and exact object plus relation-occurrence materialization |
| projection progress | Graph runtime | durable cursor and derived outbox closure through the Event position |
| graph cut | Graph and Traversal | exact participating owner revisions, completeness receipts, scope, policy, and projection positions |
| bounded result | Traversal | deterministic objects, occurrences, paths, provenance, frontier, truncation, and hydration references |

## Criteria

| Criterion | Required claim | Blocking condition |
| --- | --- | --- |
| `WMR-SI-01-DG-C01` | every publication is retryable from a durable operation or exact authoritative reconstruction source | process memory or best-effort append as sole source blocks |
| `WMR-SI-01-DG-C02` | owner completeness names exact scope, exclusions, failures, and terminal state | inferred or silent completeness blocks |
| `WMR-SI-01-DG-C03` | Events carries a typed owner payload without interpreting its semantics | Event-owned product meaning blocks |
| `WMR-SI-01-DG-C04` | `DomainObjectRef` remains an opaque address and never proves presence, truth, currentness, or authority | semantic inference from address blocks |
| `WMR-SI-01-DG-C05` | equal relation endpoints with distinct owner occurrences remain distinct through Event, Graph, and Traversal | endpoint deduplication blocks |
| `WMR-SI-01-DG-C06` | Graph commits admitted material, derived outbox effects, and projection progress in the accepted order | cursor advancement before owned effects blocks |
| `WMR-SI-01-DG-C07` | `TraversalCut` binds exact owner revisions, completeness receipts, projection positions, scope, currentness policy, and cut status | global or implicit currentness blocks |
| `WMR-SI-01-DG-C08` | equal normalized queries over equal cuts produce equal result identity | process order or live mutable reads changing identity blocks |
| `WMR-SI-01-DG-C09` | frontier and resource truncation remain explicit and never imply unexplored absence | silent truncation or false completeness blocks |
| `WMR-SI-01-DG-C10` | hydration preserves semantic-owner references and material qualifications | opaque address parsing or flattened qualified edges block |
| `WMR-SI-01-DG-C11` | Event lag, Graph lag, incomplete owner sets, replay, and restart remain independently visible and reproducible | borrowed cursors or lost uncertainty block |
| `WMR-SI-01-DG-C12` | docs and dependency-security specimens prove owner neutrality without importing their workflow semantics | docs-specific ontology or security meaning in substrate blocks |
| `WMR-SI-01-DG-C13` | future consumers receive exact graph input without their acceptance being claimed | Curation, Belief, or Planner completion inference blocks |
| `WMR-SI-01-DG-C14` | the exact candidate passes logical review and `WMR-SAO-01` Style Assurance | missing review or assurance blocks |
| `WMR-SI-01-DG-C15` | source changes remain inside the activated boundary | unauthorized architecture or downstream behavior blocks |

## Direct Product Proof

The proof must run typed publications from at least two semantically dissimilar owners through the same Event, Graph, and Traversal contracts. It must include equal-endpoint distinct occurrences, an incomplete required owner set, explicit truncation, Event-ahead-of-Graph lag, idempotent replay, and process restart.

The docs specimen must show observed material without manufacturing expected README or correctness meaning. The dependency-security specimen must retain inventory or assessment qualification without becoming docs-shaped.

## Forbidden Substitutions

Append success cannot substitute for Graph visibility. Graph visibility cannot substitute for cut inclusion. Cut completion cannot substitute for downstream acceptance. Addressability cannot substitute for presence. Empty results cannot substitute for bounded absence. One owner's cursor cannot substitute for another owner's progress.

## Acceptance Inputs

Gate Acceptance requires the exact source candidate, direct proof, logical implementation-review receipt, satisfied Style Assurance Receipt, criterion evidence, changed-surface account, and authorized exceptions.

The final verdict is `accepted`, `rejected`, or `not eligible`. Acceptance makes `WMR-SI-01` handoff eligible only.
