# meld-execution

Scope: executable intake, coherence, lowering, dispatch, and durable Task Network progression

## Ownership

This crate owns Capability runtime contracts, the Goal Set of admitted executable obligations, Execution Planning, Task compilation, the unified Task Network, dispatch, retry policy, resource coordination, and execution outcomes.

It does not own directives, beliefs, Strategy Plans, Epistemic Operations, semantic Goal satisfaction, product-domain truth, or PDS lifecycle.

## Public Contracts

The crate exposes Task admission lifecycle, Capability registration, planning ticks, Task Network queries and mutations, dispatch ports, and outcome publication.

A Task admission carries Goal attribution and producer authority. It does not carry a world-model projection or require the crate to query the world model.

## Safety

Execution protects only the invariants it owns. These include exact Capability availability, valid bindings, idempotent intake, resource safety, effect concurrency, durable state transitions, and legal Task Network progression.
