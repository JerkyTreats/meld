# Typed Loop Integration

Date: 2026-05-21
Status: active
Scope: first cross-domain contract chain for the cognitive architecture

## Purpose

The typed loop proves the shared formal contract between world model and execution before runtime dispatch exists.

It demonstrates that the system can express a current world state, a desired goal, a reusable method, a concrete composition, a state-changing effect, and a final satisfied goal through `meld-lang` values and pure operations.

## Contract Chain

1. Events provide `DomainObjectRef` and `EventRelation`.
2. World model graph identifies the subject object.
3. World model belief produces a planner-facing belief value.
4. World model planner projects a ground `WorldState`.
5. World model agent creates a ground `Goal`.
6. Execution goals stores the `Goal`.
7. Execution planning evaluates the `Goal` target against `WorldState`.
8. Execution planning matches a `Method` trigger with `unify`.
9. Execution planning substitutes bindings into a `Composition`.
10. Execution planning validates the `Composition`.
11. Execution planning applies method `Effect` values to `WorldState`.
12. Execution planning reevaluates the `Goal`.
13. Evaluation returns `Satisfied`.

## Scenario

The first subject is a docs node identified by `DomainObjectRef`.

The first belief dimension id is loaded from runtime configuration as `docs_freshness`.

The initial `WorldState` contains a `Proposition::Holds` value for `docs_freshness` below `0.7`.

The agent creates a `Goal` whose target requires `docs_freshness` above `0.7`.

The method trigger matches that target.

The method composition contains one operator whose effect updates `docs_freshness`.

Applying the method effect creates a projected `WorldState` where reevaluation returns `Satisfied`.

## Required Interfaces

- `DomainObjectRef`
- `Term`
- `Literal`
- `Condition`
- `Proposition`
- `Effect`
- `WorldState`
- `Goal`
- `GoalPriority`
- `GoalSource`
- `GoalLifecycle`
- `Method`
- `Bindings`
- `Composition`
- `Operator`
- `CostEstimate`
- `evaluate`
- `unify`
- `substitute`
- `validate`
- `apply_effects`

## Owners

`events` owns identity and relation primitives.

`world_model/graph` owns subject identity, current anchors, lineage, and relation structure.

`world_model/belief` owns runtime configuration loading, confidence, uncertainty, freshness, and evidence settlement for `docs_freshness`.

`world_model/planner` owns projection from internal world model state into `WorldState`.

`world_model/agent` owns normative judgment and `Goal` construction.

`execution/goals` owns goal storage and lifecycle mutation.

`execution/planning` owns mechanical evaluation, method matching, substitution, validation, and effect projection.

`meld-lang` owns the shared value types and pure operations.

## Acceptance Criteria

Every item in the contract chain has a named owner.

Every boundary crossing names the data type passed across it.

No step requires semantic interpretation by execution.

No step requires IO, async behavior, persistence, provider calls, task dispatch, or event append.

The scenario can be translated directly into one `meld-lang` integration test.

## Deferred Runtime Work

- task network graph executor
- task network command acceptance
- plan diffing
- switching cost model
- capability catalog bridge
- outcome publication bridge
- workflow integration strategy
