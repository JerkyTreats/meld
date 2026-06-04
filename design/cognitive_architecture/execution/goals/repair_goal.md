# Repair Goal

Date: 2026-06-02
Status: resolved into goal model
Scope: repair as a goal lifecycle pattern rather than a separate goal type

## Resolution

Repair is not a separate goal type. It is the planning loop's response to a task failure event that threatens an active goal.

When a task fails and retries are exhausted, the planning loop receives the failure event. It uses HTN lineage to identify which goal is threatened, re-evaluates the affected subtree, may reselect methods, and issues a task network command carrying cancel and inject mutations. This is the cost-aware plan transition mechanism described in [Planning Pipeline](../planning/planning_pipeline.md).

The original repair framing identified the right split:

- **why** plan change is needed → goal lifecycle, where the active goal is threatened by task failure
- **how** plan change is applied → task network commands carrying inject, cancel, relink, preserve, and prune mutation sets

What changes is that "repair" is not a goal — it is a trigger for goal re-evaluation. The goal remains the same desired belief state. The plan changes because the current path to that state has been blocked.

See [Goals](README.md) for the full goal model where this pattern is described as the planning loop's response to task failure under an active goal.

## Read With

- [Goals](README.md)
- [Planning Pipeline](../planning/planning_pipeline.md)
- [Task Network](../task_network.md)
