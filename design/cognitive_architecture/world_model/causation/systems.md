# Causal Systems

Date: 2026-05-15
Status: active
Scope: ECS systems for `world_model/causation`

## Thesis

Causal systems transform graph, belief, regime, and execution evidence into intervention records, outcome links, mechanism state, identification assessments, effect estimates, counterfactual cases, assumptions, and planner-facing summaries.

They separate evidence of temporal change from justified causal claims.
They do not settle belief, decide regime identity, rank planner policy, or dispatch tasks.

## Pipeline Overview

| Phase | System | Required input | Required output |
|---|---|---|---|
| 1 | variable registration | graph refs, belief dimensions, outcome semantics | `CausalVariable` |
| 2 | intervention lowering | execution action and task facts | `InterventionRecord` |
| 3 | outcome linking | intervention records, outcome evidence | `OutcomeLink` |
| 4 | measurement path interpretation | graph provenance, evidence channels | measurement paths and selection warnings |
| 5 | mechanism selection | variables, graph relations, belief state, regime context | `MechanismVersion` |
| 6 | confounder discovery | graph paths, belief uncertainty, selection state | `ConfounderHypothesis` |
| 7 | confounder scoring | confounder evidence, adjustment candidates | confounder risk |
| 8 | identification assessment | mechanism, confounders, measurement paths | `IdentificationAssessment` |
| 9 | effect estimation | identified evidence, mechanism, assumptions | `EffectEstimate` |
| 10 | counterfactual evaluation | effect estimate, factual case, alternative case | `CounterfactualCase` |
| 11 | assumption projection | identification, estimate, counterfactual | `CausalAssumptionSet` |
| 12 | causal summary projection | effect, risks, assumptions, counterfactual | `CausalSummary` |
| 13 | recovery and replay | source refs, method versions, cursors | rebuilt causal projections |

Replay invariant:
same graph records, belief views, execution facts, regime context, method versions, and source cursors produce the same causal outputs.

## Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| graph current state and walks | graph | variable scope, measurement paths, selection paths |
| graph provenance and lineage | graph | anchor selection warnings and supersession context |
| belief views | belief | uncertainty, freshness, evidence quality, outcome state |
| execution action and outcome facts | execution through spine | intervention lowering and outcome linking |
| regime summaries | regime | mechanism condition and assumption state |
| planner context | planner or query route | scoped causal summaries |

## Variable Registration

Input:
graph refs, belief dimensions, outcome semantics, intervention semantics.

Output:
`CausalVariable`.

Rules:

- Register state variables for persistent world conditions.
- Register intervention variables for deliberate actions and control moves.
- Register outcome variables for measured quality, utility, success, or downstream effect.
- Register selection variables when visibility or attachment policy affects observed data.
- Register regime variables only as mechanism conditions, not as regime authority.
- Preserve source refs for every variable.

## Intervention Lowering

Input:
execution action facts, task lifecycle facts, capability results, graph refs.

Output:
`InterventionRecord`.

Rules:

- Record intended target and actual target when known.
- Record intervention kind and application status.
- Record timing and expected delay window.
- Record rollback or compensation when available.
- Treat every intervention as candidate causal evidence until identification supports effect claims.

## Outcome Linking

Input:
intervention records, graph state changes, belief revisions, execution outcomes, measurement evidence.

Output:
`OutcomeLink`.

Rules:

- Link outcomes to interventions through explicit evidence refs.
- Preserve measurement channel and measurement quality.
- Preserve observed time and lag from intervention.
- Mark attribution as candidate until identification assessment.
- Reject outcome links with no compatible outcome variable or measurement path.

## Measurement Path Interpretation

Input:
graph provenance, anchor lineage, evidence channels, belief origin state.

Output:
measurement paths and selection warnings.

Rules:

- Record how the outcome became visible.
- Identify selection-shaped evidence when anchor choice may explain observed outcome.
- Identify measurement-shaped evidence when channel policy may explain observed outcome.
- Preserve graph paths and evidence refs.
- Keep selection warnings visible in effect summaries.

## Mechanism Selection

Input:
causal variables, graph relations, belief summaries, regime context.

Output:
`MechanismVersion`.

Rules:

- Select a mechanism only for scoped variables and applicable regime context.
- Preserve parent links and selection links.
- Preserve method version.
- Mark candidate mechanism when evidence is insufficient.
- Do not infer mechanism from temporal order alone.

## Confounder Discovery

Input:
graph paths, belief uncertainty, regime state, selection paths, outcome links.

Output:
`ConfounderHypothesis`.

Rules:

- Identify hidden or observed variables that may influence intervention and outcome.
- Preserve supporting and weakening evidence refs.
- Emit confounder hypotheses when identification depends on controlling them.
- Keep confounders separate from generic uncertainty.

## Confounder Scoring

Input:
confounder hypotheses, evidence quality, adjustment candidates.

Output:
confounder risk.

Rules:

- Score confounder severity.
- Record whether adjustment controls the confounder.
- Preserve risk reason codes.
- Feed unresolved confounders into identification blockers.

## Identification Assessment

Input:
mechanism version, intervention variable, outcome variable, confounder risks, measurement paths, assumptions.

Output:
`IdentificationAssessment`.

Rules:

- Determine whether effect estimation is supported, blocked, provisional, or expired.
- Preserve adjustment set and assumption refs.
- Preserve missing evidence and blocker list.
- Treat material selection warning as identification risk.
- Block effect estimation when required confounders are uncontrolled.

## Effect Estimation

Input:
identified evidence, outcome links, mechanism version, assumptions.

Output:
`EffectEstimate`.

Rules:

- Estimate effect direction and magnitude only after identification assessment.
- Preserve effect uncertainty and precision.
- Preserve identification status on the estimate.
- Preserve confounder risk and selection warning on the estimate.
- Mark estimate weak or blocked when identification is weak or blocked.

## Counterfactual Evaluation

Input:
effect estimate, factual intervention, observed outcome, alternative intervention or context, assumptions.

Output:
`CounterfactualCase`.

Rules:

- State factual intervention and observed outcome.
- State alternative action, target, timing, context, or regime.
- Use mechanism version and assumptions explicitly.
- Preserve uncertainty and confidence.
- Mark counterfactual invalid when mechanism or assumptions expire.

## Assumption Projection

Input:
identification assessment, effect estimate, counterfactual case, mechanism version, regime context.

Output:
`CausalAssumptionSet`.

Rules:

- Record causal structure assumptions.
- Record intervention application assumptions.
- Record measurement and selection assumptions.
- Record adjustment set assumptions.
- Record regime and hidden-context assumptions.
- Record violation effect and expiry.

## Causal Summary Projection

Input:
effect estimate, identification assessment, confounder risks, selection warnings, assumptions, counterfactual cases.

Output:
`CausalSummary`.

Rules:

- Expose effect direction, magnitude, uncertainty, and identification state.
- Expose confounder risk and selection warning.
- Expose assumption refs and validity state.
- Expose counterfactual summary when available.
- Exclude raw worker internals and uncommitted drafts.

## Recovery And Replay

Input:
source refs, causal records, method versions, source cursors.

Output:
rebuilt causal projections.

Rules:

- Rebuild intervention records from execution facts.
- Rebuild outcome links from outcome evidence and graph state.
- Rebuild identification and effect estimates from method versions and source refs.
- Preserve old estimates when method version changes by publishing new estimates.
- Do not rewrite old causal results.

## Failure Semantics

| Condition | Causal behavior |
|---|---|
| intervention lacks target | reject intervention lowering with reason |
| outcome lacks measurement path | keep outcome unlinked or rejected with reason |
| selection warning is material | mark identification provisional or blocked |
| uncontrolled confounder is material | block identification |
| mechanism is unavailable | emit missing mechanism status |
| regime condition is invalid | expire mechanism-dependent estimate |
| counterfactual lacks assumptions | mark counterfactual invalid |
| source evidence changes | publish new estimate instead of editing old result |

## Test Matrix

| Test | Expected proof |
|---|---|
| intervention lowering | execution fact becomes intervention record |
| outcome linking | outcome evidence links through explicit measurement path |
| anchor selection guard | selected anchor alone does not imply effect |
| confounder block | uncontrolled confounder blocks identification |
| selection warning | selection-shaped evidence remains visible |
| identification gate | blocked identification prevents strong effect estimate |
| effect replay | same inputs and method version produce same estimate |
| counterfactual assumptions | missing or expired assumption invalidates case |
| summary boundary | causal summary excludes worker internals |
| estimate supersession | source evidence change creates new estimate |

## System Rules

- Systems are deterministic over explicit inputs and method versions.
- Systems preserve graph, belief, execution, regime, and causal source refs.
- Systems emit weak support or blocked identification instead of fabricating effect support.
- Systems keep intervention evidence separate from outcome attribution.
- Systems keep selection warnings and confounder risks explicit.

## Read With

- [Causal Entities](entities.md)
- [Causal Components](components.md)
- [Causal Requirements](requirements.md)
- [Causal Layer](README.md)
