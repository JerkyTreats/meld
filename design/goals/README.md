# Goals Design

Date: 2026-03-28
Status: stub
Scope: minimal goal layer above control

`goals` sits above `control`.

`goals` answers why the system should act.
`control` answers how tasks are arranged and executed.
`task_network` may consume goal shaped intent, but it does not define the full goal model.

## First stub

The first stub goal type is:

- `repair`

This means the system needs a goal home for:

- modify plan because progress is blocked
- modify plan because work failed
- modify plan because assumptions changed

Further refinement is necessary.
The goal language is not yet settled.

## Boundary

- `goals`
  why change is needed
- `control`
  how tasks are dispatched, repaired, and reduced into state
- `plan`
  the tasks being modified

## Read With

1. [Task Network](../control/task_network.md)
2. [Control Design](../control/README.md)
