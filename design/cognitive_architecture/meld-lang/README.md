# Meld Language

`meld-lang` provides the permissive shared vocabulary used across Meld. It defines stable nouns and verbs such as Goal, Task, Capability, Composition, artifact, predicate, relation, world-state value, and Method.

The language does not enforce another domain's grammar. Strategy owns Plan correctness. Curation owns Epistemic Operation correctness. Execution owns Task compilation and Task Network safety. Product domains own their semantic products.

## Goal

A Goal names a desired state, authority, priority, lineage, and lifecycle. It does not prescribe whether satisfaction requires epistemic authorship, executable work, observation, or a combination.

## Task

A Task is a complete executable composition of bound Capability invocations with explicit artifact flow and dependencies. It is suitable for Execution intake. It is not a Strategy Plan.

## Capability

A Capability is the atomic executable contract known to Execution. It declares inputs, outputs, effects, constraints, and runner identity. Epistemic Curation is not represented as a Capability merely to enter the execution path.

## Method

A Method is reusable declarative decomposition knowledge. Domains may use Methods as candidate generators. The language does not decide when a Method is valid, authorize its result, or make hierarchical planning an Execution concern.

## Ownership Boundary

World-model `StrategyPlan`, `EpistemicOperation`, planner cuts, belief revisions, and graph publications remain owned by `meld-world-model`. Execution Goal Set admissions and Task Network records remain owned by `meld-execution`. They may embed language values without moving their domain grammar into this crate.

## Detailed Vocabulary

- [Primitives](primitives.md)
- [Goals And Methods](goals_and_methods.md)
- [Operators](operators.md)
- [Compositions](compositions.md)
- [World State](world_state.md)
- [Requirements](requirements.md)
