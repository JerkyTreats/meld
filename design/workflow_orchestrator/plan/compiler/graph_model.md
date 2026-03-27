# Plan Graph Model

Date: 2026-03-27
Status: active

## Intent

Define the plan graph model used by compiler.

## Node Model

One node is one bound capability instance.

Each node carries:

- `capability_instance_id`
- `capability_type_id`
- `capability_version`
- `scope_ref`
- `scope_kind`
- `input_slot_refs`
- `output_slot_refs`
- `binding_digest`

## Edge Model

Dependency edges express execution preconditions.

Each dependency edge carries:

- `dependency_edge_id`
- `from_capability_instance_id`
- `to_capability_instance_id`
- `edge_kind`
- `satisfaction_rule`

## Artifact Handoff Model

Artifact handoffs express producer-consumer data flow.

Each artifact handoff carries:

- `artifact_handoff_id`
- `producer_capability_instance_id`
- `producer_output_slot`
- `consumer_capability_instance_id`
- `consumer_input_slot`
- `artifact_type_id`
- `artifact_schema_version`

## Identity Rules

- plan digest is deterministic from normalized graph content
- capability instance ids are deterministic from plan digest plus stable capability role and scope identity
- dependency edge ids are deterministic from plan digest plus endpoint ids and edge kind
- artifact handoff ids are deterministic from plan digest plus producer consumer slot wiring and artifact type

## Parallel Rule

The graph model is parallel-ready by default.
Compiler must not introduce unnecessary serial edges.
