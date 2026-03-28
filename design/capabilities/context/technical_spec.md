# Context Generate Technical Spec

Date: 2026-03-28
Status: active
Scope: first-slice extraction of `context_generate` into a clean capability-facing domain seam

## Intent

Provide one implementation-facing execution spec for `context_generate`.
This spec keeps the capability boundary explicit while also mapping the work into concrete code changes, outcomes, and verification gates.

## Source Synthesis

This specification synthesizes:

- [Context Capability Readiness](README.md)
- [Context Code Path Findings](code_path_findings.md)
- [Merkle Traversal Technical Spec](../capability/merkle_traversal/technical_spec.md)

## Boundary

Start condition:
- `context generate` mixes traversal, planning, execution setup, queue-local retry assumptions, and compatibility sequencing
- upstream execution projection is level-shaped
- retry classification remains partly string matched

End condition:
- `context_generate` is a clean capability-facing seam
- traversal arrives as `ordered_merkle_node_set`
- generation execution stays in the context domain
- plan construction and traversal derivation no longer live inside generation execution setup

## Functional Contract

`context_generate` takes scope input, `ordered_merkle_node_set`, generation policy binding, provider binding, and agent binding when required.
It emits generation result artifact, frame reference artifact when present, structured observation summary, and structured effect summary.

The capability owns generation behavior only.
Compiler owns compatibility and graph coherence.
Traversal belongs to `merkle_traversal`, not to `context_generate`.

## Change To Outcome Map

### C0 Preserve the strongest domain execution seam

Code changes:
- keep `execute_generation_request` in [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs) as the primary execution seam
- avoid rewriting core generation behavior while extraction work is underway

Outcome:
- refactor work wraps and clarifies the existing domain seam instead of destabilizing it

Verification:
- characterization coverage around `execute_generation_request` stays valid through the refactor

### C1 Remove traversal derivation from generation setup

Code changes:
- stop deriving traversal in [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs) as part of generation setup
- replace that assumption with typed traversal input from `merkle_traversal`

Outcome:
- `context_generate` no longer owns traversal
- generation setup becomes capability-focused rather than mixed with tree derivation

Verification:
- generation setup consumes `ordered_merkle_node_set` rather than raw subtree traversal helpers

### C2 Replace level-shaped upstream assumptions with typed artifacts

Code changes:
- stop using `GenerationPlan.levels` as the primary upstream contract for generation capability input
- introduce typed capability input shapes that carry scope and traversal artifacts explicitly

Outcome:
- capability input becomes stable and compiler-visible
- generation no longer depends on one legacy execution projection

Verification:
- typed capability input validation passes for valid traversal artifacts
- invalid traversal input fails at the capability boundary

### C3 Separate queue behavior from capability semantics

Code changes:
- reduce branching in [queue.rs](/home/jerkytreats/meld/src/context/queue.rs) that decides behavior based on workflow-shaped execution mode
- keep queue dispatch thin and move durable semantics to capability and plan contracts

Outcome:
- queue becomes transport and dispatch logic rather than orchestration logic
- `context_generate` stops inheriting workflow-shaped behavior from queue decisions

Verification:
- queue paths still dispatch correctly without owning durable capability semantics

### C4 Make retry and repair classification explicit

Code changes:
- stop depending on error message matching in [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
- replace that path over time with typed outcome and classification contracts

Outcome:
- retry posture becomes explicit and stable
- classification drift from message wording is reduced

Verification:
- typed retry classification coverage replaces string-match coverage as cutover proceeds

### C5 Make output artifacts explicit and downstream-safe

Code changes:
- define explicit generation result and frame reference outputs
- keep observation and effect summaries structured
- preserve lineage and metadata needed by downstream capabilities

Outcome:
- `context_generate` becomes a real capability producer
- downstream compiler validation can reason about generation outputs

Verification:
- output artifacts serialize, validate, and connect cleanly to downstream capability contracts

## File Level Execution Order

1. [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs)
2. [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)
3. [plan.rs](/home/jerkytreats/meld/src/context/generation/plan.rs)
4. [program.rs](/home/jerkytreats/meld/src/context/generation/program.rs)
5. [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
6. downstream capability contract and compiler input types once introduced under `src/capability` and `src/plan`

## Verification Matrix

Boundary gates:
- traversal no longer lives inside generation setup
- queue no longer owns durable generation semantics

Execution seam gates:
- `execute_generation_request` remains the main domain execution seam
- provider behavior and frame persistence remain stable

Artifact gates:
- typed generation inputs validate cleanly
- typed generation outputs are explicit and reusable downstream

Classification gates:
- retry and repair classification stop depending on string matching

## Completion Criteria

1. `context_generate` consumes typed traversal input rather than deriving traversal
2. generation execution still runs through the preserved domain seam
3. queue-local orchestration logic is reduced
4. output artifacts are explicit enough for downstream compiler validation
5. retry and repair classification are on a typed path rather than a message-match path

## Read With

- [Context Capability Readiness](README.md)
- [Context Code Path Findings](code_path_findings.md)
- [Merkle Traversal Technical Spec](../capability/merkle_traversal/technical_spec.md)
- [Capability And Plan Implementation Plan](../PLAN.md)
