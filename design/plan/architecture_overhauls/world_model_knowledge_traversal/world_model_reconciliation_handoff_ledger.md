# World Model Reconciliation Handoff And Lifecycle Ledger

Date: 2026-08-22

Status: historical gates accepted, Startup integration candidate awaiting independent assurance and user Gate Acceptance

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
| `WMR-H04` | `TraversalCut` to immutable `PlannerCut` | `WMR-DD-01` | `WMR-DD-03` | design accepted, implementation unproved |
| `WMR-H05` | `TraversalCut` to standing Curation | `WMR-DD-01` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H06` | Traversal products to configured Belief | `WMR-DD-01` | `WMR-DD-02` | design accepted, current implementation partial |
| `WMR-H07` | Agent Plan authorization to planned Curation | `WMR-DD-03` | `WMR-DD-02` and `WMR-DD-03` | design accepted, implementation unproved |
| `WMR-H08` | Curation terminal result to Events | `WMR-DD-02` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H09` | Events to configured Belief | `WMR-DD-02` | `WMR-DD-02` | design accepted for configured routes, Curation implementation unproved |
| `WMR-H10` | Belief revision to Agent | `WMR-DD-02` | `WMR-DD-03` | design accepted, current implementation partial |
| `WMR-H11` | Agent Goal and cut to Strategy | `WMR-DD-03` | `WMR-DD-03` | design accepted, current implementation partial for executable-only candidate |
| `WMR-H12` | Strategy Plan revision to Agent progression | `WMR-DD-03` | `WMR-DD-03` | design accepted, implementation unproved |
| `WMR-H13` | Agent authorization to Execution admission | `WMR-DD-03` | `WMR-DD-04` | design accepted by `WMR-DG-04`, implementation partial |
| `WMR-H14` | Execution admission to unified Task Network | `WMR-DD-04` | `WMR-DD-04` | revision 1 accepted, coherence correction awaiting revision 2 assurance, implementation partial |
| `WMR-H15` | planning route to package dispatch | `WMR-DD-04` | `WMR-DD-04` | design accepted by `WMR-DG-04`, current handoff process-local |
| `WMR-H16` | unified Task Network ready work to dispatch | `WMR-DD-04` | `WMR-DD-04` | revision 1 accepted, shared-attribution correction awaiting revision 2 assurance, current local behavior strong |
| `WMR-H17` | Execution outcome to Events | `WMR-DD-04` | `WMR-DD-04` | design accepted by `WMR-DG-04`, current Task publication implemented |
| `WMR-H18` | integrated result lineage inspection | `WMR-DD-02` through `WMR-DD-06` | `WMR-DD-07` | proof projection accepted by `WMR-DG-07`, owner-specific handoffs accepted earlier |
| `WMR-H19` | admitted result to Agent reconciliation | `WMR-DD-03` | `WMR-DD-03` | design accepted through Execution return by `WMR-DG-04`, implementation partial |
| `WMR-H20` | PDS package to owner installation receipts | `WMR-DD-05` | `WMR-DD-05` | design accepted by `WMR-DG-05`, current package installation strong but owner set incomplete |
| `WMR-H21` | installed product to Agent genesis plan | `WMR-DD-05` | `WMR-DD-05` | design accepted by `WMR-DG-05`, implementation partial |
| `WMR-H22` | prepared product to activation generation | `WMR-DD-05` | `WMR-DD-06` | design accepted by `WMR-DG-06`, standalone substrate disconnected |
| `WMR-H23` | activation generation to participant readiness | `WMR-DD-06` | `WMR-DD-06` | design accepted by `WMR-DG-06`, current partial and disconnected |
| `WMR-H24` | owner progress to wait and wake closure | owner phases | `WMR-DD-06` | design accepted by `WMR-DG-06`, current partial and disconnected |
| `WMR-H25` | owner safe points to fenced retirement | owner phases | `WMR-DD-06` | design accepted by `WMR-DG-06`, current implementation absent or disconnected |
| `WMR-H26` | Belief revision to `PlannerCut` assembly | `WMR-DD-02` | `WMR-DD-03` | design accepted, current implementation partial |
| `WMR-H27` | Causation and Regime revisions to `PlannerCut` assembly | existing canonical owners | `WMR-DD-03` | design accepted, current implementation partial |
| `WMR-H28` | directive context and Capability catalog revision to `PlannerCut` assembly | existing Agent and Capability owners | `WMR-DD-03` | design accepted, current implementation partial |
| `WMR-H29` | Curation result Event to Graph visibility | `WMR-DD-02` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H30` | Curation result Event to configured Belief revision | `WMR-DD-02` | `WMR-DD-02` | design accepted, implementation unproved |
| `WMR-H31` | Curation-derived Belief revision to Agent acceptance | `WMR-DD-02` | `WMR-DD-03` | design accepted, current implementation partial |
| `WMR-H32` | Execution outcome to semantic owner observation | `WMR-DD-04` | `WMR-DD-04` | design accepted by `WMR-DG-04`, implementation partial |
| `WMR-H33` | returned owner observation to Events | `WMR-DD-04` | `WMR-DD-04` | design accepted by `WMR-DG-04`, implementation partial |
| `WMR-H34` | returned owner Event to Graph visibility | `WMR-DD-04` | `WMR-DD-04` | design accepted by `WMR-DG-04`, current positions exist independently |
| `WMR-H35` | returned owner Event to configured Belief revision | `WMR-DD-04` | `WMR-DD-04` | design accepted by `WMR-DG-04`, implementation partial by installed mapping |
| `WMR-H36` | return-derived Belief revision to Agent acceptance | `WMR-DD-04` | `WMR-DD-04` | design accepted by `WMR-DG-04`, implementation partial |

This registry normalizes the handoffs in the existing connectivity review and adds the Planner, PDS, activation, return-visibility, and retirement edges needed by the complete design program. Later phases may refine an edge without silently changing its producer or consumer owner.

## Accepted WMR-DD-01 Edge Details

The accepted `WMR-DD-01` coherence horizon contains `WMR-H01` through `WMR-H06`. Only `WMR-H01` through `WMR-H03` close in that slice. `WMR-H04` through `WMR-H06` are declared downstream relationships whose consumers remain deferred.

### WMR-H01 Product Observation To Events

| Obligation | Accepted design position |
| --- | --- |
| producer owner | workspace, docs, or dependency security according to product meaning |
| producer product | exact addressable observation set, typed publication batch, and completeness receipt under source identity, snapshot, owner revision, and completeness boundary |
| durable producer position | durable publication operation or outbox, or authoritative durable source revision plus complete versioned enumeration rule and mandatory restart scan |
| consumer owner | Events append authority |
| consumer acceptance position | Event append receipt with ledger identity, stable record identity, and exact sequence |
| identity and idempotency | owner observation identity includes source revision and observed semantic unit, Event record identity binds the typed publication batch, and neutral object and relation attachments remain routing hints rather than semantic authority |
| supersession | owner policy names replacement, withdrawal, or continued coexistence across revisions |
| wait | owner declares pending source, incomplete observation, or no eligible publication without claiming global quiescence |
| wake | source delivery, durable deadline, or operator action named by the owner |
| fence | source lineage and activation generation classify or reject late publication |
| restart | durable publication operation, or authoritative source revision plus mandatory enumeration, followed by idempotent Event append |
| current gap | workspace builds rich envelopes but ordinary publication is best effort, while dependency security lacks complete durable publication |

### WMR-H02 Events To Graph Materialization

| Obligation | Accepted design position |
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

| Obligation | Accepted design position |
| --- | --- |
| producer owner | each publishing domain through its graph publication contract |
| producer product | exact `GraphObjectPublication` and `RelationOccurrencePublication` revisions plus required owner completeness receipts |
| durable producer position | one owner revision receipt and projection cursor per participating owner |
| consumer owner | Graph and Traversal query boundary |
| consumer acceptance position | bounded `TraversalResult` bound to one exact `TraversalCut` and carrying required completeness receipt references and statuses |
| identity and idempotency | normalized query plus exact owner revision set derives result identity |
| supersession | owner-shaped presence and currentness lifecycle, never generic endpoint replacement |
| wait | cut reports missing owner receipts or bounded frontier without interpreting incompleteness as absence |
| wake | participating owner revision or projection-cursor advancement |
| fence | temporal, branch, perspective, owner, and currentness policy are part of query and cut identity |
| restart | owner projections, owner receipts, normalized query, and exact cut identity |
| current gap | canonical design defines the products, while current code returns bare relation values and a generic `current_only` flag |

### WMR-H04 TraversalCut To Later PlannerCut Assembly

| Obligation | Accepted design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | occurrence-rich bounded `TraversalResult` with paths, frontier, truncation state, provenance, and exact `TraversalCut` |
| durable producer position | exact participating owner revision set and normalized traversal query |
| consumer owner | Planner |
| consumer acceptance position | complete immutable `PlannerCut` or explicit refusal naming invalid sources, as accepted by `WMR-DG-03` |
| producer identity and idempotency | equal graph owner revisions, scope, query policy, and projection version produce equal `TraversalCut` identity |
| consumer identity and idempotency | equal exact source revision set, scope, authority, and assembly policy yield equal `PlannerCut` identity |
| supersession | a newer `TraversalCut` creates a distinct graph input and never rewrites the predecessor |
| wait and wake | Planner waits on missing, stale, conflicting, unauthorized, or unacceptably truncated required sources and wakes on an exact source, policy, or authority update |
| fence | temporal, transaction, horizon, branch, perspective, Agent, authority, and activation generation |
| restart | native owner stores plus exact Planner assembly request reproduce the accepted consumer decision |
| relationship state | full relationship design accepted by `WMR-DG-03`, implementation unproved |
| current gap | current Planner keeps anchor and source fact identities but loses relation topology, while canonical docs require a complete multi-source `PlannerCut` |

### WMR-H05 TraversalCut To Standing Curation

| Obligation | Accepted design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | bounded cut addressable by exact roots, owner revisions, scope, perspective, and bounds |
| consumer owner | Curation |
| consumer acceptance position | durable standing selection identity accepted under exact Agent specification, perspective, rule revision, subject, source cut, and activation fence |
| identity and idempotency | equal authority, rule, subject, cut, and selection bound derive equal operation identity |
| wait and wake | Curation waits on an unavailable required revision, incomplete cut, uninstalled rule, or no eligible selection and wakes on the exact rule, owner publication, projection, or deadline position |
| fence | Agent, perspective, branch, activation generation, rule revision, source cut, and owner authority |
| restart | standing selection, operation acceptance, terminal result, and consumer visibility positions |
| relationship state | design accepted by `WMR-DG-02`, implementation unproved |

The `WMR-DD-01` receipt alone does not claim Curation readiness, liveness, quiescence, or completion. The later consumer contract is established by `WMR-DG-02`.

### WMR-H06 Traversal Products To Configured Belief

| Obligation | Accepted design position |
| --- | --- |
| producer owner | Graph and Traversal |
| producer product | current anchor, exact provenance, or eligible owner publication under a named mapping |
| consumer owner | Belief |
| current consumer position | belief revision and view committed under one exact Belief key |
| current gap | graph-only change does not generally make an already assessed Belief key eligible again |
| `WMR-DD-01` phase-local duty | preserve exact producer identity and declare the later settlement dependency |
| downstream closure | `WMR-DD-02` defines mapping, eligibility, wait, wake, and visibility closure |
| relationship state | configured settlement design accepted by `WMR-DG-02`, current implementation partial |

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
| durable producer position | Agent authorization decision committed before handoff, as accepted in the `WMR-DD-03` detail below |
| consumer owner | Curation |
| consumer acceptance position | Curation acceptance identity after authority, operation shape, source cut, perspective, and generation validation |
| identity and idempotency | Agent authorization identity and Curation operation identity remain distinct and replay-stable |
| wait and wake | Curation waits for an exact authorization and wakes only from that structural identity, not a generic Event |
| fence | authorization lineage, Agent, Goal, Plan revision, product, perspective, branch, and activation generation |
| restart | Agent authorization and in-flight retry position plus Curation acceptance or rejection and terminal state |
| relationship state | full relationship design accepted by `WMR-DG-03`, implementation unproved |

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
| consumer acceptance position | durable milestone acceptance tied to one Plan dependency or Goal satisfaction review, as accepted in the `WMR-DD-03` detail below |
| wait and wake | Agent waits when no relevant revision satisfies the declared dependency and wakes on the exact revision, invalidation, or subscription change |
| fence | Agent subscription, perspective, branch, Goal or Plan lineage, and activation generation |
| restart | Agent input cursor and milestone decision advance only after durable absorption |
| relationship state | full relationship design accepted by `WMR-DG-03`, current implementation partial |

### WMR-H26 Belief Revision To PlannerCut Assembly

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Belief |
| producer product | immutable revision-bound view with evidence and invalidation lineage |
| durable producer position | exact Belief key and revision identity |
| consumer owner | Planner |
| consumer acceptance position | complete immutable `PlannerCut` or explicit refusal naming invalid sources, as accepted in the `WMR-DD-03` detail below |
| wait and wake | Planner waits on a missing, stale, conflicting, unauthorized, or out-of-scope required revision and wakes on the exact successor revision, policy, or authority update |
| fence | perspective, branch, source evidence, comparator policy, and revision identity |
| restart | native owner stores plus the exact Planner assembly request reproduce the accepted consumer decision |
| relationship state | full relationship design accepted by `WMR-DG-03`, current implementation partial |

## Accepted WMR-DD-03 Edge Details

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
| relationship state | design accepted by `WMR-DG-03`, implementation unproved |

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
| relationship state | design accepted by `WMR-DG-03`, current implementation partial |

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
| relationship state | design accepted by `WMR-DG-03`, implementation unproved |

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
| relationship state | design accepted by `WMR-DG-03`, implementation unproved |

### WMR-H13 Agent Authorization To Deferred Execution Admission

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Agent |
| producer product | one authorized independently complete Task with Agent, Goal, Plan, selection reference, context, authority, generation, and idempotency lineage |
| durable producer position | Agent product authorization committed before delivery |
| consumer owner | Execution |
| consumer acceptance position | durable accepted, duplicate, rejected, or conflicted Execution admission decision proposed in `WMR-DD-04` |
| cardinality | one Plan and one Goal may produce several independently authorized Tasks |
| excluded material | heterogeneous Plan, Epistemic Operations, private Strategy search state, and world-model progression logic |
| wait and wake | producer eligibility and authorization close in `WMR-DD-03`, while accepted `WMR-DD-04` defines the exact consumer decision and retry closure |
| fence | exact Plan revision, Task identity, context, authority, Goal lifecycle, and activation generation |
| restart | Agent authorization and Execution admission journal reproduce the consumer decision |
| relationship state | producer accepted by `WMR-DG-03`, consumer accepted by `WMR-DG-04`, implementation unproved |

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
| relationship state | design accepted by `WMR-DG-03`, current implementation partial |

## Accepted WMR-DD-04 Edge Details

The corrected `WMR-DD-04` coherence horizon begins with one complete Agent-authorized Task, passes through Goal Set admission and the unified Task Network, and closes through exact returned owner evidence plus Agent milestone absorption. Runtime implementation, PDS, and activation-wide lifecycle remain outside the slice.

### WMR-H13 Agent Authorization To Execution Admission

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Agent |
| producer product | one product-specific authorization carrying complete Task body, Goal, Plan revision, selection reference, frozen context, authority, generation, and idempotency |
| durable producer position | Agent authorization decision committed before handoff |
| consumer owner | Execution |
| consumer acceptance position | durable accepted, duplicate, rejected, or conflicted decision |
| identity and idempotency | authorization and admission identities remain distinct and replay-stable |
| wait and wake | Execution waits on an exact authorization or current contract revision and wakes on that structural product |
| fence | Agent, Goal, Plan, Task, Capability contracts, authority, and generation |
| restart | Agent authorization plus Execution admission journal |
| relationship state | design accepted by `WMR-DG-04`, implementation partial |

### WMR-H14 And WMR-H15 Admission Through Unified Coherence And Durable Dispatch Route

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Execution Goal Set, Planning, and lowering |
| producer product | deterministic lowering, compatibility or separation decision, admission attribution set, and route product for one admitted Task |
| durable producer position | Goal Set admission receipt, lowering identity, coherence decision, unified Task Network mutation, and durable route product or exact reconstruction inputs |
| consumer owner | Task Network and dispatch |
| consumer acceptance position | committed operational node lineage with complete admission attribution and eligible route visible to bounded dispatch selection |
| identity and idempotency | Task, admission, compatibility inputs, unified network, binding, and routing revision derive stable mutation, attribution, and route identities |
| wait and wake | predecessor conflict, missing binding, or route absent waits on exact network or binding successor |
| fence | Task, complete contributing admission set, network revision, Capability contract, exact binding revision, exact routing-rule revision, authority, and generation |
| restart | Goal Set journal, coherence decisions, unified Task Network journal, and durable route or mandatory deterministic reconstruction from admitted Task, exact binding revision, and exact routing-rule revision |
| relationship state | revision 1 return path accepted, unified coherence correction awaiting `WMR-DG-04` revision 2 assurance |

### WMR-H16 Ready Task To Fenced Attempt

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Task Network |
| producer product | exact ready operational node, complete contributing admission set, and initialization inputs |
| durable producer position | network revision and readiness state |
| consumer owner | Execution dispatch and provider realization seam |
| consumer acceptance position | durable claim, attempt, and external operation identity before effect reliance |
| identity and idempotency | operational node, every admission attribution, claim, attempt, operation, contract, exact installed binding revision, resolved Capability instance, effect target, input digest, authority, and generation remain explicit |
| wait and wake | unresolved dependency, input, claim, or worker waits on exact state change or expiry |
| fence | network, lifecycle epoch, worker, participant incarnation, activation generation, authority, exact binding revision, resolved Capability instance, and effect target |
| restart | unified network state, admission attributions, claim, attempt, operation receipt, installed binding revision, effect target, and unresolved-effect account |
| relationship state | revision 1 effect path accepted, shared-attribution correction awaiting `WMR-DG-04` revision 2 assurance |

### WMR-H17 Execution Outcome To Events

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Execution |
| producer product | durable succeeded, failed, cancelled, or uncertain Task outcome with artifacts and effect lineage |
| durable producer position | exact outcome and publication outbox identity |
| consumer owner | Events append authority |
| consumer acceptance position | append receipt naming ledger and exact sequence |
| identity and idempotency | outcome derives a deterministic Event publication identity |
| wait and wake | terminal outcome with no append receipt waits on Event availability or retry |
| fence | Task, claim, attempt, operation, outcome, ledger, and generation |
| restart | outcome outbox plus Event append receipt reconciliation |
| relationship state | design accepted by `WMR-DG-04`, current Task publication implemented |

### WMR-H32 Execution Outcome To Semantic Owner Observation

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Execution |
| producer product | exact outcome, artifacts, source target, and attempt lineage |
| durable producer position | Task outcome and Event publication positions |
| consumer owner | workspace, docs, dependency security, or another named semantic owner |
| consumer acceptance position | successor or unchanged owner observation plus exact completeness receipt |
| identity and idempotency | owner source revision and observation identity remain distinct from outcome identity |
| wait and wake | owner waits on source availability or exact observation trigger and wakes on durable source change or outcome lineage |
| fence | outcome, source, owner revision, perspective, branch, and generation |
| restart | owner source cursor and accepted publication operation |
| relationship state | design accepted by `WMR-DG-04`, implementation partial |

### WMR-H33 Returned Owner Evidence To Events

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | semantic owner |
| producer product | exact owner observation batch, completeness receipt, and deterministic publication operation |
| durable producer position | owner revision and publication outbox position |
| consumer owner | Events append authority |
| consumer acceptance position | append receipt naming ledger and exact sequence |
| identity and idempotency | owner revision, publication operation, and Event publication key remain replay-stable |
| wait and wake | owner publication without append receipt waits on Event availability or retry |
| fence | owner revision, publication key, ledger, perspective, branch, and generation |
| restart | owner publication outbox and Event append receipt reconciliation |
| relationship state | design accepted by `WMR-DG-04`, implementation partial |

### WMR-H34 Returned Owner Event To Graph Visibility

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | Events |
| producer product | accepted returned owner Event at an exact ledger sequence |
| durable producer position | Event append receipt |
| consumer owner | Graph |
| consumer acceptance position | Graph projection revision and cursor covering the exact Event sequence |
| identity and idempotency | ledger, Event record, sequence, and projection revision remain explicit |
| wait and wake | projection behind the returned Event waits on the exact sequence or projection retry |
| fence | ledger, sequence, projection revision, perspective, branch, and generation |
| restart | Event cursor plus Graph projection checkpoint |
| relationship state | design accepted by `WMR-DG-04`, current positions exist independently |

### WMR-H35 Returned Owner Evidence To Configured Belief

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | semantic owner through its accepted Event and evidence route |
| producer product | exact owner evidence under one installed route revision |
| durable producer position | owner revision and routed evidence position |
| consumer owner | configured Belief |
| consumer acceptance position | immutable Belief revision citing exact owner evidence and predecessor |
| identity and idempotency | belief key, owner evidence, route revision, comparator, and predecessor determine the revision |
| wait and wake | unrouted or unsettled evidence waits on exact route availability or predecessor revision |
| fence | owner revision, route revision, belief key, predecessor, perspective, branch, and generation |
| restart | owner evidence, route revision, and Belief revision chain |
| relationship state | design accepted by `WMR-DG-04`, implementation partial by installed route |

### WMR-H36 Return-Derived Belief To Agent Acceptance

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | configured Belief |
| producer product | immutable revision satisfying the exact declared Plan dependency |
| durable producer position | accepted Belief revision |
| consumer owner | Agent |
| consumer acceptance position | durable milestone absorption tied to exact Plan revision and dependency |
| identity and idempotency | Plan dependency, Belief revision, and Agent context determine absorption identity |
| wait and wake | missing, stale, mismatched, or invalidated Belief milestone waits on the exact required successor revision |
| fence | Belief revision, Plan, Goal, Agent, perspective, branch, authority, and generation |
| restart | Belief revision, Agent input cursor, and milestone decision |
| relationship state | design accepted by `WMR-DG-04`, implementation partial |

When a Plan declares direct owner observation, returned Event, or Graph visibility as its milestone, Agent absorbs that exact producer position through `WMR-H19`. It does not wait for an unused Belief branch.

## Accepted WMR-DD-03 Edge Details Continued

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
| relationship state | Curation, Graph, and Belief paths accepted by `WMR-DG-03`, Execution and semantic-owner return accepted through `WMR-DG-04` |

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
| relationship state | design accepted by `WMR-DG-03`, current implementation partial |

## Accepted WMR-DD-05 Edge Details

The `WMR-DD-05` coherence horizon begins with one exact principal product selection and closes at an inert prepared activation closure. It does not claim lifecycle acceptance, participant readiness, current-generation publication, or runtime admission.

### WMR-H20 PDS Package To Native Owner Installation Receipts

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | PDS structural package and router |
| producer product | exact product revision and selected package set, with each semantic package carrying its import closure, route map, structural requirements, and complete native-owner validation set |
| durable producer position | immutable package identity and validation account |
| consumer owners | every named native semantic owner |
| consumer acceptance position | immutable owner revisions and owner-qualified receipts, one complete receipt per selected package, then one product compilation receipt over the complete selected set |
| identity and idempotency | product revision, selected package set, package hashes, imports, routes, owner grammar revisions, owner revisions, package receipts, and product compilation receipt are content-stable |
| wait and wake | unresolved import, invalid component, missing owner route, partial installation, or incomplete selected package set waits on the exact successor or owner result |
| fence | product revision, selected package set, package hashes, import closures, route maps, owner grammar revisions, predecessor receipts, and compilation policy revision |
| restart | product declaration, package manifests, owner registries, partial install accounts, complete package receipts, and product compilation receipt if present |
| relationship state | design accepted by `WMR-DG-05`, current local package path strong but complete product compilation unproved |

### WMR-H21 Installed Product To Agent Genesis

| Obligation | Detailed-design position |
| --- | --- |
| producer owners | PDS product and assignment owners plus native installation router |
| producer product | exact product revision, complete product compilation receipt, situated assignment, finite declared Agent topology, and installed owner revisions per topology position |
| durable producer position | product compilation receipt and assignment identity |
| consumer owner | Agent through a root composition adapter |
| consumer acceptance position | Agent record revision, Agent subscription requests, named source-owner acceptance receipts, deterministic genesis publication operation, per-position genesis receipt, and complete topology receipt |
| identity and idempotency | assignment, topology position, Agent identity body, installed revisions, subscription requests, source contract revisions, source-owner decisions, and publication key remain stable |
| wait and wake | absent product compilation, incomplete topology, identity conflict, missing source-owner subscription decision, or missing publication operation waits on its exact owner successor |
| fence | product, compilation receipt, assignment, topology, Agent identity, subject, perspective, branch, installed revisions, source contracts, and grant lineage |
| restart | assignment, Agent records, subscription requests, source-owner acceptance receipts, genesis outbox, Event receipt if present, and topology receipt |
| relationship state | design accepted by `WMR-DG-05`, current implementation partial |

### WMR-H22 Prepared Product To Activation Generation

| Obligation | Detailed-design position |
| --- | --- |
| producer owners | activation preparation over native owner receipts |
| producer product | inert closure binding product compilation receipt, assignment, activation inputs, complete Agent topology receipt, owner preparations, Capability preparation receipt, participant plan, binding revisions, and authority inputs |
| durable producer position | immutable prepared activation closure with expected-prior position |
| consumer owner | activation lifecycle in `WMR-DD-06` |
| consumer acceptance position | explicitly deferred lifecycle accepted, conflicted, duplicate, or rejected decision |
| identity and idempotency | equal complete input set and expected-prior position yield equal closure identity |
| wait and wake | missing or conflicted native input or Capability selection waits on the exact receipt, binding, plan, policy, or authority successor |
| fence | assignment, activation input, product compilation receipt, topology receipt, Capability preparation receipt, participant plan, binding revisions, authority inputs, and expected prior |
| restart | every named native receipt, Capability preparation receipt, and the inert closure if complete |
| relationship state | producer accepted by `WMR-DG-05`, lifecycle consumer deferred to `WMR-DD-06` |

## Accepted WMR-DD-06 Edge Details

The `WMR-DD-06` coherence horizon begins with one inert prepared closure and closes a generation that is current with truthful liveness, interrupted and recoverable, replaced and draining, or safely retired. Root aggregates exact native-owner receipts without interpreting them.

### WMR-H22 Prepared Closure To Lifecycle Acceptance

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | accepted activation preparation from `WMR-DG-05` |
| producer product | exact inert prepared activation closure and expected-prior generation position |
| durable producer position | immutable closure identity |
| consumer owner | root activation lifecycle structure |
| consumer acceptance position | accepted or duplicate-of-accepted resolves one deterministic preparing generation under closed admission; rejected, conflicted, or duplicates of those outcomes resolve terminal evidence with no generation |
| identity and idempotency | assignment, closure, request key, expected prior, generation ordinal, and predecessor remain explicit |
| wait and wake | stale, incomplete, unauthorized, or conflicted closure waits on exact successor input or head position |
| fence | assignment, prepared closure, authority input, request key, and expected prior head |
| restart | prepared closure, lifecycle decision journal, generation account, and assignment-head authority |
| relationship state | design accepted by `WMR-DG-06`, current implementation disconnected |

### WMR-H23 Generation To Participant Readiness

| Obligation | Detailed-design position |
| --- | --- |
| producer owner | lifecycle and runtime composition over the accepted participant plan |
| producer product | exact generation, participant realizations, registration parity, and participant incarnations |
| durable producer position | preparing or realizing generation under closed admission |
| consumer owners | every realized native participant owner, with required status preserved and unrealized optional specifications absent from registration |
| consumer acceptance position | owner readiness receipt bound to generation, incarnation, realization, checkpoint, installed revisions, subscriptions, and readiness contract |
| identity and idempotency | participant specification, realization, incarnation, registration, supervisor lease, and readiness remain distinct |
| wait and wake | missing binding, incarnation, owner reconstruction, subscription, parity, or readiness for any realized participant waits on exact structural successor |
| fence | generation, participant plan, realization, incarnation, bindings, installed revisions, and admission epoch |
| restart | generation account, plan, realization set, registrations, owner stores, and readiness receipts |
| relationship state | design accepted by `WMR-DG-06`, current substrate disconnected |

### WMR-H24 Owner Progress To Wait And Wake Closure

| Obligation | Detailed-design position |
| --- | --- |
| producer owners | every active native participant owner |
| producer product | exact durable checkpoint, eligible-work account, or complete owner wait receipt |
| durable producer position | owner-native cursor, revision, accepted work, and wait position |
| consumer owner | structural lifecycle liveness projection and named wake owners or transports |
| consumer acceptance position | every wake reference resolves under the generation or liveness projects stalled |
| identity and idempotency | generation, incarnation, checkpoint, wait, wake owner, and transport revision remain explicit |
| wait and wake | owner-defined condition names Event, revision, subscription, deadline, operation, binding, or operator successor |
| fence | generation, incarnation, checkpoint, admission epoch, and wake-owner revision |
| restart | owner checkpoints, waits, wake-resolution receipts, and durable producer-consumer positions |
| relationship state | design accepted by `WMR-DG-06`, polling remains latency fallback only |

### WMR-H25 Owner Safe Points To Fenced Retirement

| Obligation | Detailed-design position |
| --- | --- |
| producer owners | native owners, passive sources, runtime composition, and supervisor within their own boundaries |
| producer product | closed-admission drain receipts, owner safe points, unresolved-operation summaries, passive fences, reverse-order stop receipts, and lease releases |
| durable producer position | exact non-current generation under one closed admission epoch |
| consumer owner | root lifecycle retirement aggregation |
| consumer acceptance position | fenced-quiescence receipt followed by immutable retirement receipt |
| identity and idempotency | generation, incarnation set, checkpoints, unresolved operations, passive positions, stops, releases, and head proof remain explicit |
| wait and wake | undrained work, unresolved effect, unfenced passive path, missing safe point, current head, or incomplete stop waits on its exact owner successor |
| fence | generation, closed admission epoch, current-head proof, incarnation set, owner checkpoints, and structural dependency order |
| restart | admission, owner stores, operations, subscriptions, safe points, head, stop, and lease records |
| relationship state | design accepted by `WMR-DG-06`, runtime retirement unproved |

## Accepted WMR-DD-07 Proof Projection

### WMR-H18 Integrated Result Lineage Inspection

| Obligation | Detailed-design position |
| --- | --- |
| producer owners | every accepted product, semantic, authority, Execution, return, and lifecycle owner from `WMR-DG-01` through `WMR-DG-06` |
| producer product | exact native identity and durable position already defined by its owning gate |
| durable producer position | owner-specific receipt, revision, cursor, checkpoint, decision, admission, outcome, safe point, or lifecycle aggregate |
| consumer owner | read-only inspection projection |
| consumer acceptance position | repeatable evidence resolution for one exact assignment, generation range, topology or Agent, subject, perspective, branch, and time fence |
| identity and idempotency | frozen owner position set yields the same projection without changing owner state |
| wait and wake | unavailable, incomplete, stale, conflicted, or unproved evidence remains explicit and waits only on its native owner successor |
| fence | product revision, assignment, generation, incarnation where applicable, subject, perspective, branch, provenance, and temporal boundary |
| restart | re-resolve exact native owner positions; inspection has no authoritative progress cursor |
| relationship state | proof projection accepted by `WMR-DG-07`, all underlying handoffs accepted earlier |

## Startup PDS Product Projection

The [Startup semantic transition ledger](startup_pds_design_requirements/semantic_transition_ledger.md) projects one concrete product through the accepted handoffs. Its `SPDS-H01` through `SPDS-H23` identifiers are proof-local names, not new program handoff classes.

| Startup edge family | Accepted handoff reuse | Product-specific evidence |
| --- | --- | --- |
| package, assignment, and preparation | `WMR-H20` through `WMR-H22` | exact `meld_startup` package compilation, assignment, topology, Capability preparation, participant plan, and inert closure |
| readiness, epoch, and nonce derivation | `WMR-H23` | complete native readiness, current generation, open admission epoch, deterministic Agent-owned nonce instance |
| standing expectation and mismatch | `WMR-H03`, `WMR-H05`, `WMR-H08` through `WMR-H10`, `WMR-H29` through `WMR-H31` | complete Event coverage, initial `TraversalCut`, Curation result, publications, configured mismatch Belief, Agent Goal decision |
| construction and authorization | `WMR-H04`, `WMR-H11` through `WMR-H13`, `WMR-H19`, `WMR-H26` through `WMR-H28` | complete PlannerCut, immutable heterogeneous Plan, Agent Plan judgment, Task and confirmation-operation authorization |
| unified execution and nonce publication | `WMR-H14` through `WMR-H17`, followed by `WMR-H01` and `WMR-H02` | one admitted Task, operational node, attempt, deterministic `nonce.emit.v1` request, owner-issued Event kind `nonce`, exact append receipt |
| return and satisfaction | `WMR-H04`, `WMR-H07` through `WMR-H12`, `WMR-H19`, `WMR-H29` through `WMR-H31` | Graph visibility, planned confirmation, terminal result, configured realization Belief, Agent milestone, Goal satisfaction |
| liveness, recovery, and inspection | `WMR-H18`, `WMR-H23` through `WMR-H25` | exact owner checkpoints, waits, wakes, epoch successor, safe points, and repeatable nonce account |

The nonce Capability effect composes Execution dispatch with ordinary owner publication. Execution remains ignorant of Startup meaning. The nonce owner remains ignorant of Agent and PDS meaning. Events remains a neutral durability boundary. No new transport or semantic authority is introduced.

Event append and Execution terminal outcome remain independent. Agent Goal satisfaction and unresolved operational-effect closure may also differ. Inspection preserves both positions and does not collapse nonce satisfaction into readiness, health, liveness, or quiescence.

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

`WMR-DD-01` closes owner publication through Traversal. `WMR-DD-02` closes Curation-local authorship and visibility. `WMR-DD-03` adds complete reasoning-cut assembly, Plan judgment, product authorization, Curation handoff, milestone absorption, and Agent-local progression. The corrected `WMR-DD-04` preserves Goal-attributed admission through unified Execution coherence and returned owner evidence. Accepted `WMR-DD-05` supplies exact inert product and Agent-genesis preparation. Accepted `WMR-DD-06` closes lifecycle consumption, realization, readiness, work-or-wait, recovery, replacement, and retirement. Revision 3 of `WMR-DD-07` composes these positions, explicit successor-Plan reconciliation, and the Startup nonce product proof without changing native owner meaning. A clean process tick, empty queue, Event append, prepared closure, or absent consumer cannot serve as readiness or quiescence evidence.

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
- [Startup PDS full design](startup_pds_design_requirements/startup_pds_design_specification.md)
- [Startup PDS semantic transition ledger](startup_pds_design_requirements/semantic_transition_ledger.md)
- [canonical Graph and Traversal design](../../../cognitive_architecture/world_model/graph/README.md)
- [canonical Planner design](../../../cognitive_architecture/world_model/planner/README.md)

No relationship in this ledger is implementation authorization.
