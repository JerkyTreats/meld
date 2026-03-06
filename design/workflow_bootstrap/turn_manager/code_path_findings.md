# Turn Manager Code Path Findings

Date: 2026-03-06
Scope: current code path findings for turn manager target state defined in `design/workflow_bootstrap/turn_manager/README.md`

## Intent

Describe what exists today in code for all major seams affected by the turn manager functional specification.

This document captures current behavior only.

## Source Spec

1. `design/workflow_bootstrap/turn_manager/README.md`

## Current Execution Path Snapshot

- CLI context generation routes directly to `run_generate` with node target, agent id, provider name, and frame type
- generate flow builds a node level plan and executes through frame generation queue workers
- queue delegates generation content work to generation orchestration and persists one frame output per request
- prompt and context artifacts are persisted in filesystem CAS and linked into frame metadata
- workflow domain currently publishes only workflow record schema contracts and validators

Primary seams:

- `src/cli/route.rs:1248`
- `src/context/generation/run.rs:345`
- `src/context/generation/executor.rs:78`
- `src/context/queue.rs:1054`
- `src/context/generation/orchestration.rs:18`
- `src/prompt_context/orchestration.rs:57`
- `src/workflow.rs:1`

## Spec To Code Status Matrix

Status key:

- `Present` means implemented and wired to runtime use
- `Partial` means implemented in isolated modules with no runtime owner flow
- `Missing` means no concrete code path exists

| Spec area | Status | Evidence |
|---|---|---|
| Workflow domain owns runtime registry, resolver, executor, state orchestration | Missing | `src/workflow.rs:1` exposes only record contracts module |
| Workflow profile configuration and load priority | Missing | `src/config.rs:40` has no workflows field, `src/config/merge/service.rs:16` merges only global, workspace, env |
| Agent to workflow binding with optional workflow id | Missing | `src/agent/profile/config.rs:10` has no workflow id field, `src/agent/profile/validation.rs:8` has no workflow binding validation |
| Turn ordered execution model driven by workflow profile turns | Missing | `src/context/generation/plan.rs:51` models node level plan, not workflow turns |
| Prompt reference by prompt path or artifact id in each turn | Missing | `src/agent/profile/prompt_contract.rs:13` uses agent metadata templates, no turn prompt ref input |
| Gate declaration and evaluation after each turn | Missing | no gate runtime module, only metadata write validation in `src/metadata/frame_write_contract.rs:84` |
| Durable thread, turn, gate, artifact link state records | Partial | prompt link and gate record contracts exist in `src/workflow/record_contracts`, no runtime persistence producer |
| Metadata contract binding for prompt link and gate records | Partial | contracts and validators exist in `src/workflow/record_contracts/prompt_link_record.rs:39` and `src/workflow/record_contracts/thread_turn_gate_record.rs:53` |
| Prompt and context artifact CAS linkage | Present | filesystem CAS write and verify in `src/prompt_context/storage.rs:23`, lineage preparation in `src/prompt_context/orchestration.rs:57` |
| Deterministic retry behavior with stable lineage semantics | Present | queue retry logic in `src/context/queue.rs:1015`, generation parity fixtures in `tests/integration/generation_parity.rs:337` |
| CLI and adapter delegation to workflow orchestration | Missing | no workflow command in `src/cli/parse.rs:47`, route uses context generate path in `src/cli/route.rs:1254` |

## Detailed Findings

### TM1 Workflow runtime ownership is not implemented

Observed state:

- `src/workflow.rs:1` documents workflow orchestration as out of scope
- workflow domain exports only `record_contracts`
- no registry, resolver, executor, or state store modules under `src/workflow`

Impact:

- target runtime control plane for turn manager does not exist
- runtime remains owned by context generation flow

### TM2 Workflow profile config loading does not exist

Observed state:

- root config type `MerkleConfig` has providers, agents, system, logging only
- merge service loads config file and environment overlays only
- workspace file source reads `config/config.toml` and `config/<env>.toml`

Primary seams:

- `src/config.rs:40`
- `src/config/merge/service.rs:16`
- `src/config/sources/workspace_file.rs:15`
- `src/config/sources/global_file.rs:20`

Impact:

- no workflow profile artifact discovery
- no workflow id conflict handling by priority layer

### TM3 Agent workflow binding is absent

Observed state:

- `AgentConfig` has agent id, role, prompt fields, metadata only
- validation checks prompt requirements but not workflow binding
- agent registry loading copies prompts and metadata only

Primary seams:

- `src/agent/profile/config.rs:10`
- `src/agent/profile/validation.rs:8`
- `src/agent/registry.rs:63`

Impact:

- no zero or one workflow binding enforcement
- runtime selection cannot pivot from agent id to workflow id

### TM4 Runtime execution is node batch generation, not turn workflow execution

Observed state:

- `run_generate` builds a node scoped generation plan
- plan items are node id plus agent plus provider plus frame type
- executor runs by levels and failure policy
- queue request represents one node generation attempt

Primary seams:

- `src/context/generation/run.rs:345`
- `src/context/generation/plan.rs:39`
- `src/context/generation/executor.rs:78`
- `src/context/queue.rs:105`

Impact:

- no thread id and no turn sequence contract in runtime
- no per turn input refs and output refs model

### TM5 Prompt reference model is agent template driven only

Observed state:

- prompt contract reads `system_prompt`, `user_prompt_file`, `user_prompt_directory` from agent metadata
- prompt collection chooses file or directory template by node type
- no runtime input for prompt file path reference or prompt artifact id reference

Primary seams:

- `src/agent/profile/prompt_contract.rs:8`
- `src/context/generation/prompt_collection.rs:10`

Impact:

- prompt source cannot vary by workflow turn definition
- no declarative prompt ref model exists

### TM6 Gate model in spec is missing from runtime

Observed state:

- no gate runtime registry or gate evaluator module
- no gate config schema in config domain
- only active guard rails are frame metadata contract checks and retryability rules

Primary seams:

- `src/metadata/frame_write_contract.rs:84`
- `src/context/queue.rs:1080`

Impact:

- no post turn gate execution
- no gate record persistence path

### TM7 Durable workflow state stores are not present

Observed state:

- durable stores today are node store, frame storage, and head index
- telemetry store persists progress events only
- no storage module for thread record, turn record, or gate record

Primary seams:

- `src/cli/route.rs:64`
- `src/api.rs:36`
- `src/telemetry/events.rs:7`

Impact:

- turn manager state machine cannot resume from persisted workflow state

### TM8 Prompt context CAS is implemented and wired into generation writes

Observed state:

- prompt context artifact storage writes bytes by blake3 digest path
- lineage preparation persists four artifacts and builds prompt link contract
- generation orchestration calls lineage preparation before provider execution and frame write

Primary seams:

- `src/prompt_context/storage.rs:23`
- `src/prompt_context/orchestration.rs:57`
- `src/context/generation/orchestration.rs:50`

Impact:

- target artifact linkage primitive already exists
- deterministic prompt link id and digests are available for workflow consumption

### TM9 Workflow record contracts exist but are not runtime integrated

Observed state:

- workflow domain has canonical schema version checks and id validation helpers
- prompt link record builder maps from metadata prompt link contract
- integration tests validate contracts and typed errors
- search shows no runtime producer call sites outside tests

Primary seams:

- `src/workflow/record_contracts.rs:1`
- `src/workflow/record_contracts/prompt_link_record.rs:39`
- `src/workflow/record_contracts/thread_turn_gate_record.rs:53`
- `tests/integration/workflow_contracts_conformance.rs:12`

Impact:

- contract layer is ready as a foundation
- runtime persistence and consumption still missing

### TM10 CAS read path is not used by runtime orchestration

Observed state:

- storage exposes `read_verified`
- code search shows call sites only in storage tests
- generation runtime currently writes artifacts and emits lineage events but does not rehydrate artifacts for future turns

Primary seams:

- `src/prompt_context/storage.rs:69`
- `src/prompt_context/storage.rs:133`
- `src/context/generation/orchestration.rs:63`

Impact:

- multi turn replay and input ref by artifact id is not available

### TM11 CLI has no workflow surface

Observed state:

- command enum has `Scan`, `Workspace`, `Status`, `Validate`, `Watch`, `Agent`, `Provider`, `Init`, `Context`
- no `Workflow` command group
- route maps context generate and regenerate directly to generation run

Primary seams:

- `src/cli/parse.rs:47`
- `src/cli/route.rs:1248`

Impact:

- no operator path to validate, list, inspect, or run workflow profiles

### TM12 Watch mode remains context frame generation only

Observed state:

- watch daemon can start frame generation queue when auto generation is enabled
- affected nodes are processed through `ensure_agent_frames_batched`
- this path ensures context frames and uses agent id fan out model

Primary seams:

- `src/workspace/watch/runtime.rs:49`
- `src/workspace/watch/runtime.rs:345`
- `src/api.rs:528`

Impact:

- no workflow bound turn scheduling for watch mode

## Existing Positive Baselines

### B1 Deterministic generation orchestration and retry behavior

- level based generation executor and queue retry paths are implemented
- parity fixtures validate success, retryable failure, and non retryable failure behavior

Primary seams:

- `src/context/generation/executor.rs:78`
- `src/context/queue.rs:1015`
- `tests/integration/generation_parity.rs:337`

### B2 Metadata write contract enforcement is centralized

- required keys, mutability, budget, and forbidden key policies are enforced at shared write boundary
- queue and direct write paths are covered by integration tests

Primary seams:

- `src/metadata/frame_write_contract.rs:84`
- `tests/integration/frame_queue.rs:1024`
- `tests/integration/context_api.rs:605`

### B3 Prompt context artifact lineage foundation is available

- artifacts are content addressed and verified
- lineage contract is generated before frame write
- prompt link id and digest metadata are present for downstream linking

Primary seams:

- `src/prompt_context/storage.rs:23`
- `src/prompt_context/orchestration.rs:65`
- `src/context/generation/orchestration.rs:97`

### B4 Canonical workflow record contracts are published

- thread turn gate and prompt link contracts are versioned and validated
- validator behavior has integration coverage

Primary seams:

- `src/workflow/record_contracts/thread_turn_gate_record.rs:53`
- `src/workflow/record_contracts/prompt_link_record.rs:58`
- `tests/integration/workflow_contracts_conformance.rs:13`

## Gap Summary For Turn Manager Implementation Planning

1. add workflow profile config model and loader with source priority conflict policy
2. add workflow id binding to agent config plus validation path
3. implement workflow runtime components under `src/workflow` for registry, resolver, executor, and state orchestration
4. introduce turn and gate execution contracts and durable state persistence for thread, turn, gate, and prompt link records
5. integrate workflow record contracts into runtime persistence and read paths
6. add prompt ref resolver for file path refs and artifact id refs
7. add gate evaluator system and gate failure policy handling
8. add CLI workflow surface for validation, introspection, and execution
9. evolve watch integration to schedule workflow turns when workflow binding exists
