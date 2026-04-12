# Execution Substrate

Date: 2026-04-12
Status: active
Scope: natural runtime model for planning, dispatch, and world side effects in `execution`

## Thesis

To each domain be true.

The natural substrate for `execution` is deliberate action over current belief.
In this repo that substrate is already largely present through `goals`, `control`, `task`, `capability`, `provider`, and domain-owned capability homes.

`execution` acts.
It does not own world belief.

## Natural Runtime Shape

The substrate should look like:

- goal interpretation
- planning over current belief
- dispatch into task and capability execution
- execution monitoring
- repair and replanning
- outcome publication back into the spine

This is closer to orchestration and action selection than to streaming or belief reduction.

## Core Primitives

- goal
  desired change in world state
- plan
  chosen path from current belief toward desired state
- task
  compiled unit of deliberate work
- capability
  atomic executable contract inside task execution
- control state
  orchestration, repair, waits, and continuation
- outcome record
  structured result that curation can later integrate

## Materialization Posture

`execution` should read materialized current belief from `world_state`.
It should not plan over raw event streams when a shaped world-model view exists.

It should also publish material suitable for later materialization by `world_state`:

- success
- failure
- uncertainty discovered during action
- evidence gathered during action

## What The Substrate Must Support

- planning against current belief
- information-gathering actions when belief is stale or missing
- dispatch into existing task and capability mechanics
- observation waits and repair
- replanning after outcome or belief change
- publication of rich enough outcomes for world-state revision

## What Should Not Be Forced Into This Substrate

- high-rate sensory lowering
- belief revision internals
- raw continuity tracking
- canonical cross-domain ordering policy

Those belong elsewhere.

## Relationship To Task And Capability

`task` and `capability` are not the whole cognitive loop.
They are the execution substrate inside the loop.

That means:

- `task` and `capability` remain the natural packaging for deliberate action
- they should not be forced to become the substrate for `sensory` or `world_state`
- planner to world-model coupling is the key missing bridge, not a replacement of task mechanics

## Relationship To Control

`control` is the coordination subsystem inside `execution`.
It is not a peer cognitive domain.

`control` owns:

- task-network coordination
- event reduction into execution state
- observation waits
- continuation
- repair
- HTN lineage above compiled tasks

`execution` remains the larger cognitive concern that includes planning against world state and deliberate side effects.

## Relationship To Curation

`execution` must not treat the knowledge graph as a sidecar.
Without world-model reads, planning reduces to operator selection over local mechanics.

The substrate therefore assumes:

- goals are expressed against desired world change
- planning reads curated belief
- execution publishes outcomes back for `world_state` to revise belief

## First Slice

- planner input from current belief rather than workspace state alone
- goal forms that describe desired world-model change
- outcome records that include evidence and failure shape, not only task completion
- replanning triggers when beliefs change or go stale

## Read With

- [Execution Domain](README.md)
- [Execution Control](control/README.md)
- [Execution Planning](control/planning/README.md)
- [World State Domain](../world_state/README.md)
- [Observe Merge Push](../observe_merge_push.md)
- [Task Network](control/task_network.md)
