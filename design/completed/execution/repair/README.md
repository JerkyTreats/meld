# Repair Model

Date: 2026-03-28
Status: active

## Intent

Define how control re entry and plan repair work with preserved task lineage.

## Definition

Repair is the control concern that decides where execution can safely restart, which task boundary should absorb failure, and whether the current method should continue, be retried, or be replaced.

## Core Records

The repair area should define these durable records:

- `repair_entry`
- `repair_decision`
- `task_boundary_reset`
- `method_reselection`
- `region_reuse_record`

## Rules

- repair operates on explicit task and control boundaries
- repair must distinguish primitive retry from method reselection
- completed unaffected regions should remain reusable when safe
- repair decisions must be auditable
- control may trigger controlled recompilation at task boundaries

## First Slice

The first slice repair model should be sufficient to express retry within a task boundary and method reselection at a parent task boundary.

## Next Doc

- [Repair Entry Model](repair_entry.md)
