# Repair Goal

Date: 2026-03-28
Status: stub
Scope: minimal repair goal note above control

`repair` is the first named goal type above control.

At this layer, repair means:

- valid work should continue if possible
- blocked or failed work should trigger plan change
- the system should preserve goal progress while modifying the active plan

At the control layer, the concrete function is:

- `plan::modify`

This split is important:

- `repair` as goal
  why plan change is needed
- `plan::modify` as control function
  how that plan change is applied

Further refinement is necessary before a full goal model exists.
