# Lang Crate

Date: 2026-05-18
Status: declarative
Scope: `meld-lang` crate boundary for the shared proposition language between world model and execution

## Intent

`meld-lang` owns the typed contract surface that both `meld-world-model` and `meld-execution` depend on for expressing goals, world state, operators, and compositions.

The crate exists because the language is owned by neither domain. If it lives in `meld-world-model`, execution imports a world model dependency. If it lives in `meld-execution`, the world model imports an execution dependency. Either coupling violates the architecture's boundary axiom.

The crate is pure. No IO, no async, no persistence, no side effects. Every public function takes values and returns values.

## Target Crate

`meld-lang`

## Owns

- term primitives: `Term`, `Literal`
- proposition primitives: `Proposition`, `Condition`
- effect primitives: `Effect`
- operator contracts: `Operator`, `Resolution`, `SlotConstraint`, `CapabilityRef`
- composition graph: `Composition`, `Step`, `StepKind`, `Edge`, `EdgeKind`
- goal specification: `Goal`, `GoalPriority`, `GoalSource`, `GoalLifecycle`
- method caching: `Method`, `Bindings`
- cost algebra: `CostEstimate`
- world state representation: `WorldState`
- proposition evaluation: `evaluate()`
- pattern unification: `unify()`
- binding substitution: `substitute()`
- structural validation: `validate()`
- effect application: `apply_effects()`

## Does Not Own

- capability catalog or capability resolution logic
- task compiler, task executor, or task runtime
- belief revision, evidence normalization, or comparator logic
- graph materialization or temporal truth maintenance
- causal inference or regime detection
- goal curation policy or normative judgment
- event append, replay, or sequencing
- provider execution, dispatch, or scheduling
- persistence, serialization format, or storage paths
- dimension semantics or artifact type semantics

## Code Routing

Crate location: `crates/meld-lang/src/`

## Dependency Rule

| From | To | Reason |
| --- | --- | --- |
| `meld-lang` | `meld-events` | `DomainObjectRef` and `EventRelation` as shared identity types |
| `meld-lang` | `serde`, `serde_json` | serialization of all public types |
| `meld-lang` | standard library only | pure computation, no external runtime dependencies |

## Forbidden Direction

`meld-lang` must not depend on `meld-world-model`, `meld-execution`, or root `meld`.

`meld-lang` must not perform IO, network calls, file access, or async operations.

`meld-lang` must not import provider, context, workflow, or workspace types.

## Consumer Dependencies

| From | To | What is consumed |
| --- | --- | --- |
| `meld-world-model` | `meld-lang` | `Term`, `Proposition`, `Effect`, `Goal`, `GoalSource`, `WorldState`, `Condition`, `Literal` for constructing goals and projecting world state |
| `meld-execution` | `meld-lang` | all public types; `evaluate()`, `unify()`, `substitute()`, `validate()`, `apply_effects()` for the planning loop |

## Public Contract Shape

All types derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.

All evaluation functions are pure: `fn(&input) -> output`. No `&mut self`. No `Result` types wrapping IO errors — only structural errors (validation failures, grounding errors, unbound variables).

`WorldState` is immutable. `apply_effects()` returns a new `WorldState`.

`Bindings` is immutable. `merge()` returns a new `Bindings` or `None` on conflict.

## Purity Enforcement

The crate should not include `tokio`, `async-trait`, `std::fs`, `std::net`, or any IO-capable dependency.

Tests should be pure value-in value-out assertions with no setup, no mocks, no fixtures, and no test infrastructure beyond `#[test]`.
