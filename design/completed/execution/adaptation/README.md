# Adaptation

Date: 2026-05-06
Status: resolved into planning loop
Scope: exploration of plan-execution reconciliation as a separate domain

## Resolution

This document explored adaptation as a first-class execution domain bridging continuous planning and parallel task execution. The exploration was productive — it surfaced the trigger taxonomy, stability concerns, and mutation operations that any execution architecture must address. However, the "graphs lower graphs" alignment resolved the concern differently.

The arbiter role proposed here folds into the cost-aware planning loop. The planning loop directly issues task network mutations (inject, cancel, relink, preserve, prune) when it determines that the benefit of changing the plan exceeds the switching cost. There is no separate adaptation process between the planning loop and the task network.

See [Planning Pipeline](../planning/planning_pipeline.md) for the authoritative model: two concurrent processes (planning loop + task network), connected by graph mutations.

## What This Exploration Surfaced

The following concerns were identified here and have been absorbed into the planning pipeline:

### Trigger taxonomy

The full set of conditions that cause plan re-evaluation:

| Trigger | Source | Resolution in pipeline |
|---|---|---|
| task failure, retries exhausted | task network | planning loop receives failure event, re-evaluates via HTN lineage, may reselect method |
| task success | task network | planning loop advances; outcome published for belief revision |
| observation artifact produced | task network | conditional dependency edges evaluated; branches resolved, unchosen subtrees pruned |
| belief invalidated | world model | planning loop re-evaluates affected HTN subtree via lineage scoping |
| regime shift | world model | planning loop re-evaluates broadly; cost threshold may be overridden |
| goal satisfied externally | world model | planning loop cancels remaining tasks via task network mutations |
| goal priority change | goals | planning loop re-decomposes with new priorities |
| synthesis completion | task network | catalog updated; planning loop re-evaluates suspended subtrees |

All triggers flow through the planning loop. The planning loop reads the world model, receives task network events, and decides whether to issue mutations. No separate arbitration process is needed.

### Stability mechanisms

Three mechanisms to prevent thrashing, now owned by the planning loop's cost-aware transition logic:

- **Significance threshold**: the planning loop only issues mutations when the benefit of the new plan exceeds the switching cost. Minor belief shifts that produce marginal improvements are deferred. This replaces the proposed "significance threshold" on adaptation operations.

- **Commitment window**: the cost model accounts for in-progress work. A running task has sunk cost and disruption cost if cancelled. The cost model naturally creates commitment windows — tasks won't be cancelled unless the benefit of the new plan is large enough to justify the loss.

- **Batch coalescing**: the planning loop operates continuously but issues mutations as discrete deltas. Multiple rapid belief changes are absorbed into the planning loop's next evaluation cycle. The planning loop processes current state, not every intermediate change.

### Mutation operations

The adaptation operations map directly to task network mutations:

| Adaptation operation | Task network mutation | Notes |
|---|---|---|
| inject | inject | unchanged |
| cancel | cancel | cleanup tasks are themselves injected as normal tasks |
| reorder | relink | dependency edges modified, not task priority |
| swap | cancel + inject | old subtree cancelled, new subtree injected |
| preserve | preserve | completed work's artifacts relinked into new dependency structure |
| suspend | (graph structure) | suspended tasks have unsatisfied dependencies; no special operation needed |
| resume | (graph structure) | dependencies become satisfied; task enters ready set automatically |

Suspend and resume dissolved entirely. In the graph model, a task is "suspended" when its dependencies are not yet satisfied (e.g., waiting for an observation task to complete). It "resumes" when the dependency is satisfied and it enters the ready set. This is the normal execution model — no special operation needed.

## Why Adaptation Dissolved

Three factors:

**1. The graph IS the reconciliation structure.** The adaptation document proposed maintaining a mapping between "the plan" and "the execution state" and computing deltas between them. In the graphs-lower-graphs model, the task network graph IS both the plan and the execution state. There is no separate plan artifact to reconcile against. The planning loop mutates the same graph that the task network executes.

**2. Cost-aware transitions are a planning concern.** The adaptation document proposed that adaptation would decide whether to act on plan updates based on cost. But cost evaluation requires the same information the planning loop already has: the current HTN decomposition, the cost estimates of tasks, the sunk cost of in-progress work. The planning loop is the natural owner of this decision because it has the full context.

**3. Graph structure replaces dispatch control.** The adaptation document inherited the control program's concern with dispatch ordering — what to dispatch next, what to hold, what to cancel. In the graph model, dispatch order is an emergent property of dependency satisfaction. The task network computes the ready set (nodes whose dependencies are satisfied) and dispatches them. There is no need for a separate process to determine dispatch order.

## What Remains Open

### Agent relationship

This document asked whether adaptation is the execution-side counterpart of the world model's agent concept. The question remains relevant but is reframed: is the planning loop the agent's operational identity? In a multi-agent system, each agent would have its own planning loop maintaining its own task network. This aligns with the world model's agent-as-perspective design but needs further exploration.

### Control program internal representation (Option A vs B)

This document asked whether the planning loop needs an internal control-program-like structure. In the graphs-lower-graphs model, this resolves to Option B: the planning loop operates directly on the task network graph and HTN lineage. No intermediate compiled representation is needed. The graph IS the representation.

## Read With

- [Planning Pipeline](../planning/planning_pipeline.md) — authoritative treatment of the two-process model
- [Execution Domain](../README.md)
- [Execution Gaps](../GAPS.md)
- [HTN Model](../planning/htn/README.md)
- [HTN Lineage Model](../planning/htn/lineage_model.md)
- [Task Network](../task_network.md)
- [Repair Model](../repair/README.md)
- [World Model Agent](../../world_model/agent/README.md)
