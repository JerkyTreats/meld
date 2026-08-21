# World Model Research Prompts

Date: 2026-04-26
Status: active
Scope: research prompts for expanding the world model and belief layers, informed by the current architecture and the project's broader ambitions

---

## How To Use These

Each prompt is a self-contained research query designed to be sent to an AI research assistant
or used to drive a literature review session. Each one names the gap it targets and why it
matters to this architecture.

Each prompt should include the architecture context needed to answer it well. Do not assume the
research assistant has access to the rest of this repository or prior design notes.

---

## World Model Architecture

---

### 1. Active Inference and the Free Energy Principle

**Gap:** The architecture has the right shape — spine as sensory input, world model as belief
integrator, agent as actor — but does not draw from the unified theoretical framework that
formalizes this loop.

**Summary:** [Active Inference Gap Summary](world_model_architecture/summary/ACTIVE_INFERENCE_GAP_SUMMARY.md)

**Prompt:**

> I am building a world model layer for an agentic system. The architecture is three layers:
> (1) an event spine that records immutable semantic facts with monotonic sequence; (2) a graph
> layer that materializes current anchors — typed pointers from a subject to a target under a
> perspective; (3) a belief layer that scores credibility of those anchors using Bayesian
> comparators and emits a settled BeliefView that the planner reads. The planner does not touch
> raw facts. The loop is: observe → materialize graph anchor → normalize to evidence → run
> comparator → emit belief view → planner constructs task → task executes → outcome published
> back to spine.
>
> Map this architecture against Karl Friston's Active Inference and the Free Energy Principle.
> Specifically:
>
> (a) Where does the Markov Blanket sit in this architecture — is it at the spine boundary,
> the graph boundary, or the belief view boundary?
>
> (b) How does free energy minimization map to belief revision under conflicting evidence —
> should the comparator minimize variational free energy rather than computing a standard
> posterior?
>
> (c) What does the FEP literature say about how the generative model should be structured to
> support planning under deep temporal uncertainty, where the system cannot observe the future
> effects of its current actions?
>
> (d) What is the relationship between FEP epistemic value and the "needs_observation" belief
> status in my design — should unresolved beliefs generate observation requests as a form of
> active inference?
>
> (e) What papers from 2022–2025 apply FEP to agentic software systems rather than
> neuroscience? Prioritize work that discusses discrete state spaces, event-driven observation,
> and multi-agent coordination.

---

### 2. Latent State World Models from Model-Based RL

**Gap:** The current graph only captures what has been directly observed via domain events.
There is no representation of what has not been seen, what is uncertain because it was not
observed, or how beliefs should propagate through unobserved state transitions.

**Summary:** [Model-Based RL Belief Layer Gap Summary](world_model_architecture/summary/MBRL_BELIEF_LAYER_GAP_SUMMARY.md)

**Prompt:**

> I am designing a belief layer above a temporal knowledge graph. Facts in the graph are
> derived from discrete domain events — not continuous sensor streams. The belief layer must
> handle: (1) beliefs that are credible but aging, because the relevant domain has not emitted
> new events; (2) beliefs where evidence is sparse; (3) beliefs about state transitions that
> were never directly observed but can be inferred from surrounding facts.
>
> Explain how latent state space models from model-based reinforcement learning — specifically
> the Recurrent State Space Model (RSSM) in Dreamer/DreamerV3, TDMPC2, and related systems
> — handle the distinction between observed and latent world state. Specifically:
>
> (a) How does RSSM represent uncertainty about state between observation points — what is the
> form of the posterior distribution it maintains?
>
> (b) How do these models handle the problem of a belief being stale vs. genuinely contradicted
> vs. simply under-observed — is there a formal mechanism for distinguishing these, or is it
> all collapsed into one uncertainty representation?
>
> (c) What techniques from continuous-observation model-based RL are transferable to a discrete
> event-sourced system where observations arrive irregularly and each observation is a typed
> semantic fact rather than a sensor reading?
>
> (d) What does the model-based RL literature say about world models that must represent
> multiple concurrent objects with independent state trajectories — how do object-centric world
> models (OCWM, DreamerV3 with object representations) address combinatorial state explosion?
>
> Give concrete design suggestions for a belief store that needs to represent "not yet observed"
> differently from "observed as contradicted."

---

### 3. Causal World Models

**Gap:** The knowledge graph captures correlation-shaped anchors: when execution of task X
completes, frame Y is selected. The belief layer will eventually need to reason about whether
the quality of Y causally depends on X, or whether there is a confound.

**Summary:** [Causal World Model Gap Summary](world_model_architecture/summary/CAUSAL_WORLD_MODEL_GAP_SUMMARY.md)

**Prompt:**

> I have a temporal knowledge graph with typed anchors. Current anchors record what is
> selected, not why it was selected in a causal sense. The graph knows: "task X completed,
> then anchor A was selected pointing at frame Y." It does not know whether X caused Y to be
> of high quality, or whether both were caused by a third factor.
>
> Survey causal world models in AI research. Specifically:
>
> (a) The line from Pearl's Structural Causal Model and do-calculus to learned causal world
> models — what is the architectural change required to go from a correlational temporal graph
> to a causal one?
>
> (b) How do causal discovery algorithms (PC algorithm, GES, NOTEARS, DCDI) apply to a
> temporal event-sourced graph where interventions are the execution events themselves — can
> task execution records be treated as interventional data?
>
> (c) What is the difference between causal identification from observational data versus
> interventional data in this context, and what does the system gain by treating task
> execution as an explicit intervention node in a causal DAG?
>
> (d) What does the recent literature (2023–2025) say about scalable causal world models for
> agentic systems — specifically systems where the number of causal variables is large and
> the causal structure may change over time?
>
> (e) How should a belief system represent causal uncertainty — is it sufficient to maintain
> Bayesian posteriors over causal graph structures, or is a richer representation needed?

---

### 4. Temporal Knowledge Graphs and Temporal Reasoning

**Gap:** The current system has a solid temporal fact graph implementation but has not drawn
from the temporal knowledge graph research community, which has developed sophisticated
representations for time-varying knowledge and temporal reasoning.

**Summary:** [Temporal KG Gap Summary](world_model_architecture/summary/TEMPORAL_KG_GAP_SUMMARY.md)

**Prompt:**

> I am building a temporal fact graph where every state change is represented as an event with
> a monotonic global sequence number. The graph materializes current anchors from events, and
> supports lineage traversal from current anchor back through prior anchors to the originating
> events. The belief layer above it will add confidence, revision, and calibration.
>
> Survey the temporal knowledge graph literature with a focus on what is useful for a
> belief-enhanced event-sourced system. Specifically:
>
> (a) How do temporal KG embedding systems (TComplEx, TNTComplEx, TimePlex, TERO, Temporal
> TransE) represent the temporal validity of facts — do they use time intervals, event-based
> invalidation, or learned temporal decay, and which is most compatible with an append-only
> event spine?
>
> (b) What does the temporal reasoning literature say about the right semantics for "current"
> when facts have different granularities — some anchors are updated on every task run, others
> persist for months?
>
> (c) How do event-based TKG systems (ICEWS, GDELT, YAGO-15K) model the difference between
> a persistent fact and a transient event, and how does that map to the anchor vs. spine fact
> distinction?
>
> (d) What querying patterns does the TKG literature support that the current system does not
> — for example, temporal projection, temporal joins, or "what was true at time T" queries?
>
> (e) What does the literature say about belief staleness in TKGs — which representations
> (decay functions, validity intervals, event-based invalidation) give the right semantics for
> a belief system that needs to distinguish stale beliefs from contradicted ones?

---

### 5. Non-Stationarity and Regime Detection

**Gap:** The prior calibration loop assumes stationary statistical relationships between
evidence and outcomes. In the macro risk management context that motivates this project,
regime changes break those relationships structurally, invalidating learned priors.

**Summary:** [Non-Stationarity Gap Summary](world_model_architecture/summary/NON_STATIONARITY_GAP_SUMMARY.md)

**Prompt:**

> My world model uses Bayesian comparators that learn priors from past outcomes. The prior
> for a (subject, decision_type) pair is updated each time the system makes a prediction and
> later observes an outcome. This works under the assumption that the relationship between
> evidence and outcomes is stationary.
>
> In Bayesian macroeconomic risk management, regime-switching models (Hamilton, 1989; Kim,
> 1994) handle the case where statistical relationships change structurally — calm periods vs.
> crisis periods, where factor correlations collapse and tail events dominate.
>
> How should a Bayesian belief layer handle non-stationarity? Specifically:
>
> (a) What is the formal criterion for deciding that a prior should be discarded rather than
> updated — when is a regime change detected rather than treated as normal evidence revision?
>
> (b) What does the Bayesian changepoint detection literature say (BOCPD: Adams & MacKay 2007;
> BCP: Barry & Hartigan 1993) about online regime detection, and how should the belief system
> respond when a changepoint is detected?
>
> (c) How do Markov-switching VAR models handle the transition between regimes — should the
> belief system maintain separate priors per detected regime, or use a mixture model?
>
> (d) What does the stress-testing literature (DFAST, CCAR) say about testing belief systems
> against adversarial scenarios where multiple evidence signals are simultaneously extreme —
> how does this translate to adversarial belief evaluation in a knowledge graph?
>
> (e) What is the connection between non-stationarity in belief systems and the "belief storm"
> problem — are they the same phenomenon (high evidence churn) or structurally different?

---

### 6. Predictive Coding and Hierarchical Belief Propagation

**Gap:** The belief layer treats every belief key as independent — evidence flows bottom-up
from facts to beliefs but beliefs never generate top-down predictions about what evidence
should look like. Predictive coding is the research tradition that formalizes bidirectional
message passing in hierarchical models and is the natural next reference point for the
belief architecture.

**Summary:** [Predictive Coding Gap Summary](world_model_architecture/summary/PREDICTIVE_CODING_GAP_SUMMARY.md)

**Prompt:**

> Predictive coding (Rao & Ballard 1999) proposes that a layered inference system should
> not only aggregate evidence upward — each level should also generate a prediction of what
> the level below should observe, and the actual message passed upward is the prediction
> error, not the raw observation. This produces a fundamentally different message-passing
> structure from standard bottom-up Bayesian aggregation.
>
> I want to understand this framework precisely enough to evaluate whether it is the right
> architecture for a hierarchical belief system, and if so, what changes at the design level.
> The context is a discrete, event-sourced system — not a continuous visual processing system.
>
> (a) Write out the formal generative model for a two-level predictive coding system:
> what are the latent variables at each level, what are the conditional distributions,
> and what variational objective is being minimized? Then write the equivalent formulation
> for standard bottom-up hierarchical Bayesian inference (sum-product belief propagation
> on a factor graph). State explicitly where the message-passing equations differ between
> the two.
>
> (b) Precision-weighting is the mechanism by which predictive coding handles heterogeneous
> evidence reliability: the weight given to a prediction error signal is the inverse variance
> of the generative model at that level, not the inverse variance of the observation.
> Explain what behavior this produces that standard likelihood weighting does not. Give a
> concrete numerical example where a low-precision high-magnitude error is suppressed
> relative to a high-precision low-magnitude error, and show what a standard Bayesian
> update would do with the same inputs.
>
> (c) What does the literature say about applying predictive coding to discrete, symbolic,
> or structured domains — not visual processing? Specifically address:
> Whittington & Bogacz (2017) on the equivalence between predictive coding and
> backpropagation; Salvatori et al. (2022) on predictive coding networks as general
> inference machines; and any work applying predictive coding message passing to
> knowledge graphs, ontologies, or structured world representations. What assumptions
> from the continuous-observation formulation break when observations are discrete events
> that arrive irregularly?
>
> (d) Factor graphs with loopy belief propagation also pass messages bidirectionally and
> are a standard tool for inference in structured probabilistic models. What is the
> formal relationship between loopy BP and predictive coding message passing — are they
> equivalent under some parameterization, or do they optimize different objectives?
> Under what conditions does loopy BP converge, and how do those conditions translate
> to a belief graph where the structure is dynamic and the number of levels is not
> fixed at design time?
>
> (e) What are the known failure modes of hierarchical predictive models — specifically,
> when does a strong top-down prediction suppress legitimate bottom-up evidence (the
> hallucination failure mode), and what architectural mechanisms does the literature
> propose to prevent this? Name at least two concrete mechanisms with citations.

---

## Belief Mechanism Prompts

---

### 7. Belief Revision Theory — AGM and Iterated Revision

**Gap:** The belief revision design uses append-only supersession chains, but has not been
formalized against the axiomatic theory of rational belief change.

**Prompt:**

> I am designing the belief revision layer for an event-sourced world model. The system stores
> immutable domain events in an append-only event spine. A graph projection materializes typed
> facts from those events. A belief layer evaluates each candidate belief by appending a
> `BeliefRevision` record rather than mutating prior state. Each revision includes: belief key,
> comparator identifier, evidence set, evidence roles (`support`, `contradiction`,
> `calibration`, `supersession`), confidence score, status (`settled`, `provisional`,
> `contradicted`, `needs_assessment`), timestamp or sequence, and an optional pointer to the
> prior revision it supersedes. Downstream readers consume the latest `BeliefView`, but the
> full revision history remains queryable.
>
> Research how this append-only revision model should be formalized against the belief revision
> literature, especially Alchourrón-Gärdenfors-Makinson (AGM), belief base revision, and
> Darwiche-Pearl iterated revision. Produce a design-oriented answer with citations, not just a
> literature survey. Specifically:
>
> (a) Map the event-sourced `BeliefRevision` semantics to AGM concepts: belief set, belief base,
> revision operator, contraction, expansion, consistency, and closure. Which AGM assumptions
> are violated by immutable evidence history and materialized views?
>
> (b) Evaluate the AGM postulates — success, inclusion, vacuity, consistency, extensionality,
> superexpansion, and subexpansion — against this architecture. For each postulate, state
> whether the design satisfies it, violates it, or needs an explicit policy choice.
>
> (c) Compare AGM revision with belief base revision for an event-sourced system. Should the
> system revise the accepted belief view while preserving immutable evidence, revise a derived
> evidence base, or maintain both a historical base and a current rational closure?
>
> (d) Analyze iterated revision using Darwiche & Pearl's C1-C4 postulates. Does a supersession
> chain over revisions give enough ordering semantics when contradictory evidence arrives over
> time, or is an additional epistemic state, plausibility ordering, priority relation, or
> revision policy required?
>
> (e) Explain how AGM-style revision relates to Bayesian conditioning. When can a Bayesian
> comparator and an AGM-rational revision operator be made coherent, and when do they produce
> incompatible recommendations?
>
> (f) Recommend concrete schema and reducer semantics for `BeliefRevision` and `BeliefView`.
> Include at least two worked examples with records: one where new evidence supersedes a prior
> belief cleanly, and one where conflicting evidence should remain provisional instead of
> forcing a single settled belief.

---

### 8. Epistemic vs. Aleatoric Uncertainty

**Gap:** The comparator emits a single confidence value and an uncertainty field. The design
conflates two structurally different sources of uncertainty.

**Prompt:**

> I am designing a belief layer for an event-sourced agent world model. A comparator receives
> evidence for a candidate belief and emits a confidence score, an uncertainty value, and a
> status such as `settled`, `provisional`, `contradicted`, `needs_observation`, or
> `needs_assessment`. The planner consumes the resulting `BeliefView` to choose actions. The
> current design treats uncertainty as one scalar, but the planner needs to distinguish:
> reducible uncertainty that should trigger observation or research, and irreducible
> uncertainty that should trigger hedging, contingency planning, or risk limits.
>
> Research how to separate epistemic and aleatoric uncertainty in this architecture. Assume the
> system uses discrete symbolic events and comparators, not image classifiers or continuous
> sensors. Produce implementable recommendations with citations. Specifically:
>
> (a) Define epistemic and aleatoric uncertainty formally in Bayesian machine learning,
> including the quantities used in Bayesian neural networks, Gaussian processes, ensemble
> methods, and predictive distributions. Explain which concepts transfer cleanly to symbolic
> event-sourced belief revision and which do not.
>
> (b) Recommend an output representation for `BeliefRevision`: separate epistemic and aleatoric
> fields, posterior intervals, confidence plus variance, Beta or Dirichlet distributions over
> belief truth, entropy decomposition, or another representation. Compare tradeoffs for
> correctness, planner usability, and runtime cost.
>
> (c) Explain how evidential deep learning, prior networks, posterior networks, and related
> uncertainty methods separate data uncertainty from model uncertainty. Identify the parts
> that can be adapted to a discrete evidence store without requiring neural model training.
>
> (d) Design planner policies for each uncertainty type. When should high epistemic uncertainty
> create an observation task? When should high aleatoric uncertainty create a risk mitigation
> task? How should the planner behave when both are high?
>
> (e) Clarify the status semantics. Is `needs_assessment` epistemic uncertainty, missing model
> structure, missing comparator implementation, or something else? Can a belief be
> `settled` while still having high aleatoric uncertainty?
>
> (f) Provide example `BeliefRevision` records for: sparse evidence, contradictory evidence,
> inherently stochastic outcome, missing comparator, and stale evidence. For each example,
> state the recommended status and planner action.

---

### 9. Credal Sets and Imprecise Probabilities

**Gap:** The MissingComparator case outputs "needs_assessment." This is actually deep
uncertainty — not a probability that is unknown, but a situation where the probability
model itself is undefined. The standard Bayesian framework does not handle this cleanly.

**Prompt:**

> I am designing a belief layer for an event-sourced world model. For many belief keys the
> system has a comparator: a domain-specific function that converts evidence into confidence,
> uncertainty, and status. But for some belief keys no comparator exists yet. In that
> `MissingComparator` case, returning a low probability would be misleading: the system is not
> uncertain within a known model; it lacks the model needed to assign a meaningful probability.
> The planner still needs a `BeliefView` that says whether to defer, ask for more evidence,
> request comparator design, or act conservatively.
>
> Research whether credal sets, imprecise probabilities, robust Bayesian analysis, or
> Dempster-Shafer theory provide the right formal treatment for this missing-model case.
> Prioritize practical design guidance for a Rust implementation over abstract generality.
> Specifically:
>
> (a) Define credal sets and imprecise probabilities. Explain how they differ from ordinary
> Bayesian posteriors, probability intervals, confidence intervals, and "unknown probability."
>
> (b) Explain robust Bayesian analysis when the prior, likelihood, or model class is only
> partially specified. What inference methods are practical when the system has sparse
> symbolic evidence rather than dense numeric data?
>
> (c) Compare candidate `BeliefView` representations for missing-model uncertainty: interval
> probabilities, lower and upper previsions, credal sets, p-boxes, Dempster-Shafer belief and
> plausibility, or a structured `needs_model` status. State what each representation lets the
> planner do.
>
> (d) Evaluate decision rules under imprecision: Gamma-minimax, Gamma-maximin, maximality,
> E-admissibility, interval dominance, info-gap decision theory, and abstention or deferral.
> Which rule is appropriate for an autonomous planner that can create observation tasks and
> model-building tasks?
>
> (e) Assess whether Dempster-Shafer theory is a good fit for evidence roles such as
> `support`, `contradiction`, `calibration`, and `supersession`. Include known failure modes,
> such as conflict normalization and unintuitive evidence combination.
>
> (f) Recommend an implementable representation and reducer contract. Include data structures,
> computational complexity, serialization concerns, and two examples: one where evidence is
> sparse but modelable, and one where the comparator itself is missing.

---

### 10. Proper Scoring Rules and Belief Calibration

**Gap:** The prior calibration loop adjusts priors based on outcome history but has not been
formally grounded in the theory of calibration, which defines what it means for a probabilistic
system to be well-calibrated.

**Prompt:**

> I am designing calibration for a belief layer in an event-sourced agent system. A comparator
> predicts whether a candidate belief or planned action will hold, emits a posterior
> probability, and may also emit a binary or categorical decision. Later, outcome events arrive
> on the event spine. A calibration reducer compares prior predictions with realized outcomes
> and updates calibration state for keys such as `(subject, decision_type)`, comparator
> version, domain, or evidence pattern. The goal is not just accuracy; the planner needs
> probabilities that are honest, calibrated, sharp enough to be useful, and robust under
> distribution shift.
>
> Research how proper scoring rules and calibration theory should shape this reducer. Produce
> an implementation-oriented answer with formulas, citations, and design recommendations.
> Specifically:
>
> (a) Define proper and strictly proper scoring rules, including Brier score, log score,
> spherical score, and CRPS. Explain why properness incentivizes honest probability reports
> and how each score behaves near 0 and 1.
>
> (b) Explain calibration, reliability, resolution, refinement, and sharpness. How can a
> system be calibrated but unhelpful, sharp but miscalibrated, or accurate in aggregate while
> unsafe for planning?
>
> (c) Recommend online diagnostics for a streaming event-sourced system: reliability diagrams,
> expected calibration error, maximum calibration error, Brier decomposition, rolling windows,
> exponentially weighted metrics, and subgroup calibration. Which metrics should be stored as
> events or projections?
>
> (d) Compare calibration methods suitable for online or incremental updates: Platt scaling,
> isotonic regression, beta calibration, Bayesian binning into quantiles, Venn-Abers
> prediction, conformal prediction, and hierarchical Bayesian calibration. State which are
> appropriate for sparse per-subject histories.
>
> (e) Explain calibration under non-stationarity. How should the reducer detect drift, separate
> model degradation from environment change, and decide whether to update priors, mark a
> comparator stale, or create a model review task?
>
> (f) Distinguish Bayesian coherence from empirical calibration. Can a calibrated but
> incoherent comparator harm planning? What invariants should the reducer enforce to preserve
> coherence across related beliefs?
>
> (g) Recommend a concrete calibration state schema and update algorithm for a Rust reducer,
> including minimum sample thresholds, versioning, rollback or replay behavior, and examples
> of how miscalibration should change future priors.

---

### 11. Bayesian Non-Parametrics for Prior Construction

**Gap:** The current prior store is a flat per-(subject, decision_type) float. This treats
every subject as independent, wasting information about structurally similar subjects.

**Prompt:**

> I am designing prior construction for a belief layer in an event-sourced agent world model.
> Today, the simplest design stores one scalar prior probability for each
> `(subject, decision_type)` pair and updates it from observed outcomes. This fails when most
> subjects have little history. Many subjects are structurally related: code modules share
> ownership, churn patterns, dependency neighborhoods, task type, runtime surface, or previous
> outcome patterns. The system needs priors that borrow strength across related subjects while
> still allowing individual subjects to diverge as evidence accumulates.
>
> Research Bayesian nonparametric and hierarchical methods for constructing such priors.
> Focus on methods that can be approximated in a background Rust reducer over event-sourced
> data. Specifically:
>
> (a) Explain hierarchical Bayesian partial pooling for per-subject priors. How would a model
> combine global, domain-level, subject-cluster-level, and subject-specific estimates? Compare
> this with empirical Bayes and Stein-James shrinkage.
>
> (b) Explain Gaussian Process priors for smoothing probabilities across similar subjects.
> What kernels could use subject embeddings, graph distance, dependency relationships, or
> metadata features? What are the scaling limits and sparse approximations?
>
> (c) Explain Dirichlet Process Mixtures and the Chinese Restaurant Process as ways to discover
> latent subject groups from outcomes. What would cluster assignment mean for a new subject,
> and how would it affect its prior before much direct evidence exists?
>
> (d) Compare Bayesian nonparametrics with simpler production alternatives: hierarchical
> Beta-Binomial models, logistic mixed-effects models, nearest-neighbor smoothing, feature
> hashing, contextual bandit priors, and online clustering. When is each sufficient?
>
> (e) Describe incremental inference options: conjugate updates, online EM, variational
> inference, assumed density filtering, particle filters, and periodic batch refits with
> event replay. Which are realistic for a reducer that must update continuously?
>
> (f) Recommend a prior state schema. Include how to version model assumptions, store feature
> vectors or embeddings, track effective sample size, avoid leaking future outcomes into past
> priors during replay, and expose uncertainty to the planner.
>
> (g) Provide a worked example where a new subject has no history but related subjects do.
> Show how the recommended method initializes the prior, updates it after outcomes arrive,
> and avoids overconfidence.

---

### 12. Information-Theoretic Planning and Epistemic Value

**Gap:** The planner currently selects tasks toward goal states. There is no formal mechanism
for selecting tasks whose primary value is resolving epistemic uncertainty in the world model.

**Prompt:**

> I am designing a planner for an agent system that reads a `BeliefView` from an event-sourced
> world model. Some beliefs are settled enough for action. Others are unresolved because
> evidence is sparse, stale, contradictory, high-uncertainty, or missing a comparator. The
> planner can create ordinary goal-directed tasks, but it can also create observation tasks:
> invoke a capability, run a check, query an external system, inspect state, or ask for model
> assessment. Observation has cost and may delay goal progress, so the planner needs a
> principled way to decide which uncertainty is worth resolving.
>
> Research information-theoretic planning and epistemic value for this setting. Focus on
> discrete symbolic beliefs, event-sourced evidence, and bounded computation. Produce formulas,
> citations, and concrete planner policy recommendations. Specifically:
>
> (a) Define epistemic value in active inference, Bayesian decision theory, and reinforcement
> learning. Compare expected information gain, expected entropy reduction, KL divergence from
> prior to posterior, value of information, and expected value of perfect or partial
> information.
>
> (b) Map observation tasks to Bayesian experimental design. What is the "experiment," what is
> the hypothesis space, what is the utility function, and how should observation cost,
> latency, reliability, and side effects be represented?
>
> (c) Compare prioritization rules for multiple unresolved beliefs: uncertainty sampling,
> expected information gain, expected model change, query-by-committee, Thompson sampling,
> upper confidence bounds, myopic value of information, and active inference expected free
> energy. Which are practical for a planner over symbolic `BeliefView` records?
>
> (d) Explain how POMDPs formalize the tradeoff between acting on current beliefs and gathering
> observations. Are POMCP, SARSOP, DESPOT, QMDP, or simpler receding-horizon approximations
> realistic for large event-sourced state spaces?
>
> (e) Design a planner scoring function that combines goal value, confidence, epistemic value,
> aleatoric risk, observation cost, staleness, and dependency unlock value. Include a
> pseudocode-level algorithm that can run without a full POMDP solver.
>
> (f) Identify safety risks of curiosity-driven or information-seeking objectives, including
> noisy-TV behavior, reward hacking, privacy leakage, endless exploration, and observations
> that change the world being observed. Recommend guardrails and budget constraints.
>
> (g) Provide worked examples: one unresolved belief that should trigger observation, one that
> should be ignored because observation value is low, and one high-aleatoric-risk belief that
> should trigger hedging rather than information gathering.

---

## Integration Prompts

---

### 13. Multi-Agent Belief Synchronization for Mass Agents

**Gap:** The current architecture assumes a single curation process maintaining the world
model. The project's goal of mass agents requires a design where many concurrent agents
can read and act on a shared world model without creating belief inconsistencies.

**Prompt:**

> My world model architecture uses a single event spine and a single curation process that
> materializes beliefs from spine events. Multiple execution agents will eventually read the
> shared BeliefView projections concurrently. When multiple agents act simultaneously, their
> actions produce new spine events that may invalidate each other's planning assumptions.
>
> Survey multi-agent belief synchronization approaches. Specifically:
>
> (a) What does epistemic game theory say about how agents should reason about other agents'
> beliefs when acting in a shared world? What is the formal notion of common knowledge vs.
> mutual knowledge, and does my architecture need to support either?
>
> (b) How do cooperative multi-agent RL systems (QMIX, MAPPO, MADDPG) handle the
> centralized-critic / decentralized-actor pattern — does the shared BeliefView play the
> role of the centralized critic, and if so, what are the staleness guarantees required?
>
> (c) What does the distributed Bayesian inference literature say about maintaining a
> coherent belief state across multiple concurrent belief updaters — what are the
> convergence guarantees for distributed variational inference, and when do they fail?
>
> (d) What does the CRDTs literature say about designing append-only belief stores that
> are convergent under concurrent writes — can a BeliefRevision store be made into a CRDT,
> and what semantics does that impose on the supersession chain?
>
> (e) What is the difference between a shared world model (all agents read the same state)
> and a federated world model (each agent maintains its own model with synchronization),
> and which is more appropriate for a system where agents may have different observation
> domains?

---

### 14. Connecting Macro Risk Factor Models to Belief Comparators

**Gap:** The system's Bayesian comparator uses a weighted logistic factor model. This
structure is homologous to Fama-French multi-factor risk models. The macro risk connection
is not yet exploited for richer comparator design.

**Prompt:**

> My Bayesian comparator uses a weighted additive logistic factor model: normalized evidence
> factors are combined into a posterior using logit space. This structure is isomorphic to a
> Fama-French-style risk factor model, where systematic risk factors drive a common posterior.
>
> I want to deepen this connection, drawing from quantitative finance and macroeconomic
> risk management literature. Specifically:
>
> (a) How does factor decomposition in Fama-French and its extensions distinguish systematic
> risk (factor-driven, affecting all subjects of a type) from idiosyncratic risk (subject-
> specific), and how should the comparator represent this split in its factor breakdown?
>
> (b) The Kalman filter is the update step in a dynamic factor model (Stock & Watson; Doz,
> Giannone & Reichlin). How does a Kalman update compare to a logit-space posterior update,
> and when should the system use Kalman-style estimation instead of the logistic model?
>
> (c) How do copula models (Gaussian copula, t-copula, Clayton copula) handle dependence
> structure between factors when factors are correlated under crisis regimes — the correlation
> collapse problem that broke Gaussian copula CDO pricing in 2008? How should the factor
> model handle correlated evidence signals?
>
> (d) What does the DFAST / CCAR stress-testing literature say about testing belief systems
> against adversarial evidence scenarios — simultaneous extremes across multiple factors?
> How should the belief system be designed to report explicitly when multiple factors are
> in simultaneous adverse states?
>
> (e) What is the connection between Value-at-Risk (VaR) / Expected Shortfall (CVaR) in
> macro risk and a tail-aware confidence bound in the belief comparator — how should the
> comparator report tail risk rather than just a posterior mean?

---

### 15. Belief Storm Resilience and Streaming Consistency

**Gap:** The belief substrate design mentions "storms" — high-churn periods where rapid
evidence ingestion could produce contradictory or rapidly-superseding belief revisions.
The current design has placeholders for storm handling but no formal mechanism.

**Prompt:**

> My belief substrate will face periods of high-frequency evidence ingestion — when a large
> batch of domain events arrives, many beliefs may be revised in rapid succession, potentially
> emitting contradictory BeliefViews that the planner reads during the storm. This is called
> a belief storm.
>
> Existing mitigation ideas include leases (a belief cannot be revised more than once per
> lease window) and debouncing (revision is deferred until the event rate drops). But these
> are informal.
>
> Survey storm-resilient architectures from relevant literature. Specifically:
>
> (a) What does the database literature call this problem and how do MVCC (multi-version
> concurrency control) systems ensure read consistency during high write rates — can the
> BeliefView be versioned and snapshotted so that planner reads are isolated from storm
> revision churn?
>
> (b) What does stream processing literature (Kafka Streams, Apache Flink, KSQLDB) say about
> windowing and late event handling for exactly this class of problem — how do tumbling
> windows, session windows, and watermarks apply to belief revision batching?
>
> (c) What are the formal properties of debouncing and throttling from control theory —
> specifically, what are the delay vs. consistency tradeoffs, and how should lease window
> length be chosen?
>
> (d) Can a BeliefRevision store be modeled as a CRDT (Conflict-free Replicated Data Type)
> so that concurrent writes always converge without coordination — what CRDT design supports
> supersession semantics?
>
> (e) What is the formal notion of eventual consistency for belief views, and what
> consistency model (linearizable, sequential, causal, eventual) does the planner actually
> need from the world model — is linearizability necessary, or is causal consistency
> sufficient for safe planning?

---

## Read With

- [Belief](belief/README.md)
- [Fact To Belief](belief/fact_to_belief.md)
- [Comparator Model](belief/comparator_model.md)
- [Belief Substrate](belief/substrate.md)
- [World Model Graph](../world_model/graph/README.md)
- [Observe Merge Push](../observe_merge_push.md)
- [Bayesian Evaluation Example](../execution/examples/bayesian_evaluation.md)
