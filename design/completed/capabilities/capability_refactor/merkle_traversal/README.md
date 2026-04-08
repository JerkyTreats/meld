# Merkle Traversal Capability

Date: 2026-03-28
Status: completed

## Intent

Define the capability that derives definitive ordered Merkle node batches from a Merkle scope and a traversal strategy.

## Why This Exists

Current `context generate` behavior still carries tree ordering logic inside the `context` domain.
That mixes traversal derivation with orchestration concerns.

This capability makes traversal derivation explicit and separate.

## Functional Definition

`merkle_traversal` takes a Merkle tree scope plus traversal bindings and emits typed ordered Merkle node batches.

It does not generate context.
It does not write files.
It does not execute downstream capabilities.

## First Slice Design Decisions

The user-facing capability input must carry an explicit `traversal_strategy` field.
The first slice accepted values are `bottom_up` and `top_down`.

Internally, the first slice should represent strategy as a typed enum rather than as open-ended runtime polymorphism.
That keeps the contract explicit while keeping the implementation simple.

The output artifact for the first slice is `ordered_merkle_node_batches`.
That artifact is the stable output contract consumed by downstream capabilities.

## Inputs

The first slice input contract includes tree scope reference, target selection artifact, `traversal_strategy`, and traversal policy binding.

## Outputs

The first slice output contract includes `ordered_merkle_node_batches`, traversal metadata artifact, and structured observation summary.

## Invocation Payload Example

```json
{
  "invocation_id": "invk_merkle_traversal_001",
  "capability_instance_id": "capinst_merkle_traversal_docs_writer_v1",
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
      "slot_id": "traversal_strategy",
      "source": "InitPayload",
      "value": {
        "artifact_type_id": "traversal_strategy",
        "schema_version": "v1",
        "content": {
          "strategy": "bottom_up"
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

## Artifacts Out Example

Primary emitted artifact:

```json
{
  "artifact_id": "artifact_ordered_merkle_node_batches_pkg_a_readme",
  "artifact_type_id": "ordered_merkle_node_batches",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_merkle_traversal_docs_writer_v1",
    "invocation_id": "invk_merkle_traversal_001",
    "output_slot_id": "ordered_merkle_node_batches"
  },
  "content": {
    "strategy": "bottom_up",
    "batches": [
      [
        "2f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5001",
        "2f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5002"
      ],
      [
        "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4"
      ]
    ]
  }
}
```

Supporting artifact:

```json
{
  "artifact_id": "artifact_traversal_metadata_pkg_a_readme",
  "artifact_type_id": "traversal_metadata",
  "schema_version": "v1",
  "producer": {
    "capability_instance_id": "capinst_merkle_traversal_docs_writer_v1",
    "invocation_id": "invk_merkle_traversal_001",
    "output_slot_id": "traversal_metadata"
  },
  "content": {
    "root_node_id": "9f6d8d7f1f1d7e5a1a7f8b4f5c3e2d1a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4",
    "batch_count": 2,
    "node_count": 3
  }
}
```

## Contract Rules

- the contract describes traversal derivation only
- the contract does not assume downstream intent
- the contract does not encode `context generate` semantics
- the contract must support multiple traversal strategies behind one stable capability id
- the contract is strategy in, ordered Merkle node batches out
- the first slice internal representation should use a closed enum for strategy selection
- the first slice should avoid trait-object-heavy strategy infrastructure unless future strategy growth makes it necessary

## Strategy Variants

The first slice supports `bottom_up` and `top_down`.

Additional strategies can be added later without changing the capability identity.

## Internal Representation

The first slice should use a typed internal representation such as `TraversalStrategy`.
That representation should be matched directly by the traversal service.

This is strategy-shaped behavior, but it does not require heavy design-pattern machinery.
The stable abstraction is the capability contract.
The initial implementation can stay as a small enum plus one algorithm per variant.

## Domain Boundary

The Merkle tree and traversal logic should live behind this capability contract.
`control` must consume the ordered Merkle node batch artifact rather than derive traversal internally.

## First Slice Refactor Impact

Making this capability explicit requires:

- pulling tree ordering logic out of `context generate`
- producing a typed ordered Merkle node batch artifact
- moving traversal policy selection outside `context generate`
- compiling traversal as its own capability instance in the candidate capability graph

## Non Goals

- choosing why traversal is needed
- deciding which downstream capability should consume the traversal
- embedding provider timing or batch barriers into the traversal implementation
