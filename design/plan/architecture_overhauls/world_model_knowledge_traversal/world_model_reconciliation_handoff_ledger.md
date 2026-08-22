# World Model Reconciliation Handoff And Lifecycle Ledger

Date: 2026-08-22

Status: active detailed-design handoff artifact

Implementation authorization: none, `WMR-DD-03` design only

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
| `WMR-H05` | `TraversalCut` to standing Curation | `WMR-DD-01` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H06` | Traversal products to configured Belief | `WMR-DD-01` | `WMR-DD-02` | design accepted, current implementation partial |
| `WMR-H07` | Agent Plan authorization to planned Curation | `WMR-DD-03` | `WMR-DD-02` and `WMR-DD-03` | Curation consumer design accepted, Agent producer active |
| `WMR-H08` | Curation terminal result to Events | `WMR-DD-02` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H09` | Events to configured Belief | `WMR-DD-02` | `WMR-DD-02` | design accepted for configured routes, Curation implementation unproved |
| `WMR-H10` | Belief revision to Agent | `WMR-DD-02` | `WMR-DD-03` | Belief producer design accepted, Agent consumer active |
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
| `WMR-H26` | Belief revision to `PlannerCut` assembly | `WMR-DD-02` | `WMR-DD-03` | Belief producer design accepted, Planner consumer active |
| `WMR-H27` | Causation and Regime revisions to `PlannerCut` assembly | existing canonical owners | `WMR-DD-03` | canonical products exist, integrated cut relationship pending |
| `WMR-H28` | directive context and Capability catalog revision to `PlannerCut` assembly | existing Agent and Capability owners | `WMR-DD-03` | current inputs exist in separate snapshots |
| `WMR-H29` | Curation result Event to Graph visibility | `WMR-DD-02` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H30` | Curation result Event to configured Belief revision | `WMR-DD-02` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H31` | Curation-derived Belief revision to Agent acceptance | `WMR-DD-02` | `WMR-DD-03` | Belief producer design accepted, Agent consumer active |
| `WMR-H32` | Execution outcome to semantic owner observation | `WMR-DD-04` | `WMR-DD-04` | current partial |
| `WMR-H33` | returned owner observation to Events | `WMR-DD-04` | `WMR-DD-04` | current partial |
| `WMR-H34` | returned owner Event to Graph visibility | `WMR-DD-04` | `WMR-DD-04` | current positions exist independently |
| `WMR-H35` | returned owner Event to configured Belief revision | `WMR-DD-04` | `WMR-DD-04` | current partial by installed mapping |
| `WMR-H36` | return-derived Belief revision to Agent acceptance | `WMR-DD-04` | `WMR-DD-04` | current partial for Goal satisfaction |

This registry normalizes the handoffs in the existing connectivity review and adds the Planner, PDS, activation, return-visibility, and retirement edges needed by the complete design program. Later phases may refine an edge without silently changing its producer or consumer owner.

## Accepted WMR-DD-01 Edge Details

The accepted `WMR-DD-01` coherence horizon contains `WMR-H01` through `WMR-H06`. Only `WMR-H01` through `WMR-H03` close in that slice. `WMR-H04` through `WMR-H06` are declared downstream relationships whose consumers remain deferred.

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

## Accepted WMR-DD-02 Edge Details

The `WMR-DD-02` coherence horizon closes standing Curation intake, terminal result publication, Graph visibility, and configured Belief settlement. It defines the Curation-owned acceptance side of planned work while leaving Agent authorization production and progression to `WMR-DD-03`.

### WMR-H05 TraversalCut To Standing Curation

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | occurrence-rich `TraversalResult` under one immutable `TraversalCut`, including owner-correct hydration |
| durable producer position | exact owner revision set, normalized query, bounds, frontier, and cut identity |
| consumer owner | Curation |
| consumer acceptance position | durable standing selection identity accepted under exact Agent specification, perspective, rule revision, subject, source cut, and activation fence |
| identity and idempotency | equal authority, rule, subject, cut, and selection bound derive equal operation identity |
| supersession | changed authority, rule, source cut, perspective, or bound creates a successor operation |
| wait | unavailable named owner revision, incomplete required cut, uninstalled rule, or no eligible selection |
| wake | exact rule revision, owner publication, Graph projection advancement, or declared deadline |
| fence | Agent, perspective, branch, activation generation, rule revision, source cut, and owner authority |
| restart | standing selection, operation acceptance, terminal result, and consumer visibility positions |
| relationship state | design accepted by `WMR-DG-02`, implementation unproved |

### WMR-H06 Traversal Products To Configured Belief

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Graph and Traversal plus the semantic publication owner retained in provenance |
| producer product | exact anchor, occurrence, provenance, or owner-hydrated product under a named dependency |
| durable producer position | Graph projection cursor and exact owner source revision |
| consumer owner | Belief |
| consumer acceptance position | configured evidence identity followed by immutable Belief revision under one exact key |
| eligibility | installed route names source product, perspective, mapping or dependency revision, and target key |
| idempotency | source publication identity plus installed route revision determines evidence identity |
| wait | no applicable installed route, missing required projection, or key not dirty under declared dependency |
| wake | applicable owner publication, Graph projection advancement, mapping revision, or dependency invalidation |
| fence | belief key, perspective, source owner revision, mapping or comparator revision, and activation generation |
| restart | Belief domain state and consumer cursor reconcile before cursor advancement |
| relationship state | configured settlement design accepted by `WMR-DG-02`, current implementation partial |

### WMR-H07 Agent Plan Authorization To Planned Curation

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Agent |
| producer product | exact authorization envelope carrying the complete immutable Epistemic Operation and binding Agent, Goal, Plan revision, selection reference, frozen context, authority scope, and idempotency key |
| durable producer position | deferred to Agent Plan progression in `WMR-DD-03` |
| consumer owner | Curation |
| consumer acceptance position | Curation acceptance identity after authority, operation shape, source cut, perspective, and generation validation |
| identity and idempotency | Agent authorization identity and Curation operation identity remain distinct and replay-stable |
| wait and wake | Curation waits for an exact authorization and wakes only from that structural identity, not a generic Event |
| fence | authorization lineage, Agent, Goal, Plan revision, product, perspective, branch, and activation generation |
| restart | Curation can resume from acceptance or terminal state, while Agent publication recovery remains deferred |
| relationship state | Curation consumer design accepted by `WMR-DG-02`, Agent producer active in `WMR-DD-03` |

### WMR-H08 Curation Terminal Result To Events

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Curation |
| producer product | one terminal result plus any finite Curation-owned semantic publications |
| durable producer position | exact terminal result identity after domain state is durable |
| consumer owner | Events append authority |
| consumer acceptance position | append receipts naming ledger identity and exact sequence for every required publication |
| identity and idempotency | deterministic result and semantic publication identities derive deterministic Event record identities |
| supersession | owner-issued withdrawal or supersession products, never ledger order alone |
| wait | terminal state exists but one or more required Event append receipts are absent |
| wake | append authority availability or retry scheduling against the same publication identities |
| fence | Curation authority, source cut, perspective, activation generation, and publication policy |
| restart | reconcile durable terminal result with append receipts and retry idempotently |
| relationship state | design accepted by `WMR-DG-02`, implementation unproved |

### WMR-H09 And WMR-H30 Curation Event To Configured Belief Revision

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Curation through Events |
| producer product | exact result Event or semantic publication eligible under one installed evidence route |
| durable producer position | Event ledger identity and result sequence, with Curation source-cut lineage |
| consumer owner | Belief |
| consumer acceptance position | immutable Belief revision after evidence admission and comparator settlement |
| identity and idempotency | Event record, installed mapping or dependency revision, belief key, and perspective determine evidence identity |
| non-applicability | unmapped Curation Events may advance a consumer cursor but do not become evidence |
| wait | missing installed route, missing source dependency, or no affected belief key |
| wake | matching result Event, route revision, or declared dependency invalidation |
| fence | source result, belief key, perspective, route, comparator, and activation generation |
| restart | Belief state commits before the consumer position advances |
| relationship state | design accepted by `WMR-DG-02`, implementation unproved |

### WMR-H29 Curation Result Event To Graph Visibility

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Events carrying Curation-owned graph attachments |
| producer product | sequenced terminal and semantic publication records with producer provenance |
| durable producer position | greatest required Event sequence for the result publication set |
| consumer owner | Graph projection |
| consumer acceptance position | durable projection cursor at or beyond every required sequence after materialization writes complete |
| identity and idempotency | Event record and relation occurrence identities preserve duplicate reduction safety |
| currentness | Curation-owned typed products carry validity, withdrawal, and supersession meaning |
| wait | Graph cursor remains behind a required sequence or publication is structurally inadmissible |
| wake | Event watermark or bounded replay availability |
| fence | ledger identity, retention boundary, producer domain, source cut, perspective, and activation lineage |
| restart | Graph projection store and cursor replay from the last durable position |
| relationship state | design accepted by `WMR-DG-02`, implementation unproved |

### WMR-H10 And WMR-H31 Belief Revision To Agent Acceptance

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Belief |
| producer product | immutable revision bound to exact Curation evidence and comparator policy |
| durable producer position | Belief key and revision sequence |
| consumer owner | Agent |
| consumer acceptance position | deferred durable subscription or Plan-progress position in `WMR-DD-03` |
| wait and wake | producer revision exists; Agent selection, delivery, and wake remain deferred |
| fence | Agent subscription, perspective, branch, Goal or Plan lineage, and activation generation |
| restart | Belief revision is replayable; Agent cursor recovery remains deferred |
| relationship state | Belief producer design accepted by `WMR-DG-02`, Agent consumer active in `WMR-DD-03` |

### WMR-H26 Belief Revision To PlannerCut Assembly

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Belief |
| producer product | immutable revision-bound view with evidence and invalidation lineage |
| durable producer position | exact Belief key and revision identity |
| consumer owner | Planner |
| consumer acceptance position | deferred complete `PlannerCut` assembly in `WMR-DD-03` |
| wait and wake | Belief source availability closes here, multi-source cut selection remains deferred |
| fence | perspective, branch, source evidence, comparator policy, and revision identity |
| restart | exact revision can be reselected, while full cut reconstruction remains deferred |
| relationship state | Belief producer design accepted by `WMR-DG-02`, Planner consumer active in `WMR-DD-03` |

## Active WMR-DD-03 Edge Details

The `WMR-DD-03` coherence horizon closes complete `PlannerCut` assembly, Strategy Plan delivery, Agent Plan judgment, product authorization, Curation handoff, result absorption, and successor progression. It produces an exact Task authorization envelope while leaving Execution acceptance to `WMR-DD-04`.

### WMR-H04 TraversalCut To Immutable PlannerCut

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | accepted occurrence-rich `TraversalResult` and immutable `TraversalCut` |
| durable producer position | exact graph owner revisions, query, bounds, frontier, truncation, and projection policy |
| consumer owner | Planner |
| consumer acceptance position | complete immutable `PlannerCut` or explicit refusal naming invalid sources |
| additional sources | exact Belief, Causation, Regime, directive, Capability, scope, authority, perspective, and projection-policy revisions |
| identity and idempotency | equal source revision set and assembly policy yield equal cut identity |
| wait | missing, stale, conflicting, unauthorized, or unacceptably truncated required source |
| wake | exact source revision, authority, scope, or policy change |
| fence | temporal, transaction, horizon, branch, perspective, Agent, authority, and activation generation |
| restart | reconstruct from native owner revisions and exact assembly policy |
| relationship state | proposed for Gate Acceptance in `WMR-DD-03` |

### WMR-H11 Agent Goal And Cut To Strategy

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Agent using Planner and native catalog owners |
| producer product | ground Goal plus complete `PlannerCut`, exact Curation operation catalog, Strategy policy, predecessor history, and any eligibility-time Planner reassembly result |
| durable producer position | construction request identity or Planner currentness-check identity bound to exact frozen context and returned cut or refusal |
| consumer owner | Strategy |
| consumer acceptance position | verified immutable `StrategyPlan` revision or explicit construction refusal |
| identity and idempotency | equal ground request yields stable semantic Plan identity and ordering |
| wait and wake | Strategy has no runtime wait; Agent waits on a named Planner assembly request and wakes on its exact complete-cut or refusal receipt |
| fence | Agent, Goal, cut, policy, catalog, predecessor, bounds, and generation |
| restart | reconstruct from the frozen request, not private search state |
| relationship state | proposed for Gate Acceptance in `WMR-DD-03` |

### WMR-H12 Strategy Plan Revision To Agent Progression

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Strategy |
| producer product | verified immutable heterogeneous Plan revision with desired conditions, complete Tasks, bounded Epistemic Operations, typed milestone dependencies, explanation, and lineage |
| durable producer position | Plan family, revision, frozen context, semantic body, and predecessor identity |
| consumer owner | Agent |
| consumer acceptance position | durable Plan judgment admitting, rejecting, or superseding the revision |
| identity and idempotency | repeat delivery of one Plan revision reuses the same judgment identity |
| supersession | successor Plan preserves completed facts and predecessor lineage without rewriting history |
| currentness proof | before product eligibility, Agent records a Planner reassembly receipt and compares its complete cut identity with the Plan frozen cut |
| invalidation position | a different returned cut or assembly refusal invalidates eligibility and names the successor or wait decision |
| wait | missing valid Plan, missing Planner currentness receipt, invalid frozen context, unresolved authority, or successor required |
| wake | exact Plan revision, completed Planner currentness check, authority decision, or owner milestone |
| fence | Agent, Goal, Plan revision, context, directive, branch, perspective, authority, and generation |
| restart | Agent judgment, Planner currentness checks, progression positions, and exact Plan revision |
| relationship state | proposed for Gate Acceptance in `WMR-DD-03` |

### WMR-H07 Agent Authorization To Planned Curation

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Agent |
| producer product | product-specific authorization carrying the complete immutable Epistemic Operation and binding Goal, Plan revision, selection reference, context, authority, generation, and idempotency |
| durable producer position | Agent authorization decision committed before handoff |
| consumer owner | Curation |
| consumer acceptance position | exact Curation acceptance or rejection identity defined by `WMR-DD-02` |
| identity and idempotency | authorization and Curation acceptance remain distinct and stable under retry |
| wait | product blocked, stale, unauthorized, already terminal, or durable Curation acceptance or rejection receipt absent |
| wake | exact dependency milestone, authority update, consumer receipt, or successor decision |
| fence | Agent, Goal, Plan revision, product, cut, perspective, authority, and generation |
| restart | Agent authorization and in-flight retry position plus Curation acceptance or rejection and terminal state |
| relationship state | proposed full closure in `WMR-DD-03` |

### WMR-H13 Agent Authorization To Deferred Execution Admission

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Agent |
| producer product | one authorized independently complete Task with Agent, Goal, Plan, selection reference, context, authority, generation, and idempotency lineage |
| durable producer position | Agent product authorization committed before delivery |
| consumer owner | Execution |
| consumer acceptance position | deferred to `WMR-DD-04` |
| cardinality | one Plan and one Goal may produce several independently authorized Tasks |
| excluded material | heterogeneous Plan, Epistemic Operations, private Strategy search state, and world-model progression logic |
| wait and wake | producer eligibility and authorization close here, consumer receipt and retry closure remain deferred |
| fence | exact Plan revision, Task identity, context, authority, Goal lifecycle, and activation generation |
| restart | producer authorization is durable, Execution admission recovery remains deferred |
| relationship state | producer defined, consumer deferred |

### WMR-H10 And WMR-H31 Belief Revision To Agent Acceptance

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Belief |
| producer product | immutable revision under exact key, perspective, source evidence, comparator, and invalidation lineage |
| durable producer position | Belief revision identity and sequence |
| consumer owner | Agent |
| consumer acceptance position | durable milestone acceptance tied to one Plan dependency or Goal satisfaction review |
| identity and idempotency | subscription, revision, Plan dependency, and Agent context determine acceptance identity |
| wait | no newer relevant revision or revision does not satisfy the declared dependency |
| wake | exact relevant revision, invalidation, or subscription change |
| fence | Agent, Goal, Plan revision, dependency, perspective, branch, and generation |
| restart | Agent input cursor and milestone decision advance only after durable absorption |
| relationship state | proposed for Gate Acceptance in `WMR-DD-03` |

### WMR-H19 Admitted Result To Agent Reconciliation

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Curation, Graph, Belief, or later Execution and semantic observation owners according to the Plan milestone |
| producer product | exact owner result and durable visibility position |
| durable producer position | owner-specific terminal, projection, revision, outcome, or observation position |
| consumer owner | Agent |
| consumer acceptance position | milestone acceptance, new product eligibility, successor request, or Goal disposition decision |
| identity and idempotency | Plan dependency, milestone kind, producer identity, and owner position determine absorption identity |
| wait | declared milestone absent, stale, mismatched, or invalidated |
| wake | exact owner milestone, invalidation, authority change, or deadline |
| fence | Plan revision, product, dependency, context, perspective, branch, authority, and generation |
| restart | owner positions, Agent input cursors, and durable progression decisions |
| relationship state | Curation, Graph, and Belief paths proposed for closure, Execution return remains deferred |

### WMR-H26 Through WMR-H28 Native Revisions To PlannerCut

| Obligation | Detailed-design position |
| --- | --- |
| producer owners | Belief, Causation, Regime, Agent directive, and Capability catalog owners |
| producer products | exact immutable native revisions and hydration bodies required by the decision context |
| durable producer positions | owner revision identities with provenance, invalidation, and authority lineage |
| consumer owner | Planner |
| consumer acceptance position | one complete `PlannerCut` citing every selected revision and assembly policy |
| identity and idempotency | equal revision set, scope, authority, and policy yield equal cut identity |
| wait | any required revision missing, stale, conflicted, unauthorized, or outside scope |
| wake | exact owner successor revision, policy change, or authority update |
| fence | native revision lineage, Agent perspective, branch, temporal scope, and generation |
| restart | native owner stores plus exact Planner assembly request |
| relationship state | proposed for Gate Acceptance in `WMR-DD-03` |

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

`WMR-DD-01` closes owner publication through Traversal. `WMR-DD-02` closes Curation-local authorship and visibility. `WMR-DD-03` adds complete reasoning-cut assembly, Plan judgment, product authorization, Curation handoff, milestone absorption, and Agent-local progression. `WMR-H13` still defers Execution consumer closure. A clean process tick, empty queue, Event append, or absent consumer cannot serve as readiness or quiescence evidence.

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
