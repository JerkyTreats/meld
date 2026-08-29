# WMR-VC-02 Standing Curation And Settlement Delivery Gate

Date: 2026-08-28

Gate identifier: `WMR-VC-02-DG`

Revision: frozen revision 1

Status: accepted

Source baseline: `eabc9a75`

Activation record: [WMR-VC-02 Source Activation](wmr_vc_02_source_activation_record.md)

Implementation review: [passed](../reviews/wmr_vc_02_implementation_review_receipt.md)

Style Assurance: [satisfied](../reviews/wmr_vc_02_style_assurance_receipt.md)

Gate Acceptance: [accepted](wmr_vc_02_gate_acceptance_receipt.md)

Intended handoff: one standing Curation result visible through Graph and selectively settled by Belief

Gate owner: primary integrated acceptance lane

Exception authority: user

## Coherence Horizon

The horizon begins with one exact Agent authority, one installed standing Curation rule, and one complete immutable workspace cut. It closes when the real root runtime persists one terminal Curation result, appends deterministic result and semantic publication Events, exposes the Curation-owned material through the existing Graph owner route, and commits one distinct Belief revision when the installed evidence mapping selects the result.

The horizon includes standing selection, admission, terminality, publication retry, Graph visibility, configured Belief settlement, waiting, reopen, and replay.

It excludes planned Curation, Agent result acceptance, Goal formation changes, Goal satisfaction changes, complete Planner input, Strategy, Execution, PDS, lifecycle aggregation, Startup, and product migration.

## Required Deliverables

| Deliverable | Owner | Required product |
| --- | --- | --- |
| initiating authority | Agent | exact Agent, perspective, branch, activation generation, subject, and installed rule revision |
| source cut | Graph and Traversal | complete immutable workspace cut and bounded occurrence-rich result |
| acceptance | Curation | durable admit or reject decision under one exact fence |
| terminal operation | Curation | exactly one terminal result identity for each admitted operation |
| semantic authorship | Curation | owner-correct expected entity or assessment under installed vocabulary |
| publication outbox | Curation | retryable deterministic result and semantic publication Events |
| durable carriage | Events | unchanged producer-neutral append, replay, and ledger identity |
| graph visibility | Graph | Curation owner revision selected through the existing exact cut route |
| configured settlement | Belief | selected result maps to distinct evidence and immutable Belief revision before cursor advance |
| root participation | root runtime | one concrete bounded Curation actor with truthful work and waiting reports |
| incumbent preservation | Agent and products | Goal formation, satisfaction, planned work, and product migrations remain unchanged |

## Producer And Consumer Edges

| Edge | Producer | Consumer | Claim |
| --- | --- | --- | --- |
| `WMR-VC-02-E01` | Agent and theory | Curation | initiating authority and rule revision are exact and durable |
| `WMR-VC-02-E02` | Graph and Traversal | Curation | accepted input is one complete immutable cut and bounded result |
| `WMR-VC-02-E03` | Curation acceptance | Curation execution | only admitted work can reach an accepted-operation terminal result |
| `WMR-VC-02-E04` | Curation result | Events | terminal and semantic publications are deterministic and retryable |
| `WMR-VC-02-E05` | Events | Graph | Curation owner publication projects through the incumbent Graph runtime |
| `WMR-VC-02-E06` | Events | Belief | only an installed mapping selects the terminal result as evidence |
| `WMR-VC-02-E07` | Belief ingestion | Belief revision | domain state commits before its durable consumer cursor advances |
| `WMR-VC-02-E08` | Curation and root runtime | restart | persisted positions reconstruct pending publication without repeating semantic work |

## Lifecycle Claims

Activation requires a resolved Agent authority, installed Curation rule, durable Curation store, Event append port, Event replay position, and exact Traversal query seam.

Visibility requires every semantic publication Event named by the terminal result to be covered by the Graph projection position. Event append alone is insufficient.

Waiting names the exact missing rule, incomplete cut, unchanged source selection, pending Event append, pending Graph position, or pending configured Belief position. Silence and an empty query are never terminal evidence.

Waking occurs only from a changed rule revision, changed declared input, Graph advancement that can complete the named cut, Event append recovery, or Belief cursor advancement. A deadline may re-evaluate eligibility but cannot create a successor operation.

Quiescence is local. Every accepted Curation operation is terminal, every required publication has an append receipt, and no standing selection is eligible through the observed rule and source positions. It does not prove Graph, Belief, Agent, or activation-wide quiescence.

Restart reloads durable selection, acceptance, terminal result, publication receipts, and observed source position. The same accepted operation reuses its result and Event identities.

Fencing binds Agent, perspective, branch, activation generation, rule revision, subject, source cut, bounds, vocabulary, and publication policy.

## Criteria

| Criterion | Required claim | Blocking condition |
| --- | --- | --- |
| `WMR-VC-02-DG-C01` | the real root runtime composes one concrete standing Curation actor | source is reachable only through tests or an optional adapter |
| `WMR-VC-02-DG-C02` | current Agent Goal and satisfaction actors remain canonical and behaviorally unchanged | Curation authorship replaces or enters Agent Goal judgment |
| `WMR-VC-02-DG-C03` | standing selection binds exact Agent, perspective, branch, generation, rule, subject, source cut, and bounds | ambient state, process order, or wall time enters identity |
| `WMR-VC-02-DG-C04` | Curation records one durable admission or preadmission rejection before semantic execution | rejected work can publish semantic effects |
| `WMR-VC-02-DG-C05` | every admitted operation reaches exactly one valid terminal class | silence, timeout, or empty traversal is treated as completion |
| `WMR-VC-02-DG-C06` | `incomplete` names exact frontier, exclusions, failures, and cut | bounded exhaustion implies absence or correctness |
| `WMR-VC-02-DG-C07` | `failed` remains terminal and only changed input or changed rule revision creates a successor | replay or deadline silently retries the failed identity |
| `WMR-VC-02-DG-C08` | Curation authors only installed Curation vocabulary and retains perspective, lineage, provenance, and owner currentness | Curation asserts workspace presence, Belief confidence, or another owner meaning |
| `WMR-VC-02-DG-C09` | Curation commits terminal state and publication intent before reporting completion | process memory or best-effort append is the sole recovery source |
| `WMR-VC-02-DG-C10` | result and semantic Events use deterministic producer identities through the existing Event authority | a second ledger, writer, or Event semantic validator appears |
| `WMR-VC-02-DG-C11` | Graph projects Curation semantic products only through `world_state.owner_publication.v1` | Curation writes Graph storage directly or creates another graph authority |
| `WMR-VC-02-DG-C12` | one exact workspace plus Curation cut exposes every required owner revision and occurrence | Event durability or bare reachability substitutes for the exact cut |
| `WMR-VC-02-DG-C13` | configured Belief mapping alone decides whether the Curation result becomes evidence | every Curation result automatically becomes Belief state |
| `WMR-VC-02-DG-C14` | Belief evidence and revision commit before the existing consumer cursor advances | cursor movement can pass uncommitted domain state |
| `WMR-VC-02-DG-C15` | reopen and replay reuse operation, result, Event, evidence, and Belief identities | restart repeats semantic work or creates duplicate revisions |
| `WMR-VC-02-DG-C16` | an unmapped result becomes Graph-visible and remains absent from Belief | Graph visibility is treated as configured settlement |
| `WMR-VC-02-DG-C17` | the exact candidate passes direct proof, implementation review, Style Assurance, and Gate Acceptance | any required receipt is absent |
| `WMR-VC-02-DG-C18` | source remains within the approved expansion, write scope, and tripwires | planned Curation, downstream behavior, or unauthorized architecture enters the candidate |

## Direct Product Proof

Proof must run through `ProductRuntimeAssembly`, the real Event authority, the existing Graph runtime, the existing Belief evidence ingestion actor, and the existing branch owner query.

It must demonstrate:

- one complete workspace cut consumed by a standing Curation rule
- one `applied` result with Curation-owned semantic material
- one successor operation over a changed declared input that returns `unchanged` and cites existing semantic material
- replay of the earlier `applied` operation returning the same `applied` result identity without repeating semantic work
- one explicit incomplete cut that cannot be admitted as complete input
- one preadmission rejection with no semantic publication
- one accepted failure whose identity is not retried by replay or deadline
- deterministic Event append and restart recovery after terminal persistence
- Curation Graph visibility only after projection catch-up
- one configured Curation result producing a distinct Belief revision
- one unmapped Curation result remaining absent from Belief
- reopen identity parity across Curation, Event, Graph, evidence, and Belief positions
- unchanged behavior for Agent Goal formation and satisfaction
- a dissimilar owner-neutral specimen proving no docs-specific grammar entered Curation substrate

## Acceptable Evidence

- focused Curation contract, persistence, actor, publication, and replay tests
- exact Graph and Belief integration tests over real stores and Event authority
- one real root runtime assembly and bounded tick trace
- property proof for operation identity under normalized input order
- state-machine or fuzz proof for acceptance, terminal, publication, and restart transitions when crate precedent supports it
- full workspace regression, formatting, strict changed-crate lint, and diff validation
- implementation review and Style Assurance receipts for one exact candidate

## Forbidden Substitutions

A complete cut cannot substitute for Curation admission. A terminal result cannot substitute for Event durability. Event durability cannot substitute for Graph visibility. Graph visibility cannot substitute for configured Belief settlement. Belief settlement cannot substitute for Agent acceptance or Goal satisfaction. A test-only constructor cannot substitute for root runtime composition. An existing Agent curation decision cannot substitute for a Curation operation or result.

## Blocking Standard

Any failed criterion blocks Gate Acceptance. The gate has no waiver authority. A user-authorized exception must name scope, rationale, authority, and retirement condition.

## Acceptance Budget

One initial Gate Acceptance pass and one verification pass after one bounded remediation cycle.

The user authorized implementation on 2026-08-28. Revision 1 was frozen before source edits and accepted against candidate digest `ba37bbe63575f8316373adaaaf318767514d00a103f3fe80440d0f17b3383cdb`. Acceptance establishes handoff eligibility only and does not authorize planned Curation or any later vertical.
