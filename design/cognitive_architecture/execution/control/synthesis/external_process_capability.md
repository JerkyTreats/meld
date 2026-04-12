# External Process Capability

Date: 2026-04-09
Status: proposed
Scope: ExternalProcess execution class extension to the capability contract model, enabling synthesized capabilities to wrap subprocess invocations with LLM-driven output conversion

## Intent

Define how a synthesized capability executes: run a subprocess, convert its output to a
typed artifact via an LLM adapter, and present the result through the standard capability
contract. The contract surface facing the task compiler is identical to a compiled capability.
The execution path underneath is different.

## ExecutionContract Extension

The existing `ExecutionContract` in the capability model declares `execution_class` as one of
`inline`, `queued`, or `session_scoped`. This document adds `external_process`:

```rust
enum ExecutionClass {
    Inline,
    Queued,
    SessionScoped,
    ExternalProcess {
        command_template: CommandTemplate,
        output_adapter: OutputAdapter,
        env_preconditions: Vec<EnvPrecondition>,
    },
}

struct CommandTemplate {
    program: String,
    args: Vec<ArgTemplate>,
    working_dir: Option<WorkingDirSource>,
    timeout_seconds: u64,
}

enum ArgTemplate {
    Literal(String),
    SlotValue { slot_id: InputSlotId, field_path: Option<FieldPath> },
    BindingValue { binding_id: BindingId },
}

enum OutputAdapter {
    Structured {
        output_slot_id: OutputSlotId,
        expected_schema: ArtifactSchema,
    },
    LLMConversion {
        conversion_prompt: String,
        provider_ref: ProviderRef,
        output_slot_id: OutputSlotId,
        output_schema: ArtifactSchema,
        schema_hash: SchemaHash,
    },
}

struct EnvPrecondition {
    kind: EnvPreconditionKind,
    description: String,
}

enum EnvPreconditionKind {
    ExecutableInPath { name: String },
    FileExists { path: String },
    EnvVarSet { name: String },
}
```

## Sig Adapter Behavior

The sig adapter for an `ExternalProcess` capability performs three steps:

**Step 1: argument resolution**

Resolve `ArgTemplate` values from the invocation payload's supplied inputs and bindings.
Each `SlotValue` reads from the artifact repo entry for the named slot.
Each `BindingValue` reads from the compiled capability instance's bound values.
Literals pass through unchanged.

**Step 2: subprocess execution**

Run the resolved command. Capture stdout, stderr, and exit code.
If exit code is nonzero and the capability definition treats that as a hard failure,
return a structured failure output. If the definition treats nonzero as a soft failure
(partial output still useful), proceed to step 3 with a failure flag set.

**Step 3: output conversion**

If `OutputAdapter::Structured`: parse stdout directly as JSON and validate against
`expected_schema`. If validation fails, return a schema mismatch failure output.

If `OutputAdapter::LLMConversion`: assemble a conversion prompt incorporating the raw
stdout and the target schema description. Invoke the bound provider via `provider_execute_chat`.
Parse the LLM response as JSON and validate against `output_schema`. If validation passes,
emit the resulting artifact. If validation fails, return a conversion failure output.

The LLM conversion step is a standard `provider_execute_chat` invocation. It goes through
the existing provider execution path and is subject to the same retry and failure semantics.

## Contract Stability

A synthesized capability using `LLMConversion` has one stability risk: the LLM conversion
prompt may produce slightly different output schema across invocations, even with the same
raw input.

Two mechanisms address this:

**Schema hash**: the `output_schema` has a `schema_hash` stored with the capability definition.
The sig adapter validates the emitted artifact against the hash after conversion. If the hash
does not match, the invocation fails with `SchemaHashMismatch`. The runtime catalog detects
this as schema drift and can trigger re-synthesis.

**Conversion prompt design**: the synthesis task's `contract_definition` step should produce
a conversion prompt that includes the exact target schema as a JSON schema block. This reduces
LLM output variance significantly compared to a prose-only prompt.

## Environmental Preconditions

`env_preconditions` declares what the host environment must provide for the capability to
execute. The capability runtime checks these before executing the subprocess:

- `ExecutableInPath { name: "git" }` — `git` must be resolvable via PATH
- `FileExists { path: "/usr/bin/tree-sitter" }` — a specific binary must exist
- `EnvVarSet { name: "GITHUB_TOKEN" }` — an environment variable must be set

If any precondition fails, the invocation fails immediately with an `EnvPreconditionFailed`
output. This is surfaced to control as a hard failure. Control may respond by triggering a
`CapabilitySynthesisTask` with a different approach that does not require the missing tool.

## Trust and Capability Identity

An `ExternalProcess` capability carries a `trust_level: Synthesized` marker in the runtime
catalog. The task compiler accepts synthesized capabilities for slot wiring but the compiled
task record notes that one or more capability instances are synthesized. This affects repair
semantics: a synthesized capability that fails repeatedly may warrant re-synthesis rather
than continued retry.

The `capability_type_id` for a synthesized capability is derived from a hash of its
`ExecutionContract`, `input_contract`, and `output_contract`. This gives stable identity
across catalog entries: the same synthesis outcome registered twice produces the same id
and the catalog deduplicates.

## Example: Synthesized git_diff_summary

A synthesized equivalent of the compiled `git_diff_summary` capability:

```json
{
  "capability_type_id": "h3f8a2b1...",
  "trust_level": "Synthesized",
  "input_contract": [
    { "slot_id": "node_ref", "artifact_type_id": "node_ref", "required": true },
    { "slot_id": "reference_point", "artifact_type_id": "frame_ref", "required": false }
  ],
  "output_contract": [
    { "slot_id": "change_summary", "artifact_type_id": "change_summary" }
  ],
  "execution_contract": {
    "execution_class": "ExternalProcess",
    "command_template": {
      "program": "git",
      "args": ["diff", "--stat", "{reference_point.commit}", "--", "{node_ref.path}"],
      "timeout_seconds": 30
    },
    "output_adapter": {
      "kind": "LLMConversion",
      "conversion_prompt": "Convert the following git diff --stat output to a ChangeSummary JSON matching this schema: {...}. Output only valid JSON.\n\nGit output:\n{stdout}",
      "provider_ref": "provider/default",
      "output_slot_id": "change_summary",
      "output_schema": { "$ref": "change_summary_v1" },
      "schema_hash": "blake3:a1b2c3..."
    },
    "env_preconditions": [
      { "kind": "ExecutableInPath", "name": "git" }
    ]
  }
}
```

This produces the same `change_summary` artifact type as the compiled `git_diff_summary`
capability and satisfies the same input slot in the `bayesian_evaluation` capability chain.
The task compiler sees no difference. The execution path is subprocess + LLM conversion
rather than Rust + git2.

## Read With

- [Synthesis Task](synthesis_task.md)
- [Runtime Catalog](runtime_catalog.md)
- [Synthesis Overview](README.md)
- [Bayesian Evaluation Example](../../examples/bayesian_evaluation.md)
