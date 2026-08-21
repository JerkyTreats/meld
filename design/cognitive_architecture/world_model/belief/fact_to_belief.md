# Fact To Belief

Scope: transition from Events and graph publications into belief revisions and planner views

## Thesis

A fact is immutable history.
A belief is a settled or provisional claim over that history.

Planner consumes shaped belief and world-model views.
It does not inspect raw Events during projection.
Source facts remain available for Strategy explanation, evidence gathering, audit, and provenance.

Fact to belief is a world model concern.
Belief view to action is an agent concern.

Belief supports bottom-up evidence, top-down predictions, latent hypotheses, precision-weighted evidence, observation policy, and regime-conditioned priors through one revision contract.

## Domain Inputs

Belief consumes these canonical contracts:

- [Events](../../events/README.md)
  durable append, replay, cursor, and producer authority
- [World Model Graph](../graph/README.md)
  owner-issued objects, relation occurrences, scope, provenance, and traversal cuts
- [Execution Domain](../../execution/README.md)
  execution publishes outcomes and observations without reading belief as an admission prerequisite

## Record Roles

`SpineFact`

Immutable semantic event with sequence, domain, stream, object refs, relation edges, and payload.

`GraphAnchor`

Current selected surface for a subject and perspective.
An anchor says what is current.
It does not say whether that current value should be trusted.

`EvidenceItem`

Belief-normalized record derived from one or more facts, anchors, or execution outcomes.
It carries subject, predicate, value, polarity, source, sequence range, reliability or precision where available, and provenance.

`BeliefKey`

Stable identity for the question being settled.
The key is the planner-visible unit of belief assessment.
It may include or reference perspective when different agents, branches, or evidence policies can reasonably produce different current views.

`BeliefRevision`

Append-only settlement result for one belief key.
It records comparator or inference method used, input evidence set, posterior summary, uncertainty, status, supersession, and provenance.

`BeliefView`

Planner-facing materialized projection.
It hides raw fact churn and exposes planner-readable settlement state.

`BeliefView` is the first belief microarchitecture boundary.
The world model publishes it.
The agent consumes it.

## Transition

The transition from fact to belief has six stages.

- promotion
  accept a graph or execution event as belief-relevant evidence
- normalization
  turn the event into an `EvidenceItem`
- assignment
  attach evidence to one or more `BeliefKey` values
- assessment
  run the selected comparator or inference method over the evidence set
- revision
  append a `BeliefRevision` and supersede the prior revision if needed
- projection
  update the planner-facing `BeliefView`

This is the minimum path. Hierarchical belief later adds prediction and message flow:

- prediction
  emit expected evidence, lower-level state, or outcome distribution from the current posterior
- comparison
  compare observed evidence with prediction using an appropriate likelihood or residual form
- epistemic escalation
  create an observation opportunity when uncertainty or conflict remains decision-relevant

## Graph Anchor To Belief Revision

```mermaid
flowchart LR
    A[spine fact] --> B[graph reducer]
    B --> C[graph anchor selected]
    C --> D[anchor provenance]
    D --> E[evidence item]
    E --> F[belief key assignment]
    F --> G[comparator]
    G --> H[belief revision]
    H --> I[belief view]
```

The anchor selects the current target.
The evidence item explains why the anchor matters to a belief.
The belief revision records whether the current target is trusted, contradicted, stale, provisional, invalid, or uncertain under competing hypotheses.

## Spine To Belief View

```mermaid
flowchart LR
    A[canonical spine] --> B[belief ingestor]
    B --> C[evidence store]
    C --> D[assessment queue]
    D --> E[comparator worker]
    E --> F[belief revision store]
    F --> G[planner belief view]
    F --> H[operator inspection view]
```

This loop keeps the spine as the durable source.
The belief view is current state derived from replay.
The world model stops at shaped views.

## Belief View To Action

```mermaid
flowchart LR
    A[planner belief view] --> B[agent planner]
    B --> C[task construction]
    C --> D[source fact hydration]
    D --> E[capability invocation]
    E --> F[execution outcome]
    F --> G[canonical spine]
```

The agent loop may hydrate facts after it selects an action.
That is task construction, not belief settlement.

## Evidence Mapping

Many facts may support one belief.
One fact may also affect many beliefs.

The mapping must therefore be explicit:

- `source_fact_id` links evidence to spine history
- `anchor_id` links evidence to current graph state when available
- `belief_key` links evidence to the assessed question
- `evidence_role` marks support, contradiction, context, calibration, or supersession
- `effective_seq_range` marks which fact window was assessed

Belief graph relation changes over time by appending revisions.
It should not mutate old evidence edges in place.

## Planner Boundary

Planner reads:

- belief status
- posterior summary
- confidence or precision
- uncertainty
- freshness
- contradiction state
- observation-needed state
- assessment state
- provenance summary

Planner does not read:

- raw spine event payloads
- raw graph reducer internals
- transient comparator work state
- unpublished evidence churn

Task construction may hydrate source facts after planning chooses an action.
That keeps planning semantic while allowing execution to build concrete capability inputs.

## Microarchitecture Placement

World model belief owns:

- runtime belief family configuration loading and validation
- evidence normalization
- belief key assignment
- comparator scheduling
- belief revision
- belief view projection
- provenance over belief revisions
- uncertainty, precision, and freshness summaries
- observation opportunities tied to belief uncertainty

Agent owns:

- goal policy
- planner decisions
- task construction
- fact hydration for task inputs
- capability invocation
- outcome publication

Spine owns:

- durable fact append
- replay
- subscription
- sequence
- cross-domain refs

## Read With

- [Belief](README.md)
- [Belief Microarchitecture](microarchitecture.md)
- [Comparator Model](comparator_model.md)
- [Belief Substrate](substrate.md)
- [Graph](../graph/README.md)
- [Execution Domain](../../execution/README.md)
