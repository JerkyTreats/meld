# Control Graph Model

Date: 2026-03-28
Status: active
Scope: durable control graph structure above primitive capability DAG regions

## Intent

Define the concrete program model for branching, revisiting, and region invocation.

## Summary

The control graph is the executable program shaped artifact above primitive plans.
It owns control transfer and decision structure.
Primitive regions remain responsible for capability dependency validation.

## Required Records

- `compiled_control_program`
- `control_node`
- `control_edge`
- `region_invocation`
- `guard_binding`

## Control Node Families

The first control node family should include:

- `entry`
- `invoke_region`
- `branch`
- `join`
- `loop_header`
- `await_observation`
- `repair_entry`
- `terminate`

## Edge Rules

- control edges express control transfer only
- dependency edges remain inside primitive regions
- branch edges may carry guard bindings
- loop back edges are valid only in the control graph
- every reachable path must have an eventual termination or suspension posture

## Loop Rules

- loop behavior must live in `loop_header` semantics, not in primitive region rewiring
- every loop capable path must declare checkpoint posture
- side effecting region invocations inside revisit paths must rely on capability replay posture from `design/capabilities`

## Region Invocation Rules

- one `invoke_region` node references one `compiled_primitive_plan`
- invoked regions remain independently valid DAGs
- repeated invocation of one region should be explicit in control, not duplicated by hidden runtime logic

## First Slice

The first slice should support straight line flow, conditional branch, bounded revisit, observation wait, and region invocation.
