# Execution Domain Gaps

Date: 2026-06-02
Status: reference contract
Scope: contract gaps and seams required for a complete execution architecture

## Purpose

The execution domain joins capability, task, task network, planning, repair, synthesis, and runtime continuation. The gaps are in goal policy, world model reads, outcome publication, and the relationship between compatibility workflows and the cognitive pipeline.

Each gap below names what is missing, why it blocks a complete architecture diagram, and what minimum contract would close it.

## Gap 1: Goal Model

**Contract:** Goals are typed propositions in `meld-lang`, with residual policy gaps owned by the world model agent.

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

**Contract:** The planning type substrate and expanded execution boundary precede the residual runtime work.

See [Planning Pipeline](planning/planning_pipeline.md) for the unified pipeline: two concurrent processes connected by task network commands.

The pipeline document resolves the structural gap. The previous six-stage sequential model and the separate adaptation domain both dissolved into the graphs-lower-graphs abstraction. Control flow is expressed as graph structure through conditional dependency edges and multi-dependency nodes, not as a separate compiled control program. Adaptation's reconciliation concern folds into the planning loop's cost-aware mutation proposal decisions.

`meld-lang` defines the planning type substrate: `Method` with trigger, preconditions, composition, net effects, cost, and preference; `Composition` as a step and edge graph; `Operator` with preconditions, effects, cost, and resolution; plus `unify()`, `substitute()`, and `validate()`. Method definitions are typed values that support JSON serialization. The matching contract unifies a trigger against a goal, checks preconditions, substitutes bindings, validates the composition, and projects effects.

Residual gaps within the pipeline:

- **method library deepening** — loading, indexing, and querying method libraries belongs to `meld-execution`
- **planning algorithm** — the search strategy for HTN decomposition is unspecified; `meld-lang` provides the matching primitives but the search orchestration is an `meld-execution` concern
- **task network graph executor** — the expanded execution contract precedes conditional edge evaluation, recursive sub-goal lowering, graph repair mutations, shared task reuse, and deeper runtime recovery
- **switching cost model** — cost-aware plan transitions require cost estimates on tasks and a model for computing cleanup cost, sunk cost, disruption cost, and benefit estimation

## Gap 3: World Model Read Interface

**Contract:** `WorldState` in `meld-lang` is the execution read interface.

### Contract Surface

The world model planner layer defines projection types (`WorldModelView`, `DecisionContext`, etc.) that scope beliefs through graph, belief, causation, regime, and agent perspective.

`meld-lang` defines the execution-side read contract. `WorldState` is a set of ground propositions published by the world model planner-facing projection. `evaluate()` checks propositions with three-valued semantics. `WorldState::gap()` returns unsatisfied and indeterminate sub-propositions. `WorldState::query()` performs pattern matching with variable binding.

### Resolution

The read interface is the shared language itself — not a bespoke port trait. The world model projects its internal state into `WorldState` (a `Vec<Proposition>` of ground terms). Execution evaluates goals, preconditions, and method triggers against this `WorldState` using pure functions. The three-valued evaluation semantics resolve the key questions:

- **current belief reads**: the planner reads `WorldState` propositions — entity state, dimensions, relations, artifact existence
- **precondition evaluation**: `evaluate(&world_state, &precondition)` returns `Satisfied`, `Unsatisfied { gap }`, or `Indeterminate { missing }`
- **information-gathering triggers**: `Indeterminate` results (missing knowledge vs wrong value) distinguish when to observe vs when to act
- **replanning triggers**: when `WorldState` changes such that `evaluate()` on active goal targets or composition preconditions changes result

### Residual

The world model planner layer owns projection from its internal view and decision context types into ground `WorldState` propositions.

## Gap 4: Outcome Publication Contract

**Contract:** Execution publishes task lifecycle events to the spine, and the world model derives epistemic meaning.

### Event Contract

Execution publishes typed task lifecycle events to the event spine:

- `task_requested`, `task_started`, `task_progressed`
- `task_succeeded`, `task_failed`, `task_blocked`
- `task_artifact_emitted`, `task_cancelled`
- `repair_requested`, `repair_applied`

The world model reducer subscribes through `replay_from_spine()` and `apply_event()`. It materializes claims such as `GenerationSucceeded`, `GenerationFailed`, and `ArtifactAvailable` from execution events. Execution acts, and the world model observes factual outcomes and revises belief.

Execution does not construct semantically rich outcomes. It does not know what "docs_freshness" means or whether a task result implies a belief should change. Execution reports what happened (succeeded, failed, artifacts produced). The world model derives what that means through its belief layer, causal layer, and agent normative framework.

`meld-lang` provides `Effect` (Assert, Retract, Update) and `WorldState::apply()` for the planning loop's internal effect projection — projecting what a composition's operators *would* change if executed, before dispatching tasks. This is a planning concern (forward projection), not an outcome publication concern.

### Residual

The world model reducer may begin with coarse task claims and deepen interpretation as the belief layer matures. This remains a world model concern.

Execution must emit sufficient factual detail for the world model to revise belief. Insufficient payloads require richer factual event content, never execution-owned epistemic judgment.

## Gap 5: Workflow Integration Strategy

### Compatibility Contract

Compatibility workflow execution covers:

- turn-based executor with retry and gate evaluation
- state persistence to filesystem
- prompt resolution from artifact IDs and file paths
- generation orchestration with level-by-level queue submission

The workflow route remains a compatibility path. Task packages bridge workflow definitions into task-compiler-consumable specifications.

The design docs describe a cognitive pipeline (goals, HTN planning, control programs, task network, task, capability) that does not mention workflows. The CRATE.md lists "workflow execution runtime" as owned by `meld-execution` but no design doc explains how workflows relate to the cognitive pipeline.

### Why It Blocks

The execution architecture must account for compatibility orchestration while the cognitive pipeline assumes authority. Workflows close the compatibility loop until the full goal, planning, task network, task, and capability path carries the same obligations.

This creates a chicken-and-egg problem: the cognitive subsystems cannot be proven without a working execution loop, and the working execution loop is workflows.

### Strategic Direction

Workflows should be elevated into execution subsystems through compatibility boundaries rather than discarded as an undifferentiated unit.

Two integration strategies are available, and both may apply to different parts of the workflow system:

**Strategy A: Extend workflows with execution subsystems.** Where a cognitive subsystem (planning, repair, world-model reads) can be introduced as an extension to the existing workflow runtime, do that. The workflow executor gains new capabilities over time rather than being replaced by a parallel engine.

Examples of where this applies:

- world-model reads can be added as a new input source for workflow turn resolution, alongside the existing prompt and artifact resolution
- outcome publication can be added as a new output path after turn completion, alongside the existing state persistence
- gate evaluation can extend toward observation and branch semantics

**Strategy B: Treat workflows as a compatibility layer.** Where the cognitive pipeline is incomplete, workflows paper over the gap with user configuration. The workflow profile, turn structure, and gate definitions serve as user-facing configuration for behavior that will eventually be planner-driven.

Examples of where this applies:

- workflow profiles define turn sequences on the compatibility route, where the user-authored profile is the plan
- workflow gates define quality checks on the compatibility route, where the user-authored gate is the quality contract
- workflow thread policies define retry and failure handling on the compatibility route, where the user-authored policy is the repair strategy

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

The mapping may advance incrementally while preserving the dependency order of target execution areas.

### Coherence Risk

The risk with both strategies is drift. The mapping must preserve one owner for each execution concern throughout migration.

## Dependencies Between Gaps

The gaps are not independent. Closing them in the wrong order produces circular definitions.

Contract dependency state:

- **Gap 1 goal model**: goals are typed propositions. Residual scope covers normative policy, conflict resolution, coordination, and learning. See [Lang Goals and Methods](../meld-lang/goals_and_methods.md).
- **Gap 2 planning pipeline**: typed methods, compositions, operators, matching, substitution, and validation precede recursive planning, conditional graph execution, repair mutations, shared task reuse, and switching cost.
- **Gap 3 world model read interface**: `WorldState`, evaluation, gap detection, and pattern query define the shared boundary. Planner projection remains owned by `meld-world-model`.
- **Gap 4 outcome publication**: execution emits facts and the world model derives belief meaning.
- **Gap 5 workflow integration**: workflows remain a compatibility layer during authority migration.

Dependency order:

1. shared language types and pure operations
2. factual outcome publication and world model consumption
3. recursive planning, conditional graph execution, repair mutations, shared task reuse, and switching cost
4. world model planner projection into `WorldState`
5. workflow compatibility migration as each cognitive subsystem assumes authority

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
