# Startup PDS Full Design Specification

Date: 2026-08-23

Status: user-approved design included in the World Model Reconciliation revision 3 candidate

Authority: product requirements for proposed `WMR-DG-07` revision 3, no implementation authority

Implementation authorization: none

## Problem Statement

Meld can report that processes are running while the reconciliation round trip is disconnected. Local Event, Graph, Belief, Agent, Execution, and lifecycle components may each be healthy without one real product proving that a Goal can arise from admitted knowledge, become an executable Task, produce a durable world effect, return through epistemics, and satisfy the originating Agent.

Docs Freshness and Dependency Security can expose that failure, but their product semantics make first diagnosis unnecessarily difficult. A missing README might implicate source observation, docs claim extraction, planning, file mutation, or verification. Startup needs a product whose only semantic effect is one exact Event round trip.

## Thesis

The Startup PDS is one ordinary one-Agent stewardship product. For each open activation admission epoch, it creates one deterministic startup nonce whose maintained condition is satisfied only when the Agent has durably incepted the nonce Goal and later accepted exact evidence that the owner-issued nonce Event returned through the configured epistemic route.

The canary proves one bounded path. It does not establish universal runtime health.

## Canonical Design Vocabulary

| Concept | Canonical design name | Meaning |
| --- | --- | --- |
| PDS product | `meld_startup` | first-party system stewardship product |
| reusable semantic owner | `nonce` | owner of the generic nonce publication and Capability contract |
| topology position | `startup_agent` | one declared Agent position |
| maintained condition | `startup_nonce_observed` | exact Goal inception plus returned Event acceptance for one startup nonce |
| standing operation | `assess_startup_nonce` | authors the startup expectation and bounded realization assessment |
| planned operation | `confirm_startup_nonce` | confirms the exact returned nonce under a successor cut |
| executable Capability | `nonce.emit.v1` | appends one exact owner-issued nonce Event |
| Event kind | `nonce` | reusable durable nonce publication |
| Goal kind | `observe_startup_nonce` | Agent-owned desired outcome for one startup nonce |
| inspection view | `startup_nonce_account` | read-only correlation of native positions |

These names specify semantic roles. They do not select Rust type names, module paths, schema encodings, or storage keys.

## Generalization Boundary

The Startup product keeps the names that express its actual responsibility. `meld_startup`, `startup_agent`, `startup_nonce_observed`, `assess_startup_nonce`, `confirm_startup_nonce`, `observe_startup_nonce`, and `startup_nonce_account` are intentionally product-specific.

The executable primitive is reusable. The global contract is `nonce.emit.v1`, its owner is `nonce`, and its Event kind is `nonce`. None of those contracts mentions Startup, an Agent, a Goal, an activation generation, or runtime health. Those meanings enter only through the exact generic references selected by the calling Task and interpreted by Startup theory.

No other new reusable abstraction is justified. PDS, Agent, Curation, Strategy, Execution, Events, Graph, Traversal, Belief, lifecycle, and inspection already provide the common architecture.

## Claims The Product May Make

A satisfied nonce proves that the exact product generation and admission epoch carried one bounded obligation through these native positions:

```text
Agent maintained condition
-> standing Curation mismatch
-> Agent Goal
-> Strategy Plan
-> Agent Task authorization
-> Execution admission and unified Task Network
-> global nonce Capability
-> neutral Event append
-> Graph and Traversal visibility
-> Agent-authorized confirmation Curation
-> configured Belief revision
-> Agent milestone acceptance
-> Goal satisfaction
```

It does not prove that every participant is healthy, every Event consumer is caught up, every Capability works, all product semantics are correct, all accepted work is complete, the generation is quiescent, or a later epoch still works.

Operational health, structural readiness, startup nonce satisfaction, and global quiescence remain four distinct claims.

## Product Definition

The PDS product declaration selects one exact Startup package set and one topology position. The assignment binds the Startup Agent to the Meld runtime subject, principal, perspective, branch, requested authority, and grant lineage. Activation binds the selected `nonce.emit.v1` implementation and the structural participants required by the round trip.

The product package routes stable components to native owners.

| Component | Native owner | Required meaning |
| --- | --- | --- |
| startup subject vocabulary | Agent and Curation | identify runtime, generation, admission epoch, and the expected nonce under Startup theory |
| nonce publication contract | nonce | define deterministic owner publication and hydration independent from Startup |
| emitter Capability contract | Capability and nonce | accept exact reusable nonce inputs and append only Event kind `nonce` |
| graph publication route | Graph admission | admit the nonce owner batch without interpreting it |
| standing Curation rule | Curation | establish expected Event and bounded non-realization |
| planned Curation operation contract | Curation | confirm exact Event realization under a successor cut |
| evidence mapping and comparator | Belief | settle expectation, non-realization, and realization under exact lineage |
| directive and maintained condition | Agent | create and judge the nonce Goal |
| construction and verification theory | Strategy | construct one Task and one confirmation operation |
| satisfaction contract | Agent | require exact Goal inception and returned milestone acceptance |
| presentation metadata | inspection | label native positions without creating truth |

The structural package carries component identities, routes, imports, requirements, and content. Each native owner validates and installs its own component. PDS does not interpret the bodies.

## Agent Topology And Authority

The product declares exactly one Startup Agent topology position. The Agent identity remains stable for the assignment lineage. A changed subject, perspective, branch, principal, grant, topology meaning, or product revision produces the successor identity required by the accepted product and genesis design.

The minimal authority grant permits only these effects:

- authorize the installed `nonce.emit.v1` Capability
- target the exact nonce subject and open admission epoch named by the Task
- append one deterministic Event in the nonce namespace
- authorize the installed bounded confirmation operation

The grant does not authorize generic Event fabrication, arbitrary graph authorship, arbitrary runtime commands, file mutation, provider access, or lifecycle transitions.

## Nonce Scope And Identity

There is one logical startup nonce instance per exact open admission epoch. Its Agent-owned identity binds:

- Startup product revision
- complete product compilation receipt
- situated assignment
- Startup Agent identity
- activation generation
- open admission epoch
- nonce contract revision

Wall-clock time, process identifier, thread identifier, actor poll order, retry count, Event sequence, Graph cursor, and diagnostic values are excluded from nonce identity.

The admission epoch is required. A participant interruption closes the old epoch. Reopening the same generation under a successor epoch creates a successor nonce and reruns the reconciliation round trip against the reconstructed participant set.

The startup nonce instance deterministically derives the Goal identity and one generic nonce publication request. The request sets issuer to the Startup Agent, subject to the Meld runtime, correlation references to the product, assignment, generation, epoch, and deterministic Goal identity, and fence reference to the admission epoch.

The nonce owner derives the expected Event identity from only that generic request and its contract revision. The Event identity exists before the Event record. The Goal identity exists before Agent inception. This lets Curation author an expectation without inventing an observed Event and lets every retry converge on one Event record.

## Required Semantic Identities

| Identity | Owner | Required binding |
| --- | --- | --- |
| product revision | PDS product model | exact Startup product declaration |
| compilation receipt | PDS structural compilation | complete selected package set and owner receipts |
| assignment | PDS assignment | product, principal, Agent, runtime subject, perspective, branch, grants |
| generation | root lifecycle | prepared closure and predecessor lineage |
| admission epoch | root lifecycle | exact current generation and participant incarnation set |
| startup nonce instance | Agent | product, assignment, Agent, generation, epoch, and Startup maintained-condition revision |
| reusable nonce publication | nonce | issuer, subject, ordered correlation references, fence reference, and nonce contract revision |
| expected Event | nonce | reusable nonce publication identity and Event contract revision |
| standing assessment | Curation | Agent specification, nonce, source cut, rule revision, generation, epoch |
| Goal | Agent | nonce, maintained condition, directive lineage, generation, epoch |
| PlannerCut | Planner | exact native owner revisions and decision scope |
| StrategyPlan | Strategy | Goal, cut, catalogs, policies, predecessor when present |
| Task | Strategy | exact emitter contract, inputs, effect, authority requirements, expected milestones |
| confirmation operation | Strategy and Curation | nonce, expected Event, successor cut, bounds, vocabulary, authority |
| Task authorization | Agent | Task, Goal, Plan, authority, generation, epoch, idempotency |
| Execution admission | Execution | offered Task and consumer decision |
| Event record | Events | deterministic owner publication batch and ledger position |
| realization assessment | Curation | exact Event record, cut, rule, perspective, generation, epoch |
| Belief revision | Belief | admitted evidence, route, comparator, key, predecessor |
| milestone acceptance | Agent | Plan dependency, Belief revision, nonce, Goal, generation, epoch |
| satisfaction receipt | Agent | Goal inception position and returned milestone acceptance |

Every identity is immutable. A changed semantic input produces a successor identity. A retry of equal input returns the prior owner decision or record.

## Two-Barrier Startup

Structural startup and cognitive proof are ordered but not equivalent.

```text
prepare exact Startup product
-> realize declared participants
-> collect native readiness
-> publish current generation and open admission epoch
-> derive epoch nonce
-> run cognitive round trip
-> publish Agent satisfaction receipt
```

The Startup Agent, Curation, Planner inputs, Strategy contract, Execution intake, Task Network, nonce implementation, Event authority, Graph projection, Traversal query, configured Belief, and required transports must be structurally ready before the epoch opens.

Nonce completion cannot be a readiness requirement for the generation that must run it. Operator diagnostics, Event access, lifecycle inspection, and recovery controls remain available if the nonce does not complete.

## Initial Standing Curation

After the epoch opens, Agent instantiates the maintained condition and exposes the deterministic nonce specification to its installed standing Curation rule.

The standing operation receives an immutable `TraversalCut` that includes the declared nonce owner policy, exact Event coverage boundary, Graph projection position, generation, epoch, perspective, and required completeness references.

The operation may produce these results:

| Result | Meaning |
| --- | --- |
| `applied` with expectation and non-realization | expected Event is authored and complete bounded evidence does not contain its owner publication |
| `unchanged` with expectation and non-realization | the exact Curation-owned products already exist under the same cut |
| `applied` or `unchanged` with realization | the Event already exists and the prophecy is already fulfilled for this epoch |
| `incomplete` | Event coverage, Graph projection, required owner route, or hydration is insufficient |
| `conflicted` | identity, currentness, perspective, branch, generation, or epoch disagrees |
| `failed` | accepted operation could not complete under its contract |

Missing storage alone never proves non-realization. The negative assessment requires a complete Event coverage boundary and Graph projection through that boundary under the selected cut.

If the Event is already realized, Agent may satisfy the maintained condition without creating a duplicate Goal. This is required for restart after Event publication but before final Agent acceptance.

## Goal Inception

When admitted Curation or Belief evidence establishes the exact expected Event and bounded non-realization, Agent records one deterministic Goal.

The Goal has two desired conditions:

```text
Goal G was durably incepted by Agent A for nonce N
Agent A accepted exact realization evidence for Event E under epoch U
```

The first condition is proved by the Agent-owned Goal record. It does not require graph publication. The second cannot be true before the Event returns through the declared epistemic path.

Agent may refuse or hold Goal inception when the directive, authority, generation, epoch, source cut, or maintained-condition revision is stale or conflicted. No startup coordinator may create the Goal on Agent's behalf.

## PlannerCut And Strategy Plan

Planner assembles one complete cut for the Goal. It includes the expected Event and bounded non-realization evidence, exact nonce contract revision, current Capability catalog, Agent directive and authority, activation generation and epoch, Curation operation catalog, Belief route, causal and Regime inputs required by policy, and projection policy.

Strategy constructs one immutable heterogeneous Plan with this causal shape:

```text
complete Task to emit Event E
-> Event E Graph visibility milestone
-> bounded confirmation Epistemic Operation
-> configured nonce-realization Belief revision
-> Agent milestone acceptance
-> separate Goal satisfaction judgment
```

The Task and confirmation operation are independently complete products. Strategy does not execute either product, does not target the Task Network, does not append the Event, and does not authorize work.

The Plan may contain no Task only when admitted current evidence already proves the exact Event realization for the current epoch. A historical nonce from an earlier epoch cannot satisfy the Goal.

## Task Contract

The Task binds the exact nonce, expected Event, Startup Agent, Goal, Plan revision, nonce subject, ordered correlation references, fence reference, generation, admission epoch, `nonce.emit.v1` Capability contract, selected effect target, authority requirement, Event publication policy, and idempotency identity.

Its one semantic effect is:

```text
append the exact owner-issued nonce Event E
```

The Task does not request health assessment, collect arbitrary diagnostics, modify runtime state, mutate files, change lifecycle, or signal Agent directly.

Version one requires no diagnostic payload. A later product may attach exact immutable diagnostic references that were already selected as Task inputs. Mutable snapshots, timestamps, free-form process dumps, and best-effort diagnostics cannot be inserted under the same Event identity. Rich diagnostics may be published separately by their native owners.

## Reusable Nonce Capability

The nonce domain owns the Event schema, typed publication batch, deterministic record identity, and `nonce.emit.v1` Capability contract. The implementation is a narrow adapter to Event append authority.

The contract is deliberately independent from Startup. Its request contains one nonce identity, one issuer reference, one subject reference, an ordered bounded set of correlation references, one fence reference, and the exact contract revision. Startup supplies Agent, Goal, generation, and admission epoch through those generic references. Another Strategy may use the same Capability with different semantic references without creating another contract.

The caller cannot choose the Event type, source owner, object kind, or relation grammar. The owner always publishes Event kind `nonce` under the nonce contract. This preserves reuse without exposing arbitrary Event fabrication.

The Capability validates only its owner contract:

- nonce and expected Event identity match the generic content derivation
- issuer, subject, correlation references, fence reference, and contract revision are complete
- Task authority and execution fence match the accepted Task
- Event kind is `nonce` and source owner is nonce
- payload contains no undeclared semantic or diagnostic fields

The implementation appends idempotently with the deterministic Event record identity. Event authority returns the same accepted record position on retry. No generic append-arbitrary-Event Capability is exposed.

The complete Task body is an authoritative reconstruction source for the one-record publication. A process loss after append but before provider return is reconciled by looking up or retrying the same Event record identity. A second record is forbidden.

The Capability result returns the exact append receipt or a typed rejected, conflicted, unavailable, or uncertain result. Execution records its own attempt and outcome independently.

## Execution Boundary

Agent authorizes the complete Task only after a fresh PlannerCut check and ordinary product eligibility. Execution receives the Task through the Goal Set and validates only consumer-owned shape, authority, current Capability contract, binding, generation, epoch, idempotency, and Task Network constraints.

Execution does not know what a Startup PDS is. It does not validate the nonce proposition, decide whether the Event proves health, construct a confirmation operation, inspect Belief, or satisfy the Goal.

The Task enters the one unified Task Network. Duplicate admissions for the same exact nonce may share an operational node only under the accepted compatibility contract. Each admission retains independent attribution.

The nonce Event may become durable before Execution records a terminal outcome. Agent success follows the declared epistemic milestone, not callback order. Execution must still classify its attempt and operation before its own quiescence or safe point can close.

## Event Publication And Graph Admission

The owner-issued nonce publication carries:

- nonce identity
- issuer reference
- subject reference
- ordered correlation references
- fence reference
- nonce owner and contract revision

The neutral Event envelope and Execution provenance may additionally reference:

- expected Event and typed publication identity
- Task authorization, Plan, and Execution operation lineage
- exact Event ledger and append positions

These additional references do not change the generic nonce publication identity.

Events validates neutral ledger identity, producer authority, record identity, and envelope shape. It does not interpret Goal satisfaction or startup meaning.

Graph admission uses the installed nonce owner route. It validates owner revision, occurrence identity, scope, fence, and typed publication against neutral envelope hints. Graph preserves the nonce Event object plus its subject, correlation, and fence references. Startup Curation interprets the selected references as Agent, Goal, generation, and epoch under installed theory. Graph does not assert that Agent received the Event.

Traversal returns the Event owner publication under an immutable cut with exact Event coverage and Graph projection positions. Event append alone cannot substitute for this visibility.

## Planned Confirmation Curation

Agent authorizes `confirm_startup_nonce` only after absorbing the Event Graph visibility milestone declared by the Plan.

The operation consumes the exact successor `TraversalCut`, nonce specification, expected Event, owner-hydrated Event publication, standing expectation, Goal, perspective, branch, generation, epoch, and rule revision.

It may author only Curation-owned products such as expected-event realization, required relation satisfaction, and coverage assessment. It cannot create the Event, reinterpret Execution outcome as Event presence, or claim Agent receipt.

The operation terminates `applied`, `unchanged`, `incomplete`, `conflicted`, `failed`, or preadmission `rejected` under the accepted Curation grammar. Its semantic publication reaches Events, Graph, and configured Belief independently.

## Belief And Agent Closure

The installed Belief route admits the exact confirmation result or its owner-hydrated realization product for the nonce key and Startup Agent perspective. The immutable revision cites the Event record, TraversalCut, Curation result, route, comparator, generation, epoch, and predecessor.

Agent absorbs the revision only when it matches the exact Plan dependency, nonce, Goal, perspective, branch, generation, and admission epoch. Agent then records a separate milestone acceptance.

Goal satisfaction requires both the original Agent Goal inception position and the exact returned milestone acceptance. The resulting satisfaction receipt is the authoritative startup nonce completion evidence.

The receipt does not close outstanding Execution uncertainty, participant waits, lifecycle safe points, or global quiescence.

## Work, Wait, And Wake Semantics

Every participant publishes native work or a complete wait.

| Position | Required wait | Required wake |
| --- | --- | --- |
| product not compiled | exact missing owner receipt or package closure | owner installation successor |
| generation not current | prepared or ready generation under closed admission | exact current-head and open-epoch publication |
| standing cut incomplete | missing Event coverage, owner route, or Graph projection | exact owner receipt or projection successor |
| Goal held | stale directive, authority, generation, epoch, or mismatch | exact successor decision input |
| Planner refusal | named missing or conflicting source | exact source or policy successor |
| Task authorization pending | unmet Plan dependency or failed currentness check | exact milestone or successor cut |
| Execution unavailable | admission, binding, network, claim, or provider position | exact Execution or binding successor |
| Event not visible | append absent or Graph cursor behind | exact Event record or projection advancement |
| confirmation pending | missing visibility or authorization | exact milestone or Agent authorization |
| Belief unsettled | route, evidence, key, or predecessor missing | exact route or evidence successor |
| Agent receipt pending | exact milestone absent or stale | matching Belief revision or successor Plan |
| nonce complete | no eligible work for this nonce | successor admission epoch or explicit product upgrade |

Polling and heartbeat may reduce latency. Neither is wake evidence. A deadline wakes owner evaluation but does not manufacture failure truth, create a new nonce, or authorize duplicate work.

## Retry, Recovery, And Successors

Equal nonce inputs under the same admission epoch always resolve the same nonce, Goal, expected Event, and owner publication identities. Retries reuse existing owner decisions.

If Event append succeeded but provider return was lost, Execution reconciles the exact Event record identity. Curation and Agent may continue from durable Event evidence while Execution separately closes its uncertain operation.

If the process restarts without an admission-epoch change, native stores and cursors resume the same nonce. If lifecycle replaces a participant incarnation, it closes admission and opens a successor epoch only after readiness. That epoch creates a successor nonce.

A changed product revision, assignment, Agent identity, nonce contract, maintained condition, Curation rule, Belief route, Strategy policy, Capability contract, generation, or epoch creates the appropriate successor products and preserves all historical positions.

A failed standing or planned Curation operation is terminal under its identity. Retry requires the accepted successor trigger. A failed or conflicted Task does not permit a second Event identity for the same nonce.

## Inspection Contract

The `startup_nonce_account` is a specialization of the accepted read-only inspection projection. One request names assignment, generation, admission epoch, Startup Agent, nonce, perspective, branch, and temporal boundary.

It resolves these positions without changing them:

```text
product compiled
Agent genesis complete
generation current
admission epoch open
nonce instantiated
standing assessment terminal
Goal incepted
PlannerCut complete
Plan admitted
Task authorized
Execution admission accepted
Task Network node attributed
attempt and external operation recorded
nonce Event durable
nonce Event Graph visible
confirmation operation accepted and terminal
nonce Belief settled
Agent milestone accepted
Goal satisfied
```

The projection shows every resolved owner identity, the deepest position on the success-critical path, the first missing or conflicted successor, and unresolved side obligations such as an uncertain Execution outcome. It may therefore show nonce satisfied and Execution unresolved at the same time.

Evidence states remain `available`, `incomplete`, `stale`, `conflicted`, `unavailable`, and `unproved`. Presentation may summarize the bounded result as nonce verified, nonce pending, nonce stalled, or nonce conflicted. It must not label the whole runtime healthy or unhealthy from this receipt alone.

## Optional Dependent-Product Gate

The core Startup PDS is observational and non-gating. It produces an Agent-owned satisfaction receipt suitable for later policy use.

A separately selected fail-closed policy may require that a dependent PDS activation request cite a current Startup nonce satisfaction receipt for the exact bootstrap generation and admission epoch. Compilation and inert preparation of dependent products may proceed before that receipt. Their live activation request remains withheld.

PDS product control owns the dependency declaration and request policy. Agent owns the satisfaction receipt. Root lifecycle validates only the structural reference supplied with an activation intent and never interprets the nonce meaning.

The bootstrap assignment can never depend on its own nonce. Diagnostic, Event, inspection, lifecycle, and recovery surfaces remain ungated.

This policy is not part of the core nonce requirements and is not selected by this document. Selecting it changes the accepted statement that PDS leaves the live path after preparation and therefore requires explicit architectural amendment.

## Design Requirements

### Product And Ownership

| Requirement | Binding statement |
| --- | --- |
| `SPDS-R001` | the Startup PDS shall be an ordinary PDS product compiled through native owner routes |
| `SPDS-R002` | the product shall declare exactly one Startup Agent topology position in version one |
| `SPDS-R003` | PDS shall own product structure and lineage but no situated nonce truth |
| `SPDS-R004` | the reusable nonce owner shall own the nonce publication and Event semantics |
| `SPDS-R005` | Events, Graph, Execution, root lifecycle, and inspection shall remain semantically neutral to nonce satisfaction |
| `SPDS-R006` | no Startup PDS requirement shall change `meld-lang` grammar enforcement |
| `SPDS-R007` | the global nonce Capability shall contain no Startup, Agent, Goal, product, or lifecycle meaning in its contract grammar |
| `SPDS-R008` | Startup theory shall bind its specific Agent, Goal, generation, and epoch meaning through generic nonce references |

### Identity And Fencing

| Requirement | Binding statement |
| --- | --- |
| `SPDS-R010` | exactly one logical nonce shall exist per open admission epoch |
| `SPDS-R011` | the startup nonce instance shall bind product, compilation, assignment, Agent, generation, epoch, and maintained-condition revision |
| `SPDS-R012` | expected Event and Goal identities shall be deterministic before Event append and Goal inception |
| `SPDS-R013` | retries under equal inputs shall reuse owner identities and decisions |
| `SPDS-R014` | a successor admission epoch shall create a successor nonce even within the same generation |
| `SPDS-R015` | historical nonce evidence shall never satisfy a successor epoch |
| `SPDS-R016` | generic nonce publication identity shall bind issuer, subject, ordered correlations, fence, and nonce contract revision only |
| `SPDS-R017` | the reusable Capability shall always publish Event kind `nonce` under the nonce owner |

### Epistemic Inception And Planning

| Requirement | Binding statement |
| --- | --- |
| `SPDS-R020` | standing Curation shall establish expectation and non-realization only under a complete bounded cut |
| `SPDS-R021` | Agent alone shall incept the Goal from admitted mismatch evidence |
| `SPDS-R022` | Planner shall assemble the complete frozen construction context |
| `SPDS-R023` | Strategy shall construct one immutable Plan containing a complete emitter Task and a complete confirmation operation |
| `SPDS-R024` | Agent shall judge the Plan and authorize each eligible product separately |
| `SPDS-R025` | Strategy shall remain ignorant of Task Network realization |

### Execution And Event Effect

| Requirement | Binding statement |
| --- | --- |
| `SPDS-R030` | Execution shall receive only the complete authorized Task |
| `SPDS-R031` | the Task shall enter the unified Task Network through ordinary Goal Set admission |
| `SPDS-R032` | the emitter Capability shall have one semantic effect, append the exact nonce Event |
| `SPDS-R033` | the Capability shall not expose generic arbitrary Event emission |
| `SPDS-R034` | the Event record identity shall be deterministic and idempotent across uncertain provider return |
| `SPDS-R035` | Execution outcome and nonce Event durability shall remain independent positions |

### Return And Satisfaction

| Requirement | Binding statement |
| --- | --- |
| `SPDS-R040` | Graph shall admit the owner-issued publication through a declared nonce route |
| `SPDS-R041` | Event append shall not substitute for Graph visibility |
| `SPDS-R042` | Agent shall authorize confirmation only after the Plan-declared visibility milestone |
| `SPDS-R043` | Curation shall confirm realization without claiming Agent receipt |
| `SPDS-R044` | Belief shall admit confirmation only through the installed route and exact lineage |
| `SPDS-R045` | Agent shall record a distinct returned milestone acceptance |
| `SPDS-R046` | Goal satisfaction shall cite both Goal inception and returned milestone acceptance |

### Lifecycle And Operations

| Requirement | Binding statement |
| --- | --- |
| `SPDS-R050` | structural readiness and current publication shall precede nonce eligibility |
| `SPDS-R051` | nonce completion shall never be a readiness prerequisite for its own bootstrap epoch |
| `SPDS-R052` | each owner shall expose durable work, wait, wake, fence, and restart positions |
| `SPDS-R053` | deadlines shall trigger evaluation but shall not create semantic truth or fresh authority |
| `SPDS-R054` | participant replacement shall close admission before a successor incarnation enters work |
| `SPDS-R055` | global quiescence and safe retirement shall remain lifecycle aggregates beyond nonce satisfaction |

### Inspection And Product Policy

| Requirement | Binding statement |
| --- | --- |
| `SPDS-R060` | inspection shall correlate native positions without writing owner state |
| `SPDS-R061` | inspection shall expose the first missing successor and unresolved parallel obligations |
| `SPDS-R062` | no nonce projection shall claim universal runtime health |
| `SPDS-R063` | version one shall work without diagnostic attachments |
| `SPDS-R064` | a dependent-product gate shall require separate policy selection and approval |
| `SPDS-R065` | operator and recovery surfaces shall remain available when the nonce stalls |

## Required Proof Scenarios

| Scenario | Required observable result |
| --- | --- |
| fresh epoch | one nonce progresses from standing mismatch through Agent satisfaction |
| Event already durable after restart | standing Curation observes realization and Agent closes without duplicate Event or Goal work |
| crash before Event append | same Task and Event identity remain eligible after restart |
| crash after Event append before provider return | one Event exists, epistemic return may proceed, Execution separately reconciles uncertainty |
| Graph lag | Event is durable while nonce remains unsatisfied and inspection names Graph projection as next position |
| Belief route absent | Graph visibility remains true while settlement is unavailable and Agent does not satisfy |
| confirmation rejection | Task effect remains historical, Goal remains unresolved, and no false satisfaction appears |
| stale epoch Task | Execution rejects intake or effect under the closed epoch and no successor Event is forged |
| participant replacement | old epoch is fenced and successor epoch creates a new nonce |
| duplicate Task admission | one idempotent Event record results and every admission retains attribution |
| Event without matching Goal lineage | Curation or Agent rejects the mismatch and the nonce remains unsatisfied |
| satisfied nonce with uncertain Execution callback | Agent satisfaction and unresolved Execution operation are both visible |
| whole-runtime idle after nonce | quiescence appears only after every realized participant supplies complete waits and viable wakes |

## Non-Requirements

The Startup PDS does not require a new Event ledger, universal graph ontology, graph database, dedicated scheduler, Task Network, generic command bus, model provider call, prompt, filesystem mutation, health scoring engine, startup coordinator, lifecycle store, or second inspection authority.

It does not require edits to Docs Freshness, Dependency Security, legacy Workflow, `meld-lang`, Event envelope grammar, generic Execution planning semantics, or Agent authority semantics.

## Design Completion Finding

The full design closes without changing the seven accepted World Model Reconciliation owner contracts. It specializes them with one new first-party product and one very small product-semantic owner.

The non-gating canary therefore challenges only final product proof and implementation delivery sequencing. The optional dependent-product gate is the sole refinement that materially changes an accepted architectural statement.
