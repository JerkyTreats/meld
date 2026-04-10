# Bug: readme_final Used as Prerequisite Artifact Type Instead of frame_ref

Date: 2026-04-07
Status: fixed
Scope: docs_writer prerequisite artifact type in package spec and workflow YAML

## Summary

The bottom-up prerequisite dependency in docs_writer uses `readme_final` as the artifact type
for the child-to-parent handoff. `readme_final` is a content artifact. The frame store already
holds this content, keyed by `FrameID`. Using `readme_final` as the prerequisite type causes the
task artifact repo to duplicate content that already exists in the frame store.

The correct prerequisite artifact type is `frame_ref`, which `context_generate_finalize` already
emits as a declared output alongside `readme_final`.

## Affected Locations

- `design/completed/capabilities/task/docs_writer_package.md`, line 254
  - "parent node `evidence_prepare` requires child `frame_ref` artifacts with `frame_type: readme`"
- `workflows/packages/docs_writer_v2.yaml`, lines 43, 82, 90
  - `existing_output_artifact_type_id: readme_final`
  - `output_type: readme_final`
  - `producer_artifact_type_id: readme_final`

## Root of the Problem

`context_generate_finalize` emits two distinct artifacts:

- `generation_result` / `readme_final`: the full README content
- `frame_ref`: `{ frame_id, node_id, frame_type }` — a pointer into the frame store

These are different artifact types with different semantics. `readme_final` carries content.
`frame_ref` carries a reference.

The prerequisite slot wiring for the bottom-up dependency (child output → parent input) should
use `frame_ref`, not `readme_final`. The parent `evidence_prepare` capability does not need the
content inlined into the task artifact repo. It needs to know which frame to read. It dereferences
the `frame_ref` through the frame store to get the actual content.

## Consequences of the Current Design

**Storage duplication**: the README text exists in the frame store (content-addressed by FrameID)
and again as a `readme_final` artifact in the task artifact repo. Same bytes, two stores.

**Ambiguous prior source**: when computing Bayesian priors from frame history, it is unclear
whether to walk the frame chain (frame store, workspace-scoped, durable) or the task artifact
projection (task-scoped, ephemeral). With `readme_final` as content in the artifact repo, both
appear to be authoritative. With `frame_ref` as the prerequisite type, the frame store is
unambiguously the source.

**Naming confusion**: `readme_final` implies content finality. Using it for the prerequisite slot
conflates "the final README content this task produced" with "the artifact another task needs to
proceed." These are different questions answered by different artifact types.

## Correct Design

The prerequisite slot for the bottom-up dependency should use `frame_ref`:

```
parent evidence_prepare requires child frame_ref artifacts
  where frame_ref.frame_type == "readme"
  for all direct child nodes when such children exist
```

The `frame_ref` artifact is:
- already emitted by `context_generate_finalize` (declared output slot)
- already scoped to `{ frame_id, node_id, frame_type }` — sufficient for the parent to locate
  the child's README via the frame store
- not a duplication of content — pointer only

`readme_final` may still exist as the within-task content artifact consumed by the final
`style_refine` turn. That use is correct. The problem is specifically its use as the
**inter-node prerequisite type** that crosses from child task region into parent task region.

## Fix

1. Update `design/capabilities/task/docs_writer_package.md` line 254:
   - change "requires child `readme_final` summary artifacts" to "requires child `frame_ref`
     artifacts (frame_type: readme)"

2. Update `workflows/packages/docs_writer_v2.yaml`:
   - change `existing_output_artifact_type_id: readme_final` to `frame_ref`
   - change `producer_artifact_type_id: readme_final` to `frame_ref`
   - the `output_type: readme_final` on the `style_refine` turn is within-task and correct as-is

3. Ensure the `evidence_prepare` capability (or its sig adapter) dereferences `frame_ref`
   through the frame store rather than treating the prerequisite artifact as inline content.

## Non-Fix

The `readme_final` artifact type itself is not wrong. It is the correct type for the final
README content within a task region. Only its use as the cross-node prerequisite artifact type
is the bug.

## Read With

- [Docs Writer Package](docs_writer_package.md)
- [Context Generate Finalize](../capability/context_generate_finalize/README.md)
- [Task Design](README.md)
