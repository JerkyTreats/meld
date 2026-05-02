# Active Inference Gap Summary

Source report: [Active Inference Mapping for an Event-Spine World Model](../pdf/Active%20Inference%20Mapping%20for%20an%20Event-Spine%20World%20Model.pdf)

Prompt source: [Active Inference and the Free Energy Principle](../../research_prompts.md#1-active-inference-and-the-free-energy-principle)

## Definitions

- **Active inference:** A framework where agents infer hidden states of the world and choose actions that reduce expected uncertainty and expected mismatch with preferred outcomes.
- **Free Energy Principle (FEP):** The broader theoretical claim that adaptive systems maintain themselves by minimizing free-energy-like bounds on surprise.
- **Variational free energy (VFE):** An optimization objective for approximate Bayesian inference over current hidden states. Minimizing VFE makes an approximate posterior closer to the true posterior.
- **Expected free energy (EFE):** A policy-selection objective that scores future actions by pragmatic value and epistemic value.
- **Generalized free energy (GFE):** A formulation that unifies inference over present and future outcomes by treating future outcomes as hidden states.
- **Markov blanket:** The statistical boundary that separates internal states from external states through sensory states and active states.
- **Sensory state:** The part of the Markov blanket influenced by the environment. In Meld, the closest analogue is observation ingress before facts are internalized into the spine.
- **Active state:** The part of the Markov blanket through which the system acts on the environment. In Meld, the closest analogue is task execution and outbound effects.
- **Generative model:** The model that explains how hidden states produce observations and how states evolve under actions or policies.
- **Hidden state:** A latent cause inferred from observations, such as world conditions that anchors summarize.
- **Policy:** A candidate course of action or observation. In active inference, policies are evaluated by expected free energy, not only reward.
- **Epistemic value:** The expected information gain from an action or observation.
- **Pragmatic value:** The expected progress toward preferred outcomes or goals.
- **Precision:** Confidence or inverse uncertainty assigned to beliefs or prediction errors.
- **BeliefView:** Meld's planner-facing summary of settled or actionable belief state. In this mapping, it should carry posterior state, uncertainty, and observation/action intent.

## Prompt Gap

The prompt targeted this gap:

> The architecture has the right shape — spine as sensory input, world model as belief integrator, agent as actor — but does not draw from the unified theoretical framework that formalizes this loop.

The research question was how Meld's event spine, graph materialization, belief layer, and planner map onto active inference and whether free energy should replace or refine the current Bayesian comparator model.

## Answer To The Gap

The report answers the gap by saying Meld's three-layer design maps cleanly to active inference if each layer is assigned the right role.

The event spine is best understood as the realized history of observations, actions, and outcomes after they cross the system boundary. The graph layer materializes current hypotheses about hidden causes. The belief layer performs posterior inference, tracks uncertainty or precision, and compares models. The planner should consume a belief or policy view rather than raw facts.

The main mismatch is that active inference normally integrates state inference and policy selection more tightly than Meld's current separation between belief and planning. That separation can remain an engineering choice, but the `BeliefView` should carry policy-relevant structure: posterior beliefs, uncertainty, precision, and whether the lowest-free-energy next move is to act or observe.

## Prompt Questions Mapped To Report Answers

### Markov Blanket Placement

The Markov blanket is not the graph boundary or the `BeliefView` boundary. It is the system-environment statistical interface, partitioned into sensory and active states.

Answer: the closest Meld analogue is the ingress/egress edge around the event spine. Incoming observations are sensory-like before internalization; task executions and effects are active-like. The spine log itself is internal state after the observation has crossed the boundary.

### Free Energy And Belief Revision

The report says posterior computation and free energy minimization are not alternatives of the same kind. The posterior is the target belief state. VFE is the optimization objective used when exact inference is intractable.

Answer: if a comparator can compute an exact posterior, it remains compatible with active inference. If inference is approximate, use VFE internally. The planner-facing artifact should still expose posterior marginals, uncertainty, and conflict diagnostics, not just a free-energy scalar.

### Deep Temporal Uncertainty

Active inference does not recommend a flat current-state model for future effects that cannot yet be observed. It recommends a deep temporal generative model where higher-level states summarize slower structure and lower-level states track faster event dynamics.

Answer: the graph can remain a current-state materialization only if the belief layer reasons over trajectories, policies, and future observations. Anchors are present-time slices of a deeper generative structure.

### Epistemic Value And Observation Requests

The report maps `needs_observation` directly to epistemic value, with a constraint: not every uncertain belief should trigger observation.

Answer: unresolved beliefs should generate observation requests when the expected information gain exceeds cost, delay, and opportunity cost. Observation requests should be treated as policies, not exception paths.

### Recent Software-Oriented Literature

The report finds that direct FEP work on agentic software remains smaller than neuroscience and robotics work, but relevant papers exist around graphical models, expected-free-energy planning, behavior trees, long-horizon active inference, event-like abstractions, and multi-agent belief sharing.

Answer: the most transferable ideas are graphical-model message passing for comparators, EFE/GFE planning for delayed effects, and federated or factorized inference for multi-agent belief synchronization.

## Design Consequences

This report turns the active-inference gap into several concrete design requirements:

- Treat the spine ingress/egress adapters as the closest Markov blanket analogue, not the internal graph or `BeliefView`.
- Keep exact Bayesian comparators where tractable; use VFE for approximate inference.
- Use EFE or GFE for policies that choose whether to act, observe, wait, or intervene.
- Add precision or uncertainty fields to planner-facing belief views.
- Let the belief layer emit candidate observation policies, not only settled states.
- Represent future outcomes and trajectories as latent structure when current actions have delayed effects.
- Keep graph anchors as current materialized hypotheses, while the belief layer reasons over hidden causes and policy-conditioned transitions.
- Make `needs_observation` decision-relevant by attaching target belief, observation channel, expected ambiguity reduction, cost, and expiry horizon.

## Suggested BeliefView Fields

The report implies a richer `BeliefView` should eventually include:

- `posterior_beliefs`: posterior marginals over relevant anchors or hidden states
- `precision`: confidence or inverse uncertainty for each belief
- `free_energy_score`: optional internal diagnostic for approximate inference quality
- `expected_free_energy`: policy score when choosing actions or observations
- `epistemic_value`: expected information gain from observing
- `pragmatic_value`: expected progress toward preferred outcomes
- `observation_policy`: candidate observation action when uncertainty is decision-relevant
- `policy_horizon`: expiry or time window over which the policy remains useful
- `future_outcome_beliefs`: latent predictions over delayed outcomes

## What Remains Open

The report leaves several design choices unresolved:

- How tightly the planner should be integrated with the belief layer versus remaining a separate executor.
- Whether EFE/GFE scoring belongs inside comparators, a belief-orchestration layer, or planner policy ranking.
- How to represent deep temporal trajectories without making the graph layer itself responsible for all future-state inference.
- How much of the active-inference vocabulary should become explicit API surface versus internal implementation guidance.

The practical conclusion is architectural: Meld already has the right loop shape, but active inference suggests that belief views should carry uncertainty, precision, and observation/action policy value, not only settled state.
