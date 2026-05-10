# Execution Domain Gaps

Date: 2026-05-04
Status: active
Scope: open contracts and undefined seams preventing a complete execution architecture

## Purpose

The execution domain has strong lower layers (capability, task, task network, control program) and strong internal design (repair, synthesis, runtime continuation). The gaps are at the top (goal model), at the edges (world model reads, outcome publication), and in the relationship between the current production path and the cognitive pipeline direction.

Each gap below names what is missing, why it blocks a complete architecture diagram, and what minimum contract would close it.

## Gap 1: Goal Model

**Status: goal model defined with ownership split, residual gaps identified.**

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

**Status: pipeline defined under graphs-lower-graphs model, residual gaps identified.**

See [Planning Pipeline](planning/planning_pipeline.md) for the unified pipeline: two concurrent processes (planning loop + task network) connected by graph mutations.

The pipeline document resolves the structural gap. The previous six-stage sequential model and the separate adaptation domain both dissolved into the graphs-lower-graphs abstraction. Control flow is expressed as graph structure (conditional dependency edges, multi-dependency nodes), not as a separate compiled control program. Adaptation's reconciliation concern folds into the planning loop's cost-aware mutation decisions.

Residual gaps within the pipeline:

- **method library** — the library of available HTN decompositions does not exist in design or code; method definitions need preconditions over WorldModelView, sub-task sets, ordering constraints, cost estimates, and preference ordering
- **planning algorithm** — the search strategy for HTN decomposition is unspecified
- **task network graph executor** — the upper level of the fractal (graph of tasks, same execution model as graph of capabilities within a task) is not implemented; requires graph-based ready-set computation, conditional edge evaluation, and mutation acceptance
- **switching cost model** — cost-aware plan transitions require cost estimates on tasks and a model for computing cleanup cost, sunk cost, disruption cost, and benefit estimation

## Gap 3: World Model Read Interface

### What Exists

The world model planner layer defines:

- `WorldModelView`
- `ActionableBeliefView`
- `ObservationPolicy`
- `ExpectedInformationGain`
- `DecisionRelevance`
- `AbstentionState`
- `CausalEffectSummary`
- `RiskEnvelope`
- `ExecutionPreconditions`

The execution substrate says: "execution should read materialized current belief from the world model." The planning README says: "make planning explicitly world-model-aware" as a next item. The substrate also says: "planner to world-model coupling is the key missing bridge."

### Why It Blocks

This is the most important seam in the cognitive architecture. The observe-model-act loop requires execution to read from the world model before acting. Without a defined read interface on the execution side, there is no contract for how planning consumes world-model state. The architecture diagram cannot draw the arrow from world model to execution.

### What Would Close It

A planning input contract on the execution side that declares what it needs from the world model:

- **current belief reads**: what does the planner query? entity state, relation state, belief confidence, freshness?
- **precondition evaluation**: how does the planner check whether a goal's preconditions hold?
- **information-gathering triggers**: when does the planner request observation rather than action? what staleness or uncertainty threshold triggers sensing?
- **replanning triggers**: what belief change events cause the planner to revise the current plan?

This contract should consume the types defined in `world_model/planner` without importing world-model internals. The execution side should define a port trait (consistent with the existing port pattern in `execution/ports.rs`) that the world model planner layer satisfies.

## Gap 4: Outcome Publication Contract

### What Exists

The task network defines task lifecycle events:

- `task_requested`, `task_started`, `task_progressed`
- `task_succeeded`, `task_failed`, `task_blocked`
- `task_artifact_emitted`, `task_cancelled`
- `repair_requested`, `repair_applied`

The substrate says execution must publish: "success, failure, uncertainty discovered during action, evidence gathered during action."

### Why It Blocks

Task lifecycle events are internal orchestration signals. They tell the task network what happened to a task. They do not tell the world model what happened to the world.

The world model cannot consume "task_42 succeeded." It needs semantic outcome content: "the build now passes," "file X changed," "the API returned a 404." Outcome records must carry evidence and failure shape in a form that curation can integrate into belief revision.

Without this contract, the architecture diagram cannot draw the return arrow from execution back to the event spine in a way that closes the cognitive loop.

### What Would Close It

An outcome record contract that bridges task completion to world-model-legible facts:

- **outcome fact shape**: structured record containing what changed, what was observed, what failed, what remains uncertain
- **evidence items**: artifacts produced during execution that the world model can normalize into evidence for belief revision
- **failure shape**: not just "failed" but why and what was learned from the failure
- **publication target**: events published to the spine using fact types that curation reducers can consume (likely `ObservationFact`, `ActionFact`, `OutcomeFact` from the event spine vocabulary)

The outcome contract should be symmetric with the read interface: execution reads `WorldModelView` and publishes `OutcomeFact` records. The world model reads `OutcomeFact` records and updates belief.

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

- **Gap 1 (goal model)**: defined. Goals are belief predicates, lifecycle is specified, belief–goal bridge is drawn. Residual: generation policy, conflict resolution, multi-agent coordination.
- **Gap 2 (planning pipeline)**: defined under graphs-lower-graphs. Two concurrent processes, graph mutations, cost-aware transitions. Residual: method library, planning algorithm, task network graph executor, switching cost model.
- **Gap 3 (world model read interface)**: open. The goal model now makes this more concrete — goals need BeliefView reads, satisfaction checking needs continuous belief comparison, observation goals need the observation-needed signal. The read interface should be the next gap closed.
- **Gap 4 (outcome publication)**: open. The goal model's satisfaction-through-belief-revision pattern makes this concrete — execution outcomes must become spine facts that revise beliefs that satisfy goals. The return arrow of the cognitive loop.
- **Gap 5 (workflow integration)**: continuous. Workflows remain the compatibility layer where cognitive subsystems are not yet built.

Recommended next resolution:

1. **Gap 3 (world model read interface)**: the goal model and planning pipeline both consume belief views. The read interface contract is now well-motivated by concrete consumers: goal satisfaction checking, planning loop world-model-view subscription, observation goal triggers from belief uncertainty.
2. **Gap 4 (outcome publication)**: symmetric with gap 3. Execution outcomes must become evidence that revises beliefs that close the goal cycle.
3. **Gap 2 residuals**: method library and task network graph executor are the implementation gaps that prevent end-to-end execution.
4. **Gap 5**: continuous integration as each subsystem matures.

## Read With

- [Execution Domain](README.md)
- [Goals](goals/README.md)
- [Execution Planning](planning/README.md)
- [Task Network](task_network.md)
- [Planning Pipeline](planning/planning_pipeline.md)
- [Synthesis Overview](synthesis/README.md)
- [World Model Planner](../world_model/planner/README.md)
- [Events Design](../events/README.md)
