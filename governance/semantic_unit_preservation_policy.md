# Semantic Unit Preservation Policy

Date: 2026-06-08
Status: active

## Intent

Prevent duplicated truth while canonical domain products move across boundaries.

The project favors carrying domain products intact through wrappers, queues, event envelopes, store records, task outcomes, and handoff records until a receiving domain accepts ownership and translates the product into its own model.

## Definitions

- Canonical product: a domain-owned value that represents one semantic fact, command, record, graph unit, language unit, or runtime result.
- Wrapper: a value that carries a canonical product with operational metadata.
- Operational metadata: routing, sequence, revision, timestamp, idempotency, retry, status, cursor, hash, diagnostic, or storage metadata added around a product.
- Receiver-owned model: a value produced after a boundary accepts ownership and translates input into an index, projection, command, query DTO, diagnostic, or compatibility shape.
- Duplicated truth: a wrapper contains a product while also re-declaring fields from that product as separate authoritative fields.

## Core Rule

A canonical product must travel intact through intermediate wrappers until a receiver-owned boundary explicitly translates it.

Wrappers may add operational metadata. Wrappers must not copy product fields into sibling fields that must stay consistent with the contained product.

## Required Shape

Prefer this shape:

```rust
struct StoredEvent {
    seq: u64,
    envelope: EventEnvelope,
}
```

Avoid this shape:

```rust
struct StoredEvent {
    seq: u64,
    event_id: String,
    event_type: String,
    data: serde_json::Value,
}
```

The second shape is only valid when `StoredEvent` is a receiver-owned model and no longer claims to transport `EventEnvelope` as the source of truth.

## Allowed Translation Boundaries

Translation is allowed when ownership changes clearly.

Valid receiver-owned translations include:

- durable indexes derived from stored records
- query projections and read models
- planner-facing views
- diagnostics and validation reports
- command-specific DTOs at adapter boundaries
- compatibility shims governed by [Compatibility Shim Policy](compatibility_shim_policy.md)
- storage keys and hash records that are not semantic product fields

## Disallowed Patterns

- A store record contains a domain record and repeats the domain record identity, revision, state, or hash as authoritative fields.
- An event payload contains a domain product and repeats product fields next to it.
- A queue item contains a request product and repeats target fields as separate truth.
- A task outcome contains artifact records and also carries sibling availability records that repeat artifact identity and schema truth.
- A public view contains a key product and repeats scope fields from the key.

## Crate Boundary Rule

Primitive crates should preserve primitive products without embedding usage-specific opinions.

Consumer crates own usage-specific translation and validation. For example, a shared language crate may preserve a `Term`, while execution may decide which term variants are valid for executable artifact routing.

## Migration Rule

When fixing duplicated truth in serialized records, event payloads, or public contracts:

- add characterization tests for the old shape before changing behavior
- add parity tests proving the new shape preserves the same semantic product
- use compatibility wrappers when old persisted data must remain readable
- call out intentional breaking changes under [Compatibility Policy](compatibility_policy.md)

## Review Checklist

Before approving a wrapper, event, queue item, store record, or handoff record, ask:

- What is the canonical product?
- Which domain owns that product?
- Is this value only carrying the product, or has ownership changed?
- Are any product fields repeated beside the product?
- If fields are repeated, are they operational metadata, storage keys, or a receiver-owned projection?
- Can the repeated fields drift from the product?
- Are persisted shape changes covered by characterization and parity tests?
