# Program Oriented Capability Chain With Preserved HTN Lineage

## Summary

Expand the capability chain layer from a pure DAG of bound capability instances into a two layer model:

- a planning layer that performs HTN decomposition, search, and dynamic choice
- an execution layer that runs a compiled control graph with explicit control state, explicit data state, and preserved HTN lineage

Do not make the current compiled plan DAG itself directly Turing complete by adding arbitrary back edges into the existing plan graph model.

Instead, introduce a new compiled artifact kind above the current DAG:

- keep the current DAG model as the primitive capability dependency substrate
- add a program level control graph or state machine that can sequence, branch, loop, suspend, and repair across primitive capability subgraphs

This keeps capability contracts stable while giving HTN and interactive agent behavior a place to live.

## Why This Direction

Your target is interactive flexibility plus preserved HTN hierarchy.

That combination is a poor fit for a single locked DAG because:

- loops invalidate current cycle rejection rules
- dynamic revisits require runtime control state, not just node readiness
- repair needs hierarchy aware context, not only flat executable edges
- partial observability requires checkpoints and resumable decision points
- agentic adaptation requires plan growth or continuation rebinding during execution

A program level control graph handles those concerns more cleanly than overloading the existing compiled plan graph.

## Architectural Changes

### 1. Split three concerns that are currently intentionally separate but adjacent

- capability contract
- executable primitive graph
- planner or controller graph

Use these roles:

- `capability`
  - domain owned typed contracts
  - unchanged core responsibility

- `primitive plan`
  - validated dependency DAG of bound capability instances
  - still useful for parallel regions and artifact handoff validation
  - becomes a building block, not the whole story

- `control program`
  - first class control nodes for branch, join, loop, await observation, method choice, repair entry, and terminate
  - owns instruction pointer or continuation state
  - owns runtime frame state and method lineage

- `htn decomposition record`
  - preserves task, chosen method, subtask network, and mapping from abstract tasks to control graph regions and primitive capability nodes

### 2. Replace one graph with a layered runtime model

Recommended compiled runtime shape:

- control graph
  - nodes for decision and flow
  - edges for control transfer
- primitive region graph
  - existing DAG style capability instances
  - embedded as leaf regions or blocks inside control nodes
- hierarchy graph
  - task and method lineage
  - linked to both control nodes and primitive capability instances

This gives you Blueprint style authoring power without destroying primitive validation.

### 3. Redefine compiler into two passes

Pass A
- planner or decomposer produces a hierarchical intermediate form
- may be Turing complete
- may search, recurse, call heuristics, and revise candidate structure

Pass B
- compiler lowers the hierarchical form into:
  - control program
  - primitive capability regions
  - hierarchy lineage records
- validates each primitive region as a DAG even if the outer control program loops

This preserves the existing compiler thesis better than turning the current compiler into an online planner.

## Impact Areas

### Semantics

Current semantics
- readiness is derived from dependency satisfaction in a DAG

New semantics
- readiness depends on control location, dynamic guards, observation results, and loop state
- a capability node may be valid but not currently enabled because the program counter is elsewhere
- artifact handoff semantics split into:
  - compile time data wiring
  - runtime mutable environment or blackboard access

### Validation

You lose some current static guarantees if you allow full graph cycles in the same model.

To control that, move from whole graph validation to layered validation:

- primitive capability regions remain statically type checked and DAG-valid
- control graph is checked for structural well formedness, explicit exit paths, checkpoint presence, and loop policy
- HTN lineage is checked for valid task to method refinement links

Add required policies for every loop:

- termination posture
- max retry or max iteration when applicable
- checkpoint boundary
- side effect class
- repair policy

### Execution

Execution becomes a durable interpreter, not a topological walker.

Runtime needs:

- continuation record
- current control node id
- current hierarchy cursor
- variable environment or binding store
- observation store
- artifact store and lineage
- retry and repair counters
- suspension and resume boundary

This is a major expansion from the current deferred execution posture described in [Control Design](/home/jerkytreats/meld/design/control/README.md).

### Observability and audit

This gets better if you preserve hierarchy.

You can expose:

- active abstract task
- chosen method
- current control region
- current primitive capability node
- executed prefix under a task subtree
- repair history tied to original task intent

Without preserved hierarchy, repair in an HTN system degrades into generic workflow retry.

### Repair and replanning

This is the strongest reason to preserve task lineage.

When execution fails, you need to know:

- which primitive failed
- which abstract task it served
- which method branch is currently active
- whether repair can stay inside the same task subtree
- whether re decomposition must occur at a higher task boundary

That matches the HTN repair direction already noted in [design/htn_goap_research.md](/home/jerkytreats/meld/design/htn_goap_research.md).

## Important Public Interfaces And Type Changes

Add new public artifacts rather than mutating the current `compiled_plan` into an overloaded shape.

### New records

- `compiled_control_program`
  - durable executable control artifact
- `control_node`
  - typed node kinds such as branch, loop, await, invoke_region, terminate, repair_entry
- `control_edge`
  - control transfer with guard metadata
- `runtime_continuation`
  - resumable execution state
- `htn_task_instance`
  - task occurrence with hierarchy identity
- `htn_method_instance`
  - chosen method record
- `task_region_link`
  - maps HTN lineage to control regions and primitive nodes
- `execution_frame`
  - variable and observation environment for control execution

### Existing record posture

Keep and narrow the existing plan records:

- `compiled_plan`
  - either rename to `compiled_primitive_plan`
  - or keep as a region level artifact
- `capability_instance`
  - stays valid
- `dependency_edge`
  - stays valid inside primitive regions
- `artifact_handoff`
  - stays valid for producer consumer data flow

### Capability contract additions

Capabilities need more execution metadata than the current first slice describes.

Add fields for:

- side effect class
- replay safety
- compensation posture
- suspension safety
- input binding mode
- output persistence class
- observation contract
- failure classification contract

These are needed once loops and repair can revisit the same capability multiple times.

## Recommended Execution Rules

- Loops exist only in the control graph, not inside primitive capability regions
- Primitive capability regions remain DAGs
- Every loop header must declare a checkpoint boundary
- Every side effecting capability must declare idempotency or compensation posture
- Runtime decisions consume explicit observation artifacts, not hidden ambient state
- HTN task and method lineage is durable and queryable at runtime
- Repair always starts from the nearest valid task boundary, not from a raw node retry alone
- Dynamic capability insertion is allowed only through controlled recompilation of a new region or subprogram, not by mutating a locked region in place

## Main Risks

- static analyzability drops if control and dependency semantics are mixed into one graph
- resume correctness becomes hard without explicit continuations
- retries become dangerous without side effect and idempotency contracts
- audit becomes noisy if hierarchy and execution are not linked
- planner and executor boundaries blur if decomposition happens ad hoc during execution
- debugging becomes difficult if observations are implicit rather than persisted as artifacts

## Main Recommendation

Do not expand the existing current plan DAG into a fully cyclic general graph.

Instead:

1. Keep the current DAG model as the validated primitive execution substrate.
2. Add a new control program layer that is allowed to branch, loop, suspend, and repair.
3. Preserve HTN hierarchy as a first class compiled artifact.
4. Treat dynamic adaptation as controlled recompilation or method reselection at task boundaries.
5. Make continuation and observation persistence mandatory from the first execution slice.

This gives you most of the expressive power you want without throwing away the compiler clarity you already designed.

## Test Cases And Scenarios

- compile a simple primitive DAG region and confirm current compiler behavior remains unchanged
- compile a control program with a bounded retry loop over one primitive capability region
- resume mid loop after process restart and confirm no duplicated side effects
- preserve task and method lineage for one abstract task decomposed into two primitive regions
- perform repair inside a failed task subtree without recompiling unrelated completed regions
- reselect a method after a runtime observation invalidates the current branch
- validate rejection of a control loop with no checkpoint policy
- validate rejection of a side effecting capability used in a loop with no replay posture
- validate artifact lineage across repeated visits to the same capability region
- expose telemetry showing active abstract task, active method, active control node, and active primitive capability

## Assumptions And Defaults

- interactive agent behavior is a first class target
- HTN hierarchy must remain visible at runtime
- planner expressiveness lives primarily above the primitive plan layer
- the existing compiled DAG model remains valuable and should not be discarded
- durable execution and repair are more important than making the whole system appear as one uniform graph
- dynamic graph growth happens through controlled subprogram compilation, not arbitrary in place mutation of a locked artifact
