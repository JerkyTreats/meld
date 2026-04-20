# Workflow Subsystem

## Scope

Turn-based workflow execution with agent authorization, state persistence, gate evaluation, and CLI tooling.

## Purpose

Enforce deterministic workflow execution by validating agent bindings, orchestrating turns with gates and prompts, persisting state, and providing CLI operations.

## API Surface

### Agent-to-Workflow Binding

- `validate_agent_binding`
- `resolve_bound_workflow_id`

### Workflow Command Service

- `WorkflowCommandService`
- `WorkflowCommandService::run_list`
- `WorkflowCommandService::run_validate`
- `WorkflowCommandService::run_inspect`
- `WorkflowCommandService::run_execute`
- `WorkflowListItem`
- `WorkflowListResult`
- `WorkflowValidateResult`
- `WorkflowInspectResult`
- `WorkflowExecuteRequest`
- `WorkflowExecuteResult`
- `parse_node_id`
- `resolve_node_id`

### Workflow Events

- `ExecutionWorkflowTurnEventData`
- `From<crate::telemetry::WorkflowTurnEventData> for ExecutionWorkflowTurnEventData`
- `workflow_turn_started_envelope`
- `workflow_turn_completed_envelope`
- `workflow_turn_failed_envelope`

### Workflow Executor

- `execute_registered_workflow`
- `execute_registered_workflow_async`
- `execute_registered_workflow_via_task_async`
- `build_thread_id`
- `workflow_turn_frame_type`

### Workflow Facade

- `execute_workflow_target`
- `execute_workflow_target_async`
- `execute_registered_workflow_target`
- `execute_registered_workflow_target_async`
- `build_target_execution_request`

### Gate Evaluation

- `GateEvaluationResult`
- `evaluate_gate`

### Workflow Normalization

- `normalize_output_for_gate`
- `normalize_json_sections`
- `normalize_markdown_sections`
- `forbidden_section_tokens`
- `is_low_signal_json_value`
- `is_low_signal_text`
- `normalize_token`
- `split_markdown_sections`
- `parse_heading`
- `MarkdownSection`

### Workflow Profile

- `WorkflowProfile`
- `WorkflowProfile::validate`
- `WorkflowProfile::ordered_turns`
- `WorkflowThreadPolicy`
- `WorkflowTurn`
- `WorkflowGate`
- `WorkflowArtifactPolicy`
- `WorkflowFailurePolicy`
- `PromptRefKind`
- `PromptRefKind::parse`

### Workflow Registry

- `WorkflowRegistry::load`
- `WorkflowRegistry::get`
- `WorkflowRegistry::contains`
- `WorkflowRegistry::iter`
- `RegisteredWorkflowProfile`
- `collect_workflow_profile_paths`
- `validate_prompt_refs`
- `resolve_prompt_path`

### Workflow Resolver

- `ResolvedTurnInputs`
- `resolve_turn_inputs`
- `resolve_prompt_template`
- `render_turn_prompt`

### Workflow State Store

- `WorkflowStateStore`
- `WorkflowThreadStatus`
- `WorkflowTurnStatus`
- `WorkflowThreadRecord`
- `WorkflowTurnRecord`
- `ThreadTurnGateRecordV1`
- `PromptLinkRecordV1`
- `upsert_thread`
- `upsert_turn`
- `upsert_gate`
- `upsert_prompt_link`
- `load_thread`
- `load_turns`
- `completed_output_map`

### Workflow Summary Telemetry

- `command`

### Workflow Tooling CLI

- `handle_cli_command`

## Behavior Notes

### Agent-to-Workflow Binding

- An agent with no bound workflow_id is always accepted as valid.
- Only agents with role Writer may bind to a workflow.
- A bound workflow_id must exist in the provided WorkflowRegistry.
- Validation failure returns ApiError::ConfigError with descriptive message.

### Workflow Command Service

- `WorkflowCommandService::run_list` returns `WorkflowListResult` with workflows sorted lexicographically by `workflow_id`.
- `WorkflowCommandService::run_execute` requires either `--node` or `--path`, but not both, for node resolution.
- Agent must have writer role; otherwise `ApiError::Unauthorized` is returned.
- If agent has a workflow binding, it must match the request `workflow_id`; otherwise `ApiError::ConfigError` is returned.
- Provider existence is validated before execution using `api.provider_registry()`.
- `parse_node_id` enforces 32-byte hex input with optional `0x` prefix; otherwise `ApiError::InvalidFrame` is returned.

### Workflow Events

- Event envelopes use `workflow_id` as `stream_id` under the `execution` domain.
- Workflows, threads, and turns are linked with `belongs_to` and `targets` relations.
- Turns may produce frames through the `produced` relation and target nodes through the `targets` relation when `final_frame_id` and `node_id` are present.
- Plan association is expressed via `belongs_to` relation when `plan_id` is present.
- Workspace node references are conditionally omitted when `node_id` is empty.

### Workflow Executor

- Failure policy `resume_from_failed_turn` enables resuming from the first failed turn when `request.force=false`.
- When `request.force=true`, the head is tombstoned and force reset events are emitted; existing state is ignored.
- State store persists thread and turn records with status values `Pending`, `Running`, `Completed`, and `Failed`, plus `next_turn_seq` and `final_frame_id` where applicable.
- Thread ID is computed deterministically from `workflow_id`, hex encoded `node_id`, and `frame_type` using blake3 hashing and truncating to 16 hex chars prefixed with `thread-`.
- Intermediate turns derive frame types in format `{requested_frame_type}--workflow-turn-{seq}-{prompt_link_id}`. Final turn reuses `requested_frame_type` directly.

### Workflow Facade

- Each synchronous and asynchronous execution path validates that the request contains a workflow-backed program by calling `request.program.workflow_id()` and returns `ApiError::ConfigError` if missing.
- Registered workflow target execution enforces that the request's `program.kind` is `TargetExecutionProgramKind::Workflow` and returns `ApiError::ConfigError` otherwise.
- If a workflow execution completes without producing a final frame, the execution fails with `ApiError::GenerationFailed` containing the workflow ID.
- `TargetExecutionResult.reused_existing_head` is set to true when `summary.turns_completed` equals zero.
- Both `execute_registered_workflow_target` and `execute_registered_workflow_target_async` return `TargetExecutionResult` with `final_frame_id`, `reused_existing_head`, `program`, `workflow_id`, `thread_id`, and `turns_completed` derived from the execution summary.

### Gate Evaluation

- `GateEvaluationResult` uses `GateOutcome::Pass` for successful evaluations and `GateOutcome::Fail` otherwise.
- `evaluate_gate` routes to specialized handlers based on `gate.gate_type`: `schema_required_fields`, `required_sections`, or `no_semantic_drift`.
- `schema_required_fields` checks for required fields in both JSON keys and case-insensitive text presence.
- `required_sections` enforces presence of required sections and absence of forbidden sections derived from rules.
- `no_semantic_drift` first ensures non-empty output, then delegates to `required_sections` when `required_sections_from_input` or `required_fields` are present.
- Required sections can be dynamically derived from structured input using `required_sections_from_input` rule and `markdown_section_name` mappings.
- Unknown `gate_type` results in immediate failure with message indicating the unknown type.
- Empty output causes `no_semantic_drift` gate to fail regardless of other rules.
- Section matching is case-insensitive and ignores non-alphanumeric characters via `normalize_section_token`.

### Workflow Normalization

- For `required_sections` gates, the `normalize_json_sections` function removes forbidden and low signal JSON sections.
- For `no_semantic_drift` gates, the `normalize_markdown_sections` function removes forbidden and low signal markdown sections.
- The `is_low_signal_json_value` function identifies JSON values as low signal if they are null, empty strings, empty arrays, or empty objects.
- The `is_low_signal_text` function considers text as low signal if it is empty or normalized to `insufficientcontext`.
- The `normalize_token` function strips non-alphanumeric characters and converts text to lowercase for comparison.
- The `MarkdownSection` struct represents a markdown section with heading level, heading text, and body, and provides a render method to output formatted content.
- The `split_markdown_sections` function parses input text into a vector of `MarkdownSection` structs, separating headings and bodies.
- The `parse_heading` function identifies markdown headings by detecting leading `#` characters and extracting the heading level and text.

### Workflow Profile

- `WorkflowProfile::validate` enforces determinism by rejecting empty required fields, duplicate turn or gate IDs, duplicate turn seq values, and invalid cross-references to undefined gate_ids.
- `WorkflowTurn` `retry_limit` and `timeout_ms` must be greater than zero, and `turn.seq` must be unique per profile.
- `WorkflowFailurePolicy.mode`, `WorkflowFailurePolicy.resume_from_failed_turn`, and `WorkflowFailurePolicy.stop_on_gate_fail` control failure handling behavior.
- `PromptRefKind::parse` resolves prompt references using known prefixes: `artifact:` maps to `ArtifactId`, `builtin:` maps to `prompts/{rest}.md`, and other values map directly to `FilePath`.

### Workflow Registry

- `WorkflowRegistry::load` rejects duplicate `workflow_id` values in the same directory layer.
- During profile collection, files in `prompts` and `packages` subdirectories are ignored.
- Only files with `.yaml` or `.yml` extensions are processed.
- Each profile is validated by calling `profile.validate()` and `validate_prompt_refs()`.
- Prompt references are resolved as absolute paths if provided absolutely, or relative to the profile file if provided relatively; unresolved references cause load failure.
- Collection order is deterministic due to sorting of collected paths.

### Workflow Resolver

- `resolve_turn_inputs` constructs a context payload string by ordering input values alphabetically by key and formatting each with `format_input_payload`.
- `target_context` input_ref triggers `collect_target_context`, which dispatches to file or directory context collection based on node type.
- For file nodes, `collect_file_target_context` uses in-memory frames if available, otherwise reads from file system.
- For directory nodes, `collect_directory_target_context` aggregates child contexts ordered by `child_context_priority` in this order: README.md, mod.rs, lib.rs, then default.
- `resolve_prompt_template` accepts `artifact:artifact-id` or relative/absolute file paths; relative paths are resolved relative to the workflow profile parent directory.
- `render_turn_prompt` appends a Task directive including `turn_id` and `output_type`, and a Context section that shows `Insufficient context` when `context_payload` is empty.
- Missing input_ref in `prior_outputs` or failure to resolve prompt path results in `ApiError::ConfigError` with descriptive message.

### Workflow State Store

- `new` constructs the store using XDG workspace data dir or falls back to temp dir when unavailable.
- `upsert_gate` and `upsert_prompt_link` validate their records before writing via `validate_thread_turn_gate_record_v1` and `validate_prompt_link_record_v1`.
- `load_turns` returns a sorted `Vec<WorkflowTurnRecord>` by `seq` ascending when turn directory exists.
- `completed_output_map` returns a `HashMap<String, String>` containing completed turn outputs keyed by `output_type` and `turn_id`.
- `ensure_root_directories` creates `threads`, `turns`, `gates`, and `prompt_links` subdirectories under the root.

### Workflow Summary Telemetry

- The `command` function always produces a `TypedSummaryEvent` with `event_type` equal to `workflow_summary`.
- The resulting `event.data` is a JSON object with fixed keys: `scope`, `action`, `ok`, `duration_ms`, and `error`.
- The output is deterministic: for identical inputs, the produced `TypedSummaryEvent` matches the expected structure exactly.

### Workflow Tooling CLI

- Runtime configuration is loaded from a specified file or from workspace_root via ConfigLoader. Errors during loading are converted to ApiError::ConfigError.
- Execute constructs a WorkflowExecuteRequest from CLI parameters, merges path and path_positional, and passes QueueEventContext for telemetry reporting.
- Each command supports both human-readable and JSON output based on the format parameter. JSON serialization errors produce ApiError::ConfigError.
- Non-JSON output for List shows `workflow_id | version | title | source_path`.
- Non-JSON output for Inspect shows structured key-value fields including turns and gates.
- List and Inspect commands acquire read locks on the parking_lot::RwLock<WorkflowRegistry> before calling WorkflowCommandService methods.

## Usage

### Agent-to-Workflow Binding

Call `validate_agent_binding` with an AgentIdentity and a WorkflowRegistry reference to validate agent workflow bindings.

### Workflow Command Service

Workflow command operations are invoked via `WorkflowCommandService` methods:
- `run_list` retrieves all registered workflows from a `WorkflowRegistry`.
- `run_validate` loads and validates workflows from a `WorkflowConfig`.
- `run_inspect` retrieves detailed profile information for a specific workflow.
- `run_execute` initiates workflow execution with authorization and provider checks.

Execution requests require an agent with writer role, a valid provider name, and exactly one node resolution mechanism: `--node` or `--path`.

### Workflow Events

Construct workflow turn event envelopes via `workflow_turn_started_envelope`, `workflow_turn_completed_envelope`, or `workflow_turn_failed_envelope`, passing `session_id` and `ExecutionWorkflowTurnEventData`. Use `From<crate::telemetry::WorkflowTurnEventData>` for conversion from telemetry events.

### Workflow Executor

Use `execute_registered_workflow_async` for native async execution; it supports thread state restoration, retry logic, and gate evaluation.

Call `execute_registered_workflow` with `ContextApi`, `workspace_root`, `RegisteredWorkflowProfile`, `WorkflowExecutionRequest`, and optional event context for synchronous execution.

When `workflow_uses_task_package_path` returns `true`, execution delegates to `execute_registered_workflow_via_task_async` and uses task capabilities and graph traversal to resolve final frame.

### Workflow Facade

Use `execute_workflow_target` or `execute_workflow_target_async` to execute a workflow by workflow ID found in `TargetExecutionRequest.program`.

Use `execute_registered_workflow_target` or `execute_registered_workflow_target_async` to execute a pre-resolved `RegisteredWorkflowProfile`, assuming the workflow program kind has been validated.

### Gate Evaluation

Call `evaluate_gate` with a `WorkflowGate` reference, output string, and optional `input_values` `HashMap` reference to evaluate a gate.

### Workflow Normalization

Use the `normalize_output_for_gate` function to normalize workflow output based on the gate type. For `required_sections` gates, the `normalize_json_sections` function is used; for `no_semantic_drift` gates, the `normalize_markdown_sections` function is used.

### Workflow Profile

Construct a `WorkflowProfile` with required fields, then call `validate` to ensure consistency. Use `ordered_turns` to process turns in deterministic sequence. Use `PromptRefKind::parse` to resolve prompt references according to supported prefixes.

### Workflow Registry

Call `WorkflowRegistry::load` with a `WorkflowConfig` instance to initialize the registry. Use `get`, `contains`, or `iter` to access loaded profiles.

### Workflow Resolver

Invoke `resolve_turn_inputs` with `ContextApi`, `node_id`, `frame_type`, `WorkflowTurn`, and `prior_outputs` to obtain `ResolvedTurnInputs`. Use `resolve_prompt_template` to fetch prompt template by artifact ID or path. Combine results with `render_turn_prompt` to produce final prompt.

### Workflow State Store

Construct a `WorkflowStateStore` by calling `new` with a workspace root path. Store thread and turn records via `upsert_thread` and `upsert_turn`. Store gates and prompt links via `upsert_gate` and `upsert_prompt_link`. Retrieve stored data via `load_thread` and `load_turns`. Aggregate completed outputs via `completed_output_map`.

### Workflow Summary Telemetry

Call the `command` function with the action name, success status, duration in milliseconds, and optional error string to generate a structured telemetry event.

### Workflow Tooling CLI

Invoke `handle_cli_command` with a WorkflowCommands variant and appropriate parameters. Output is returned as a formatted string or JSON depending on the command's `format` field.

## Caveats

### Agent-to-Workflow Binding

- Insufficient context to specify exact format or lifetime of workflow_id string in AgentIdentity.
- Insufficient context to specify how WorkflowRegistry.load and WorkflowConfig are constructed beyond visible test usage.

### Workflow Command Service

- Insufficient context for execution frame semantics, session handling, and error propagation details in `WorkflowCommandService::run_execute`.
- Insufficient context for `WorkflowRegistry` load, validation, and registration mechanics outside `run_validate` and `run_inspect`.

### Workflow Executor

- Insufficient context to specify event delivery guarantees beyond best-effort emission via `QueueEventContext`.
- Insufficient context to confirm artifact anchor validation beyond observed execution artifact checks and `frame_ref` type expectation.
- Insufficient context to specify hash algorithm details or truncation behavior beyond observed `thread-` prefix and 16-char hex suffix.

### Workflow Facade

- Insufficient context to determine error handling behavior for `ConfigLoader::load` failures, `WorkflowRegistry::load` failures, or `registry.get` lookup failures beyond what is verified.

### Workflow Normalization

- Insufficient context

## Related Components

- `./src/agent/identity.rs`
- `./src/config.rs`
- `./src/context/frame.rs`
- `./src/context/queue.rs`
- `./src/error.rs`
- `./src/provider.rs`
- `./src/store/mod.rs`
- `./src/api.rs`
- `./src/telemetry.rs`
- `./src/events.rs`
- `./src/workspace.rs`
- `./src/workflow/binding.rs`
- `./src/workflow/commands.rs`
- `./src/workflow/events.rs`
- `./src/workflow/executor.rs`
- `./src/workflow/facade.rs`
- `./src/workflow/gates.rs`
- `./src/workflow/normalization.rs`
- `./src/workflow/profile.rs`
- `./src/workflow/registry.rs`
- `./src/workflow/resolver.rs`
- `./src/workflow/state_store.rs`
- `./src/workflow/summary.rs`
- `./src/workflow/tooling.rs`
- `./src/workflow/record_contracts/id_validation.rs`
- `./src/workflow/record_contracts/prompt_link_record.rs`
- `./src/workflow/record_contracts/schema_version.rs`
- `./src/workflow/record_contracts/thread_turn_gate_record.rs`
