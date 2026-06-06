# Execution Domain Gaps

Date: 2026-06-02
Status: active
Scope: open contracts and undefined seams preventing a complete execution architecture

## Purpose

The execution domain has strong lower layers (capability, task, task network, control program) and strong internal design (repair, synthesis, runtime continuation). The gaps are at the top (goal model), at the edges (world model reads, outcome publication), and in the relationship between the current production path and the cognitive pipeline direction.

Each gap below names what is missing, why it blocks a complete architecture diagram, and what minimum contract would close it.

## Gap 1: Goal Model

**Status: resolved. Goal type implemented in `meld-lang`. Residual gaps identified.**

See [Goals](goals/README.md) for the full goal model and [World Model Agent](../world_model/agent/README.md) for the curation side.

The goal model resolves the structural gap through a clean ownership split. Execution owns the Goal Set as data (lifecycle, priority, satisfaction criteria) and exposes a public curation API. The world model agent curates the goal set — it evaluates perspective-scoped beliefs against its normative framework and issues goal mutations (add, modify, remove, satisfy) through the API. Execution reacts to the current goal set without understanding why it changed.

Goals are propositions about desired belief states. Satisfaction checking is owned by the world model agent (because it requires evaluating belief). The agent also has read access to active goals, enabling prediction (what evidence to expect given active goals) and anomaly detection (goals without belief movement).

The prior tension between world-state propositions and operational triggers is resolved — goals are propositions owned by execution, triggers are belief evaluations made by the world model agent that result in goal set mutations.

Residual gaps within the goal model:

- **agent normative framework** — the agent's policy for what belief states it cares about, what divergence thresholds trigger action, how it prioritizes; this is the core of the agent's decision-making and lives in world_model/agent
- **goal conflict resolution** — strategy for competing goals beyond priority and preemption policy
- **multi-agent goal coordination** — protocol for goals that interact across agents with overlapping concerns
- **goal learning** — whether the agent can refine its normative framework from goal outcomes

## Gap 2: Planning Pipeline

**Status: type substrate and expanded execution slice implemented. Residual runtime gaps identified.**

See [Planning Pipeline](planning/planning_pipeline.md) for the unified pipeline: two concurrent processes connected by task network commands.

The pipeline document resolves the structural gap. The previous six-stage sequential model and the separate adaptation domain both dissolved into the graphs-lower-graphs abstraction. Control flow is expressed as graph structure through conditional dependency edges and multi-dependency nodes, not as a separate compiled control program. Adaptation's reconciliation concern folds into the planning loop's cost-aware mutation proposal decisions.

`meld-lang` now provides the planning type substrate: `Method` with trigger, preconditions, composition, net effects, cost, and preference; `Composition` as a step and edge graph; `Operator` with preconditions, effects, cost, and resolution; plus `unify()`, `substitute()`, and `validate()`. Method definitions are typed values that can be serialized to and deserialized from JSON. The method matching flow — unify trigger against goal, check preconditions against world state, substitute bindings into composition, validate, project effects — is proven in integration tests.

Residual gaps within the pipeline:

- **method library loading** — the `Method` type and JSON serialization exist; the runtime infrastructure for loading, indexing, and querying method libraries belongs to `meld-execution` and is not yet implemented
- **planning algorithm** — the search strategy for HTN decomposition is unspecified; `meld-lang` provides the matching primitives but the search orchestration is an `meld-execution` concern
- **task network graph executor** — the expanded execution slice exists; remaining gaps are conditional edge evaluation, recursive sub-goal lowering, graph repair mutations, shared task reuse, and deeper runtime recovery
- **switching cost model** — cost-aware plan transitions require cost estimates on tasks and a model for computing cleanup cost, sunk cost, disruption cost, and benefit estimation

## Gap 3: World Model Read Interface

**Status: resolved. `WorldState` in `meld-lang` is the read interface.**

### What Exists

The world model planner layer defines projection types (`WorldModelView`, `DecisionContext`, etc.) that scope beliefs through graph, belief, causation, regime, and agent perspective.

`meld-lang` now provides the execution-side read contract: `WorldState` is a set of ground propositions published by the world model's planner-facing projection. `evaluate()` checks propositions against `WorldState` with three-valued semantics (Satisfied, Unsatisfied with gap, Indeterminate for missing knowledge). `WorldState::gap()` returns unsatisfied and indeterminate sub-propositions. `WorldState::query()` performs pattern matching with variable binding.

### Resolution

The read interface is the shared language itself — not a bespoke port trait. The world model projects its internal state into `WorldState` (a `Vec<Proposition>` of ground terms). Execution evaluates goals, preconditions, and method triggers against this `WorldState` using pure functions. The three-valued evaluation semantics resolve the key questions:

- **current belief reads**: the planner reads `WorldState` propositions — entity state, dimensions, relations, artifact existence
- **precondition evaluation**: `evaluate(&world_state, &precondition)` returns `Satisfied`, `Unsatisfied { gap }`, or `Indeterminate { missing }`
- **information-gathering triggers**: `Indeterminate` results (missing knowledge vs wrong value) distinguish when to observe vs when to act
- **replanning triggers**: when `WorldState` changes such that `evaluate()` on active goal targets or composition preconditions changes result

### Residual

The world model planner layer must implement the projection from its internal types (`WorldModelView`, `DecisionContext`, etc.) into ground `WorldState` propositions. This is a `meld-world-model` concern — the contract shape is defined, the implementation is not.

## Gap 4: Outcome Publication Contract

**Status: resolved. Execution publishes task lifecycle events to the spine. The world model consumes them and derives epistemic meaning.**

### What Exists

Execution publishes typed task lifecycle events to the event spine:

- `task_requested`, `task_started`, `task_progressed`
- `task_succeeded`, `task_failed`, `task_blocked`
- `task_artifact_emitted`, `task_cancelled`
- `repair_requested`, `repair_applied`

The world model reducer subscribes to these events via `replay_from_spine()` and `apply_event()`. It currently materializes claims from execution events: `GenerationSucceeded`, `GenerationFailed`, `ArtifactAvailable`. This closes the cognitive loop — execution acts, the world model observes execution's events and revises belief.

Execution does not construct semantically rich outcomes. It does not know what "docs_freshness" means or whether a task result implies a belief should change. Execution reports what happened (succeeded, failed, artifacts produced). The world model derives what that means through its belief layer, causal layer, and agent normative framework.

`meld-lang` provides `Effect` (Assert, Retract, Update) and `WorldState::apply()` for the planning loop's internal effect projection — projecting what a composition's operators *would* change if executed, before dispatching tasks. This is a planning concern (forward projection), not an outcome publication concern.

### Residual

The world model's reducer currently produces coarse claims from task events. As the belief layer matures, the reducer's interpretation of execution events will grow richer — more nuanced belief revision from the same task lifecycle signals. This is a world model concern, not an execution gap.

Execution's responsibility is to emit sufficient factual detail in its task events (what ran, what artifacts were produced, what failed and how) so that the world model has adequate signal. If the current event payloads prove insufficient for belief revision, the fix is richer event content — not execution constructing epistemic judgments.

## Gap 5: Workflow Integration Strategy

### What Exists

The codebase has approximately 12,000 lines of workflow execution in `meld-execution`:

- turn-based executor with retry and gate evaluation
- state persistence to filesystem
- prompt resolution from artifact IDs and file paths
- generation orchestration with level-by-level queue submission

This is the current production execution path. It runs today. The task/capability engine was proven through workflows. The task package system (`task/package/`) already bridges workflow definitions into task-compiler-consumable specs.

The design docs describe a cognitive pipeline (goals, HTN planning, control programs, task network, task, capability) that does not mention workflows. The CRATE.md lists "workflow execution runtime" as owned by `meld-execution` but no design doc explains how workflows relate to the cognitive pipeline.

### Why It Blocks

The execution architecture cannot be drawn without accounting for the 12,000 lines of working orchestration that make the system usable today. The cognitive pipeline (goals → planning → control programs → task network) is the direction of travel, but every subsystem in that pipeline except task and capability is unbuilt. Until the full pipeline exists, workflows are the substrate that closes the loop.

This creates a chicken-and-egg problem: the cognitive subsystems cannot be proven without a working execution loop, and the working execution loop is workflows.

### Strategic Direction

Workflows should not be ripped and replaced. At 12,000 lines of proven orchestration, they are a non-trivial component that should be elevated into the execution subsystems rather than discarded.

Two integration strategies are available, and both may apply to different parts of the workflow system:

**Strategy A: Extend workflows with execution subsystems.** Where a cognitive subsystem (planning, repair, world-model reads) can be introduced as an extension to the existing workflow runtime, do that. The workflow executor gains new capabilities over time rather than being replaced by a parallel engine.

Examples of where this applies:

- world-model reads can be added as a new input source for workflow turn resolution, alongside the existing prompt and artifact resolution
- outcome publication can be added as a new output path after turn completion, alongside the existing state persistence
- gate evaluation already performs a simple form of observation-and-branch; this can be extended toward the control program semantics rather than reimplemented

**Strategy B: Treat workflows as a compatibility layer.** Where the cognitive pipeline is incomplete, workflows paper over the gap with user configuration. The workflow profile, turn structure, and gate definitions serve as user-facing configuration for behavior that will eventually be planner-driven.

Examples of where this applies:

- workflow profiles currently define turn sequences that a planner would eventually derive from goals and belief; until planning exists, the user-authored profile is the plan
- workflow gates currently define quality checks that a belief layer would eventually drive; until belief exists, the user-authored gate is the quality contract
- workflow thread policies currently define retry and failure handling that repair semantics would eventually own; until repair is fully specified, the user-authored policy is the repair strategy

### What Would Close It

A mapping document that walks through each major workflow subsystem and classifies it:

| Workflow subsystem | Lines (approx) | Strategy | Target execution area | Integration path |
|---|---|---|---|---|
| turn executor | 800 | extend | task network / runtime | turn execution becomes task-network-driven dispatch with workflow profiles as task package specs |
| gate evaluation | 225 | extend | program / repair | gates become guard bindings or observation-wait conditions in control programs |
| state persistence | 250 | extend | runtime / continuation | workflow thread state becomes continuation state in the durable runtime model |
| prompt resolution | 290 | keep | capability | prompt resolution is a capability-level concern that survives as-is |
| generation orchestration | 500 | extend | task network | level-by-level queue submission becomes task-network dispatch |
| direct executor | 4,000 | compatibility | task network | the non-task-path executor is the compatibility layer for workflows that have not been lowered into task packages |
| lifecycle state machine | 900 | extend | runtime | thread lifecycle maps onto task-network state (pending → running → completed/failed) |
| event emission | 200 | extend | outcome publication | workflow turn events become structured outcome facts |
| retry / failure | 370 | compatibility → extend | repair | currently user-configured; extends toward repair semantics as repair matures |
| normalization | 180 | keep | capability | output normalization is a capability-level concern |

The mapping does not need to be implemented all at once. It should identify which workflow subsystems can be extended incrementally and which must wait for their target execution area to exist.

### Current Tension

The risk with both strategies is drift. If workflows are extended with cognitive subsystems piecemeal, the result may be a hybrid that is harder to reason about than either the current workflow engine or the target cognitive pipeline. The mapping document should define a coherence rule: at any point in time, a given execution concern should be owned by exactly one system (workflow or cognitive subsystem), not split across both.

## Dependencies Between Gaps

The gaps are not independent. Closing them in the wrong order produces circular definitions.

Current resolution state:

- **Gap 1 (goal model)**: **resolved and implemented.** `Goal`, `GoalPriority`, `GoalSource`, `GoalLifecycle` are implemented in `meld-lang`. Goals are typed propositions in the shared language. The world model agent constructs goals as `Proposition` targets with priority and lifecycle metadata. Execution evaluates goals mechanically without interpreting semantic intent. Residual: agent normative framework, goal conflict resolution, multi-agent coordination, goal learning. See [Lang Goals and Methods](../meld-lang/goals_and_methods.md).
- **Gap 2 planning pipeline**: **type substrate and expanded execution slice implemented.** `Method`, `Composition`, `Operator`, `unify()`, `substitute()`, and `validate()` are implemented in `meld-lang`. The planning loop reads goals and world state as propositions, matches methods via pattern unification, substitutes bindings into compositions, validates, and projects effects. Phase 8 proves multi node task network lowering, task init materialization, real task runtime dispatch, and replay. Residual: recursive planning algorithm, conditional graph execution, graph repair mutations, shared task reuse, and switching cost model. See [Lang Compositions](../meld-lang/compositions.md).
- **Gap 3 (world model read interface)**: **resolved and implemented.** `WorldState`, `evaluate()`, `EvalResult`, gap detection, and pattern query are implemented in `meld-lang`. The world model publishes `WorldState` as a set of ground propositions. Execution evaluates propositions with three-valued semantics. Residual: world model planner projection from internal types into ground `WorldState` propositions (`meld-world-model` concern). See [Lang World State](../meld-lang/world_state.md).
- **Gap 4 (outcome publication)**: **resolved.** Execution publishes task lifecycle events to the spine. The world model reducer consumes them and materializes claims for belief revision. `Effect` and `WorldState::apply()` in `meld-lang` serve forward projection in the planning loop, not outcome publication. Residual: world model reducer enrichment as belief layer matures (world model concern).
- **Gap 5 (workflow integration)**: continuous. Workflows remain the compatibility layer where cognitive subsystems are not yet built.

Recommended next resolution:

1. ~~**`meld-lang` first slice implementation**~~: **complete.** All types and pure operations implemented. Full evaluation loop proven end-to-end. Consumer crates (`meld-execution`, `meld-world-model`) compile with `meld-lang` as dependency.
2. ~~**Gap 4 residual (outcome publication)**~~: **resolved.** The loop already closes — execution emits task events, world model reducer consumes them and materializes claims. Reducer enrichment is a world model concern.
3. **Gap 2 residuals**: recursive planning algorithm, conditional graph execution, graph repair mutations, shared task reuse, and switching cost model.
4. **Gap 3 residual**: world model planner projection into `WorldState` (`meld-world-model`).
5. **Gap 5**: continuous integration as each subsystem matures.

## Read With

- [Execution Domain](README.md)
- [Goals](goals/README.md)
- [Execution Planning](planning/README.md)
- [Task Network](task_network.md)
- [Planning Pipeline](planning/planning_pipeline.md)
- [Synthesis Overview](synthesis/README.md)
- [Lang Domain](../meld-lang/README.md)
- [Lang Requirements](../meld-lang/requirements.md)
- [World Model Planner](../world_model/planner/README.md)
- [Events Design](../events/README.md)
