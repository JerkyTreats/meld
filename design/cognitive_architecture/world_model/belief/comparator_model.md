# Comparator Model

Scope: comparator and inference families used to assess evidence, update posterior state, and settle belief revisions

## Thesis

Comparators are the assessment boundary between evidence and belief.

A comparator consumes evidence for one `BeliefKey` and emits one proposed `BeliefRevision`.
The comparator does not own fact ingestion, scheduling, task execution, or planner policy.
The comparator also does not own belief family content.
Family-specific factors, priors, evidence bindings, thresholds, and projection rules are runtime configuration.

Bayesian comparators are preferred because they are typed, inspectable, replayable, and calibratable.
Semantic settlement is allowed as a provisional shortcut when no stronger comparator exists.

Comparator is one inference method. Some beliefs use simple comparator updates. Others use factor-graph messages, variational updates, predictive residuals, or hypothesis-set scoring. Every method provides deterministic replay over explicit inputs and an inspectable posterior or revision output.

## Inputs

Every comparator engine receives:

- belief key
- perspective or evidence policy when material
- prior belief revision when available
- evidence items
- source sequence range
- comparator engine configuration from a runtime config snapshot
- provenance bundle

Every comparator engine returns:

- belief status
- posterior summary
- confidence
- uncertainty
- precision or reliability notes where available
- contributing evidence ids
- contradicted evidence ids
- observation need when uncertainty remains decision-relevant
- calibration fields when available
- explanation summary
- next suggested posture as an advisory signal only

## Comparator Families

`BayesianComparator`

Preferred comparator for typed evidence.
It updates a prior with measured factors and records a posterior.
The factor names, evidence schema bindings, weights, normalizers, and projection fields come from runtime configuration.
The comparator configuration declares the complete evidence model and deterministic posterior calculation.

`RuleComparator`

Deterministic comparator for narrow cases where evidence maps directly to status.
Examples include impossible version ranges, explicit task failure states, and hard capability contract mismatches.

`SemanticSettlementComparator`

LLM or heuristic driven comparator for beliefs that lack typed evidence math.
It is cheap to implement, but expensive in tokens and weaker in rigor.
It must mark the revision as provisional unless an explicit policy says otherwise.

`MissingComparator`

No comparator exists for the belief key.
The result should be `needs_assessment` or `needs_observation`, not a guessed belief.

`MessagePassingInference`

Structured inference for beliefs whose evidence forms a discrete graph, hierarchy, or factorized model.
It may use exact constraints, belief propagation, variational messages, or local likelihood gradients.
It should still publish a replayable `BeliefRevision` or posterior summary rather than exposing raw message state as the public contract.

`PredictiveResidualInference`

Narrow inference for cases where a higher-level belief predicts a lower-level continuous or modelable state and a residual representation is valid.
It should carry precision and damping metadata.
It should not be used as the default representation for symbolic event mismatch.

## Trust Policy

Comparator kind affects default trust.

| Comparator | Default status | Default trust | Calibration path |
| --- | --- | --- | --- |
| `BayesianComparator` | settled | high | compare posterior to later outcomes |
| `RuleComparator` | settled | medium to high | validate rule failures and false positives |
| `SemanticSettlementComparator` | provisional | low to medium | replace with typed comparator when possible |
| `MissingComparator` | unresolved | none | synthesize or implement a comparator later |
| `MessagePassingInference` | provisional or settled | method dependent | compare posterior and convergence diagnostics to later outcomes |
| `PredictiveResidualInference` | provisional or settled | method dependent | calibrate precision and residual thresholds against later observations |

## Cost-Benefit Comparator

The comparator pattern extends beyond truth-assessment to action-worthiness. The world model agent uses cost-benefit comparators to decide whether belief divergence warrants goal generation.

A cost-benefit comparator consumes beliefs about state, cost, and value — it is a comparator over comparator outputs. The mechanism is the same: prior + evidence → posterior with confidence and uncertainty.

| Input | Source |
| --- | --- |
| State belief | Belief comparator output for the relevant belief key |
| Cost belief | Learned from execution outcome facts such as time, tokens, and success rate |
| Value belief | Learned from downstream outcome correlation |
| Inaction cost | Accumulating cost of not acting for maintenance invariants |
| Regime context | Prior set selected by the Regime domain |

The cost-benefit comparator produces an act/tolerate posterior. "Tolerate" means the belief change is absorbed without goal generation — this is the frequency reduction between the belief layer and the goal layer.

The full mechanism is defined in [Goal Curation](../agent/goal_curation.md).

Semantic settlement should carry:

- comparator kind
- prompt or policy version
- evidence ids used
- source sequence range
- confidence generated by the comparator
- trust downgrade applied by policy
- expiry pressure

## Supporting Contracts

- [Observation Wait Semantics](../../execution/planning/observation_wait_semantics.md)
  data-flow dependency semantics for observation tasks
- [Guard Expression Semantics](../../execution/planning/guard_expression_semantics.md)
  conditional dependency edge evaluation over structured artifacts
- [Goal Curation](../agent/goal_curation.md)
  cost-benefit comparator for Goal generation and action-worthiness

## Bayesian Comparator Shape

The Bayesian comparator should be deterministic for the same inputs.

It should record:

- prior source
- prior value
- evidence factors
- factor weights
- posterior value
- posterior uncertainty
- decision threshold if one is used
- calibration target

The first practical comparator can be configured to mirror the docs writer example:

- `ChangeSummary` can supply churn, age, and commit rate through runtime evidence schema bindings
- `AstImpact` can supply public API change through runtime evidence schema bindings
- posterior decides whether the belief supports action
- later execution outcome calibrates the prior

The Bayesian comparator remains a generic weighted engine.
It must not introduce a comparator type named after `docs_freshness` or any other runtime family.

## Semantic Settlement Policy

Semantic settlement exists to keep the system moving.
It should not become the hidden core of belief.

Use semantic settlement when:

- evidence is available but no typed comparator exists
- the belief is useful for planning but low risk
- the system can tolerate provisional status
- token cost is acceptable

Do not use semantic settlement when:

- safety depends on the belief
- the result drives irreversible side effects
- typed evidence is already available
- the belief storm rate is high

## External Capability Edge

External capability synthesis is adjacent but deferred.

When a comparator needs evidence that available observations cannot produce, Belief emits an observation opportunity for Agent and Strategy consideration.
Execution may later synthesize or register a capability that gathers the missing evidence.

This edge is an observation opportunity. It carries expected information gain, target belief, evidence channel, cost, delay, and expiry when those fields are known.

For now, the belief layer should record:

- missing evidence type
- affected belief key
- comparator that requested it
- suggested artifact type
- source evidence that made the gap visible

## Read With

- [Belief Microarchitecture](microarchitecture.md)
- [Fact To Belief](fact_to_belief.md)
- [Belief Substrate](substrate.md)
- [Execution Planning](../../execution/planning/README.md)
