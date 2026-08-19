# Native Capability And Task Substrate

Date: 2026-08-19
Status: architectural discovery, implementation not authorized
Scope: restore one action language across Capability publication, Strategy construction, Agent authorization, Task compilation, and Task Network execution

## Finding

Meld has a native execution substrate, but current Strategy and PDS paths introduced parallel representations around it.

The native shape is:

```text
runtime Capability publication
→ complete active Capability catalog
→ Strategy candidate Composition
→ Agent authorization
→ Execution Task compilation
→ Task Network execution
```

A Capability is the universal atomic runtime primitive. It may wrap functionality anywhere in Meld or bind an external implementation. A Task is a composite graph of bound Capability instances. A Task Network is a graph of Tasks.

PDS does not own a Capability catalog, action-class catalog, or Capability-to-theory mapping. A PDS distribution may ship a normal Capability contributor or physical executable binding, but that contribution enters through the same runtime registration path as every other Capability.

## Current Fracture

The implementation currently carries several overlapping action descriptions:

- the runtime execution Capability contract
- `StrategyCapability` as a Strategy-side shadow contract
- PDS-authored exact Operators
- episode-local `Operator` and `Resolution` fields that repeat Capability fields
- Execution Task definitions and Task Network nodes

Runtime validation makes those descriptions agree after the fact. That validation is evidence of duplicate authority rather than a clean boundary.

The current lowering path also creates one Task for each Operator even though Task already supports a graph of Capability instances. Recursive Goal steps, conditional edges, intentional Task regions, and full HTN lineage are not yet one lossless lowering path.

## Architectural Direction

Strategy and Execution must speak one graph algebra while retaining separate authority.

- runtime contribution and activation produce the complete exact Capability catalog
- Strategy constructs a ground Composition over exact Capability identities
- an Operator, if retained, represents one situated use inside that Composition and never another Capability contract
- Agent authorization preserves the exact Composition
- Execution validates feasibility and lowers the authorized graph without semantic replanning
- Task compilation owns executable closure and Task identity
- Task Network owns accepted operational state and dispatch

The same semantic unit must cross these boundaries intact. Receiver-specific views may expose only needed fields, but they may not become independent sources of truth.

## Unresolved Architecture

- which existing crate owns the canonical cross-domain Capability contract
- which contract fields are universal operational truth and which are situated Strategy estimates
- whether `Operator` remains a distinct episode-local type or becomes a bound Capability occurrence
- how Composition represents intentional composite Task regions
- how recursive Goals and Methods preserve HTN lineage through lowering
- how active catalog scope, physical binding, authority, and implementation availability remain distinct
- how observed cost and risk enter planning without becoming PDS theory

These questions must be settled together. Removing the PDS catalog alone would leave the deeper duplication unresolved.
