# Repair Goal

Date: 2026-06-02
Status: active
Scope: repair as a goal lifecycle pattern rather than a separate goal type

## Pattern

Repair is not a separate Goal type. It is bounded convergence after a task outcome threatens an active Goal.

When retries are exhausted, Execution records and publishes the failure. The Agent judges whether intent remains active. Strategy may construct a replacement proposal from the revised world state. Execution may select another still-authorized alternative or realize a newly authorized decision through task-network mutations.

The repair pattern splits three concerns:

- **why** plan change is needed → goal lifecycle, where the active goal is threatened by task failure
- **which semantic replacement is viable** → Strategy proposal and Agent authorization
- **how** an authorized change is applied → task-network commands carrying inject, cancel, relink, preserve, and prune mutation sets

Repair is not a Goal. It is a trigger for Goal reevaluation and possibly renewed Strategy. The Goal may remain the same desired belief state while a new authorized path replaces blocked work.

See [Goals](README.md) for the full goal model where this pattern is described as the planning loop's response to task failure under an active goal.

## Read With

- [Goals](README.md)
- [Planning Pipeline](../planning/planning_pipeline.md)
- [Task Network](../task_network.md)
