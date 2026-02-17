# God Module Detangling Spec

Date: 2026-02-15

## Objective

Define a focused refactor plan for god modules by extracting bounded domains and assigning ownership for logic currently mixed across:

- `src/tooling/cli.rs`
- `src/api.rs`
- `src/provider.rs`
- `src/agent.rs`

## Scope

This spec covers domain extraction only. It does not redesign business behavior or CLI UX.

- Cross domain planning map: [Src Module Structure Map](src_module_structure_map.md).

## Domain Inventory

### `src/tooling/cli.rs`

#### Domain: CLI Shell (parse/route/help)
- Focused shell spec: [CLI Shell Parse Route Help Spec](cli/cli_shell_parse_route_help_spec.md).
- Official domain status: Exists as legacy `src/tooling/cli.rs` and should migrate to thin `src/cli` adapters only.
- Logical concerns to move:
- Workspace mutations and validation orchestration out of `CliContext` (`run_workspace_*` family).
- Provider/agent CRUD and validation workflows out of command handlers.
- Context generation planning and execution (`build_generation_plan`, queue/runtime lifecycle).
- Impact/risk:
- Moderate regression risk in command output parity (text/json formatting differences).
- Low data integrity risk if command semantics remain identical and service contracts are stable.

#### Domain: Workspace Lifecycle (scan/validate/delete/restore/compact)
- Focused workspace spec: [Workspace Lifecycle Services Spec](workspace/workspace_lifecycle_services.md).
- Official domain status: Partial (building blocks exist: `tree`, `store`, `ignore`, `workspace_status`); no dedicated application service.
- Logical concerns to move:
- `run_workspace_validate`, `run_workspace_delete`, `run_workspace_restore`, `run_workspace_compact`, `run_workspace_list_deleted`.
- Root computation + store/index consistency checks currently embedded in CLI.
- Impact/risk:
- High whack-a-mole risk if split ad hoc because workspace logic touches node store, frame storage, head index, ignore rules, and tombstone policy.
- Mitigation: introduce a `workspace` service with explicit request/response types before moving handlers.

#### Domain: Context Generation Orchestration
- Focused generation spec: [Context Generation Orchestration Spec](context/context_generation_orchestration.md).
- Context domain structure spec: [Context Domain Structure Spec](context/context_domain_structure.md).
- Official domain status: Partial, core logic exists but ownership is split across `src/tooling/cli.rs`, `src/generation/*`, and `src/frame/queue.rs`.
- Logical concerns to move:
- Plan construction (`build_generation_plan`) and subtree precondition checks from CLI.
- Queue start/stop and orchestrator invocation from CLI command path.
- Impact/risk:
- High leakage risk around runtime assumptions (`tokio::Runtime::new`, `block_on`) and queue worker lifecycle.
- Mitigation: add context domain orchestration and queue services that own runtime boundary and return deterministic command results.

#### Domain: Provider Diagnostics/Connectivity
- Focused provider diagnostics spec: [Provider Diagnostics Connectivity Spec](provider/provider_diagnostics_connectivity.md).
- Official domain status: Partial (`provider::ProviderRegistry` exists; diagnostics currently in CLI).
- Logical concerns to move:
- Provider connectivity/model checks (runtime creation + `list_models` calls).
- Unified status provider section assembly logic.
- Impact/risk:
- Medium risk of duplicate networking behavior across commands if not centralized.
- Mitigation: single provider diagnostics service with shared timeout/retry/error mapping policy.

#### Domain: Agent/Provider Config Management Commands
- Focused config command spec: [Agent Provider Config Management Commands Spec](agent/agent_provider_config_management_commands.md).
- Official domain status: Partial (registries exist; persistence and validation mixed into registries and CLI).
- Logical concerns to move:
- Command-side create/update/delete validation workflows.
- File-write flow currently calling registry static save/delete methods directly.
- Impact/risk:
- Medium risk of policy drift (validation rules diverging between status/validate/create/update paths).
- Mitigation: use explicit command-level use cases per aggregate (`CreateAgent`, `UpdateProvider`, etc.).

#### Domain: Telemetry Event Engine and Session Lifecycle
- Focused telemetry spec: [Telemetry Event Engine Spec](telemetry/telemetry_event_engine_spec.md).
- Official domain status: Partial, event primitives exist under `progress` but lifecycle policy is embedded in CLI `execute`.
- Logical concerns to move:
- Start/finish/prune session policy and event naming conventions.
- Command summary emission scaffolding.
- Impact/risk:
- Low functional risk, medium observability risk (missing/renamed events).
- Mitigation: keep current event names as compatibility contract during migration.

#### Domain: CLI Presentation Formatting
- Focused presentation spec: [CLI Presentation Formatting Spec](cli/cli_presentation_formatting.md).
- Official domain status: Partial (spec exists; rendering remains co-located in CLI handlers).
- Logical concerns to move:
- Text and JSON formatting helpers spread across handler methods.
- Table rendering and truncation logic.
- Impact/risk:
- Low data risk, medium UX risk from accidental output shape changes (especially JSON fields consumed by automation).
- Mitigation: snapshot tests for representative command outputs.

### `src/api.rs`

#### Domain: Context Query API
- Focused query spec: [Context Query API Spec](api/context_query_api.md).
- Context domain structure spec: [Context Domain Structure Spec](context/context_domain_structure.md).
- Official domain status: Partial, query logic exists but ownership is split across `src/api.rs`, `src/views.rs`, and `src/composition.rs`.
- Logical concerns to move:
- Read-path policy logic (`get_node`, convenience query helpers).
- `NodeContext` convenience methods should remain query-facing and not expand into mutation concerns.
- Impact/risk:
- Low risk if behavior is preserved; high readability gain by isolating read use cases.

#### Domain: Frame Mutation + Head/Basis Updates
- Official domain status: Exists (`frame`, `heads`, `regeneration`), currently coordinated in one service.
- Logical concerns to move:
- Shared write transaction pattern now duplicated across `put_frame`, `synthesize_branch`, `regenerate`.
- Index persistence trigger policy (`persist_indices`) after mutations.
- Impact/risk:
- High integrity risk if head/basis updates and persistence are split inconsistently.
- Mitigation: central mutation coordinator that enforces atomic update order.

#### Domain: Node Lifecycle (tombstone/restore/compact)
- Official domain status: Partial (spec exists; service extraction from `ContextApi` is pending).
- Logical concerns to move:
- Subtree traversal and logical deletion/restoration/compaction operations.
- Node/head/frame purge policies and TTL behavior.
- Impact/risk:
- High whack-a-mole risk due coupling to traversal + head index internals + frame storage cleanup.
- Mitigation: define explicit invariants and reusable subtree operation primitives before extraction.

#### Domain: Synthesis/Regeneration/Composition Orchestration
- Official domain status: Exists as modules (`synthesis`, `regeneration`, `composition`) but orchestration lives in `ContextApi`.
- Logical concerns to move:
- Agent authorization checks, lock acquisition, and pre/post mutation bookkeeping.
- Concern-specific orchestration should move to separate use-case services.
- Impact/risk:
- Medium risk of behavior drift (especially ordering/locking semantics).
- Mitigation: contract tests around deterministic outputs and mutation side effects.

#### Domain: Agent/Provider Boundary Access
- Official domain status: Exists (`agent`, `provider` registries) but API currently exposes registry handles.
- Logical concerns to move:
- Direct registry exposure methods should be replaced with narrower query/use-case methods.
- Impact/risk:
- Medium leakage risk if external callers depend on current internals.
- Mitigation: keep compatibility wrappers temporarily and deprecate incrementally.

### `src/provider.rs`

#### Domain: Provider Transport Clients
- Official domain status: Exists conceptually; needs structural split into submodules.
- Logical concerns to move:
- OpenAI/Anthropic/Ollama/Local client implementations and provider-specific wire mappings.
- HTTP error normalization logic shared across providers.
- Impact/risk:
- Medium risk of provider-specific regressions (request/response schema mismatch).
- Mitigation: per-provider integration tests for `complete` and `list_models`.

#### Domain: Provider Registry (in-memory aggregate)
- Official domain status: Exists.
- Logical concerns to move:
- Keep only aggregate operations (`get`, `list`, `create_client`) and remove filesystem I/O responsibilities.
- Impact/risk:
- Low risk if method signatures remain stable.

#### Domain: Provider Config Repository (XDG persistence)
- Official domain status: Partial (spec exists; repository extraction from registry/CLI paths is pending).
- Logical concerns to move:
- `load_from_xdg`, `get_provider_config_path`, `save_provider_config`, `delete_provider_config`.
- Impact/risk:
- Medium risk around path resolution and precedence (config file vs XDG override).
- Mitigation: repository tests for precedence and filename/provider-name validation.

#### Domain: Provider Validation/Diagnostics
- Official domain status: Partial.
- Logical concerns to move:
- Syntactic config validation from registry.
- Connectivity/model availability validation from CLI into a shared diagnostics service.
- Impact/risk:
- Medium risk of duplicated validation logic if not centralized in one service.

### `src/agent.rs`

#### Domain: Agent Identity and Authorization
- Official domain status: Exists.
- Logical concerns to move:
- Keep `AgentIdentity`, roles/capabilities, and authorization checks as core domain model.
- Impact/risk:
- Low risk; this is already cohesive.

#### Domain: Agent Registry (in-memory aggregate)
- Official domain status: Exists.
- Logical concerns to move:
- Keep CRUD/list/filter operations; remove config file and prompt file responsibilities.
- Impact/risk:
- Low risk if registry API shape remains stable.

#### Domain: Agent Config Repository (XDG + prompt loading)
- Official domain status: Partial (spec exists; repository extraction from registry/CLI paths is pending).
- Logical concerns to move:
- `load_from_xdg`, `save_agent_config`, `delete_agent_config`, prompt path resolution and read.
- Impact/risk:
- Medium risk around prompt resolution and backward compatibility (`system_prompt` vs `system_prompt_path`).
- Mitigation: migration tests covering both prompt input styles.

#### Domain: Agent Validation Service
- Official domain status: Partial.
- Logical concerns to move:
- `validate_agent` checks (config presence, prompt readability, metadata template requirements).
- Impact/risk:
- Medium risk of hidden policy coupling with CLI command behavior.
- Mitigation: validation result schema as shared contract used by status/validate commands.

## Migration Sequence

1. Extract repositories first (agent/provider config persistence), keeping old call sites.
2. Extract application services for workspace lifecycle and generation orchestration.
3. Extract API mutation coordinator for head/basis/persistence invariants.
4. Slim CLI into parse + route + format only.
5. Seal module boundaries (remove direct field access like `head_index.heads` from non-domain modules).

## Refactor Guardrails

- Preserve behavior through compatibility wrappers until all call sites are migrated.
- Add characterization tests before moving high-risk flows:
- workspace delete/restore/compact
- context generate (recursive and non-recursive)
- provider validate/status connectivity checks
- Enforce “one owner per concern”:
- registries: in-memory aggregate operations
- repositories: filesystem/XDG persistence
- services: orchestration and policy
- adapters (CLI/API): input/output translation only

## Acceptance Criteria

- `src/cli` owns command parsing/routing + presentation only; orchestration moved to domain services.
- `src/tooling/cli.rs` remains a temporary compatibility wrapper only during migration.
- `src/api.rs` does not directly own unrelated lifecycle concerns (workspace compaction/tombstone service extracted).
- `src/provider.rs` and `src/agent.rs` no longer mix domain model, persistence, and diagnostics in one module.
- No crate-external module reaches into domain internals (for example, direct `HeadIndex` field access).
