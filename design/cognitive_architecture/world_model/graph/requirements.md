# Graph And Traversal Requirements

## Identity

- Preserve owner-issued object identity through opaque `DomainObjectRef` values.
- Use an injective and versioned durable key codec.
- Give every relation occurrence its own stable identity.
- Preserve qualified semantic units without lossy flattening.

## Scope

- Represent existence separately from addressability.
- Preserve valid time, transaction time, branch, perspective, owner authority, currentness, and revision.
- Bind every traversal result to one immutable cut.
- Prevent federation from silently merging incompatible scopes.

## Publication

- Admit graph material only through owner-declared publication contracts.
- Make replay idempotent and deterministic.
- Preserve withdrawal and supersession as durable lifecycle facts.
- Append Curation publications through Events before shared graph admission.

## Traversal

- Support neighbor reads and bounded multi-hop walks.
- Filter by relation family, direction, scope, currentness policy, and owner.
- Return exact objects, relation occurrences, paths, provenance, and cut identity.
- Enforce resource bounds without truncation becoming a false semantic conclusion.

## Authority

- Treat provenance as explanation rather than belief authority.
- Treat graph reachability as discovery rather than evidence.
- Keep product meaning with the publishing owner.
- Keep authorship with Curation and evidence settlement with Belief.
