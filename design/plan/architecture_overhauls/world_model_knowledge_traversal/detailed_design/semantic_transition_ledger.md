# WMR-DD-01 Semantic Transition Ledger

Date: 2026-08-21

Status: active-slice design product

Scope: owner observation through immutable `TraversalCut`

## Purpose

This ledger fixes only the identities and durable positions needed by the first knowledge vertical. It is a semantic account across owner contracts, not a shared runtime record, universal schema, or demand that every owner persist the same shape.

## Identity Families

| Identity | Owner | Created when | Must remain distinct from | Durable evidence |
| --- | --- | --- | --- | --- |
| source identity | observation owner | a source is admitted under one owner namespace | activation generation and snapshot identity | owner source record or exact source reference |
| source revision | observation owner | the owner establishes one exact source state | Event sequence and graph cut | owner revision or snapshot receipt |
| observation identity | observation owner | one semantic unit is observed within a source revision | object address and Event record | owner observation product |
| observation-set identity | observation owner | the owner closes a bounded completeness scope | individual observation and traversal result | completeness receipt under source revision |
| publication-operation identity | observation owner | publication becomes retryable across Event append | Event record identity | durable owner operation or publication outbox |
| object address | semantic product owner | an object becomes cross-domain addressable | presence, truth, currentness, and publication revision | opaque `DomainObjectRef` |
| object-publication identity | semantic product owner | one address receives an owner-qualified presence and scope statement | object address and Event record | owner revision plus scope and publication identity |
| relation-occurrence identity | semantic product owner | one exact qualified relation is asserted | endpoint equality and relation type | owner occurrence identity plus revision and scope |
| Event record identity | producing owner through Events | one envelope is durably accepted | owner observation and Event sequence | stable record identity and append receipt |
| Event position | Events | the ledger assigns an exact sequence | graph cursor and cut identity | ledger identity plus sequence |
| graph fact identity | Graph projection | one admitted Event becomes graph-readable material | Event record identity and owner publication identity | projection fact tied to source Event position |
| graph projection position | Graph projection | all projection writes through one Event sequence are durable | Event append position and query result | durable graph cursor plus outbox closure |
| owner graph revision receipt | publishing owner through graph contract | one owner projection revision is available to traversal | global currentness and another owner revision | owner, scope, revision, and projection position |
| traversal query identity | Traversal | roots, relation policy, scope, cut request, and bounds are normalized | result identity and cut identity | normalized query representation |
| `TraversalCut` identity | Graph and Traversal | exact participating owner revision receipts are selected | query identity and complete `PlannerCut` | exact owner revision set plus scope and policy |
| traversal result identity | Traversal | one normalized query runs against one exact cut | cut identity and consumer acceptance | normalized query plus exact cut identity |
| hydration reference identity | source owner or declared read port | a cut preserves a route to owner-qualified product material | opaque object address and copied foreign payload | source product reference plus owner and explanation role |

## Position Ladder

These positions are ordered by causality but remain independently owned:

```text
owner observation durable
-> owner publication operation durable
-> Event append durable at sequence N
-> Graph projection durable through sequence N
-> owner graph revision receipt included in cut C
-> TraversalResult R completed against cut C
-> later consumer acceptance position
```

No earlier position proves a later one. In particular:

- call completion does not prove owner publication durability
- owner publication durability does not prove Event append
- Event append does not prove Graph visibility
- Graph visibility does not prove inclusion in a requested cut
- `TraversalCut` completion does not prove Belief revision, Curation acceptance, complete `PlannerCut`, or Agent visibility

## Authority Ledger

| Question | Owning authority | Non-authorities |
| --- | --- | --- |
| what source state was observed | workspace, docs, dependency security, or another source owner | Events, Graph, Traversal, Planner |
| what an object or relation means | semantic publishing owner | Events and Graph storage |
| whether an envelope is durably carried | Events | producer call stack and consumers |
| whether owner material is graph-readable through a sequence | Graph projection | Event append receipt and query caller |
| which owners and revisions form one graph cut | Graph and Traversal contract | Planner, Curation, Belief |
| whether a traversal is complete within declared bounds | Traversal | later reasoning consumers |
| whether a README is expected | Curation in `WMR-DD-02` | workspace, Graph, Traversal |
| whether observed docs satisfy an expectation | docs and configured Belief under later contracts | Graph and Planner |
| which graph input joins a complete reasoning cut | Planner in `WMR-DD-03` | Traversal alone |

## Idempotency And Supersession

Owner retry uses publication-operation identity. Events deduplicates carriage through Event record identity. Graph replay deduplicates projection by ledger identity, sequence, and fact identity. Traversal result identity depends on normalized query and exact cut.

Supersession remains owner-shaped:

- a new source revision does not erase the old source revision
- a new object publication does not change object address identity
- equal relation endpoints do not collapse distinct occurrences
- withdrawal and replacement are durable owner lifecycle facts
- a new cut succeeds rather than mutates its predecessor
- missing owner receipts make a cut incomplete rather than silently current

## Scope And Currentness

Addressability does not prove presence. Presence states such as observed, expected, absent, withdrawn, and archived are owner products.

Currentness always names owner policy, owner revision, temporal scope, branch scope, and perspective where applicable. The current `current_only` query flag is implementation evidence of a lossy seam, not the future semantic contract.

## Wait, Wake, Fence, And Restart Terms

| Edge | Wait | Wake | Fence | Restart source |
| --- | --- | --- | --- | --- |
| `WMR-H01` | source unavailable, observation incomplete, or publication pending | source delivery, durable deadline, or operator action | source lineage and activation generation | source cursor and publication operation |
| `WMR-H02` | ledger quiet beyond graph cursor or invalid publication blocked | Event watermark plus durable fallback | ledger identity, retention boundary, owner route, source revision | graph cursor, indexes, and derived outbox |
| `WMR-H03` | required owner receipt absent or bounded frontier incomplete | owner revision or projection-position advancement | query scope, owner set, perspective, time, branch, currentness policy | owner projections, receipts, normalized query, exact cut |

These terms do not establish activation-wide readiness or quiescence. That aggregation belongs to `WMR-DD-06`.

## Deferred Consumer References

`WMR-H04` records the reproducible graph input later consumed by Planner. `WMR-H05` records the source cut later accepted by standing Curation. `WMR-H06` records the exact graph products later evaluated by configured Belief.

Their consumer acceptance, wait, wake, and completion semantics remain deferred. Their presence here prevents relationship loss without making them active-slice deliverables.

## Fixed Semantic Tests

- equal endpoints with different occurrence identities remain different relations
- an address with no owner presence product is not absent
- Event sequence N may be durable while Graph remains behind N
- a graph projection may be current for one owner while the requested multi-owner cut remains incomplete
- equal queries against equal cuts produce equal result identity
- a later consumer may reject or defer a valid cut without invalidating the producer position

No entry in this ledger authorizes a storage shape or public API.
