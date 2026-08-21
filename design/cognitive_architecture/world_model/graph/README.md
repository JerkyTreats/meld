# Graph And Traversal

Graph is the world-model substrate for addressable epistemic products and owner-issued relation occurrences. Traversal is the common read capability over that substrate.

The substrate is a federated traversal language, not a universal store or ontology. A domain may retain its own storage and publish only the products needed for cross-domain reasoning.

## Addressing

`DomainObjectRef` is the canonical cross-domain address atom. It identifies an owner-issued object but does not prove existence, truth, currentness, branch presence, temporal validity, perspective, or authority.

Address encoding is opaque, injective, versioned, and safe for durable indexing. Consumers never derive semantic meaning by parsing an address key.

## Publication

An owner publishes graph material as typed objects and relation occurrences. Every publication preserves owner, object identity, occurrence identity, source product, provenance, valid time, transaction time, currentness, branch scope, perspective scope where relevant, and owner revision.

Graph mention does not imply presence. Presence, absence, expectation, withdrawal, and supersession are explicit owner-shaped facts.

## Traversal

Traversal accepts roots, allowed relation families, direction, temporal and branch scope, perspective, currentness policy, cut identity, and resource bounds.

Results preserve reached objects and exact relation occurrences. They retain path lineage and the source cut. Equal endpoints connected by different occurrences remain distinguishable.

Traversal may cross federated owners through registered publication contracts. It never promotes one branch, perspective, or owner assertion into global authority.

## Eligibility

Not every epistemic record becomes a graph node. A durable product opts into traversal when cross-domain discovery, explanation, dependency reasoning, or Plan construction requires stable addressing and relation semantics.

Qualified semantic units remain intact. A relationship with material qualifications is represented as a typed owner record or reified object rather than flattened into a bare edge.

## Boundaries

Traversal reads. Curation authors. Belief admits evidence. Planner freezes cuts. Product owners define meaning. Graph storage performs none of those foreign functions.

- [Requirements](requirements.md)
- [Specification](spec.md)
