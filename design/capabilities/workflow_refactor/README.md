# Workflow Refactor

Date: 2026-03-28
Status: active
Scope: narrow current workflow into a compatibility trigger path while capability, provider, context, and control boundaries are established

## Intent

Define the cleanup required around the current workflow implementation.
This area exists to clear the ground for the capability layer and task compiler while keeping current trigger flows working during the refactor window.

The new direction does not preserve `workflow` as a durable abstraction.
Current workflow behavior is treated as a compatibility surface whose orchestration responsibilities must move elsewhere.

## Direction

The immediate direction is:

- stop treating workflows as a durable orchestration owner
- stop treating workflow binding as an agent concern
- pull reusable implementation seams out of workflow internals
- delegate ordered execution into `control`
- express docs-writer behavior later as task compilation rather than workflow turns

`docs_writer_thread_v1` is still useful as a concrete example of multi-step behavior.
What survives is the behavior shape, not the workflow abstraction.

## Cleanup Goal

Start condition:
- workflows exist as profile registry, executor, state store, CLI command surface, watch-mode path, and context execution mode

End condition:
- workflow survives only as a compatibility facade where still needed
- workflow-specific seams no longer shape orchestration ownership
- capability and task own the durable implementation model
- docs writer becomes a future compiled-task example rather than a durable workflow runtime

## Compatibility Note

This cleanup should preserve current end-to-end trigger flows during the refactor window.

As part of this refactor:

- workflow may remain as a CLI-facing compatibility surface
- orchestration moves into `control`
- provider execution moves into `provider`
- workflow thread and turn state stop being the durable orchestration model

The external trigger path may remain stable while internal ownership changes.

## Read With

- [Workflow Cleanup Technical Spec](technical_spec.md)
- [Workflow Refactor Code Path Findings](code_path_findings.md)
- [Capability And Task Design](../README.md)
- [Capability And Task Implementation Plan](../PLAN.md)
- [Interregnum Orchestration](../../control/interregnum_orchestration.md)
- [Merkle Traversal Technical Spec](../capability/merkle_traversal/technical_spec.md)
- [Context Code Path Findings](../context/code_path_findings.md)
- [Context Technical Spec](../context/technical_spec.md)
