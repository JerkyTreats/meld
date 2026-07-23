# Docs Freshness Flywheel Implementation Guide

Date: 2026-06-22
Status: historical build guide, superseded for runtime completion
Scope: reviewed implementation handoff for one executable `docs_freshness` flywheel

Authority: [Runtime Completion Ground Map](runtime_completion_ground_map.md) and [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) replace this guide for current implementation. Its source inventory may be reused as evidence, but its activation system, one-turn proof, and unbounded package-execution assumptions are not build authority.

## Purpose

This guide turns the docs freshness flywheel scope into build-ready domain instructions.

It is based on current code analysis and the domain spec skeleton. Current implementation evidence is listed separately from target design so implementation can proceed without confusing existing gaps for intended behavior.

The goal is one durable flywheel turn:

```text
activation config
-> directive and seed agent bootstrap
-> low belief and active goal
-> planning maintains ordered task network work
-> workspace scan satisfies docs writer dependency
-> task dispatch
-> publication
-> evidence ingestion
-> belief revision
-> satisfaction mutation
-> reopen proof
```

## Source Inputs

- `design/plan/integration/docs_freshness_flywheel_next_iteration_report.md`
- `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md`
- Current code analysis from domain agents and local source inspection
- `AGENTS.md` domain architecture and docs style rules

## Review Status

This guide is ready for independent design review. Review lanes should check spec coverage, domain ownership, clean boundaries, durability, idempotency, test sufficiency, and orchestration readiness before buildout starts.

## Architecture Alignment

This slice follows the cognitive architecture as an open-world loop, but only implements the smallest path that current code can support.

The full intent is preserved through these boundaries:

- events remain the shared temporal substrate
- world model graph and belief own current state, evidence, confidence, and planner-facing belief views
- world model agents own normative curation of goals from belief
- execution owns goals, planning, task network graph mutation, dispatch, provider work, and outcome publication
- the task network is the executable plan, not an artifact beside the plan
- root runtime assembly and supervisor wire domains and lifecycle only

The slice deliberately defers causation, regime, dynamic spawned agents, synthesis escalation, broad sensory observation, plan repair, and cost-aware plan transitions. It uses existing docs writer package execution, existing belief and goal primitives, existing publication mechanics, and narrow adapters where current functionality is already present.

Planning must therefore stay shaped like current execution planning: read active goals and planner-facing world state, decompose through method and package bridges, submit task network graph commands, and let the task network own readiness and execution state.

## Build Order

Shared contracts must land before parallel domain work.

1. Activation config and validated activation DTOs.
2. Directive record, seed agent directive link, curation rule record, and bootstrap receipt.
3. Workspace scan request, scan receipt identity, scan artifacts, and `local_io` capability class.
4. Goal command and mutation receipts with content checked replay.
5. Planning package bridge contracts that preserve full `DomainObjectRef`.
6. Runtime handle tick report and semantic handle registry contracts.
7. Integration identity map for deterministic ids across reopen checkpoints.

After shared contracts are stable, these areas can proceed in parallel:

- workspace scan capability core and root adapter
- world model bootstrap stores and runtime facade
- goal receipt hardening and active goal query port
- planning package bridge tests
- publication idempotency tests
- CLI parse and dry-run validation

These gates are sequential:

- activation before bootstrap and runtime selection
- directive and rule records before curation delivery
- active goal before planning includes `workspace_scan` task work
- planning preserves `workspace_scan` as an upstream dependency of docs writer package work when workspace state is missing or stale
- workspace scan artifacts before docs writer dispatch uses workspace refs
- package trigger bridge before task dispatch proof
- publication before evidence replay proof
- evidence revision before satisfaction proof

## Domain Map

| Domain | Owner | Primary write scope | Depends on |
| --- | --- | --- | --- |
| Product assembly and activation | root runtime assembly | config validation only | provider config and runtime registry |
| Workspace scan | workspace plus execution catalog | workspace node store and scan receipts | activation target selector and task dispatch |
| Agent bootstrap | `meld-world-model` agent | directives, agents, rules, subscriptions, activations, receipts | activation config |
| Belief runtime | `meld-world-model` belief | config snapshots, evidence, revisions, views, cursors | publication event contract |
| Agent curation and satisfaction | `meld-world-model` agent | decisions, sink receipts, satisfaction records | belief query and execution goal ports |
| Execution goals | `meld-execution` goals | goals, lifecycle, receipts | agent command shape |
| Planning and package bridge | `meld-execution` planning | planning diagnostics and task network command proposals | active goals, world state, package registry |
| Task network and dispatch | `meld-execution` task network and task runtime | claims, artifacts, outcomes, publications | scan task node, package task node, and provider access |
| Publication and event spine | `meld-execution` publication plus events | publication marks and event envelopes | task outcome publication |
| Supervisor and handles | runtime supervisor plus domain handles | lifecycle and heartbeat only | concrete handle factories |
| CLI activation | CLI adapter | no direct domain writes | activation service and runtime tooling |
| Integration proof | integration tests | test stores only | all prior contracts |

## Product Assembly And Activation Config

Owner: root product assembly.

### Current Code Anchors

- Spec skeleton defines this domain at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:52`.
- Generic runtime config lives at `src/runtime/assembly.rs:57`.
- Default runtime config disables `execution.task_dispatch` at `src/runtime/assembly.rs:263`.
- Workspace load calls `ProductRuntimeConfig::for_product_root` at `src/runtime/assembly.rs:351`.
- Supervisor handoff package is built at `src/runtime/assembly.rs:460`.
- Runtime registry is static metadata at `src/runtime/assembly.rs:532`.
- Repository config has providers and storage but no activation block at `src/config.rs:40`.
- Runtime CLI delegates through `src/runtime/tooling.rs:159`, `src/runtime/tooling.rs:198`, and `src/runtime/tooling.rs:496`.

### Current State

The product runtime assembly shell can resolve a product root, describe runtime state without opening stores, open product stores for runtime handoff, build ports, and fail before supervisor start when an enabled runtime requires unavailable provider access.

It does not carry a docs freshness activation file, activation DTO validation, provider binding refs, target selector validation, directive identity validation, branch scope, perspective policy, belief family config ref, curation threshold rule config, required artifact type, publication event mapping config, task network id, frame type, or owner-scoped domain factory inputs.

### Spec Gaps

- Missing `DocsFreshnessActivationConfig`.
- Missing activation file loader.
- Missing validated activation DTO and diagnostics.
- Missing provider binding resolution from repository provider config.
- Missing branch scope, perspective policy, belief family config ref, curation threshold rule config, required artifact type, publication event mapping config ref, task network id, and frame type validation.
- Missing activation runtime id selection.
- Missing dry-run activation path that validates without opening stores.
- Assembly diagnostics exist, but activation validation is not populating them.

### Implementation Guide

1. Add `DocsFreshnessActivationConfig` and `ValidatedDocsFreshnessActivation` in the root runtime assembly boundary.
2. Add an activation file loader owned by product assembly. The loader may support TOML, YAML, or JSON, but it must parse into the same DTO.
3. Accept workspace root, repository config, activation file path, and CLI overrides. CLI overrides patch the DTO before validation.
4. Resolve relative paths in assembly only.
5. Validate `subject_ref`, `branch_scope`, `perspective_policy`, `belief_family_config_ref`, `directive_id`, `seed_agent_id`, `curation_rule_id`, curation threshold rule config or ref, required artifact type, publication event mapping config ref, `method_id`, `task_package_id`, `task_network_id`, `provider_binding_ref`, `frame_type`, target selector, force policy, and enabled runtime ids before stores open.
6. Convert validated activation into `ProductRuntimeConfig` without using `for_product_root` as the activated path.
7. Build `ProductActivationRuntimeInputs` as owner-scoped packages. Include no raw activation document and no durable writes.
8. Hand only the relevant typed package to each runtime factory.
9. Preserve `describe` as the dry-run path.
10. Add assembly tests for valid activation file, missing activation fields, invalid provider binding, required provider config absence, unsupported file format, owner-scoped input separation, and no semantic store writes.

### Contracts To Add Or Change

- `DocsFreshnessActivationConfig`
- `DocsFreshnessActivationFile`
- `ValidatedDocsFreshnessActivation`
- `ActivationDiagnostics`
- `ProviderBindingRef`
- `BranchScope`
- `PerspectivePolicy`
- `BeliefFamilyConfigRef`
- `CurationRuleConfigRef`
- `RequiredArtifactType`
- `PublicationEventMappingConfigRef`
- `TaskNetworkId`
- `FrameType`
- `TargetSelector`
- `ForcePolicy`
- `ProductActivationRuntimeInputs`
- owner-scoped world model belief runtime input
- owner-scoped world model agent runtime input
- owner-scoped execution runtime input
- owner-scoped publication runtime input
- activated runtime config builder

### Durability And Idempotency

Activation file load and validation are pure. Assembly must not write directive, agent, goal, belief, task, publication, or event records.

Duplicate load of the same activation file and overrides must produce the same validated DTO and the same runtime factory input packages.

Config drift is detected by owning domains during bootstrap and replay. Assembly can carry hashes and ids, but it does not decide semantic conflicts.

Runtime factories must not receive a handle to the activation file, the raw parsed document, or source format metadata. They receive typed values only.

### Boundary Rules

Root assembly may resolve paths, validate config shape, derive provider availability, select runtime ids, and build supervisor inputs.

Root assembly must not query semantic stores or create semantic records.

CLI must remain an adapter over this service.

### Verification

- unit tests in `src/runtime/assembly.rs`
- CLI parse tests beside `src/cli/parse.rs:846`
- static scan for `ProductRuntimeConfig::for_product_root`
- integration proof dry-run checkpoint

### Risks

The main risk is letting activation become shortcut bootstrap logic. Keep activation as validation and handoff only.

## Workspace Scan Capability

Owner: workspace domain plus execution capability catalog.

### Current Code Anchors

- Spec skeleton defines the capability at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:122`.
- Scan state exists at `src/workspace/types.rs:37`.
- Scan assessment happens at `src/workspace/commands.rs:102`.
- CLI scan builds and writes the tree at `src/workspace/commands.rs:570`.
- Node writes occur at `src/workspace/commands.rs:621`.
- Store flush occurs at `src/workspace/commands.rs:653`.
- Existing no-force early return is at `src/workspace/commands.rs:594`.
- Current scan emits workspace facts through progress at `src/workspace/commands.rs:660` and `src/workspace/commands.rs:806`.
- Current workspace capability constants start at `src/workspace/capability.rs:27`.
- Capability registration omits scan at `src/workflow/task_path.rs:13`.
- Execution class lacks `local_io` at `crates/meld-execution/src/capability/contracts.rs:128`.
- Execution context has read node methods but no scan write port at `src/execution/ports.rs:328`.

### Current State

Workspace scan is a CLI service. It computes a Merkle tree, writes `NodeRecord` values, flushes the store, optionally syncs ignore state, emits progress and workspace fact envelopes, and returns a human string.

The capability framework can host domain-owned invokers and emitted artifacts, but `workspace_scan` is not registered. Workspace scan writes need a workspace-owned contract adapted at the root, not a generic execution-owned node write port.

### Spec Gaps

- Missing `workspace_scan` capability contract.
- Missing capability metadata for version `1`, owning domain `workspace`, scope kind `workspace`, and scope ref kind `workspace_root`.
- Missing structured scan request, scan policy, ignore policy, target selector, summary artifact, object refs, and observed refs.
- Missing durable scan request or receipt identity separate from workspace snapshot identity.
- Existing output is a string.
- Existing scan can emit envelopes inside scan flow.
- Ignore sync is a hidden write unless modeled in scan policy.
- Scan artifact schemas need stable ids that do not depend on fixture constants.

### Implementation Guide

1. Add scan DTOs under workspace ownership.
2. Extract scan effect core from `WorkspaceCommandService::scan`.
3. Keep CLI telemetry in tooling and command wrappers.
4. Build task artifacts separately from scan persistence.
5. Add `ExecutionClass::LocalIo` or a documented compatibility mapping.
6. Define workspace-owned scan write and flush contracts, then adapt those contracts at the root for capability invocation.
7. Add `WorkspaceScanCapability` under `src/workspace/capability.rs` or child files under `src/workspace/capability/`.
8. Keep new scan extraction under `src/workspace/scan.rs` or `src/workspace/capability/scan.rs` without expanding legacy `mod.rs` modules.
9. Register the invoker in `src/workflow/task_path.rs`.
10. Plan `workspace_scan` as a task network capability task before docs writer package work when workspace state is missing or stale.
11. Update package or method fixtures only after catalog validation accepts `workspace_scan`.

### Contracts To Add Or Change

- capability type id `workspace_scan`
- capability version `1`
- owning domain `workspace`
- scope kind `workspace`
- scope ref kind `workspace_root`
- input artifact `workspace_scan_request` schema `1`
- output artifacts `workspace_scan_summary`, `workspace_snapshot_ref`, `workspace_root_node_ref`, and `workspace_observed_node_refs`
- `WorkspaceScanReceipt`
- effect specs for filesystem read, node store write, and node store flush
- execution class support for `local_io`
- workspace-owned scan write and flush contracts

### Durability And Idempotency

Use root hash as workspace snapshot identity.

Use a separate scan request or receipt id for scan execution identity. Derive it from canonical workspace root, root hash, target selector, scan policy, ignore policy, force policy, and artifact schema versions.

Same request id and same request content replays. Same root hash and no force returns existing snapshot refs only when the scan request identity also matches. Force with unchanged content writes equivalent node records under the same ids.

`WorkspaceScanReceipt` includes receipt id, request hash or canonical request body, workspace root ref, target selector, scan policy hash, ignore policy hash, force policy, snapshot ref, root node ref, observed node refs artifact ref, source task id, status, completed sequence, and error summary when failed.

Session id is lineage or telemetry context, not scan identity.

### Boundary Rules

Workspace owns scan semantics, node materialization, workspace refs, and scan artifact construction.

Execution owns generic capability contracts, catalog validation, invocation, and artifact handoff only. Execution must not own generic workspace node write APIs.

Root adapters may connect the execution capability invocation to workspace-owned scan contracts.

The capability must not call providers, write docs, mutate belief, mutate goals, or append canonical events.

Do not add `mod.rs`.

### Verification

- contract validation tests for `workspace_scan`
- catalog metadata tests for version, owner, scope kind, and scope ref kind
- scan core tests for missing, current, stale, and forced unchanged state
- capability invocation tests proving full object refs
- no direct event append assertion
- scan receipt id and artifact stable id tests
- regression tests for progress observability and traversal replay

### Risks

Changing execution class serialization may affect fixtures. Root adapter expansion may break fake contexts. Ignore sync needs explicit scan policy ownership.

## World Model Agent Bootstrap

Owner: `meld-world-model` agent domain.

### Current Code Anchors

- Spec skeleton defines bootstrap at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:187`.
- `AgentRecord` embeds directive text at `crates/meld-world-model/src/agent/contracts.rs:101`.
- `SeedAgentRegistration` has no directive id at `crates/meld-world-model/src/agent/contracts.rs:451`.
- Seed registration replays by id without conflict checks at `crates/meld-world-model/src/agent/registration.rs:18`.
- Subscriptions are idempotent by natural key at `crates/meld-world-model/src/agent/subscription.rs:21`.
- Agent store lacks directive, rule, and bootstrap receipt trees at `crates/meld-world-model/src/agent/store.rs:15`.
- Docs freshness fixture keeps seed and rule config in memory at `tests/integration/docs_freshness_fixture.rs:95`.
- Reopen setup manually registers agent and subscription at `tests/integration/docs_freshness_reopen_contract.rs:508`.

### Current State

Seed agent and subscription persistence exist. Directive identity and curation rule identity are not durable first-class records. `AgentActivationRecord` and operational marking exist, but docs freshness bootstrap does not use them as its durable activation and operational status contract yet.

### Spec Gaps

- Missing `DirectiveRecord`.
- Missing `AgentCurationRuleRecord`.
- Missing bootstrap command, report, and receipt.
- Missing directive id on seed registration.
- Existing `AgentActivationRecord` is not wired into bootstrap operational status.
- Existing replay does not detect same id with changed subject, directive, scope, or perspective.

### Implementation Guide

1. Add directive, curation rule, bootstrap command, bootstrap report, and bootstrap receipt contracts in `agent/contracts.rs`.
2. Add `directive_id` to seed registration and agent record.
3. Preserve a compatibility read path for existing embedded directive text.
4. Extend `AgentStore` with directive, rule, and bootstrap receipt trees.
5. Add conflict-aware put methods.
6. Change seed registration replay to compare durable fields.
7. Add `AgentBootstrapRuntime` facade that writes directive, agent, rule, subscription, `AgentActivationRecord`, bootstrap receipt, marks `AgentRecord.status` operational after active subscription confirmation, and flushes.
8. Replace fixture manual registration with the bootstrap facade.
9. Wire activation config into bootstrap before curation runtimes run.

### Contracts To Add Or Change

- `DirectiveRecord`
- `AgentCurationRuleRecord`
- `AgentBootstrapCommand`
- `AgentBootstrapReport`
- `AgentBootstrapReceipt`
- `AgentActivationRecord`
- `SeedAgentRegistration.directive_id`
- conflict errors that name the field

### Durability And Idempotency

Same directive id and same text replays. Same directive id and different text fails. Same seed agent id with changed subject or directive fails. Same rule id with changed threshold fails. Same subscription natural key replays.

Bootstrap receipt turns repeated activation into confirmation, not rewrite. `AgentActivationRecord` records the activation attempt, while `AgentRecord.status` records the current operational state.

### Boundary Rules

Bootstrap may write only agent-owned records. It must not evaluate belief confidence, submit goal commands, decide satisfaction, write task networks, call providers, or append events.

### Verification

- first bootstrap write
- exact replay
- directive text conflict
- seed subject conflict
- rule threshold conflict
- operational status requires active subscription
- docs freshness setup uses bootstrap facade

### Risks

Migrating from embedded directive text to directive id needs compatibility handling for stored agents.

## World Model Belief Runtime

Owner: `meld-world-model` belief domain.

### Current Code Anchors

- Spec skeleton defines belief runtime at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:256`.
- Belief contracts exist at `crates/meld-world-model/src/belief/contracts.rs:116`, `crates/meld-world-model/src/belief/contracts.rs:188`, `crates/meld-world-model/src/belief/contracts.rs:214`, `crates/meld-world-model/src/belief/contracts.rs:293`, `crates/meld-world-model/src/belief/contracts.rs:420`, and `crates/meld-world-model/src/belief/contracts.rs:444`.
- Config hashing and validation are at `crates/meld-world-model/src/belief/config.rs:58`.
- Assessment persists snapshot, evidence, assignments, lease, revision, view, and flush at `crates/meld-world-model/src/belief/runtime.rs:89`.
- Promoted ingestion is idempotent at `crates/meld-world-model/src/belief/ingestion.rs:123`.
- Belief store trees start at `crates/meld-world-model/src/belief/store.rs:32`.
- Planner projection reads public query facades at `crates/meld-world-model/src/planner/query.rs:12`.
- Root runtime currently maps task success events into promoted evidence at `src/runtime/ports.rs:71`.
- Reopen proof still injects synthetic belief view near `tests/integration/docs_freshness_reopen_contract.rs:788`.

### Current State

The belief runtime is mostly present. It loads config, normalizes graph anchors and promoted records, computes revisions, writes views, and exposes planner-safe reads.

Docs task evidence replay exists as a root runtime port over caller-supplied event windows. It does not own a durable replay cursor.

### Spec Gaps

- Missing explicit `BeliefAssessmentRequest`.
- Missing bounded dirty-key tick report.
- Missing durable event replay cursor.
- Config snapshot put overwrites by hash without conflict checking.
- Graph assessment path does not use the same conflict-safe evidence writes as promoted ingestion.
- Missing graph anchor is fatal, so configured low confidence for missing evidence is incomplete.
- Docs event mapping hardcodes source kind and probabilities in root runtime port.
- Root runtime adapter currently owns too much evidence mapping behavior.

### Implementation Guide

1. Add `BeliefAssessmentRequest`, `BeliefDirtyKeyTickRequest`, and `BeliefRuntimeTickReport`.
2. Add durable replay cursor records in `BeliefStore`.
3. Advance cursor only after durable ingestion, reassessment, and durable no-op receipts.
4. Harden config snapshots with conflict-aware `put_config_snapshot_once`.
5. Refactor assessment behind `BeliefAssessmentRequest`.
6. Use conflict-safe evidence writes in graph assessment.
7. Implement bounded dirty-key ticking with budget, committed count, retryable errors, fatal errors, and output checkpoint.
8. Move docs replay to a belief runtime handle that loads and persists cursor.
9. Put event-to-evidence mapping, source kind selection, probability config, cursor advancement, and no-op receipts behind a belief-owned mapper contract.
10. Extend proofs so belief view is rebuilt from revision state after reopen.

### Contracts To Add Or Change

- `BeliefAssessmentRequest`
- `BeliefDirtyKeyTickRequest`
- `BeliefRuntimeTickReport`
- `BeliefEvidenceReplayCursorRecord`
- belief-owned docs event evidence mapper contract
- durable no-op receipt for skipped events
- durable no-op receipt for docs writer failure events
- expanded docs task evidence replay report

### Durability And Idempotency

Evidence ids and assignment ids are deterministic. Existing promoted ingestion conflict checks should become the standard for all evidence paths.

Replay cursor belongs with belief evidence replay, not test harness callers. Cursor advances after all durable writes for the replay window succeed.

### Boundary Rules

Belief may read graph through query traits, ingest promoted records, and write belief-owned records. It must not create goals, execute tasks, call providers, or publish events.

Root ports may expose event replay and event append access. They must not convert execution events into `PromotedEvidence` or own probability mapping.

### Verification

- `cargo test -p meld-world-model belief`
- `cargo test -p meld-world-model planner`
- `cargo test -p meld --test integration_tests outcome_evidence`
- `cargo test -p meld --test integration_tests docs_freshness_reopen_contract`
- new tests for config conflict, durable cursor replay, dirty-key report, missing evidence behavior, docs writer failure no-op receipt, and reopen without synthetic view writes

### Risks

Cursor ownership is the main boundary risk. If root owns it, belief replay is not domain durable. If belief reads event stores directly, world model starts depending on product runtime details.

## World Model Agent Curation And Satisfaction

Owner: `meld-world-model` agent domain.

### Current Code Anchors

- Spec skeleton defines curation and satisfaction at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:322`.
- Curation dedupe keys exist at `crates/meld-world-model/src/agent/contracts.rs:202`.
- Durable curation decisions exist at `crates/meld-world-model/src/agent/contracts.rs:329`.
- Sink receipts exist at `crates/meld-world-model/src/agent/contracts.rs:408`.
- Runtime persists decision, submits sink, records receipt, then advances cursor at `crates/meld-world-model/src/agent/runtime.rs:196`.
- Satisfaction review tick is at `crates/meld-world-model/src/agent/runtime.rs:486`.
- Agent store persists decisions idempotently at `crates/meld-world-model/src/agent/store.rs:298`.
- Goal command and mutation ports exist at `src/runtime/ports.rs:447` and `src/runtime/ports.rs:474`.
- Integration tests prove goal acceptance and satisfaction at `tests/integration/goal_acceptance.rs:334` and `tests/integration/goal_acceptance.rs:386`.

### Current State

This area is substantially implemented. Decisions, mutation commands, sink receipts, cursor advancement, and reopen proof exist. Integration setup still builds active goal and mutation sink closures from raw stores.

### Spec Gaps

- Curation rule records are not durable.
- There is no active goal query port in `src/runtime/ports.rs`.
- Subscription status is not enforced in delivery.
- There is no separate durable satisfaction review record.
- Product runtime wiring still uses fixture-level closures in reopen tests.

### Implementation Guide

1. Finish bootstrap so curation loads durable rule records by id.
2. Add execution active goal query port.
3. Add a belief query port or narrow delivery source.
4. Add subscription delivery driver that lists active subscriptions, enforces status, builds delivery, loads rule, and calls runtime.
5. Promote test sink mapping helpers into runtime adapters.
6. Add durable satisfaction review record.
7. Wire supervisor handles to ports instead of fixture closures.

### Contracts To Add Or Change

- active goal query port
- durable curation rule lookup
- satisfaction review record
- extended agent runtime report with receipt reuse and cursor counts

### Durability And Idempotency

Keep `AgentGoalCurationRuntime` as canonical because it persists decisions, records sink receipts, and advances the subscription cursor after sink success.

Avoid lower-level paths that advance after decision persistence without a sink barrier.

### Boundary Rules

Agent curation owns belief threshold interpretation and satisfaction judgment. Execution owns goal storage and lifecycle mutation. Root assembly wires ports only.

### Verification

- suspended subscription not delivered
- durable rule replay
- repeated delivery reuses sink receipt
- satisfaction replay after reopen does not duplicate mutation
- product runtime wiring avoids raw active goal store access

### Risks

Two delivery paths can diverge. Product runtime should use the sink-safe runtime facade only.

## Execution Goals

Owner: `meld-execution` goals domain.

### Current Code Anchors

- Spec skeleton defines execution goals at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:387`.
- Goal records are defined at `crates/meld-execution/src/goals/contracts.rs:7`.
- Commands are defined at `crates/meld-execution/src/goals/contracts.rs:20`.
- In-memory add replays by command id at `crates/meld-execution/src/goals/store.rs:31`.
- Persistent store trees are at `crates/meld-execution/src/goals/persistent_store.rs:18`.
- Durable active and lifecycle reads are at `crates/meld-execution/src/goals/persistent_store.rs:274`.
- Runtime ports wire commands and mutations at `src/runtime/ports.rs:140` and `src/runtime/ports.rs:429`.
- Goal acceptance tests are at `tests/integration/goal_acceptance.rs:334`.
- Product storage reopen proof is at `tests/integration/product_storage_assembly.rs:75`.

### Current State

The goals domain is separated and persistent. It supports add, lifecycle mutation, durable storage, source identity dedupe, and active goal reads.

### Spec Gaps

- Same command id with divergent content replays instead of conflict.
- Receipts store outcomes but not command bodies or hashes.
- Foreign agent mutation is not rejected by execution because the adapter drops agent ownership.
- Runtime has no goal query port.
- Modify and lifecycle mutations are not fully atomic with receipt writes.

### Implementation Guide

1. Add canonical command receipt records for add and lifecycle mutation.
2. Store command kind, command id, source identity, actor agent id, request hash or canonical body, outcome, and sequence.
3. Change replay so identical id plus identical content replays, while divergent content conflicts.
4. Introduce execution-owned mutation acceptance request that preserves agent id, goal id, mutation kind, review sequence, and dedupe key.
5. Enforce mutation ownership against stored goal record.
6. Make persistent modify and lifecycle mutation atomic across record update and receipt write.
7. Add common query facade over active goals and lifecycle records.
8. Add root runtime goal query port.

### Contracts To Add Or Change

- `GoalCommandReceipt`
- `GoalMutationReceipt`
- `GoalReplayConflict`
- `GoalMutationAcceptanceRequest`
- active goal query trait
- lifecycle query trait

### Durability And Idempotency

Exact replay must be content checked. Goal records and receipts should commit in one durable transaction per command.

Satisfied goals leave active query results but remain visible through lifecycle query after reopen.

### Boundary Rules

World model decides when to request goals and satisfaction. Execution validates command invariants and owns goal lifecycle state.

### Verification

- divergent command id conflict
- foreign agent mutation rejection
- active query excludes satisfied goals
- lifecycle query survives reopen
- product runtime port reopen proof for active and satisfied states

### Risks

Receipt hashing must be schema stable. Existing persisted outcome records need compatibility replay policy.

## Execution Planning And Method Package Bridge

Owner: `meld-execution` planning and task package adapter.

### Current Code Anchors

- Spec skeleton defines this bridge at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:442`.
- Planning runtime lowers composed plans at `crates/meld-execution/src/planning/runtime.rs:346`.
- Runtime submits `ApplyMutationSet` at `crates/meld-execution/src/planning/runtime.rs:404`.
- Generic lowerer builds `TaskDefinition` at `crates/meld-execution/src/planning/lowering.rs:423`.
- `Term::Object` collapse is documented at `crates/meld-execution/src/planning/lowering.rs:953`.
- Docs writer package is declared at `crates/meld-execution/src/task/package/docs_writer_v2.yaml:1`.
- Required package fields start at `crates/meld-execution/src/task/package/docs_writer_v2.yaml:7`.
- `WorkflowPackageTriggerRequest` is defined at `crates/meld-execution/src/task/package/contracts.rs:65`.
- Trigger validation is at `crates/meld-execution/src/task/package/prepare.rs:123`.
- Fixture defines generic `refresh_docs_v1` at `tests/integration/docs_freshness_fixture.rs:292`.
- Reopen proof bypasses lowering at `tests/integration/docs_freshness_reopen_contract.rs:599`.

### Current State

Planning can select a method, produce `ExecutionComposition`, lower operator steps into task nodes, and submit task network commands. Package code can load and validate the built-in docs writer package.

The paths are not bridged. Docs freshness planning emits generic capability work while integration injects a fixture task network mutation.

### Spec Gaps

- Planning request lacks workspace readiness, latest scan receipt, workspace snapshot ref, task network completion state, and an execution-owned package bridge request carrying task package id, provider binding, frame type, force, and target selector.
- No bridge converts `refresh_docs_v1` plus bindings into `WorkflowPackageTriggerRequest`.
- Missing provider binding cannot produce a planning diagnostic.
- Full `DomainObjectRef` is not preserved.
- Existing materialization check can skip changed package trigger fields if task id is unchanged.

### Implementation Guide

1. Add planning package bridge request, output, and diagnostic types.
2. Add planning query inputs for workspace readiness, latest scan receipt, workspace snapshot ref, accepted task network commands, and completed task network nodes.
3. Bind `refresh_docs_v1` to docs writer package through a method sidecar or bridge map.
4. Preserve full `DomainObjectRef` through package bridge.
5. Derive `node_id` or path only in an explicit workspace target adapter.
6. When workspace readiness is missing or stale, decompose the active goal into task network structure where `workspace_scan` is upstream of docs writer work.
7. Allow either one ordered graph command or later graph delta commands. The required invariant is that docs writer work is not ready until scan receipt and snapshot ref are available.
8. Implement package trigger lowerer that loads `docs_writer`, builds `WorkflowPackageTriggerRequest`, and validates it.
9. Return blocking diagnostics and no mutation for missing provider binding, missing frame type, unsupported target, or invalid package trigger.
10. Build package task nodes using package contracts and task network dependency edges.
11. Keep prompt and workspace hydration in dispatch.
12. Submit through task network command boundary with graph identity included in command identity.
13. Map root activation into an execution-owned package bridge request before entering `meld-execution`.
14. Update reopen setup to use bridge output instead of fixture mutation injection.

### Contracts To Add Or Change

- serializable package trigger proposal
- package lowering diagnostics
- execution-owned package bridge request with provider binding ref, frame type, force policy, package id, method id, task network id, and full target ref
- planning workspace readiness query
- planning scan receipt query
- planning task network completion query
- package lineage fields on task node or dedicated package task node contract
- durable planning receipt or durable command diagnostic for blocking no-mutation outcomes

### Durability And Idempotency

Use stable scan task identity from goal id, method id, task network id, workspace readiness state, full target ref, scan policy, ignore policy, and force policy.

Use stable package task identity from goal id, method id, task network id, world state frame id, package id, workflow id, full target ref, provider binding ref, frame type, force, and package spec identity.

If package work is emitted after scan completion, include latest scan receipt id and workspace snapshot ref in the package identity. If package work is emitted in the same ordered graph as scan work, encode the dependency through upstream scan task id and required artifact selectors.

Package trigger content must reach mutation set identity so changed provider, frame type, force, or target creates a new deterministic command.

### Boundary Rules

Planning may select method, validate package trigger readiness, and propose task network mutations.

Planning must not call providers, execute workflow turns, append events, or mutate belief and goal state.

Task package adapter validates package authoring and builds package task seed contracts. Dispatch owns provider access and execution.

### Verification

- `cargo test -p meld-execution planning_runtime`
- `cargo test -p meld-execution package`
- docs freshness tests for package bridge output
- planning decomposes missing or stale workspace state into scan work upstream of package work
- docs writer task is not ready until scan receipt and snapshot ref exist
- repeated planning does not duplicate accepted equivalent work
- static scan for object ref collapse
- reopen proof uses bridge output

### Risks

Fixing object refs only in the package bridge may leave generic lowering violating the same rule.

## Execution Task Network And Dispatch

Owner: `meld-execution` task network and task runtime domains.

### Current Code Anchors

- Spec skeleton defines task dispatch at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:518`.
- Docs writer package trigger request is used at `tests/integration/docs_writer_task.rs:302` and `tests/integration/docs_writer_task.rs:386`.
- Docs writer task completion is proven at `tests/integration/docs_writer_task.rs:350`.
- Package task execution calls `execute_task_to_completion` at `tests/integration/docs_writer_task.rs:413`.
- Idempotent package expansion is covered at `tests/integration/docs_writer_task.rs:541`.

### Current State

Docs writer task execution can run to completion when the package trigger is manually prepared. It requires scan state, test agent, provider config, registered workflow, catalog and registry setup, and package trigger input.

### Spec Gaps

- No route from planned package task node to existing package execution path.
- No route from planned `workspace_scan` task node to workspace-owned scan capability execution.
- No runtime task dispatch handle claim path for docs writer package tasks in the docs freshness loop.
- Provider availability policy is not connected to activation provider binding.
- Missing provider behavior is not split between invalid config and transient provider unavailability.
- Context storage and prompt storage are not explicit dispatch inputs.
- Outcome-to-publication handoff needs to be owned by task network dispatch, not test setup.

### Implementation Guide

1. Accept `workspace_scan` capability task nodes and docs writer package task nodes through the task network command boundary.
2. Add a scan dispatch path that invokes workspace-owned scan contracts through the root adapter and commits scan artifacts.
3. Add a package dispatch hydration path that reads package trigger material from task init payload.
4. Consume prior scan artifacts by stable snapshot ref, or use a narrow workspace-owned snapshot query port. Do not fall back to raw current workspace store reads.
5. Resolve context storage and prompt storage through explicit dispatch factory inputs.
6. Resolve provider binding through an execution-owned package bridge request and provider runtime ports.
7. Reject invalid provider binding before runtime dispatch through activation or planning diagnostics.
8. For configured but unavailable providers, emit a retryable dispatch report before task claim. Do not create task leases while provider readiness is false.
9. Claim ready scan and docs writer package tasks with lease guards only after their prerequisites are satisfied.
10. Execute docs writer work through existing package task path.
11. Commit scan receipt, artifacts, final frame refs, outcome, and pending publication in task network journal.
12. Replay successful outcome without duplicate publication.

### Contracts To Add Or Change

- task claim request for package task nodes
- task claim request for `workspace_scan` capability task nodes
- scan task artifact commit contract
- scan receipt commit contract
- package task init material record
- context storage and prompt storage dispatch ports
- task execution report
- artifact commit contract
- outcome commit contract
- pending publication creation contract

### Durability And Idempotency

Task claim is lease guarded. Scan task claims do not require provider access. Docs writer task claims require provider readiness before lease creation. Artifact ids derive from task run and slots. Repeated successful outcomes replay without duplicate pending publication.

### Boundary Rules

Dispatch may call provider and write task-owned records. It must not mutate belief, mutate goals, or append canonical events directly.

### Verification

- ready `workspace_scan` task executes to completion
- scan dispatch records one scan receipt and stable artifacts
- ready docs writer package task executes to success with test provider
- invalid provider binding is rejected before runtime dispatch
- configured provider unavailable at dispatch produces retryable dispatch report without task claim or lease
- package traversal expands expected workspace nodes
- final frame artifact is committed
- successful outcome creates one pending publication

### Risks

Existing tests prepare many dependencies manually. Runtime dispatch must replace setup shortcuts without changing package semantics.

## Execution Publication And Event Spine

Owner: `meld-execution` publication runtime plus events domain.

### Current Code Anchors

- Spec skeleton defines publication at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:582`.
- Publication runtime exists at `crates/meld-execution/src/task_network/runtime.rs:20`.
- `publish_pending` delegates to publication bridge at `crates/meld-execution/src/task_network/runtime.rs:44`.
- Request contract starts at `crates/meld-execution/src/task_network/publication.rs:25`.
- Event record id field is at `crates/meld-execution/src/task_network/publication.rs:62`.
- Envelope builder starts at `crates/meld-execution/src/task_network/publication.rs:156`.
- Envelope sets record id at `crates/meld-execution/src/task_network/publication.rs:209`.
- Pending publications are processed at `crates/meld-execution/src/task_network/publication.rs:243`.
- Publication event record id helper is at `crates/meld-execution/src/task_network/publication.rs:494`.

### Current State

Publication runtime is close to the target shape. It can publish pending publications through an event append sink, use deterministic event record ids, and mark publications after append.

### Spec Gaps

- Needs concrete supervisor handle integration.
- Needs explicit docs writer success and failure publication policy.
- Needs evidence source mapping contract aligned with belief replay.
- Needs integration proof that repeated runtime ticks do not duplicate append after reopen.

### Implementation Guide

1. Preserve existing publication bridge as domain-owned append boundary.
2. Add concrete runtime handle that loads pending publications with bounded budget.
3. Use event append sink from product runtime ports.
4. Keep deterministic event record id from publication id.
5. Mark publication only after event append success or idempotent replay.
6. Expose publication tick report to supervisor status.
7. Align success event type with belief support evidence mapping.
8. Publish docs writer failure as an execution fact when policy enables it, but require belief to map that failure to a durable no-op receipt with no evidence assignment in this slice.

### Contracts To Add Or Change

- publication runtime handle request
- publication runtime report in supervisor handle report format
- docs writer success publication mapping
- docs writer failure publication mapping with belief no-op policy

### Durability And Idempotency

Publication id derives from outcome. Event record id derives from publication id. Repeated append replays through event store idempotency. Publication mark validates immutable outcome fields.

### Boundary Rules

Publication appends canonical events. It must not interpret belief, mutate goals, execute tasks, or decide satisfaction.

### Verification

- pending docs writer success appends one event
- repeated tick appends no duplicate
- publication mark survives reopen
- failure publication creates no belief support evidence and is consumed as a durable no-op receipt

### Risks

Failure publication policy is explicit for this slice: publication may append the failure fact, and belief records a no-op receipt rather than support or negative evidence.

## Supervisor And Runtime Handles

Owner: runtime supervisor and domain runtime handles.

### Current Code Anchors

- Spec skeleton defines handles at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:628`.
- Supervisor stores `InertRuntimeHandle` values at `src/runtime/supervisor/entrypoint.rs:175`.
- Supervisor start begins at `src/runtime/supervisor/entrypoint.rs:195`.
- Supervisor tick begins at `src/runtime/supervisor/entrypoint.rs:440`.
- Shutdown waits for safe point at `src/runtime/supervisor/entrypoint.rs:537`.
- Enabled runtimes start at `src/runtime/supervisor/entrypoint.rs:654`.
- Runtime start occurs at `src/runtime/supervisor/entrypoint.rs:676`.
- Existing tests cover heartbeat and restart policy at `src/runtime/supervisor/entrypoint.rs:1285` and `src/runtime/supervisor/entrypoint.rs:1363`.

### Current State

Supervisor owns lifecycle, leases, heartbeat, health snapshots, restart policy, shutdown, safe points, and flush. It starts inert handles. Tick renews leases and writes heartbeat and health only.

### Spec Gaps

- No concrete bounded semantic runtime handles.
- No semantic tick request or report in supervisor status.
- No domain handle registry carrying activation inputs.
- No provider-dependent dispatch enablement beyond generic preflight.
- Shutdown safe point is lifecycle-only, not tied to domain cursor flush reports.
- Scan execution route must be explicit. This guide uses task dispatch for `workspace_scan`, not a separate supervisor scan handle.

### Implementation Guide

1. Define a concrete handle trait or enum that supports bounded semantic tick, heartbeat, safe point, shutdown, and flush.
2. Keep supervisor lifecycle responsibilities unchanged.
3. Add domain handle factories that receive opened stores, ports, activation runtime inputs, work budget, and cancellation signal.
4. Implement handles in dependency order: bootstrap, belief evidence replay, goal curation, planning, task dispatch, publication, satisfaction.
5. Keep `workspace_scan` under task dispatch as a capability task route.
6. Supervisor tick calls bounded semantic tick for enabled handles after lease renewal.
7. Status includes runtime id, health, last semantic tick report, budget exhaustion, and diagnostics.
8. Shutdown waits for domain safe points, then flushes product stores.

### Contracts To Add Or Change

- runtime handle trait or enum
- bounded tick request
- heartbeat report
- semantic tick report
- safe point report
- shutdown report
- flush report with domain cursor summary

### Durability And Idempotency

Supervisor restart does not imply semantic replay by itself. Each domain handle resumes from its own durable cursor or receipt.

Heartbeat and health snapshots do not replace domain receipts.

### Boundary Rules

Supervisor may start, stop, tick, and report health. It must not construct directives, mutate goals, assess belief, publish events directly, or execute provider work outside handles.

### Verification

- runtime run starts enabled concrete handles
- each handle ticks with bounded budget
- provider-dependent dispatch disabled unless available
- shutdown reaches safe point and flushes stores
- status reports semantic runtime ids and health

### Risks

Supervisor must not become semantic orchestration logic. Keep ordering in handle dependencies and durable cursors, not in ad hoc supervisor decisions.

## CLI Activation Surface

Owner: CLI adapter.

### Current Code Anchors

- Spec skeleton defines CLI activation at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:687`.
- Runtime commands exist at `src/cli/parse.rs:127` and `src/cli/parse.rs:275`.
- Runtime status and run parse tests start at `src/cli/parse.rs:846`.
- Workflow execute already has provider, frame type, and force fields at `src/cli/parse.rs:793`.
- Context commands expose provider, frame type, and force fields at `src/cli/parse.rs:608`.

### Current State

CLI exposes runtime status and runtime run. It exposes workflow and context commands with provider and frame options, but there is no docs freshness activation command.

### Spec Gaps

- Missing activation command.
- Missing config file path option.
- Missing target selector option.
- Missing dry-run activation.
- Missing runtime run handoff option.
- Missing output format for diagnostics.

### Implementation Guide

1. Add parse shape for docs freshness activation.
2. Accept folder path, optional config path, provider selection, target selector, dry run, output format, and runtime handoff duration.
3. Keep parse module declarative.
4. Add tooling function that builds activation loader inputs and calls product activation service.
5. Dry run uses describe and validation only.
6. Non-dry run delegates to activation bootstrap through runtime service, not direct domain writes.
7. Runtime handoff starts supervisor only after activation validation passes.

### Contracts To Add Or Change

- activation CLI command DTO
- CLI diagnostics output DTO
- runtime handoff options

### Durability And Idempotency

CLI writes no domain records directly. Repeated activation with same config confirms durable records through bootstrap. Dry run writes nothing.

### Boundary Rules

CLI parses and delegates. It must not write agent, goal, belief, task, publication, or event stores directly.

### Verification

- command validates folder path
- dry run shows planned domain records without writes
- activation followed by runtime run reaches bootstrap complete
- invalid provider binding fails with actionable diagnostic

### Risks

The command can become too broad if it tries to be both configuration authoring and runtime execution. Keep authoring, validation, and run handoff explicit.

## Integration Proof

Owner: integration test suite.

### Current Code Anchors

- Spec skeleton defines proof at `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md:736`.
- Reopen after active goal is at `tests/integration/docs_freshness_reopen_contract.rs:255`.
- Reopen after pending publication is at `tests/integration/docs_freshness_reopen_contract.rs:291`.
- Reopen after publication append is at `tests/integration/docs_freshness_reopen_contract.rs:335`.
- Evidence ingestion is called directly at `tests/integration/docs_freshness_reopen_contract.rs:397`.
- Satisfaction review is driven manually at `tests/integration/docs_freshness_reopen_contract.rs:422`.
- Active goal setup manually accepts curation command at `tests/integration/docs_freshness_reopen_contract.rs:527`.
- Synthetic belief view construction begins at `tests/integration/docs_freshness_reopen_contract.rs:788`.

### Current State

The current reopen test proves many mechanics, including active goal reopen, pending publication reopen, publication append before satisfaction, evidence ingestion, and satisfaction closure. It still uses fixture shortcuts and direct cross-domain writes after setup.

### Spec Gaps

- No proof starts from activation config and folder path.
- Bootstrap is not the source of directive, agent, rule, and subscription records.
- Planning proof bypasses package bridge output.
- Evidence replay cursor is supplied by test.
- Satisfaction is manually invoked through test closures.
- Reopen checkpoints do not yet prove each semantic transition through runtime handles.

### Implementation Guide

1. Create product fixture workspace and activation config.
2. Run activation load and validation, then reopen.
3. Tick bootstrap handle until directive, agent, rule, and subscription exist, then reopen.
4. Tick belief and curation until one active goal exists, then reopen.
5. Tick planning until task network state contains scan work for the active goal and docs writer work is either absent or dependent on scan artifacts, then reopen.
6. Tick task dispatch until scan completion artifacts and scan receipt exist, then reopen.
7. Tick planning or task network readiness until docs writer package work is present and ready only because scan receipt and snapshot ref satisfy its dependencies, then reopen.
8. Tick dispatch until docs writer task outcome exists, then reopen.
9. Tick dispatch until one pending publication exists, then reopen.
10. Tick publication until canonical event append and publication mark exist, then reopen.
11. Tick belief evidence replay until belief revision exists, then reopen.
12. Tick satisfaction until goal lifecycle closes, then reopen.
13. Assert every checkpoint recovers identical durable state.
14. Assert every post-activation semantic transition has a matching enabled runtime handle tick report.

### Contracts To Add Or Change

- product fixture activation identity map
- bounded runtime driver for tests
- enabled runtime handle tick report assertions
- reopen checkpoint assertion helpers
- deterministic provider stub contract

### Durability And Idempotency

Every checkpoint must flush, reopen stores, and continue from durable cursors or receipts. No manual cross-domain writes are allowed after activation.

Every semantic transition after activation must be attributed to a tick report from the enabled runtime handle that owns the transition.

### Boundary Rules

Tests may inspect stores for assertions. Tests must not bypass semantic transitions after activation setup.

### Verification

Final gate target:

```text
cargo test --test integration_tests docs_freshness
```

Additional focused gates:

```text
cargo test -p meld-world-model belief
cargo test -p meld-world-model planner
cargo test -p meld-execution planning_runtime
cargo test -p meld-execution package
cargo test -p meld --test integration_tests outcome_evidence
cargo test -p meld --test integration_tests docs_freshness_reopen_contract
```

### Risks

Integration can look complete while still depending on fixture-only shortcuts. The proof must reject direct semantic writes after activation.

## Static Boundary Scans

Keep these scans in the build gate set.

```text
rg -n "Term::Object\\(object\\) => object.object_id" crates/meld-execution/src
rg -n "emit_envelope_best_effort" src/workspace | rg "src/workspace/(scan|capability)"
rg -n "register_seed_agent" crates/meld-world-model/src tests/integration
rg -n "ProductRuntimeConfig::for_product_root" src/runtime src/config
rg -n "single_task_mutation_set" tests/integration crates
rg -n "BeliefView \\{" tests/integration/docs_freshness_reopen_contract.rs
rg --files | rg '/mod\\.rs$'
```

Expected outcomes:

- object ref lowering is removed or isolated to compatibility-only paths
- workspace scan capability does not emit canonical events directly
- seed agent registration has conflict semantics for activation-owned fields
- runtime config loading carries provider and enabled runtime state into assembly
- docs freshness proof does not inject task network mutation sets directly
- docs freshness proof does not inject synthetic belief views
- new work does not expand `mod.rs` layout

## Buildout Handoff

Use this guide as the input to phased implementation orchestration.

Source plan path: `design/plan/integration/docs_freshness_flywheel_next_iteration_report.md`

Spec skeleton path: `design/plan/integration/docs_freshness_flywheel_domain_spec_skeleton.md`

Synthesized guide path: `design/plan/integration/docs_freshness_flywheel_implementation_guide.md`

Shared contracts first:

- activation config and validated activation
- directive, agent, curation rule, bootstrap receipt
- workspace scan request identity, artifacts, and receipts
- goal command and mutation receipts
- package trigger bridge and full object refs
- runtime handle reports
- integration identity map

Delivery waves:

1. Shared contract gate.
2. Bootstrap and active goal gate.
3. Goal-driven scan and package planning gate.
4. Dispatch and publication gate.
5. Evidence and satisfaction gate.
6. End to end reopen gate.

Blocked domains:

- task dispatch is blocked on scan task shape, package bridge output, context storage, and prompt storage
- evidence replay is blocked on publication event contract, belief-owned mapper, and cursor ownership
- satisfaction handle is blocked on active goal query port
- final proof is blocked on semantic runtime handles

Parallel-safe domains:

- workspace scan core after target selector and scan receipt identity land
- agent store bootstrap after directive and rule contracts land
- goal receipt hardening after command receipt contract lands
- publication handle after task outcome publication contract lands
- CLI parse after activation DTO lands

Residual risks:

- provider binding can leak into planning if activation config is not explicit
- object ref loss can remain in generic lowering if only docs bridge is fixed
- root adapters can become semantic owners if event-to-evidence mapping is not belief-owned
- fixture shortcuts can hide missing runtime handle behavior
- migration of existing agent records needs compatibility policy
