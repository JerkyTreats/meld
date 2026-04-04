# Continuation Model

Date: 2026-03-28
Status: active
Scope: durable runtime state for resume, audit, and observation driven control

## Intent

Define the minimum continuation state required for reliable resume.

## Summary

A control runtime is durable only when current position and decision state are explicit.
Resume should not depend on hidden in memory execution context.

## Required Records

- `runtime_continuation`
- `control_position`
- `task_position`
- `execution_environment`
- `observation_snapshot`
- `checkpoint_record`

## Continuation Contents

The continuation should carry:

- current control node identity
- current control edge or pending transfer when relevant
- active task instance identity
- active method instance identity when one exists
- environment bindings needed for the next step
- latest observation snapshot used for branch selection
- checkpoint lineage and resume validation digest

## Checkpoint Rules

- every suspension or revisit capable path must have checkpoint posture
- checkpoints must be sufficient for process restart resume
- checkpoint validation must confirm control digest, primitive region digest, and lineage linkage before resume

## Observation Rules

- observation driven branch choices should use stored observation state
- runtime should persist the observation that justified the current control branch
- repair should be able to compare current observation state with prior branch assumptions

## First Slice

The first slice continuation should support restart safe resume at a control node boundary with task lineage intact.
