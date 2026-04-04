# Merkle Traversal Technical Spec

Date: 2026-03-28
Status: active
Scope: first-slice extraction of `merkle_traversal` from current context generation logic

## Intent

Provide one implementation-facing execution spec for `merkle_traversal`.
This spec keeps the capability boundary explicit while also mapping the work into concrete code changes, outcomes, and verification gates.

## Source Synthesis

This specification synthesizes:

- [Merkle Traversal Capability](README.md)
- [Merkle Traversal Code Path Findings](code_path_findings.md)
- [Context Capability Readiness](../../context/README.md)
- [Interregnum Orchestration](../../../control/interregnum_orchestration.md)

## Boundary

Start condition:
- traversal derivation lives inside context generation setup
- the current durable projection is `levels`
- bottom-up traversal is hardcoded
- traversal and descendant readiness are blended into one control path

End condition:
- `merkle_traversal` is a distinct capability
- user-facing input includes `traversal_strategy`
- first-slice strategies are `bottom_up` and `top_down`
- output is a typed `ordered_merkle_node_batches`
- downstream code can consume traversal output without deriving traversal internally

## Functional Contract

`merkle_traversal` takes Merkle scope input, target selection input, and `traversal_strategy`.
It emits `ordered_merkle_node_batches`, traversal metadata, and structured observations.

The first slice should represent strategy internally as a closed enum such as `TraversalStrategy`.
This is strategy-shaped behavior, but the implementation should stay simple.
One enum plus one algorithm per variant is sufficient.

## Change To Outcome Map

### T0 Introduce explicit traversal strategy contract

Code changes:
- define `traversal_strategy` in the traversal capability input contract
- define first-slice accepted values as `bottom_up` and `top_down`
- define `ordered_merkle_node_batches` as the first-slice traversal output artifact

Outcome:
- callers specify traversal intent explicitly
- traversal output becomes a stable capability artifact rather than an implicit execution projection

Verification:
- contract validation accepts `bottom_up` and `top_down`
- unsupported strategy values fail deterministically

### T1 Extract traversal derivation out of `build_plan`

Code changes:
- isolate traversal derivation logic currently embedded in [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)
- move depth grouping and traversal sequencing into a traversal-owned service
- keep current bottom-up behavior as the baseline algorithm

Outcome:
- traversal derivation stops being owned by generation planning
- the codebase gains a reusable traversal seam

Verification:
- extracted bottom-up path produces the same node ordering as current behavior for characterization inputs

### T2 Add top-down traversal variant

Code changes:
- add a top-down branch in the traversal service selected by `TraversalStrategy`
- preserve deterministic subtree walk semantics while flipping ancestor-descendant order

Outcome:
- the first slice supports more than one traversal variant without changing capability identity

Verification:
- top-down outputs ancestors before descendants
- bottom-up and top-down produce visibly different orders on non-trivial tree shapes

### T3 Replace level-shaped traversal output with ordered node batch artifact

Code changes:
- stop treating `Vec<Vec<NodeID>>` level output as the primary traversal product
- introduce a typed ordered-node-batch artifact for traversal output
- keep execution release and provider timing as downstream control concerns

Outcome:
- traversal output remains structural and batch-shaped
- later control flows can consume traversal output without re-deriving it

Verification:
- traversal output artifact serializes and validates cleanly
- downstream control can release execution wave by wave from the ordered batch output

### T4 Separate traversal from descendant readiness semantics

Code changes:
- stop treating `find_missing_descendant_heads` as part of traversal output semantics
- keep descendant readiness validation separate from ordered-node-batch derivation

Outcome:
- traversal capability owns traversal only
- readiness checks stop distorting traversal artifact meaning

Verification:
- traversal output remains defined even when descendant readiness is evaluated elsewhere

### T5 Preserve downstream compatibility with control-owned orchestration

Code changes:
- update downstream seams so `control` consumes `ordered_merkle_node_batches`
- keep compatibility lowering or projection paths outside the traversal capability

Outcome:
- traversal is a reusable upstream capability rather than a context-only helper
- control can coordinate ordered execution without embedding traversal logic

Verification:
- control input expectations accept `ordered_merkle_node_batches`
- no downstream path needs to call internal traversal helpers directly

## File Level Execution Order

1. [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)
2. [plan.rs](/home/jerkytreats/meld/src/context/generation/plan.rs)
3. the new traversal capability-facing contract location under `src/capability` once introduced
4. the new traversal implementation location under `src/context` or another traversal-owning domain seam
5. downstream consumers that currently assume `levels`

## Verification Matrix

Contract gates:
- `traversal_strategy` accepts only supported first-slice values
- `ordered_merkle_node_batches` is a stable typed artifact

Behavior gates:
- bottom-up preserves current behavior
- top-down is deterministic and ancestor-first

Boundary gates:
- traversal no longer lives inside generation planning
- readiness checking is not part of traversal artifact semantics

Compatibility gates:
- `control` can consume traversal output without internal traversal derivation

## Completion Criteria

1. `merkle_traversal` exists as a standalone capability contract
2. bottom-up and top-down both work through one stable strategy field
3. traversal output is `ordered_merkle_node_batches`, not raw level vectors
4. downstream control input can consume traversal output directly
5. traversal derivation is no longer embedded in generation planning setup

## Read With

- [Merkle Traversal Capability](README.md)
- [Merkle Traversal Code Path Findings](code_path_findings.md)
- [Context Code Path Findings](../../context/code_path_findings.md)
- [Context Technical Spec](../../context/technical_spec.md)
- [Capability And Task Implementation Plan](../../PLAN.md)
