# Strategy Requirements

Date: 2026-07-23
Status: active
Scope: normative requirements for Agent-authorized Strategy construction

## Objective

Strategy must turn an Agent-curated Goal draft and authoritative world-model views into one or more causally justified concrete `meld-lang::Composition` candidates before that Goal may enter Execution. It must do so without transferring epistemic authority into Execution or procedural control into PDS.

## Core invariants

The detailed requirements preserve six simple rules:

1. A Goal draft is not yet an Execution Goal.
2. Initial Goal admission requires at least one eligible Strategy candidate.
3. Strategy searches both reusable knowledge and novel constructions.
4. The known Strategy catalog is not the complete space of possible Strategies.
5. `NoMethodAvailable` before admission is different from quiescence after admission.
6. Execution may realize or reject authorized meaning, but it may not invent new meaning.

The first implementation should prove these rules with one configured domain shape. It does not require learned ranking, automatic Strategy promotion, catalog governance, or a general Strategy language.

## Ownership requirements

### STR-001 Agent authority

Every Strategy decision must be authorized by a Directive Agent or an explicitly delegated Agent acting under a durable grant.

The authorization must identify the Goal draft, allowed scope, normative posture, authority boundary, delegation lineage, and revocation boundary.

### STR-002 No independent epistemic authority

Strategy must consume typed verdicts from graph, belief, causation, regime, domain evaluators, and world-model planner projection.

Strategy must not own a second evidence-admission engine, correctness comparator, causal model, efficacy posterior, uncertainty model, or information-gain model.

### STR-003 Persistence does not confer authority

The domain that stores Strategy records must not gain semantic authority merely through persistence custody.

Strategy decision authority remains attached to the authorizing Agent and any explicit delegation.

### STR-004 Goal lifecycle isolation

Strategy must not directly add, modify, satisfy, suspend, resume, abandon, or remove Execution Goals.

Before initial admission, the Goal draft remains world-model curation state. Strategy construction does not create an Execution Goal. The Agent may authorize an admission bundle only when the Strategy decision contains at least one eligible candidate.

After admission, Goal lifecycle changes remain Agent curation decisions submitted through Execution Goal APIs.

Internal Strategy obligations must not be represented as active Goals unless the Directive Agent separately authorizes them.

## Input requirements

### STR-010 Exact lineage

Every Strategy attempt must reference exact revisions for:

```text
Directive
Goal draft
PDS package or linked facet set
profile
assignment
activation
effective authority
capability catalog
verified Method inventory
execution operational projection when consumed
world-model frame
correctness or outcome theory
Agent and delegation grant
```

### STR-011 Trusted scope

Directive grounding, Belief Reconciliation, and graph projection must provide a typed scope and belief view at exact revisions.

The Directive Agent authorizes expansion of maintained belief questions over that scope. Strategy may expand a Goal draft into action obligations only after the relevant epistemic grounding is represented in the input frame.

Strategy must not ground work over stale, indeterminate, unauthorized, or unbounded scope.

### STR-012 Typed verdicts

Strategy inputs must distinguish:

```text
relevance
evidence admission
projected sufficiency
actual settled correctness
causal effect
efficacy
uncertainty
risk
authority
capacity
cost
```

One verdict must not silently substitute for another.

### STR-013 Operational projection

Strategy may read a projection of active Goals, accepted planning commitments, task progress, available capabilities, effective quotas, and reusable artifacts.

Execution remains authoritative for the underlying operational records.

Every operational value used for construction or ranking must be captured in the exact Strategy input frame and invalidation dependencies.

### STR-014 Known Strategy catalog

World-model Strategy must own one canonical persisted catalog of known reusable Strategies.

The catalog is the complete set of persisted reusable Strategy knowledge. It must not claim to enumerate every Strategy that can be constructed from PDS semantics.

Catalog entries must reference exact verified Method revisions or reusable Composition templates without duplicating Execution-owned Method bodies.

For one Goal draft, Strategy must derive an available set from applicable catalog entries and novel episode-specific candidates constructed during the bounded attempt.

### STR-015 Directive-grounded belief input

Goal drafts must derive from reconciled belief questions instantiated by Directive grounding or from explicit user-directed desired state.

Strategy must not invent missing persistent belief questions while constructing executable work. Missing epistemic grounding returns to the Directive Agent and belief domains.

## Construction requirements

### STR-020 Obligation graph

Strategy must expand the Goal draft desired state into an internal Strategy obligation graph before constructing executable work.

Every obligation must cite the domain-theory rule, Goal target, concrete subject, and world-model revision that introduced it.

### STR-021 Evidence paths

Strategy must discover evidence paths through typed graph relations, artifact coverage, provenance, freshness, and owning-domain verdicts.

A graph edge may establish relevance. It must not establish admission, sufficiency, or truth by itself.

### STR-022 Semantic action affordances

Strategy must construct work only from declared or imported semantic action affordances.

Each usable affordance must expose:

```text
subject and scope binding
planning preconditions
predicted effects
required and produced artifacts
evidence meaning
capacity constraints
authority class
outcome contract
cost and efficacy projection route
```

Capability implementation identity remains an Execution concern until resolution.

### STR-023 Means-end construction

Every selected action must have a causal role in satisfying one or more obligations.

Artifact type compatibility alone must not justify an action or edge.

### STR-024 Alternative construction

Strategy must retain multiple causally valid alternatives when available.

Different alternatives may use direct evidence, reusable settled artifacts, semantic compression, observation, intervention, or hybrid paths.

Candidate discovery must include applicable known Strategies and novel construction from PDS semantic affordances. An empty known Strategy catalog must not by itself produce `NoMethodAvailable`.

### STR-025 Concrete graph construction

Strategy must be able to construct a topology-specific concrete `Composition` after observation grounds the current subject set.

Independent branches must remain unordered. An ordering or dataflow edge requires an explicit semantic justification.

Every subgoal step must resolve inside the proposal to an exact child Composition or an exact reusable Method revision, bindings, and expanded Composition hash. Agent judgment authorizes the complete resolved candidate. Execution must not perform novel semantic search while lowering a subgoal.

### STR-026 Edge justification

Every Strategy-derived edge must cite its semantic dependency. Evidence and obligation-discharge edges require additional epistemic support.

```text
source and target obligation where applicable
producing action effect
required downstream precondition or input
capacity or authority fact where material
```

An evidence or obligation-discharge edge must also cite:

```text
ground graph relation
relevance verdict
admission verdict for existing evidence
projected-sufficiency verdict
prospective evidence contract and admission gate when the evidence does not yet exist
```

### STR-027 Candidate validation

Before authorization, a candidate must show:

```text
every required outcome has a producing action
every action precondition holds or is produced upstream
every dataflow input satisfies its capability and artifact contract
every existing evidence input is admitted
every future evidence input has a prospective contract and authoritative admission gate
every material obligation has an existing admitted discharge or a valid projected discharge path
projected effects can satisfy the Goal
required evaluator independence is preserved
scope and authority remain valid
```

Actual domain success remains unsettled until post-execution evidence is reconciled.

### STR-028 Prospective evidence

A future artifact used as epistemic evidence or obligation discharge may appear in a candidate only through a prospective artifact contract that identifies its producer, predicted semantic type, content-identity rule, outcome contract, admission authority, and downstream obligation.

That epistemic downstream use must be gated on a later authoritative admission verdict. Ordinary operational artifacts use capability and artifact contracts without acquiring epistemic authority. Execution may wait for and route an admission verdict but must not produce it.

## Projection and ranking requirements

### STR-030 Feasibility before preference

Strategy must reject candidates that violate semantic validity, evidence coverage, authority, capability availability, capacity, cost ceiling, or assignment scope before preference ranking.

Cost must not be used to prove causal coherence.

### STR-031 Bound candidate projection

Strategy must request or consume projections over the bound candidate graph rather than rely only on static operator literals.

The projection may include:

```text
probability of crossing the Goal threshold
expected critical-path time
expected provider and monetary cost
parallel resource demand
expected convergence turns
uncertainty
risk
expected information gain
```

### STR-032 Fan-out cost

Fan-out time must use critical-path and resource-contention reasoning rather than summing independent sibling duration.

### STR-033 Correctness posture

Correctness or another domain outcome remains a Goal condition or projected benefit. It must not be collapsed into ordinary resource cost.

The Agent value posture determines how viable candidates trade resolution speed, resource use, uncertainty, and risk.

## Product requirements

### STR-040 Strategy decision

An initial Strategy attempt for a Goal draft must produce either a candidate proposal with at least one eligible alternative or `NoMethodAvailable`.

A later attempt for an already admitted Goal may produce a candidate proposal or an explicit abstention record.

The proposal becomes judgeable only after its complete content-addressed closure has persisted atomically or every referenced record exists and hash-verifies.

Construction does not confer authorization. The Directive Agent or explicit delegate must accept a candidate proposal through a separate durable judgment before a `StrategyDecision` exists.

Initial Agent acceptance must recheck that the Goal draft remains current, delegation and effective authority remain valid, assignment and activation revisions remain current, proposal closure hash-verifies, and every selected alternative remains eligible.

A Strategy decision must contain:

```text
immutable decision identity and revision
candidate proposal reference
authorizing Agent and delegation lineage
Goal draft identity and revision
proposed Goal identity
exact input lineage
immutable Agent-authorized selection policy
one or more concrete Composition candidates
selected candidate when selection is authorized
edge-justification records
typed verdict references
assumptions
validity horizon
invalidation dependencies
outcome-contract references
```

Accepted Goal admission separately supplies the first Execution Goal revision and lifecycle epoch. The immutable pre-admission Strategy decision does not predict those values.

### STR-041 Concrete Composition

An episode-specific candidate must be represented as a concrete `meld-lang::Composition` with an owning Strategy wrapper.

The wrapper owns provenance and lifecycle. `meld-lang` remains pure and persistence-free.

### STR-042 Reusable Method proposal

Strategy may propose a reusable `meld-lang::Method` only when the derivation is generalized beyond the current episode and exact subject topology.

Execution owns Method verification, registration, namespace, visibility, revision, and quarantine.

### STR-043 Goal association

A durable association must connect the Goal draft, admitted Goal revision, and Strategy decision without embedding the Strategy payload in the Goal.

Initial admission must hand Execution the Goal and a nonempty authorized candidate inventory together. Execution must reject initial admission with an empty inventory.

### STR-044 Explanation

Inspection must explain why every candidate action and edge exists, which authoritative verdicts supported it, why alternatives were rejected, and which assumptions remain unresolved.

### STR-045 Outcome association

Strategy identity and selected candidate lineage must survive through the Strategy decision, planning commitment, and outcome event.

This lineage must support later curation that compares projected efficacy with observed reality and prefers a previously successful reusable Strategy when it remains applicable. The first implementation does not require a learned efficacy model.

## Boundedness and lifecycle requirements

### STR-050 Bounded attempts

Every Strategy attempt must be bounded by candidate count, expansion depth, search budget, elapsed budget, or an explicit combination.

### STR-051 Determinism and model use

The same deterministic inputs and Strategy version must produce the same result.

Model-backed or otherwise nondeterministic construction must record the model identity, prompt or policy asset, sampled output artifact, validation path, and replay posture.

### STR-052 Idempotency

Repeated processing of the same Goal draft or admitted Goal, input frame, Strategy version, and invalidation state must not produce duplicate active decisions or equivalent operational work.

### STR-053 Invalidation

Strategy must reconsider a decision when a declared dependency changes.

Dependencies may include graph topology, belief revision, evidence admission, capability catalog, verified Method inventory, activation, authority, capacity, efficacy, Goal draft or admitted Goal revision, or PDS semantics.

### STR-054 Supersession

Strategy decisions are append-only revisions. Supersession must preserve the prior decision, reason, source revisions, and relationship to any accepted planning commitment.

### STR-055 Abstention and quiescence

`NoMethodAvailable` applies before initial Goal admission when bounded construction cannot produce any eligible reusable or novel candidate. It is a visible runtime error and the Goal draft must remain outside Execution.

Abstention applies after Goal admission when no useful action is currently available for an active Goal.

An unsatisfied Goal remains `Active` while its Strategy association or convergence loop may become quiescent. Strategy must not hot-loop solely because the Goal remains active.

### STR-056 Wake conditions

New evidence, Goal revision, capability change, Method inventory change, authority change, expiry, outcome, or material operational change may wake Strategy when it changes eligibility or invalidates the current decision.

Abstention and its durable wake registration must commit atomically with the quiescent Strategy-association transition. Wake registration must preserve dependency references, source cursors, dependency epochs, and attempt generation so restart replays every relevant change without a lost-wake gap.

## Execution handoff requirements

### STR-060 Scoped visibility

Execution must receive initial Goals only through an Agent-authorized admission bundle with at least one candidate Composition visible to the current Goal, assignment, activation, and effective authority.

### STR-061 Exact mechanical tuning

Strategy must supply allowed bindings, artifact contracts, validity dependencies, world-frame requirements, reuse keys, and authorized alternatives.

Execution may tune only through exact contract matching and pure proposition evaluation.

### STR-062 Rejection

Execution must reject a candidate when its Strategy proof is missing, stale, inapplicable, unavailable, unauthorized, or incompatible with current task-network state.

Rejection returns typed operational facts to the Strategy and Agent loop. It does not authorize Execution to invent replacement meaning.

The existing Execution `NoApplicableMethod` result remains a mechanical boundary failure. When it occurs for a newly admitted Goal with an authorized candidate inventory, it signals stale or inconsistent realization rather than ordinary semantic absence.

### STR-063 Accepted commitment

An operational planning commitment exists only after a task-network mutation is accepted.

The commitment must identify:

```text
Strategy decision and candidate
Goal revision
Strategy alternative revision and Composition hash
complete Method-derivation inventory reference and hash
each used Method inventory revision, Method revision, content hash, template hash, bindings, parent step, and expanded Composition hash
world-model frame
task-network id
base revision and state hash
accepted commit id
accepted revision and state hash
mutation-set id
applied bindings
selected alternative
reuse and work-avoidance decisions
package, profile, assignment, and activation lineage
effective authority and capability-catalog revisions
Agent authority and Goal lifecycle epoch fences
```

### STR-064 Semantic event

Execution must publish an accepted planning commitment as a canonical semantic event through an atomic outbox created with the accepted task-network commit.

The event record identity must derive from the accepted commit identity so replay cannot duplicate it. The event records intent and operational commitment. It must not support Goal satisfaction or claimed domain success without separate outcome evidence.

### STR-065 Authority fence

Every Strategy decision must carry Agent authority and Strategy eligibility epoch fences. Accepted Goal admission must supply the initial Goal lifecycle epoch. Every planning request must carry all three.

Task-network commitment and every new dispatch claim must validate those fences through an authority-preserving linearizable contract. Authority revocation and Strategy invalidation advance their owning epochs. The Goal lifecycle epoch must advance on every lifecycle, replacement, or content transition that changes work eligibility, including activation, suspension, resume, satisfaction, reopening, abandonment, removal, supersession, and replacement. Every such transition serializes against commits and claims.

### STR-066 Evidence-admission ingestion

An authoritative evidence-admission verdict must enter the single-writer task network through an authority-preserving command or an equivalent persisted reducer input.

The accepted record must bind owning-domain revision, prospective contract, exact artifact content, subject, scope, schema, admission authority, and verdict identity. Dependency readiness may advance only after the accepted record is durable in the task-network revision stream. Replay must not depend on a mutable external read.

### STR-067 Planning-command uniqueness

For Strategy-originated task-network mutations, `network_id` plus the planning request idempotency key must be the reducer uniqueness key. Command identity must derive from that key or be rejected when the key already exists.

The reducer must atomically persist the uniqueness mapping, command result, graph commit, planning commitment, terminal planning response, and publication outbox. A retry with another caller-supplied command identity must return the prior result and must not duplicate work.

## Verification requirements

### STR-070 Boundary verification

Tests must prove that Strategy cannot write belief revisions, mutate Goal lifecycle, register capabilities, mutate the task network, or publish task outcomes.

Tests must prove that Execution cannot create semantic edges, substitute evidence, or reinterpret outcome criteria.

Tests must prove that an initial Goal cannot enter Execution without a nonempty Agent-authorized Strategy inventory.

### STR-071 Derivation verification

Each worked domain example must include metamorphic controls that change declared semantics, graph relations, authority, capacity, and evidence admission and then prove that candidate topology changes for the declared reason.

### STR-072 Replay verification

Exact input lineage and Strategy version must reconstruct the decision or identify the recorded nondeterministic artifact needed for replay.

### STR-073 Commitment verification

A Strategy decision without accepted task-network mutation must never appear as an operational planning commitment.

### STR-074 Outcome separation

Task completion, artifact creation, Strategy projection, and planning commitment must not independently satisfy the Goal.

Only Agent satisfaction curation over reconciled authoritative outcome evidence may close the Goal.

### STR-075 Generality

At least one non-documentation domain must construct a structurally similar Strategy from declared semantics without docs-specific Strategy branches or runtime grammar changes.

## Non-goals

Strategy is not:

- a second belief engine
- a PDS package compiler
- an Execution task planner under a new name
- a workflow language
- an authority-granting mechanism
- a capability implementation
- a task-network store
- a guarantee that predicted effects will occur
- a requirement that every Goal use model-backed construction
- a claim that the known Strategy catalog enumerates every constructible Strategy

## Read with

- [World Model Strategy](README.md)
- [Strategy Contracts](contracts.md)
- [Docs Freshness Strategy](docs_freshness.md)
- [World Model Agent](../agent/README.md)
- [Directive Grounding](../agent/directive_grounding.md)
- [World Model Planner](../planner/README.md)
- [Meld Lang](../../meld-lang/README.md)
- [Execution Planning](../../execution/planning/README.md)
