# Context Workflow Orchestration Plan

Date: 2026-03-07
Status: complete
Scope: context generation orchestration with workflow contract consumption

## Overview

This document defines the phased implementation plan for `completed/workflow_bootstrap/context_workflow_orchestration`.
The plan isolates `context` and `workflow` responsibilities, defines a clear target execution contract, and cuts `context generate` over to consume that contract without surrendering subtree orchestration.

Primary objective:
- keep `context generate` as the single owner of target expansion ordering batching skip policy retry policy and session progress while allowing workflow backed agents to run workflow logic for each target item

Target outcome:
- `context generate` expands directory targets into bottom up execution levels for all writer agents
- workflow backed agents run one target local workflow per generation item through an explicit contract boundary
- `context` depends only on workflow public contract types and facade functions
- telemetry can describe both batch progress and workflow progress within the same command session

## CLI Path Default Exception List

Project direction is path first targeting.
Current command surfaces that still include non default path behavior:

- `meld context generate` accepts `--node` as an alternate selector
- `meld context regenerate` accepts `--node` as an alternate selector
- `meld context get` accepts `--node` as an alternate selector
- `meld workflow execute` accepts `--node` as an alternate selector

This plan does not expand non default path behavior.

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 1 | Target execution contract extraction | None | complete |
| 2 | Workflow facade and contract implementation | Phase 1 | complete |
| 3 | Context orchestration cutover | Phase 1 and Phase 2 | complete |
| 4 | Telemetry and verification hardening | Phase 1 through Phase 3 | complete |

---

### Phase 1 — Target execution contract extraction

**Goal**: Define the context owned target execution contract and move execution mode selection out of the command route.

**Source docs**:
- [Foundation Cleanup Plan](../foundation_cleanup/PLAN.md)
- [Turn Manager Plan](../turn_manager/PLAN.md)
- [Metadata Contracts Plan](../metadata_contracts/PLAN.md)

| Task | Completion |
|------|------------|
| Create `src/context/generation/program.rs` for target execution request result and program reference types. | Complete |
| Add context owned execution mode selection service under `src/context/generation/selection.rs`. | Complete |
| Extend generation plan items with program reference data required for target execution dispatch. | Complete |
| Re export the new contract from `src/context/generation.rs` and `src/context.rs` without exposing workflow internals. | Complete |
| Add unit coverage for execution mode selection and plan serialization. | Complete |

**Exit criteria**:
- `context` owns a stable target execution contract
- generation items can describe one shot and workflow backed execution through data only
- command routing no longer needs workflow specific branching knowledge to select execution mode

**Key files and seams**:
- `src/context/generation.rs`
- `src/context.rs`
- `src/context/generation/program.rs`
- `src/context/generation/selection.rs`
- `src/context/generation/plan.rs`
- `src/context/generation/run.rs`
- `src/agent/identity.rs`
- `tests/integration/context_cli.rs`

**Verification gates**:
- compile gate: `cargo check`
- unit gate: `cargo test context::generation::plan`
- unit gate: `cargo test context::generation::selection`

---


**Implementation evidence**:
- compile gate passed: `cargo check`
- unit gate passed: `cargo test context::generation::plan -- --nocapture`
- unit gate passed: `cargo test context::generation::selection -- --nocapture`

**Phase completion notes**:
- `context` now owns the target execution contract through `TargetExecutionProgram` and related request and result types
- execution mode selection now flows through one context service rather than direct workflow binding checks in command routing
- generation plan items can carry execution program data without pulling workflow internals into the plan model

---

### Phase 2 — Workflow facade and contract implementation

**Goal**: Add a workflow public facade that implements the target execution contract for workflow backed agents while keeping workflow turn state internal to the workflow domain.

**Source docs**:
- [Turn Manager Plan](../turn_manager/PLAN.md)
- [Workflow Bootstrap README](../README.md)

| Task | Completion |
|------|------------|
| Add `src/workflow/facade.rs` as the only workflow entry seam consumed by `context`. | Complete |
| Define workflow request to target execution request mapping inside the workflow domain. | Complete |
| Return target execution result data with final frame id workflow id thread id and completed turn count. | Complete |
| Keep thread id derivation gate evaluation and resume policy inside workflow runtime internals. | Complete |
| Add unit and integration coverage for workflow facade behavior and completed thread reuse. | Complete |

**Exit criteria**:
- `context` can request workflow backed target execution through one workflow facade
- workflow runtime internals stay hidden behind domain owned API surface
- workflow facade returns only target level outcomes required by context orchestration

**Key files and seams**:
- `src/workflow.rs`
- `src/workflow/facade.rs`
- `src/workflow/executor.rs`
- `src/workflow/commands.rs`
- `tests/integration/workflow_cli.rs`

**Verification gates**:
- compile gate: `cargo check`
- unit gate: `cargo test workflow::executor`
- integration gate: `cargo test --test integration_tests integration::workflow_cli::`

---


**Implementation evidence**:
- compile gate passed: `cargo check`
- unit gate passed: `cargo test workflow::executor -- --nocapture`
- integration gate passed: `cargo test --test integration_tests integration::workflow_cli:: -- --nocapture`

**Phase completion notes**:
- workflow execution now has a public facade that maps target execution requests to workflow runtime requests and back to target results
- workflow command execution now uses the workflow facade rather than calling workflow executor internals directly
- workflow runtime still owns thread id derivation gate behavior and resume semantics behind the public facade

---

### Phase 3 — Context orchestration cutover

**Goal**: Remove workflow bypass in `context generate` and dispatch all generation items through the target execution contract while preserving bottom up batching.

**Source docs**:
- [Foundation Cleanup Plan](../foundation_cleanup/PLAN.md)
- [Turn Manager Plan](../turn_manager/PLAN.md)

| Task | Completion |
|------|------------|
| Remove direct workflow short circuit from `src/context/generation/run.rs`. | Complete |
| Resolve execution program once per command and attach that program to every generation item in the plan. | Complete |
| Route queue request processing through contract dispatch rather than direct one shot generation only. | Complete |
| Include execution program in dedupe identity so future program kinds remain isolated. | Complete |
| Add integration coverage for recursive directory generation with workflow backed agents. | Complete |
| Verify parent directory generation waits for lower level workflow backed items before final parent execution. | Complete |

**Exit criteria**:
- workflow backed agents respect subtree batching and bottom up ordering under `context generate`
- one workflow thread runs per target item rather than one workflow thread for the root directory only
- non workflow agents keep current behavior and public output shape

**Key files and seams**:
- `src/context/generation/run.rs`
- `src/context/generation/executor.rs`
- `src/context/generation/orchestration.rs`
- `src/context/queue.rs`
- `src/workflow/facade.rs`
- `tests/integration/context_cli.rs`
- `tests/integration/progress_observability.rs`

**Verification gates**:
- compile gate: `cargo check`
- integration gate: `cargo test --test integration_tests integration::context_cli::`
- integration gate: `cargo test --test integration_tests integration::workflow_cli::`
- integration gate: `cargo test --test integration_tests integration::frame_queue::`

---


**Implementation evidence**:
- compile gate passed: `cargo check`
- integration gate passed: `cargo test --test integration_tests integration::context_cli:: -- --nocapture`
- integration gate passed: `cargo test --test integration_tests integration::workflow_cli:: -- --nocapture`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue:: -- --nocapture`
- integration gate passed: `cargo test --test integration_tests integration::progress_observability::context_generate_with_workflow_agent_uses_context_plan_levels -- --nocapture`

**Phase completion notes**:
- workflow backed agents now stay inside `context generate` subtree planning and queue dispatch rather than bypassing into one root only workflow call
- queue dedupe identity now isolates execution program kind so workflow backed items and one shot items do not collide
- queued workflow execution uses an async workflow facade path so worker tasks do not start nested runtimes

---

### Phase 4 — Telemetry and verification hardening

**Goal**: Unify target level and workflow level telemetry within the `context generate` session and close the verification loop for the new contract boundary.

**Source docs**:
- [Metadata Contracts Plan](../metadata_contracts/PLAN.md)
- [Turn Manager Plan](../turn_manager/PLAN.md)
- [Observability Spec](../../completed/observability/observability_spec.md)

| Task | Completion |
|------|------------|
| Add execution program markers to generation plan and node events. | Complete |
| Add workflow target and turn progress events emitted through the workflow facade path. | Complete |
| Add typed summary coverage for `context generate` and `context regenerate`. | Complete |
| Add characterization coverage for workflow backed recursive generate telemetry and summary ordering. | Complete |
| Run full regression suite for touched domains and document verification evidence. | Complete |

**Exit criteria**:
- one command session can reconstruct overall batch progress and workflow turn progress together
- context summaries and workflow summaries remain ordered before `session_ended`
- verification evidence proves parity for non workflow and workflow backed generation paths

**Key files and seams**:
- `src/telemetry/events.rs`
- `src/telemetry/emission/summary_mapper.rs`
- `src/cli/help.rs`
- `src/context/generation/executor.rs`
- `src/workflow/facade.rs`
- `tests/integration/progress_observability.rs`

**Verification gates**:
- compile gate: `cargo check`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`
- integration gate: `cargo test --test integration_tests integration::generation_parity::`
- full suite gate: `cargo test`

---

**Implementation evidence**:
- compile gate passed: `cargo check`
- integration gate passed: `cargo test --test integration_tests integration::progress_observability:: -- --nocapture`
- integration gate passed: `cargo test --test integration_tests integration::generation_parity:: -- --nocapture`
- full suite gate passed: `cargo test`

**Phase completion notes**:
- generation plan and node telemetry now carry execution program markers so session consumers can distinguish one shot and workflow backed execution
- workflow facade execution now emits target and turn progress events that preserve plan id level index and target identity within the same command session
- `context generate` and `context regenerate` now emit typed context generation summaries before `session_ended`

## Verification Strategy

Verification proceeds one phase at a time.
Each phase closes with the phase specific gates listed above.
After every completed phase the plan status and task completion table are updated in place with verification evidence and completion notes.

## Implementation Order Summary

1. Define context owned target execution contract and selection rules
2. Add workflow public facade that implements the contract
3. Cut `context generate` and queue dispatch over to contract consumption
4. Harden telemetry summaries and integration coverage

## Related Documentation Links

- [Workflow Bootstrap README](../README.md)
- [Foundation Cleanup Plan](../foundation_cleanup/PLAN.md)
- [Turn Manager Plan](../turn_manager/PLAN.md)
- [Metadata Contracts Plan](../metadata_contracts/PLAN.md)
- [Observability Spec](../../completed/observability/observability_spec.md)
