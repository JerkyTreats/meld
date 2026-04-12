# HTN Lineage Model

Date: 2026-03-28
Status: active
Scope: durable hierarchy records that preserve task and method intent above primitive capability regions

## Intent

Define the concrete lineage model for HTN aware control.

## Summary

The compiled control artifact must preserve task and method lineage as first class data.
Repair, audit, and explanation should operate on task intent rather than only on flat primitive nodes.

## Required Records

- `htn_task_instance`
  - one occurrence of an abstract or primitive task in the hierarchy
- `htn_method_instance`
  - one chosen method for one task instance
- `task_parent_link`
  - parent child relationship among task instances
- `method_child_link`
  - method to introduced child task mapping
- `task_region_link`
  - mapping from task lineage to one or more primitive plan regions or control nodes

## Identity Rules

- every task instance has a stable `task_instance_id`
- every method instance has a stable `method_instance_id`
- lineage identity is deterministic from control digest plus task or method role plus parent lineage
- primitive region links must remain stable across resume and audit

## Mapping Rules

- one abstract task may decompose into multiple child tasks
- one chosen method owns the child task set for one task instance
- one task instance may map to multiple primitive regions when execution spans several invocations
- every invoked primitive region must map back to at least one task instance

## Runtime Use

Runtime should surface:

- active task instance
- active method instance
- current child task progress
- executed prefix under the current task subtree

## Repair Use

Repair should be able to answer:

- which task boundary absorbed the failure
- whether the current method can continue
- whether a parent task must reselect its method
- which completed regions remain reusable

## First Slice

The first slice should support one parent task, one chosen method, multiple child tasks, and links into one or more primitive regions.
