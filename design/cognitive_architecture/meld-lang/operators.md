# Operators

An Operator is a declarative executable occurrence inside a Composition. It names an exact Capability contract and supplies situated bindings, predicted preconditions, predicted effects, artifact ports, constraints, and cost estimates.

The Capability contract remains authoritative for executable inputs, outputs, effects, resources, and runner behavior. An Operator cannot override it.

Strategy uses Operators to construct complete Tasks. Execution resolves the exact Capability revision, validates its own runtime invariants, and compiles the Operator without re-evaluating Goal contribution.

The language owns Operator shape and pure structural validation. It does not query a Capability catalog or perform resolution.
