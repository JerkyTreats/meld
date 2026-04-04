# Control Design Plan

Date: 2026-03-28
Status: proposed
Scope: decompose the control layer into concrete design areas above capability contracts and primitive plan

## Objective

Establish `design/control` as the durable design home for HTN decomposition, control graph semantics, continuation state, and repair.

This design work does not implement a new runtime.
It defines a decision complete structure so later implementation can proceed without forcing Turing complete behavior into the primitive plan DAG.

## Summary

The control layer should be built as a second compilation and execution layer:

- planner or decomposer produces HTN shaped intent and choices
- primitive compiler still validates acyclic capability regions
- control compilation produces a durable control program above those regions
- runtime executes the control program with explicit continuation and repair state

## First Slice Outcomes

- `design/control` is the design home for program shaped execution
- HTN lineage remains visible after compilation
- control flow is modeled separately from capability dependency flow
- runtime resume is modeled through explicit continuation records
- repair is modeled through task boundary re entry and method reselection

## Scope Guardrails

In scope:

- HTN lineage records
- control node families
- control edge semantics
- continuation state
- checkpoint boundaries
- repair entry semantics
- cross references into `design/capabilities`

Out of scope:

- executor implementation
- runtime storage backend
- planner heuristics
- HDDL authoring format
- capability catalog redesign

## Work Breakdown

### Phase 0

Freeze vocabulary and boundaries.

- define `compiled_primitive_plan`
- define `compiled_control_program`
- define HTN lineage records
- define continuation and repair as durable control concerns

### Phase 1

Define HTN lineage.

- task instance identity
- method instance identity
- decomposition boundary rules
- region links from task lineage to primitive plan regions

### Phase 2

Define the control graph.

- control node families
- control edges and guard semantics
- region invocation semantics
- loop policy and checkpoint requirements

### Phase 3

Define runtime continuation.

- control position
- task position
- environment bindings
- observation state
- checkpoint state
- resume validation

### Phase 4

Define repair.

- primitive retry within task boundary
- method reselection at parent task boundary
- controlled recompilation boundaries
- reuse rules for unaffected completed regions

## Acceptance Shape

The control design is ready for implementation when:

- a reader can distinguish `control` from primitive `plan`
- each control subarea has named durable records
- loop and revisit behavior have a home outside primitive DAG wiring
- resume semantics no longer depend on hidden in memory state
- repair semantics no longer degrade into generic retry alone

## Proposed Next Docs

1. `htn/lineage_model.md`
2. `program/control_graph.md`
3. `runtime/continuation_model.md`
4. `repair/repair_entry.md`
