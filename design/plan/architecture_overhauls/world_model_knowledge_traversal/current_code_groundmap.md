# Current Code Ground Map For Strategy, Goals, And Epistemic Flow

Date: 2026-08-20

Status: evidence-only current-state assessment

## Purpose

This document records the current repository ground for reviewing Goal language, Strategy plan construction, bounded epistemic operations, Curation, and Event-backed knowledge flow.

It does not select an architecture. It separates three kinds of statement throughout:

- implemented code
- active design text
- behavior absent from current code

The current working tree contains uncommitted architecture-document changes. Active design text is therefore evidence of the design currently being edited, not evidence of implemented runtime behavior.

## Concern And Scope

The assessed behavior begins when belief and graph state reach an Agent, follows Agent Goal curation and Strategy construction, crosses Goal admission into Execution, then follows execution outcomes through Events, Traversal, Belief, and Agent satisfaction curation. The docs freshness branch is the concrete product path.

In scope are Strategy candidates, Task-shaped construction, Agent Goal decisions, Goal Set admission, Events append and replay, Traversal reduction and query, Belief evidence integration, docs freshness implementation, and every current code or active-design use of the word curation relevant to this path.

Out of scope are architecture selection, naming recommendations, migration design, implementation sequencing, and changes to Execution or Task Network.

The evidence basis is the repository at the date above. Discovery used the caller-named domains and a bounded breadth pass over the five workspace packages, followed by one-level decomposition of the frozen affected set.

## Regenerated Domain Snapshot

The workspace declares five package-level ownership boundaries:

```text
meld
meld-events
meld-execution
meld-lang
meld-world-model
```

This snapshot comes from the current [workspace manifest](../../../../Cargo.toml).

The complete top-level `meld-world-model` domain set is:

```text
agent
belief
planner
strategy
waiting
world_state
```

This snapshot comes from the current [world-model crate surface](../../../../crates/meld-world-model/src/lib.rs).

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `meld` | `own` | Owns docs freshness capabilities, PDS theory loading, root runtime composition, and adapters between Agent commands and Execution Goal acceptance | `complete` for mapping current behavior | [docs capability](../../../../src/docs/capability.rs), [runtime Goal port](../../../../src/runtime/ports.rs), [runtime assembly](../../../../src/runtime/assembly.rs) | not applicable | none |
| `meld-events` | `own` | Owns event envelopes, graph coordinates, relations, durable append, bounded replay, and ledger identity | `complete` for mapping current behavior | [event contracts](../../../../crates/meld-events/src/events.rs), [object and relation contracts](../../../../crates/meld-events/src/events/contracts.rs), [append authority](../../../../crates/meld-events/src/events/authority.rs) | not applicable | none |
| `meld-execution` | `own` | Owns the Goal Set, operational authorization copy, Composition realization, Task compilation, Task Network mutation, dispatch, and execution event publication | `complete` for mapping current behavior | [Goal Set contracts](../../../../crates/meld-execution/src/goals/contracts.rs), [Goal Set API](../../../../crates/meld-execution/src/goals/api.rs), [planning runtime](../../../../crates/meld-execution/src/planning/runtime.rs), [lowering](../../../../crates/meld-execution/src/planning/lowering.rs) | not applicable | none |
| `meld-lang` | `own` | Owns the shared Goal, Proposition, Operator, Effect, and Composition value language consumed by Strategy and Execution | `complete` for mapping current behavior | [Goal](../../../../crates/meld-lang/src/goal.rs), [Composition](../../../../crates/meld-lang/src/composition.rs), [crate surface](../../../../crates/meld-lang/src/lib.rs) | not applicable | none |
| `meld-world-model` | `own` | Owns graph materialization, Belief, planner projection, Agent Goal curation, Strategy search, Strategy verification, and Agent satisfaction curation | `complete` for mapping current behavior | [crate surface](../../../../crates/meld-world-model/src/lib.rs), [Strategy contracts](../../../../crates/meld-world-model/src/strategy/contracts.rs), [Agent curation](../../../../crates/meld-world-model/src/agent/curation.rs), [Traversal reducer](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs) | not applicable | none |

## Frozen Affected-Domain Set

The affected package set is frozen as `meld`, `meld-events`, `meld-execution`, `meld-lang`, and `meld-world-model`.

All five are on the current path being mapped. Inclusion here does not imply that all five require change.

## Implemented End-To-End Path

The current implemented path is:

```text
Event records
-> Traversal graph projection
-> Belief evidence and revisions
-> planner projection
-> Agent Goal curation
-> Strategy executable Composition construction
-> Agent Strategy authorization
-> Execution Goal Set admission
-> Execution planning and Task Network lowering
-> Task execution events
-> Belief evidence ingestion
-> Agent Goal satisfaction curation
```

The stages are independently durable in several places. They do not form a single stored Strategy plan object.

## Current Strategy Candidate And Task Construction

### Implemented code

`StrategyProblem` contains one ground `Goal`, one immutable `WorldState`, one planner snapshot identity, one Strategy theory snapshot, executable Capability views, optional Methods, and one evaluation policy. It has no epistemic-operation catalog or heterogeneous plan state. See [StrategyProblem](../../../../crates/meld-world-model/src/strategy/contracts.rs#L86).

`StrategyCandidate` contains one `Composition`, bindings, one settlement obligation, one prospective evidence route, selected Capability identities, and an evaluation value. See [StrategyCandidate](../../../../crates/meld-world-model/src/strategy/contracts.rs#L136).

Search begins with a proposed ground Goal, unifies one settlement rule, grounds the settlement obligation, and searches Method-seeded and direct Capability candidates. See [Strategy search entry](../../../../crates/meld-world-model/src/strategy/search.rs#L12).

Direct search selects Capability-backed operators whose effects unify with the settlement obligation, then recursively closes required artifact inputs through other executable Capabilities. See [direct candidate construction](../../../../crates/meld-world-model/src/strategy/search.rs#L170).

Candidate completion rejects any step that is not `StepKind::Op`. It also requires at least one selected Capability outcome contract to equal the prospective evidence route outcome contract. See [candidate completion](../../../../crates/meld-world-model/src/strategy/search.rs#L378).

Independent verification likewise treats non-operator steps as invalid, checks current preconditions, artifact closure, settlement contribution, evidence routing, and content identity. See [candidate verification](../../../../crates/meld-world-model/src/strategy/verification.rs#L10).

The code product is a semantic `Composition`, not a `CompiledTaskRecord`. Execution later lowers each operator step into a compiled Task node. See [Execution composition lowering](../../../../crates/meld-execution/src/planning/lowering.rs).

### Canonical distinction

The active Strategy README calls Strategy the only canonical semantic planning workflow and states that successful construction returns one authorizable Task. It states that Agent publishes Goal plus Task and that Execution receives no Strategy reasoning contract. See [active Strategy design](../../../cognitive_architecture/world_model/strategy/README.md).

The active search design calls `StrategyCandidate` the sole canonical semantic product and describes its action graph as a ground Capability dependency blueprint rather than a compiled Task. See [active Strategy search design](../../../cognitive_architecture/world_model/strategy/search.md).

### Absent behavior

No implemented type named `StrategyPlan`, `EpistemicOperation`, `BoundedEpistemicOperation`, or `EpiOp` exists under `crates`, `src`, `theory`, or `tests` at this evidence cut.

No current Strategy candidate can contain distinct epistemic and executable step variants. `meld-lang::StepKind` contains only a Capability-backed operator or a recursive Goal proposition. See [StepKind](../../../../crates/meld-lang/src/composition.rs#L29).

No implemented Strategy runtime persists causal plan progression across new Events or revised world-model snapshots. Each search call is a pure bounded function over one immutable request. See [search](../../../../crates/meld-world-model/src/strategy/search.rs#L17).

## Agent Goal Curation And Goal Set Publication

### Implemented code

The current Agent record owns identity, perspective, subject, branch, directive, installed curation rule, and maintained condition. See [AgentRecord](../../../../crates/meld-world-model/src/agent/contracts.rs#L103).

The current `AgentDecisionKind` vocabulary is Goal command, Goal mutation command, absorbed, or indeterminate. See [AgentDecisionKind](../../../../crates/meld-world-model/src/agent/contracts.rs#L81).

Threshold curation reads an Agent, belief state, planner projection, active Goal summary, and an installed curation rule. It can emit a proposed Goal when the maintained condition is breached. See [threshold curation](../../../../crates/meld-world-model/src/agent/curation.rs#L245).

Satisfaction curation evaluates active or satisfied Goals against the current planner projection. It emits satisfy or reopen lifecycle mutations when the evaluated state supports those decisions. See [satisfaction curation](../../../../crates/meld-world-model/src/agent/curation.rs#L420).

When a Goal command exists, Agent runtime invokes Strategy over the exact planner projection. A successful candidate is attached to both the durable Agent decision and the Goal command as `StrategyAuthorization`. A failed or denied construction becomes an indeterminate Agent decision and the Goal command is removed. See [Agent Strategy authorization](../../../../crates/meld-world-model/src/agent/runtime.rs#L406) and [authorization assembly](../../../../crates/meld-world-model/src/agent/strategy.rs#L119).

The named `CurationGoalSetPort` accepts only Agent Goal commands and Goal mutation commands. See [Curation Goal Set port](../../../../crates/meld-world-model/src/agent/goal_port.rs).

Root runtime maps an Agent Goal command into `GoalAcceptanceRequest`, copying the authorized Composition, bindings, Capability identities, Method lineage, theory lineage, and authority decision into Execution-owned transport. See [Agent Goal acceptance adapter](../../../../src/runtime/ports.rs#L540).

Execution validates and stores the accepted Goal. A proposed Goal becomes active under the normal Agent admission policy. See [Goal Set acceptance](../../../../crates/meld-execution/src/goals/api.rs#L240) and [acceptance validation](../../../../crates/meld-execution/src/goals/api.rs#L327).

The Execution Goal record stores one optional operational copy of the Strategy authorization beside the Goal. See [ExecutionGoalRecord](../../../../crates/meld-execution/src/goals/contracts.rs#L5).

### Active design

The active Agent runtime design defines curation as deterministic Goal drafting and satisfaction over explicit belief and planner inputs. It states that a Goal draft passes through Strategy before the named Goal Set boundary. See [Agent runtime surface](../../../cognitive_architecture/world_model/agent/runtime_surface.md).

### Absent behavior

No current Agent curation decision can emit an epistemic-operation request, a graph relation, a Curation-owned entity, or a heterogeneous Strategy plan.

No current Goal Set record accepts or stores a non-executable epistemic product. Its Strategy authorization carries an executable `Composition`. See [ExecutionStrategyAuthorization](../../../../crates/meld-execution/src/goals/contracts.rs#L29).

## Events Append And Ingestion

### Implemented code

`EventEnvelope` contains producer ownership, event type, stream identity, domain objects, directed relations, source-record provenance, and producer-owned payload data. See [EventEnvelope](../../../../crates/meld-events/src/events.rs#L151).

`DomainObjectRef` supplies a stable domain, object kind, and object identity coordinate. `EventRelation` supplies a producer-named directed edge between two coordinates. The Events domain explicitly does not interpret payload or graph semantics. See [graph attachment contracts](../../../../crates/meld-events/src/events/contracts.rs).

The append authority validates ledger provenance and genesis identity, writes durably through the single writer, and returns an identity-bearing sequence receipt. See [durable append](../../../../crates/meld-events/src/events/authority.rs#L370).

The replay capability returns bounded pages against an identity-checked ledger cursor. See [bounded replay](../../../../crates/meld-events/src/events/authority.rs#L451).

Execution Task runtime publishes Task lifecycle events. The docs belief mapping listens for `execution.task.succeeded` records carrying a `docs_freshness_assessment` artifact. See [docs outcome mapping](../../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json#L27).

### Absent behavior

Events has no curation-specific command protocol and no epistemic-operation executor. It can already carry producer-authored objects and relations, but current Curation code does not publish such envelopes.

No current event type denotes the start, completion, abstention, or bounded incompleteness of an epistemic operation.

## Traversal Event Reduction And Queries

### Implemented code

Traversal is implemented under `world_state::graph`. Its module contract says that it reduces runtime Events into object facts, current anchors, anchor history, and relation indexes while leaving belief semantics outside the graph layer. See [Traversal module](../../../../crates/meld-world-model/src/world_state/graph.rs).

`TraversalReducer` replays Event records, copies graph-readable Events into `TraversalFactRecord`, derives anchor intents, updates current-anchor indexes, and emits derived anchor Events. See [TraversalReducer](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs#L23).

Current admission into the traversal projection is restricted by producer domain id to `workspace_fs`, `context`, and `execution`. See [traversal source filter](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs#L239).

`TraversalFactRecord` preserves the source sequence, event type, object coordinates, and relations. `GraphWalkSpec` supplies direction, relation filters, maximum depth, current-only selection, and optional fact inclusion. `GraphWalkResult` returns visited objects, visited facts, and bare traversed relations. See [Traversal contracts](../../../../crates/meld-world-model/src/world_state/graph/contracts.rs#L127).

`TraversalQuery` exposes current anchors, histories, neighbors, bounded walks, facts for an object, and anchor provenance. See [Traversal query facade](../../../../crates/meld-world-model/src/world_state/graph/query.rs#L33).

`GraphRuntime` catches the projection up from the Event authority before consumers query current indexes. It persists a cursor and an outbox for derived graph Events. See [GraphRuntime](../../../../crates/meld-world-model/src/world_state/graph/runtime.rs#L75).

### Absent behavior

Traversal does not execute semantic derivation rules and exposes no public semantic graph-authorship API independent of Events.

Events from an additional producer domain are not currently materialized unless the traversal source filter is changed or the producer uses one of the three admitted domain ids.

The walk result does not retain a relation occurrence identity or source sequence directly beside each returned relation. A caller can request visited facts and correlate relation values separately, but the returned relation list itself contains bare `EventRelation` values.

## Belief Integration

### Implemented code

Belief has two current graph couplings.

Anchored belief assessment asks Traversal for a current anchor and its provenance, normalizes that material into Evidence, assigns the Evidence, compares it, and commits a belief revision and view. See [anchored Belief assessment](../../../../crates/meld-world-model/src/belief/runtime.rs#L131).

Unanchored belief families explicitly skip graph anchors and begin with a prior-based revision. Evidence can later arrive through promoted Event ingestion. See [unanchored Belief assessment](../../../../crates/meld-world-model/src/belief/runtime.rs#L238).

The Evidence ingestion actor replays Events, applies an installed `OutcomeEvidenceMapping`, converts matching outcomes into promoted Evidence records, and calls the Belief-owned ingestion path. See [Evidence ingestion actor](../../../../crates/meld-world-model/src/belief/evidence_ingestion.rs).

Planner combines one Belief view with current graph-anchor identities and source fact identities. It reduces graph availability to `PlannerGraphScope` rather than exposing relation topology to Strategy. See [Planner query](../../../../crates/meld-world-model/src/planner/query.rs#L27).

### Active design

Belief-owned normalization, evidence assignment, assessment, revision, and projection are evidence materialization and settlement. Canonical Epistemic Curation is a separate authorship domain. See [Belief specification](../../../cognitive_architecture/world_model/belief/spec.md) and [Evidence materialization](../../../cognitive_architecture/world_model/belief/materialization.md).

### Absent behavior

Belief does not execute a general Agent-authored epistemic-operation contract.

Planner does not freeze or return a bounded relation walk for Strategy. It returns propositions plus anchor and source-fact references through its current projection product.

## Docs Freshness Current Branch Behavior

### Implemented code

The maintained condition is belief confidence above `0.7` for the `docs_freshness` dimension. See [maintained condition](../../../../theory/docs_freshness/maintained_condition.docs_freshness.json).

The Strategy settlement obligation is existence of a `docs_freshness_assessment` artifact. The only Capability that asserts that obligation is `docs.assess_published_scope`. That Capability requires a `docs_publication_receipt`. See [docs Strategy theory](../../../../theory/docs_freshness/strategy_theory.docs_freshness.json).

Closing the required artifact inputs produces the configured chain:

```text
docs.inspect_scope
-> docs.draft_patch_set
-> docs.validate_patch_set
-> docs.publish_patch_set
-> docs.assess_published_scope
```

`docs.inspect_scope` scans meaningful directories and source files but deliberately excludes managed README files. See [scope inspection](../../../../src/docs/capability.rs#L572).

`docs.draft_patch_set` generates one README patch for every meaningful directory through the configured provider. It does not first compare an existing README against required source claims. See [README drafting](../../../../src/docs/capability.rs#L693).

`docs.validate_patch_set` extracts claims from candidate README patches, obtains claim verdicts, requires citations, can revise rejected candidates, and fences accepted exact bytes. See [claim validation](../../../../src/docs/claim_validation.rs#L320).

`docs.publish_patch_set` is declared as a write Capability and produces a publication receipt. `docs.assess_published_scope` can run only from that receipt and emits the assessment artifact consumed by Belief. See [docs capability implementations](../../../../src/docs/capability.rs#L232).

The outcome interpretation maps a successful execution event containing that assessment artifact into `docs_freshness` Evidence fields. See [docs outcome interpretation](../../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json#L27).

The Agent later evaluates the revised planner state and may emit satisfaction for its active Goal. See [Goal satisfaction curation](../../../../crates/meld-world-model/src/agent/curation.rs#L420).

### Current mismatch stated as evidence

The current branch cannot establish that an already present README is correct without constructing and running the configured executable chain. Existing README bytes are excluded from scope inspection, the Strategy theory has no epistemic-only settlement producer, and the assessment Capability requires a publication receipt.

The current branch therefore treats docs freshness correction and docs freshness epistemic establishment as one executable artifact pipeline. This is an implemented-model limitation, not a test failure claim.

The current claim types are attached to candidate README patch validation artifacts. They are not promoted as graph-addressed source claims, required README claims, or coverage relations. See [claim records](../../../../src/docs/claim_validation.rs#L100).

No implemented product represents expected README identity before filesystem observation. A missing README is discovered operationally because the docs pipeline produces a patch path for every meaningful directory, not because Traversal exposes an expected README node without a current workspace realization.

### Active design

The active docs freshness Strategy example correctly states that Goal meaning is observed correspondence, that Task completion does not prove correctness, and that evidence reconciliation remains separate from Agent satisfaction. See [docs freshness Strategy example](../../../cognitive_architecture/world_model/strategy/docs_freshness.md).

The discovery document for bounded epistemic operations proposes first-class epistemic steps, but its status explicitly says implementation is not authorized. See [bounded epistemic operations discovery](bounded_epistemic_operations.md).

## Current Meanings Named Curation

The repository currently uses curation in several distinct senses.

`AgentCuration` is implemented Goal draft and Goal satisfaction decision logic over Belief, planner projection, active Goals, and installed Agent theory. Its emitted boundary products are Goal commands and Goal mutation commands. See [Agent curation code](../../../../crates/meld-world-model/src/agent/curation.rs) and [Agent decision contracts](../../../../crates/meld-world-model/src/agent/contracts.rs#L503).

`AgentCurationRuleRegistryStore` is an implemented append-only registry for exact revisions of Agent Goal curation rules. See [curation rule registry](../../../../crates/meld-world-model/src/agent/curation_registry.rs).

`CurationGoalSetPort` is an implemented named command boundary from Agent Goal curation into Execution Goal storage. See [Goal Set port](../../../../crates/meld-world-model/src/agent/goal_port.rs).

Belief integration and revision are distinct from canonical Epistemic Curation. The implemented code exposes the Belief behaviors as Evidence normalization, ingestion, assessment, store, and query modules rather than a public general `Curation` domain. See [Belief module surface](../../../../crates/meld-world-model/src/belief.rs) and [Evidence materialization](../../../cognitive_architecture/world_model/belief/materialization.md).

Docs claim validation is implemented semantic assessment of candidate README claims. The code does not name it Curation and does not publish its claim graph as Traversal input. See [docs claim validation](../../../../src/docs/claim_validation.rs).

There is no top-level `curation` module in the current `meld-world-model` domain snapshot.

## Does Current Curation Fire An Event

Current Agent Goal curation does not append an Event as its primary output. It persists an Agent decision, submits a Goal command directly through the named Goal Set port, and records a sink receipt. See [Agent runtime submission](../../../../crates/meld-world-model/src/agent/runtime.rs#L505) and [Goal Set port](../../../../crates/meld-world-model/src/agent/goal_port.rs).

Current Agent satisfaction curation likewise submits a Goal lifecycle mutation through the Execution boundary. It does not publish a graph relation.

Current docs assessment reaches Belief through an Event, but the Event is an Execution Task success event carrying a Task artifact. It is not an epistemic-operation event and it is emitted after Execution. See [docs outcome mapping](../../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json#L27).

Current Traversal does consume qualifying Events and materialize their objects and relations. This mechanism is implemented independently of Agent curation. See [Traversal reducer](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs#L62).

## Pass Two Affected-Domain Decomposition

This decomposition records current major concerns one level below each affected package. The change-posture column is `not needed` because this artifact is a ground map and does not authorize a future change.

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| docs capability invocation | `meld` docs | Implements inspect, draft, validate, publish, and assess as executable Capabilities | Describe current docs freshness mechanics | `not needed` | Claim products remain Task artifacts | [docs capabilities](../../../../src/docs/capability.rs) |
| docs claim assessment | `meld` docs | Extracts and assesses claims from candidate README patches | Describe current semantic validation | `not needed` | Source claims and required coverage are not graph products | [claim validation](../../../../src/docs/claim_validation.rs) |
| root runtime composition | `meld` runtime | Wires Agent Goal command to Goal Set and composes graph, belief, Agent, planning, and dispatch actors | Describe current handoffs | `not needed` | Adapter shape can look authoritative if read apart from owner contracts | [runtime ports](../../../../src/runtime/ports.rs), [runtime assembly](../../../../src/runtime/assembly.rs) |
| event contracts | `meld-events` | Carries object coordinates, relations, provenance, and producer payload | Describe graph-capable envelope | `not needed` | Events does not interpret semantic meaning | [event envelope](../../../../crates/meld-events/src/events.rs#L151) |
| append and replay | `meld-events` | Owns durable append, sequence identity, replay, and cursor bounds | Describe current ingestion spine | `not needed` | Consumer projection completeness depends on replay cursor health | [authority](../../../../crates/meld-events/src/events/authority.rs) |
| Goal Set | `meld-execution` | Stores active Goal lifecycle and optional exact Strategy authorization | Describe Agent to Execution seam | `not needed` | Current authorization shape is executable Composition only | [Goal contracts](../../../../crates/meld-execution/src/goals/contracts.rs) |
| operational planning | `meld-execution` | Reads accepted Goals and realizes authorized or legacy Compositions | Describe current lowering input | `not needed` | Legacy unauthorised planning path still exists in code | [planning runtime](../../../../crates/meld-execution/src/planning/runtime.rs#L336) |
| Task lowering | `meld-execution` | Compiles operator steps into Task nodes and Task Network mutation sets | Describe executable realization | `not needed` | Recursive Goal steps are deferred | [lowering](../../../../crates/meld-execution/src/planning/lowering.rs) |
| Goal values | `meld-lang` | Represents desired proposition, Agent ownership, priority, provenance, and lifecycle | Describe shared Goal language | `not needed` | Goal does not identify an executor class | [Goal](../../../../crates/meld-lang/src/goal.rs) |
| Composition values | `meld-lang` | Represents operator and recursive Goal steps with semantic edges | Describe current Strategy action graph | `not needed` | No epistemic-operation step variant exists | [Composition](../../../../crates/meld-lang/src/composition.rs) |
| Strategy construction | `meld-world-model` Strategy | Builds one verified executable Composition and prospective evidence route per candidate | Describe current Strategy product | `not needed` | Prospective evidence metadata cannot progress as a first-class step | [Strategy contracts](../../../../crates/meld-world-model/src/strategy/contracts.rs) |
| Agent curation | `meld-world-model` Agent | Produces Goal commands, lifecycle mutations, abstention, or indeterminate decisions | Describe current curation meaning | `not needed` | The name curation is narrower than graph authorship | [Agent curation](../../../../crates/meld-world-model/src/agent/curation.rs) |
| planner projection | `meld-world-model` Planner | Combines Belief view with graph anchor identities into Strategy-readable propositions | Describe Strategy context assembly | `not needed` | Relation topology is not preserved in the projection | [Planner query](../../../../crates/meld-world-model/src/planner/query.rs) |
| Belief integration | `meld-world-model` Belief | Normalizes graph anchors or promoted Event outcomes into Evidence and revisions | Describe epistemic signal lowering | `not needed` | General epistemic operation execution is absent | [Belief runtime](../../../../crates/meld-world-model/src/belief/runtime.rs), [Evidence ingestion](../../../../crates/meld-world-model/src/belief/evidence_ingestion.rs) |
| Traversal materialization | `meld-world-model` world state graph | Reduces admitted Event domains into fact and relation indexes | Describe knowledge graph read substrate | `not needed` | Producer-domain allowlist excludes new domain ids | [Traversal reducer](../../../../crates/meld-world-model/src/world_state/graph/reducer.rs) |
| waiting declarations | `meld-world-model` waiting | Carries diagnostic declarations such as no active Goals or no newer revision | Describe current diagnostic role | `not needed` | It has no semantic planning or curation authority | [waiting contracts](../../../../crates/meld-world-model/src/waiting.rs) |

## Ownership And Boundary Synthesis

The current code assigns executable candidate construction to Strategy, authorization and Goal lifecycle decisions to Agent, accepted Goal storage and realization to Execution, event durability to Events, graph materialization and read queries to Traversal, evidence meaning and revision to Belief, and docs claim correctness mechanics to docs.

The root crate maps between owner contracts and composes runtimes. `meld-lang` supplies shared values but does not run any part of the flow.

The smallest missing connective behavior observable from current code is not one missing function. There is no contract family connecting an Agent-authored epistemic operation request to a durable semantic result, no Strategy product that can sequence that result beside Tasks, and no current Agent curation output for graph authorship.

This paragraph records absence only. It does not state where those behaviors should be added.

## Separated Scopes

The runtime path includes all five workspace packages.

Current behavior ownership sits in root docs and runtime composition, Events, Execution Goal and Task domains, language values, and the world-model Agent, Strategy, Planner, Belief, and Traversal domains.

No implementation write scope is established by this assessment. The artifact is evidence for review, not authorization to change any package.

## Explicit Non-Integration Findings

Execution currently does not consume graph relations, Belief revisions, or Agent curation rationale when lowering an authorized Composition. It consumes the accepted Goal, its operational Strategy authorization, current execution projection, Capability catalog, and Task Network state.

Traversal does not own producer payload meaning, claim validation, Belief comparison, Goal curation, Strategy search, or Task execution.

Events does not own semantic derivation or knowledge correctness.

`meld-lang` does not enforce Strategy grammar or perform execution.

Current Agent curation does not integrate with Events as a graph author.

Current docs claim validation does not integrate its individual claim entities and coverage verdicts with Traversal.

## Evidence Confidence

Confidence is high for current public types, direct runtime handoffs, domain ownership, and the docs freshness Capability chain because these are present in code and installed theory.

Confidence is high for the stated absences because symbol and contract searches covered `crates`, root `src`, `theory`, and `tests`, and the complete world-model top-level domain snapshot contains no Curation domain.

Confidence is medium for which active design text is canonical because the relevant architecture files are currently modified in the working tree. The document therefore labels them active design rather than implemented authority.

## Unresolved Evidence Questions

The repository does not currently answer whether a future epistemic operation would share Goal language, whether its completion would be an Event, whether one Agent authorization covers a whole heterogeneous plan or each enabled product, or which runtime would progress such a plan.

Those are design questions for the dedicated reviews. They are not current-code facts.
