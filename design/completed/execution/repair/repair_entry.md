# Repair Entry Model

Date: 2026-03-28
Status: active
Scope: task aware re entry and method reselection semantics for control runtime

## Intent

Define where repair begins and what decisions it may take.

## Summary

Repair is not generic retry alone.
It must account for the active task boundary, the chosen method, the current control node, and the reusable completed regions under the same lineage.

## Required Records

- `repair_entry`
- `repair_decision`
- `task_boundary_reset`
- `method_reselection`
- `region_reuse_record`

## Repair Entry Rules

- every repair entry identifies the failing control node
- every repair entry identifies the active task instance
- repair begins at the nearest safe task boundary
- primitive retry and method reselection are distinct repair outcomes

## Repair Decisions

The first decision family should include:

- retry current primitive region within the same task boundary
- reset current task boundary and replay its method path
- reselect a method at the parent task boundary
- trigger controlled recompilation for one task subtree
- terminate with failure when no safe repair path exists

## Region Reuse Rules

- completed unaffected regions may be reused when lineage and input assumptions remain valid
- reuse must be explicit and auditable
- repair must not silently invalidate prior successful regions

## First Slice

The first slice repair model should support primitive retry within task scope and parent level method reselection with explicit reuse records.
