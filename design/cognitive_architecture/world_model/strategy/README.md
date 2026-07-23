# World Model Strategy

Date: 2026-07-23
Status: active
Scope: Agent-authorized semantic construction of evidence-backed action Compositions

## Thesis

`world_model/strategy` owns the transition from desired reality to an evidence-backed theory of action.

A Directive says what should be maintained. Directive grounding turns that intent into questions about concrete things. Belief Reconciliation answers those questions as far as current evidence allows. When the Agent decides that a belief divergence warrants action, it creates a Goal draft.

Strategy then asks:

```text
Given what we want, what we currently believe, and what actions exist,
which concrete theories of action could satisfy this Goal?
```

It may reuse a known Strategy, instantiate a verified Method, construct a new Composition, or combine those sources. The Agent judges the resulting candidates. Execution sees only an admitted Goal and its authorized candidates.

## End-to-end flow

```mermaid
flowchart TD
    subgraph WM[World model]
        D[Directive]
        PDS[PDS operational domain theory]
        STATE[Current graph and world-model views]

        GROUND[Ground Directive into belief questions]
        BELIEFS[Reconcile current beliefs]
        CURATE[Decide whether action is warranted]
        DRAFT[Goal draft]

        KNOWN[Known Strategy catalog]
        METHODS[Verified Method inventory]
        BUILD[Construct bounded Strategy candidates]
        CANDIDATES[Concrete Composition candidates]
        AVAILABLE{Any eligible candidate}
        ERROR[NoMethodAvailable]
        JUDGE[Directive Agent judgment]
        ADMISSION[Goal admission bundle]
    end

    subgraph EX[Execution]
        GOAL[Admitted Goal]
        PLAN[Realize authorized candidate]
        NETWORK[Commit task network]
    end

    D --> GROUND
    PDS --> GROUND
    STATE --> GROUND
    GROUND --> BELIEFS
    STATE --> BELIEFS
    BELIEFS --> CURATE
    D --> CURATE
    CURATE --> DRAFT

    DRAFT --> BUILD
    PDS -->|action meaning and outcome theory| BUILD
    STATE -->|current projections| BUILD
    KNOWN -->|reusable Strategies| BUILD
    METHODS -->|reusable decompositions| BUILD

    BUILD --> CANDIDATES
    CANDIDATES --> AVAILABLE
    AVAILABLE -->|No| ERROR
    AVAILABLE -->|Yes| JUDGE
    D -->|authority and value posture| JUDGE
    JUDGE --> ADMISSION

    ADMISSION --> GOAL
    GOAL --> PLAN
    ADMISSION -->|authorized candidates| PLAN
    PLAN --> NETWORK
```

The center line is the product flow. PDS, current state, known Strategies, and verified Methods are inputs to particular stages. They are not themselves runtime stages.

## Canonical authority split

| Concern | Owns |
|---|---|
| PDS | state-free domain meaning |
| Graph | current objects, relations, lineage, and provenance |
| Belief | evidence admission, assessment, uncertainty, and revision |
| Causation and regime | effect and structural-context views |
| World-model planner projection | action-relevant world-model reads |
| Directive Agent | maintained intent, Goal drafts, value posture, and authorization |
| Strategy | candidate theories of action |
| Execution Planning | operational realization of authorized candidates |
| Task network | committed work and execution state |
| Events | semantic commitments and observed outcomes |

The Directive Agent may explicitly delegate Strategy construction to another Agent. Delegation must preserve the granting Agent, allowed scope, authority, objective, and revocation boundary.

Persistence custody does not confer semantic authority. Another domain may store a Strategy record without gaining authority to construct or approve its meaning.

## Why Strategy exists

A Goal draft states desired reality. It does not contain a causal theory for reaching that reality.

Execution capabilities expose possible operations. Their availability does not establish semantic relevance, evidentiary sufficiency, causal coherence, or authority.

Strategy supplies the missing semantic bridge:

```text
Goal draft
+ current evidence and causal projections
+ reusable Strategies and Methods
+ PDS action affordances
→ concrete candidate Compositions
```

Without this boundary, one of two failures occurs.

Execution begins interpreting domain meaning and becomes a second epistemic authority.

Alternatively, PDS packages prescribe complete workflows and become another procedural runtime language.

Strategy keeps domain meaning epistemic while allowing execution mechanics to remain generic.

## Relationship to PDS

Persistent Domain Stewardship supplies declarative operational domain theory.

It may declare or link:

- objects and relations
- observations and evidence routes
- belief dimensions and comparators
- maintained objectives
- semantic action affordances
- outcome verification
- authority requirements
- governance and value posture

PDS contains no current belief state, live Goal, Strategy decision, generated Composition, task, commitment, or outcome.

Strategy consumes a compiled and activated theory with exact package, profile, assignment, activation, and authority lineage. Strategy does not require one final PDS source schema. A central package, federated facets, or root-composed first proof may supply equivalent typed semantics.

The proposal corpus under [Persistent Domain Stewardship](../../../persistent_domain_stewardship/README.md) remains non-authoritative for PDS schema and lifecycle choices. This Strategy area is authoritative for the runtime boundary between domain theory, Agent judgment, and Execution Planning.

## Relationship to Agent

The Directive Agent owns Strategy judgment.

The Agent:

- grounds Directive-maintained conditions into concrete belief questions over trusted scope
- curates a Goal draft when reconciled belief diverges from the desired state
- supplies perspective and normative posture
- selects the decision context and normative relevance without overriding epistemic verdicts
- authorizes Strategy construction and any delegated Agent
- accepts, rejects, or supersedes the proposed Goal and Strategy decision together
- remains responsible for Goal curation and satisfaction curation

Strategy obligations are not Execution Goals. They are internal semantic nodes explaining why actions and edges belong in a candidate Composition.

Strategy cannot mutate Goal lifecycle. A Goal draft remains world-model curation state until Strategy presents at least one eligible candidate and the Agent authorizes admission. The initial handoff to Execution contains the nonempty authorized candidate inventory with the Goal. Later Goal changes, suspension, satisfaction, and abandonment remain Agent curation decisions submitted through Execution Goal APIs.

## Relationship to world-model planner projection

Strategy is not a second belief engine.

It consumes typed projections owned by graph, belief, causation, regime, and world-model planner. These may include:

- trusted scope projections
- belief revisions and applicability views
- relevance, evidence-admission, and sufficiency verdicts
- observation opportunities
- causal effect summaries
- efficacy and uncertainty projections
- precondition assessments
- regime sensitivity and risk envelopes
- validity horizons and source revisions

Strategy may request a candidate-intervention projection for a proposed Composition. The owning world-model domains remain authoritative for the resulting verdicts.

Strategy must not create private persistent posteriors for correctness, evidence sufficiency, causal effect, efficacy, uncertainty, or information gain.

## Relationship to Execution Planning

Strategy decides semantic approach. Execution Planning decides operational realization.

Strategy may:

- ground an authorized scope into internal obligations
- construct alternative causal Compositions
- select semantic action affordances
- justify dataflow and ordering edges
- require outcome verification
- request typed feasibility and efficacy projections
- present candidate alternatives and exact reuse conditions for Agent authorization

Execution Planning may:

- evaluate pure propositions against the referenced `WorldState`
- validate concrete candidate and Operator preconditions
- bind only Strategy-authorized variables
- reject stale, inapplicable, unavailable, or unauthorized candidates
- resolve operators to current capabilities
- apply exact Strategy-supplied reuse keys
- coordinate resources and other active plans
- schedule independent work concurrently
- lower the chosen Composition
- commit task-network mutations
- publish semantic planning commitments

Execution Planning must not:

- invent a new causal dependency
- redefine a domain objective
- decide that one evidence form substitutes for another
- add semantic work because it interpreted a Directive
- change claimed outcome meaning
- treat task success as Goal satisfaction

Missing or stale semantic proof causes rejection and renewed Strategy construction. It does not authorize Execution to repair the meaning itself.

## Composition and Method

An episode-specific Strategy normally produces a concrete `meld-lang::Composition`.

A `Composition` is a fixed graph for one Strategy decision. Episode object and topology bindings are ground. Any remaining operational binding slot must be explicit in the proposal. Every subgoal step resolves inside that proposal to an exact child Composition or an exact Method revision and bindings. Agent authorization covers the complete resolved candidate, so Execution never searches for novel subgoal meaning.

A `Method` is a reusable cached decomposition template. Strategy may separately propose a generalized Method when its derivation is independent of one episode. Execution owns Method verification, registration, namespace, visibility, revision, and quarantine.

```text
Strategy decision
  → one or more concrete Composition candidates
  → optional reusable Method proposal

Execution Method library
  ← only verified reusable Method revisions
```

One-off Compositions must not enter one undifferentiated global Method library.

## Known Strategy catalog

The names in this area describe different things:

| Name | Meaning |
|---|---|
| Known Strategy catalog | every reusable Strategy currently persisted |
| Available Strategy set | candidates that apply to one Goal draft in one world-model frame |
| Strategy alternative | one candidate with its evidence and justification |
| Composition | the concrete action graph inside that candidate |
| Method | a reusable Composition template verified and stored by Execution |

The known catalog is complete only for persisted reusable knowledge. It does not enumerate every Strategy Meld could construct.

A catalog entry minimally identifies:

```text
Strategy identity and revision
Goal pattern
PDS and authority lineage
reusable Method or Composition template references
prior selection and outcome references
availability posture
```

Execution retains custody of verified Method bodies. Catalog entries refer to exact Method revisions rather than duplicating them.

For one Goal draft, Strategy combines applicable catalog entries with novel episode-specific candidates. An empty catalog does not imply an empty available set.

Selection lineage preserves the catalog entry or novel candidate identity through Strategy decision, planning commitment, and outcome events. This is the minimal foundation for later curation that compares predicted efficacy with observed reality and prefers a previously successful Strategy when it remains applicable. The first implementation need not define a learned efficacy model.

### First implementation shape

The first implementation should remain deliberately small:

```text
one persisted known Strategy catalog
one Goal-pattern lookup
one bounded novel-construction path
one combined available candidate set
one explicit NoMethodAvailable result
one Goal admission bundle
one selected-Strategy to outcome association
```

It does not require learned ranking, generalized Strategy promotion, catalog governance, distributed consensus, or a complete Strategy lifecycle framework. Those remain replaceable until the basic cognitive path proves useful.

## Strategy flow

One bounded attempt has four phases:

```text
1. Understand the Goal draft against current world-model evidence.
2. Reuse known Strategies and Methods where they apply, then construct novel candidates as needed.
3. Validate and compare the resulting concrete Compositions.
4. Present eligible candidates to the Agent for judgment and Goal admission.
```

Construction is bounded by candidate count, expansion depth, search budget, elapsed budget, or an explicit combination.

Two empty-result cases must remain distinct:

| Result | When | Meaning |
|---|---|---|
| `NoMethodAvailable` | before initial Goal admission | Meld cannot connect the desired state to any eligible theory of action |
| Strategy abstention | after Goal admission | the Goal remains valid, but no useful action is available in the current state |

`NoMethodAvailable` is a visible runtime error and the Goal draft stays outside Execution. Abstention makes an active Goal quiescent until relevant state changes.

## Bounded convergence

Strategy participates in an open-ended convergence loop without hot-looping.

```text
observe
→ ground Directive into belief questions
→ reconcile belief
→ curate Goal draft
→ construct bounded Strategy
→ admit Goal with nonempty Strategy inventory
→ realize bounded work
→ observe outcome
→ reconcile again
```

Each Strategy attempt is bounded. Convergence is not limited to one attempt.

A below-threshold outcome may produce a new world-model verdict and a different Strategy decision. An unchanged input frame must not repeatedly generate equivalent work without new eligibility, invalidation, or retry posture.

## World change and planning commitment

A candidate Composition is counterfactual. Constructing or comparing it does not change world state and does not require a canonical event.

An Agent-authorized Strategy decision is a durable judgment but not yet an operational commitment.

An accepted task-network mutation is a real operational change. Execution must publish a semantic planning commitment through the atomic task-network outbox after acceptance.

```text
candidate Composition
  hypothetical

Strategy decision
  authorized theory of action

accepted task-network mutation
  operational commitment

task outcome
  observed execution result
```

A planning commitment records what Meld decided to attempt. It is not evidence that the intended domain effect occurred.

## Current implementation status

The active architecture is ahead of current implementation.

Current Meld has:

- typed Goals, Operators, Effects, Compositions, Methods, and WorldState
- loadable authored Methods
- first-applicable Method selection
- capability resolution
- composition lowering
- durable task-network mutation and execution
- event publication for task outcomes
- a narrow world-model planner projection
- Agent Goal curation

The minimum next slice does not yet have:

- Directive grounding into dynamically instantiated belief questions
- a Goal draft gate before Execution Goal admission
- a persisted known Strategy catalog
- a bounded Strategy constructor that combines reuse and novel construction
- a visible `NoMethodAvailable` result
- Goal admission with a nonempty authorized candidate set
- selected-Strategy to outcome association

The broader architecture also does not yet have:

- a general Agent-owned Strategy runtime
- a durable Strategy decision store or projection
- rich evidence-admission and sufficiency views for Strategy
- scoped universal Goal expansion
- semantic action affordance discovery
- Composition search over alternatives
- candidate-network efficacy projection
- durable Goal-to-Strategy association
- Strategy invalidation and replacement
- atomic proposal, Agent judgment, and Strategy event outboxes
- Agent authority and Goal lifecycle epoch fences at commitment and dispatch
- generic many-valued Composition fan-in
- semantic planning commitment publication

Current planning behavior must therefore be read as a configured-path first slice rather than the completed Strategy architecture.

## Documents

- [Strategy Requirements](requirements.md)
  normative behavior and boundary requirements
- [Strategy Contracts](contracts.md)
  shared records, lifecycle, lineage, durability, and event handoffs
- [Docs Freshness Strategy](docs_freshness.md)
  authoritative worked example and bottom-up fan-out viability proof

## Read with

- [World Model Agent](../agent/README.md)
- [Directive Grounding](../agent/directive_grounding.md)
- [World Model Planner](../planner/README.md)
- [World Model Belief](../belief/README.md)
- [Causal Layer](../causation/README.md)
- [Regime Layer](../regime/README.md)
- [Meld Lang](../../meld-lang/README.md)
- [Execution Planning](../../execution/planning/README.md)
- [Planning Pipeline](../../execution/planning/planning_pipeline.md)
- [Task Network](../../execution/task_network.md)
- [Persistent Domain Stewardship](../../../persistent_domain_stewardship/README.md)
