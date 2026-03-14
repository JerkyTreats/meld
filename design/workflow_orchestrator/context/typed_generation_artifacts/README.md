# Typed Generation Artifacts

Date: 2026-03-14
Status: active

## Parent Requirements

- [Context Refactor Requirements](../README.md)
- [Primitive Task Contract](../../primitive_task_contract/README.md)
- [Task Model](../../task_model/README.md)

## Intent

Emit typed generation artifacts with stable schema identifiers so workflow compilation can validate context outputs the same way it validates any other task family.

## Current Pressure

- current context execution returns strong domain results, but workflow ready artifact typing is still incomplete
- prompt output, context payload, frame lineage, and generation summaries need durable type identity for handoff and validation
- without stable artifact typing, downstream file write, publish, and repair logic would need to infer too much from ad hoc result shapes

## Requirements

- context generation outputs must declare stable artifact type identifiers and schema versions
- artifact contracts must cover rendered prompt, context payload, frame reference, and execution summary outputs as needed
- workflow compilation must be able to validate downstream bindings against those artifact contracts before runtime starts
- artifact typing must remain compatible with current context storage and lineage guarantees

## Planned Deliverables

- a typed artifact catalog for context generation outputs
- workflow visible schema identifiers for generation result families
- binding examples for downstream file write, validation, and publish tasks

## Verification Focus

- downstream tasks reject mismatched generation artifacts before execution
- schema changes are versioned rather than inferred from runtime behavior
- artifact typing strengthens repair and resume decisions without duplicating context storage logic

## Related Code

- `src/context/generation/contracts.rs`
- `src/context/generation/orchestration.rs`
- `src/context/generation/plan.rs`
- `src/workflow/record_contracts`
