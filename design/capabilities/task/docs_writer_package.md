# Docs Writer Package

Date: 2026-04-04
Status: active
Scope: concrete workflow-package retrofit for docs writer as a task-driven execution unit

## Intent

Define one full docs-writer package that `src/control` can trigger and lower into a task DAG.

This document retrofits the existing idea of `workflow` into a package and trigger surface over a fully task-driven execution model.
The old durable workflow runtime goes away.
What remains is:

- one package that describes docs-writer behavior
- one trigger contract that `control` can accept
- one task definition that compiles into a task DAG
- one live task run that executes through task and capability boundaries

## Package Thesis

The docs-writer package should be treated as:

- a named behavior package
- a task-definition source
- a prompt and policy bundle
- a trigger-facing compatibility surface

It should not be treated as:

- a second runtime
- a hidden executor
- a separate orchestration engine next to task

In other words, the package is packaging.
Execution is still task-driven.

## Package Contents

A docs-writer package should include:

- package identity and version
- trigger schema
- default bindings
- prompt asset references
- task template
- output artifact contract expectations

Recommended conceptual package contents:

```yaml
package_id: docs_writer
package_version: v2
trigger:
  trigger_id: workflow.docs_writer.run
  init_slots:
    - target_selector
    - traversal_strategy
    - force_posture
defaults:
  agent_ref: agent/docs_writer
  provider_ref: provider/default
  frame_type: readme
  traversal_strategy: bottom_up
assets:
  prompts:
    evidence_gather: prompts/docs_writer/evidence_gather.md
    verification: prompts/docs_writer/verification.md
    readme_struct: prompts/docs_writer/readme_struct.md
    style_refine: prompts/docs_writer/style_refine.md
task_template:
  template_id: docs_writer_bottom_up_v1
```

## Control Trigger Contract

`src/control` should trigger docs writer through one package-facing request.

Recommended shape:

```rust
struct WorkflowPackageTriggerRequest {
    package_id: WorkflowPackageId,
    package_version: WorkflowPackageVersion,
    trigger_id: WorkflowTriggerId,
    init_payload: TaskInitializationPayload,
    run_policy: TaskRunPolicy,
}
```

Control should use that request to:

1. resolve the package
2. load the task template from the package
3. compile or load the compiled task record
4. create one live task run
5. dispatch that task run into task execution

After that point, capability progression belongs to task execution, not to the package trigger surface.

## What Control Actually Triggers

Control should not trigger individual capabilities for docs writer.
Control should trigger one docs-writer task run.

That means the package trigger path is:

```text
control trigger
  -> resolve docs_writer package
  -> materialize task definition
  -> compile task DAG
  -> create task run with seed artifacts
  -> dispatch task run
```

This keeps the packaging layer thin and the task layer durable.

## Initialization Payload

The docs-writer package should expect seed artifacts such as:

- `target_selector`
- optional `traversal_strategy`
- optional `force_posture`
- optional `provider_override`
- optional `run_label`

Example initialization payload:

```json
{
  "task_id": "task_docs_writer",
  "compiled_task_ref": "compiled_task_docs_writer_bottom_up_v1",
  "init_artifacts": [
    {
      "slot_id": "target_selector",
      "artifact_type_id": "target_selector",
      "schema_version": "v1",
      "content": {
        "path": "packages/pkg-a"
      }
    },
    {
      "slot_id": "traversal_strategy",
      "artifact_type_id": "traversal_strategy",
      "schema_version": "v1",
      "content": {
        "strategy": "bottom_up"
      }
    },
    {
      "slot_id": "force_posture",
      "artifact_type_id": "force_posture",
      "schema_version": "v1",
      "content": {
        "force": false,
        "replay": false
      }
    }
  ],
  "task_run_context": {
    "requested_by": "workflow.docs_writer.run",
    "trace_id": "trace_docs_writer_pkg_a_001"
  }
}
```

These seed artifacts become the first entries in the task artifact repo.

## Task Template Shape

The docs-writer package should lower to one task definition template that uses repeated capability types with different bindings.

At the top level, the template should include:

1. one `WorkspaceResolveNodeId` instance
2. one `MerkleTraversal` instance
3. one repeated per-node generation region

That repeated region should reuse the same first-slice capability types:

- `ContextGeneratePrepare`
- `ProviderExecuteChat`
- `ContextGenerateFinalize`

The variation between docs-writer stages should come from bindings and output contracts, not from inventing a new executor.

## Per Node Docs Writer Region

For each traversed node, the package should compile a four-stage docs-writer chain that matches the current workflow behavior:

1. evidence gather
2. verification
3. readme struct
4. style refine

Each stage should compile as capability instances like:

- prepare
- execute
- finalize

That means one node region conceptually expands to:

```text
evidence_prepare
  -> evidence_execute
  -> evidence_finalize
  -> verification_prepare
  -> verification_execute
  -> verification_finalize
  -> struct_prepare
  -> struct_execute
  -> struct_finalize
  -> style_prepare
  -> style_execute
  -> style_finalize
```

The capability types stay stable.
The bindings differ by stage:

- prompt asset ref
- expected output artifact type
- frame type or persistence policy where needed

## Docs Writer Artifact Chain

The docs-writer region should emit at least these logical artifact families:

- `evidence_map`
- `verification_report`
- `readme_struct`
- `readme_final`
- `frame_ref`
- `effect_summary`

Those artifacts should drive the per-node chain:

- `verification_prepare` requires `evidence_map`
- `struct_prepare` requires `verification_report`
- `style_prepare` requires `readme_struct`
- `style_finalize` emits `readme_final` and final frame outputs

## Bottom Up Dependency Rule

To preserve the current bottom-up docs-writer behavior, parent node generation should also depend on child node completion artifacts.

Recommended durable rule:

- parent node `evidence_prepare` requires child `readme_final` summary artifacts for all direct child nodes when such children exist

That one rule is enough to encode bottom-up dependency durably in the task graph.
It means:

- leaves can begin immediately after traversal
- sibling leaves can run in parallel
- a parent remains blocked until child final outputs exist

This is task-owned dependency truth, not control-owned special knowledge.

## Compiled DAG Sketch

Conceptually, the compiled docs-writer DAG should look like:

```text
seed target_selector
  -> workspace_resolve_node_id
  -> merkle_traversal

for each leaf node:
  -> evidence_prepare
  -> evidence_execute
  -> evidence_finalize
  -> verification_prepare
  -> verification_execute
  -> verification_finalize
  -> struct_prepare
  -> struct_execute
  -> struct_finalize
  -> style_prepare
  -> style_execute
  -> style_finalize

for each parent node:
  child style_finalize outputs
    -> parent evidence_prepare
```

Within one node region, the chain is serial.
Across independent nodes in the same ready region, execution is parallel-safe.

## Compile Time Responsibilities

When the docs-writer package is lowered into a task definition, compile time should:

- create stable capability instance ids for all stage instances
- bind prompt and policy refs for each stage
- derive the within-node serial edges
- derive the child-to-parent artifact dependency edges
- validate that each stage input is satisfiable from either seed artifacts or upstream stage outputs
- fail compilation if any required artifact contract is missing or ambiguous

This means the ordering knowledge becomes part of the compiled task record.

## Run Time Responsibilities

After compilation:

- task executor assembles invocation payloads from the artifact repo
- task executor releases all ready leaf stage instances in parallel where legal
- task executor persists emitted artifacts after each invocation
- task executor unlocks parent stages when required child artifacts appear
- control remains above that flow as task-network dispatch and retry-intent owner

## Error Boundaries

The docs-writer package should respect the standard split:

- package resolution error
  - package id, trigger id, or asset bundle is invalid
- task compile error
  - the docs-writer task definition is structurally invalid
- task run creation error
  - required seed artifacts were not supplied
- capability invocation error
  - one evidence, verification, struct, or style stage failed

This avoids hiding package-definition errors inside runtime execution.

## Why This Retrofits Workflow Correctly

The old workflow concept survives only as:

- package identity
- trigger identity
- prompt and policy bundle
- compatibility-facing entry surface

The actual execution model becomes:

- task definition
- compiled task DAG
- task artifact repo
- task executor
- capabilities

That is the retrofit:

- workflow as package
- task as durable execution unit

## Read With

- [Task Design](README.md)
- [Task Control Boundary](../task_control_boundary.md)
- [Capability Model](../capability/README.md)
- [Workflow Refactor](../workflow_refactor/README.md)
- [Interregnum Orchestration](../../control/interregnum_orchestration.md)
