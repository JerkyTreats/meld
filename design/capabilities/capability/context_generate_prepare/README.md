# Context Generate Prepare

Date: 2026-04-04
Status: active
Scope: first-slice task-facing capability for assembling provider-ready generation input

## Intent

`ContextGeneratePrepare` turns node-scoped context, prompt data, and bound generation policy into one `provider_execute_request` artifact.
It is the context-side preparation half of generation.

## Boundary

This capability should:

- consume structured node and lineage inputs
- resolve prompts and context payload
- prepare provider-ready request data
- emit preparation and observation summaries

This capability should not:

- perform provider transport
- persist task artifacts
- decide retry or ordering

## Runtime Initialization

Runtime initialization should hold:

- capability instance identity
- node scope kind
- bound agent reference
- bound provider reference
- bound generation policy
- input and output slot contracts

## Invocation Payload Example

```json
{
  "invocation_id": "invk_ctx_prepare_001",
  "capability_instance_id": "capinst_ctx_prepare_docs_writer_v1",
  "supplied_inputs": [
    {
      "slot_id": "resolved_node_ref",
      "source": "ArtifactHandoff",
      "value": {
        "artifact_id": "artifact_resolved_node_ref_pkg_a_readme",
        "artifact_type_id": "resolved_node_ref",
        "schema_version": "v1",
        "content": {
          "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4",
          "path": "packages/pkg-a/README.md"
        }
      }
    },
    {
      "slot_id": "upstream_observation",
      "source": "ArtifactHandoff",
      "value": {
        "artifact_id": "artifact_parent_summary_001",
        "artifact_type_id": "generation_result",
        "schema_version": "v1",
        "content": {
          "summary": "Parent package README established package purpose."
        }
      }
    },
    {
      "slot_id": "force_posture",
      "source": "InitPayload",
      "value": {
        "artifact_type_id": "force_posture",
        "schema_version": "v1",
        "content": {
          "force": false,
          "replay": false
        }
      }
    }
  ],
  "upstream_lineage": {
    "task_id": "task_docs_writer",
    "task_run_id": "taskrun_docs_writer_001",
    "batch_index": 2,
    "node_index": 0
  },
  "execution_context": {
    "attempt": 1,
    "trace_id": "trace_docs_writer_001"
  }
}
```

## Sig Adapter Resolution

The sig adapter resolves:

- `resolved_node_ref`
- optional `upstream_observation`
- optional `force_posture`
- bound agent, provider, and generation policy values from runtime initialization

Into an internal chain similar to:

```rust
load_node_and_agent
-> build_prompt_messages
-> prepare_generated_lineage
-> shape_provider_execute_request
```

## Artifacts Out

Primary emitted artifact:

```json
{
  "artifact_id": "artifact_provider_execute_request_pkg_a_readme",
  "artifact_type_id": "provider_execute_request",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_ctx_prepare_docs_writer_v1",
    "invocation_id": "invk_ctx_prepare_001",
    "output_slot_id": "provider_execute_request"
  },
  "content": {
    "request_id": "provider_req_001",
    "provider_ref": "provider/openai_primary",
    "model": "gpt-4.1",
    "messages": [
      {
        "role": "system",
        "content": "You are the docs writer for this workspace."
      },
      {
        "role": "user",
        "content": "Write the README for packages/pkg-a using the provided project context."
      }
    ],
    "response_contract": {
      "kind": "text_completion"
    }
  }
}
```

Supporting artifacts:

```json
{
  "artifact_id": "artifact_preparation_summary_pkg_a_readme",
  "artifact_type_id": "preparation_summary",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_ctx_prepare_docs_writer_v1",
    "invocation_id": "invk_ctx_prepare_001",
    "output_slot_id": "preparation_summary"
  },
  "content": {
    "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4",
    "agent_id": "docs_writer",
    "frame_type": "readme",
    "prompt_digest": "sha256:abc123",
    "context_digest": "sha256:def456"
  }
}
```

```json
{
  "artifact_id": "artifact_prompt_context_lineage_pkg_a_readme",
  "artifact_type_id": "prompt_context_lineage_summary",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_ctx_prepare_docs_writer_v1",
    "invocation_id": "invk_ctx_prepare_001",
    "output_slot_id": "prompt_context_lineage_summary"
  },
  "content": {
    "prompt_link_id": "plink_001",
    "prompt_digest": "sha256:abc123",
    "context_digest": "sha256:def456"
  }
}
```

## Failure Shape

```json
{
  "failure_kind": "PreparationFailed",
  "message": "Prompt context could not be assembled",
  "details": {
    "node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4"
  }
}
```

## Read With

- [Capability Model](../README.md)
- [Capabilities By Domain](../by_domain.md)
- [Context Technical Spec](../../../completed/capability_refactor/context/technical_spec.md)
- [Task Design](../../task/README.md)
