# Goal Curation

Date: 2026-05-10
Status: active
Scope: the world model agent's process for curating execution's goal set through cost-benefit evaluation over belief

## Thesis

Goal curation is the agent's mechanism for translating belief state into execution commitment. The agent watches belief revisions, evaluates them through cost-benefit comparators, and emits goal mutations through execution's public API.

This is not a novel mechanism. It is the belief→goal instance of the same watching/reducing pattern that operates at every layer boundary in the architecture:

- Sensory workers watch external sources, emit typed observations
- Graph reducers watch spine events, materialize anchors
- Belief comparators watch graph anchors and facts, produce belief revisions
- **The agent watches belief revisions, produces goal mutations**
- The planning loop watches the goal set, produces task network mutations

At each boundary, the watcher reduces frequency and increases connectivity. Sense data is high-frequency and isolated. Events are discrete and typed. Facts have some connections to other facts. Beliefs have high connectivity to facts, other beliefs, and agents. Goals have the highest connectivity — connecting beliefs, cost data, regime context, and active goal state.

The frequency reduction is the performance model. The agent's goal curation can afford expensive cross-referencing (cost beliefs, value beliefs, active goal set, regime context) precisely because it fires at the lowest frequency in the system. Most belief changes are absorbed without producing a goal mutation.

## Cost-Benefit Evaluation

Goal generation is a Bayesian cost-benefit comparison. The decision to add, modify, or remove a goal uses the same comparator pattern as belief assessment, but applied to action-worthiness rather than truth-assessment.

The inputs:

- **State belief**: the current posterior on the relevant belief key (e.g., "semantic context is stale by 14 files, confidence 0.85")
- **Cost belief**: the expected cost of pursuing the goal, learned from execution outcomes (e.g., "semantic conversion historically costs 10 minutes, X tokens")
- **Value belief**: the expected value of achieving the desired state, learned from downstream outcome correlation (e.g., "when semantic context is fresh, downstream planning decisions improve by Y")
- **Inaction cost**: the accumulating cost of not acting, relevant for maintenance invariants (e.g., "stale semantic context has been degrading decision quality for N hours")
- **Regime context**: which prior set is active for cost-benefit evaluation

The comparison:

```
evidence: divergence magnitude, cost posterior, value posterior, inaction accumulation
prior: "was acting on similar divergence historically worth it?" (regime-scoped)
posterior: act / tolerate (with confidence)
→ if act: emit goal mutation through execution's curation API
→ if tolerate: absorb belief change, no goal mutation
```

The "tolerate" outcome is the frequency reduction. Most belief changes don't cross the cost-benefit threshold. The goal layer fires less often than the belief layer.

## Cost Beliefs

Cost beliefs are derived from execution outcomes. When execution completes a task, it publishes outcome facts to the spine:

- elapsed time
- resource consumption (tokens, compute, etc.)
- success or failure
- retry count
- actual vs estimated cost

These outcome facts flow through the normal belief pipeline: spine → facts → evidence → belief revision. The result is a belief about execution cost for each class of action.

Cost beliefs calibrate over time. As execution gets cheaper (better tools, cached results, optimized capabilities), the cost posterior drops. As it gets more expensive, the posterior rises. Goal generation responds automatically — more goals pass the threshold when costs are low, fewer when costs are high.

## Value Beliefs

Value beliefs are derived from outcome correlation. When the agent acts on a divergence and the resulting belief revision produces downstream improvement, the value posterior strengthens. When acting produces no meaningful downstream change, the value posterior weakens.

Value beliefs require longer calibration windows than cost beliefs because the causal chain from goal → execution → outcome → downstream belief change is longer and noisier. Value beliefs should carry wider uncertainty initially and narrow as outcome data accumulates.

This is where goal curation connects to the causal layer. The question "does acting on this divergence actually produce downstream value?" is a causal question. The causal layer's intervention and outcome semantics can feed value belief assessment, though the first slice can use simpler outcome correlation.

## User Input as High-Weight Evidence

User-directed input is not a bypass of cost-benefit evaluation. It is high-weight evidence on the value side.

```
User says: "Fix the tests"
  → high-weight value evidence enters the cost-benefit comparator
  → posterior on "act" is overwhelming (user weight dominates cost)
  → goal generated through same mechanism
  → cost belief still computed and preserved
```

Cost data is preserved even for user-directed goals because:

- Cleanup needs cost estimation if the goal is later abandoned
- The system can surface cost to the user ("this will take approximately 45 minutes and X tokens")
- Future cost beliefs calibrate from all executions, including user-directed ones

The same mechanism handles all goal sources. `BeliefDivergence`, `UserDirected`, `Maintenance`, and `GoalDecomposition` sources are not different pathways — they are different evidence profiles entering the same cost-benefit comparator. User input is high-weight value evidence. Maintenance invariant violation is accumulating inaction cost. Belief divergence is the standard case.

## Regime Change and Prior Scoping

Learned priors become a burden when the landscape fundamentally shifts. If the agent learned cost-benefit priors during normal development, those priors are structurally wrong during incident response — not slightly wrong, but wrong in kind. The value of documentation goals drops to near zero. The value of stability goals spikes. Same cost-benefit comparators, completely different landscape.

The regime layer detects structural shifts. When a regime change is detected, the agent's cost-benefit comparators scope their priors to the new regime:

**If the regime has been seen before**: retrieve archived priors from the regime library. "During the last incident, stability goals had high value, documentation goals had near-zero value." Immediate recalibration without learning from scratch.

**If the regime is novel**: widen uncertainty on all cost-benefit priors. The agent becomes exploratory — less confident in act/tolerate decisions, more likely to generate observation goals rather than action goals. Prior narrowing resumes as new outcome data arrives under the new regime.

**When the regime ends**: archive the current regime's priors in the regime library. Old priors are preserved, not erased. When this regime recurs, calibrated priors are available.

This connects directly to the regime layer's existing concepts:

- `RegimeLibrary` indexes archived prior sets, including cost-benefit priors per concern class
- `RegimePosterior` informs which prior set is active
- `ChangepointState` triggers prior scoping transitions
- `RegimeEntryPrior` provides the starting prior when a new regime segment begins

## Belief Monitoring

The agent monitors beliefs through subscription to belief revision events. This is the "watching" mechanism at the belief→goal boundary.

### Subscription binding

The agent's subscriptions are bound during the bootstrap lifecycle (see [Agent Lifecycle](README.md#agent-lifecycle)). Agent creation is an execution goal. Capabilities invoke the world model's public interface to:

1. Survey existing beliefs and evidence channels for the agent's subject scope
2. Register the agent identity and perspective
3. Register belief keys for dimensions that should exist but don't
4. Bind subscriptions to each relevant belief key

The subscription filter — which belief keys the agent watches — is derived from directive decomposition during bootstrap, not declared statically. Execution decomposes the semantic directive into concrete belief dimensions. The world model's public interface provides the traversal and registration operations. See [World Model Public Interface](../public_interface.md).

The subscription filter is the agent's definition of "what I care about." It does not define what to do about changes — the cost-benefit comparator handles that. It defines which changes reach the comparator at all.

### Event-driven evaluation

When a belief revision event arrives for a watched belief key:

1. Agent reads the updated belief view
2. Agent evaluates the cost-benefit comparator for that concern class
3. Agent checks the active goal set for redundancy and coherence
4. Agent emits goal mutation (or absorbs the change)

### Freshness-driven evaluation

The "nothing changed and that's a problem" case does not require a separate periodic sweep. The belief layer tracks freshness. When freshness decays past a belief's staleness threshold, the belief layer emits a freshness-decay revision. The agent watches this like any other belief revision.

This means even staleness-triggered goals go through the same cost-benefit evaluation. "Semantic context hasn't been checked in 4 hours" is evidence. The cost-benefit comparator decides whether that staleness warrants action given current cost and value beliefs.

### Active goal feedback

The agent reads the active goal set when evaluating. This provides:

- **Redundancy check**: if a goal already addresses this belief divergence, don't generate another
- **Progress monitoring**: if a goal has been active for N cycles without belief movement, evaluate whether to modify (adjust approach), escalate (raise priority), or abandon (cost exceeds remaining value)
- **Coherence check**: if the active goal set implies certain beliefs should be changing, flag incoherence when they aren't
- **Prediction**: active goals predict expected evidence. Evidence that matches predictions is less surprising. The agent can damp its response to expected belief changes from another agent's active execution

## The Decision Loop

The complete agent decision loop for goal curation:

```
belief revision event arrives (or freshness decay fires)
  → is this belief key in my subscription filter?
  → read current belief view for the affected key
  → read cost belief for this concern class
  → read value belief for this concern class
  → read regime context (which prior set is active)
  → read active goal set
  → run cost-benefit comparator:
      evidence: divergence, cost posterior, value posterior, inaction cost
      prior: regime-scoped learned prior for this concern class
      posterior: act / tolerate
  → if tolerate: done (frequency reduction — most changes absorbed here)
  → if act:
      → is there an existing goal addressing this? modify if needed
      → is there goal conflict? evaluate priority, may suspend other goals
      → emit goal mutation: add / modify / suspend / satisfy / abandon
```

Satisfaction checking follows the same watching pattern:

```
belief revision event arrives
  → does this belief now satisfy an active goal's desired state?
  → evaluate satisfaction criteria (predicate, confidence, freshness, stability)
  → if satisfied: emit satisfy mutation through execution's API
```

Satisfaction can occur from any source — the system's own execution, external action, or unrelated changes. The agent detects it the same way: through belief revision matching a goal's satisfaction criteria.

## Relationship to Comparator Model

The cost-benefit comparator is a new comparator family alongside the existing belief comparators (Bayesian, Rule, SemanticSettlement, Missing, MessagePassing, PredictiveResidual).

The distinction:

- **Belief comparators** answer: "what should be believed about the world?"
- **Cost-benefit comparators** answer: "should the agent act to change the world?"

The inputs differ. Belief comparators consume evidence about state. Cost-benefit comparators consume beliefs about state, cost, and value — they are comparators over comparator outputs. The mechanism is the same: prior + evidence → posterior with confidence and uncertainty.

The cost-benefit comparator should meet the same requirements as belief comparators:

- Deterministic replay over explicit inputs
- Inspectable posterior and decision
- Calibration from later outcomes
- Provenance over inputs and decision

## Cold Start

Before the system has execution history, cost beliefs are uninformed. Three sources provide initial priors:

1. **Capability contracts**: capabilities can declare estimated cost envelopes (time, tokens, resource class). These are weak priors but better than nothing.
2. **Explicit configuration**: an agent can be initialized with cost priors for its concern classes. These are manually set and should be marked as uncalibrated.
3. **Uninformative priors**: when no cost data exists, the comparator defaults to wide uncertainty. The practical effect is that the agent acts on strong divergences (where value clearly dominates uncertain cost) but abstains on marginal ones until cost data arrives.

Value beliefs are similarly cold at start. The first slice should default to acting on strong divergences with clear value signals (user-directed, maintenance invariant violation) and deferring marginal ones until outcome data calibrates the value posterior.

## The Normative Framework, Reduced

The normative framework concepts discussed in the goal model reduce to cost-benefit posteriors:

| Normative concept | Realized as |
|---|---|
| Concern declaration | Subscription filter on belief keys |
| Divergence threshold | Cost-benefit posterior hasn't crossed decision boundary |
| Tolerance policy | Region where expected cost exceeds expected value — derived, not configured |
| Priority | Value-to-cost ratio — computed, not assigned |
| Regime sensitivity | Which prior set the cost-benefit comparator uses |
| Maintenance invariant | Concern with accumulating inaction cost |

The normative framework is: which belief keys the agent watches, and what regime-scoped priors it carries for the cost-benefit comparison on each. That is a small, learnable, inspectable thing.

## What This Design Does Not Cover

### Cost-benefit comparator specification

The shape of the comparator is defined. The specific factors, weights, and decision boundary require implementation design and will calibrate from experience.

### Multi-agent goal coordination

When multiple agents' cost-benefit evaluations produce conflicting goals, coordination is needed. The shared task network provides structural coordination (shared dependencies). Normative coordination (which agent's goals take priority when they conflict) is not specified.

### Value measurement methodology

How to measure downstream value of goal achievement is a research question. Simple outcome correlation may suffice initially but may not capture long-term or indirect value. The causal layer's intervention semantics are the eventual foundation for rigorous value assessment.

### Subscription filter refinement

The subscription filter is derived from directive decomposition during bootstrap (see [Agent Lifecycle](README.md#agent-lifecycle)). How subscriptions evolve over time — narrowing to high-value belief keys, expanding when new evidence channels appear — is not fully specified. Re-survey on capability catalog changes provides the mechanism but the policy is not defined.

## Read With

- [World Model Agent](README.md)
- [World Model Public Interface](../public_interface.md)
- [Comparator Model](../belief/comparator_model.md)
- [Goals](../../execution/goals/README.md)
- [Regime Layer](../regime/README.md)
- [Belief](../belief/README.md)
- [Fact To Belief](../belief/fact_to_belief.md)
- [Observe Merge Push](../../observe_merge_push.md)
