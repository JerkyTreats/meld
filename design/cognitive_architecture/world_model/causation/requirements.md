# Causal Requirements

Date: 2026-05-15
Status: active
Scope: expanded implementation requirements for `world_model/causation`

## Thesis

Causal implementation must prove a replayable path from graph, belief, execution, and regime records to variables, interventions, outcome links, identification assessments, effect estimates, counterfactuals, assumptions, and planner-facing causal summaries.

The first slice must prevent selected anchors, temporal order, or task success from becoming implicit causal proof.

## Functional Requirements

### Variables

- Define `CausalVariable` for state, intervention, outcome, selection, regime condition, and confounder roles.
- Preserve subject scope, branch scope, perspective scope, and regime scope where material.
- Define measurement shape for each variable.
- Preserve source refs for variable creation.

### Interventions

- Lower execution action facts and task facts into `InterventionRecord`.
- Preserve intended target, actual target, intervention kind, application status, timing, delay window, rollback, and compensation.
- Treat interventions as candidate causal evidence until identification supports effect claims.

### Outcomes

- Link interventions to measured outcome variables through `OutcomeLink`.
- Preserve outcome value, measurement channel, measurement quality, observed time, and lag from intervention.
- Preserve candidate attribution state.
- Reject outcome links that lack compatible measurement semantics.

### Selection And Measurement

- Record measurement paths from graph provenance, evidence channels, and belief origin state.
- Preserve selection warnings when anchor selection, reporting policy, attention policy, or measurement policy may explain an observed outcome.
- Keep selection warnings visible in causal summaries.

### Mechanisms

- Define `MechanismVersion` for variable sets and structural form.
- Preserve parent links, selection links, method version, and regime condition.
- Mark mechanism candidate when evidence is insufficient.
- Avoid mechanism inference from temporal order alone.

### Confounders

- Represent hidden or observed confounder candidates as `ConfounderHypothesis`.
- Preserve supporting and weakening evidence refs.
- Score confounder risk separately from effect uncertainty.
- Feed uncontrolled material confounders into identification blockers.

### Identification

- Produce `IdentificationAssessment` before strong effect estimation.
- Preserve identification status, confidence, adjustment set, blockers, missing evidence, selection warnings, and assumption refs.
- Block identification when required confounders are uncontrolled or measurement paths are invalid.

### Effects

- Produce `EffectEstimate` with direction, magnitude, posterior summary, uncertainty, precision, identification status, confounder risk, selection warning, evidence window, method version, and provenance.
- Publish new estimates when source evidence or method version changes.
- Preserve old estimates.

### Counterfactuals

- Produce `CounterfactualCase` for alternative action, target, timing, context, or regime queries.
- Preserve factual case, alternative case, expected outcome, uncertainty, confidence, mechanism version, and assumptions.
- Mark counterfactual invalid when assumptions or mechanism conditions fail.

### Assumptions

- Record causal structure, intervention, measurement, adjustment, selection, regime, and hidden-context assumptions.
- Preserve validity, expiry, violation effect, and source refs.
- Expose assumption refs in planner-facing summaries.

### Summaries

- Publish `CausalSummary` for planner and perspective consumers.
- Include effect support, uncertainty, identification status, confounder risk, selection warning, assumption state, counterfactual summary, and provenance.
- Exclude raw worker internals and uncommitted drafts.

## Public Interface Requirements

- Query effect summary by intervention variable and outcome variable.
- Query identification status for an effect query.
- Query confounder blockers for an effect query.
- Query counterfactual case by factual intervention and alternative case.
- Query causal assumptions for an effect estimate.
- Query intervention records by execution action ref.
- Query outcome links by intervention record.
- Query causal summaries for planner context.

## Boundary Requirements

- Causation consumes graph structure and provenance.
- Causation consumes belief uncertainty and evidence quality.
- Causation consumes execution action and outcome facts through the spine.
- Causation consumes regime context as mechanism condition.
- Causation emits mechanism-aware summaries.
- Causation does not settle belief.
- Causation does not decide regime identity.
- Causation does not rank planner policy.
- Causation does not dispatch tasks.

## Nonfunctional Requirements

### Replay

- Every causal record is rebuildable from source refs, method version, assumptions, and source cursor.
- Every estimate preserves evidence window and method version.
- Counterfactual outputs preserve factual case, alternative case, mechanism, and assumptions.

### Determinism

- Same source records and method version produce same identification assessment, effect estimate, and causal summary.
- Semantic or heuristic causal steps record method version and remain provisional by default.

### Audit

- Source refs survive intervention lowering, outcome linking, identification, effect estimation, counterfactual evaluation, assumption projection, and summary projection.
- Selection warnings, confounder risk, and blocked identification remain explicit.

### Safety

- Temporal precedence is insufficient for effect support.
- Anchor replacement is insufficient for effect support.
- Execution success is insufficient for downstream effect support.
- Blocked identification prevents strong causal summary.

## Read With

- [Causal Spec](spec.md)
- [Causal Layer](README.md)
