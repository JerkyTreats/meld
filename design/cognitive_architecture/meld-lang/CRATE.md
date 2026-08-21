# meld-lang

Scope: pure shared vocabulary for Goals, propositions, executable compositions, and evaluation

The crate owns stable value types and pure structural operations needed by more than one domain. It has no IO, persistence, async runtime, provider integration, domain storage, or authority.

The crate does not own Strategy Plans, Epistemic Operations, graph publications, belief revisions, Execution admissions, or Task Network records. Those aggregates remain with their semantic domains.

Public functions are deterministic value transformations. Structural validation proves type and graph shape only. It never proves Goal relevance, evidence authority, Plan correctness, Capability availability, or executable safety.
