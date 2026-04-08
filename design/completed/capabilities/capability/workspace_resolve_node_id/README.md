# Workspace Resolve Node Id

Date: 2026-04-04
Status: active
Scope: first-slice task-facing capability for turning authored target input into one stable node reference

## Intent

`WorkspaceResolveNodeId` is the entry capability for authored path-like or node-id-like inputs.
It gives downstream capabilities one stable `resolved_node_ref` artifact rather than forcing every later capability to repeat target parsing and lookup work.

## Boundary

This capability should:

- accept structured target selector input
- apply workspace-root and lookup policy bindings
- emit one stable node reference artifact and one summary artifact

This capability should not:

- traverse the Merkle tree
- assemble prompt context
- talk to providers
- persist task artifacts

## Runtime Initialization

Runtime initialization should hold:

- capability instance identity
- workspace scope kind
- `workspace_root` binding
- optional `include_tombstoned` policy binding
- input and output slot contracts

## Invocation Payload Example

The caller should provide only structured external values:

```json
{
  "invocation_id": "invk_resolve_001",
  "capability_instance_id": "capinst_workspace_resolve_docs_writer_v1",
  "supplied_inputs": [
    {
      "slot_id": "target_selector",
      "source": "InitPayload",
      "value": {
        "artifact_type_id": "target_selector",
        "schema_version": "v1",
        "content": {
          "path": "packages/pkg-a/README.md"
        }
      }
    }
  ],
  "upstream_lineage": {
    "task_id": "task_docs_writer",
    "task_run_id": "taskrun_docs_writer_001"
  },
  "execution_context": {
    "attempt": 1,
    "trace_id": "trace_docs_writer_001"
  }
}
```

## Sig Adapter Resolution

The sig adapter resolves:

- `target_selector` input artifact
- `workspace_root` binding
- optional `include_tombstoned` policy binding

Into internal arguments similar to:

```rust
resolve_target_node(api, workspace_root, path_or_node_id, include_tombstoned)
```

## Artifacts Out

Primary emitted artifact:

```json
{
  "artifact_id": "artifact_resolved_node_ref_pkg_a_readme",
  "artifact_type_id": "resolved_node_ref",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_workspace_resolve_docs_writer_v1",
    "invocation_id": "invk_resolve_001",
    "output_slot_id": "resolved_node_ref"
  },
  "content": {
    "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4",
    "path": "packages/pkg-a/README.md",
    "include_tombstoned": false
  }
}
```

Summary artifact:

```json
{
  "artifact_id": "artifact_target_resolution_summary_pkg_a_readme",
  "artifact_type_id": "target_resolution_summary",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_workspace_resolve_docs_writer_v1",
    "invocation_id": "invk_resolve_001",
    "output_slot_id": "target_resolution_summary"
  },
  "content": {
    "selector_kind": "path",
    "lookup_mode": "workspace_path_then_canonical_fallback",
    "resolved": true
  }
}
```

## Failure Shape

If resolution fails, the capability should emit structured failure data upward rather than persisting anything itself:

```json
{
  "failure_kind": "TargetResolutionFailed",
  "message": "Path not found in workspace tree",
  "details": {
    "path": "packages/pkg-a/README.md"
  }
}
```

## Read With

- [Capability Model](../README.md)
- [Capabilities By Domain](../by_domain.md)
- [Task Design](../../task/README.md)
