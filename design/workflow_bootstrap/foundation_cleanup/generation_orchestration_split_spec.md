# Generation Orchestration Split Spec

Date: 2026-03-01

## Intent

Split generation request processing into focused units so metadata contract changes and prompt artifact changes can land with smaller blast radius.

## Problem

Current generation request processing handles prompt assembly, context collection, provider execution, metadata creation, and frame write in one large path.
Any contract change requires edits across mixed concerns.

## Target Orchestration Model

Ownership:
- `src/context/queue` owns queue lifecycle and retry control
- `src/context/generation` owns generation orchestration units
- `src/metadata` owns frame metadata construction and validation
- `src/prompt_context` owns prompt and context artifact writes

Execution units:
- input resolver
- prompt and context collector
- provider executor
- frame metadata builder
- frame writer

## Required Changes

1. extract generation sub steps into dedicated units under context generation domain

2. keep queue worker focused on dequeue, retry policy, and telemetry

3. call metadata builder through explicit contract

4. call frame writer through shared validation boundary

5. preserve existing behavior with characterization tests before cleanup edits

## Done Criteria

- queue worker no longer constructs frame metadata directly
- queue worker no longer performs prompt and context concatenation inline
- each orchestration unit has focused tests and clear input output contracts

## Verification

- characterization tests for existing queue outcomes before refactor
- parity tests for generated frame content and retry behavior after split
- targeted tests for metadata builder integration and frame write integration
