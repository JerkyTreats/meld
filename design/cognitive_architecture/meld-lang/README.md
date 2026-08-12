# Meld Lang

Date: 2026-05-18
Status: active
Scope: shared typed substrate for propositions, goals, operators, and compositions across world model and execution

## Thesis

The cognitive architecture requires a shared language between `world_model` and `execution`. The world model expresses what it believes and what it desires. Execution plans and acts to close the gap between belief and desire. Without a shared language, either domain must interpret the other's intent, violating the boundary axiom: world model owns epistemic judgment, execution owns operational commitment.

`meld-lang` is that shared language. It defines three primitives — terms, propositions, and effects — and the structural types composed from them at runtime: operators, compositions, goals, and methods. Both `meld-world-model` and `meld-execution` depend on `meld-lang`. Neither depends on the other.

The language is pure. No IO, no async, no side effects, no persistence. Every function takes values and returns values. The crate is the typed contract surface between epistemic and operational authority.

## Motivation

The gap this crate resolves is the planning pipeline's missing middle. The world model owns graph, belief, causation, regime, and planner-facing projection. Execution owns task compilation, capability contracts, readiness computation, and task execution. Strategy uses the shared language to construct semantic candidate Compositions. Execution Planning uses the same language to realize Agent-authorized candidates as task-network mutations.

Execution must not interpret semantic goals through linguistic decomposition layers — parsing goal fields, retrieving matching methods by frame analysis, filling precondition gaps through means-end search, or falling back to an LLM for novel situations. That pushes epistemic work into execution — the wrong domain for it.

The resolution: define a formal language that both domains speak natively. The world model constructs Goals as typed propositions and Strategy constructs candidate Compositions. Execution evaluates propositions and compiles Agent-authorized Compositions mechanically. Meaning and intention live in the world model where they belong.

A critical property of this language is **runtime composition**. Goals, Methods, Operators, and Compositions are runtime values rather than compile-time action enums. [World Model Strategy](../world_model/strategy/README.md) may seed construction from a configured Method and may construct directly from Capability contracts; either path yields a concrete Composition that passes through the same validation used by Execution.

## Core Design

### Three Primitives

The language has exactly three primitive types. Everything else is structure over them.

**Term** — a reference to anything in the world. Domain objects, belief dimensions, artifact types, literal values, unbound variables, and derived references from operator outputs. Terms are runtime values. New kinds of objects, dimensions, and artifacts require no language changes.

**Proposition** — a typed statement about the world. Relations between terms. A finite set of proposition shapes (the grammar) combined with open-ended terms (the vocabulary). Propositions express what the world model believes, what goals desire, what operators require, and what operators produce. The same type everywhere.

**Effect** — a state change to the proposition space. What an operator does to the world. Assert a proposition, retract a proposition, or update a belief dimension. Effects are the typed bridge between operator execution and world state change.

### Grammar vs. Vocabulary

The proposition shapes and condition shapes are compiled — they define what kinds of statements the evaluator understands. These are enum variants: `Holds`, `Exists`, `Accessible`, `Related`, `All`, `Any`, `Not` for propositions; `Above`, `Below`, `Equals`, `Within`, `Exceeds`, `Present`, `Absent` for conditions.

The vocabulary — which objects, which dimensions, which values, which artifact types — is entirely runtime. New dimensions, new artifact types, new domain objects are just new `Term` values. They work with existing evaluation, unification, and validation logic without recompilation.

Grammar changes (new proposition shapes or new condition kinds) are rare and deliberate. Vocabulary changes are continuous and expected.

### Structure Types

**Operator** — a runtime-constructed contract: preconditions (propositions that must hold), effects (state changes), cost estimate, and a resolution hint for finding a matching capability in the catalog. Operators are NOT named action families with compile-time variants. They are defined by their typed boundary. The catalog resolves them at dispatch via query, not identity lookup.

**Composition** — a directed graph of steps (operators or sub-goals) with typed edges (ordering, data flow, conditional). Constructed at runtime. Validated by the language crate's structural checker. Compiled against the capability catalog by execution. A composition is the planning language's equivalent of a task network subgraph.

**Goal** — a proposition with operational metadata (priority, source, lifecycle). Constructed by the world model agent. The target is a `Proposition`. Execution evaluates it mechanically. No semantic interpretation by execution.

**Method** — a cached, reusable Composition with a trigger pattern. Methods are optional optimizations. The system works without them because Strategy may construct episode-specific concrete Compositions. Methods exist because proven generalized decompositions should not be re-derived every cycle.

### World State

`WorldState` is a set of ground propositions (no variable terms) representing the current modeled world. Published by the world model's planner-facing projection. Consumed by the planning loop for proposition evaluation, gap detection, and precondition checking. The language crate owns the `WorldState` type and the pure evaluation operations over it.

## Boundary

`meld-lang` owns:

- `Term`, `Literal` — the reference primitive
- `Proposition`, `Condition` — the statement primitive
- `Effect` — the state change primitive
- `Operator`, `Resolution`, `SlotConstraint`, `CapabilityRef` — operator contract and resolution
- `Composition`, `Step`, `StepKind`, `Edge`, `EdgeKind` — composition graph
- `Goal`, `GoalPriority`, `GoalSource`, `GoalLifecycle` — goal specification
- `Method` — cached composition with trigger pattern
- `CostEstimate` — multi-dimensional cost
- `WorldState` — ground proposition set
- `evaluate()` — proposition evaluation against world state
- `unify()`, `Bindings` — pattern matching with variable binding
- `substitute()` — binding substitution into compositions
- `validate()` — structural validation of compositions
- `apply_effects()` — effect application producing new world state
- `settlement()` and the settlement proposition shape — the pure transform Strategy regression evaluates, a deliberate grammar addition defined in [Goals and Methods](goals_and_methods.md)

`meld-lang` does not own:

- capability resolution against a catalog (execution's concern)
- task compilation (execution's compiler)
- belief revision from evidence (world model's concern)
- goal curation decisions (world model agent's normative judgment)
- Strategy decision persistence and authorization
- event publication or consumption (spine's concern)
- dispatch, scheduling, retry, or runtime lifecycle (execution runtime)
- persistence of any kind
- dimension semantics — the crate does not know what "docs_freshness" means

## Dependency Rule

`meld-lang` depends on `meld-events` for `DomainObjectRef` and `EventRelation`. These are the shared identity types.

`meld-lang` does not depend on `meld-world-model`, `meld-execution`, or root `meld`.

Both `meld-world-model` and `meld-execution` depend on `meld-lang`.

```
meld (core)
 ├── meld-execution ─────┬── meld-lang ── meld-events
 │                        │
 └── meld-world-model ───┘
```

## Documents

- [Completed Implementation Plan](../../completed/meld_lang/PLAN.md)
  phased implementation order, gates, and verification for the `meld-lang` crate
- [Crate Boundary](CRATE.md)
  crate identity, ownership, dependency rule, forbidden directions
- [Requirements](requirements.md)
  functional, structural, and nonfunctional requirements
- [Primitives](primitives.md)
  Term, Proposition, Condition, Effect — the three primitives and their grammar
- [Operators and Resolution](operators.md)
  Operator contract, Resolution query, catalog bridge
- [Compositions](compositions.md)
  Composition graph, Step, Edge, structural validation
- [Goals and Methods](goals_and_methods.md)
  Goal specification, Method caching, pattern unification
- [World State and Evaluation](world_state.md)
  WorldState, evaluate(), gap detection, effect application

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Execution Domain](../execution/README.md)
- [Planning Pipeline](../execution/planning/planning_pipeline.md)
- [Task Network](../execution/task_network.md)
- [World Model Domain](../world_model/README.md)
- [World Model Planner](../world_model/planner/README.md)
- [World Model Agent](../world_model/agent/README.md)
- [Events Domain](../events/README.md)
- [Execution Gap Ledger](../../plan/execution/gaps.md)
