# Causal Entities

Date: 2026-05-15
Status: active
Scope: ECS entities for `world_model/causation`

## Thesis

Causal entities are stable identities for variables, mechanisms, interventions, outcome links, confounders, identification state, effect estimates, counterfactuals, assumptions, and planner-facing summaries.

They separate temporal evidence from mechanism claims.
They prevent graph reachability, anchor selection, and execution success from becoming implicit causal proof.

## Entity Categories

| Category | Entities | Persistence expectation |
|---|---|---|
| Variable identity | `CausalVariable` | durable |
| Mechanism | `MechanismVersion` | durable when proposed or selected |
| Intervention | `InterventionRecord` | durable |
| Outcome evidence | `OutcomeLink` | durable |
| Confounding | `ConfounderHypothesis` | durable when material |
| Identification | `IdentificationAssessment` | materialized projection |
| Effect | `EffectEstimate` | durable revision or materialized projection |
| Counterfactual | `CounterfactualCase` | materialized projection |
| Assumption | `CausalAssumptionSet` | durable with estimate or case |
| Consumer view | `CausalSummary` | materialized projection |

## Identity Rules

| Entity | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `CausalVariable` | variable family plus subject scope plus dimension plus perspective or regime scope | none | causation |
| `MechanismVersion` | mechanism family plus variable set plus regime condition plus version | one or more `CausalVariable` values | causation |
| `InterventionRecord` | execution action ref plus target variable plus intervention kind | `CausalVariable` target | causation |
| `OutcomeLink` | intervention record plus outcome variable plus measurement evidence id | `InterventionRecord` | causation |
| `ConfounderHypothesis` | candidate hidden variable plus affected intervention and outcome variables | variable pair or effect query | causation |
| `IdentificationAssessment` | intervention variable plus outcome variable plus evidence window plus method version | effect query | causation |
| `EffectEstimate` | intervention variable plus outcome variable plus evidence window plus mechanism version | effect query | causation |
| `CounterfactualCase` | factual intervention plus alternative intervention plus outcome variable plus mechanism version | effect query | causation |
| `CausalAssumptionSet` | effect estimate or counterfactual case plus assumption version | estimate or case | causation |
| `CausalSummary` | effect estimate plus planner context plus summary version | estimate | causation |

## `CausalVariable`

One modeled variable in the causal layer.

Purpose:
names a state, intervention, outcome, selection, regime, or confounder variable.

Required components:

- `VariableIdentity`
- `VariableScope`
- `VariableKind`
- `VariableMeasurement`
- `VariableProvenance`

Creation:
created by causal variable registration, intervention lowering, outcome linking, or mechanism definition.

Lifecycle:
active, deprecated, merged, archived.

Dependencies:
graph object refs, belief dimensions, outcome semantics, variable registry.

## `MechanismVersion`

One regime-conditioned mechanism candidate.

Purpose:
defines a candidate causal structure or update rule used to identify and estimate effects.

Required components:

- `MechanismIdentity`
- `MechanismScope`
- `MechanismStructure`
- `RegimeCondition`
- `MechanismProvenance`

Creation:
created by mechanism registration, mechanism selection, or regime-conditioned model update.

Lifecycle:
candidate, selected, superseded, archived.

Dependencies:
causal variables, graph relations, belief summaries, regime state, mechanism registry.

## `InterventionRecord`

One attempted intervention context.

Purpose:
records what action was attempted, intended target, actual target, scope, timing, and execution evidence.

Required components:

- `InterventionIdentity`
- `InterventionTarget`
- `InterventionSemantics`
- `InterventionTiming`
- `InterventionProvenance`

Creation:
created by lowering execution action and task outcome facts into causal intervention records.

Lifecycle:
candidate, applied, partially applied, failed, compensated.

Dependencies:
execution action facts, task run refs, graph object refs, belief evidence refs.

## `OutcomeLink`

One evidence link from intervention to measured outcome.

Purpose:
connects an intervention record to an observed outcome variable through explicit measurement evidence.

Required components:

- `OutcomeLinkIdentity`
- `OutcomeMeasurement`
- `OutcomeTiming`
- `OutcomeAttribution`
- `OutcomeProvenance`

Creation:
created by outcome linking over execution outcomes, graph state changes, and belief revisions.

Lifecycle:
candidate, accepted, rejected, superseded.

Dependencies:
intervention record, outcome variable, evidence item, measurement path.

## `ConfounderHypothesis`

One hidden-cause explanation candidate.

Purpose:
records a variable or context that may influence both intervention selection and outcome.

Required components:

- `ConfounderIdentity`
- `ConfounderScope`
- `ConfounderEvidence`
- `ConfounderRisk`
- `ConfounderProvenance`

Creation:
created by confounder discovery, selection-path interpretation, failed identification, or regime mismatch.

Lifecycle:
candidate, material, controlled, dismissed, archived.

Dependencies:
belief uncertainty, graph relations, selection paths, regime state, outcome links.

## `IdentificationAssessment`

One assessment of whether an effect is identifiable.

Purpose:
states whether available evidence and assumptions can support effect estimation for an intervention and outcome pair.

Required components:

- `IdentificationScope`
- `IdentificationStatus`
- `AdjustmentSet`
- `IdentificationBlockers`
- `IdentificationProvenance`

Creation:
created by identification assessment before or during effect estimation.

Lifecycle:
valid, blocked, provisional, expired.

Dependencies:
causal variables, confounder hypotheses, measurement paths, selection warnings, assumptions.

## `EffectEstimate`

One posterior effect summary.

Purpose:
records effect direction, magnitude, uncertainty, evidence window, identification status, and provenance.

Required components:

- `EffectIdentity`
- `EffectScope`
- `EffectPosterior`
- `EffectUncertainty`
- `EffectProvenance`

Creation:
created by effect estimation after identification assessment.

Lifecycle:
current, superseded, invalidated, archived.

Dependencies:
intervention records, outcome links, mechanism version, identification assessment, assumptions.

## `CounterfactualCase`

One alternative-world evaluation.

Purpose:
answers what outcome is expected under a different action, target, timing, or context.

Required components:

- `CounterfactualIdentity`
- `FactualCase`
- `AlternativeCase`
- `CounterfactualOutcome`
- `CounterfactualProvenance`

Creation:
created by counterfactual evaluation from an effect estimate and mechanism version.

Lifecycle:
current, superseded, invalidated, archived.

Dependencies:
effect estimate, mechanism version, assumptions, factual intervention, alternative intervention.

## `CausalAssumptionSet`

One explicit assumption set for an estimate or counterfactual.

Purpose:
records intervention, measurement, adjustment, selection, regime, and hidden-context assumptions.

Required components:

- `AssumptionScope`
- `CausalAssumptions`
- `AssumptionValidity`
- `AssumptionViolationEffect`
- `AssumptionProvenance`

Creation:
created with identification assessment, effect estimate, or counterfactual case.

Lifecycle:
active, violated, expired, superseded.

Dependencies:
mechanism version, identification assessment, evidence policy, regime context.

## `CausalSummary`

One planner-facing causal projection.

Purpose:
exposes supported effect, weak support, blocked identification, confounding, selection warnings, assumptions, and counterfactual state without exposing raw causal worker internals.

Required components:

- `CausalSummaryIdentity`
- `CausalEffectView`
- `CausalRiskView`
- `CausalAssumptionView`
- `CausalSummaryProvenance`

Creation:
created by causal summary projection.

Lifecycle:
current, stale, superseded, invalid.

Dependencies:
effect estimate, identification assessment, confounder hypotheses, assumption set, counterfactual case.

## Entity Rules

- `CausalVariable` is the identity root for mechanism reasoning.
- `InterventionRecord` records attempted action semantics, not proof of effect.
- `OutcomeLink` records measured outcome evidence, not attribution by itself.
- `IdentificationAssessment` gates effect estimation.
- `EffectEstimate` preserves identification status and uncertainty.
- `CounterfactualCase` depends on explicit mechanism and assumption state.
- `CausalSummary` is the public consumer view.

## Read With

- [Causal Components](components.md)
- [Causal Systems](systems.md)
- [Causal Requirements](requirements.md)
- [Causal Layer](README.md)
