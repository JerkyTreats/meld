# World Model Reconciliation Detailed Design Delivery Program Ledger

Date: 2026-08-21

Mode: design

Status: awaiting approval

Readiness: approval-ready

Canonical initiative name: World Model Reconciliation

User working name: World Model Refactor

Implementation authorization: no active slice, completed detailed-design slice `WMR-DD-01` had zero source-code authority

## Objective And Product Proof

The program must turn the approved World Model Reconciliation architecture into an ordered set of owner-to-owner detailed designs that can later support bounded implementation programs without losing end-to-end coherence.

The architecture failure under pressure is not a missing isolated contract. Current owners already have many durable local products. The failure is that observation, graph visibility, epistemic authorship, Plan progression, Execution, result visibility, activation, and retirement do not yet compose into one fully specified product loop.

The program thesis is:

> Close one owner-to-owner vertical at a time, with its producer-consumer and lifecycle obligations visible, in the order needed to make the next vertical meaningful.

Direct product proof: one traceable specification set in which an independent reviewer can follow both docs freshness paths and the dependency security dissimilarity path through exact owner products, visibility positions, authority changes, waits, wakes, generation fences, restart positions, and safe retirement.

The proof must distinguish:

- an already-correct README that reaches epistemic closure without a Task
- a missing or incorrect README that produces an eligible complete Task and returns owner evidence
- dependency security assessment that separates observation, epistemic judgment, and executable intervention
- Event append, graph materialization, Belief revision, Agent acceptance, and Goal satisfaction
- a locally correct deliverable from a coherent cross-deliverable handoff

This is design-product proof. It is not evidence that the runtime behavior has been implemented.

Explicit non-goals are Rust types, field lists, storage schemas, API signatures, migration packets, implementation commits, estimates, release sequencing, and compatibility work.

Applicable evidence and policy:

- [detailed design workstream framing](detailed_design_workstream_framing.md)
- [current code ground map](current_code_groundmap.md)
- [architecture-wide impact assessment](impact_assessment/impact_assessment.md)
- [runtime lifecycle assessment](runtime_initialization_lifecycle/runtime_initialization_lifecycle_assessment.md)
- [producer-consumer connectivity review](runtime_initialization_lifecycle/reviews/producer_consumer_connectivity_review.md)
- [canonical World Model architecture](../../../cognitive_architecture/world_model/README.md)
- [Graph and Traversal specification](../../../cognitive_architecture/world_model/graph/spec.md)
- [Planner architecture](../../../cognitive_architecture/world_model/planner/README.md)

## Maturity Envelope

Posture: `exploratory`

Obligation floor: `operational durability for incumbent records and recovery seams`

Confidence: high for current ownership and gap evidence, medium for future contract shapes

Maturity evidence:

| Dimension | Evidence | Program effect |
| --- | --- | --- |
| product proof | the target heterogeneous reconciliation loop is not implemented | limit the program to one design vertical at a time |
| consumer evidence | current Events, Traversal, Belief, Agent, and Execution consumers are real and durable | preserve their identities, cursors, idempotency, and replay obligations |
| semantic stability | canonical owner boundaries are approved, while Curation, Plan progression, and lifecycle contracts remain thin | permit detailed design inside fixed ownership seams without assuming final shapes |
| operational exposure | durable stores, restart paths, fencing, and publication outboxes exist | require restart and partial-progress reasoning in every handoff |
| observed failure evidence | current docs freshness cannot prove an already-correct README without the write pipeline | keep docs freshness as primary proof |
| persistence stakes | several current stages commit durable state before downstream delivery | forbid call-stack success or Event append from standing in for consumer visibility |
| integration depth | the complete path crosses five packages and multiple independently durable owners | require cross-deliverable Gate Acceptance |

User override: maturity envelope and `WMR-DD-01` design activation accepted on 2026-08-21

The envelope is accepted for `WMR-DD-01`. Later slices remain unauthorized.

Hard limits:

- zero source-code changes during this program mode
- zero new crates, workspace members, durable stores, services, background runtimes, or compatibility systems
- no change to Events semantic ownership
- no change to `meld-lang` semantic ownership
- no use of legacy Workflow as a canonical seam or compatibility target
- no universal schema that collapses owner products into the transition ledger
- one active detailed-design slice at a time
- later slices remain unauthorized backlog

Investigation budget:

- twelve batched inspection calls
- sixteen source files beyond named design and policy evidence
- stop and lower confidence if the budget is exhausted

Design review budget:

- one integrated architecture review owner
- one initial review pass
- one verification pass only after accepted corrections
- blockers limited to product-path incompleteness, incorrect ownership, current data or security risk, policy violation, or maturity-envelope violation

Gate Acceptance budget:

- one independent acceptance owner
- one initial acceptance pass
- one program-owner disposition of frozen violations
- one bounded remediation cycle
- one gate-verification pass

Program authority and exception authority: user

## Authorized Active Slice

None.

`WMR-DD-01`, Semantic Spine Within Owner Publication To Frozen Traversal Cut, is complete with an accepted Gate Receipt. `WMR-DD-02` remains unauthorized.

Design owner: Codex detailed-design implementor

Integrated review owner: Codex integrated architecture review lane

Gate Acceptance owner: Codex separate cross-deliverable acceptance lane

Exact write scope:

- `detailed_design/wmr_dd_01_worker_packet.md`
- `detailed_design/semantic_transition_ledger.md`
- `detailed_design/owner_publication_to_traversal_cut.md`
- `world_model_reconciliation_handoff_ledger.md`
- `delivery_gates/wmr_dg_01_owner_publication_to_frozen_cut.md`
- `reviews/wmr_dd_01_integrated_design_review_receipt.md`
- `delivery_gates/wmr_dg_01_acceptance_receipt.md`
- this program ledger
- folder `README.md`

### WMR-DD-01 product increment

Close the first complete knowledge vertical:

```text
owner observation
-> durable Event position
-> graph admission and occurrence-preserving materialization
-> bounded immutable TraversalCut
-> exact graph input suitable for later PlannerCut assembly
```

D0 is not delivered as a standalone substrate phase. Its identity, authority, position, idempotency, supersession, wait, wake, fence, and retirement vocabulary is created only as needed to close D1.

### WMR-DD-01 deliverables

- a bounded semantic transition ledger for identities and positions used by the first vertical
- an owner-publication contract family covering workspace and docs as the primary proof and dependency security as the dissimilarity proof
- a reconciled Graph and Traversal detailed design that preserves relation occurrence identity, owner scope, explicit presence, currentness policy, provenance, and cut completeness
- an explicit downstream boundary that distinguishes the completed `TraversalCut` from later `PlannerCut` assembly and the older `WorldModelView` vocabulary
- the active entries in the [handoff and lifecycle ledger](world_model_reconciliation_handoff_ledger.md)
- direct proof traces for already-correct docs, missing docs, and dependency security inputs
- Gate Acceptance evidence for [WMR-DG-01](delivery_gates/wmr_dg_01_owner_publication_to_frozen_cut.md)

### WMR-DD-01 owned change scope

Design artifacts only under this architecture folder and narrowly required canonical World Model design documents if an accepted reconciliation requires them.

No source-code path is authorized. Likely later implementation paths are evidence anchors, not write scope.

### WMR-DD-01 existing seams

- `DomainObjectRef`, `EventRelation`, and `EventEnvelope`
- idempotent Event append and bounded replay
- `TraversalFactRecord`, durable graph cursor, and derived outbox
- workspace snapshot and node observation Events
- current graph query and Planner projection boundaries
- canonical owner-issued graph publication and `TraversalCut` architecture

### WMR-DD-01 dependencies

- approved ownership boundaries in the canonical cognitive architecture
- frozen five-package affected set
- current code and lifecycle evidence maps dated 2026-08-20 through 2026-08-21
- no D2 through D6 contract may be treated as already implemented

### WMR-DD-01 stop conditions

- a required solution moves semantic meaning into Events, root lifecycle, or adapters
- the cut cannot preserve owner revision, occurrence identity, scope, and completeness without a new cross-domain protocol
- a later Curation or Strategy consumer must be implemented to prove the producer design
- the design needs source changes, migration detail, or a new storage authority
- the affected package set or canonical ownership changes
- the slice exceeds its Gate Definition coherence horizon

### WMR-DD-01 closeout evidence

- every active handoff edge has a producer product, durable producer position, consumer acceptance position, idempotency rule, visibility milestone, wait, wake, fence, and restart source
- deferred consumers are explicitly marked and cannot create false readiness or quiescence
- the two docs freshness traces and dependency security trace reach an exact immutable `TraversalCut`
- integrated design review passes over the exact design candidate
- `WMR-DG-01` produces an accepted Gate Acceptance Receipt

## Uncommitted Backlog

| Slice | Design vertical | Dependency | Activation evidence | Authorization |
| --- | --- | --- | --- | --- |
| `WMR-DD-02` | Epistemic Authorship And Settlement Loop | accepted `WMR-DG-01` receipt | exact `TraversalCut` and owner-publication contracts available to Curation and Belief | not authorized |
| `WMR-DD-03` | PlannerCut Assembly, Heterogeneous Plan Construction, And Agent Progression | `WMR-DD-01` plus constructible-operation and settlement portions of `WMR-DD-02` | Planner receives exact Graph, Belief, Causation, Regime, directive, Capability catalog, scope, authority, and projection-policy revisions; Strategy receives the resulting cut plus constructible Task and Epistemic Operation catalogs; Agent receives Plan and admitted owner-result products | not authorized |
| `WMR-DD-04` | Executable Admission And Observation Return | accepted `WMR-DD-03` receipt | complete Task identity, cardinality, authorization, and Plan lineage settled | not authorized |
| `WMR-DD-05` | Product Compilation And Agent Genesis | accepted native-owner contracts from `WMR-DD-02` through `WMR-DD-04` | PDS compilation targets have owner-stable meanings | not authorized |
| `WMR-DD-06` | Activation Generation And Lifecycle Closure | accepted `WMR-DD-01` through `WMR-DD-05` receipts | exact participants, products, positions, waits, wakes, and safe points exist | not authorized |
| `WMR-DD-07` | Product Proof And Inspection | accepted prior receipts | complete docs freshness and dependency security specifications can be replayed through lifecycle | not authorized |

## Product Trace

The canonical complete trace is:

```text
principal declaration
-> PDS compilation and owner installation
-> Agent genesis and activation generation
-> owner observation
-> Events
-> Graph and Traversal
-> configured Belief and exact causal and regime revisions
-> immutable PlannerCut assembly
-> Curation or Strategy
-> Agent Plan authority and progression
-> Curation or Execution
-> independently visible result milestones
-> Agent reconciliation
-> fenced quiescence or successor work
-> safe retirement
```

The active proposed slice covers only owner observation through an immutable `TraversalCut`. Earlier product declaration and later Belief settlement, complete `PlannerCut` assembly, cognition, realization, and lifecycle remain visible dependencies rather than active design scope.

## Affected Domains

The frozen affected package set is:

- `meld`
- `meld-events`
- `meld-execution`
- `meld-lang`
- `meld-world-model`

Runtime participation does not imply write scope.

The active proposed slice affects these design owners:

| Domain concern | Owner | Active relationship | Design posture |
| --- | --- | --- | --- |
| workspace and docs observation | root product owners | publish exact addressable observations under a source cut | extend existing owner publication design |
| dependency security observation | dependency security | pressure the publication contract with dissimilar records | reconcile current partial publication design |
| durable carriage | Events | assign durable sequence and replay owner material neutrally | reuse unchanged |
| graph admission and materialization | World State Graph | validate routes and preserve occurrences, provenance, scope, and owner revisions | reconcile and extend current design |
| bounded traversal | Traversal | return exact occurrence-rich results against one cut | reconcile current code gap with canonical design |
| graph input to reasoning cut | Planner | accept one exact `TraversalCut` without claiming a complete `PlannerCut` | defer full assembly to `WMR-DD-03` |
| future epistemic consumer | Curation | consume a bounded cut without owning its assembly | relationship only, consumer detail deferred to `WMR-DD-02` |
| belief settlement input | Belief | consume exact owner publications through installed mappings | settlement closure belongs to `WMR-DD-02` |
| complete reasoning cut assembly | Planner | bind graph, belief, causal, regime, directive, Capability catalog, scope, authority, and projection policy revisions | close in `WMR-DD-03` before Strategy construction |

## Expansion Decisions

| Candidate expansion | Current behavior unblocked | Existing approach | Current consumers | Authority | Disposition |
| --- | --- | --- | --- | --- | --- |
| Curation detailed contract and eventual implementation | later epistemic authorship | canonical Curation ownership is approved and runtime behavior is absent | Agent and Strategy | architecture approved, `WMR-DD-02` activation and later implementation authority required | detailed design backlog under `WMR-DD-02`, not architectural expansion |
| new graph storage authority | none proven | current graph store and federated design may support the contract | current Traversal and Planner | not authorized | reject unless `WMR-DD-01` proves necessity |
| new Events grammar | none | owner payload and object or relation attachments already exist | all current Event consumers | outside envelope | reject |
| new shared language Plan grammar | none | World Model can own Plan over unchanged language values | Strategy and Execution | outside envelope | reject |
| new cross-domain lifecycle protocol | lifecycle closure may later require explicit owner receipts | current lifecycle records and owner products are partial | runtime owners | requires `WMR-DD-06` and user approval | backlog, no early design freeze |
| compatibility path for legacy Workflow | no canonical behavior | explicit isolation already exists | legacy callers only | outside program | reject |

## Hard Limits And Tripwires

The hard limits are the limits recorded in the maturity envelope.

Mandatory pause tripwires:

- any source-code edit
- any new top-level domain beyond the frozen affected set
- any proposal to move product semantics into Events, root runtime, lifecycle, harness, CLI, or telemetry
- any proposal to change `meld-lang` for heterogeneous Plan ownership
- any standalone D0 artifact that becomes a universal model rather than a bounded seam account
- any active-slice requirement that depends on completing Curation, Strategy, Agent progression, Execution intake, PDS, or lifecycle design
- any gate criterion that treats a deferred consumer as operationally ready
- any attempt to claim a complete `PlannerCut` before Belief, Causation, Regime, directive, and Capability catalog revisions have explicit assembly contracts
- any new crate, durable store, service, background runtime, protocol, migration system, or compatibility system

Changed source files tripwire: zero

Added source lines tripwire: zero

Design artifact size is governed by semantic closure and Gate Acceptance rather than an arbitrary line target.

## Phase Inventory

| Phase | Product increment | Dependency | Status | Proof |
| --- | --- | --- | --- | --- |
| program design | approval-ready ledger, handoff tracker, and first Gate Definition | existing evidence maps | complete | [integrated program design review receipt](reviews/world_model_reconciliation_program_design_review_receipt.md) |
| `WMR-DD-01` | semantic spine inside publication-to-TraversalCut vertical | program approval | complete | [accepted WMR-DG-01 receipt](delivery_gates/wmr_dg_01_acceptance_receipt.md) |
| `WMR-DD-02` | Curation operation and settlement vertical | `WMR-DD-01` | backlog | provisional `WMR-DG-02` |
| `WMR-DD-03` | PlannerCut assembly, Strategy Plan, and Agent progression vertical | `WMR-DD-01` and `WMR-DD-02` | backlog | provisional `WMR-DG-03` |
| `WMR-DD-04` | Agent Task to Execution and observation return | `WMR-DD-03` | backlog | provisional `WMR-DG-04` |
| `WMR-DD-05` | PDS compilation and Agent genesis | `WMR-DD-02` through `WMR-DD-04` | backlog | provisional `WMR-DG-05` |
| `WMR-DD-06` | activation generation and lifecycle closure | prior native-owner designs | backlog | provisional `WMR-DG-06` |
| `WMR-DD-07` | integrated product proof and inspection | all prior receipts | backlog | provisional `WMR-DG-07` |

## Work State

| Slice | Branch or worktree | Owner | Status | Commit | Notes |
| --- | --- | --- | --- | --- | --- |
| program design | `design/world-model-reconciliation` | program designer and integrated review owner | complete | pending delivery commit | included with the first accepted design gate |
| `WMR-DD-01` | `design/world-model-reconciliation` | Codex detailed-design implementor | complete | pending delivery commit | Gate accepted |

The worktree already contained the user-owned README modification and untracked detailed-design framing from the preceding discovery work. This program preserves them as current evidence and does not treat them as unrelated implementation changes.

## Delivery Gate Definitions

| Gate | Revision | Slice | Coherence horizon | Owner | Status |
| --- | --- | --- | --- | --- | --- |
| [WMR-DG-01](delivery_gates/wmr_dg_01_owner_publication_to_frozen_cut.md) | 3 frozen | `WMR-DD-01` | owner observation through immutable `TraversalCut`, plus declared deferred-consumer edges | separate Gate Acceptance owner | accepted |
| `WMR-DG-02` | provisional | `WMR-DD-02` | Curation request through independently visible result | unassigned | backlog |
| `WMR-DG-03` | provisional | `WMR-DD-03` | exact source revisions through complete `PlannerCut`, Goal, and durable Plan progression | unassigned | backlog |
| `WMR-DG-04` | provisional | `WMR-DD-04` | eligible complete Task through returned owner observation | unassigned | backlog |
| `WMR-DG-05` | provisional | `WMR-DD-05` | principal declaration through exact inert activation inputs | unassigned | backlog |
| `WMR-DG-06` | provisional | `WMR-DD-06` | prepared activation through current or safely retired generation | unassigned | backlog |
| `WMR-DG-07` | provisional | `WMR-DD-07` | complete product and lifecycle proof | unassigned | backlog |

Later gates name only a coherence horizon and activation condition. Their criteria remain provisional until their slice is proposed.

## Gate Evidence

| Gate | Command or observation | Result | Date | Notes |
| --- | --- | --- | --- | --- |
| program-design structure | delivery-program ledger validation | passed | 2026-08-21 | required headings and state model present |
| `WMR-DG-01` evidence ground | current code ground map and targeted seam inspection | available | 2026-08-21 | Event, graph, Traversal, workspace, and lifecycle anchors confirmed |
| `WMR-DG-01` acceptance | candidate digest `41c8dbff632906f9863647a677f5e65d406196cdc1e24bb0b0ab177f6f805888` | accepted | 2026-08-21 | fourteen criteria passed with no violation |

## Gate Acceptance

| Gate | Candidate | Verdict | Violations | Receipt | Notes |
| --- | --- | --- | --- | --- | --- |
| `WMR-DG-01` | `41c8dbff632906f9863647a677f5e65d406196cdc1e24bb0b0ab177f6f805888` | `accepted` | none | [Gate Acceptance Receipt](delivery_gates/wmr_dg_01_acceptance_receipt.md) | handoff eligible, next slice unauthorized |

Last accepted gate definition: `WMR-DG-01` revision 3 frozen

Acceptance state: accepted

The Gate Receipt makes `WMR-DD-01` handoff eligible. It does not authorize `WMR-DD-02`.

## Complexity Delta

Program-design delta:

- source files changed: zero
- new crates or dependencies: zero
- new public contracts: zero
- new stores or schemas: zero
- new background runtimes: zero
- design artifacts added: the workstream framing, this ledger, the handoff ledger, one proposed Gate Definition, and one integrated review receipt
- product behavior proved: none, design evidence only

Active-slice delivery delta:

- new design artifacts: five
- new active-slice artifact lines: 558
- existing program artifacts updated: Gate Definition, handoff ledger, program ledger, and folder index
- source files changed: zero
- dependencies or workspace members added: zero
- public runtime contracts added: zero
- stores or schemas added: zero
- background runtimes added: zero
- direct runtime behavior proved: none
- direct design behavior proved: owner observation through immutable `TraversalCut`

## Commit Effects

Program commit rule: every accepted detailed-design gate closes through one delivery commit before the next slice is activated. A rejected or not-eligible gate does not produce a delivery commit.

If applied, this commit adds the approved World Model Reconciliation detailed-design program, closes `WMR-DD-01` at an immutable `TraversalCut`, records its integrated review and accepted Gate Receipt, and leaves `WMR-DD-02` through `WMR-DD-07` unauthorized.

## Review State

Review owner: Codex integrated architecture review

Review receipt: [World Model Reconciliation program design review receipt](reviews/world_model_reconciliation_program_design_review_receipt.md)

Initial findings: completed with four accepted findings

Frozen finding set:

- `WMR-PROGRAM-F01` accepted, `WMR-DD-01` closed a complete `PlannerCut` before all canonical source revisions had owner contracts
- `WMR-PROGRAM-F02` accepted, the program deferred first proof of returned Curation and Execution result visibility to final product inspection
- `WMR-PROGRAM-F03` accepted, program review completion had no named reviewer, exact candidate identity, or durable receipt
- `WMR-PROGRAM-F04` accepted, the expansion table misclassified approved Curation ownership as an unapproved architecture expansion

Verification: passed for the frozen finding set with no correction-caused regression

Review budget: one initial pass and one verification pass after accepted corrections

### WMR-DD-01 Review

Review owner: Codex integrated architecture review lane

Review receipt: [WMR-DD-01 integrated design review](reviews/wmr_dd_01_integrated_design_review_receipt.md)

Candidate digest: `f2efca7d2269e39322cdeac0aa854875259f371995c43c92e7de7ef381315c78`

Frozen finding set: empty

Verification: not invoked because no correction was required

## Risks And Exceptions

| Risk | Current control | Escalation condition |
| --- | --- | --- |
| isolated correctness, coherence failure | handoff ledger plus predefined cross-deliverable gates | any phase cannot name exact producer and consumer positions |
| D0 becomes a universal schema | embed D0 vocabulary inside `WMR-DD-01` | later domains are forced to adopt one shared runtime record |
| future consumer pulled into an earlier slice | deferred-consumer state and coherence horizon | Gate Acceptance requires later implementation or later detailed design |
| false readiness or quiescence | lifecycle columns require explicit wait, wake, fence, and restart status | a missing consumer is reported as ready or quiet |
| gate becomes an unbounded review | frozen criteria, one remediation cycle, and separate program-owner disposition | acceptance owner proposes redesign or expands scope |
| current code shapes overconstrain design | code is evidence for implemented behavior, canonical architecture owns intended boundaries | a current type is treated as the required future contract without semantic proof |
| architecture prose outruns product proof | docs freshness and dependency security pressure every slice | a design cannot complete either declared trace |

Authorized exceptions: none

## Reassessment

Reassess the envelope only after a completed design vertical produces new ownership evidence, a real implementation consumer appears, an observed runtime failure changes the obligation, or the user changes scope.

Before activating `WMR-DD-02`, record:

- accepted [WMR-DG-01 receipt](delivery_gates/wmr_dg_01_acceptance_receipt.md)
- exact downstream preconditions established by `WMR-DD-01`: owner publication identity and completeness, Event and Graph positions, occurrence-rich traversal, deterministic cuts, owner-correct hydration, and deferred Curation and Belief acceptance positions
- whether the exploratory posture still limits solution breadth
- whether any proposed Curation storage or runtime authority is an architectural expansion
- explicit user authorization

## Final Reconciliation

The authorized `WMR-DD-01` slice is complete and the program is awaiting authorization for any later slice.

Closeout evidence:

- ledger validation passed
- the [integrated program design review receipt](reviews/world_model_reconciliation_program_design_review_receipt.md) freezes the exact candidate and records one bounded verification pass
- readiness and unresolved decisions are recorded for user disposition
- `WMR-DD-01` passed integrated design review and `WMR-DG-01` Gate Acceptance
- `WMR-DD-02` through `WMR-DD-07` remain unauthorized backlog

Source implementation has not started and is not authorized by this ledger.
