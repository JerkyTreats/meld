# Graph And Traversal Specification

## Core Products

`GraphObjectPublication` binds an opaque object address to owner, product type, presence state, scope, provenance, and revision.

`RelationOccurrencePublication` binds one stable occurrence identity to source, relation type, target, qualifications, scope, provenance, currentness, and owner revision.

`TraversalCut` names the immutable source boundary across participating owners.

`TraversalQuery` names roots, relation policy, direction, scope, currentness policy, cut, and resource bounds.

`TraversalResult` returns reached object publications, relation occurrences, paths, frontier, truncation state, and exact cut identity.

## Identity Rules

Object address identity is owner-issued. Publication identity additionally includes owner revision and scope. Relation occurrence identity is independent from endpoint equality. Traversal result identity derives from the normalized query and exact cut.

## Presence And Currentness

Presence is an explicit state such as present, expected, absent, withdrawn, or archived. Currentness is evaluated under an owner policy and scope. No generic `current_only` flag may erase the policy or scope used to decide currentness.

## Federation

Each owner supplies a traversal projection or publication reducer that conforms to the shared query and result protocol. Federation joins results by opaque address while preserving owner, occurrence, scope, and revision.

A federated cut contains one exact revision receipt per participating owner. Missing owner receipts make the cut incomplete rather than silently current.

## Curation Path

Curation emits typed publications through Events. Graph admission validates owner route, schema, occurrence identity, scope, and idempotency, then advances the graph projection. A Curation result may report unchanged when the requested publication already exists at the required revision.

## Planner Path

Planner issues bounded traversal queries, retains occurrence-rich results, and freezes them with belief, causal, regime, Capability, directive, and policy revisions. Strategy consumes only that immutable planner cut.
