# Capability Synthesis Task

Date: 2026-04-09
Status: proposed
Scope: CapabilitySynthesisTask definition — the task that discovers or writes external programs, validates their output, and registers new synthesized capabilities in the runtime catalog

## Intent

Define `CapabilitySynthesisTask` as a standard task in the task network that produces a new
runtime catalog entry as its terminal output.

The task is triggered when the HTN planner cannot satisfy a capability requirement from the
existing catalog. Its output is a `CompiledCapabilityRef` pointing to a newly registered
synthesized capability. After the task completes, the planner can requery the catalog and
proceed.

## Why This Is a Task, Not a Capability

Synthesis involves multiple steps that benefit from task-level semantics:

- **Artifact persistence across steps**: the environment scan feeds the synthesis plan, which
  feeds the test run, which feeds contract definition. These are intermediate artifacts that
  need to survive retries.
- **Retry at step granularity**: if the test run fails, retry from there, not from environment
  scan. Task-level attempt lineage tracks this.
- **Conditional execution**: catalog registration only runs if validation passes. This is a
  task-level dependency edge, not retry logic.
- **Multi-turn LLM interaction**: the synthesis plan step may involve multiple provider turns
  or a full agent harness session. This is naturally expressed as a capability chain inside
  a task.

## Seed Artifacts

```json
{
  "task_id": "task_capability_synthesis",
  "init_artifacts": [
    {
      "slot_id": "goal_context",
      "artifact_type_id": "synthesis_goal_context",
      "content": {
        "artifact_type_needed": "change_summary",
        "artifact_schema_ref": "change_summary_v1",
        "evidence_description": "quantified summary of git-tracked changes scoped to a workspace node",
        "node_scope": "workspace_fs",
        "requestor_task_id": "task_impact_assessment_node_42"
      }
    }
  ]
}
```

`artifact_type_needed` is the target output schema the synthesized capability must produce.
`evidence_description` is a natural language description of what useful output looks like,
used by the synthesis plan capability as context.
`requestor_task_id` provides traceability back to the task that triggered synthesis.

## Capability Chain

```
environment_scan
    |
synthesis_plan         (provider_execute_chat or agent harness)
    |
execution_test
    |
contract_definition    (provider_execute_chat)
    |
contract_validation
    |
catalog_registration   (runs only if contract_validation passed)
    |
    → CompiledCapabilityRef
```

### environment_scan

Surveys the host environment for available tools relevant to the goal context.

Input: `synthesis_goal_context`
Output: `AvailableToolsManifest { executables: Vec<ExecutableRecord>, languages_available: Vec<String>, package_managers: Vec<String> }`

Implementation: shell inspection — `which`, `find`, `PATH` enumeration, language runtime
detection. This is a compiled capability with deterministic behavior.

### synthesis_plan

Produces a concrete plan for how to generate the needed artifact.

Input: `synthesis_goal_context` + `AvailableToolsManifest`
Output: `SynthesisPlan { approach: String, command_or_script: String, args_template: Vec<String>, conversion_prompt: String, estimated_output_format: String }`

Implementation: `provider_execute_chat` with a prompt that includes the goal context,
available tools, and target artifact schema. The LLM reasons about what combination of
available tools can produce the needed evidence and how to convert the output.

**Agent harness variant**: when the goal context is complex or no suitable tool is found,
the synthesis plan step can invoke a full agent harness (Claude Code, Codex, or equivalent)
with shell access. The agent can write scripts, test them, and return a complete
`SynthesisPlan`. From the task's perspective this is still one capability invocation; the
agent session is an implementation detail of the provider execution.

### execution_test

Runs the proposed command or script against a test input to validate it produces output.

Input: `SynthesisPlan`
Output: `RawOutputSample { stdout: String, stderr: String, exit_code: i32, timed_out: bool }`

If `exit_code` is nonzero or `timed_out` is true, this capability returns a soft failure
output. Control triggers retry of `synthesis_plan` with the failure context appended —
giving the LLM a chance to revise its approach given the test failure.

### contract_definition

Converts the `SynthesisPlan` and `RawOutputSample` into a fully specified
`SynthesizedCapabilityDef`.

Input: `SynthesisPlan` + `RawOutputSample` + `synthesis_goal_context`
Output: `SynthesizedCapabilityDef { input_contract, output_contract, execution_contract, env_preconditions }`

Implementation: `provider_execute_chat` with a prompt that includes the raw output sample,
the target artifact schema, and a request to produce a capability definition JSON matching
the `SynthesizedCapabilityDef` schema. The LLM produces the typed input/output contract
and the `LLMConversion` output adapter with a concrete conversion prompt.

### contract_validation

Validates that the `SynthesizedCapabilityDef` can actually produce a valid artifact.

Input: `SynthesizedCapabilityDef` + `RawOutputSample`
Output: `ValidationRecord { passed: bool, schema_match_score: f32, artifact_sample: Option<serde_json::Value>, failure_notes: Vec<String> }`

Implementation: compiled capability. Runs the `OutputAdapter` conversion from the
`SynthesizedCapabilityDef` against the `RawOutputSample.stdout`. Validates the result
against the `output_schema`. Records the schema match score (what fraction of required
fields are present and correctly typed).

`passed` is true when `schema_match_score >= 0.95`. A score below this threshold is a
soft failure. Control triggers retry of `contract_definition` with the validation failure
notes appended as context.

### catalog_registration

Registers the validated capability in the runtime catalog.

Input: `SynthesizedCapabilityDef` + `ValidationRecord { passed: true }`
Output: `CompiledCapabilityRef { capability_type_id, catalog: RuntimeCatalog, schema_hash }`

This capability only executes if `ValidationRecord.passed == true`. This is a task-level
dependency edge: `catalog_registration` input slot `validation_record` requires an artifact
where `validation_record.passed == true`. If the artifact has `passed == false`, the slot
is not satisfied and `catalog_registration` remains blocked.

Implementation: compiled capability. Writes the `SynthesizedCapabilityDef` to the sled
runtime catalog namespace. Returns the `CompiledCapabilityRef` that the requesting task
can use.

## Retry Model

| Step | Failure type | Retry target |
|---|---|---|
| `environment_scan` | Hard (shell error) | Retry same step |
| `synthesis_plan` | Soft (LLM revision needed) | Retry `synthesis_plan` with failure context |
| `execution_test` | Soft (script/command failed) | Retry `synthesis_plan` with test failure appended |
| `contract_definition` | Soft (LLM revision needed) | Retry `contract_definition` with failure context |
| `contract_validation` | Soft (schema mismatch) | Retry `contract_definition` with validation notes |
| `catalog_registration` | Hard (sled write error) | Retry same step |

After three failed retries of `synthesis_plan`, control should escalate to repair rather
than continuing to retry. The goal context may be unsatisfiable with the available tools.

## Idempotency

If a `CapabilitySynthesisTask` for the same `synthesis_goal_context` is triggered while
one is already running, the second request should be deduplicated. Control checks the
runtime catalog for a pending or recently completed synthesis for the same `artifact_type_needed`
before dispatching a new task.

If the synthesis task completed and registered a capability, `catalog_registration` returns
the existing `CompiledCapabilityRef` rather than creating a duplicate entry.

## Output: CompiledCapabilityRef

The terminal output artifact:

```json
{
  "artifact_type_id": "compiled_capability_ref",
  "content": {
    "capability_type_id": "h3f8a2b1...",
    "catalog": "runtime",
    "artifact_types_produced": ["change_summary"],
    "schema_hash": "blake3:a1b2c3...",
    "synthesis_task_run_id": "taskrun_synthesis_001",
    "registered_at_seq": 4821
  }
}
```

The `registered_at_seq` is the spine event sequence at which the capability was registered.
The HTN planner uses this to confirm the catalog was updated before replanning.

## Read With

- [External Process Capability](external_process_capability.md)
- [Runtime Catalog](runtime_catalog.md)
- [Synthesis Overview](README.md)
- [Task Design](../../completed/capabilities/task/README.md)
- [Execution Planning](../planning/README.md)
