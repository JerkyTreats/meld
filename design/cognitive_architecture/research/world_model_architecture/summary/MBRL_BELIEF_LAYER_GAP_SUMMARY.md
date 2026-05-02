# Model-Based RL Belief Layer Gap Summary

Source report: [Belief Layers Over Temporal Knowledge Graphs Through the Lens of Model-Based Reinforcement Learning](../pdf/Belief%20Layers%20Over%20Temporal%20Knowledge%20Graphs%20Through%20the%20Lens%20of%20Model-Based%20Reinforcement%20Learning.pdf)

Prompt source: [Latent State World Models from Model-Based RL](../../research_prompts.md#2-latent-state-world-models-from-model-based-rl)

## Definitions

- **Model-based reinforcement learning (MBRL):** A reinforcement learning approach that learns or uses a world model to predict future states, observations, rewards, or values before acting.
- **World model:** A predictive model of how the environment changes. In this report, the useful analogy is not policy learning, but belief-state maintenance under partial observation.
- **Latent state:** A hidden internal state used by the model to represent parts of the world that are not directly observed.
- **Observation:** New evidence from the environment. For Meld, observations are typed semantic facts from domain events, not pixels or continuous sensor readings.
- **Predictive prior:** The model's predicted belief state before seeing the next observation.
- **Posterior:** The belief state after conditioning the predictive prior on a new observation.
- **Filtering:** Online inference that updates belief using observations available up to the current time.
- **Smoothing:** Retrospective inference that revises hidden past states or transition times after later observations arrive.
- **Recurrent State Space Model (RSSM):** The latent dynamics model used by PlaNet and Dreamer-style systems. It combines deterministic recurrent state with stochastic latent state.
- **Dreamer / DreamerV3:** Model-based RL systems that use an RSSM-style world model with an explicit prior/posterior split. DreamerV3 uses categorical stochastic latents.
- **PlaNet:** A model-based RL system that uses RSSM latent dynamics and planning in latent space.
- **TD-MPC2:** A decoder-free model-based RL system focused on control-relevant latent prediction. It is useful for rollout ideas but less direct for calibrated posterior belief semantics.
- **KL divergence:** A measure of mismatch between two probability distributions. In this context, large prior/posterior divergence is a signal of surprise, contradiction, or revision pressure.
- **Under-observed belief:** A belief with insufficient evidence to identify state sharply. It is not the same as a contradicted belief.
- **Stale belief:** A belief whose evidence is aging because no new events have arrived, but which has not been explicitly contradicted.
- **Latent transition belief:** A belief that a state transition occurred even though no event directly observed the transition time.
- **Hazard or intensity model:** A model of when events are expected to occur. It helps distinguish "nothing happened" from "we expected an update by now and did not get one."
- **Object-centric world model:** A world model that factors state by persistent objects, slots, or entity files rather than representing the whole world as one monolithic state.
- **Object permanence:** The ability to keep an entity present in belief even when it is absent from current observation.

## Prompt Gap

The prompt targeted this gap:

> The current graph only captures what has been directly observed via domain events. There is no representation of what has not been seen, what is uncertain because it was not observed, or how beliefs should propagate through unobserved state transitions.

The research question was how latent-state models from model-based RL can inform a belief layer above a discrete, event-sourced temporal graph where observations are irregular semantic facts rather than continuous sensor streams.

## Answer To The Gap

The report answers the gap by extracting one core architectural pattern from MBRL: keep a predictive prior between observations, then update to a posterior when a new event arrives.

Dreamer-style RSSMs are the closest fit because they expose a clear prior/posterior split. Between observations, the model rolls the predictive prior forward. When an observation arrives, it computes an observation-conditioned posterior. That maps naturally to a belief layer above an event graph.

The report also draws a boundary: vanilla RSSM does not by itself provide semantic labels for stale, contradicted, under-observed, never-observed, or smoothed-inferred beliefs. Those distinctions must be represented in Meld's belief schema, not expected to emerge automatically from a generic latent model.

## Prompt Questions Mapped To Report Answers

### RSSM Uncertainty Between Observations

RSSM maintains latent state with a dynamics prior and an observation-conditioned posterior. PlaNet uses a diagonal Gaussian posterior; DreamerV3 keeps the same prior/posterior split but uses categorical stochastic latents.

Answer: the transferable part is the interface: store the predicted belief before the event and the updated belief after the event. The exact Gaussian or categorical latent distribution is less important than preserving prior, posterior, and surprise.

### Stale Versus Contradicted Versus Under-Observed

Vanilla RSSM does not name these as separate symbolic states. A contradiction appears as large prior/posterior mismatch. Staleness appears as growing prior uncertainty while the model rolls forward without new observations. Under-observation appears as a broad posterior because the event does not identify state sharply.

Answer: the belief layer should make these distinctions explicit with semantic fields. They should not be collapsed into one confidence number.

### Transfer To Irregular Event-Sourced Graphs

The report identifies several transferable techniques:

- explicit elapsed-time inputs for irregular event gaps
- masks or missingness signals because absence can be informative
- predicate-specific likelihood heads instead of image reconstruction
- smoothing for transitions that were never directly observed
- event hazard or intensity models for expected refresh cadence
- multi-step latent consistency for long gaps between observations

Answer: adapt MBRL around discrete semantic fact likelihoods and irregular time, not around pixel reconstruction.

### Multiple Concurrent Objects

The report finds that object-centric models address combinatorial growth by factoring state into objects, slots, or persistent files with sparse interactions. Structured World Belief is especially relevant because it separates object presence in belief from visibility in observation and keeps multiple hypotheses through particle methods.

Answer: factor the belief layer by entity-local state chains plus relation factors. Do not rely on whole-graph snapshots as the main state representation.

### Not Yet Observed Versus Observed As Contradicted

The concrete design answer is to separate truth status, observation origin, coverage, and uncertainty decomposition.

Answer: "not yet observed" should remain unknown or never-observed unless local completeness or explicit negative evidence licenses contradiction. Absence is not false under open-world semantics.

## Design Consequences

This report turns the latent-state gap into several concrete belief-store requirements:

- Store both prior and posterior for each belief update.
- Store a divergence or surprise score between prior and posterior.
- Separate truth status from observation status, coverage status, freshness, and uncertainty.
- Represent latent transitions as hypotheses with bounded intervals and posterior mass over possible change times.
- Use smoothing to revise hidden transition timing after later observations.
- Maintain predicate-specific persistence, refresh cadence, or hazard models.
- Treat explicit negative support and local completeness as the legitimate paths to contradiction.
- Factor belief by persistent entity files or local state chains plus relation factors.
- Preserve multiple hypotheses when entity identity, transition timing, or state is ambiguous.

## Suggested Belief Fields

The report implies a belief record should avoid a single undifferentiated confidence value. A practical schema should include:

- `truth_status`: `supported`, `contradicted`, `unknown`, or `inconsistent`
- `origin_status`: `directly_observed`, `smoothed_inference`, `predictive_persistence`, or `never_observed`
- `coverage_status`: `visible_or_reported`, `not_reported`, or `outside_local_completeness_scope`
- `freshness`: elapsed time, predicate-specific half-life, or hazard-derived staleness
- `predictive_uncertainty`: spread or entropy in the predictive prior
- `evidence_sparsity`: how much direct support the belief has
- `model_disagreement`: optional ensemble or hypothesis disagreement
- `prior_posterior_divergence`: update surprise when new evidence arrives

## What Remains Open

The report leaves several unresolved choices:

- Distribution choice: the architecture can borrow the prior/posterior interface without committing yet to Gaussian, categorical, particle, or interval-valued uncertainty.
- Hazard modeling: predicate-specific event cadence is needed, but the exact implementation can range from simple elapsed-time thresholds to learned point-process models.
- Scale: object-centric factorization reduces state growth but does not make arbitrary whole-world inference exact or cheap.
- Semantics: MBRL supplies latent filtering and smoothing patterns, but Meld still needs explicit symbolic belief semantics for contradiction, coverage, provenance, and planner-facing status.

The practical conclusion is architectural: use MBRL as a template for predictive priors, posterior updates, smoothing, and object permanence, but make belief semantics explicit in the event-sourced data model.
