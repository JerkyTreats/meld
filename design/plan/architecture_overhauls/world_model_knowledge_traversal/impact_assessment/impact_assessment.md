# Architecture Wide Strategy Plan Reconciliation Impact Assessment

Date: 2026-08-20

Status: final evidence assessment for later canonical requirements

## Problem Statement

The current docs freshness branch gets its primary use case wrong in one precise way. It can change documentation, but it cannot establish that existing documentation is already correct.

The installed path excludes managed README files from inspection evidence. It constructs candidate README claims only after drafting. Its only freshness assessment requires a publication receipt. The implemented chain is therefore:

```text
inspect source
-> draft README patches
-> validate candidate claims
-> publish patches
-> assess the published scope
```

If README F already exists and is correct, Meld still has no epistemic-only path to establish that result. If README F is absent, Meld discovers the absence only because the write pipeline drafts a path for every meaningful directory. No current entity represents the expectation that README F must exist before workspace observation can fail to realize it.

The same structural gap appears in dependency security. That domain defines deterministic inventory, advisory, assessment, and verification records with explicit completeness and bounded posture. Current owner admission covers only inventory and advisory adapter values in an in-memory map. Assessment and verification remain unpersisted function results, and no declared security Event is appended. A separate root lifecycle tracks Capability-shaped fixture operations. The epistemic work is still exposed to Strategy only through executable Capability contracts.

These are not failures of Task Network scheduling. They are failures in how the world model represents expected state, observed state, epistemic work, and causal planning.

The current [code ground map](../current_code_groundmap.md) establishes the implemented docs path, one-candidate Strategy shape, Agent-to-Execution handoff, Event ingestion, Traversal reduction, and Belief flow. The recursive crate reports establish the entity-level impact behind this synthesis.

## Assessment Thesis

The architecture change is real and major, but it is not a system-wide rewrite.

The semantic refactor belongs overwhelmingly inside `meld-world-model`. Strategy becomes the pure constructor and reconstructor of a heterogeneous causal Plan. Agent owns durable Plan authority, progression, and reconciliation. Curation becomes a new world-model domain that realizes bounded Epistemic Operations and authors its results through Events. Traversal remains the graph materialization and read substrate. Planner supplies Strategy with a frozen, relation-rich knowledge cut. Belief remains a selective evidence consumer.

The rest of Meld participates through sacred seams. Product domains publish owner-shaped observations. Events carries them without interpreting them. `meld-lang` supplies permissive nouns and pure functions without learning Strategy grammar. Execution accepts eligible executable products and remains ignorant of epistemic work. Task Network continues to own only how and when executable work runs.

```text
product observations
-> Events
-> Traversal
-> Curation and configured Belief
-> Agent reconciliation
-> pure Strategy Plan construction
   -> eligible Epistemic Operation to Curation
   -> eligible executable product to Execution
-> new Events
-> independent world-model visibility milestones
```

The Plan is the causal aggregate. Goal names desired state. Task and Epistemic Operation are distinct means of discharge. Neither product becomes the other, and neither product becomes the Plan.

## Recursive Evidence Lineage

The assessment began with every public semantic entity in each affected domain. Entity findings were synthesized into owning domains, domain findings into crates, and crate findings into this architecture-wide result.

```text
current public entities and state transitions
-> sub-crate domain findings
-> five crate syntheses
-> architecture-wide impact
-> later canonical requirements
```

The [world-model and language assessment](entity_assessments/world_model_and_lang.md) covers every current domain in both crates, the absent Curation boundary, all affected public entity families, installed theory, and crate-level synthesis.

The [Execution and Events assessment](entity_assessments/execution_and_events.md) covers Goal Set cardinality, authorized intake, lowering, Task, Task Network, Event append, replay, cursors, and both crate syntheses.

The [root Meld assessment](entity_assessments/root_meld.md) covers all thirty-seven exported root domains, thirteen affected domains, product-owned observations, theory selection, runtime composition, boundary adapters, and root crate synthesis.

## Architecture Wide Crate Impact

| Crate | Runtime relationship | Proven behavior pressure | Likely write scope | Reused unchanged |
| --- | --- | --- | --- | --- |
| `meld-world-model` | Owns Agent, Strategy, Planner, Belief, Traversal, and future Curation | Major and direct | Agent, Strategy, Planner, Traversal, new Curation, exports, selected theory | Belief core, Graph replay discipline, source anchors, waiting, legacy claim compatibility |
| root `meld` | Publishes product evidence, installs theory, composes runtimes, adapts owner seams | Moderate and use-case driven | docs, dependency security, fixed theory selection, world initialization, runtime composition, harness evidence views | generic theory router, Event authority binding, Task facade, provider, legacy workflow isolation |
| `meld-execution` | Consumes eligible executable products and lowers them into Task Network | Narrow seam pressure | authorized intake and lowering seam, with Goal Set storage conditional on product cardinality | Capability, execution authority, Task, Task Network, dispatch, result publication |
| `meld-events` | Carries and replays all durable producer records | None | none proven | envelope, object refs, relations, provenance, append, replay, cursors, watermarks, observability |
| `meld-lang` | Supplies shared desired-state and executable vocabulary | None | none proven | all current language domains and pure helpers |

Runtime participation is deliberately broader than write scope. Every language value and every Execution stage remains on a complete run path, but that does not make those crates owners of the redesign.

## Complete Domain Coverage

The root crate sweep covered all thirty-seven exported domains. Its frozen relationship set is `capability`, `config`, `dependency_security`, `docs`, `events`, `execution`, `harness`, `init`, `runtime`, `task`, `theory`, `workspace`, and `world_state`. The remaining twenty-four root domains have explicit `none` findings in the [root report](entity_assessments/root_meld.md#pass-one-domain-sweep).

The world-model sweep covered `agent`, `belief`, `planner`, `strategy`, `waiting`, and `world_state`. The affected current set is Agent, Belief, Planner, Strategy, and world state. Waiting has an explicit `none` finding. Curation is assessed as a missing target domain rather than being inserted into the current source map as if it already existed.

The Execution sweep covered `authority`, `capability`, `error`, `execution`, `generation`, `goals`, `lib`, `planning`, `publish`, `task`, `task_network`, `traversal`, `waiting`, and `workflow`. The runtime relationship set is authority, Capability, execution ports, Goals, crate exports, planning, Task, and Task Network. Error, generation, publish, execution-local traversal, waiting, and legacy workflow have explicit `none` findings.

The Events sweep covered error, Events, and crate exports. Events and exports are reused unchanged. Error has an explicit `none` finding.

The language sweep covered authority, Composition, Condition, Cost, Effect, evaluation, Goal, Method, Operator, Proposition, substitution, Term, unification, validation, and WorldState. Every domain participates as shared vocabulary or pure reasoning. No language domain enters likely Rust write scope.

## World Model Entity To Domain Synthesis

### Agent

Current Agent already owns the root perspective, directive, maintained condition, Goal authority, durable decisions, replay-safe sink handoff, and final Goal satisfaction judgment. It currently invokes Strategy once for one Goal, stores one candidate authorization, and sends one executable Goal command.

The missing entity family is durable active Plan state. Current code has no Plan identity, revision lineage, per-product eligibility, epistemic handoff receipt, or reconciliation cursor. The entity evidence places those responsibilities with Agent because they are persistent authority and progression, not pure construction.

Agent therefore carries major behavior impact. It does not absorb Curation semantics. It gains a separate handoff to the Curation owner and asks Strategy to reconstruct from a new frozen knowledge cut when admitted knowledge changes.

### Strategy

Current `StrategyCandidate` is one executable `Composition` plus a settlement obligation, prospective evidence metadata, Capability identities, bindings, and evaluation. Prospective evidence is descriptive. It cannot execute before a Task, eliminate a Task, gate a Task, or settle an epistemic-only branch.

Strategy carries the largest semantic change. Its product becomes a heterogeneous causal Plan over desired conditions, closed executable products, closed bounded Epistemic Operations, dependencies, exact context, and revision lineage. Strategy remains pure and bounded. It constructs and independently verifies immutable Plan revisions. It does not execute products and does not own persistent progression.

The current Strategy theory packages also assume executable Capabilities. Docs freshness requires write-before-know. Dependency security represents epistemic acquisition as executable action. Those installed theory products are directly affected.

### Curation

Curation has no current domain or public entity to extend. It is a new world-model ownership boundary.

Its missing entity families are a bounded Epistemic Operation contract, standing and planned entry paths, durable operation identity, terminal lifecycle, typed result vocabulary, perspective and rule lineage, currentness and supersession semantics, replay-safe actor state, and Event publication.

Curation reads through bounded knowledge contracts and publishes through Events. It does not mutate Traversal storage. It may author expected README F, attach material source claims, assess realization against a frozen workspace snapshot, and publish completed, unchanged, abstained, bounded-incomplete, rejected, or failed results. A terminal result matters even when no graph claim changes because downstream Plan dependencies must distinguish completion from silence.

### Planner

Current Planner reduces Traversal into accessibility, anchor ids, source fact ids, and a flat `meld-lang::WorldState`. That projection is sufficient for the current one-candidate path but loses the relation topology and occurrence provenance needed for expected-versus-observed reasoning.

Planner is affected because Strategy needs a bounded, perspective-correct, replayable knowledge cut. The evidence proves that the current projection is too thin. It does not yet decide whether Planner owns the richer cut or consumes a neutral frozen-cut product shared with Curation.

### Traversal And World State

Traversal is already the knowledge graph substrate in the intended loose sense. It owns Event-backed materialization, structural graph reads, bounded walks, stored relation occurrences, provenance, source anchors, and catch-up-aware queries.

Its impact is narrower than Curation. The current reducer admits only `workspace_fs`, `context`, and `execution` producer domains. It must be able to admit deliberately graph-attached Curation products without learning their semantics. The store preserves relation occurrence identity, but `GraphWalkResult` returns bare relations and discards that identity. Currentness is meaningful mainly for source-specific selected anchors and cannot become universal edge semantics.

Traversal therefore needs admission and read-fidelity work. It does not gain epistemic grammar, arbitrary graph mutation, owner judgment, or universal currentness.

### Belief

Belief already has the right consumer boundary. It ingests intact Events through installed mappings, promotes configured evidence, commits append-only revisions, and gives Agent exact revision signals.

No Belief Rust change is proven. Installed mapping theory changes only where a Curation result is intended to update a named Belief family. Graph reachability alone does not become evidence admission.

## Root Product And Composition Synthesis

### Docs

Docs must expose owner observations before Curation can reason over them. Current source evidence is aggregated text. Current README claims exist only for generated candidates. Current correctness assessment is fenced to a publication candidate.

The product impact is independently addressable observed source claims, observed README claims, snapshot identity, and owner verification records. Curation then owns requirement, materiality, realization, coverage, and bounded assessment relations under an Agent perspective. Docs does not own the Strategy Plan.

This distinction is what makes the following epistemic-only path possible:

```text
folder F observed
-> Curation authors expected README F
-> source and README observations become addressable
-> Curation relates required source claims to README claims
-> Curation assesses realization and coverage
-> Agent reconciles the Goal from admitted knowledge
```

If realization or coverage fails, Strategy has a clean signal from which to construct executable work. If both hold, no Task is needed.

### Dependency Security

Dependency security supplies a second concrete use case. It separates adapter transport from in-memory owner admission for inventory and advisory values. A separate root lifecycle records bounded Capability operation attempts, terminal status, activation lineage, and admitted product references. This is useful precedent for operation identity and completion accounting, but it is not durable admission of the complete security product family.

It is not a ready-made generic Curation domain. Its operation record is tied to executable Capability contracts, its admission covers only two adapter result kinds, and its declared Events are not published. The required assessment pressure is to distinguish owner observation, bounded epistemic operation, and genuinely executable intervention, then durably publish owner-admitted products needed by Traversal. Reusing its lifecycle blindly would move Curation into root runtime and Execution.

### Workspace

Workspace already publishes physical snapshot, node, containment, and observation identities. A concrete connective gap remains: the graph preserves the reached object coordinate but not the Event payload containing the path and node kind. A generic walk cannot by itself tell Curation that a reached node is `README.md`.

Typed hydration is therefore required somewhere on the owner-correct boundary. A workspace code change is only conditional because the evidence does not select among richer promoted observation, exact source Event hydration, or a narrow workspace query contract.

### Theory, Initialization, Runtime, And Harness

The generic PDS package, route, installation receipt, and router already preserve exact producer ownership. They do not need a universal grammar change. The fixed root selection and runtime views do enumerate the current executable-only theory image, so those adapters are affected when Strategy Plan and Curation theory become installable.

Root runtime currently maps one Agent decision and one `StrategyCandidate` directly into one Execution authorization. It must compose the new world-model owners and route their public products without learning their internal meaning.

Harness projections currently describe the one-executable-candidate flywheel. They are affected only as evidence surfaces. They should observe Plan and Curation records if the harness remains the canonical architecture proof tool. They do not define those records.

## Execution, Events, And Language Seams

### Execution

Execution can remain the simple execution machine defined by the execution invariant. It receives eligible complete executable work with Goal attribution, protects execution-owned transport shape, executable structure, live Capability availability, authority, idempotency, and Task Network state, then lowers and realizes that work.

The current authorized planning path still requests fresh world state, compares planner frames, and reassesses operator preconditions. That is world-model premise revalidation inside a consumer. The evidence places change pressure on the authorized intake-to-lowering seam so it can consume a complete Agent-authorized executable product without interpreting the heterogeneous Plan.

Goal Set storage impact is conditional. Current storage holds one authorization per Goal, and Goal modification deliberately preserves the old authorization. That cannot represent several executable products under one Execution-facing Goal identity. It may require no storage redesign if every eligible executable product receives a distinct derived Execution-facing Goal. Canonical requirements must decide product and Goal cardinality before claiming Goal Set writes.

Task, Task Network, scheduling, ranking, parallelization, sequencing, dispatch, artifacts, and execution result publication remain unchanged in meaning. Task Network never coordinates dependencies between Tasks and Epistemic Operations.

### Events

No `meld-events` behavior change is proven. `EventEnvelope`, `DomainObjectRef`, `EventRelation`, provenance, idempotent append, replay, durable cursors, watermarks, and observability already form the required carrier and recovery spine.

Curation owns its payload grammar, identities, perspectives, bounds, terminal semantics, and currentness. Events accepts and replays those records without deciding what they mean. A durable Event append does not prove Traversal materialization, Belief reconciliation, or Agent visibility. Each consumer owns its own cursor and semantic completion.

### Language

No `meld-lang` Rust change is proven. Goal and Proposition already express desired state. Composition, Operator, Effect, Method, Cost, authority, substitution, and validation already express or inspect executable means. WorldState, evaluation, and unification remain pure reasoning values.

The heterogeneous Plan can be a world-model-owned aggregate over those unchanged values. Adding Epistemic Operations to `StepKind`, broadening shared Method grammar, or teaching language validation to enforce Strategy intent would violate the ownership boundary and needlessly widen the refactor.

## Separated Architecture Scope

### Proven Core Behavior Change

The evidence proves a core world-model change in Agent Plan authority and progression, Strategy Plan construction and reconstruction, a new Curation domain, richer bounded Strategy context, and Traversal admission plus occurrence-preserving reads.

It also proves product-enablement pressure in docs and dependency security, fixed theory selection and runtime composition, and the root adapter that currently equates one Strategy candidate with one executable authorization.

The authorized Execution intake path is under direct pressure because it re-proves world-model premises. Goal Set storage is only conditional on the eventual executable product cardinality model.

### Likely Write Concentration

The primary Rust write concentration is:

```text
crates/meld-world-model/src/agent
crates/meld-world-model/src/strategy
crates/meld-world-model/src/planner
crates/meld-world-model/src/world_state
crates/meld-world-model/src/curation
crates/meld-world-model/src/lib.rs
```

The root use-case and composition concentration is:

```text
src/docs
src/dependency_security
src/config/stewardship
src/init/world
src/runtime/assembly.rs
src/runtime/ports.rs
src/runtime/theory.rs
src/harness
theory/docs_freshness
theory/dependency_security
```

Workspace is conditional on the hydration contract. Execution writes are constrained to authorized intake and lowering, with Goal Set persistence conditional on cardinality. Selected Belief mappings are data and theory impact rather than proven core Rust impact.

### Proven Reuse

No semantic change is proven for `meld-events`, `meld-lang`, world-model Belief core, Execution Capability, execution authority, Task, Task Network, provider integration, generic theory routing, physical workspace snapshot identity, or legacy workflow internals.

Legacy workflow remains active compatibility code, but it receives no role in the canonical Plan architecture.

## Requirements Evidence Gaps

The impact scope is sufficiently clear to support canonical requirements, but several requirements decisions remain genuinely open.

The first is the durable Plan representation and authorization granularity. Evidence places progression with Agent and immutable construction with Strategy. It does not decide whether Agent authorizes a whole Plan revision, each newly eligible product, or both at separate moments.

The second is the exact visibility milestone on each Plan dependency. Event durability, Traversal materialization, Belief revision, Agent delivery, Curation termination, and workspace observation are distinct owner positions. A later specification must name which one enables each dependent product.

The third is executable product identity and cardinality. That decision controls whether the current Execution Goal Set can be adapted at its existing one-product-per-Goal shape or needs storage changes.

The fourth is the frozen knowledge-cut owner. Current code proves that Planner loses needed relation topology. It does not decide whether Planner owns the richer cut or consumes a neutral cut shared with Curation.

The fifth is Curation vocabulary. Operation bounds, rule revisions, admissible terminal results, perspective, expected-entity identity, currentness, and supersession need one owner-shaped contract. The evidence does not yet choose its exact schema.

The sixth is typed workspace hydration. The gap is proven, but the smallest owner-correct connective contract remains unresolved.

The seventh is dependency-security classification. Its existing operations prove the need for bounded lifecycle and complete owner products. They do not by themselves decide which operations are epistemic, executable, or standing owner work.

## Final Impact Judgment

The proposed architecture is justified by current code and materially improves Strategy planning capability. It gives Strategy access to relevant relation-rich knowledge, allows epistemic work to become causal rather than decorative, allows epistemic work to eliminate unnecessary Tasks, and lets post-execution verification participate in the same Plan without entering Task Network.

The cost is a major world-model refactor plus targeted product publication and root composition work. The evidence does not support a broad rewrite of Events, language, Execution internals, or Task Network. Holding those seams fixed is not merely scope reduction. It is the ownership model that keeps the overhaul coherent.

This assessment is the evidence basis for canonical requirements. It does not yet authorize those requirements, an implementation sequence, migrations, or code changes.

## Review Quality

The world-model and language reviewer produced the strongest ownership result. It established entity by entity that durable Plan progression belongs with Agent, immutable Plan construction belongs with Strategy, Curation owns epistemic realization, and the mixed Plan can remain world-model local over unchanged language values. Confidence is high. Its only deliberate uncertainty is the Planner versus neutral frozen-cut boundary and exact Agent authorization granularity.

The Execution and Events reviewer made the most important scope correction. It verified Goal Set storage and modification semantics, then refused to treat Goal Set writes as unconditional. It also proved that current authorized planning revalidates world-model premises while Task Network and Events can remain unchanged. Focused Goal, planning, Event authority, and durable cursor tests passed.

The root Meld reviewer had the widest sweep and completed all thirty-seven domains with explicit `none` findings. It caught the dependency-security operation precedent and the workspace typed-hydration gap, both of which materially sharpen later requirements without widening ownership. Its uncertainty is appropriately limited to operation classification and the hydration shape.

All three reports completed entity-first decomposition, upward domain and crate synthesis, exact evidence links, terminology checks, prose checks, and diff validation.
