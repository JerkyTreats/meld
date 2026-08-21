# Epistemic Curation

Curation is the world-model domain for bounded authorship of shared epistemic structure. It creates or updates expected entities, claims, qualifications, and relation occurrences under Agent authority.

Curation is separate from Traversal. Traversal reads the knowledge graph. Curation authors graph material.

Curation is separate from Strategy. Strategy determines which epistemic result is needed and places an Epistemic Operation in a Plan. Curation performs that operation.

Curation is separate from Execution. It does not invoke Capabilities or enter the Task Network.

## Bounded Epistemic Operation

An `EpistemicOperation` names exact targets, requested assertions or relations, owner and Agent authority, perspective scope, preconditions, idempotency, expected result, and publication policy.

The operation is bounded when the consumer can determine its complete write set or an owner-approved query bound before committing effects.

## Publication

Successful shared authorship is appended through Events. The Event envelope is a neutral carrier. The Curation payload retains semantic meaning, authority, provenance, perspective, and currentness.

Graph admission consumes the publication through its own owner contract. Belief may later admit the result as evidence. Neither transition is implied by Event append alone.

## Examples

Curation may author that a README is required for a folder, connect source claims to required README claims, connect a realized README to those requirements, or withdraw a stale relation occurrence. It cannot claim that the file exists unless an authorized semantic owner supplies that fact.
