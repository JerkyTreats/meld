# Language Primitives

A `Term` is a variable, literal, or opaque object reference. An object reference uses `DomainObjectRef` as an address atom and carries no implied existence or truth.

A `Proposition` is a predicate with ordered ground or variable terms. A `Condition` composes propositions through explicit logical operators. An `Effect` declares a predicted proposition change for reasoning.

A `Literal` preserves typed scalar identity. Domain-specific meaning stays in owner-issued schemas rather than generic string parsing.

Pure operations include grounding, unification, substitution, normalization, structural validation, proposition evaluation, and predicted effect application.
