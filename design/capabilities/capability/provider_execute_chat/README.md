# Provider Execute Chat

Date: 2026-04-04
Status: active
Scope: first-slice task-facing capability for provider execution over ready generation work

## Intent

`ProviderExecuteChat` is the provider-domain execution capability for ready chat-like requests.
It accepts provider-ready structured input and emits normalized provider result artifacts.

## Boundary

This capability should:

- accept one or more provider-ready execution requests
- apply provider-specific batching, throttling, and retries
- emit normalized result, usage, and timing artifacts

This capability should not:

- assemble prompt context
- persist task artifacts
- decide task readiness or retry intent

## Runtime Initialization

Runtime initialization should hold:

- capability instance identity
- provider execution class
- static lane and retry policy bindings when chosen at compile time
- input and output slot contracts
- batching and throttling policy metadata

## Invocation Payload Example

```json
{
  "invocation_id": "invk_provider_execute_001",
  "capability_instance_id": "capinst_provider_execute_docs_writer_v1",
  "supplied_inputs": [
    {
      "slot_id": "provider_execute_request",
      "source": "ArtifactHandoff",
      "value": {
        "artifact_id": "artifact_provider_execute_request_pkg_a_readme",
        "artifact_type_id": "provider_execute_request",
        "schema_version": "v1",
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

- `provider_execute_request`
- optional dynamic lane hint or retry class values from execution context
- static provider policy bindings from runtime initialization

Into an internal chain similar to:

```rust
group_for_provider_lane
-> prepare_provider_client
-> execute_completion
-> normalize_result
```

## Artifacts Out

Primary emitted artifact:

```json
{
  "artifact_id": "artifact_provider_execute_result_pkg_a_readme",
  "artifact_type_id": "provider_execute_result",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_provider_execute_docs_writer_v1",
    "invocation_id": "invk_provider_execute_001",
    "output_slot_id": "provider_execute_result"
  },
  "content": {
    "request_id": "provider_req_001",
    "provider_name": "openai_primary",
    "model": "gpt-4.1",
    "finish_reason": "stop",
    "content": "# Package A\n\nPackage A provides ...",
    "normalized_status": "succeeded"
  }
}
```

Supporting artifacts:

```json
{
  "artifact_id": "artifact_provider_usage_pkg_a_readme",
  "artifact_type_id": "provider_usage_summary",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_provider_execute_docs_writer_v1",
    "invocation_id": "invk_provider_execute_001",
    "output_slot_id": "provider_usage_summary"
  },
  "content": {
    "prompt_tokens": 812,
    "completion_tokens": 426,
    "total_tokens": 1238
  }
}
```

```json
{
  "artifact_id": "artifact_provider_timing_pkg_a_readme",
  "artifact_type_id": "provider_timing_summary",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_provider_execute_docs_writer_v1",
    "invocation_id": "invk_provider_execute_001",
    "output_slot_id": "provider_timing_summary"
  },
  "content": {
    "duration_ms": 1820,
    "lane_id": "openai_primary:gpt-4.1:text_completion"
  }
}
```

## Failure Shape

```json
{
  "failure_kind": "ProviderExecutionFailed",
  "message": "Provider rate limit exceeded",
  "details": {
    "provider_name": "openai_primary",
    "retry_class": "backoff_retryable"
  }
}
```

## Read With

- [Capability Model](../README.md)
- [Capabilities By Domain](../by_domain.md)
- [Provider Capability Design](../../provider/README.md)
- [Task Design](../../task/README.md)
