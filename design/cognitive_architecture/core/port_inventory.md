# Initial Port Inventory

Date: 2026-04-25
Status: frozen for migration start
Scope: execution owned ports and root adapter responsibilities

## Purpose

Phase 0 freezes the first port list before execution boundary work starts.
Later phases may refine names, but new responsibilities should not appear without updating this inventory and the plan.

## Execution Owned Ports

| Port | Purpose | Expected root adapter home |
| --- | --- | --- |
| `ContextReadPort` | read node context, frame heads, and frame composition needed by execution | `src/context` and root adapter wiring |
| `ContextWritePort` | write frames and head mutations needed by execution outcomes | `src/context` and root adapter wiring |
| `PromptArtifactReadPort` | load prompt context artifacts and prompt templates | `src/prompt_context` |
| `NodeResolutionPort` | resolve workspace rooted node paths and identifiers | `src/workspace` and CLI routing |
| `ProviderExecutionPort` | execute provider backed generation requests | `src/provider` |
| `ProviderValidationPort` | validate configured providers and provider bindings | `src/provider` and config adapters |
| `EventPublicationPort` | publish canonical event envelopes and idempotent derived facts | `src/events` through progress or event runtime adapters |
| `WorldModelQueryPort` | read traversal anchors, provenance, and legacy claim compatibility views | `src/world_state` query services |
| `WorkflowProfileLoadPort` | load workflow profiles and command facing workflow definitions | `src/workflow` and config adapters |

## Root Adapter Rules

- root owns concrete adapters that implement these ports
- execution owns the traits and request contracts that consume these ports
- `ContextApi` may wrap multiple adapters during migration, but it is not the target port surface

## Phase Links

- Phase 3 extracts the traits and request contracts
- Phase 4 moves workflow runtime onto explicit adapter wiring
- Phase 5 removes remaining ambient root facade reach through

## Read With

- [PLAN](../PLAN.md)
- [Core Migration](MIGRATION.md)
- [Execution Contract Extraction](../completed/execution_contract_extraction.md)
