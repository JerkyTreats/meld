# Context Generate Finalize

Date: 2026-04-04
Status: active
Scope: first-slice task-facing capability for validating generation results and emitting durable context-facing outputs

## Intent

`ContextGenerateFinalize` turns provider output plus preparation context into validated generated results and frame-related effect outputs.
It is the context-side finalization half of generation.

## Boundary

This capability should:

- consume provider result and preparation summary artifacts
- validate generated metadata inputs
- shape generated content into result and frame outputs
- declare durable write effects

This capability should not:

- persist task artifacts
- decide whether retry should occur
- perform provider transport

## Runtime Initialization

Runtime initialization should hold:

- capability instance identity
- frame family scope
- frame type binding
- persistence policy binding when statically chosen
- input and output slot contracts
- effect contract for frame and head writes

## Invocation Payload Example

```json
{
  "invocation_id": "invk_ctx_finalize_001",
  "capability_instance_id": "capinst_ctx_finalize_docs_writer_v1",
  "supplied_inputs": [
    {
      "slot_id": "provider_execute_result",
      "source": "ArtifactHandoff",
      "value": {
        "artifact_id": "artifact_provider_execute_result_pkg_a_readme",
        "artifact_type_id": "provider_execute_result",
        "schema_version": "v1",
        "content": {
          "request_id": "provider_req_001",
          "provider_name": "openai_primary",
          "model": "gpt-4.1",
          "finish_reason": "stop",
          "content": "# Package A\n\nPackage A provides ...",
          "normalized_status": "succeeded"
        }
      }
    },
    {
      "slot_id": "preparation_summary",
      "source": "ArtifactHandoff",
      "value": {
        "artifact_id": "artifact_preparation_summary_pkg_a_readme",
        "artifact_type_id": "preparation_summary",
        "schema_version": "v1",
        "content": {
          "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4",
          "agent_id": "docs_writer",
          "frame_type": "readme",
          "prompt_digest": "sha256:abc123",
          "context_digest": "sha256:def456"
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

- `provider_execute_result`
- `preparation_summary`
- bound frame type and persistence policy values from runtime initialization

Into an internal chain similar to:

```rust
build_and_validate_generated_metadata
-> construct_frame_output
-> apply_context_write_effects
-> shape_finalization_outputs
```

## Artifacts Out

Primary emitted artifact:

```json
{
  "artifact_id": "artifact_generation_result_pkg_a_readme",
  "artifact_type_id": "generation_result",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_ctx_finalize_docs_writer_v1",
    "invocation_id": "invk_ctx_finalize_001",
    "output_slot_id": "generation_result"
  },
  "content": {
    "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4",
    "frame_type": "readme",
    "content": "# Package A\n\nPackage A provides ...",
    "status": "materialized"
  }
}
```

Supporting artifacts:

```json
{
  "artifact_id": "artifact_frame_ref_pkg_a_readme",
  "artifact_type_id": "frame_ref",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_ctx_finalize_docs_writer_v1",
    "invocation_id": "invk_ctx_finalize_001",
    "output_slot_id": "frame_ref"
  },
  "content": {
    "frame_id": "1c4f9d5c9a7b2e6d4f3a1b8c7e9d2f6a4c1b3d5e7f9a2c4d6e8f0a1b2c3d4e5",
    "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4",
    "frame_type": "readme"
  }
}
```

```json
{
  "artifact_id": "artifact_effect_summary_pkg_a_readme",
  "artifact_type_id": "effect_summary",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_ctx_finalize_docs_writer_v1",
    "invocation_id": "invk_ctx_finalize_001",
    "output_slot_id": "effect_summary"
  },
  "content": {
    "writes": [
      {
        "effect_target": "frame_store",
        "kind": "exclusive_write"
      },
      {
        "effect_target": "active_head",
        "kind": "exclusive_write"
      }
    ]
  }
}
```

## Failure Shape

```json
{
  "failure_kind": "FinalizationFailed",
  "message": "Generated metadata validation failed",
  "details": {
    "frame_type": "readme",
    "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4"
  }
}
```

## Read With

- [Capability Model](../README.md)
- [Capabilities By Domain](../by_domain.md)
- [Context Technical Spec](../../context/technical_spec.md)
- [Task Design](../../task/README.md)
