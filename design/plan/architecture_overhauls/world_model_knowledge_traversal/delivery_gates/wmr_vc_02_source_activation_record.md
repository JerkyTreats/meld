# WMR-VC-02 Source Activation Record

Date: 2026-08-28

Slice identifier: `WMR-VC-02`

Status: complete

Source baseline: `eabc9a75`

Authority: user authorization to implement `SI-02` after receiving the prepared `WMR-VC-02` boundary

Approved expansion: one Curation-owned durable tree family inside the existing world-model database

Frozen gate: [WMR-VC-02-DG revision 1](wmr_vc_02_standing_curation_settlement_gate.md)

Accepted candidate digest: `ba37bbe63575f8316373adaaaf318767514d00a103f3fe80440d0f17b3383cdb`

Implementation review: [passed](../reviews/wmr_vc_02_implementation_review_receipt.md)

Style Assurance: [satisfied](../reviews/wmr_vc_02_style_assurance_receipt.md)

Gate Acceptance: [accepted](wmr_vc_02_gate_acceptance_receipt.md)

## Accepted Maturity Envelope

Posture: `first slice`

Obligation floor: operational durability for accepted Curation operations, publication recovery, Graph projection, Belief cursor ordering, and incumbent Agent records

Direct product proof: a real `ProductRuntimeAssembly` consumes one complete workspace cut through standing Curation, durably persists one terminal result, publishes deterministic Curation Events, exposes Curation material through the existing Graph owner route, and commits one distinct Belief revision only under an installed mapping. Reopen and replay preserve all identities without repeating semantic work.

Hard limits:

- no new crate or dependency
- no new service, Event authority, Graph authority, standalone database, or second Belief consumer
- one Curation-owned durable tree family inside the existing world-model database
- one implementor

Tripwires:

- more than eighteen production source files changed
- more than two thousand eight hundred production lines added
- any direct Curation write into Graph or Belief storage
- any Agent Goal formation or satisfaction behavior change
- any planned Curation, Agent progression, product migration, or deadline-created successor operation

## Frozen Affected-Domain Set

```text
root runtime and theory
Events
Graph and Traversal
Agent
Curation
Belief
workspace source
```

Events, Graph and Traversal, Agent, Belief, and workspace remain unchanged participants unless direct evidence proves a narrow adapter correction is required.

## Active Product Trace

```text
exact Agent authority and installed standing rule
-> complete workspace TraversalCut and bounded result
-> durable Curation acceptance
-> terminal Curation result and publication intent
-> deterministic Event append
-> existing Graph owner projection
-> complete workspace plus Curation cut
-> existing configured Belief evidence ingestion
-> distinct immutable Belief revision
```

## Owned Write Scope

- new world-model `curation` domain for contracts, persistence, selection, bounded authorship, publication, actor behavior, and public query
- world-model crate exports
- root runtime storage, theory resolution, and concrete actor composition
- root world initialization only where exact Curation rule installation requires it
- one neutral standing rule and evidence mapping specimen
- focused unit, integration, property, state-machine or fuzz, restart, and regression proof
- program review, Style Assurance, and Gate Acceptance records

## Existing Seams To Reuse

- immutable `TraversalCut` and bounded occurrence-rich result
- `OwnerPublicationOperation` and `world_state.owner_publication.v1`
- `EventAppendCapability` and existing Event replay
- `GraphRuntime` and `TraversalQuery`
- `EvidenceIngestionActor` and `ConfiguredOutcomeMappingSet`
- `ProductRuntimeAssembly` bounded actor factory and reports
- public Agent identity, perspective, branch, activation, and rule lineage

## Forbidden Changes

- no edit to `meld-events`
- no new Graph or Belief writer
- no change to Agent Goal formation or satisfaction semantics
- no planned Curation intake or Plan product
- no Planner, Strategy, Execution, Meld Language, PDS, Startup, docs product, dependency-security product, or Workflow behavior
- no archived implementation recovery
- no public abstraction justified only by future consumers

## Selected Evidence

- direct root runtime assembly and bounded tick integration
- Curation contract, rejection, terminality, persistence, outbox, replay, and restart tests
- Graph visibility and incomplete-cut tests
- configured and unmapped Belief settlement tests
- operation identity property proof
- state-machine or fuzz viability where the durable transition contract warrants it
- unchanged Agent Goal and satisfaction regression proof
- full workspace tests, formatting, changed-crate lint, fuzz build when applicable, and diff validation

## Review And Commit Ownership

Implementor: primary agent

Implementation review owner: primary integrated implementation review lane

Style Assurance owner: primary Style Assurance lane

Gate Acceptance owner: primary integrated acceptance lane

Commit expectation: one runtime-focused delivery commit only after direct proof, implementation review, satisfied Style Assurance, and accepted Gate Acceptance

## Stop Conditions

Stop before any unapproved expansion, later-phase behavior, tripwire breach, or requirement to change the frozen gate. Report the exact evidence and request user disposition rather than absorbing the change.
