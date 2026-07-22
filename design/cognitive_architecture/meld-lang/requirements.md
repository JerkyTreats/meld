# Lang Requirements

Date: 2026-05-18
Status: active
Scope: functional, structural, and nonfunctional requirements for `meld-lang`

## Thesis

`meld-lang` must provide a minimal, pure, runtime-composable language for expressing propositions, goals, operators, and compositions. The first slice must prove the full evaluation loop: construct a goal from terms, evaluate it against a world state, match a method, substitute bindings, validate the composition, and apply effects to produce a new world state. Later slices extend the grammar and add optimization without changing the evaluation boundary.

## Functional Requirements

### Term Primitives

- Define `Term` as the atomic unit of reference.
- Support domain object references via `DomainObjectRef` from `meld-events`.
- Support belief dimension references as string-identified terms.
- Support artifact type references as string-identified terms.
- Support literal values: boolean, text, number (f64), and duration.
- Support unbound variables with string names for pattern matching.
- Support derived references that point to a step output and field path for runtime resolution.
- All term variants must be serializable and deserializable.
- New objects, dimensions, artifact types, and literal values must not require recompilation. They are vocabulary, not grammar.

### Proposition Primitives

- Define `Proposition` as a typed statement about the world composed from terms.
- Support `Holds`: a belief dimension of a subject satisfies a condition.
- Support `Exists`: an artifact of a given type has been produced for a given scope.
- Support `Accessible`: a scope is reachable for observation or action.
- Support `Related`: a relationship holds between two domain objects.
- Support boolean composition: `All`, `Any`, `Not`.
- Propositions with all concrete terms (no variables) are ground. Propositions with variable terms are patterns.
- The same `Proposition` type must be usable as: world state assertion, goal target, method trigger, operator precondition, operator effect argument, and sub-goal in a decomposition.

### Condition Primitives

- Define `Condition` as a comparison over a belief dimension's value.
- Support threshold comparisons: `Above(Term)`, `Below(Term)`.
- Support equality: `Equals(Term)`, `In(Vec<Term>)`.
- Support temporal range: `Within(Term)`, `Exceeds(Term)`.
- Support observation status: `Present` (any value exists), `Absent` (no value observed).
- New condition variants are grammar changes and require deliberate extension.

### Effect Primitives

- Define `Effect` as a state change to the proposition space.
- Support `Assert`: make a proposition true in the world state.
- Support `Retract`: remove a proposition from the world state.
- Support `Update`: modify a belief dimension's value for a subject.
- Effects compose: a list of effects applied in order produces a deterministic new world state.

### Operators

- Define `Operator` as a runtime-constructed contract with: preconditions (propositions), effects, cost estimate, and resolution hint.
- Operators are NOT compile-time enum variants. They are defined by their typed boundary.
- An operator's identity within a composition is a runtime-assigned string.
- `Resolution` must carry enough information for a capability catalog to find a matching capability without the operator naming a specific capability: required input artifact types, required output artifact types, scope kind, and freeform tags.
- `Resolution` must support an optional `specific` field for cases where the exact capability is known, as an escape hatch.
- Operators must carry `CostEstimate` with time, money, and provider call dimensions.

### Compositions

- Define `Composition` as a directed graph of steps with typed edges.
- Steps are either Operators or subgoals. A Strategy proposal must resolve each subgoal to an exact child Composition or an exact reusable Method revision, bindings, and expanded hash. Agent judgment authorizes the complete resolved candidate.
- Edges carry ordering, dataflow, conditional, or evidence-admission semantics.
- An evidence-admission edge must identify the prospective artifact contract, admission authority, and expected content identity. It is satisfied only by a matching authoritative admitted verdict.
- Compositions are constructed at runtime. Construction may use authored assets, model-backed capabilities, deterministic programs, or Agent-authorized Strategy. Novel semantic construction belongs to Strategy.
- All construction paths produce the same `Composition` type.

### Goals

- Define `Goal` as a proposition target with operational metadata.
- Goal target is a `Proposition`. Execution evaluates it mechanically.
- Goals carry: goal identity, agent identity, priority (urgency and optional cost ceiling), source (belief divergence, user directive, maintenance invariant, or decomposition from parent), and lifecycle (proposed, active, suspended, satisfied, abandoned).
- Goal source carries enough provenance for the world model agent's justification to be auditable without execution interpreting it.
- Goals are constructed by the world model agent in the proposition language. Execution does not construct goals.

### Methods

- Define `Method` as a named, reusable composition with a trigger pattern.
- The trigger is a `Proposition` with `Term::Variable` in bindable positions.
- Methods carry: preconditions (propositions that must hold in world state for the method to apply), the composition (the decomposition recipe), net effects (the combined state change), cost estimate, and preference ordering.
- Methods are optional. The system must function without Methods because Strategy may construct episode-specific Compositions directly.
- Methods may be loaded from serialized files at runtime. New methods do not require recompilation.

### World State

- Define `WorldState` as a set of ground propositions.
- Construction must validate that all propositions are ground (no variable terms). Reject with error if any variable term is present.
- World state is immutable. All operations return new values.
- Provide `satisfies()`: does this world state satisfy a given proposition? Handles `All`/`Any`/`Not` composition. Variable terms in the query are existentially quantified.
- Provide `gap()`: returns the sub-propositions of a query that are not satisfied.
- Provide `apply()`: applies a list of effects to produce a new world state.
- Provide `propositions()`: iterate over all current ground propositions.

### Evaluation

- Define `evaluate()` as a pure function from world state and proposition to a three-valued result: `Satisfied`, `Unsatisfied` (with gap), or `Indeterminate` (dimension or object not present in state).
- `Indeterminate` is distinct from `Unsatisfied`. It means the world model has not asserted anything about the referenced dimension or object. Strategy may propose authorized observation work. Execution reacts only through an authorized candidate.
- Evaluation of `All` requires all children satisfied. Evaluation of `Any` requires at least one child satisfied. Evaluation of `Not` inverts the child result. `Indeterminate` propagates through composition per three-valued logic.

### Unification

- Define `unify()` as a pure function from a pattern proposition and a concrete proposition to an optional set of bindings.
- Variable terms in the pattern bind to the corresponding term in the concrete proposition.
- Unification fails (returns `None`) if the proposition shapes differ or if a variable would need to bind to two different terms.
- `Bindings` is a map from variable name to bound term.
- `Bindings` supports `merge()`: combine two binding sets, returning `None` on conflict.

### Substitution

- Define `substitute()` as a pure function from a composition and bindings to a new composition with all variable terms replaced by their bound values.
- Return error if any variable in the composition is not present in the bindings.
- Substitution applies to all operators' preconditions, effects, resolution hints, and to all sub-goal propositions.

### Structural Validation

- Define `validate()` as a pure function from a composition to a validation result.
- Check: all edge endpoints reference existing step identities.
- Check: no duplicate step identities.
- Check: the graph is acyclic (a DAG).
- Check: data flow edges reference artifact types present in the upstream operator's effects or resolution outputs.
- Check: conditional edges reference steps that produce evaluable output.
- Warn: steps with no incoming edges other than the first step.
- Warn: operator effects that do not contribute to any downstream precondition or goal.
- Validation does NOT resolve operators to capabilities. That is the consumer's responsibility.
- Validation does not check whether effects achieve a Goal. Strategy owns semantic achievement proof and Execution validates the authorized proof mechanically.

### Cost Algebra

- Define `CostEstimate` with time (milliseconds), money (microdollars), and provider calls.
- Support addition: sum two cost estimates dimension-wise.
- Support comparison against a cost ceiling: does a cost estimate exceed any dimension of a ceiling?
- Support aggregation over a composition: sum all operator costs.

## Structural Requirements

### Runtime Composition

- All types that represent goals, operators, methods, and compositions must be constructable at runtime from primitive terms.
- No compile-time enum variant shall represent a specific goal, a specific method, a specific operator, or a specific action family.
- The compiled grammar (proposition shapes, condition shapes, effect shapes, edge kinds) is finite and stable. The vocabulary (which terms fill those shapes) is open and runtime.
- Serialized methods and compositions loaded from files at runtime must produce the same types as programmatically constructed values.

### Type Purity

- All public types derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.
- All public functions are pure: deterministic, no side effects, no mutation of inputs.
- `WorldState` and `Bindings` are immutable. Operations return new values.
- Error types are structural (validation failures, grounding errors, unbound variables), not IO errors.

### Single Type Across Roles

- `Proposition` is the same type whether used as: world state assertion, goal target, method trigger pattern, operator precondition, effect argument, or sub-goal in decomposition.
- `Term` is the same type whether referencing an object in world state or a variable in a pattern.
- This single-type property is what makes the language a shared substrate rather than a collection of parallel type hierarchies.

## Nonfunctional Requirements

### Determinism

- Same world state, same proposition, same evaluation. No randomness.
- Same pattern, same concrete proposition, same unification result. No ordering dependence.
- Same composition, same bindings, same substitution result.
- Same composition, same validation result.
- Same effects, same world state, same resulting world state.

### Testability

- Every public function testable with pure value-in value-out assertions.
- No mocks, no fixtures, no setup, no test infrastructure beyond `#[test]`.
- Test a full loop — construct goal, evaluate, unify against method, substitute, validate, apply effects — in a single test function with no external dependencies.

### Serialization Stability

- All types must round-trip through `serde_json` without loss.
- Serialized methods and compositions loaded from files are the mechanism for runtime-extensible method libraries. Serialization format stability is a compatibility concern.

### Performance

- Proposition evaluation against a world state with hundreds of ground propositions must be fast enough for the planning loop to evaluate multiple goals per cycle.
- Unification, substitution, and validation are per-goal-per-method operations. They must be fast enough to scan a method library of tens to hundreds of methods per planning cycle.
- No performance requirement on world state sizes beyond hundreds of propositions in the first slice.

## First Slice Requirements

- Implement `Term` with all variants: `Object`, `Dimension`, `ArtifactType`, `Literal`, `Variable`, `Derived`.
- Implement `Literal` with: `Bool`, `Text`, `Number`, `Duration`.
- Implement `Proposition` with: `Holds`, `Exists`, `Accessible`, `Related`, `All`, `Any`, `Not`.
- Implement `Condition` with: `Above`, `Below`, `Equals`, `In`, `Within`, `Exceeds`, `Present`, `Absent`.
- Implement `Effect` with: `Assert`, `Retract`, `Update`.
- Implement `Operator` with preconditions, effects, cost, and resolution.
- Implement `Resolution` with input/output slot constraints, scope kind, tags, and optional specific capability ref.
- Implement `Composition` with steps and edges.
- Implement `Goal` with target proposition, priority, source, and lifecycle.
- Implement `Method` with trigger, preconditions, composition, net effects, cost, and preference.
- Implement `WorldState` with ground proposition set, `satisfies()`, `gap()`, `apply()`.
- Implement `evaluate()` with three-valued result.
- Implement `unify()` with variable binding.
- Implement `substitute()` for compositions.
- Implement `validate()` for structural soundness.
- Implement `CostEstimate` with addition, ceiling comparison, and composition aggregation.
- Test full evaluation loop in a single test: goal construction, world state evaluation, method unification, binding substitution, composition validation, effect application, and re-evaluation showing goal satisfaction.

## Deferred Requirements

- Numeric fluent tracking across world state transitions.
- Temporal proposition variants (valid-time-scoped assertions).
- Composition optimization (dead step elimination, parallel opportunity detection).
- Method learning from execution traces.
- Incremental world state (diff-based update rather than full replacement).
- Proposition indexing for large world states (hundreds of thousands of propositions).
- Composition merging (combining two compositions that share steps).
- Multi-agent perspective scoping within the language (currently carried by `Goal.agent_id`).

## Read With

- [Lang Overview](README.md)
- [Lang Crate](CRATE.md)
- [Execution Domain](../execution/README.md)
- [Planning Pipeline](../execution/planning/planning_pipeline.md)
- [World Model Domain](../world_model/README.md)
- [World Model Planner](../world_model/planner/README.md)
- [World Model Belief](../world_model/belief/README.md)
