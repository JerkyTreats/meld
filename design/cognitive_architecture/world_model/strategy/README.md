# World Model Strategy

Date: 2026-07-22
Status: active
Scope: Agent-authorized semantic construction of evidence-backed action Compositions

## Thesis

`world_model/strategy` owns the transition from desired reality to an evidence-backed theory of action.

A Directive Agent decides what should be maintained. World-model domains determine what is currently believed, what evidence is admissible, what remains uncertain, and what causal or efficacy claims are justified. Strategy combines those authoritative views with operational domain theory and constructs one or more concrete `meld-lang::Composition` values believed capable of satisfying an authorized Goal.

Execution does not reinterpret that meaning. It validates current applicability, resolves operators to capabilities, tunes authorized alternatives against live operational state, lowers the selected Composition, and commits task-network mutations.

```text
PDS operational domain theory
+ current world-model views
+ Directive Agent Goal and value posture
+ semantic action affordances
→ Agent-authorized Strategy decision
→ concrete Composition candidates
→ Execution realization
→ task-network commitment
```

Strategy is part of the world model because causal adequacy, evidence sufficiency, uncertainty, and intended semantic effect must remain under epistemic and Agent authority. Execution remains authoritative for operational commitment.

## Canonical authority split

```text
PDS
  declares state-free operational domain theory

Graph
  owns current objects, relations, lineage, and provenance

Belief
  owns evidence admission, assessment, uncertainty, and revision

Causation and regime
  own justified effect and structural-context views

World-model planner projection
  assembles typed action-relevant verdicts

Directive Agent
  owns desired state, normative posture, and Strategy authority

Strategy
  constructs semantic theories of action under Agent authority

Execution Planning
  realizes authorized theories against operational reality

Task network
  persists and executes committed work

Events
  record semantic commitments, attempts, and outcomes
```

The Directive Agent may explicitly delegate Strategy construction to another Agent. Delegation must preserve the granting Agent, allowed scope, authority, objective, and revocation boundary.

Persistence custody does not confer semantic authority. Another domain may store a Strategy record without gaining authority to construct or approve its meaning.

## Why Strategy exists

A Goal states desired reality. It does not contain a causal theory for reaching that reality.

Execution capabilities expose possible operations. Their availability does not establish semantic relevance, evidentiary sufficiency, causal coherence, or authority.

Strategy supplies the missing semantic construction:

```text
desired proposition
→ grounded obligations
→ admitted evidence paths
→ semantic action alternatives
→ causally justified dependency graph
→ concrete Composition candidates
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

- authorizes the Goal or Goal-linked scope expansion
- supplies perspective and normative posture
- selects the decision context and normative relevance without overriding epistemic verdicts
- authorizes Strategy construction and any delegated Agent
- accepts, rejects, or supersedes Strategy decisions
- remains responsible for Goal curation and satisfaction curation

Strategy obligations are not Execution Goals. They are internal semantic nodes explaining why actions and edges belong in a candidate Composition.

Strategy cannot mutate Goal lifecycle. New ground Goals, Goal changes, suspension, satisfaction, and abandonment remain Agent curation decisions submitted through Execution Goal APIs.

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

## Strategy flow

One bounded Strategy attempt follows this semantic flow:

```text
1. Read an Agent-authorized Goal and exact lineage.
2. Read a trusted world-model scope and verdict snapshot.
3. Expand the desired state into internal obligations.
4. Discover admitted evidence and semantic action candidates.
5. Construct causally valid Composition alternatives.
6. Request typed feasibility, efficacy, and risk projections.
7. Rank viable alternatives under the Agent value posture.
8. Record a candidate proposal or abstention.
9. Record the Agent authorization as a Strategy decision.
10. Hand the decision to Execution Planning.
```

Construction is bounded by candidate count, expansion depth, search budget, elapsed budget, or an explicit combination.

No eligible candidate produces abstention. The unsatisfied Goal remains `Active` while its Strategy association becomes quiescent until new evidence, capability availability, authority, or world state makes useful action available.

## Bounded convergence

Strategy participates in an open-ended convergence loop without hot-looping.

```text
observe
→ reconcile belief
→ curate Goal
→ construct bounded Strategy
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

Current Meld does not yet have:

- an Agent-owned Strategy runtime
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
- [World Model Planner](../planner/README.md)
- [World Model Belief](../belief/README.md)
- [Causal Layer](../causation/README.md)
- [Regime Layer](../regime/README.md)
- [Meld Lang](../../meld-lang/README.md)
- [Execution Planning](../../execution/planning/README.md)
- [Planning Pipeline](../../execution/planning/planning_pipeline.md)
- [Task Network](../../execution/task_network.md)
- [Persistent Domain Stewardship](../../../persistent_domain_stewardship/README.md)
