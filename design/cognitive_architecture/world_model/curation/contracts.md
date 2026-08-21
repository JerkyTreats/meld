# Curation Contracts

A Curation request carries operation identity, Agent authorization, Goal and Plan lineage, perspective, target bounds, graph preconditions, requested publications, and an idempotency key.

Curation validates request shape, Agent authority, idempotency, and graph-owned state invariants. It does not re-run Strategy reasoning or evaluate Goal importance.

A result records applied, unchanged, rejected, or conflicted status with exact publication identities and durable Event positions. An unchanged result is successful epistemic closure when the requested graph state already holds.
