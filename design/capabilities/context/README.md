# Context Capability Readiness

Date: 2026-03-27
Status: active

## Intent

Define the refactor work required to make current `context generate` behavior capability-ready and task-ready.

## Current Problem

Current `context generate` behavior still mixes target expansion, Merkle traversal, generation execution, retry and resume assumptions, workflow-shaped sequencing, and output shaping.
That mixed ownership prevents clean capability contracts and clean task compilation.

## Required End State

The `context` domain should own only domain behavior behind explicit capability contracts.

For the first slice, `context` must expose task-facing preparation and finalization capability contracts, typed input artifact contracts, typed output artifact contracts, and the execution-facing implementation behind those contracts.

The following concerns must move out of `context`: compiled task graph assembly, dependency edge construction, artifact handoff validation, Merkle traversal policy, and workflow-shaped retry policy.

## First Slice Refactor Work

- extract target expansion outputs into typed artifacts
- extract Merkle traversal inputs and outputs into a separate capability contract
- isolate generation execution behind a capability-facing adapter
- remove hidden coupling between generation execution and Merkle traversal policy
- remove hidden coupling between generation execution and workflow-wide retry policy
- make result artifacts explicit enough for downstream file materialization

## Required Inputs

The first slice `ContextGeneratePrepare` capability must accept node scope reference, generation policy binding, provider binding, agent binding when required, and explicit upstream lineage or observation inputs when present.

The first slice `ContextGenerateFinalize` capability must accept provider execute result, preparation summary, and any persistence policy binding needed for frame materialization.

## Required Outputs

The first slice `ContextGeneratePrepare` capability must emit a provider-ready execute request plus structured preparation summary.

The first slice `ContextGenerateFinalize` capability must emit generation result artifact, frame reference artifact when materialized frames exist, structured observation summary, and structured effect summary.

## Non Goals

- moving task compilation into `context`
- preserving hidden Merkle traversal behavior inside `context`
- preserving workflow-specific sequencing inside `context`

## Read With

- [Context Code Path Findings](code_path_findings.md)
- [Context Technical Spec](technical_spec.md)
- [Capability And Task Design](../README.md)
- [Domain Architecture](../domain_architecture.md)
