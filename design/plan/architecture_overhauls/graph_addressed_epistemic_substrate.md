# Graph-Addressed Epistemic Substrate

Date: 2026-08-19
Status: architectural discovery, implementation not authorized
Scope: make the world model a coherent continuously lowered epistemic network that Strategy can reason over

## Finding

Meld already performs continuous epistemic lowering:

```text
raw signal
→ sensory lowering
→ promoted Event
→ graph fact and current anchor
→ evidence
→ belief revision
→ planner projection
→ Strategy
```

Each stage reduces frequency and increases integrated epistemic value. This is the epistemic counterpart to Execution lowering a candidate into runnable work.

The current layers are graph-shaped but do not yet form one coherent navigable epistemic network.

## Current Fracture

`DomainObjectRef` is a stable cross-domain coordinate. `EventRelation` is a typed directed relation. Neither is an object body or a semantic property bag.

The traversal graph materializes only a subset of event domains. Evidence items, evidence assignments, belief revisions, belief views, and other higher constructs retain provenance internally but are not generally addressable nodes in the shared graph. Owner-specific relationship stores therefore contain topology that generic traversal and planner projection cannot see.

The planner projection further compresses this state into a small flat proposition set. It does not currently expose the containment, derivation, materiality, coverage, contradiction, and verification relations required by the README correctness example.

## Architectural Direction

Every durable promoted epistemic product should be addressable and related through one cross-domain graph language. Each owner still retains its typed authoritative record and invariant-enforcing store.

```text
DomainObjectRef
    shared address

domain-owned record
    typed content and lifecycle

domain-owned store
    invariants and authoritative mutation

event publication
    graph identity, relations, and provenance

graph projection
    shared traversal and planning surface
```

Frames are an existing example of this pattern. Context owns the Frame body while graph-visible events expose its identity and relations.

Strategy should receive an immutable, Goal-scoped, multi-resolution projection of the epistemic graph. It should begin with higher-value constructs and include selected lower-level nodes, relations, facts, and lineage when the Goal requires them. Strategy must not query a live mutable graph during search.

## Missing README Pressure

A missing README cannot be represented only as an absent file node. Meld needs a stable identity for the expected slot and bounded evidence that a complete workspace observation found no realizing file.

The relevant graph shape includes:

```text
Folder
→ requires
README slot

Source revision
→ derives
Semantic object
→ decomposes into
Source claim

Source claim
→ material to
README correctness

README revision
→ contains
README claim

Verification
→ covers
Source claim
→ against
README revision
```

This makes the entity that should exist addressable before its artifact exists. It does not eliminate the need for complete-scan and negative-evidence semantics.

## Unresolved Architecture

- which higher epistemic products become graph-addressable
- which relationships are direct edges and which require reified relation nodes
- how owner stores project graph views without creating a second authority
- how logical expected-object identity works for absent artifacts
- how a planner requests and freezes a deeper relation-rich projection
- how graph compaction, belief revision, and historical lineage interact
- how semantic object and claim candidates enter owner admission

The target is one graph language, not one universal graph store.
