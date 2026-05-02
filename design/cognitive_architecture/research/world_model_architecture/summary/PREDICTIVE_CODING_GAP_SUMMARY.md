# Predictive Coding Gap Summary

Source report: [Predictive Coding for Hierarchical Belief Systems](../pdf/Predictive%20Coding%20for%20Hierarchical%20Belief%20Systems.pdf)

Prompt source: [Predictive Coding and Hierarchical Belief Propagation](../../research_prompts.md#6-predictive-coding-and-hierarchical-belief-propagation)

## Definitions

- **Predictive coding:** An inference scheme where higher levels send predictions downward and lower levels send precision-weighted prediction errors upward.
- **Prediction error:** The mismatch between an observed or lower-level state and the state predicted by the level above.
- **Precision weighting:** Scaling a prediction error by the inverse uncertainty of the relevant generative conditional. High-precision small errors can matter more than low-precision large errors.
- **Generative model:** A probabilistic model of how higher-level hidden causes produce lower-level states and observations.
- **Variational free energy:** The objective minimized by classical predictive coding under a Laplace or point-estimate posterior approximation.
- **Laplace approximation:** An approximation that represents a posterior around a mode, commonly reducing inference to optimization over means with precision terms.
- **Factor graph:** A graph representation of how a joint probability distribution factors into local functions over variables.
- **Belief propagation (BP):** A message-passing algorithm for computing marginal beliefs on factor graphs. On trees, sum-product BP is exact.
- **Loopy belief propagation:** BP applied to graphs with cycles. It is approximate and not guaranteed to converge generally.
- **Bethe free energy:** The variational objective whose stationary points correspond to loopy BP fixed points.
- **Gaussian BP:** Belief propagation specialized to linear-Gaussian models, carrying sufficient statistics for Gaussian marginals.
- **Discrete event factor:** A likelihood or constraint over symbolic, categorical, Bernoulli, count, temporal, or relational events.
- **Inference epoch:** A bounded inference pass over a temporarily frozen graph topology.
- **Damping:** Blending old and new messages to improve stability in iterative message passing.
- **Sensory attenuation:** Selectively lowering precision for expected self-generated signals while preserving sensitivity to unexpected external evidence.
- **Epistemic action:** An action selected to reduce uncertainty, such as requesting more evidence when conflicting messages persist.

## Prompt Gap

The prompt targeted this gap:

> The belief layer treats every belief key as independent — evidence flows bottom-up from facts to beliefs but beliefs never generate top-down predictions about what evidence should look like. Predictive coding is the research tradition that formalizes bidirectional message passing in hierarchical models and is the natural next reference point for the belief architecture.

The research question was whether predictive coding should become the architecture for a discrete, event-sourced hierarchical belief system, or whether it should be used more narrowly as one inference method within a broader probabilistic graph.

## Answer To The Gap

The report answers the gap by saying predictive coding is useful, but not sufficient as the whole architecture.

For Meld, predictive coding is best understood as an inference schedule over a generative model. It is strongest for differentiable, approximately Gaussian hierarchies where inference can relax between observations. Meld's belief system is different: observations are discrete semantic events, graph structure can change online, symbolic constraints matter, and exact or calibrated marginals may matter.

The best fit is therefore a hybrid:

- Use factor graphs, variational message passing, or belief propagation for discrete event structure and symbolic factors.
- Use predictive-coding-style local residual updates for continuous embeddings, continuous latent submodels, or differentiable hierarchy components.
- Preserve top-down predictions and precision-weighted error messages as design patterns, but do not force every event into a subtraction-based residual.

## Prompt Questions Mapped To Report Answers

### Formal Model Versus Bottom-Up Bayes

Classical predictive coding uses a layered Gaussian generative model and minimizes a variational free-energy surrogate over latent means. Upward messages are precision-weighted residuals. Factor-graph BP sends functions or distributions over variables and, on trees, computes exact marginals.

Answer: the real contrast is not bidirectional versus bottom-up. Exact Bayesian inference on hierarchies is already bidirectional. The difference is residual passing with local gradient descent versus distribution passing with sum-product integration.

### Precision Weighting

Precision weighting means a large error from an unreliable channel can be suppressed while a smaller error from a reliable channel dominates the update.

Answer: predictive coding makes reliability weighting local and explicit in the message itself. A Meld belief hierarchy should attach precision or reliability to every conditional link, not only to raw event sources.

### Discrete And Structured Domains

The literature supports predictive coding beyond vision, especially in differentiable networks, arbitrary directed predictive-coding graphs, and arbitrary-distribution predictive coding. It is thinner on classical predictive-coding message passing for knowledge graphs, ontologies, or symbolic world models.

Answer: for discrete semantic events, the prediction error is not usually a raw subtraction. The local signal should come from the relevant likelihood: categorical, Bernoulli, count, hazard, temporal, or relational.

### Relationship To Loopy BP

Loopy BP and predictive coding are related local message-passing schemes, but they are not generally equivalent. Loopy BP fixed points correspond to stationary points of Bethe free energy. Predictive coding performs gradient descent on a variational free-energy objective over latent means under a particular approximation family.

Answer: they coincide only in narrow cases such as tree-structured linear-Gaussian models where posterior means or modes agree. For discrete or multimodal beliefs, use factor-graph methods for the discrete structure.

### Failure Modes And Safeguards

The core failure mode is hallucination-like suppression: over-precise high-level priors or under-precise bottom-up evidence can cause legitimate lower-level evidence to be explained away.

Answer: add safeguards analogous to attention, sensory attenuation, and epistemic action: adaptive precision control, precision floors for trusted bottom-up channels, selective attenuation for self-predicted signals, and observation requests when conflicts persist.

## Design Consequences

This report turns the predictive-coding gap into several concrete requirements:

- Represent the belief system as an explicit generative graph over events, latent states, aggregates, and time.
- Let parent nodes send predictions downward where the conditional is differentiable or otherwise modelable.
- Let child nodes return precision-weighted residuals only where residuals make sense.
- For discrete events, pass variational messages or likelihood gradients instead of raw residuals.
- Attach reliability or precision parameters to event-to-state, state-to-aggregate, and aggregate-to-hypothesis conditionals.
- Process dynamic graphs in inference epochs with temporarily frozen topology.
- Use damping or asynchronous scheduling for iterative message passing.
- Treat graph edits as objective changes that require local re-solving or re-initialization.
- Add precision floors for trusted evidence channels.
- Trigger epistemic actions when high-level and low-level messages remain in high-uncertainty conflict.

## Suggested Belief Fields

The report implies hierarchical belief records or edges should eventually expose:

- `prediction`: the top-down expected lower-level state or event distribution
- `prediction_error`: the local mismatch signal, when a residual representation is valid
- `precision`: reliability assigned to the relevant conditional
- `message_kind`: residual, likelihood_gradient, variational_message, or exact_constraint
- `conditional_model`: the local likelihood or generative mapping used for the update
- `inference_epoch`: the topology-frozen pass in which a message was computed
- `damping_factor`: stabilization parameter for iterative updates
- `self_predicted`: whether the evidence channel was expected as a consequence of the system's own action
- `epistemic_escalation`: whether persistent conflict should trigger an observation request

## What Remains Open

The report leaves several design choices unresolved:

- Which parts of Meld's belief hierarchy, if any, are continuous enough for classical predictive-coding residual dynamics.
- Which discrete factors should use BP, variational message passing, exact constraints, or likelihood-gradient updates.
- How to schedule inference when the graph grows while messages are still relaxing.
- How to calibrate precision so top-down priors do not silence legitimate bottom-up evidence.
- How much predictive-coding vocabulary should be exposed in public APIs versus used as internal implementation language.

The practical conclusion is architectural: predictive coding should inform hierarchical prediction, precision weighting, and conflict handling, but Meld should use a hybrid inference architecture rather than replacing structured probabilistic inference with classical Rao-Ballard predictive coding alone.
