# Control Program Model

Date: 2026-04-08
Status: active

## Intent

Define the program shaped control layer that the task network executes over compiled tasks.

## Definition

A control program is a durable compiled graph that sequences, branches, loops, awaits
observation task output, dispatches tasks, and terminates.

Tasks are the objects being orchestrated. Control nodes are the instructions.
Compiled tasks remain responsible for capability dependency structure.
The control graph owns control transfer only.

## Core Records

The program area defines these durable records:

- `CompiledControlProgram`
- `ControlNode`
- `ControlEdge`
- `GuardBinding`      (see guard_binding_semantics.md)

`region_invocation` has been renamed `dispatch_task` to align with settled task vocabulary.

## Rules

- control flow and dependency flow are separate concerns
- control nodes divide into task-operating nodes and control flow nodes
- loop and revisit behavior live in control, not in compiled task wiring
- entry, suspension, and termination points are durable program facts
- every loop capable path must declare checkpoint posture

## First Slice

The first slice control program should be sufficient to express straight-line task dispatch,
conditional branch via guard artifact, parallel dispatch with join, bounded revisit, and
observation wait.

## Documents

- [Control Graph Model](control_graph.md)
- [Await Observation Semantics](await_observation_semantics.md)
- [Guard Binding Semantics](guard_binding_semantics.md)
