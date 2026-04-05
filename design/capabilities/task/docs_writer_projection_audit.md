# Docs Writer Projection Audit

Date: 2026-04-04
Status: active
Scope: audit of the current docs-writer task template to separate projection concerns, capability concerns, and unresolved state ownership

## Intent

Define what should move out of the current docs-writer task template and into a more durable projection entity before full task-network work exists.

The goal is not to build full task-network now.
The goal is to stop letting docs-writer package lowering carry hidden runtime policy.

## Current Reality

The current implementation in [docs_writer.rs](/home/jerkytreats/meld/src/task/templates/docs_writer.rs) is doing three different jobs:

- structural lowering into a task definition
- projection seeding from current workspace and frame state
- docs-writer specific policy interpretation

Those jobs should not stay collapsed into one template helper.

## What The Template Is Doing Today

### Structural lowering

These parts are valid package lowering work:

- resolve target node id
- call `merkle_traversal`
- expand per-node stage chains
- wire stage-to-stage artifact dependencies
- wire child `readme_final` into parent `evidence_gather`
- compile the resulting task definition

This is structural truth.
It is close to task authoring and can eventually become structured package data plus generic lowering.

### Projection seeding

These parts are not just structural lowering:

- inspect current heads for `context-docs-writer`
- decide which nodes already have completed final outputs
- filter active traversal batches
- inject existing child `readme_final` values as init artifacts

This is a runtime view over one task run.
It is the early form of a projection.

### Policy interpretation

These parts are docs-writer policy:

- parent `evidence_gather` consumes child `readme_final`
- only `style_refine` persists the final frame
- workflow turn prompt refs map to repeated prepare, execute, finalize regions

This policy is valid.
It should remain explicit.
It just should not stay hidden inside template code forever.

## Recommended Split

### Move up into a projection entity

Create a docs-writer run projection that represents the current execution view for one task run.

Recommended owned facts:

- target node id
- ordered traversal batches
- node completion state by node id
- existing final output availability by node id
- active node set
- blocked parent set
- released node set
- current stage frontier if stage-level release is needed later

This projection is not a capability.
It is task-scoped run state derived from durable task records plus current workspace frame state at run creation.

### Move down into capability concerns

The following policy should become capability-facing, not template-owned:

- traversal order derivation
  already belongs in `merkle_traversal`
- prompt preparation and stage shaping
  already belongs in `context_generate_prepare`
- provider execution
  already belongs in `provider_execute_chat`
- final output shaping and persistence policy
  already belongs in `context_generate_finalize`

Potential next capability to add:

- `docs_writer_release_planning`

Its role would be:

- consume traversal batches plus projection snapshot
- emit ready node or stage release sets
- emit blocked reasons
- remain stateless and artifact-driven

That would let docs-writer policy fit the capability system without making capability stateful.

### Keep in generic task execution

The generic executor should still own:

- payload assembly
- invocation triggering
- artifact persistence
- ready-set reevaluation
- failure recording

That work should not become docs-writer specific.

## Recommended Projection Shape

The minimum useful projection can stay narrow.

Conceptual shape:

```rust
struct DocsWriterProjection {
    task_run_id: String,
    target_node_id: NodeID,
    traversal_batches: Vec<Vec<NodeID>>,
    node_states: HashMap<NodeID, DocsWriterNodeState>,
}

struct DocsWriterNodeState {
    final_output_state: FinalOutputState,
    release_state: ReleaseState,
    child_requirements_satisfied: bool,
}

enum FinalOutputState {
    Missing,
    SeededFromExistingFrame,
    EmittedByTaskRun,
}

enum ReleaseState {
    Pending,
    Ready,
    Released,
    Completed,
    Failed,
}
```

This is enough to express:

- which nodes are already done
- which nodes still need work
- which parents are unblocked
- which nodes may be released now

## What Should Stop Living In The Template

The following logic is the main candidate to leave `docs_writer.rs`:

- `collect_existing_readme_artifacts`
- `filter_active_batches`
- active versus seeded child routing

Those functions are projection logic, not pure package structure.

## What Can Stay In The Template For Now

The following is acceptable interregnum package lowering:

- stage expansion by ordered docs-writer turns
- binding prompt refs and gate contracts into repeated stage instances
- static dependency rule that parent `evidence_gather` depends on child `readme_final`

This is authored behavior shape.
It is still package definition work, even if it is currently written in Rust.

## Stateful Questions Still Open

### When is projection materialized

Open question:

- only at run creation
- or recomputed after every artifact emission

Recommendation:

- materialize at run creation
- update by reducer style projection updates after artifact emission

### Who owns node completion truth

Open question:

- task artifact repo alone
- or projection plus repo

Recommendation:

- artifact repo is source of truth
- projection is a cached execution view

### Where does ready-set policy live

Open question:

- generic task readiness engine only
- or docs-writer specific release planning capability

Recommendation:

- keep generic readiness for already compiled dependency edges
- add a release-planning capability only if docs-writer still needs dynamic release policy after package lowering is made more declarative

### How much state should exist before task-network

Open question:

- should the projection already look like a task-network projection

Recommendation:

- yes in shape
- no in scope

Use a task-scoped projection now so it can later lift into task-network with minimal redesign.

## Immediate Refactor Direction

Before building full task-network, the next durable step should be:

1. define a task-scoped docs-writer projection record
2. move seeded-existing-output detection into projection creation
3. make template lowering consume the projection instead of directly querying heads
4. decide whether release planning can remain generic or needs a dedicated capability

That gives a better boundary now without forcing the entire task-network implementation first.

## Bottom Line

The current template is carrying too much runtime meaning.

The clean split is:

- package template owns structural docs-writer shape
- projection owns current docs-writer run state
- capability owns stateless computation over projection snapshots and artifacts
- task executor owns generic triggering and persistence

That is the path that removes hidden docs-writer control logic without requiring full task-network immediately.
