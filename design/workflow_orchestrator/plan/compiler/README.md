# Plan Compiler

Date: 2026-03-27
Status: active

## Intent

Define compiler as the first plan concern.

## Definition

Compiler takes a candidate capability graph plus compile-time bindings and emits a locked plan record.

Compiler does:

- capability instance binding
- dependency edge validation
- artifact handoff validation
- scope validation
- binding validation
- graph validation
- plan digest generation
- plan record emission

Compiler does not:

- choose goals
- choose capabilities
- search for decompositions
- mutate plan structure at execution time

## Inputs

- candidate capability graph
- scope input
- binding input
- capability catalog records
- artifact schema records

## Outputs

- compiled plan record
- compile diagnostics
- stable capability instance ids
- stable dependency edge ids
- stable artifact handoff ids
- plan digest

## Validation Rules

- all required inputs must be satisfied
- all artifact producers and consumers must be type-compatible
- all edge endpoints must resolve
- all required handoffs must resolve
- duplicate semantic edges are rejected
- cycles are rejected
- disconnected graph segments are rejected unless explicitly allowed by the submitted graph definition

## First Slice

The first slice compiler must support:

- compatibility-lowered docs writer paths
- compatibility-lowered `context generate`
- parallel-ready graphs even when early plans are small
