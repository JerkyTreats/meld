# Repair Goal

Date: 2026-05-08
Status: resolved into goal model
Scope: repair as a goal lifecycle pattern rather than a separate goal type

## Resolution

Repair is not a separate goal type. It is the planning loop's response to a task failure event that threatens an active goal.

When a task fails and retries are exhausted, the planning loop receives the failure event. It uses HTN lineage to identify which goal is threatened, re-evaluates the affected subtree, may reselect methods, and issues task network mutations (cancel failed subtree, inject replacement). This is the cost-aware plan transition mechanism described in [Planning Pipeline](../planning/planning_pipeline.md).

The original repair framing identified the right split:

- **why** plan change is needed → goal lifecycle (the active goal is threatened by task failure)
- **how** plan change is applied → task network mutations (inject, cancel, relink, preserve, prune)

What changes is that "repair" is not a goal — it is a trigger for goal re-evaluation. The goal remains the same (the desired belief state). The plan changes because the current path to that state has been blocked.

See [Goals](README.md) for the full goal model where this pattern is described as the planning loop's response to task failure under an active goal.

## Read With

- [Goals](README.md)
- [Planning Pipeline](../planning/planning_pipeline.md)
- [Task Network](../task_network.md)
