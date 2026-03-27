# Petgraph Choice

Date: 2026-03-27
Status: active

## Decision

Use `petgraph` as the graph substrate for plan graph infrastructure.

## Why

- mature and widely used Rust graph crate
- supports directed graphs and common validation algorithms
- sufficient for DAG validation and traversal needs
- flexible enough to support compiler-owned plan semantics without leaking external abstractions into domain code

## Usage Rule

`petgraph` is infrastructure, not the plan model.

Project code must wrap `petgraph` with plan-owned types for:

- capability instance ids
- dependency edge ids
- artifact handoff ids
- graph validation rules
- compiler projections

## Research Outcome

No additional DAG crate research is needed for the first slice.
The chosen path is to build a thin plan graph layer on top of `petgraph`.

## Non Goals

- exposing raw `petgraph` types as domain contracts
- encoding compiler semantics directly into third-party graph types
