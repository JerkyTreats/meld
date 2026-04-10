# Bayesian Evaluation Capability

Date: 2026-04-08
Status: proposed
Scope: deterministic probabilistic scoring capability that combines evidence artifacts into a structured decision artifact

## Intent

Combine typed evidence artifacts into a single posterior probability estimate and a binary
dispatch decision.

This is not an LLM call. It is a deterministic weighted scoring function over typed numeric
inputs. The same evidence with the same bindings produces the same output every time.

## Boundary

This capability:

- accepts typed evidence artifacts (ChangeSummary, AstImpact)
- reads a prior probability from bindings or a prior store
- applies a weighted factor model to compute a posterior probability
- emits a `DecisionArtifact` with the posterior, the binary decision, and a factor breakdown

This capability does not:

- generate text
- call a provider
- persist state
- update the prior (that is a reducer concern over the event spine)

## Runtime Initialization

Runtime initialization should hold:

- capability instance identity
- `prior_probability` binding: f32, default 0.3
  The baseline probability that documentation is outdated at any snapshot.
- `factor_weights` binding: named weight map, default weights defined per factor below
- `decision_threshold` binding: f32, default 0.6
  Posterior probability above which `should_execute` is set to true.
- `prior_store_ref` binding: optional reference to a sled-backed prior projection keyed by
  `(node_id, decision_type)`. When present, the stored prior overrides the static binding.

## Input Slots

- `change_summary` — `ChangeSummary` artifact, required
- `ast_impact` — `AstImpact` artifact, optional

Missing optional evidence does not fail the evaluation. Each absent factor is dropped from
the weighted sum and the remaining weights are renormalized. This is the standard Bayesian
treatment of missing observations.

## Factor Model

The first slice uses a weighted additive logistic model.

### Factors and Default Weights

| Factor | Source | Weight | Normalization |
|---|---|---|---|
| `age` | `change_summary.days_since_last_doc_update` | 0.35 | `min(days / 30.0, 1.0)` |
| `churn` | `change_summary.lines_added + lines_removed` | 0.30 | `min(lines / 150.0, 1.0)` |
| `api_change` | `ast_impact.public_api_changed` | 0.25 | `1.0 if true, 0.0 if false` |
| `commit_rate` | `change_summary.commit_count_since_reference` | 0.10 | `min(commits / 10.0, 1.0)` |

When `no_prior_record` is true in the `ChangeSummary`, `age` is set to `1.0` (maximum signal).

### Posterior Computation

```
evidence_score = sum(weight_i * normalized_factor_i) / sum(weight_i for present factors)

logit_prior = ln(prior / (1 - prior))
logit_posterior = logit_prior + evidence_score * sensitivity_scale

posterior = sigmoid(logit_posterior)
should_execute = posterior >= decision_threshold
```

`sensitivity_scale` is a fixed constant (default 3.0) that controls how strongly evidence
moves the posterior away from the prior. It is not a per-invocation binding.

This model is intentionally simple. Its value is in making the decision quantifiable and
inspectable, not in modeling complex interactions. Factor interactions can be added later
without changing the capability contract.

## Artifacts Out

```json
{
  "artifact_type_id": "decision_artifact",
  "schema_version": "v1",
  "content": {
    "node_id": "9f6d8d7f...",
    "decision_type": "docs_writer_update",
    "probability": 0.74,
    "should_execute": true,
    "prior_used": 0.3,
    "threshold_used": 0.6,
    "contributing_factors": [
      { "factor": "age",        "normalized_value": 0.47, "weight": 0.35 },
      { "factor": "churn",      "normalized_value": 0.31, "weight": 0.30 },
      { "factor": "api_change", "normalized_value": 1.0,  "weight": 0.25 },
      { "factor": "commit_rate","normalized_value": 0.80, "weight": 0.10 }
    ],
    "absent_factors": []
  }
}
```

`contributing_factors` records every factor used and its contribution. This makes the decision
inspectable and debuggable. It is also the material a future calibration reducer needs to
correlate predictions against outcomes.

## Failure Shape

```json
{
  "failure_kind": "InsufficientEvidence",
  "message": "change_summary absent; evaluation cannot proceed",
  "details": {}
}
```

`change_summary` is the only required input. If it is absent, the capability fails rather than
guessing. `ast_impact` absence is a soft degradation, not a failure.

## Prior Store Integration

When `prior_store_ref` is bound, the capability reads:

```
prior_store.get((node_id, "docs_writer_update")) -> Option<f32>
```

If present, the stored prior overrides `prior_probability`. If absent, the static binding is
used. This means the first invocation for any node uses the static prior, and subsequent
invocations may use a refined prior if the prior reducer has updated it.

The prior store is a sled namespace maintained by a reducer that subscribes to
`task_artifact_emitted` events for `decision_artifact` and the subsequent `frame_written` events
for the same node. That reducer is defined separately and is not owned by this capability.

## Publication Rule

Published as a task-facing capability because:

- its output is a typed structured artifact consumed by control dispatch logic
- it is schedulable independently from generation work
- it is the terminal step in the evidence chain and produces the `DecisionArtifact` that
  control reads at the `branch` node

## Read With

- [Git Diff Summary](../git_diff_summary/README.md)
- [AST Change Impact](../ast_change_impact/README.md)
- [Impact Assessment](../../../control/impact_assessment.md)
- [Guard Binding Semantics](../../../control/program/guard_binding_semantics.md)
- [Capability Model](../README.md)
