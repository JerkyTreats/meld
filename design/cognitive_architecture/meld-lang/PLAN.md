# Meld Lang Language Contract

Date: 2026-06-02
Status: active
Scope: pure shared language between world model and execution

## Purpose

`meld-lang` defines the typed propositions and pure operations shared by world model projection and execution planning. It expresses what holds, what should hold, how actions may change projected state, and how methods lower goals into compositions.

The crate is a language substrate. It does not own storage, runtime coordination, planning search, task dispatch, belief revision, or event publication.

## Authority

`meld-lang` owns:

- term and literal grammar
- propositions and conditions
- effects and world state evaluation
- operators, resolution, and cost estimates
- composition graphs and structural validation
- goals and lifecycle value types
- methods, bindings, unification, and substitution
- deterministic serialization for public language values

The world model owns projection into ground world state. Execution owns planning policy, method selection, task network commands, and outcome handling.

## Contract Surface

| Concern | Public values and operations | Invariant |
| --- | --- | --- |
| terms | `Term`, `Literal`, `DomainObjectRef` | one term grammar is used in facts, patterns, goals, and effects |
| propositions | `Proposition`, `Condition` | proposition structure is independent of domain-specific action families |
| state | `WorldState`, `EvalResult`, `evaluate`, `WorldState::gap`, `WorldState::query` | evaluation is pure and three-valued |
| effects | `Effect`, `WorldState::apply` | applying effects returns a new state |
| operators | `Operator`, `Resolution`, `CostEstimate` | operators declare preconditions, effects, execution resolution, and cost |
| compositions | `Composition`, `Step`, `Edge`, `validate` | plans are dependency graphs with explicit structural validity |
| goals | `Goal`, `GoalSource`, `GoalPriority`, `GoalLifecycle` | desired state uses the same proposition grammar as world state |
| methods | `Method`, `Bindings`, `unify`, `substitute` | method matching and lowering are deterministic pure operations |

## Terms And Propositions

`Term` represents concrete objects, typed literals, dimensions, and pattern variables. Ground world state contains no unbound variables. Method triggers and templates may contain variables that unification binds before substitution.

`Proposition` is the common statement type for world state facts, goal targets, method triggers, preconditions, effects, and sub-goals. No execution or world model layer defines a parallel proposition grammar.

`Condition` expresses comparisons and temporal or set relationships without embedding planner policy. Domain-specific policy enters through typed values and projected facts, not new hard-coded proposition variants.

## World State Evaluation

`WorldState` is a set of ground propositions projected by the world model for execution consumption.

Evaluation has three outcomes:

- satisfied when the projected state proves the proposition
- unsatisfied when projected values contradict or fail the condition
- indeterminate when required knowledge is absent

`WorldState::gap` identifies unsatisfied and indeterminate sub-propositions. `WorldState::query` matches proposition patterns and returns variable bindings.

The same world state and proposition always produce the same evaluation result.

## Effects And Operators

`Effect` describes assertion, retraction, or update against projected state. Applying an effect produces a new `WorldState` and leaves the input unchanged.

An `Operator` combines:

- preconditions evaluated against world state
- effects used for forward projection
- a resolution that identifies executable work
- a cost estimate used by execution policy

Forward projection predicts what an operator would change. It does not assert that a real-world outcome occurred. Only factual outcome events can support world model revision.

## Compositions

`Composition` is a graph of operator and sub-goal steps connected by typed edges. The graph expresses ordering, conditional dependency, and data flow.

Structural validation requires:

- unique step identities
- valid edge endpoints
- acyclic dependency structure
- data-flow artifact types produced by their source operators
- conditional guards sourced from operator steps
- ground values at every variable-bearing position after substitution

Multiple roots and leaves are valid. Disconnected steps and unused effects are warnings rather than structural errors.

Composition values remain declarative. Execution owns recursive lowering, task identity, dispatch fencing, repair, and continuation.

## Goals

`Goal` carries a stable identity, agent identity, proposition target, source, priority, and lifecycle. Goal targets use the same `Proposition` type evaluated against `WorldState`.

The world model agent constructs and curates goals. Execution persists accepted lifecycle commands and plans against active goals. `meld-lang` defines the values and pure checks without owning those mutations.

## Methods And Bindings

`Method` carries a trigger pattern, preconditions, composition template, net effects, cost, and preference.

Method matching follows one semantic flow:

1. unify the trigger pattern with a concrete goal
2. evaluate substituted preconditions against world state
3. compare net effects with the goal target
4. substitute bindings through the composition
5. validate the bound composition

`Bindings` is immutable. Binding and merge operations return new values. Conflicting bindings fail without modifying either input.

Unification is asymmetric. The method trigger is the pattern and the goal proposition is concrete. Shape mismatch and conflicting bindings return no match.

Substitution walks every variable-bearing position. An unbound variable produces an explicit error that identifies the missing variable.

## Purity And Dependency Invariants

- public language operations perform no IO
- public language operations do not depend on async runtimes
- values do not hold storage handles, clocks, network clients, or mutable runtime state
- `meld-lang` does not depend on `meld-world-model`, `meld-execution`, or root `meld`
- shared object identity may come from the event contract crate
- serialization support may depend on `serde` and `serde_json`

## Determinism And Immutability

- equal world state and proposition inputs produce equal evaluation results
- equal pattern and concrete inputs produce equal bindings
- equal composition and binding inputs produce equal substitution results
- equal effects and world state inputs produce equal new state
- state application leaves the input world state unchanged
- binding and merge operations leave their inputs unchanged

## Serialization And Evolution

All public language values have deterministic JSON representations. Serialized methods and compositions preserve semantic identity across load and replay.

Grammar evolution follows these rules:

- new fields use explicit defaults when old serialized values remain valid
- enum changes preserve unambiguous decoding or require a versioned contract
- field ordering remains deterministic for canonical serialization
- numeric literals reject or normalize values such as `NaN` that violate stable equality
- object identity retains its owning domain, kind, and id without lossy string flattening

## Error Boundaries

Pure operations return typed errors or explicit non-match values.

- malformed structures return validation errors
- unbound variables return substitution errors
- incompatible binding merges return conflict results
- unsupported or ambiguous serialized grammar returns decoding errors
- indeterminate world knowledge remains an evaluation result, not an operational error

## Cross-Domain Use

The world model planner produces `WorldState` values and preserves projection provenance outside the pure state value. Execution evaluates goals, matches methods, validates compositions, and applies effects for forward projection.

Neither consumer may attach hidden mutable state or domain authority to language values. Runtime identity, event cursors, leases, storage transactions, and outcome publication travel through their owning contracts.

## Read With

- [Lang Domain](README.md)
- [Lang Crate](CRATE.md)
- [Lang Requirements](requirements.md)
- [Primitives](primitives.md)
- [Operators And Resolution](operators.md)
- [Compositions](compositions.md)
- [Goals And Methods](goals_and_methods.md)
- [World State And Evaluation](world_state.md)
- [Execution Domain](../execution/README.md)
- [Execution Integration Contracts](../execution/GAPS.md)
- [Planning Pipeline](../execution/planning/planning_pipeline.md)
- [World Model Planner](../world_model/planner/README.md)
- [World Model Agent](../world_model/agent/README.md)
