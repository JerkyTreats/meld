# Context Capability Readiness

Date: 2026-03-27
Status: active

## Intent

Define the refactor work required to make current `context generate` behavior capability-ready and plan-ready.

## Current Problem

Current `context generate` behavior still mixes target expansion, Merkle traversal, generation execution, retry and resume assumptions, workflow-shaped sequencing, and output shaping.
That mixed ownership prevents clean capability contracts and clean plan compilation.

## Required End State

The `context` domain should own only domain behavior behind explicit capability contracts.

For the first slice, `context` must expose a `context_generate` capability contract, typed input artifact contracts, typed output artifact contracts, and the execution-facing implementation behind that contract.

The following concerns must move out of `context`: plan graph assembly, dependency edge construction, artifact handoff validation, Merkle traversal policy, and workflow-shaped retry policy.

## First Slice Refactor Work

- extract target expansion outputs into typed artifacts
- extract Merkle traversal inputs and outputs into a separate capability contract
- isolate generation execution behind a capability-facing adapter
- remove hidden coupling between generation execution and Merkle traversal policy
- remove hidden coupling between generation execution and workflow-wide retry policy
- make result artifacts explicit enough for downstream file materialization

## Required Inputs

The first slice `context_generate` capability must accept scope reference, ordered Merkle node set artifact, generation policy binding, provider binding, and agent binding when required.

## Required Outputs

The first slice `context_generate` capability must emit generation result artifact, frame reference artifact when materialized frames exist, structured observation summary, and structured effect summary.

## Non Goals

- moving plan compilation into `context`
- preserving hidden Merkle traversal behavior inside `context`
- preserving workflow-specific sequencing inside `context`
