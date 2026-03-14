# Context Refactor Requirements

Date: 2026-03-14
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)
- [Context Generate Task](../context_generate_task/README.md)

## Intent

Break the HTN driven `src/context` refactor into requirement scoped work items.
These work items preserve domain ownership while moving global orchestration policy into workflow.

## Scope

- keep prompt assembly, provider execution, frame persistence, and retrieval in `src/context`
- move target derivation, ordering policy, workflow level retry, repair choice, and publish handoff out of `src/context`
- define stable workflow facing contracts for generation inputs and outputs
- preserve current CLI behavior through compatibility compilation during migration

## Work Items

- [Generation Surface Focus](generation_surface_focus/README.md)
- [Compatibility Compilation](compatibility_compilation/README.md)
- [Target Plan Input](target_plan_input/README.md)
- [Quality And Metadata Parity](quality_and_metadata_parity/README.md)
- [Workflow Contract Boundary](workflow_contract_boundary/README.md)
- [Typed Generation Artifacts](typed_generation_artifacts/README.md)

## Findings

- [Code Path Findings](code_path_findings.md)

## Code Pressure

- `src/context/generation/run.rs` still owns recursive target discovery, subtree validation, and level shaping
- `src/context/queue.rs` still holds queue level orchestration seams that future workflow compilation must call through cleanly
- `src/context/generation/orchestration.rs` is the execution seam for prompt assembly, provider calls, metadata construction, and frame writes
- `src/context/generation/program.rs` and `src/context/generation/selection.rs` already expose the first public compatibility contract between `context` and workflow

## Exit Shape

- workflow compiles target plans before context execution starts
- context consumes ordered plans and returns typed generation artifacts
- workflow level retry, repair, and publish decisions no longer hide inside context execution
- command level behavior remains stable while old entry paths compile into the new workflow aware contract

## Related Areas

- [Context Generate Task](../context_generate_task/README.md)
- [Ordering Task](../ordering_task/README.md)
- [Primitive Task Contract](../primitive_task_contract/README.md)
- [Task Model](../task_model/README.md)
- [Workflow Definition](../workflow_definition/README.md)
- [Migration Plan](../migration_plan/README.md)
