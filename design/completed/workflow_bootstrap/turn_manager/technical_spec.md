# Turn Manager Technical Specification

Date: 2026-03-06
Status: active

## Intent

Define one executable technical specification for turn manager that unifies desired behavior and verified code reality.

This document is normative for implementation and verification.

## Source Synthesis

This specification synthesizes:

- `design/completed/workflow_bootstrap/turn_manager/README.md`
- `design/completed/workflow_bootstrap/turn_manager/code_path_findings.md`
- `design/completed/workflow_bootstrap/metadata_contracts/technical_spec.md`

## Scope

- workflow profile configuration model
- workflow profile loading and validation
- explicit agent to workflow binding with zero or one workflow id per agent
- deterministic turn execution with declared order and gate checks
- durable thread turn gate and artifact linkage persistence
- operator surface for workflow validate inspect and execute
- watch mode integration for workflow bound agents

## Non Goals

- multi workflow binding per agent
- remote profile distribution
- dynamic code execution from profile content

## Domain Ownership

Ownership contract:

- `src/workflow` owns workflow profile registry resolution orchestration and workflow state records
- `src/context` owns context frame retrieval and frame write contracts
- `src/prompt_context` owns prompt render and context payload artifact storage
- `src/metadata` owns metadata key descriptors mutability policy visibility policy and budget policy
- `src/cli` and `src/tooling` remain thin adapters that delegate to workflow domain

Boundary rules:

- workflow orchestration lives in `src/workflow`
- workflow domain consumes only public contracts from peer domains
- workflow domain does not redefine metadata schemas owned by metadata
- runtime cross domain calls use explicit contracts only

## Code Reality Snapshot

Current status from code path findings:

| Spec Area | Status | Verified Evidence |
|---|---|---|
| workflow runtime ownership in workflow domain | Missing | `src/workflow.rs` exports record contracts only |
| workflow profile config model and loader | Missing | `src/config.rs` has no workflow profile field |
| agent to workflow binding | Missing | `src/agent/profile/config.rs` has no workflow id field |
| turn ordered runtime execution | Missing | `src/context/generation/plan.rs` is node plan based |
| per turn prompt reference model | Missing | `src/agent/profile/prompt_contract.rs` is agent template based |
| gate declaration and evaluation runtime | Missing | no gate runtime module in workflow domain |
| durable thread turn gate stores | Missing | no workflow state store module |
| canonical workflow record validators | Partial | `src/workflow/record_contracts` present not runtime wired |
| prompt and context artifact lineage | Present | `src/prompt_context/storage.rs` and orchestration path active |
| deterministic retry baseline | Present | `src/context/queue.rs` retry semantics test covered |
| CLI workflow command surface | Missing | `src/cli/parse.rs` has no workflow command group |
| watch mode workflow scheduling | Missing | `src/workspace/watch/runtime.rs` fans out frame generation |

Implication:

- this spec introduces a new runtime control plane in workflow domain
- existing context generation remains compatibility path until workflow binding is enabled

## Functional Requirements

### FR1 Workflow Profile Artifacts

Runtime supports user owned workflow profiles with deterministic load priority.

Required load priority:

1. workspace workflow profiles
2. user config workflow profiles
3. built in default workflow profiles

Conflict policy:

- first profile by workflow id wins by priority order
- duplicate workflow id in same priority layer fails validation

### FR2 Workflow Profile Schema

Required top level fields:

- `workflow_id`
- `version`
- `title`
- `description`
- `thread_policy`
- `turns`
- `gates`
- `artifact_policy`
- `failure_policy`

Required thread policy fields:

- `start_conditions`
- `dedupe_key_fields`
- `max_turn_retries`

Required turn fields:

- `turn_id`
- `seq`
- `title`
- `prompt_ref`
- `input_refs`
- `output_type`
- `gate_id`
- `retry_limit`
- `timeout_ms`

Required gate fields:

- `gate_id`
- `gate_type`
- `required_fields`
- `rules`
- `fail_on_violation`

Required artifact policy fields:

- `store_output`
- `store_prompt_render`
- `store_context_payload`
- `max_output_bytes`

Required failure policy fields:

- `mode`
- `resume_from_failed_turn`
- `stop_on_gate_fail`

Optional extension fields:

- `thread_profile`
- `target_agent_id`
- `target_frame_type`
- `final_artifact_type`

### FR3 Agent Binding

Agent binding is explicit and optional.

Rules:

- agent with no `workflow_id` uses existing one shot frame generation path
- agent with `workflow_id` uses workflow runtime
- invalid `workflow_id` fails agent profile validation for write capable agents
- each agent binds to zero or one workflow id

### FR4 Prompt Reference Resolution

Turn `prompt_ref` supports:

- local file path reference
- content addressed artifact id reference

Runtime behavior:

- resolve prompt source for each turn
- render prompt with turn inputs
- persist prompt render artifact when policy allows
- emit prompt linkage metadata for downstream record contracts

### FR5 Turn Execution Semantics

Runtime executes turns by stable ascending `seq`.

For each turn:

1. resolve declared input refs from workflow state and artifact store
2. resolve and render prompt
3. execute provider call
4. validate output shape for turn output type
5. persist output artifact under artifact policy
6. evaluate gate assigned by `gate_id`
7. persist turn record and gate record

Failure handling:

- respect per turn retry limit
- respect workflow failure policy for gate failure and execution failure
- preserve deterministic input snapshot for all retries

### FR6 Gate System

Gate behavior is profile declared and post turn.

Requirements:

- gate registry with known gate types
- gate evaluator with pure deterministic evaluation for same inputs
- gate result persistence into canonical gate record
- strict stop behavior when configured by failure policy

### FR7 Durable State Records

Workflow runtime persists deterministic state records:

- thread records
- turn records
- gate records
- prompt link and output link records

Record contracts:

- runtime consumes canonical validators from `src/workflow/record_contracts`
- runtime does not redefine schema structs already owned by canonical contracts
- linkage uses content addressed ids and digests

### FR8 CLI And Adapter Surface

Operator surface adds workflow commands for:

- validate profile sets
- list resolved workflow ids with source
- inspect profile details
- execute workflow run for target inputs

Routing rules:

- adapter parses and validates request shape
- adapter delegates execution to workflow orchestration

### FR9 Watch Integration

Watch runtime behavior:

- agent with workflow binding schedules workflow run path
- agent with no workflow binding stays on existing frame generation path
- scheduling remains deterministic for identical file change events

## Runtime Architecture

### A1 Registry

Responsibilities:

- discover workflow profiles from all sources
- validate schema and cross references
- expose lookup by workflow id plus source provenance

### A2 Resolver

Responsibilities:

- resolve agent to workflow binding
- resolve per turn prompt refs and input refs
- resolve artifact reads via verified prompt context storage contracts

### A3 Executor

Responsibilities:

- run turn loop in declared order
- enforce retry and failure policy
- call gate evaluator and branch by result

### A4 State Store

Responsibilities:

- persist and load thread turn gate prompt linkage records
- support resume for failed or interrupted workflow runs
- preserve deterministic resume semantics by stable record ids

### A5 Compatibility Adapter

Responsibilities:

- preserve existing context generate path for non workflow agents
- provide explicit migration seam from old orchestration path to workflow runtime

## Determinism Contracts

- turn order is stable by `seq`
- gate evaluation is pure for identical inputs
- artifact ids are content addressed
- retries reuse same input snapshot contract
- persisted record ids are deterministic from workflow id thread identity turn id and attempt index

## Validation Rules

Profile load fails on:

- duplicate `workflow_id` in same priority layer
- unresolved prompt reference
- duplicate turn `seq`
- unknown `gate_type`
- unresolved `gate_id` reference from a turn

Runtime fails on:

- missing required output fields for turn output type
- strict gate failure when stop policy is enabled
- artifact or metadata budget overrun

## Execution Tracks

### T1 Workflow Profile Loader

Deliverables:

- config model for workflow profile sources
- source priority merge and conflict handling
- schema validation with typed deterministic errors

Acceptance:

- deterministic profile set for identical config inputs
- source provenance visible in inspection command output

### T2 Agent Binding Integration

Deliverables:

- optional `workflow_id` in agent config
- validation rules for bound workflow id resolution
- registry integration for binding lookup

Acceptance:

- unbound agents preserve legacy behavior
- bound agents route to workflow runtime

### T3 Workflow Runtime Core

Deliverables:

- workflow registry resolver executor modules under `src/workflow`
- deterministic turn loop with retry policies
- gate evaluation execution path

Acceptance:

- declared turn sequence executes in stable order
- retry and fail fast behaviors match profile policy

### T4 Durable Workflow State

Deliverables:

- workflow state store for thread turn gate and linkage records
- resume behavior for interrupted runs
- canonical contract validators wired into writes and reads

Acceptance:

- resume from failed turn is deterministic
- records pass canonical validation in runtime and tests

### T5 Prompt Ref And Artifact Read Integration

Deliverables:

- prompt ref resolver for file path and artifact id
- verified artifact read integration for downstream turns
- deterministic linkage from prompt ref to persisted prompt link record

Acceptance:

- artifact id prompt refs execute successfully
- verified reads reject digest mismatch deterministically

### T6 CLI And Watch Integration

Deliverables:

- workflow command group with validate list inspect execute
- route delegation into workflow runtime orchestration
- watch scheduling path for workflow bound agents

Acceptance:

- command surface can run end to end workflow execution
- watch mode uses workflow path when workflow binding exists

### T7 Verification Lock

Deliverables:

- integration coverage for profile loading binding execution gate behavior and state persistence
- parity coverage for legacy path and workflow bound path
- deterministic failure assertions for all typed runtime errors

Acceptance:

- all verification gates below pass
- no unresolved high severity regression from prior generation flows

## Verification Gates

Characterization gates:

- legacy context generation behavior remains stable for unbound agents
- queue retry semantics remain unchanged for legacy path

Contract gates:

- profile validation failures are typed and deterministic
- agent binding errors are typed and deterministic
- gate failures follow declared failure policy

State gates:

- thread turn gate and linkage records persist with canonical schema
- resume behavior reproduces deterministic turn continuation

Artifact gates:

- prompt and context artifact reads use verified digest path
- prompt linkage ids and digests are stable under replay

Adapter gates:

- CLI workflow commands delegate only to workflow orchestration
- watch runtime chooses correct path by workflow binding presence

## Completion Criteria

Turn manager technical scope is complete when all statements are true:

1. T1 through T7 deliverables are implemented and verified
2. workflow runtime owns turn orchestration for workflow bound agents
3. canonical workflow record contracts are runtime integrated
4. workflow command surface and watch integration are active
5. legacy one shot path remains available for unbound agents with no behavioral regression
