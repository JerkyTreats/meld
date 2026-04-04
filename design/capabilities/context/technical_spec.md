# Context Technical Spec

Date: 2026-04-03
Status: active
Scope: definitive refactor spec for preparing `src/context` for the capability feature set

## Intent

Define the concrete refactor required to make `context` usable as a clean capability provider.
This spec is based on the current code, not on the target architecture alone.

The key problem is not that `context` lacks a working generation seam.
The key problem is that the domain currently wraps that seam in compatibility planning, queue policy, and workflow-aware routing.

The refactor must preserve the atomic generation seam while moving orchestration concerns out of the domain.
It must also move provider-service execution concerns into `provider`.

## Source Synthesis

This specification synthesizes:

- [Context Capability Readiness](README.md)
- [Context Code Path Findings](code_path_findings.md)
- [Provider Capability Design](../provider/README.md)
- [Capability Model](../capability/README.md)
- [Domain Architecture](../domain_architecture.md)
- [Interregnum Orchestration](../../control/interregnum_orchestration.md)
- [Workflow Cleanup Technical Spec](../workflow_refactor/technical_spec.md)

## Definitive Boundary

### Start Condition

Current `context` owns all of these concerns at once:

- atomic generation behavior
- target expansion and ordering
- compatibility execution envelope creation
- queue startup and queue routing decisions
- workflow-aware execution mode selection
- partial lineage and telemetry shaping

### End Condition

After refactor, `context` should own only:

- atomic generation behavior
- context-owned query and frame read or write behavior
- typed capability-facing inputs and outputs
- domain-local validation and metadata construction needed for that atomic behavior

After refactor, `context` should not own:

- compiled task graph construction
- target ordering policy
- traversal batch release
- batch barrier coordination
- provider batching and throttling
- provider retry and backoff policy
- workflow-specific routing
- queue-local retry semantics that depend on orchestration mode
- compatibility plan envelope as the primary public contract

## Core Position

The atomic seam to preserve is [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs) and specifically `execute_generation_request`.

The main code to shrink or move around that seam is:

- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)
- [plan.rs](/home/jerkytreats/meld/src/context/generation/plan.rs)
- [executor.rs](/home/jerkytreats/meld/src/context/generation/executor.rs)
- [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs)
- [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)

This is the central architectural fact revealed by the code findings:

- the atomic domain seam already exists
- the orchestration shell around it is what blocks capability readiness

## Functional Contract Target

The first-slice `context_generate` capability should be shaped so that it can be called without asking `context` to derive the task graph around it.

Required inputs:

- scope reference for the target node
- typed traversal or target set artifact from outside `context`
- provider binding
- agent binding when needed
- generation policy binding
- explicit force or replay posture when relevant

Required outputs:

- generation result artifact
- frame reference artifact when a frame is materialized
- structured observation summary
- structured effect summary
- explicit failure classification suitable for retry policy outside the domain
- explicit provider handoff and result boundary

## Refactor Rules

### R1 Preserve the atomic generation seam

Required change:

- keep `execute_generation_request` as the core domain execution path
- keep prompt assembly, lineage preparation, metadata construction, result validation, and frame persistence behind that seam
- extract provider binding resolution and completion execution into a provider-domain handoff

Reason:

- this is the cleanest part of the current domain boundary
- rewriting the full path would increase risk without fixing the orchestration problem
- provider transport work is a separate concern and should not remain context-owned

### R2 Move target derivation out of `run_generate`

Required change:

- stop treating `build_plan` in [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs) as the durable upstream contract
- move subtree traversal, level construction, and head-reuse ordering policy out of the main domain entry path

Reason:

- target graph derivation is task or control work, not atomic context behavior
- keeping it inside `context` prevents a future task compiler from owning graph structure

### R3 Move traversal batch release into `control`

Required change:

- move bottom-up release order and wave progression out of `context`
- let `control` consume structural traversal batches and coordinate execution barriers

Reason:

- ordered release across batches is orchestration rather than atomic generation
- that logic must live somewhere real during the refactor window
- `control` is the correct owner once `context` narrows to capability-ready behavior

### R4 Downgrade `GenerationPlan` to compatibility status

Required change:

- stop treating `GenerationPlan` as the future durable public contract
- keep it only as a migration envelope while the new task layer comes online

Reason:

- the current type is level-based
- it lacks explicit dependency edges, artifact contracts, and capability instance identity
- it is useful for migration, but it should not define the future context capability boundary

### R5 Remove workflow mode from context-owned execution selection

Required change:

- remove workflow-aware execution mode selection from [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs)
- remove direct workflow override from the context generate request surface in [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)

Reason:

- current code shows inverted control
- `context` chooses the multi-step envelope and only then routes one branch into workflow
- the future architecture requires the opposite posture

### R6 Reduce queue to transport and bounded retry mechanics

Required change:

- remove `TargetExecutionProgramKind::Workflow` branching from [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
- remove workflow-specific retry classification from the queue
- stop using message text such as `failed gate` as orchestration policy input

Reason:

- queue should not decide which orchestration model is active
- queue should not own execution policy that depends on workflow-specific meaning
- capability and task layers need typed outcomes instead

### R7 Replace compatibility lineage gaps with explicit upstream lineage

Required change:

- stop emitting `workflow_id`, `plan_id`, and `level_index` as empty placeholders inside the atomic seam
- allow upstream task or control layers to supply explicit lineage when present

Reason:

- current telemetry proves the compatibility shell knows about lineage
- the atomic seam does not currently preserve it
- later capability execution will need explicit upstream lineage, not implicit workflow assumptions

### R8 Keep temporary bridge shapes only when they help migration

Required change:

- `TargetExecutionRequest` and related bridge shapes may remain temporarily
- they must be treated as compatibility inputs, not the long-term domain surface

Reason:

- the current queue and workflow paths already depend on these types
- migration should be incremental
- but these bridges should not become the final capability contract

## Change Program

### Phase C0

Freeze the atomic seam and identify replacement boundaries.

Required outcomes:

- `execute_generation_request` is named as the preserved seam
- `build_plan` and `GenerationPlan` are explicitly marked compatibility-only
- queue workflow branching is identified as removal work rather than capability work
- `control` is identified as the temporary orchestration owner for ordered batch release

### Phase C1

Extract target derivation and ordering from the domain entry path.

Required outcomes:

- recursive subtree ordering no longer originates in `run_generate`
- head reuse checks no longer decide durable graph shape inside `context`
- the domain can accept precomputed target inputs
- traversal batch release is delegated to `control`

### Phase C2

Introduce capability-facing input and output types around the atomic seam.

Required outcomes:

- provider, agent, and policy inputs become explicit
- generation result and frame reference outputs become explicit
- failure classification becomes structured enough for upstream retry logic
- provider execution becomes an explicit handoff rather than a hidden internal call

### Phase C3

Shrink queue responsibility.

Required outcomes:

- queue no longer branches into workflow execution
- queue no longer reasons about workflow-specific retry posture
- queue remains a transport and concurrency mechanism only

### Phase C4

Retire compatibility envelopes from the public domain center.

Required outcomes:

- `GenerationPlan` no longer defines the main public shape for context execution
- public exports move toward capability-facing contracts
- compatibility wrappers remain only where migration still requires them

## Verification Gates

### Boundary Gates

- `context` no longer derives compiled task graph structure internally
- `context` no longer selects workflow execution mode internally
- queue no longer chooses between atomic generation and workflow execution

### Seam Gates

- `execute_generation_request` still performs generation successfully
- provider handoff and frame persistence remain stable
- prompt-context lineage and metadata validation still occur inside the atomic seam

### Contract Gates

- capability-facing inputs validate before domain execution starts
- capability-facing outputs are explicit and reusable downstream
- retry classification is no longer inferred from workflow-specific error text

### Observability Gates

- upstream lineage can be attached when present
- atomic domain telemetry no longer hardcodes empty compatibility lineage fields as the only posture

## Completion Criteria

1. `run_generate` no longer defines the durable public shape of context execution.
2. `build_plan` no longer owns target graph derivation for the future architecture.
3. `GenerationPlan` is clearly compatibility-only.
4. queue no longer branches on workflow program kind.
5. `execute_generation_request` remains the preserved atomic domain seam.
6. `context_generate` can be called by a future task layer without asking `context` to invent the task graph around it.

## Read With

- [Context Capability Readiness](README.md)
- [Context Code Path Findings](code_path_findings.md)
- [Workflow Cleanup Technical Spec](../workflow_refactor/technical_spec.md)
- [Capability And Task Implementation Plan](../PLAN.md)
