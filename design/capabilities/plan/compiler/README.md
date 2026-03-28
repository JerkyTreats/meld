# Plan Compiler

Date: 2026-03-27
Status: active

## Intent

Define compiler as the first plan concern.

## Definition

Compiler takes a candidate capability graph plus compile-time bindings and emits a locked plan record.
It performs capability instance binding, dependency edge validation, artifact handoff validation, scope validation, binding validation, graph validation, plan digest generation, and plan record emission.
It does not choose goals, choose capabilities, search for decompositions, or mutate plan structure at execution time.

## Inputs

Compiler consumes candidate capability graph input, scope input, binding input, capability catalog records, and artifact schema records.

## Outputs

Compiler emits a compiled plan record, compile diagnostics, stable capability instance ids, stable dependency edge ids, stable artifact handoff ids, and a plan digest.

## Validation Rules

All required inputs must be satisfied.
All artifact producers and consumers must be type-compatible.
All edge endpoints and required handoffs must resolve.
Duplicate semantic edges and cycles are rejected.
Disconnected graph segments are rejected unless the submitted graph definition explicitly allows them.

## First Slice

The first slice compiler must support compatibility-lowered docs writer paths, compatibility-lowered `context generate`, and parallel-ready graphs even when early plans are small.
