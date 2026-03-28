# Workflow Refactor

Date: 2026-03-28
Status: active
Scope: remove current workflow functionality so capability and plan can become the durable orchestration model

## Intent

Define the cleanup required around the current workflow implementation.
This area exists to clear the ground for the capability layer and plan compiler.

The new direction does not preserve `workflow` as a durable abstraction.
Current workflow behavior is treated as legacy orchestration that must be removed or replaced.

## Direction

The immediate direction is:

- stop treating workflows as a first-class product surface
- stop treating workflow binding as an agent concern
- pull reusable implementation seams out of workflow internals
- express docs-writer behavior later as capability graph compilation rather than workflow turns

`docs_writer_thread_v1` is still useful as a concrete example of multi-step behavior.
What survives is the behavior shape, not the workflow abstraction.

## Cleanup Goal

Start condition:
- workflows exist as profile registry, executor, state store, CLI command surface, watch-mode path, and context execution mode

End condition:
- workflow-specific seams no longer shape the runtime
- capability and plan own the durable orchestration model
- docs writer becomes a future capability-graph example rather than a workflow profile

## Breaking Change Note

This cleanup intentionally breaks current workflow functionality.

As part of this refactor:

- workflow binding is removed from agents
- workflow-specific execution routing is removed from context and watch mode
- workflow CLI surfaces are removed or reduced to migration-only helpers
- workflow thread and turn state stop being the durable orchestration model

This is intentional.
The functionality can return later once capability and plan exist at the correct abstraction layer.

## Read With

- [Workflow Cleanup Technical Spec](technical_spec.md)
- [Workflow Refactor Code Path Findings](code_path_findings.md)
- [Capability And Plan Design](../README.md)
- [Capability And Plan Implementation Plan](../PLAN.md)
- [Merkle Traversal Technical Spec](../capability/merkle_traversal/technical_spec.md)
- [Context Generate Technical Spec](../context/technical_spec.md)
