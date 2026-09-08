# World Model Knowledge Traversal And Epistemic Curation Assessment

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-08-20

Status: discovery assessment, implementation not authorized

## Finding

A common traversal substrate is justified inside world-model, but only as a discovery and materialization capability.

The supported architecture has two deliberately separate domains.

Traversal is the graph read substrate. In a loose architectural sense, it is the knowledge graph. It materializes promoted nodes and edges and exposes bounded, provenance-preserving reads:

```text
owner-admitted semantic product
-> promoted Event with object and relation attachments
-> generic traversal materialization
-> bounded walk with occurrence provenance
-> typed owner hydration
-> Agent or Strategy decision input
```

Epistemic Curation is a separate world-model author operating under Agent authority. It uses Traversal and publishes new Curation-owned nodes and edges:

```text
Agent specification plus admitted knowledge
-> Curation
-> curation-owned epistemic entities and connections
-> promoted Events
-> Traversal materialization
```

Strategy should not become the graph curator. Strategy authors the causal plan and may construct both closed Tasks and bounded epistemic-operation requests as distinct plan products. Curation executes the epistemic operations and authors their graph results. This preserves Curation as a sibling world-model domain operating for an Agent rather than hiding it inside Task construction.

The concise architecture rule is:

> Traversal reads the knowledge graph. Curation authors Agent-scoped epistemic connections. Strategy plans against the resulting world-state mismatch.

## Concern And Scope

The concern is whether world-model can use one shared traversal surface to discover the unknown intermediate knowledge needed for documentation and other epistemic questions, and whether a separate entity operating under Agent authority may organize or extend that knowledge without involving Execution.

In scope:

- common graph indexing of promoted objects and relations
- bounded discovery of unknown intermediate identities
- occurrence provenance and source hydration
- pure epistemic questions and derivations
- a distinct world-model Curation domain operating under Agent authority
- curation-owned expected entities and epistemic connections
- proposals only when Curation needs to assert meaning owned elsewhere
- deterministic owner derivation from admitted facts
- documentation expectations, claims, materiality, coverage, and realization

Out of scope:

- Execution Planning
- Task Network design
- Capability scheduling or dispatch
- universal product-wide graph admission
- treating every durable record as a graph node
- one ontology or one semantic owner
- implementation sequencing and migration planning

Caller limits preserved:

- this concern is related to Strategy but is not necessarily Strategy
- the question is pure epistemic curation
- Execution and Task Network are irrelevant to the ownership decision
- the common substrate must improve contextual knowledge without absorbing domain meaning
- Curation is graph authorship while Traversal remains graph materialization and read

The discovery budget was fixed at twelve batched source inspections and was exhausted. Confidence is high for current code boundaries and moderate for the final epistemic proposal shape.

## Current Code Verdict

World-model already has a real common traversal read substrate:

- `TraversalStore` indexes graph-readable facts, objects, relation edges, anchors, and lineage
- `TraversalQuery` exposes neighbor lookup, bounded walks, facts, current anchors, and provenance
- `WorldModelQueries` catches the graph projection up to Events before serving reads
- `DomainObjectRef` and `EventRelation` provide producer-authored structural attachments
- graph writes are replay materialization over Events rather than arbitrary direct mutation

It is incomplete as a common knowledge substrate:

- graph replay accepts only `workspace_fs`, `context`, and `execution` source domains
- anchor intent extraction is hardcoded to four event types
- `GraphWalkResult` returns bare relations and loses the identity of each relation occurrence
- `current_only` has special semantics only for `selected` relations
- planner projection reduces graph knowledge mainly to accessibility, anchor ids, and source fact ids
- Agent curation emits Goal and Goal-lifecycle decisions only
- no public epistemic proposal or owner-admission port exists
- documentation claim assessments remain Task artifacts and are not promoted as graph-addressable owner products

Strategy cannot curate graph knowledge today. Agent cannot delegate to a distinct Curation domain because that domain does not exist. Belief can curate belief revisions through its own evidence and comparator pipeline, but that is belief-specific settlement rather than general Agent-directed epistemic authorship.

## Why Traversal Helps

Traversal solves a discovery problem, not a truth problem.

The starting subject rarely names every relevant intermediate object. A folder Goal does not already contain the identities of its expected documentation slot, current README revision, source claims, README claims, verification reports, evidence assignments, and belief revisions.

A bounded relation walk can discover those identities without a use-case-specific orchestration chain. Typed hydration can then recover each owner's authoritative body.

This is materially different from a bespoke query DTO when several dissimilar domains need unknown intermediate discovery. The common value is:

- one bounded neighbor and walk contract
- one object coordinate form
- one relation attachment form
- one provenance path back to promoted facts
- one way to freeze the discovered cut for later reasoning

Traversal does not determine whether an item is true, material, current, correct, supported, or relevant. Those meanings remain owner products.

## Necessity Test

A common traversal substrate is not necessary merely because the data can be drawn as a graph.

A typed owner query is cheaper and clearer when the caller already knows the relevant object identities, one owner can answer the question, and the joins are stable. Traversal becomes materially useful when all of these conditions hold:

- the starting subject does not name the relevant intermediate entities
- the discovery path crosses multiple semantic owners
- relation shape influences which evidence is relevant
- the answer must retain a bounded, replayable explanation of how the context was reached

The documentation case meets this test only after docs publishes the semantic products that traversal cannot invent. A folder seed does not name its expected README slot, current revision, source claims, coverage assessments, or verification products. Those entities cross workspace, docs, and potentially belief ownership. Their paths affect the answer, and a correctness judgment must cite the exact snapshot and claim coverage used.

Without promoted docs products, a more capable walk still cannot answer the question. With those products, bespoke orchestration would repeatedly reconstruct the same cross-owner discovery and provenance behavior. That is the point at which the common substrate becomes materially superior.

## The Missing README Question

The statement that a README must exist before it can be correct contains two different epistemic entities.

First, Curation applies the Agent specification and its installed documentation rule to a material folder. It authors an expected README entity and a requirement edge. The expected entity now exists epistemically even though no workspace file realizes it.

Second, the workspace may or may not contain an observed README that realizes the expected entity in one complete snapshot.

No missing edge can prove absence. A safe knowledge shape is:

```text
material folder
-> requires
expected README F

workspace snapshot
-> covers
material folder

README revision
-> realizes
expected README F

bounded realization assessment
-> assesses
expected README F
-> under
workspace snapshot
```

The expected README and an observed README must have different identities. Curation owns the expected entity. Workspace owns the observed file entity. This prevents a normative expectation from falsely asserting that the physical file already exists.

When no observed README realizes the expectation, the durable product is a bounded non-realization assessment citing complete scan scope, snapshot identity, exclusions, and failures. Curation may author that Agent-scoped assessment from the expected entity and the admitted workspace scan. It must not infer absence from lack of a `realizes` edge.

This is pure epistemic curation. No Goal, Task, or action is involved. Once the mismatch is available to world-model, Strategy has an explicit planning premise: expected README F is unrealized.

## The Source Claim Promotion Question

The question about which source-file claims must be promoted into a README is another Curation rule applied under an Agent specification.

Useful owner products are:

```text
source revision
-> contains
source claim

source claim
-> material to
README correctness obligation

README revision
-> contains
README claim

verification
-> covers
source claim
-> against
README revision
```

Traversal can discover candidate claims and coverage paths. It cannot author materiality or coverage edges.

Current docs code already has durable-shaped `ReadmeClaim`, `ClaimAssessment`, `ReadmeClaimReport`, citations, exact content hashes, and claim-policy identity. Those products currently remain inside execution artifacts. They lack owner-issued graph coordinates and promoted Event publication.

The first missing bridge is promotion of independently addressable source claims, README claims, and verification products. Extraction owners retain authority over those observed products.

The Curation domain then applies the Agent specification to author edges such as `material to`, `required in`, and `covered by`. Those relations are Curation-owned epistemic judgments with exact Agent perspective, policy, source snapshot, and provenance. Traversal indexes them without interpreting them.

Curation can then answer a pure epistemic question such as:

```text
Which material source claims lack accepted coverage
in the current README revision under this exact snapshot
```

The answer remains a perspective-scoped Curation product. It does not require a Goal or Task unless obtaining new evidence requires an action.

## Traversal And Curation Boundary

Traversal and Curation must not share authorship responsibility.

| Concern | Owner | Output |
| --- | --- | --- |
| structural graph materialization | Traversal | indexes over promoted Events |
| graph reads | Traversal | bounded nodes, edge occurrences, and provenance |
| Agent specification application | Curation | expected entities and epistemic connections |
| perspective-scoped assessment | Curation | realization, relevance, materiality, and coverage judgments |
| observed workspace state | workspace | observed files, revisions, and scan coverage |
| extracted documentation products | extraction owner | addressable claims and verification records |
| evidence settlement | belief | evidence assignments, revisions, and views |
| append and replay | Events | promoted immutable records |

Curation is therefore not a Traversal behavior. It is an Event producer and a Traversal consumer. It may author accepted graph knowledge within its own vocabulary and Agent scope.

When Curation needs to make a claim in another domain's vocabulary, it emits a proposal to that owner. That exception does not reduce Curation to proposal routing.

## Strategy Versus Epistemic Curation

Strategy and epistemic curation may share reasoning techniques but do not own the same product.

| Concern | Strategy | Curation |
| --- | --- | --- |
| input | Goal, frozen epistemic snapshot, Capability snapshot, and constructible epistemic-operation contracts | Agent specification or bounded epistemic-operation request, admitted facts, and Curation rules |
| output | causal plan containing independently closed Task and epistemic-operation products | expected entities, epistemic edges, assessments, and answers |
| authority | explains which causal events are needed and why | authors knowledge under one Agent perspective |
| accepted truth | never writes it | writes Curation-owned knowledge under an Agent perspective and proposes foreign-owner assertions |
| Execution relationship | only authorized Task products cross the Goal Set seam | none for epistemic operations |

Strategy may construct a bounded epistemic-operation request or ask for a deeper frozen projection. It should not execute the operation or materialize the answer as accepted graph truth.

If the question can be answered from admitted facts and installed Curation rules, Curation can derive and promote the result without Execution.

If answering requires a new observation or external computation, the Agent may separately form a Goal and ask Strategy for a Task. That is a later transition, not part of pure curation.

## Pass One Domain Sweep

The domain universe was regenerated from current top-level files in `crates/meld-world-model/src`.

```text
agent
belief
lib
planner
strategy
waiting
world_state
```

There is no current `curation` top-level domain. That absence is the main ownership gap exposed by this assessment.

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `own` | Owns perspective, subscriptions, durable Goal curation decisions, and satisfaction curation | `partial` | [Agent curation](../../../../crates/meld-world-model/src/agent/curation.rs), [Agent contracts](../../../../crates/meld-world-model/src/agent/contracts.rs) | not applicable | Supply the specification and perspective for a separate Curation domain |
| `belief` | `consume` | Owns evidence normalization, assignment, assessment, revisions, and belief views | `complete` for this concern | [belief contracts](../../../../crates/meld-world-model/src/belief/contracts.rs), [promoted evidence ingestion](../../../../crates/meld-world-model/src/belief/ingestion.rs) | Curation may consume belief views, but no Belief write is proven | none |
| `lib` | `adapter` | Re-exports graph, belief, planner, Agent, and Strategy surfaces | `partial` | [crate surface](../../../../crates/meld-world-model/src/lib.rs) | not applicable | Export only explicit traversal and epistemic curation contracts |
| `planner` | `consume` | Reads belief and graph, but reduces graph scope to accessibility and source handles | `partial` | [planner query](../../../../crates/meld-world-model/src/planner/query.rs), [planner contracts](../../../../crates/meld-world-model/src/planner/contracts.rs) | not applicable | Freeze relation-rich knowledge cuts for Strategy without absorbing Curation |
| `strategy` | `consume` | Consumes a flat planner product and does not write graph knowledge | `partial` | [Strategy contracts](../../../../crates/meld-world-model/src/strategy/contracts.rs), [Strategy design](../../../cognitive_architecture/world_model/strategy/README.md) | not applicable | Consume Curation-authored mismatches without becoming a graph writer |
| `waiting` | `none` | Provides observational waiting declarations for bounded actors | `not needed` | [waiting declarations](../../../../crates/meld-world-model/src/waiting.rs) | A future curation actor may reuse the pattern, but waiting semantics do not decide the substrate or curation authority | none |
| `world_state` | `own` | Owns event-backed graph materialization, current anchors, provenance, relation indexes, and bounded traversal | `partial` | [graph reducer](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs), [graph query](../../../../crates/meld-world-model/src/world_state/graph/query.rs), [graph store](../../../../crates/meld-world-model/src/world_state/graph/store.rs) | not applicable | Generalize promoted graph-attachment indexing while preserving owner admission |

## Frozen Affected-Domain Set

The affected world-model domains are:

```text
agent
curation
lib
planner
strategy
world_state
```

`belief` and `waiting` are explicitly excluded from required write scope. `curation` is included as a missing target domain rather than an implemented domain.

## Pass Two Affected-Domain Decomposition

### World State And Traversal

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| generic relation indexing | Traversal | reducer indexes object and relation attachments only for three hardcoded source domains | index every explicitly graph-attached promoted Event admitted to the traversal contract | `extend existing` | treating every Event as graph knowledge | [source filter](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs) |
| anchor derivation | Traversal with source-specific rules | four event types produce anchor intents through hardcoded extraction | keep anchor semantics separate from generic relation indexing | `extend existing` | a generic graph reducer may guess currentness | [source intent](../../../../crates/meld-world-model/src/world_state/graph/source_intent.rs) |
| occurrence provenance | Traversal | stored relation records retain fact id and sequence, but walk returns bare relations | return relation occurrence identity and source fact handles in curation-grade walks | `extend existing` | bare edges lose authority, scope, and currentness | [relation record and walk](../../../../crates/meld-world-model/src/world_state/graph/store.rs) |
| currentness | semantic owner and typed projection | `current_only` is meaningful only for `selected` edges | require owner-shaped currentness or hydrate owner records | `new local behavior` | graph may universalize one lifecycle model | [relation visibility](../../../../crates/meld-world-model/src/world_state/graph/store.rs) |
| write boundary | Events and semantic producers | graph writes only by reducing Events and emits derived anchor Events | preserve event-backed materialization and reject arbitrary mutation of Traversal storage | `reuse unchanged` | direct store mutation would bypass provenance and replay | [graph runtime](../../../../crates/meld-world-model/src/world_state/graph/runtime.rs) |

### Agent

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| perspective and authority | Agent | durable perspective, subject, branch, directive, and curation-rule identity | bind every epistemic question and answer to exact Agent scope | `reuse unchanged` | unscoped answers may masquerade as shared truth | [Agent record](../../../../crates/meld-world-model/src/agent/contracts.rs) |
| Goal curation | Agent | pure threshold and satisfaction rules emit Goal commands or lifecycle mutations | remain separate from epistemic Curation | `reuse unchanged` | every unresolved question could become a Goal |
| Curation delegation | Agent | no separate epistemic Curation contract exists | provide exact specification, perspective, subject, branch, and policy identity | `new local behavior` | Curation may become detached from the Agent whose world model it serves |
| mismatch consumption | Agent | satisfaction curation already reacts to world-state relationships | allow Curation-authored mismatches to inform Goal need | `extend existing` | Curation findings may be mistaken for actions |

### Curation

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| domain existence | Curation | no top-level domain exists | become a distinct world-model epistemic domain operating under Agent authority | `new domain` | folding it into Agent or Strategy obscures authorship |
| traversal consumption | Curation | no common consumer exists | read bounded nodes, edge occurrences, provenance, and typed products | `new local behavior` | live or unbounded reads make results unreplayable |
| expected entities | Curation | no owner exists for expected but unrealized objects | author stable Curation-owned identities such as expected README F | `new local behavior` | using workspace identity would conflate expectation with observation |
| epistemic connections | Curation | no general Agent-scoped edge author exists | apply installed rules to author requirement, materiality, coverage, and realization assessments | `new local behavior` | relation vocabulary may silently claim another owner's authority |
| publication | Curation through Events | no Curation events exist | publish deterministic, perspective-scoped products for Traversal materialization | `new local behavior` | replay loops and duplicate derivations require stable provenance and identity |
| foreign vocabulary | target semantic owner | no general proposal port exists | propose only assertions whose meaning belongs to another domain | `new local behavior` | Curation may overreach its owned vocabulary |

### Planner

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| traversal scope | Planner | reads anchors only for the subject | perform bounded relation-scoped discovery from Strategy roots | `extend existing` | unbounded walks make relevance implicit | [planner query](../../../../crates/meld-world-model/src/planner/query.rs) |
| typed hydration | Planner with semantic owners | output carries generic source and hydration refs | combine reached structure with owner-shaped records | `new local behavior` | flattening typed owner meaning into graph decoration |
| frozen cut | Planner | output has one projection version but little topology | freeze roots, walk specification, occurrences, typed products, source cut, and completeness | `extend existing` | live reads during Strategy construction break replay |
| consumer shaping | Planner | one Strategy-oriented `WorldState` projection | shape the Strategy view while Curation remains a direct Traversal consumer or uses a neutral query facade | `extend existing` | one universal DTO may become a hidden ontology |

### Strategy

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| context consumption | Strategy | consumes flat `WorldState` and source handles | consume the relation-rich frozen view needed for Task Construction | `extend existing` | Strategy may query live graph or settle truth itself |
| missing question | Strategy | can report construction failures but has no epistemic question product | emit a typed unresolved knowledge need to Agent | `new local behavior` | a question may be treated as an accepted assertion |
| graph mutation | semantic owners | Strategy has no graph write contract | remain absent | `not needed` | Strategy writing truth would let it satisfy its own premises |

### Crate Surface

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| traversal reads | world-model crate | exports anchors, facts, walks, and query facades | export curation-grade occurrence-preserving walks | `extend existing` | exposing raw stores bypasses domain contracts | [crate exports](../../../../crates/meld-world-model/src/lib.rs) |
| curation contract | Curation | no domain surface exists | export Curation rules, expected entities, epistemic connections, assessments, and publication ports | `new local behavior` | a generic edge API may erase Curation vocabulary |

## External Publication Boundary

The common substrate does not eliminate producer work. It makes producer participation uniform.

| External owner | Current integration | Required relationship | Verdict |
| --- | --- | --- | --- |
| Events | already carries objects, relations, record identity, sequence, and source records | transport accepted owner products intact | reuse unchanged |
| workspace | publishes source, snapshot, node, containment, and observation relations | continue publishing structural workspace facts | reuse unchanged |
| context | publishes frame, head, attachment, derivation, selection, and supersession relations | continue publishing context-owned facts | reuse unchanged |
| docs | owns claim extraction, assessment, citations, policy identity, exact bytes, and publication receipts but does not publish graph knowledge | expose stable observed source claims, README claims, and verification products for Curation | proven missing observed-product publication for the docs use case |
| root composition | wires event authority and graph runtime | route ports only | adapter only |

No Events semantic change is required. The opt-in signal is the producer's deliberate graph attachment on a promoted Event, not a central registry of domain meaning.

## Capability-Type Architecture Interpretation

The capability-style analogy is valid when stated as follows:

> Any semantic owner may opt a promoted product into common traversal by issuing stable object coordinates and graph attachments that satisfy the structural participation contract. The owner retains the typed body, lifecycle, currentness, and semantic proof. Traversal owns indexing, bounded discovery, occurrence provenance, and hydration handles.

This is one canonical traversal capability with implementation participation owned by producing domains.

Curation is one such producer. Its distinctive role is to author epistemic entities and edges derived from an Agent specification rather than from direct observation.

It is not one canonical traversal domain that owns all implementations. The common part is the structural contract and query behavior. Domain-specific participation remains implemented and enforced by the domain that knows the meaning.

The current `EventEnvelope` attachment pattern is already close to this model. The main substrate defect is that traversal ignores graph-attached Events outside a hardcoded domain allowlist and returns too little occurrence information for curation-grade reasoning.

## Smallest Missing Connective Behavior

The smallest credible bridge has five pieces:

1. Traversal generically materializes explicitly graph-attached promoted Events while keeping source-specific anchor rules separate.
2. A new Curation domain consumes bounded Traversal reads and authors perspective-scoped expected entities, epistemic connections, and assessments through Events.
3. Agent supplies the specification and perspective to Curation and consumes its mismatches without turning pure epistemic work into a Goal.
4. Planner preserves enough Curation-authored topology for Strategy to plan against expected versus observed world state.
5. Strategy can compose independently closed bounded epistemic operations and Tasks into one causal plan while sending only Tasks to Execution.

The documentation proof additionally requires independently addressable observed source claims, README claims, and verification products. Curation owns the expected README, materiality, requirement, coverage, and bounded realization relationships under the Agent specification.

This is smaller than the earlier product-wide graph overhaul and larger than merely calling the existing `walk` method.

## Separated Scope

### Runtime path

```text
observational owners
-> Events
-> Traversal
-> Curation
-> expected entities and epistemic connections
-> Events
-> Traversal
-> Planner Strategy view
-> Strategy
```

### Behavior that would need to change

- traversal event eligibility and occurrence-preserving query results
- new Curation domain authorship and replay behavior
- Agent-to-Curation specification boundary
- planner preservation of curated mismatches for Strategy
- heterogeneous Strategy plan construction over bounded epistemic operations and Tasks
- observed claim and verification publication for the documentation proof

### Likely write ownership

- `meld-world-model::world_state::graph`
- `meld-world-model::curation`
- `meld-world-model::agent`
- `meld-world-model::planner`
- world-model crate exports
- root docs publication adapters only for addressable observed claim products

### Reused unchanged

- `meld-events` append and replay authority
- `DomainObjectRef` structural coordinate
- `EventRelation` structural edge
- belief evidence admission and comparator authority
- Strategy Task output contract
- Execution and Task Network
- workspace and context graph publication already present

## Explicit Non-Integration Decisions

- Execution has no role in pure epistemic curation.
- Task Network has no role in pure epistemic curation.
- Strategy does not write accepted graph truth.
- Traversal does not author documentation requirements, correctness, materiality, or coverage.
- Curation-owned perspective knowledge does not become perspective-free global truth.
- missing edges do not prove nonexistence.
- belief assignment does not become a generic graph reducer decision.
- Events does not interpret relation meaning.
- `DomainObjectRef` does not carry truth, authority, branch, time, or currentness.
- not every durable world-model record becomes a traversal node.
- no universal ontology, graph database, or producer registry is justified.

## Unresolved Questions

- the exact Curation-owned relation vocabulary and typed products
- which rules are installed by Agent specification and which are native Curation rules
- the stable identity scheme for expected but unrealized entities
- how Curation retracts or supersedes derived connections when its source specification changes
- how deterministic replay prevents derivation loops when Curation consumes the graph it helps author
- which assertions are valid Curation-owned products rather than proposals to another owner
- the minimum occurrence shape needed beyond source fact id and sequence
- how owner-targeted proposal routing resolves the semantic owner from relation vocabulary
- which source claims are stable enough for independent addressability
- how bounded non-realization records encode coverage, exclusions, failures, and snapshot identity
- whether Curation reads Traversal directly or through a neutral frozen-cut assembler

These are discovery questions. They do not reopen Execution or authorize a product-wide graph migration.

## Evidence Basis And Confidence

Primary evidence:

- [graph contracts](../../../../crates/meld-world-model/src/world_state/graph/contracts.rs)
- [graph reducer](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs)
- [graph source intent](../../../../crates/meld-world-model/src/world_state/graph/source_intent.rs)
- [graph query](../../../../crates/meld-world-model/src/world_state/graph/query.rs)
- [graph store](../../../../crates/meld-world-model/src/world_state/graph/store.rs)
- [graph runtime](../../../../crates/meld-world-model/src/world_state/graph/runtime.rs)
- [Agent curation](../../../../crates/meld-world-model/src/agent/curation.rs)
- [Agent contracts](../../../../crates/meld-world-model/src/agent/contracts.rs)
- [belief contracts](../../../../crates/meld-world-model/src/belief/contracts.rs)
- [belief evidence normalization](../../../../crates/meld-world-model/src/belief/evidence.rs)
- [planner query](../../../../crates/meld-world-model/src/planner/query.rs)
- [docs claim validation](../../../../src/docs/claim_validation.rs)
- [workspace graph publication](../../../../src/workspace/events.rs)
- [context graph publication](../../../../src/context/events.rs)
- [Current code ground map](current_code_groundmap.md)
- [bounded epistemic operations refinement](bounded_epistemic_operations.md)

Confidence is high that common traversal already exists as a partial implemented substrate and that current world-model lacks a distinct Curation author.

Confidence is high that Curation must author an expected README entity distinct from an observed workspace file for Strategy to receive a coherent mismatch.

Confidence is high that Curation can author its own graph knowledge through Events without mutating Traversal storage directly.

Confidence is moderate on whether Curation should read Traversal directly or share a neutral frozen-cut assembler with Planner. That internal placement remains open.
