# World Model Architecture Synthesis

This directory synthesizes the world-model architecture research into one higher-level design abstraction for Meld.

The common conclusion across the research is that the world model should not be a single graph, a single Bayesian comparator, or a single learned latent model. It should be a layered inference system:

```text
world boundary
  -> event spine
  -> temporal state graph
  -> belief and generative inference
  -> planner-facing belief or policy view
  -> task execution
  -> event spine
```

The event spine preserves what happened. The graph materializes what currently or historically holds. The belief layer reasons over what is trusted, stale, contradicted, latent, causal, regime-dependent, or worth observing. The planner should consume a settled or actionable view, not raw facts.

## Source Evidence

- [Active Inference Gap Summary](summary/ACTIVE_INFERENCE_GAP_SUMMARY.md)
  maps the existing loop to active inference and argues that `BeliefView` should carry uncertainty, precision, and observation/action policy value.
- [Model-Based RL Belief Layer Gap Summary](summary/MBRL_BELIEF_LAYER_GAP_SUMMARY.md)
  contributes the prior/posterior, filtering/smoothing, latent transition, hazard, and object-permanence pattern.
- [Causal World Model Gap Summary](summary/CAUSAL_WORLD_MODEL_GAP_SUMMARY.md)
  separates correlation-shaped anchors from causal variables for state, task, outcome, selection, and regime.
- [Temporal KG Gap Summary](summary/TEMPORAL_KG_GAP_SUMMARY.md)
  validates the event-spine plus anchor-layer split and clarifies valid time, transaction time, reference time, intervals, and temporal query semantics.
- [Non-Stationarity Gap Summary](summary/NON_STATIONARITY_GAP_SUMMARY.md)
  makes regime uncertainty first-class and distinguishes structural change from high-churn belief storms.
- [Predictive Coding Gap Summary](summary/PREDICTIVE_CODING_GAP_SUMMARY.md)
  supports hierarchical top-down prediction and precision-weighted conflict handling, while recommending a hybrid with factor-graph or variational message passing.
- [Bibliography](summary/BIBLIOGRAPHY.md)
  lists extracted source references from the whitepapers.

## Synthesized Architecture

### 1. Boundary And Spine

The Markov blanket analogue is not the graph or the planner view. It is the system boundary where observations enter and actions leave. Once observations are normalized into the event spine, they are internalized evidence.

The spine should remain append-only and semantic. It records observations, actions, outcomes, corrections, supersessions, and provenance. It should not be overloaded with belief confidence, decay, causal effect estimates, or regime identity.

### 2. Temporal State Graph

The graph layer materializes state from the spine. Its job is truth maintenance over time:

- current anchors
- as-of anchors
- valid-time intervals
- transaction-time history
- lineage and provenance
- supersession and invalidation

Temporal KG research supports this split. Embedding models can help with scoring and forecasting, but they should not replace event-sourced truth maintenance.

### 3. Belief And Generative Inference

The belief layer should be a generative inference layer above the graph, not just a confidence score over current anchors.

It should preserve:

- predictive prior before new evidence
- posterior after evidence
- prior/posterior divergence or surprise
- uncertainty and precision
- freshness and predicate-specific hazard
- contradiction versus staleness versus under-observation
- latent transition hypotheses
- regime posterior and changepoint probability
- causal uncertainty over mechanisms, interventions, confounders, and regimes

This layer is where Bayesian comparators, variational inference, smoothing, regime detection, causal reasoning, and hierarchical prediction belong.

### 4. Causal And Regime Structure

Typed anchors are evidence, not causal claims. A selected anchor should not imply that the preceding task caused the selected frame to be good.

The causal model should separate:

- state variables
- task or intervention variables
- outcome variables
- selection or measurement variables
- regime or context variables

Non-stationarity adds another axis: the model should not assume one fixed causal or statistical structure. It should retain separate regime posteriors and use mixture predictions while regime identity is uncertain.

### 5. Planner-Facing View

The planner-facing view should be richer than "settled belief." It should expose the action-relevant result of inference:

- settled beliefs
- uncertainty and precision
- stale or under-observed beliefs
- candidate observation policies
- expected information gain
- pragmatic value or decision relevance
- causal effect estimates where available
- regime uncertainty when it changes the decision

This keeps planning away from raw facts while still allowing unresolved beliefs to become useful actions.

## Core Design Rule

Do not collapse truth, evidence, confidence, freshness, causality, and action value into one field.

The research repeatedly points to the same separation:

- truth maintenance belongs in the event spine and temporal graph
- belief maintenance belongs in posterior inference and revision
- freshness belongs in hazard or decay models
- contradiction requires explicit counterevidence or completeness
- causality requires state/action/outcome/selection separation
- non-stationarity requires regime uncertainty
- observation requests are policies with epistemic value

## Candidate Public Concepts

The architecture likely needs public concepts along these lines:

- `TemporalAnchor`
- `ValidityInterval`
- `AsOfQuery`
- `BeliefRevision`
- `BeliefPrior`
- `BeliefPosterior`
- `BeliefPrecision`
- `ObservationPolicy`
- `LatentTransition`
- `RegimePosterior`
- `CausalVariable`
- `InterventionRecord`
- `OutcomeVariable`
- `SelectionVariable`

These names are not final API commitments. They identify the conceptual boundaries implied by the research.

## Open Architecture Questions

- How much active-inference vocabulary should appear in public APIs versus remain implementation guidance?
- Which belief updates can stay exact Bayesian comparators, and which need variational or message-passing approximations?
- How should the system schedule inference when the graph topology changes during belief propagation?
- What is the minimum useful regime model for the first implementation slice?
- Which causal variables can be logged reliably enough to support intervention reasoning?
- What planner contract best represents both settled beliefs and observation policies?

## Reading Order

Start with [Temporal KG Gap Summary](summary/TEMPORAL_KG_GAP_SUMMARY.md) for the spine and graph semantics, then [Model-Based RL Belief Layer Gap Summary](summary/MBRL_BELIEF_LAYER_GAP_SUMMARY.md) for belief-state mechanics. Read [Causal World Model Gap Summary](summary/CAUSAL_WORLD_MODEL_GAP_SUMMARY.md) and [Non-Stationarity Gap Summary](summary/NON_STATIONARITY_GAP_SUMMARY.md) next for structural uncertainty. Finish with [Active Inference Gap Summary](summary/ACTIVE_INFERENCE_GAP_SUMMARY.md) and [Predictive Coding Gap Summary](summary/PREDICTIVE_CODING_GAP_SUMMARY.md) for planner-facing policy value and hierarchical inference.
