# HTN Model

Date: 2026-03-28
Status: active

## Intent

Define the hierarchical task model owned by `control`.

## Definition

The HTN model owns abstract task identity, method identity, decomposition boundaries, and lineage from abstract task intent down to primitive capability regions.

It preserves why a region exists, not only what primitive nodes it contains.

## Core Records

The HTN area should define these durable records:

- `htn_task_instance`
- `htn_method_instance`
- `task_region_link`
- `task_parent_link`
- `method_child_link`

## Rules

- abstract task and method lineage must survive compilation
- primitive execution records must map back to task lineage
- method choice must be inspectable and durable
- hierarchy is not flattened away when repair depends on task boundaries
- one task may map to one or more primitive regions
- one method choice must declare the child task set it introduces

## First Slice

The first slice HTN model should be sufficient to map one abstract task through one chosen method into one or more primitive capability regions.

## Next Doc

- [HTN Lineage Model](lineage_model.md)
