# World Model Planner

Planner materializes immutable, relation-rich world-model cuts for Agent and Strategy reasoning. It is a projection boundary, not a plan constructor or execution scheduler.

## Planner Cut

A `PlannerCut` binds exact graph object and relation occurrences, belief revisions, causal mechanisms, regime state, directive context, Capability catalog revision, temporal scope, branch scope, perspective, authority, and projection policy.

The cut preserves semantic qualification and provenance. Equal source revisions and equal policy produce equal cut identity.

## Traversal

Planner uses the common Traversal capability to discover relevant multi-hop context from Goal and directive anchors. Resource bounds, relation policy, scope, and exclusion rules are explicit inputs. Traversal returns occurrences rather than flattened adjacency.

## Boundary

Planner does not select Tasks, create Epistemic Operations, authorize work, settle belief, dispatch Capabilities, or interpret Task Network state. Strategy consumes the frozen cut and constructs a heterogeneous Plan. Agent decides whether and how that Plan progresses.

- [Specification](spec.md)
