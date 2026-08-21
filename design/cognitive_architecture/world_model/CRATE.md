# meld-world-model

Scope: world-model epistemics, reasoning, curation, Strategy Plan construction, and Agent reconciliation

## Ownership

This crate owns graph admission and traversal, belief settlement, causal and regime knowledge, immutable planner cuts, bounded Epistemic Operations, heterogeneous Strategy Plans, and durable Agent reconciliation.

It does not own Event storage, product-domain truth, Capability execution, Task Network state, PDS package administration, or root composition.

## Public Domain Contracts

The crate exposes explicit contracts for owner-issued graph publication, traversal queries, belief admission and reads, planner cut materialization, Curation requests and results, Strategy Plan construction and verification, and Agent lifecycle and decisions.

Internal domains communicate through these contracts. No domain reaches into another domain's storage or private modules.

## Authority

Product owners retain authority over the semantic products they publish. Belief owns epistemic admission. Curation owns graph authorship mechanics. Strategy owns Plan construction correctness. Agent owns authorization and progression. The crate never grants Execution authority or interprets Task Network state.
