# Workflow Bootstrap Roadmap

Date: 2026-03-01
Status: active
Scope: thread managed docs workflow foundation

## Entry Point

This is the primary entry point for the workflow bootstrap effort.
Use this file to understand outcome targets, operation order, ownership boundaries, and verification gates.
Each workload now lives in its own folder with a workload `README.md` as the anchor document.
Future workload specific specs should be added inside that workload folder and linked from the workload `README.md`.

## Milestone Outcome

Deliver a durable docs workflow that runs as ordered turns inside one thread, with strict metadata contracts, deterministic gates, and artifact lineage from source context to final `README.md`.

## Why This Work Matters

- quality improves through staged artifacts and explicit verification
- failures isolate to one turn and support safe resume
- provider variance is handled by internal thread state ownership
- metadata safety improves by removing raw prompt and raw context payload from frame metadata

## Reading Order

1. [Boundary Cleanup Foundation Spec](foundation_cleanup/README.md)
2. [Workflow Metadata Contracts Spec](metadata_contracts/README.md)
3. [Turn Manager Generalized Spec](turn_manager/README.md)
4. [Docs Writer Thread Turn Configuration Spec](docs_writer/README.md)
5. [Future Work Backlog](future_work.md)

## Workstream Overviews

### C0 Boundary Cleanup Foundation

Primary spec:
- [Boundary Cleanup Foundation Spec](foundation_cleanup/README.md)

Output:
- isolated metadata write and read boundaries
- reduced cross domain metadata coupling
- smaller generation orchestration units with clear responsibilities

Exit criteria:
- frame metadata contract checks exist at one shared write boundary
- storage integrity checks do not depend on free form metadata map lookup
- queue generation path is split into focused units with characterization tests

### R1 Context Placement Refactor

Primary spec:
- [Workflow Metadata Contracts Spec](metadata_contracts/README.md)

Output:
- prompt render and context payload stored as local CAS artifacts
- frame metadata stores only typed identifiers and digests

Exit criteria:
- `context get` with metadata cannot reveal raw prompt text
- `context get` with metadata cannot reveal raw context payload
- digest references resolve to CAS artifacts

### R2 Metadata Contract Refactor

Primary spec:
- [Workflow Metadata Contracts Spec](metadata_contracts/README.md)

Output:
- metadata key registry with ownership, class, and size contracts
- deterministic key validation for identity and attested classes

Exit criteria:
- invalid key writes fail deterministically
- oversized metadata writes fail deterministically

### F1 Conversation Metadata Feature

Primary specs:
- [Workflow Metadata Contracts Spec](metadata_contracts/README.md)
- [Turn Manager Generalized Spec](turn_manager/README.md)

Output:
- durable thread, turn, gate, and artifact link records
- deterministic turn sequencing and resume from failed turn

Exit criteria:
- one thread stores ordered turns with stable ids
- downstream turns remain blocked after failed gate until retry succeeds

### F2 Minimal Turned Docs Workflow Feature

Primary specs:
- [Docs Writer Thread Turn Configuration Spec](docs_writer/README.md)
- [Turn Manager Generalized Spec](turn_manager/README.md)

Output:
- docs writer profile executes four turns in fixed order
- each turn writes one artifact and each gate writes pass fail record

Exit criteria:
- artifact chain exists in full order from `evidence_map` to `readme_final`
- final output is `README.md` compatible content without evidence map section

## Operation Order

1. Complete C0 boundary cleanup foundation
2. Complete R1 context placement changes
3. Complete R2 metadata contract enforcement
4. Implement F1 thread and turn state model
5. Implement F2 docs writer workflow profile
6. Run full verification gates for data safety and workflow correctness

## Domain Ownership And Boundaries

- `src/prompt_context` owns prompt and context artifacts
- `src/metadata` owns metadata contracts, registry, and validation
- `src/workflow` owns thread and turn orchestration
- `src/context` owns frame generation and retrieval

Boundary direction:
- `src/context` delegates prompt and context artifact handling to `src/prompt_context`
- `src/context` delegates metadata validation to `src/metadata`
- `src/workflow` uses `src/context` through explicit domain contracts only

## Verification Gates

Data safety gates:
- no raw prompt text in frame metadata
- no raw context payload in frame metadata
- metadata budgets enforced for governed keys

Workflow correctness gates:
- thread record exists for each run
- turn count equals expected sequence length
- turn order is stable by declared sequence
- gate outcomes are deterministic for same inputs

Artifact integrity gates:
- complete artifact lineage across all four turns
- artifact identifiers are content addressed
- retry preserves turn input snapshot

## Execution Checklist

- [ ] isolate frame metadata contract validation at one shared write boundary
- [ ] remove storage integrity dependency on free form metadata map lookup
- [ ] split generation request processing into focused orchestration units
- [ ] implement CAS artifact placement for prompt and context payload
- [ ] enforce metadata key registry and size budgets
- [ ] persist thread turn gate and artifact link records
- [ ] load and validate workflow profile from config
- [ ] execute docs writer through turn manager
- [ ] verify deterministic gates and resume behavior

## Non Goals For This Milestone

- encryption rollout
- remote blob services
- multi thread orchestration
- generalized non docs workflows

## Future Work

Post feature exploration items live in [Future Work Backlog](future_work.md).
