# Plan Record

Date: 2026-03-27
Status: active

## Intent

Define the durable record emitted by compiler.

## Required Records

- `compiled_plan`
- `capability_instance`
- `dependency_edge`
- `artifact_handoff`
- `compile_diagnostic`

## Record Rules

- records preserve the locked plan without loss of meaning
- records are sufficient for later execution
- records are sufficient for operator inspection
- records are sufficient for deterministic resume validation

## First Slice Identity Fields

- `compiled_plan_id`
- `plan_digest`
- `capability_instance_id`
- `dependency_edge_id`
- `artifact_handoff_id`
- `scope_digest`
- `binding_digest`
