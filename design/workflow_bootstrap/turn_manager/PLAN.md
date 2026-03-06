# Turn Manager Implementation Plan

Date: 2026-03-06
Status: active planning
Scope: workflow bootstrap turn manager

## Overview

This document defines the phased implementation plan for `workflow_bootstrap/turn_manager`.
The plan maps target behavior from the functional spec and technical spec to dependency ordered phases with explicit gates and evidence expectations.

Primary objective:
- deliver workflow bound turn execution with deterministic state persistence and profile driven control flow while preserving legacy one shot behavior for unbound agents

Planned outcome:
- workflow profile registry resolver executor and state store are owned by `src/workflow`
- agent configuration supports explicit optional workflow binding
- workflow turn and gate execution is deterministic and policy driven
- runtime uses canonical workflow record contracts for thread turn gate and prompt linkage persistence
- CLI and watch runtime integrate workflow execution paths with clear adapter boundaries

## Related Specs

- [Turn Manager Functional Specification](README.md)
- [Turn Manager Code Path Findings](code_path_findings.md)
- [Turn Manager Technical Specification](technical_spec.md)
- [Workflow Metadata Contracts Plan](../metadata_contracts/PLAN.md)
- [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md)

## CLI Path Default Exception List

Project direction is path first targeting.
Current command surfaces that still include non default path behavior:

- `merkle context generate` accepts `--node` as an alternate selector
- `merkle context regenerate` accepts `--node` as an alternate selector
- `merkle context get` accepts `--node` as an alternate selector
- `merkle workspace delete` accepts `--node` as an alternate selector
- `merkle workspace restore` accepts `--node` as an alternate selector

This turn manager plan does not expand non default path behavior.

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 1 | Workflow profile loader and schema validation | None | planned |
| 2 | Agent workflow binding integration | Phase 1 | planned |
| 3 | Workflow runtime core registry resolver executor | Phase 1 and Phase 2 | planned |
| 4 | Durable workflow state persistence and resume | Phase 3 | planned |
| 5 | Prompt reference and artifact read integration | Phase 3 and Phase 4 | planned |
| 6 | CLI and watch adapter integration | Phase 2 through Phase 5 | planned |
| 7 | Verification lock and readiness signoff | Phase 1 through Phase 6 | planned |

---

### Phase 1 - Workflow profile loader and schema validation

**Goal**: introduce workflow profile discovery load priority and schema validation contracts.

**Source docs**:
- [Turn Manager Functional Specification](README.md)
- [Turn Manager Code Path Findings](code_path_findings.md)
- [Turn Manager Technical Specification](technical_spec.md)

| Task | Completion |
|------|------------|
| Add workflow profile source model in config domain with workspace user and default layers. | Planned |
| Implement deterministic source priority merge with duplicate workflow id detection per priority layer. | Planned |
| Implement profile schema decoding and validation for thread turn gate artifact and failure policy sections. | Planned |
| Add typed deterministic errors for invalid profile payload unresolved references and duplicate sequence ids. | Planned |
| Add inspection helper contract that exposes workflow id source and version metadata. | Planned |

**Exit criteria**:
- runtime resolves one deterministic profile set for identical inputs
- invalid profiles fail with typed deterministic validation errors
- profile provenance is available for inspection adapters

**Key files and seams**:
- `src/config.rs`
- `src/config/merge/service.rs`
- `src/config/sources/workspace_file.rs`
- `src/config/sources/global_file.rs`
- `src/workflow`
- `tests/integration/config_integration.rs`

**Planned verification evidence**:
- compile gate: `cargo check`
- unit gate: `cargo test workflow::registry`
- integration gate: `cargo test --test integration_tests integration::config_integration::`

---

### Phase 2 - Agent workflow binding integration

**Goal**: add optional workflow binding in agent profiles and validation for write capable agents.

**Source docs**:
- [Turn Manager Functional Specification](README.md)
- [Turn Manager Technical Specification](technical_spec.md)

| Task | Completion |
|------|------------|
| Add optional `workflow_id` field in agent profile config contract. | Planned |
| Extend agent validation to verify referenced workflow id exists in resolved registry. | Planned |
| Preserve zero or one workflow binding rule per agent. | Planned |
| Keep unbound agents on legacy one shot generation flow with no behavior drift. | Planned |
| Add typed binding errors for missing workflow ids and invalid compatibility states. | Planned |

**Exit criteria**:
- bound agent validation fails deterministically for unknown workflow id
- unbound agent generation behavior remains unchanged
- binding resolution seam is explicit and test covered

**Key files and seams**:
- `src/agent/profile/config.rs`
- `src/agent/profile/validation.rs`
- `src/agent/registry.rs`
- `src/workflow`
- `tests/integration/config_integration.rs`
- `tests/integration/generation_parity.rs`

**Planned verification evidence**:
- compile gate: `cargo check`
- unit gate: `cargo test agent::profile::validation`
- integration gate: `cargo test --test integration_tests integration::config_integration::`
- parity gate: `cargo test --test integration_tests integration::generation_parity::`

---

### Phase 3 - Workflow runtime core registry resolver executor

**Goal**: implement workflow owned orchestration for turn ordered execution with gate checkpoints.

**Source docs**:
- [Turn Manager Functional Specification](README.md)
- [Turn Manager Technical Specification](technical_spec.md)
- [Turn Manager Code Path Findings](code_path_findings.md)

| Task | Completion |
|------|------------|
| Implement workflow registry module for validated profile lookup. | Planned |
| Implement resolver module for turn input refs prompt refs and output refs contracts. | Planned |
| Implement executor turn loop with deterministic sequence ordering retry limits and failure policy branching. | Planned |
| Introduce gate evaluator registry and deterministic known gate type evaluation contracts. | Planned |
| Add compatibility adapter seam so unbound agent execution path remains legacy context generation. | Planned |

**Exit criteria**:
- bound agents execute declared turns in stable sequence
- retry and fail fast behavior matches profile failure policy
- gate outcomes are deterministic and persisted through runtime contracts

**Key files and seams**:
- `src/workflow.rs`
- `src/workflow`
- `src/context/generation/run.rs`
- `src/context/generation/executor.rs`
- `src/context/queue.rs`
- `tests/integration/generation_parity.rs`

**Planned verification evidence**:
- compile gate: `cargo check`
- unit gate: `cargo test workflow::executor`
- integration gate: `cargo test --test integration_tests integration::generation_parity::`

---

### Phase 4 - Durable workflow state persistence and resume

**Goal**: persist thread turn gate and linkage records with deterministic ids and resume semantics.

**Source docs**:
- [Turn Manager Functional Specification](README.md)
- [Turn Manager Technical Specification](technical_spec.md)
- [Workflow Metadata Contracts Plan](../metadata_contracts/PLAN.md)

| Task | Completion |
|------|------------|
| Add workflow state store contracts for thread turn gate and linkage records. | Planned |
| Wire runtime writes to canonical validators in `src/workflow/record_contracts`. | Planned |
| Implement resume from failed turn behavior using persisted deterministic state. | Planned |
| Add idempotent write semantics for retry and replay scenarios. | Planned |
| Add typed state store errors for missing invalid and incompatible records. | Planned |

**Exit criteria**:
- workflow state records persist and rehydrate with canonical schema validation
- resume from failed turn follows deterministic continuation rules
- state persistence handles replay with no duplicate semantic side effects

**Key files and seams**:
- `src/workflow/record_contracts.rs`
- `src/workflow/record_contracts/thread_turn_gate_record.rs`
- `src/workflow/record_contracts/prompt_link_record.rs`
- `src/workflow`
- `tests/integration/workflow_contracts_conformance.rs`

**Planned verification evidence**:
- compile gate: `cargo check`
- unit gate: `cargo test workflow::record_contracts`
- integration gate: `cargo test --test integration_tests integration::workflow_contracts_conformance::`

---

### Phase 5 - Prompt reference and artifact read integration

**Goal**: support per turn prompt refs by file path or artifact id and verified artifact reads for downstream turns.

**Source docs**:
- [Turn Manager Functional Specification](README.md)
- [Turn Manager Technical Specification](technical_spec.md)
- [Workflow Metadata Contracts Plan](../metadata_contracts/PLAN.md)

| Task | Completion |
|------|------------|
| Implement prompt ref resolver for file path and artifact id sources. | Planned |
| Integrate verified artifact read path from prompt context storage into workflow resolver. | Planned |
| Persist prompt render and output linkage records with deterministic digest references. | Planned |
| Ensure retry path reuses stable input snapshot and stable prompt linkage identity. | Planned |
| Add deterministic failure behavior for digest mismatch missing artifact and invalid prompt ref type. | Planned |

**Exit criteria**:
- artifact id prompt refs execute successfully under workflow runtime
- verified reads reject digest mismatch deterministically
- linkage records remain stable under replay

**Key files and seams**:
- `src/prompt_context/storage.rs`
- `src/prompt_context/orchestration.rs`
- `src/workflow`
- `tests/integration/generation_parity.rs`
- `tests/integration/workflow_contracts_conformance.rs`

**Planned verification evidence**:
- compile gate: `cargo check`
- unit gate: `cargo test prompt_context`
- unit gate: `cargo test workflow::resolver`
- integration gate: `cargo test --test integration_tests integration::workflow_contracts_conformance::`

---

### Phase 6 - CLI and watch adapter integration

**Goal**: expose workflow operator commands and integrate workflow scheduling into watch runtime.

**Source docs**:
- [Turn Manager Functional Specification](README.md)
- [Turn Manager Technical Specification](technical_spec.md)
- [Turn Manager Code Path Findings](code_path_findings.md)

| Task | Completion |
|------|------------|
| Add workflow command group for validate list inspect and execute actions. | Planned |
| Route CLI workflow actions through thin adapters to workflow orchestration. | Planned |
| Add watch runtime branch that schedules workflow runs for bound agents. | Planned |
| Preserve legacy frame generation scheduling for unbound agents. | Planned |
| Add command and watch integration tests for positive and failure paths. | Planned |

**Exit criteria**:
- workflow command surface executes through workflow orchestration only
- watch runtime chooses workflow path for bound agents and legacy path for unbound agents
- adapter behavior is deterministic and parity tested

**Key files and seams**:
- `src/cli/parse.rs`
- `src/cli/route.rs`
- `src/workspace/watch/runtime.rs`
- `src/api.rs`
- `tests/integration/context_cli.rs`
- `tests/integration/progress_observability.rs`

**Planned verification evidence**:
- compile gate: `cargo check`
- integration gate: `cargo test --test integration_tests integration::context_cli::`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`

---

### Phase 7 - Verification lock and readiness signoff

**Goal**: run end to end verification and publish readiness evidence for implementation complete state.

| Task | Completion |
|------|------------|
| Run compile and full test gates after all phase code changes. | Planned |
| Run focused parity checks for unbound legacy and bound workflow execution paths. | Planned |
| Validate typed deterministic error behavior for profile binding gate and state failures. | Planned |
| Confirm CLI path exception list remains unchanged from current policy baseline. | Planned |
| Publish phase completion notes and verification evidence in this plan. | Planned |

**Exit criteria**:
- all required gates pass with no unresolved high severity regressions
- workflow runtime is active for bound agents with deterministic persistence and gate behavior
- legacy one shot behavior remains stable for unbound agents

**Planned verification evidence**:
- compile gate: `cargo check`
- full suite gate: `cargo test`
- targeted gate: `cargo test --test integration_tests integration::generation_parity::`
- targeted gate: `cargo test --test integration_tests integration::workflow_contracts_conformance::`
- targeted gate: `cargo test --test integration_tests integration::context_cli::`

## Readiness Checklist

Implementation is ready for completion signoff only when all items are true:

1. Phase 1 through Phase 7 tasks are marked complete with evidence links or commands
2. workflow runtime owns bound agent orchestration and gate execution
3. canonical workflow record contracts are runtime integrated and validated
4. command and watch adapters delegate to workflow orchestration through explicit contracts
5. legacy unbound execution path remains deterministic and parity covered
