# Planner Entities

Date: 2026-05-14
Status: active
Scope: ECS entities for `world_model/planner` projections

## Thesis

Planner entities are deterministic projection records.

They are not free-form semantic artifacts.
Each entity must have a stable identity rule, a source authority, a lifecycle, and a reason to exist as an entity instead of a nested component value.

Every planner entity answers these questions:

- what lower-layer records can rebuild it
- what context makes it valid
- what downstream domain may read it
- what gap blocks implementation

## Entity Categories

| Category | Entities | Persistence expectation |
|---|---|---|
| Root query entity | `DecisionContext` | value object, cache key, optional audit record |
| Lens entity | `PlannerAgentLens` | read from agent domain, not owned by planner |
| Source packet entities | `PlannerGraphInput`, `PlannerBeliefInput`, `PlannerCausalInput`, `PlannerRegimeInput` | projection inputs, cacheable by source cursor set |
| View root entity | `WorldModelView` | read model result, materializable for replay and explanation |
| View child entities | `ActionableBeliefView`, `ObservationOpportunityView`, `PreconditionAssessment`, `CausalEffectSummary`, `RiskEnvelope`, `AbstentionState`, `ConflictSummary`, `SensitivitySummary`, `AssumptionSet` | child records under one view |
| Reference value | `HydrationHandle` | component value unless independently indexed |

## Identity Rules

| Entity | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `DecisionContext` | canonical hash of perspective, subject scope, branch scope, times, desired predicate, candidate intervention, and horizon | none | planner query contract |
| `ViewSnapshot` | canonical hash of source cursor set, projection version, reference time, transaction time, and expiry horizon | `WorldModelView` | planner projection |
| `PlannerAgentLens` | agent id plus lens version | none | agent domain |
| `PlannerGraphInput` | context id plus graph cursor set plus graph input version | `DecisionContext` | graph domain |
| `PlannerBeliefInput` | context id plus belief key plus belief revision id plus belief input version | `DecisionContext` | belief domain |
| `PlannerCausalInput` | context id plus causal claim or effect estimate ref plus causal input version | `DecisionContext` | causation domain |
| `PlannerRegimeInput` | context id plus active segment or regime posterior ref plus regime input version | `DecisionContext` | regime domain |
| `WorldModelView` | context id plus lens id plus source cursor set plus projection version | `DecisionContext` | planner domain |
| `ActionableBeliefView` | world model view id plus belief key plus predicate ref | `WorldModelView` | planner domain |
| `ObservationOpportunityView` | world model view id plus target evidence ref plus channel ref | `WorldModelView` | planner domain |
| `PreconditionAssessment` | world model view id plus predicate ref plus candidate intervention ref when present | `WorldModelView` | planner domain |
| `CausalEffectSummary` | world model view id plus causal claim ref or effect estimate ref | `WorldModelView` | planner domain |
| `RiskEnvelope` | world model view id | `WorldModelView` | planner domain |
| `AbstentionState` | world model view id | `WorldModelView` | planner domain |
| `ConflictSummary` | world model view id plus affected belief key plus conflict kind | `WorldModelView` | planner domain |
| `SensitivitySummary` | world model view id plus affected output ref plus source layer | `WorldModelView` | planner domain |
| `AssumptionSet` | world model view id | `WorldModelView` | planner domain |
| `HydrationHandle` | source layer plus domain object ref plus source id plus explanation role | any output entity | source layer plus planner projection |

## Context Entities

### `DecisionContext`

One scoped planning question.

Required fields:

- perspective
- subject scope
- branch scope
- reference time
- transaction time
- desired predicate
- candidate intervention
- decision horizon

Implementation requirement:
define canonical serialization before any cache, replay, or query route uses `DecisionContext`.

Open gap:
`SubjectScope`, `BranchScope`, `ReferenceTime`, `TransactionTime`, `BeliefPredicate`, `InterventionRef`, and `DecisionHorizon` need shared contracts.

### `ViewSnapshot`

One replay and validity boundary for a completed view.

Required fields:

- event cursor
- source cursor set by layer
- reference time
- transaction time
- projection version
- expiry horizon
- input packet ids

Implementation requirement:
`ViewSnapshot` must be attached to every `WorldModelView` and must be sufficient to rerun projection from the same lower records.

Open gap:
source cursor shape is not yet shared across graph, belief, causation, and regime.

### `PlannerAgentLens`

One perspective-specific admissibility lens.

Required fields:

- agent id
- lens version
- perspective
- trust profile
- evidence policy
- observation scope
- tolerated uncertainty
- regime sensitivity profile

Implementation requirement:
planner must receive the lens through an agent read port or by value in the query envelope.
Planner must not import agent internals.

Open gap:
agent lens contracts are not yet defined as typed public records.

## Input Packet Entities

### `PlannerGraphInput`

One graph packet scoped to a `DecisionContext`.

Carries:

- subject refs
- current anchor records
- lineage records
- relation neighborhood
- object history
- provenance bundles
- branch presence
- graph hydration handles
- graph source cursor

Implementation requirement:
graph input must be produced by a graph-owned projection or read adapter.
Planner may filter graph input for context, but planner must not derive truth, confidence, or causal support from graph input alone.

Open gap:
`ObjectHistory`, `BranchPresence`, and graph source cursor contracts are not fully exposed in current implementation.

### `PlannerBeliefInput`

One belief packet scoped to a `DecisionContext`.

Carries:

- belief key
- subject ref
- dimension
- posterior summary
- uncertainty summary
- precision summary
- freshness summary
- contradiction state
- origin state
- coverage state
- evidence refs
- assessment state
- observation opportunities
- belief source cursor

Implementation requirement:
belief input must preserve belief revision refs and evidence refs through every planner output that depends on them.

Open gap:
belief layer needs typed public records for posterior, uncertainty, precision, freshness, contradiction, origin, coverage, and assessment state.

### `PlannerCausalInput`

One causal packet scoped to a `DecisionContext`.

Carries:

- intervention variable
- target variable
- outcome variable
- effect estimate
- effect uncertainty
- identification status
- confounder risk
- selection warning
- counterfactual summary
- causal assumptions
- causal source cursor

Implementation requirement:
planner may summarize effect support, but must not estimate effects or choose adjustment sets.

Open gap:
causation layer needs read contracts for causal variables, effect estimates, identification status, confounder risk, selection warnings, and assumption refs.

### `PlannerRegimeInput`

One regime packet scoped to a `DecisionContext`.

Carries:

- regime posterior
- changepoint state
- run length belief
- continuation score
- break score
- mixture prediction
- sensitivity set
- stress metrics
- active segment status
- regime source cursor

Implementation requirement:
planner may use regime state for sensitivity, expiry, risk, assumptions, and abstention.
Planner must not decide the active regime.

Open gap:
regime layer needs typed read contracts for active segment state, mixture prediction, stress metrics, and sensitivity sets.

## Output Entities

### `WorldModelView`

One assembled planner-facing read model for a scoped question.

Required children:

- `ViewSnapshot`
- zero or more `ActionableBeliefView`
- zero or more `ObservationOpportunityView`
- zero or more `PreconditionAssessment`
- zero or more `CausalEffectSummary`
- one `RiskEnvelope`
- one `AbstentionState`
- zero or more `ConflictSummary`
- zero or more `SensitivitySummary`
- one `AssumptionSet`
- hydration handles needed for explanation and execution handoff

Implementation requirement:
`WorldModelView` is the only root output from `query_world_model_view`.
Other planner routes may return child projections, but they must be derivable from the same projection pipeline.

Open gap:
the public interface has route names but no typed response envelope with errors, warnings, cursor metadata, and partial result semantics.

### `ActionableBeliefView`

One belief projection for the scoped decision.

Required outcomes:

- supports
- blocks
- uncertain
- stale
- conflicted
- under-observed
- not assessed

Implementation requirement:
each view must name the predicate it affects and carry evidence refs, belief revision refs, decision relevance, and projection version.

Open gap:
decision relevance scoring needs an explicit scale, threshold basis, and tie-break rule.

### `ObservationOpportunityView`

One projected information-gathering opportunity.

Required outcomes:

- target evidence
- evidence channel
- expected information gain
- cost
- expiry
- blocking status
- dependency on belief, conflict, freshness, or coverage gap

Implementation requirement:
planner ranks opportunities but does not dispatch observation or choose execution method.

Open gap:
observation cost and channel refs need a cross-domain contract with execution capabilities.

### `PreconditionAssessment`

One assessment of world-facing conditions for a desired predicate or candidate intervention.

Required outcomes:

- supported
- blocked
- uncertain
- stale
- conflicted
- not assessed

Implementation requirement:
precondition assessment must stay world-facing.
Execution method readiness, resource readiness, retry rules, and dispatch policy remain outside planner.

Open gap:
the boundary between world-facing condition and execution method precondition needs a shared test fixture with execution.

### `CausalEffectSummary`

One planner-facing causal effect projection.

Required outcomes:

- supported effect
- weak effect support
- blocked identification
- confounded
- selection-shaped
- counterfactual unavailable

Implementation requirement:
preserve causal claim refs, adjustment assumptions, selection warnings, and counterfactual refs.

Open gap:
causal input records are design-only and need implementation contracts.

### `RiskEnvelope`

One composed typed risk entity for the view.

Required risk dimensions:

- belief uncertainty
- low precision
- stale evidence
- contradiction
- weak coverage
- causal identification
- confounder risk
- selection warning
- regime uncertainty
- changepoint risk
- stress sensitivity

Implementation requirement:
risk dimensions stay typed and traceable.
The planner must not collapse them into one authority score.

Open gap:
risk severity scale, blocking threshold, and mitigation hint vocabulary are undefined.

### `AbstentionState`

One explicit no-commit state.

Required outcomes:

- no abstention
- soft abstention
- hard abstention
- proceed under assumptions

Implementation requirement:
abstention must cite the source refs that caused it and must distinguish missing support from negative support.

Open gap:
execution needs a clear mapping from abstention state to planner handoff behavior.

### `ConflictSummary`

One contradiction and conflict projection.

Required conflict kinds:

- direct counterevidence
- supersession
- invalidation
- weak coverage
- competing hypothesis

Implementation requirement:
conflict blocks a decision only when it affects the scoped predicate or candidate intervention.

Open gap:
belief contradiction taxonomy needs stable typed values.

### `SensitivitySummary`

One structural fragility projection.

Required fields:

- affected output
- source layer
- flip condition
- weakening condition
- expiry rule
- stress scenario refs

Implementation requirement:
sensitivity must state what lower-layer change would materially alter the view.

Open gap:
regime sensitivity and stress metrics need a threshold vocabulary.

### `AssumptionSet`

One explicit dependency set for the view.

Required assumption kinds:

- causal
- regime
- coverage
- freshness
- evidence policy
- scope

Implementation requirement:
every assumption must state validity scope, source refs, violation effect, and expiry rule.

Open gap:
assumption refs need shared formatting across causal, regime, belief, and planner records.

### `HydrationHandle`

One stable reference to lower-layer source records.

Use as a component value by default.
Promote to entity only if handles become independently indexed, audited, or dereferenced outside the owning output.

Required fields:

- source layer
- domain object ref
- source id
- source cursor
- projection path
- explanation role

Implementation requirement:
hydration handles support explanation and replay.
They do not substitute for typed projection fields.

Open gap:
hydration handle dereference routes are not defined in the public interface.

## Entity Rules

- Every planner output must be derivable from explicit lower-layer records and projection version.
- Every planner output must include `ViewSnapshot` reachability.
- Child entities must carry parent `WorldModelView` identity.
- Input packet entities must carry source cursor boundaries.
- Score concepts such as `ExpectedInformationGain` and `DecisionRelevance` are components or fields, not entities.
- `ObservationPolicy` may exist inside projection internals, but the public planner output is `ObservationOpportunityView`.
- `ExecutionPreconditions` should be expressed as `PreconditionAssessment`, because execution owns operational readiness.
- `HydrationHandle` is not a root entity unless implementation needs independent indexing.

## Implementation Gap Register

| Gap | Blocks | Needed owner |
|---|---|---|
| canonical `DecisionContext` serialization | cache keys, replay keys, route idempotence | planner |
| source cursor set format | deterministic replay across layers | graph, belief, causation, regime |
| typed agent lens contract | perspective-scoped admissibility | agent |
| typed belief view contract | belief input projection | belief |
| typed causal output contract | causal effect summary and abstention | causation |
| typed regime output contract | risk, sensitivity, expiry | regime |
| hydration dereference routes | explanation and execution handoff | public interface |
| decision relevance scale | ranking and blocking | planner |
| risk severity scale | abstention and execution handoff | planner |
| partial result semantics | robust query behavior during missing lower-layer data | public interface |

## Read With

- [Planner Components](components.md)
- [Planner Systems](systems.md)
- [World Model Planner](README.md)
