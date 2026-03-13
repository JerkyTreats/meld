# Workflow Definition

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Define workflow as the composition layer above atomic tasks and as the future home of HTN aligned decomposition.
Workflow should no longer mean only turn sequencing for one concrete docs flow.

## HTN Position

- a workflow run is one compiled task network for one top level task over one declared target scope and one input snapshot
- workflow owns compound task decomposition, method selection, dependency shaping, and task network compilation
- primitive tasks are atomic tasks and do not own cross task orchestration
- workflow runtime owns checkpoints, repair records, and durable execution trace across atomic task execution

## Provisional Answers

### Smallest Stable Definition

- the smallest stable definition of a workflow is one top level task compiled into a validated task network
- that task network must declare stable task ids, dependency edges, artifact handoffs, and scope identity before execution begins
- a workflow definition may later support multiple methods, but the runtime should execute one compiled network at a time

### Planning Boundary

- workflow planning owns top level intent interpretation, method choice, decomposition, target expansion, dependency edges, artifact wiring, and compile time validation
- atomic task execution owns only task behavior, local precondition checks, runtime observations, and declared outputs
- workflow may reject a plan before execution starts if task contracts do not compose cleanly

### Durable Runtime Units

- `workflow_run` records one attempt to execute one compiled task network
- `plan` records the compiled network digest, method choice, task graph, and input snapshot
- `task_instance` records one concrete compound or primitive task occurrence within the run
- `target_batch` records a deterministic subset of targets when execution is batched
- `artifact_handoff` records output slot to input slot bindings across task instances
- `checkpoint` records safe resume boundaries after compile or execution milestones
- `repair_record` records divergence, preserved work, and chosen recovery path
- `execution_trace` records ordered planning and runtime events across the full run

### Resume And Repair

- resume should default to preserving completed atomic task work when its inputs, scope digest, and side effect expectations remain valid
- resume should restart from the nearest valid checkpoint rather than always from the beginning
- partial success should produce a repair decision instead of an implicit full restart
- repair should be explicit in durable state so migration from current turn behavior remains explainable

## Initial Requirements

- workflow config compiles into a validated task network with stable task ids
- task instances declare dependencies and artifact handoff explicitly
- workflow runtime owns task status, retry policy, checkpoints, and repair records
- workflow should orchestrate both target local task instances and batch scoped task instances
- workflow should support structures broader than one prompt thread

## Worked Mapping

### Docs Writer Today

- the current docs writer flow in `src/workflow/builtin.rs` can be read as one top level task that decomposes into evidence gathering, verification, structure generation, and style refinement
- in the current foundation phase, these stages may still be represented through compatibility workflow shapes
- in the later HTN aligned shape, this flow becomes a clear example of one compound task refined into a stable sequence of sub tasks

### Task Layer

- ordering, context generation, validation, and file write should be modeled as atomic task families once their contracts are stable
- workflow should invoke those tasks through explicit contracts rather than by embedding their orchestration rules

## Current Code Pressure

- `src/workflow` is currently centered on turn execution and gate checks
- `src/context` still owns major orchestration concerns such as ordering and recursive planning
- the current profile surface looks broader than the runtime semantics it truly enforces

## Residual Questions

- how rich should first phase method selection be before it adds more cost than value
- where should decomposition records live relative to existing workflow state storage
- which repair cases should be automated in the first phase and which should halt for operator choice
- how much of the method library should be declarative in the first phase versus compatibility code backed

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Task Model](../task_model/README.md)
- [Primitive Task Contract](../primitive_task_contract/README.md)
- [Migration Plan](../migration_plan/README.md)
- [Completed Workflow Bootstrap](../../completed/workflow_bootstrap/README.md)
