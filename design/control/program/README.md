# Control Program Model

Date: 2026-03-28
Status: active

## Intent

Define the program shaped control layer that runs above primitive capability regions.

## Definition

A control program is a durable control graph or state machine that can sequence, branch, loop, await observations, invoke primitive regions, and terminate.

Primitive regions may remain DAGs even when the control program revisits them.

## Core Records

The program area should define these durable records:

- `compiled_control_program`
- `control_node`
- `control_edge`
- `region_invocation`
- `guard_binding`

## Rules

- control flow and dependency flow are separate concerns
- control nodes are explicit and typed
- loop and revisit behavior live in control, not in primitive region wiring
- entry, exit, and suspension points are durable program facts
- every loop capable path must declare checkpoint posture

## First Slice

The first slice control program should be sufficient to express straight line flow, conditional branch, bounded revisit, and primitive region invocation.

## Next Doc

- [Control Graph Model](control_graph.md)
