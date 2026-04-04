# Runtime Model

Date: 2026-03-28
Status: active

## Intent

Define the runtime state required to execute control programs durably.

## Definition

The runtime model owns continuation state, control position, task position, observation state, checkpoint state, and the binding environment needed for resume and audit.

## Core Records

The runtime area should define these durable records:

- `runtime_continuation`
- `control_position`
- `task_position`
- `execution_environment`
- `observation_snapshot`
- `checkpoint_record`

## Rules

- resume state must be explicit
- control position must be durable
- observation driven decisions must be recoverable from stored state
- runtime must not rely on hidden in memory control state for correctness
- continuation must be sufficient to resume after process restart

## First Slice

The first slice runtime model should be sufficient to resume after a process restart without losing current control location or current HTN lineage.

## Next Doc

- [Continuation Model](continuation_model.md)
