# Runtime Capability Catalog

Date: 2026-04-09
Status: proposed
Scope: sled-backed catalog for synthesized capabilities, trust model, lookup semantics, and schema drift handling

## Intent

Define the runtime catalog that stores synthesized capability definitions and the lookup
semantics that allow the task compiler and HTN planner to use synthesized capabilities
alongside compiled ones.

## Two Catalogs, One Interface

The capability system has two catalogs that share a query interface:

**Compiled catalog**: HashMap populated at startup from registered Rust capability implementations.
Compiled capabilities are statically verified. Their contracts are trusted unconditionally.

**Runtime catalog**: sled namespace populated at runtime by `CapabilitySynthesisTask`.
Synthesized capabilities are validated at synthesis time. Their contracts are trusted
conditionally — subject to schema drift detection.

The query interface sees both. The caller asks for capabilities by artifact type produced.
The response merges results from both catalogs. Compiled capabilities take precedence when
both produce the same artifact type.

## Sled Namespace

```
capability_catalog::synthesized::{capability_type_id} → SynthesizedCapabilityRecord
capability_catalog::by_output::{artifact_type_id}     → Vec<capability_type_id>
capability_catalog::schema_hashes::{capability_type_id} → SchemaHash
capability_catalog::validation_records::{capability_type_id} → ValidationRecord
```

`by_output` is the secondary index that enables lookup by artifact type. Updated atomically
when a new capability is registered.

## SynthesizedCapabilityRecord

```rust
struct SynthesizedCapabilityRecord {
    capability_type_id: CapabilityTypeId,
    input_contract: Vec<InputSlotSpec>,
    output_contract: Vec<OutputSlotSpec>,
    execution_contract: ExecutionContract,        // ExecutionClass::ExternalProcess
    env_preconditions: Vec<EnvPrecondition>,
    trust_level: TrustLevel,                      // TrustLevel::Synthesized
    schema_hash: SchemaHash,                      // hash of output schema at validation time
    synthesis_task_run_id: TaskRunId,
    registered_at_seq: u64,                       // spine event sequence
    status: CatalogEntryStatus,
}

enum TrustLevel {
    Compiled,
    Synthesized,
}

enum CatalogEntryStatus {
    Active,
    Stale { detected_at_seq: u64, reason: StaleReason },
    Superseded { by: CapabilityTypeId },
}

enum StaleReason {
    SchemaHashMismatch,
    EnvPreconditionFailed,
    ExcessiveInvocationFailures { failure_count: u32 },
}
```

## Lookup Semantics

The query: `catalog.find_for_output(artifact_type_id: ArtifactTypeId) -> Vec<CapabilityOption>`

Resolution order:

1. Check compiled catalog for capabilities producing `artifact_type_id`.
   Return all matches with `TrustLevel::Compiled`.

2. Check `capability_catalog::by_output::{artifact_type_id}` in sled.
   For each result, load `SynthesizedCapabilityRecord`. Filter to `status == Active`.
   Return matches with `TrustLevel::Synthesized`.

3. If no matches in either catalog: return empty. The planner treats this as a synthesis
   opportunity and may instantiate `CapabilitySynthesisTask`.

When both catalogs return results, the task compiler prefers `TrustLevel::Compiled`.
If only synthesized results exist, they are used. The trust level is recorded in the compiled
task record so repair semantics can distinguish the two cases.

## Schema Drift Detection

A synthesized capability's output schema is validated once at synthesis time. The environment
and the LLM conversion behavior may change after registration, causing the schema to drift.

Schema drift is detected through two paths:

**Invocation failure path**: when a synthesized capability invocation fails with
`SchemaHashMismatch` (the LLM conversion produced output that does not match the stored
schema hash), the runtime catalog marks the entry as `Stale { reason: SchemaHashMismatch }`.
The task network surfaces this to control. Control may trigger a new `CapabilitySynthesisTask`
for the same goal context.

**Proactive validation**: optionally, the catalog may schedule periodic re-validation of
synthesized capabilities by replaying the `contract_validation` step against a cached
`RawOutputSample`. If validation fails, the entry is marked stale proactively. This is a
background task, not on the critical execution path.

## Capability Supersession

When a new `CapabilitySynthesisTask` produces a capability that covers the same output type
as an existing stale entry, the new entry is registered and the old entry is marked
`Superseded { by: new_capability_type_id }`. The secondary index is updated to point to the
new entry.

Stale entries are retained for lineage. The `synthesis_task_run_id` and `registered_at_seq`
fields allow the event spine to reconstruct the full capability history for a given
`artifact_type_id`.

## Compiled Task Integration

When the task compiler resolves a synthesized capability, it records the capability type id
and trust level in the `BoundCapabilityInstance`:

```rust
struct BoundCapabilityInstance {
    capability_instance_id: CapabilityInstanceId,
    capability_type_id: CapabilityTypeId,
    trust_level: TrustLevel,
    // ... existing fields
}
```

A compiled task that includes one or more synthesized capability instances is tagged as
`contains_synthesized_capabilities: true`. This tag informs repair logic: if a synthesized
capability fails repeatedly, control may prefer re-synthesis over continued retry.

## Event Spine Integration

Catalog mutations emit spine events in the `task_graph` domain:

- `capability_synthesized { capability_type_id, artifact_types_produced, schema_hash, seq }`
- `capability_marked_stale { capability_type_id, reason, seq }`
- `capability_superseded { old_id, new_id, seq }`

These events enable temporal queries: "what capabilities were available at sequence S?" is
answerable by filtering spine events up to S and projecting the catalog state.

## Bootstrapping

On startup, the runtime catalog is loaded from sled. No synthesis is triggered at startup.
The compiled catalog is populated first. The runtime catalog extends it.

If the sled store is empty or missing, the runtime catalog starts empty and synthesized
capabilities are acquired on demand as planning needs them.

## Read With

- [External Process Capability](external_process_capability.md)
- [Synthesis Task](synthesis_task.md)
- [Synthesis Overview](README.md)
- [Multi-Domain Spine](../../../events/multi_domain_spine.md)
- [Event Manager Requirements](../../../events/event_manager_requirements.md)
