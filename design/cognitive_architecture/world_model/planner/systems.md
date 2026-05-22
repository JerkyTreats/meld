# Planner Systems

Date: 2026-05-14
Status: active
Scope: projection systems for `world_model/planner`

## Thesis

Planner systems are deterministic projection functions.

They read lower-layer input packets and planner context, then write planner-facing entities and components.
They do not settle belief, estimate causal effects, detect regimes, curate goals, dispatch tasks, or repair execution.

## Pipeline Overview

| Phase | System | Required input | Required output |
|---|---|---|---|
| 1 | decision-context normalization | `DecisionContext`, optional query envelope | normalized context, context id |
| 2 | agent-lens acquisition | normalized context | `PlannerAgentLens` |
| 3 | lower-input acquisition | normalized context, lens | graph, belief, causal, regime input packets |
| 4 | source packet validation | input packets, source cursor set | accepted packets, warnings, missing packet markers |
| 5 | belief projection | belief packet, graph packet, context, lens | belief views, preconditions, conflicts, observation opportunities |
| 6 | causal projection | causal packet, context, lens | causal effect summaries, causal assumptions, causal risks |
| 7 | regime projection | regime packet, context, lens | regime risks, sensitivity, expiry, regime assumptions |
| 8 | risk and assumption composition | belief, causal, regime outputs | risk envelope, assumption set |
| 9 | abstention projection | risk, conflict, assumptions, source warnings | abstention state |
| 10 | hydration projection | all accepted source refs and outputs | hydration handle set |
| 11 | view snapshot projection | source cursor set, expiry, projection version | `ViewSnapshot` |
| 12 | planner-view assembly | all child outputs | `WorldModelView` |

Replay invariant:
same normalized context, same lens, same lower-layer source cursor set, and same projection version produce the same `WorldModelView`.

## Required Read Ports

| Port | Owner | Required for | Gap |
|---|---|---|---|
| graph packet read | graph | identity, anchors, lineage, provenance, branch presence, hydration | object history and branch presence are not fully exposed |
| belief packet read | belief | posterior, uncertainty, freshness, contradiction, coverage, observation opportunities | typed belief view contract needed |
| causal packet read | causation | intervention effects, identification, confounders, selection warnings | causal public read contract needed |
| regime packet read | regime | regime uncertainty, changepoints, sensitivity, stress metrics | regime public read contract needed |
| agent lens read | agent | admissibility, trust, tolerance, observation scope | typed lens contract needed |
| source cursor read | each source layer | replay and view snapshot | shared cursor shape needed |

Planner systems must depend on these ports.
They must not import lower-layer internals.

## Public Route Mapping

| Route | Pipeline behavior |
|---|---|
| `query_world_model_view` | run full pipeline and return `WorldModelView` |
| `query_observation_opportunities` | run full pipeline, return opportunity children and shared snapshot metadata |
| `query_preconditions` | run full pipeline, return precondition children and shared snapshot metadata |

Route rule:
partial routes do not use separate projection logic.
They select from the same deterministic pipeline so behavior remains consistent.

## Projection Table

| Projection | Owning domain | System | Output |
|---|---|---|---|
| `DecisionContext -> normalized context` | planner | decision-context normalization | scoped admissibility basis |
| `normalized context -> PlannerAgentLens` | agent | agent-lens acquisition | `PlannerAgentLens` |
| `GraphInput -> PlannerGraphInput` | graph | graph-input projection | `PlannerGraphInput` |
| `BeliefView -> PlannerBeliefInput` | belief | belief-input projection | `PlannerBeliefInput` |
| `CausalOutput -> PlannerCausalInput` | causation | causal-input projection | `PlannerCausalInput` |
| `RegimeOutput -> PlannerRegimeInput` | regime | regime-input projection | `PlannerRegimeInput` |
| `PlannerBeliefInput + DecisionContext + PlannerAgentLens -> ActionableBeliefView` | planner | actionable-belief projection | `ActionableBeliefView` |
| `PlannerBeliefInput + PlannerGraphInput + DecisionContext -> PreconditionAssessment` | planner | precondition assessment projection | `PreconditionAssessment` |
| `PlannerBeliefInput + DecisionContext -> ObservationOpportunityView` | planner | observation opportunity projection | `ObservationOpportunityView` |
| `PlannerBeliefInput + DecisionContext -> ConflictSummary` | planner | conflict-summary projection | `ConflictSummary` |
| `PlannerCausalInput + DecisionContext -> CausalEffectSummary` | planner | causal-effect projection | `CausalEffectSummary` |
| `PlannerBeliefInput + PlannerCausalInput + PlannerRegimeInput + DecisionContext + PlannerAgentLens -> RiskEnvelope` | planner | risk-envelope projection | `RiskEnvelope` |
| `PlannerRegimeInput + planner outputs -> SensitivitySummary` | planner | sensitivity-summary projection | `SensitivitySummary` |
| `PlannerBeliefInput + PlannerCausalInput + PlannerRegimeInput + DecisionContext -> AssumptionSet` | planner | assumption-set projection | `AssumptionSet` |
| `RiskEnvelope + ConflictSummary + AssumptionSet + source warnings -> AbstentionState` | planner | abstention projection | `AbstentionState` |
| `all admitted source refs -> HydrationHandle set` | planner | hydration-handle projection | hydration handles |
| `source cursor set + expiry + projection version -> ViewSnapshot` | planner | view-snapshot projection | `ViewSnapshot` |
| `all planner projections -> WorldModelView` | planner | planner-view assembly | `WorldModelView` |
| `WorldModelView -> ExecutionPlanningInput` | execution | execution planning input projection | execution port input |
| `TaskLifecycleEvent + Artifacts + ExecutionContext -> OutcomeFact` | execution | outcome publication projection | `OutcomeFact` |
| `OutcomeFact -> EvidenceItem` | belief | outcome evidence projection | `EvidenceItem` |
| `BeliefRevision + PlannerAgentLens + ActiveGoals + CostBeliefs + ValueBeliefs + RegimeContext -> GoalMutation` | agent | goal curation projection | `GoalMutation` |

## Decision-Context Normalization

Input:
`DecisionContext`.

Output:
normalized context, context id, and route warnings.

Rules:

- Canonicalize subject scope.
- Canonicalize branch scope.
- Require reference time and transaction time.
- Validate that desired predicate and candidate intervention are compatible when both are present.
- Validate that broad survey routes are explicitly marked when no desired predicate exists.
- Produce deterministic context id.

Failure behavior:
invalid context returns a route error before lower-layer reads.

Implementation gap:
canonical serialization for `DecisionContext` is undefined.

## Agent-Lens Acquisition

Input:
normalized context.

Output:
`PlannerAgentLens`.

Rules:

- Resolve lens by perspective and agent id when agent id is present.
- Validate lens perspective against context perspective.
- Apply evidence policy before scoring decision relevance.
- Apply tolerated uncertainty only after source summaries are available.
- Carry lens id and lens version into output provenance.

Failure behavior:
missing required lens returns route error unless the route supplies an explicit default lens.

Implementation gap:
agent lens records and default lens policy are undefined.

## Lower-Input Acquisition

Input:
normalized context and lens.

Output:
planner input packets and missing packet markers.

Rules:

- Request graph input for subject scope, branch scope, and reference boundary.
- Request belief inputs for relevant subjects, dimensions, and predicate scope.
- Request causal inputs only when candidate intervention or causal predicate requires them.
- Request regime inputs for subjects and decision horizon.
- Request source cursors with every packet.

Failure behavior:
missing optional packet becomes explicit absence.
missing required packet becomes warning or abstention depending on route and context.

Implementation gap:
source domains do not yet expose all packet reads.

## Source Packet Validation

Input:
input packets and source cursor set.

Output:
accepted packets, warnings, and missing packet markers.

Rules:

- Reject packets outside subject scope.
- Reject packets outside branch scope.
- Reject packets outside reference time or transaction time.
- Reject packet versions unsupported by projection version.
- Preserve missing comparator, stale view, invalid view, unresolved mixture, blocked identification, and source cursor absence as typed warnings.

Failure behavior:
invalid source packet does not silently disappear.
It becomes route error, warning, or abstention input.

Implementation gap:
version negotiation between planner and lower source packets is not specified.

## Belief-Input Projection

Input:
belief views, belief revisions, evidence refs, assessment state, and observation opportunities.

Output:
`PlannerBeliefInput`.

Rules:

- Select belief views matching subject scope, perspective, branch scope, and decision horizon.
- Normalize posterior, uncertainty, precision, freshness, contradiction, origin, and coverage into planner components.
- Preserve evidence refs and belief revision refs as hydration handles.
- Keep `ObservationOpportunity` as source-layer output until planner filters it into `ObservationOpportunityView`.
- Mark missing comparator, provisional assessment, stale view, and invalid view explicitly.

Replay invariant:
same belief state and projection version produce the same belief input packet.

Implementation gap:
belief source records and public view contracts are not yet available in code.

## Graph-Input Projection

Input:
graph anchors, lineage, relation walks, object history, provenance, branch presence, and source cursors.

Output:
`PlannerGraphInput`.

Rules:

- Select graph records in subject scope.
- Preserve lineage and provenance for every current anchor used by the view.
- Include relation neighborhood only to the depth and filters required by `DecisionContext`.
- Emit hydration handles for every graph record needed by execution or explanation.
- Do not infer confidence, truth status, causal effect, or regime status from graph state alone.

Replay invariant:
same graph state and as-of boundary produce the same graph input packet.

Implementation gap:
the graph implementation exposes anchors, walks, and provenance, but object history, branch presence, and source cursor envelopes need public route shape.

## Actionable-Belief Projection

Input:
`PlannerBeliefInput`, `DecisionContext`, and `PlannerAgentLens`.

Output:
`ActionableBeliefView`.

Rules:

- Map posterior summary to supported, unsupported, uncertain, mixed, stale, conflicted, under-observed, or not assessed.
- Downgrade support when freshness is stale for the decision horizon.
- Downgrade support when precision is below agent tolerance.
- Mark conflict when contradiction state affects the scoped predicate.
- Mark under-observed when coverage cannot license the predicate.
- Carry evidence refs, belief revision refs, decision relevance, and projection version.

Abstention handoff:
emit abstention input when critical uncertainty, stale evidence, unresolved contradiction, invalid assessment, or insufficient coverage blocks commitment.

Implementation gap:
decision relevance scoring needs scale and threshold rules.

## Precondition Assessment Projection

Input:
`PlannerBeliefInput`, `PlannerGraphInput`, and `DecisionContext`.

Output:
`PreconditionAssessment`.

Rules:

- Evaluate desired predicate or candidate intervention condition against scoped belief state.
- Use graph input only for identity, relation scope, and hydration.
- Return supported, blocked, uncertain, stale, conflicted, or not assessed.
- Treat contradiction as blocking when it targets the required condition.
- Treat weak coverage as uncertain unless a local completeness rule licenses absence.
- Preserve predicate refs and source belief refs.

Boundary:
planner assesses world-facing conditions.
execution assesses method readiness, resource readiness, retry rules, dispatch, and repair.

Implementation gap:
shared fixtures are needed to keep planner preconditions and execution readiness from overlapping.

## Observation Opportunity Projection

Input:
`PlannerBeliefInput` and `DecisionContext`.

Output:
`ObservationOpportunityView`.

Rules:

- Admit only opportunities linked to decision-relevant uncertainty, freshness, conflict, or coverage gaps.
- Carry expected information gain as a score component.
- Carry cost, expiry, target evidence, evidence channel, and blocking status.
- Rank opportunities by decision relevance and expected information gain after admissibility filtering.
- Do not dispatch observation or choose an execution method.

Implementation gap:
evidence channel refs and observation cost records need a shared contract with execution capabilities.

## Conflict-Summary Projection

Input:
`PlannerBeliefInput` and `DecisionContext`.

Output:
`ConflictSummary`.

Rules:

- Group contradictions by affected belief key and decision predicate.
- Distinguish direct counterevidence, supersession, invalidation, weak coverage, and competing hypothesis conflict.
- Mark conflict as blocking only when it affects the scoped decision.
- Preserve competing evidence refs and hydration handles.

Implementation gap:
belief contradiction taxonomy needs stable typed values.

## Causal-Input Projection

Input:
causal variables, effect estimates, intervention records, outcome links, counterfactual cases, confounder hypotheses, and assumptions.

Output:
`PlannerCausalInput`.

Rules:

- Select causal outputs relevant to candidate intervention, target variable, outcome variable, and decision horizon.
- Preserve identification status and confounder risk.
- Preserve selection and measurement warnings.
- Carry causal assumptions as refs, not prose authority.
- Do not re-estimate causal effects inside planner.

Replay invariant:
same causal source records and projection version produce the same causal input packet.

Implementation gap:
causal source records are design-only and need public read contracts.

## Causal-Effect Projection

Input:
`PlannerCausalInput` and `DecisionContext`.

Output:
`CausalEffectSummary`.

Rules:

- Project effect direction, effect magnitude, uncertainty, and identification status.
- Mark weak support when identification status is blocked or confounder risk is material.
- Mark selection-shaped evidence when measurement or anchor selection may explain the outcome.
- Preserve counterfactual summary when available.
- Preserve causal assumptions and source refs.

Implementation gap:
effect support categories and blocked identification semantics need typed values.

## Regime-Input Projection

Input:
regime posterior, changepoint state, run length belief, continuation score, break score, mixture prediction, sensitivity set, stress metrics, and active segment state.

Output:
`PlannerRegimeInput`.

Rules:

- Select regime outputs relevant to the perspective, subject scope, and decision horizon.
- Preserve unresolved mixture state.
- Preserve active segment status and changepoint uncertainty.
- Preserve stress metrics only when they affect the scoped decision.
- Do not decide regime identity inside planner.

Replay invariant:
same regime source records and projection version produce the same regime input packet.

Implementation gap:
regime source records are design-only and need public read contracts.

## Risk-Envelope Projection

Input:
`PlannerBeliefInput`, `PlannerCausalInput`, `PlannerRegimeInput`, `DecisionContext`, and `PlannerAgentLens`.

Output:
`RiskEnvelope`.

Rules:

- Create separate risk dimensions for belief uncertainty, low precision, staleness, contradiction, weak coverage, causal identification, confounder risk, selection warning, regime uncertainty, changepoint risk, and stress sensitivity.
- Preserve source layer and source refs for each risk dimension.
- Mark a risk as blocking only when it violates decision context or agent tolerance.
- Do not collapse typed risk dimensions into a single authority score.

Implementation gap:
risk severity scale and blocking thresholds are undefined.

## Sensitivity-Summary Projection

Input:
`PlannerRegimeInput` and relevant planner outputs.

Output:
`SensitivitySummary`.

Rules:

- Identify regime changes that would flip belief support, causal effect support, risk level, or abstention state.
- Include stress metrics that cross planner-relevant thresholds.
- Emit expiry horizon when regime uncertainty makes the view time-sensitive.
- Preserve regime refs and stress scenario refs.

Implementation gap:
flip and weakening condition vocabulary is undefined.

## Assumption-Set Projection

Input:
`PlannerCausalInput`, `PlannerRegimeInput`, `PlannerBeliefInput`, and `DecisionContext`.

Output:
`AssumptionSet`.

Rules:

- Record causal assumptions used by effect summaries.
- Record regime assumptions used by risk and sensitivity summaries.
- Record coverage and freshness assumptions used by belief and precondition projections.
- Record evidence policy assumptions from agent lens.
- Record scope assumptions from decision context.
- State violation effect for each assumption.

Implementation gap:
assumption refs and violation effects need typed records.

## Abstention Projection

Input:
`RiskEnvelope`, `ConflictSummary`, `AssumptionSet`, accepted source packet warnings, and missing packet markers.

Output:
`AbstentionState`.

Rules:

- Emit hard abstention when required belief is invalid, critical evidence is missing, contradiction blocks the decision, causal identification is blocked for a required intervention, or regime uncertainty invalidates the decision horizon.
- Emit soft abstention when action may proceed only under explicit assumptions.
- Emit no abstention when all blocking risks are absent or below tolerance.
- Preserve source refs that caused abstention.
- Distinguish missing support from negative support.

Implementation gap:
execution handoff semantics for hard and soft abstention are not yet specified.

## Hydration-Handle Projection

Input:
all source refs admitted into planner projection.

Output:
hydration handles attached to planner outputs.

Rules:

- Preserve domain object refs and source ids.
- Record source cursor and projection path.
- Record why the handle exists, such as evidence, provenance, explanation, execution hydration, or replay.
- Deduplicate handles by source identity and explanation role.
- Never use hydration handles as substitute authority for typed projection fields.

Implementation gap:
hydration dereference routes are not defined.

## View-Snapshot Projection

Input:
source cursors, expiry candidates, input packet ids, and projection version.

Output:
`ViewSnapshot`.

Rules:

- Record reference time and transaction time from `DecisionContext`.
- Record event cursor or source cursor set used by lower inputs.
- Record projection version.
- Record input packet ids.
- Record expiry horizon from freshness, observation opportunity, regime sensitivity, and causal assumption expiry.

Implementation gap:
source cursor set shape is undefined.

## Planner-View Assembly

Input:
all planner output entities.

Output:
`WorldModelView`.

Rules:

- Assemble only outputs sharing the same normalized `DecisionContext`.
- Attach `ViewSnapshot`.
- Attach all hydration handles needed for explanation and execution handoff.
- Include abstention state even when empty.
- Include risk, conflict, sensitivity, and assumptions even when non-blocking.
- Preserve projection version and lower-layer source cursors.

Implementation gap:
typed response envelope is missing for partial views, warnings, and route metadata.

## Cross-Domain Projections

Some projections sit outside `world_model/planner` but complete the loop.
They are listed here for placement and dependency clarity.
Their detailed entity, component, and system docs should live under the owning domain when that domain is split into the same structure.

`WorldModelView -> ExecutionPlanningInput`
belongs to execution.
It defines the read port that execution planning may consume without importing world model internals.

`TaskLifecycleEvent + Artifacts + ExecutionContext -> OutcomeFact`
belongs to execution.
It turns task activity into world-model-legible outcome evidence.

`OutcomeFact -> EvidenceItem`
belongs to belief.
It normalizes execution outcomes back into belief evidence.

`BeliefRevision + PlannerAgentLens + ActiveGoals + CostBeliefs + ValueBeliefs + RegimeContext -> GoalMutation`
belongs to agent.
It turns belief revision and cost-benefit state into goal curation.

## Failure Semantics

| Condition | Planner behavior |
|---|---|
| invalid context | route error |
| missing required lens | route error or explicit default lens warning |
| missing graph identity input | route error for scoped view |
| missing belief input for desired predicate | hard abstention or not assessed precondition |
| stale belief input | stale output plus risk dimension |
| invalid belief view | hard abstention when required |
| missing causal input for candidate intervention | soft or hard abstention based on context requirement |
| blocked causal identification | causal risk plus abstention when intervention is required |
| unresolved regime mixture | regime risk plus possible expiry or abstention |
| missing source cursor | route warning at minimum, route error when replay is required |

## Test Matrix

| Test | Expected proof |
|---|---|
| replay determinism | same inputs and projection version produce same view id and child outputs |
| source ref preservation | every derived output traces to lower-layer refs or hydration handles |
| context filtering | out-of-scope records cannot affect outputs |
| branch filtering | records outside branch scope cannot affect outputs |
| stale belief handling | stale evidence downgrades support and feeds expiry |
| contradiction handling | scoped contradiction creates conflict and possible abstention |
| causal boundary | planner never estimates effect from graph or belief alone |
| regime boundary | planner never decides active regime |
| public route consistency | partial routes select from full pipeline outputs |
| execution boundary | planner preconditions do not include dispatch or readiness policy |

## System Rules

- Projection systems must be deterministic.
- Projection systems must be replayable from lower-layer records and projection version.
- Projection systems must not mutate lower-layer authority records.
- Projection systems must preserve provenance through source refs or hydration handles.
- Projection systems must emit abstention or uncertainty instead of fabricating missing support.
- Projection systems may rank or score views, but ranking is not dispatch policy.
- Public planner routes must share projection logic.
- Missing data must become typed absence, warning, route error, or abstention.

## Implementation Gap Register

| Gap | Blocks | Needed owner |
|---|---|---|
| context canonicalization | route idempotence and cache keys | planner |
| agent lens read port | admissibility filtering | agent |
| graph packet contract | graph input projection | graph |
| belief packet contract | belief projections | belief |
| causal packet contract | causal summaries | causation |
| regime packet contract | risk, sensitivity, expiry | regime |
| source cursor set | replay and snapshot | all source layers |
| score thresholds | ranking, blocking, abstention tests | planner |
| execution handoff contract | abstention and precondition consumption | execution, planner |
| partial response envelope | robust public routes | public interface |

## Read With

- [Planner Entities](entities.md)
- [Planner Components](components.md)
- [World Model Planner](README.md)
