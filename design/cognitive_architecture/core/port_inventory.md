# Execution Port Inventory

Date: 2026-04-25
Status: active
Scope: execution-owned ports and root adapter responsibilities

## Purpose

This inventory defines the contracts that let execution consume product capabilities without depending on root crate internals.

Execution owns each port trait and its request and response types. The root product crate owns concrete adapters, dependency injection, storage discovery, and product configuration.

## Execution Owned Ports

| Port | Contract | Root adapter owner |
| --- | --- | --- |
| `ContextReadPort` | read node context, frame heads, and frame composition needed by execution | `src/context` |
| `ContextWritePort` | write frames and head mutations produced by execution outcomes | `src/context` |
| `PromptArtifactReadPort` | read and persist prompt context artifacts used by execution | `src/prompt_context` |
| `SystemPromptPort` | load the configured system prompt for an agent | context and agent adapters |
| `NodeResolutionPort` | resolve workspace-rooted node paths and identifiers | `src/workspace` and CLI routing |
| `ProviderExecutionPort` | execute provider-backed generation requests | `src/provider` |
| `ProviderValidationPort` | validate configured providers and provider bindings | `src/provider` and config adapters |
| `PromptLineagePort` | construct and persist prompt lineage for generation requests | prompt context and generation adapters |
| `GeneratedMetadataPort` | read prior generation metadata and build validated output metadata | context generation adapters |
| `EventPublicationPort` | publish canonical event envelopes and idempotent factual outcomes | `src/events` through the product event authority |
| `ExecutionProgressPort` | emit interactive progress without becoming durable event authority | CLI progress adapters |
| `WorkspaceScanPort` | execute workspace scans through the workspace domain boundary | `src/workspace` |
| `WorldModelQueryPort` | read current artifact anchors for task runs | world model query adapters |
| `BeliefContextReadPort` | read planner-safe belief signals for context assembly | world model query adapters |
| `WorkflowProfileLoadPort` | load workflow profiles and command-facing workflow definitions | `src/workflow` and config adapters |
| `PlanningProjectionPort` | project goal-scoped `WorldState` with durable frame provenance | world model planner adapters |

## Adapter Invariants

- Root adapters implement execution-owned traits without exposing root internals.
- Execution request types express the complete cross-domain contract.
- Adapters parse product storage and route formats, then delegate to the owning domain.
- Event publication uses the product event authority and never opens a second canonical writer.
- World model reads and planning projections use public contracts and never reach into graph or belief storage internals.
- Context, provider, workflow, and workspace adapters do not own execution policy.
- Cross-domain failures are translated into explicit port errors at the boundary.

## Composition

The root product crate assembles these adapters and provides them to execution through explicit dependency injection. An adapter may coordinate more than one product service only when the execution port contract names that coordination as one semantic operation.

The product shell remains responsible for user-facing route parsing, configuration, and storage location. Execution remains responsible for planning, task network commands, task and capability dispatch, and outcome publication intent.

## Read With

- [Cognitive Architecture](../README.md)
- [Core Crate](CRATE.md)
- [Execution Domain](../execution/README.md)
- [Execution Integration Contracts](../execution/GAPS.md)
- [World Model Public Interface](../world_model/public_interface.md)
- [Events Domain](../events/README.md)
