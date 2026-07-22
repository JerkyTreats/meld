# HTN Model

Date: 2026-03-28
Status: active

## Intent

Define Execution-owned realization lineage for hierarchical structure already selected by an Agent-authorized Strategy decision.

## Definition

The HTN model owns operational identity and lineage from an exact Strategy candidate, its complete Method-derivation inventory, and resolved child structure down to primitive capability regions.

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
- Strategy-selected Method revision, bindings, and expanded hash must be inspectable and durable
- hierarchy is not flattened away when repair depends on task boundaries
- one task may map to one or more primitive regions
- every Method instance must reflect the exact child task set already resolved in the authorized candidate

## First Slice

The first slice HTN model should be sufficient to map one authorized candidate and its Method derivation set into one or more primitive capability regions.

## Next Doc

- [HTN Lineage Model](lineage_model.md)
