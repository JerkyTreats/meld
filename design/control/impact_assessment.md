# Impact Assessment

Date: 2026-04-08
Status: proposed
Scope: ImpactAssessment as the first concrete observation task, defined for docs_writer but general in pattern

## Intent

Define `ImpactAssessment` as the first observation task in the control system.

An observation task gathers quantified facts about the world and produces a structured
decision artifact. Control then uses that artifact at a `branch` node to decide whether
to dispatch downstream work.

The concrete motivation is the docs_writer problem: any file change currently triggers a
full rebuild because there is no method for assessing the impact level of a change.
`ImpactAssessmentTask` is the answer to that problem.

## Vocabulary Note

The term `Decision` is reserved for architectural design decisions in this repository's
documents. The control concept described here is called `ImpactAssessment`.
The output artifact is a `DecisionArtifact`. The task is an `ImpactAssessmentTask`.

## Pattern

An observation task is any task whose primary output is a typed assessment artifact rather
than a workspace effect. It exists to inform control, not to change anything.

The pattern is always:

```
[dispatch ImpactAssessmentTask]
       |
[await_observation: decision_artifact]
       |
[branch: guard=decision_artifact.should_execute]
  true  |                     | false
[invoke DocsWriter]       [skip this node]
```

This maps directly onto existing control graph primitives.
No new control node types are required for this pattern.

## ImpactAssessmentTask Definition

The task compiles a three-capability chain:

```
git_diff_summary
    |
ast_change_impact    (depends on: change_summary from git_diff_summary)
    |
bayesian_evaluation  (depends on: change_summary, ast_impact)
    |
    -> DecisionArtifact
```

`git_diff_summary` and `ast_change_impact` are parallel-safe once `git_diff_summary` has
completed, since `ast_change_impact` depends on `change_summary` for the file list but
not on `ast_impact`. In the first slice, the chain is serial for simplicity.

## Seed Artifacts

```json
{
  "task_id": "task_impact_assessment",
  "init_artifacts": [
    {
      "slot_id": "node_ref",
      "artifact_type_id": "node_ref",
      "content": { "node_id": "9f6d8d7f..." }
    },
    {
      "slot_id": "head_frame_ref",
      "artifact_type_id": "frame_ref",
      "content": {
        "frame_id": "...",
        "node_id": "9f6d8d7f...",
        "frame_type": "readme"
      }
    }
  ]
}
```

`head_frame_ref` is the `frame_ref` artifact for the node's current head README frame.
It provides the reference point for `git_diff_summary`. If no head frame exists, this slot
is absent and `git_diff_summary` applies the no-prior-record signal.

## Force Posture Integration

When `force_posture.force == true`, control skips ImpactAssessment entirely and dispatches
the docs_writer task directly. This is a control-level decision made before the
ImpactAssessmentTask is created. No ImpactAssessmentTask is dispatched, no
`await_observation` node is entered.

`force_posture` is evaluated by control at the point where the goal is triggered, before
the plan is materialized.

## Prior Computation from Frame History

The `bayesian_evaluation` capability reads a prior probability for each node. That prior
is maintained as a sled projection derived from the frame chain for `(node_id, "readme")`.

The prior projection computes:

- `recency`: age of the head frame timestamp
- `churn_rate`: generation frequency over the last N frames
- `content_stability`: proportion of consecutive frames with identical content

These are combined into a stored prior that the capability reads via `prior_store_ref`.

On first use for any node, no stored prior exists and the static binding default (0.3) is used.
As the system accumulates history, the prior becomes node-specific and calibrated.

The prior reducer subscribes to `frame_written` events. It does not require
ImpactAssessment decisions to exist before it starts running — it learns from frame history
alone. Full calibration (correlating predictions with outcomes) becomes possible once
ImpactAssessmentTask decisions are being made and recorded in the event spine.

## Per-Node Fan-Out

In the docs_writer flow, one ImpactAssessmentTask is dispatched per node in the traversal.
This fan-out follows the same `TaskExpansion` mechanism used for the docs_writer generation
regions. Each task is scoped to one node and produces one `DecisionArtifact`.

The control program for the docs_writer flow looks like:

```
[traverse workspace]
    |
for each node:
    [dispatch ImpactAssessmentTask(node)]
    [await_observation: decision_artifact for node]
    [branch: should_execute]
        true  -> [dispatch DocsWriterRegion(node)]
        false -> [skip node]
```

Nodes whose assessments complete early do not block nodes whose assessments are still running.
The `await_observation` for each node is independent.

## Two-Stage Gate Option

For nodes with large file sets, a two-stage gate reduces cost:

**Stage 1 (cheap)**: run only `git_diff_summary`. If `commit_count_since_reference == 0` and
`days_since_last_doc_update < 3`, emit a fast-skip `DecisionArtifact` without running
`ast_change_impact` or `bayesian_evaluation`.

**Stage 2 (full)**: all three capabilities for all other nodes.

This is an optional optimization. The single-stage path is correct and sufficient for the
first slice.

## Generality of the Pattern

ImpactAssessment for docs_writer is the first instance of a general pattern.
Other observation tasks could include:

- `QualityAssessmentTask`: does this function need a docstring?
- `ChangeRiskAssessmentTask`: does this changeset warrant a review flag?
- `KnowledgeGraphUpdateTask`: does a new thesis node need to be created?

Each follows the same pattern: evidence capabilities → bayesian_evaluation → DecisionArtifact
→ `await_observation` → `branch`.

The capability contracts (`git_diff_summary`, `ast_change_impact`, `bayesian_evaluation`) are
reusable across these observation tasks. New observation tasks add new evidence capabilities
and new prior types, not new infrastructure.

## Read With

- [Await Observation Semantics](program/await_observation_semantics.md)
- [Guard Binding Semantics](program/guard_binding_semantics.md)
- [Git Diff Summary](../capabilities/capability/git_diff_summary/README.md)
- [AST Change Impact](../capabilities/capability/ast_change_impact/README.md)
- [Bayesian Evaluation](../capabilities/capability/bayesian_evaluation/README.md)
- [Task Expansion Plan](../capabilities/task/task_expansion_plan.md)
- [Control Graph Model](program/control_graph.md)
