# Epistemic Curation Source Assessment

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-08-28

Status: approval-ready design evidence

Proposed source slice: `WMR-VC-02`

Implementation authorization: none

## Problem First

The accepted first vertical makes one exact owner publication available through an immutable `TraversalCut`. The live runtime still cannot apply an Agent-scoped epistemic rule to that cut, author Curation-owned expected state or assessments, persist a terminal operation, publish the authored products through Events, or let an installed Belief mapping settle the result.

Current `world_model.agent_goal_curation` and `world_model.satisfaction_curation` do something different. They consume Belief and planner projections to form Goals or judge Goal lifecycle. They neither author epistemic graph products nor execute bounded epistemic operations. Reusing those actors as the missing Curation owner would merge Agent judgment with semantic authorship.

The proposed architectural thesis is one sentence: add one standing Curation owner between exact Traversal input and existing Event, Graph, and configured Belief authorities while leaving Agent Goal formation and satisfaction unchanged.

## Concern And Scope

Direct behavior: one installed standing Curation rule accepts one complete immutable cut, durably reaches one terminal result, publishes its Curation-owned semantic products through the existing Event authority, becomes visible through the existing Graph owner route, and produces a Belief revision only when the installed mapping selects that result.

In scope:

- standing Curation under exact Agent, perspective, branch, generation, rule, subject, and cut identity
- durable admission, terminal result, publication outbox, retry, and restart state
- `applied`, `unchanged`, `abstained`, `incomplete`, `conflicted`, and `failed` terminal classes
- deterministic owner publication through `world_state.owner_publication.v1`
- one Curation terminal Event shape that existing configured evidence mapping can select
- Graph visibility and optional configured Belief settlement
- root runtime composition, bounded work reporting, waiting, and restart proof

Out of scope:

- Strategy-planned Curation
- Agent Plan authorization, progression, or result acceptance
- Planner changes
- Strategy changes
- Goal formation or satisfaction behavior changes
- Execution, PDS, lifecycle aggregation, Startup, or product migration
- docs-specific or dependency-security-specific runtime semantics
- foreign-owner assertion acceptance

Authority comes from the accepted architecture and the request to prepare the historical `SI-02` outcome. Source implementation still requires explicit approval of `WMR-VC-02` and its expansion decision.

## Maturity Envelope

Posture: `first slice`

Obligation floor: operational durability for accepted operations, Event publication, Graph projection, Belief cursor ordering, and restart replay

Confidence: high for ownership boundaries and existing transport seams, moderate for the exact Curation storage shape until implementation inspection

Evidence:

- `WMR-VC-01` proves the exact owner cut and Graph owner publication route
- root runtime already composes Graph replay, Belief evidence ingestion, Agent Goal curation, and Agent satisfaction curation
- Belief mapping already selects arbitrary producer Event types from installed configuration
- no current Curation domain, operation store, terminal grammar, or runtime actor exists

Direct product proof: a real `ProductRuntimeAssembly` run consumes a complete workspace cut, performs standing Curation, persists the terminal operation, appends deterministic Curation Events, reaches a complete workspace plus Curation cut, and commits one configured Belief revision. Reopen and replay reproduce the same identities without repeating semantic work.

Hard limits:

| Expansion | Limit |
| --- | --- |
| new crates | none |
| new dependencies | none |
| new services or background systems | none |
| new Event authority or Graph authority | none |
| new standalone database | none |
| new Curation durable schema | one Curation-owned tree family inside the existing world-model database, subject to explicit approval |
| new cross-domain contracts | only the Curation public contract and its existing Event and Traversal port adapters |
| parallel implementors | one |

Tripwires:

- more than eighteen production source files changed
- more than two thousand eight hundred production lines added
- any change to `meld-events`
- any second Event consumer for Belief settlement
- any direct Curation write into Graph or Belief storage
- any change to Agent Goal formation or satisfaction semantics
- any planned Curation or Plan progression behavior
- any docs or dependency-security product migration
- any rule or result identity that depends on process order, wall time, or a live mutable read

Investigation budget: twelve batched inspection calls and sixteen source files beyond named design and policy

Review budget: one integrated design review and one bounded verification after accepted corrections

## Regenerated Domain Snapshot

The workspace package boundary remains:

```text
meld
meld-events
meld-execution
meld-lang
meld-world-model
```

The current world-model domain boundary remains:

```text
agent
belief
planner
strategy
waiting
world_state
```

There is no current top-level `curation` domain. The proposed domain is supported by accepted design but absent from source.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| root runtime and theory | `adapter` | composes Graph, Belief, and Agent actors and resolves installed theory | `partial` | [runtime assembly](../../../../src/runtime/assembly.rs), [runtime theory](../../../../src/runtime/theory.rs) | root must not interpret Curation grammar | compose one Curation actor and exact rule revision |
| Events | `publish` | owns durable append, replay, record identity, and producer-neutral payload carriage | `complete` | [Event authority](../../../../crates/meld-events/src/events/authority.rs) | no Events semantic change is needed | reuse unchanged |
| Graph and Traversal | `publish` | owns exact cuts, occurrence-rich results, generic owner projection, and owner visibility | `complete` | [Graph contracts](../../../../crates/meld-world-model/src/world_state/graph/contracts.rs), [Graph query](../../../../crates/meld-world-model/src/world_state/graph/query.rs) | Graph must not execute Curation rules | reuse unchanged |
| Agent | `publish` | owns Agent identity, perspective, branch, maintained condition, Goal formation, and satisfaction | `partial` | [Agent contracts](../../../../crates/meld-world-model/src/agent/contracts.rs), [Agent actor](../../../../crates/meld-world-model/src/agent/actor.rs) | Agent must not become the Curation executor | expose exact initiating authority without changing Goal behavior |
| Curation | `own` | no current domain, store, actor, operation, result, or publication authority exists | `not started` | [world-model crate surface](../../../../crates/meld-world-model/src/lib.rs) | not applicable | add one standing Curation path |
| Belief | `consume` | installed mappings replay Events, map selected records, commit revisions, then advance a durable cursor | `partial` | [evidence ingestion](../../../../crates/meld-world-model/src/belief/evidence_ingestion.rs), [mapping contract](../../../../crates/meld-world-model/src/belief/outcome/mapping.rs) | Belief does not inherit all Curation output | reuse code and add only installed mapping data and proof |
| Planner | `none` | currently consumes Belief and structural Graph inputs | `not needed` | [planner query](../../../../crates/meld-world-model/src/planner/query.rs) | Agent and Planner acceptance are deferred | none |
| Strategy | `none` | constructs one executable candidate and no epistemic operation | `not needed` | [Strategy contracts](../../../../crates/meld-world-model/src/strategy/contracts.rs) | planned Curation is deferred | none |
| Execution | `none` | owns Goals, Tasks, Task Network state, and outcome publication | `not needed` | [Execution crate surface](../../../../crates/meld-execution/src/lib.rs) | epistemic operations are not executable Tasks | none |
| Meld Language | `none` | owns Goal, Proposition, and Composition values | `not needed` | [language crate surface](../../../../crates/meld-lang/src/lib.rs) | no new universal grammar is justified | none |
| workspace source | `publish` | supplies the complete owner cut proved by `WMR-VC-01` | `complete` | [source runtime groundmap](source_runtime_groundmap.md) | workspace never interprets Curation meaning | reuse unchanged |
| docs product | `none` | no source migration is authorized | `not needed` | [program ledger](world_model_reconciliation_source_delivery_program_ledger.md) | the proof uses only a dissimilar specimen | none |
| dependency security product | `none` | no source migration is authorized | `not needed` | [program ledger](world_model_reconciliation_source_delivery_program_ledger.md) | the proof uses only a dissimilar specimen | none |
| PDS and lifecycle | `none` | current activation and supervision remain incumbent | `not needed` | [runtime assembly](../../../../src/runtime/assembly.rs) | aggregate activation closure is a later vertical | none |
| legacy Workflow | `none` | no accepted relationship to standing Curation exists | `not needed` | [program ledger](world_model_reconciliation_source_delivery_program_ledger.md) | compatibility is not demonstrated | none |

## Frozen Affected-Domain Set

The frozen affected set is:

```text
root runtime and theory
Events
Graph and Traversal
Agent
Curation
Belief
workspace source
```

Events, Graph and Traversal, Agent, Belief, and workspace are on the runtime path. Their inclusion does not imply source changes.

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| actor composition | root runtime | concrete Graph, Belief, and Agent handles already tick through one supervisor | bind one standing Curation handle to public stores and ports | `extend existing` | root could become semantic scheduler | [runtime assembly](../../../../src/runtime/assembly.rs) |
| rule installation | root theory adapter | resolves exact Agent curation and other theory revisions | resolve one exact Curation rule revision without reusing Agent threshold rules | `extend existing` | same name could conceal distinct authorities | [runtime theory](../../../../src/runtime/theory.rs) |
| durable carriage | Events | producer-keyed append and replay are canonical | carry Curation result and owner publication payloads intact | `reuse unchanged` | Event type must not define semantic validity | [Event authority](../../../../crates/meld-events/src/events/authority.rs) |
| exact input | Graph and Traversal | immutable cut plus bounded result preserve owner revisions, positions, occurrences, bounds, and hydration | supply the accepted source cut and result to Curation | `reuse unchanged` | live query after admission would break replay | [Graph contracts](../../../../crates/meld-world-model/src/world_state/graph/contracts.rs) |
| semantic output projection | Graph and Traversal | generic owner publication accepts any matching owner batch | project Curation-owned objects and relation occurrences | `reuse unchanged` | Curation currentness cannot be inferred from Event order | [owner publication](../../../../crates/meld-world-model/src/world_state/graph/contracts.rs) |
| initiating authority | Agent | durable record already owns perspective, branch, subject, and installed Agent rules | cite exact Agent and activation generation in standing selection | `extend existing` | no Agent write may be required for Curation completion | [Agent contracts](../../../../crates/meld-world-model/src/agent/contracts.rs) |
| Goal formation and satisfaction | Agent | incumbent actors create Goals and lifecycle mutations from Belief and planner inputs | remain canonical and unchanged | `reuse unchanged` | historical naming may invite accidental replacement | [Agent curation](../../../../crates/meld-world-model/src/agent/curation.rs) |
| operation grammar | Curation | absent | define standing selection, operation, acceptance, terminal result, semantic publication, and successor identities | `new local behavior` | incomplete grammar could permit silent retry or foreign assertions | [accepted detailed design](detailed_design/epistemic_authorship_and_settlement.md) |
| operation persistence | Curation | absent | commit acceptance, terminal result, publication receipts, and selection position under one flush boundary | `new local behavior` | cursor advance before owned effects would lose work | [transition ledger](detailed_design/epistemic_operation_transition_ledger.md) |
| bounded execution | Curation | absent | derive only installed Curation vocabulary from one frozen cut | `new local behavior` | generic rule engines and unbounded recursion exceed maturity |
| publication | Curation | absent | reconstruct deterministic Event records and owner batches after restart | `new local behavior` | best-effort append would lose terminal products |
| result mapping | Belief | installed mappings already select producer Event types and payload fields | select the Curation terminal Event only when configured | `reuse unchanged` | Graph reachability must not imply evidence |
| revision commit | Belief | evidence state commits before its cursor advances | produce a distinct immutable Belief revision from mapped Curation evidence | `reuse unchanged` | Curation result and Belief revision identities must remain separate |
| observed source | workspace | complete exact publication and retry are proven | remain the first concrete source cut | `reuse unchanged` | Curation cannot assert workspace presence |

## Ownership And Boundary Synthesis

Curation is the only new semantic owner. Agent supplies initiating identity and perspective but retains Goal judgment. Traversal supplies immutable structural input and later indexes output but never derives it. Events carries output without interpretation. Belief selectively consumes one configured result and creates its own revision.

The smallest missing connective behavior is a standing Curation actor with a durable owner-local operation store and public adapters to exact Traversal query plus Event append. No new transport, graph schema authority, Belief consumer, planner product, or execution path is needed.

The current Agent actors are incumbent authorities for Agent Goal behavior, not an incumbent implementation of epistemic Curation. Their source is outside likely write scope except for a narrow read-only initiating-authority contract if existing public Agent records cannot supply the exact generation fence.

## Separated Scope

Runtime path:

```text
Agent authority and installed standing rule
-> exact workspace TraversalCut
-> Curation acceptance and terminal result
-> Event append
-> Graph owner projection
-> configured Belief evidence ingestion
-> immutable Belief revision
```

Behavior that changes:

```text
Curation
root composition and theory resolution
installed mapping data for the proof specimen
```

Likely implementation write concentration:

```text
crates/meld-world-model/src/curation.rs
crates/meld-world-model/src/curation/
crates/meld-world-model/src/lib.rs
src/runtime/assembly.rs
src/runtime/storage.rs
src/runtime/theory.rs
src/init/world/
focused world-model and integration tests
one neutral theory fixture
```

The exact file list remains an implementation packet concern. Agent, Events, Graph, Belief, workspace, Planner, Strategy, Execution, Meld Language, and product domains are not authorized write scope merely because they appear on the runtime path.

## Explicit Non-Integration Decisions

- Do not rename or replace the current Agent curation actors in this slice.
- Do not add Curation behavior to `agent`, `belief`, `planner`, `strategy`, or `world_state` internal modules.
- Do not add Curation grammar to Events or Meld Language.
- Do not add a second Belief replay cursor or ingestion actor.
- Do not let Curation write Graph or Belief stores directly.
- Do not interpret an `unchanged` or `applied` result as Goal satisfaction.
- Do not require docs or dependency-security product migration for acceptance.
- Do not implement planned invocation until Agent Plan authorization exists in a later vertical.

## Expansion Decision

The proposed slice needs one new Curation-owned durable tree family inside the existing world-model database. Existing Agent decision storage cannot represent Curation acceptance, terminal results, semantic publication outbox state, or restart position without assigning those meanings to the wrong owner.

This is architectural expansion under the first-slice maturity envelope. It is approval-ready because it unblocks the direct product path, adds no database or service, and has one current runtime consumer. It remains unauthorized until the user explicitly approves `WMR-VC-02` with this storage boundary.

## Unresolved Questions

No question blocks design approval. Implementation must select one explicit successor trigger for failed standing work from the accepted set of renewed authority, explicit retry generation, or changed declared input. The recommended first vertical permits only changed declared input or changed rule revision. Deadlines may wake eligibility evaluation but may not mint a successor operation.

The exact internal count of Curation trees may change during implementation without changing the approved expansion so long as they remain one Curation-owned tree family in the existing database and no second writer exists.

## Evidence Basis And Confidence

Evidence combines the accepted `WMR-VC-01` source and receipts, current Agent actor and curation code, current Graph cut and owner publication contracts, current Belief evidence ingestion and configurable mapping, current root assembly, and the accepted epistemic authorship design.

Confidence is high for the frozen affected set, incumbent disposition, and existing seams. Confidence is moderate for the estimated file and line tripwires because no Curation source domain exists yet. Crossing either tripwire pauses implementation for explicit user disposition.

## Preparation Outcome

`WMR-VC-02` is approval-ready as a standing Curation vertical. Implementation has not started. The historical `WMR-SI-02` remains withdrawn and must not be reactivated.
