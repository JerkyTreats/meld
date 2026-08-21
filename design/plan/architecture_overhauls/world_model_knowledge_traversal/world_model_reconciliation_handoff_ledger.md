# World Model Reconciliation Handoff And Lifecycle Ledger

Date: 2026-08-21

Status: program design artifact, active edge details proposed

Implementation authorization: none

## Purpose

This ledger records the producer-consumer and lifecycle relationships that must remain coherent while their owning designs are delivered in different phases.

The program ledger tracks phases and authority. This artifact tracks edges. It does not create a runtime coordinator, shared domain grammar, or implementation requirement.

An earlier phase may define and later implement a producer while its consumer remains deferred. That relationship is safe only when the deferred state is explicit and no readiness, visibility, quiescence, or completion claim borrows a future consumer position.

## Relationship States

| State | Meaning |
| --- | --- |
| `current implemented` | both current products and positions exist on the inspected path |
| `current partial` | durable local behavior exists but one or more edge obligations are missing |
| `producer defined, consumer deferred` | the producer contract may close while the named consumer remains a later-phase obligation |
| `design pending` | neither side has a sufficiently exact future contract |
| `design accepted` | the owning detailed design and Gate Acceptance establish the relationship |
| `implementation unproved` | design is accepted but runtime behavior has not been implemented and directly proved |
| `proof projection only` | owner-specific handoffs close earlier and this entry only composes their lineage for inspection |

## Program Edge Registry

| Edge | Producer to consumer | Producer definition phase | Consumer closure phase | Current evidence state |
| --- | --- | --- | --- | --- |
| `WMR-H01` | product observation to Events | `WMR-DD-01` | `WMR-DD-01` | current partial |
| `WMR-H02` | Events to Graph materialization | `WMR-DD-01` | `WMR-DD-01` | current implemented for admitted domains |
| `WMR-H03` | owner graph projections to bounded `TraversalCut` | `WMR-DD-01` | `WMR-DD-01` | current partial |
| `WMR-H04` | `TraversalCut` to immutable `PlannerCut` | `WMR-DD-01` | `WMR-DD-03` | producer defined, consumer deferred |
| `WMR-H05` | `TraversalCut` to standing Curation | `WMR-DD-01` | `WMR-DD-02` | producer defined, consumer deferred |
| `WMR-H06` | Traversal products to configured Belief | `WMR-DD-01` | `WMR-DD-02` | current partial |
| `WMR-H07` | Agent Plan authorization to planned Curation | `WMR-DD-03` | `WMR-DD-02` and `WMR-DD-03` | design pending |
| `WMR-H08` | Curation terminal result to Events | `WMR-DD-02` | `WMR-DD-02` | design pending |
| `WMR-H09` | Events to configured Belief | `WMR-DD-02` | `WMR-DD-02` | current implemented for installed mappings |
| `WMR-H10` | Belief revision to Agent | `WMR-DD-02` | `WMR-DD-03` | current implemented for current Goal path |
| `WMR-H11` | Agent Goal and cut to Strategy | `WMR-DD-03` | `WMR-DD-03` | current partial for executable-only candidate |
| `WMR-H12` | Strategy Plan revision to Agent progression | `WMR-DD-03` | `WMR-DD-03` | design pending |
| `WMR-H13` | Agent authorization to Execution admission | `WMR-DD-03` | `WMR-DD-04` | current implemented for one executable authorization |
| `WMR-H14` | Execution admission to Task Network | `WMR-DD-04` | `WMR-DD-04` | current implemented with semantic seam pressure |
| `WMR-H15` | planning route to package dispatch | `WMR-DD-04` | `WMR-DD-04` | current partial because handoff is process-local |
| `WMR-H16` | Task Network ready work to dispatch | `WMR-DD-04` | `WMR-DD-04` | current implemented |
| `WMR-H17` | Execution outcome to Events | `WMR-DD-04` | `WMR-DD-04` | current implemented for Task publication |
| `WMR-H18` | integrated result lineage inspection | `WMR-DD-02` through `WMR-DD-04` | `WMR-DD-07` | proof projection only, owner-specific handoffs close earlier |
| `WMR-H19` | admitted result to Agent reconciliation | `WMR-DD-03` | `WMR-DD-03` | current partial for Goal creation and satisfaction |
| `WMR-H20` | PDS package to owner installation receipts | `WMR-DD-05` | `WMR-DD-05` | current implemented for executable-only theory image |
| `WMR-H21` | installed product to Agent genesis plan | `WMR-DD-05` | `WMR-DD-05` | current partial |
| `WMR-H22` | prepared product to activation generation | `WMR-DD-05` | `WMR-DD-06` | standalone lifecycle substrate only |
| `WMR-H23` | activation generation to participant readiness | `WMR-DD-06` | `WMR-DD-06` | current partial and disconnected |
| `WMR-H24` | owner progress to wait and wake closure | owner phases | `WMR-DD-06` | current partial and disconnected |
| `WMR-H25` | owner safe points to fenced retirement | owner phases | `WMR-DD-06` | design pending |
| `WMR-H26` | Belief revision to `PlannerCut` assembly | `WMR-DD-02` | `WMR-DD-03` | current partial |
| `WMR-H27` | Causation and Regime revisions to `PlannerCut` assembly | existing canonical owners | `WMR-DD-03` | canonical products exist, integrated cut relationship pending |
| `WMR-H28` | directive context and Capability catalog revision to `PlannerCut` assembly | existing Agent and Capability owners | `WMR-DD-03` | current inputs exist in separate snapshots |
| `WMR-H29` | Curation result Event to Graph visibility | `WMR-DD-02` | `WMR-DD-02` | design pending |
| `WMR-H30` | Curation result Event to configured Belief revision | `WMR-DD-02` | `WMR-DD-02` | design pending |
| `WMR-H31` | Curation-derived Belief revision to Agent acceptance | `WMR-DD-02` | `WMR-DD-03` | design pending |
| `WMR-H32` | Execution outcome to semantic owner observation | `WMR-DD-04` | `WMR-DD-04` | current partial |
| `WMR-H33` | returned owner observation to Events | `WMR-DD-04` | `WMR-DD-04` | current partial |
| `WMR-H34` | returned owner Event to Graph visibility | `WMR-DD-04` | `WMR-DD-04` | current positions exist independently |
| `WMR-H35` | returned owner Event to configured Belief revision | `WMR-DD-04` | `WMR-DD-04` | current partial by installed mapping |
| `WMR-H36` | return-derived Belief revision to Agent acceptance | `WMR-DD-04` | `WMR-DD-04` | current partial for Goal satisfaction |

This registry normalizes the handoffs in the existing connectivity review and adds the Planner, PDS, activation, return-visibility, and retirement edges needed by the complete design program. Later phases may refine an edge without silently changing its producer or consumer owner.

## Active Proposed Slice Edge Details

The proposed `WMR-DD-01` coherence horizon contains `WMR-H01` through `WMR-H06`. Only `WMR-H01` through `WMR-H03` must close in that slice. `WMR-H04` through `WMR-H06` are declared downstream relationships whose consumers remain deferred.

### WMR-H01 Product Observation To Events

| Obligation | Proposed design position |
| --- | --- |
| producer owner | workspace, docs, or dependency security according to product meaning |
| producer product | exact addressable observation set under source identity, snapshot, owner revision, and completeness boundary |
| durable producer position | owner publication operation or exact owner receipt that survives append retry |
| consumer owner | Events append authority |
| consumer acceptance position | Event append receipt with ledger identity, stable record identity, and exact sequence |
| identity and idempotency | owner observation identity includes source revision and observed semantic unit, while Event record identity deduplicates carriage |
| supersession | owner policy names replacement, withdrawal, or continued coexistence across revisions |
| wait | owner declares pending source, incomplete observation, or no eligible publication without claiming global quiescence |
| wake | source delivery, durable deadline, or operator action named by the owner |
| fence | source lineage and activation generation classify or reject late publication |
| restart | owner source cursor and publication operation, followed by idempotent Event append |
| current gap | workspace builds rich envelopes but ordinary publication is best effort, while dependency security lacks complete durable publication |

### WMR-H02 Events To Graph Materialization

| Obligation | Proposed design position |
| --- | --- |
| producer owner | Events |
| producer product | sequenced `EventRecord` carrying producer-owned objects, relation occurrences, provenance, and payload |
| durable producer position | exact ledger identity and sequence |
| consumer owner | World State Graph projection |
| consumer acceptance position | graph cursor at or beyond the sequence after all projection writes and derived outbox work are durable |
| identity and idempotency | Event record identity plus ledger identity and source sequence |
| supersession | producer lifecycle facts remain durable inputs, graph projection does not infer replacement from endpoint equality |
| wait | ledger quiet beyond the graph cursor or projection blocked on invalid owner publication |
| wake | Event watermark for canonical append plus durable fallback for direct-store derived Events |
| fence | ledger identity, retention boundary, admitted owner route, and source revision |
| restart | graph cursor, projection indexes, and derived Event outbox |
| current gap | implementation is strong for `workspace_fs`, `context`, and `execution`, while future owners are filtered and relation occurrence identity is not returned directly by walks |

### WMR-H03 Owner Graph Projections To TraversalCut

| Obligation | Proposed design position |
| --- | --- |
| producer owner | each publishing domain through its graph publication contract |
| producer product | exact `GraphObjectPublication` and `RelationOccurrencePublication` revisions |
| durable producer position | one owner revision receipt and projection cursor per participating owner |
| consumer owner | Graph and Traversal query boundary |
| consumer acceptance position | bounded `TraversalResult` bound to one exact `TraversalCut` |
| identity and idempotency | normalized query plus exact owner revision set derives result identity |
| supersession | owner-shaped presence and currentness lifecycle, never generic endpoint replacement |
| wait | cut reports missing owner receipts or bounded frontier without interpreting incompleteness as absence |
| wake | participating owner revision or projection-cursor advancement |
| fence | temporal, branch, perspective, owner, and currentness policy are part of query and cut identity |
| restart | owner projections, owner receipts, normalized query, and exact cut identity |
| current gap | canonical design defines the products, while current code returns bare relation values and a generic `current_only` flag |

### WMR-H04 TraversalCut To Later PlannerCut Assembly

| Obligation | Proposed design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | occurrence-rich bounded `TraversalResult` with paths, frontier, truncation state, provenance, and exact `TraversalCut` |
| durable producer position | exact participating owner revision set and normalized traversal query |
| consumer owner | Planner |
| consumer acceptance position | deferred to `WMR-DD-03`, where Planner assembles one immutable `PlannerCut` from the graph cut and every other exact source revision |
| producer identity and idempotency | equal graph owner revisions, scope, query policy, and projection version produce equal `TraversalCut` identity |
| consumer identity and idempotency | deferred to `WMR-DD-03` with Belief, Causation, Regime, directive, Capability catalog, scope, authority, and projection policy revisions |
| supersession | a newer `TraversalCut` creates a distinct graph input and never rewrites the predecessor |
| wait and wake | graph-input availability closes in `WMR-DD-01`, while complete reasoning-cut waits and wakes close in `WMR-DD-03` |
| fence | temporal, branch, perspective, owner, and currentness policy belong to the producer cut, while Agent authority and directive scope are deferred to Planner assembly |
| restart | exact graph owner revisions and query policy reproduce the producer cut, while full Planner restart closes in `WMR-DD-03` |
| relationship state | producer defined, consumer deferred |
| current gap | current Planner keeps anchor and source fact identities but loses relation topology, while canonical docs require a complete multi-source `PlannerCut` |

### WMR-H05 TraversalCut To Standing Curation

| Obligation | Proposed design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | bounded cut addressable by exact roots, owner revisions, scope, perspective, and bounds |
| consumer owner | Curation |
| consumer acceptance position | deferred to `WMR-DD-02`, expected to be durable accepted operation or standing-work identity |
| identity and idempotency | producer cut identity is fixed in `WMR-DD-01`, operation identity is deferred |
| wait and wake | consumer-owned declaration and structural wake are deferred |
| fence | producer records the activation and perspective inputs it can supply, consumer validation is deferred |
| restart | producer cut is reproducible, consumer restart source is deferred |
| relationship state | producer defined, consumer deferred |

`WMR-DD-01` must not claim Curation readiness, liveness, quiescence, or completion from this entry.

### WMR-H06 Traversal Products To Configured Belief

| Obligation | Proposed design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | current anchor, exact provenance, or eligible owner publication under a named mapping |
| consumer owner | Belief |
| current consumer position | belief revision and view committed under one exact Belief key |
| current gap | graph-only change does not generally make an already assessed Belief key eligible again |
| active-slice duty | preserve exact producer identity and declare the later settlement dependency |
| deferred duty | `WMR-DD-02` defines mapping, eligibility, wait, wake, and visibility closure |
| relationship state | current partial, settlement closure deferred |

## Lifecycle Projection

The handoff ledger and lifecycle view are two projections of the same relationship graph.

For every edge, lifecycle design must eventually name:

- activation generation and participant incarnation
- producer readiness receipt
- durable producer position
- consumer visibility position
- owner progress or wait receipt
- structurally resolvable wake reference
- late-delivery and generation fence
- restart source
- safe point and unresolved-operation summary
- stop or retirement receipt

During `WMR-DD-01`, only the lifecycle information already meaningful for `WMR-H01` through `WMR-H03` may be closed. `WMR-H04` records a reproducible producer position but defers complete Planner consumption. Later owner semantics remain deferred. A clean process tick, empty queue, Event append, or absent consumer cannot serve as readiness or quiescence evidence.

## Gate Use

Every Delivery Gate selects edge identifiers from this ledger. Gate criteria test only relationships inside the declared coherence horizon.

An edge with a later consumer can pass an earlier design gate only when:

- the producer product and position are exact
- the later consumer and closure phase are named
- unresolved consumer fields remain visibly deferred
- no acceptance claim depends on those deferred fields
- the next phase receives explicit preconditions rather than inferred readiness

`WMR-H18` is not a substitute for `WMR-H29` through `WMR-H36`. The final inspection phase composes lineage from already-closed Curation and Execution return paths. It may not become their first consumer-visibility proof.

The acceptance owner may report a criterion-bound violation. The program owner decides whether it is an evidence correction, active-slice defect, gate-definition defect, unauthorized expansion, later-phase dependency, or authorized exception.

## Evidence Lineage

Primary evidence:

- [producer-consumer connectivity review](runtime_initialization_lifecycle/reviews/producer_consumer_connectivity_review.md)
- [runtime lifecycle and supervision review](runtime_initialization_lifecycle/reviews/runtime_lifecycle_and_supervision_review.md)
- [detailed design workstream framing](detailed_design_workstream_framing.md)
- [current code ground map](current_code_groundmap.md)
- [canonical Graph and Traversal design](../../../cognitive_architecture/world_model/graph/README.md)
- [canonical Planner design](../../../cognitive_architecture/world_model/planner/README.md)

No relationship in this ledger is implementation authorization.
