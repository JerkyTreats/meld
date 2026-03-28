# Capability Model

Date: 2026-03-27
Status: active

## Intent

Define capability as the durable orchestration contract.

## Definition

A capability is a domain-owned contract.
It declares required inputs, produced outputs, artifact schemas, scope rules, side-effect class, idempotency posture, binding requirements, and validation rules.

Capability is not implementation.
Implementation remains in the owning domain.

## Capability Instance

A plan node is one bound capability instance.

Each capability instance has one capability type id, one capability version, one scope binding, one input set, one output set, and one binding set.

## First Slice Capability Families

- `context_generate`
- `merkle_traversal`
- `compatibility_turn`

## Contract Rules

- capability contracts must be explicit and typed
- capability contracts must be versioned
- capability contracts must be sufficient for compile-time validation
- capability contracts must not rely on hidden runtime convention
- capability contracts must be owned by the domain that can keep them true

## Implementation Boundary

`context/` owns `context_generate`.
Future file materialization code owns `write_file`.
Future workspace refresh code owns `refresh_workspace`.

Workflow is not the owning domain.
Plan is not the owning domain.
Those layers consume capability contracts.
