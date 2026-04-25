# Assessment By Domain Policy

Status: active

Date: 2026-04-21

## Purpose

Assessment By Domain is a structured review method for architectural concerns that may span multiple top level domains.

The goal is to make cross domain impact explicit without violating the maxim that each domain must remain true to its own source truth, behavior, and storage concerns.

This policy is a review method. It does not create a requirement that every architectural concern integrate with every domain.

## When To Use

Use Assessment By Domain when an architectural concern affects or may affect three or more top level `src` modules.

Use it when reviewing a substrate, layer, federation model, event spine, traversal model, storage policy, workflow handoff, compatibility migration, or other concern whose scope may be unclear.

Do not use it for local single domain changes unless the change is explicitly establishing a cross domain contract.

Do not use it to force needless integration. A correct answer for a domain may be `not needed`.

## Core Rules

- Define the concern before assessing domains.
- Define what the concern does and does not do.
- Assess each domain by its own responsibilities.
- Record non integration explicitly when no integration is needed.
- Separate current state from required state.
- Use evidence from code, tests, design artifacts, or runtime behavior.
- Do not move domain behavior into the substrate being assessed.
- Do not make adapters authoritative for domain truth.

## Current Domain Snapshot

This snapshot is advisory. Regenerate it at the start of each assessment.

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

| Domain | Category | Default Assessment Question |
| --- | --- | --- |
| `agent` | Product domain | Does the concern affect agent identity, role, or configuration ownership |
| `api` | Adapter | Does the concern require facade wiring only |
| `branches` | Product domain | Does the concern affect branch identity, branch registration, federation, or branch scoped state |
| `capability` | Product substrate | Does the concern affect capability contracts or invocation boundaries |
| `cli` | Adapter | Does the concern require command surface, help text, or formatting only |
| `concurrency` | Infrastructure | Does the concern affect runtime coordination or shared execution limits |
| `config` | Infrastructure | Does the concern affect loaded configuration contracts |
| `context` | Product domain | Does the concern affect frames, heads, generation, queue, or context query |
| `control` | Product domain | Does the concern affect orchestration plans or node outcome state |
| `error` | Infrastructure | Does the concern need stable error mapping or error contracts |
| `events` | Substrate | Does the concern affect spine event contracts, append, replay, or compatibility |
| `heads` | Compatibility support | Does the concern touch legacy head index behavior |
| `ignore` | Infrastructure | Does the concern affect ignored path or file selection policy |
| `init` | Adapter | Does the concern require initialization assets or bootstrap behavior |
| `lib` | Crate surface | Does the concern require public exports |
| `logging` | Infrastructure | Does the concern affect logs only |
| `merkle_traversal` | Infrastructure | Does the concern affect tree traversal strategy |
| `metadata` | Product domain | Does the concern affect metadata schema, metadata policy, or metadata storage |
| `prompt_context` | Product domain | Does the concern affect prompt artifacts or prompt lineage |
| `provider` | Product domain | Does the concern affect provider profile or execution binding |
| `session` | Lifecycle domain | Does the concern affect command lifecycle separate from durable facts |
| `store` | Infrastructure | Does the concern affect node store or persistence primitives |
| `task` | Product domain | Does the concern affect task compile, run, artifacts, or package behavior |
| `telemetry` | Observability domain | Does the concern affect summaries, session observation, or downstream observability |
| `types` | Crate surface | Does the concern affect shared public types |
| `views` | Presentation domain | Does the concern affect read models or presentation shaped output |
| `workflow` | Product domain | Does the concern affect workflow profiles, turns, gates, or state |
| `workspace` | Product domain | Does the concern affect workspace source truth, scans, watch, publish, or status |
| `world_state` | Product substrate | Does the concern affect graph, projection, claims, or query over facts |

## Assessment Template

Use this table for each assessed concern.

| Domain | Needed Integration | Current Integration | Completeness | Evidence | Non Integration Rationale | Follow Up |
| --- | --- | --- | --- | --- | --- | --- |
| `domain_name` | `none`, `observe`, `publish`, `consume`, `own`, or `adapter` | Current state summary | `not needed`, `not started`, `partial`, `complete`, or `blocked` | Code, test, or design links | Required when needed integration is `none` | Concrete action or `none` |

## Integration Levels

| Level | Meaning |
| --- | --- |
| `none` | The concern does not truthfully apply to this domain |
| `observe` | The domain only needs visibility for diagnostics, status, or compatibility |
| `publish` | The domain emits facts, events, records, or state into the concern |
| `consume` | The domain reads the concern as an input to domain behavior |
| `own` | The domain owns part of the concern implementation or source truth |
| `adapter` | The domain only exposes, routes, formats, or wires another domain contract |

## Procedure

1. State the concern in one paragraph.
2. State explicit in scope behavior.
3. State explicit out of scope behavior.
4. Regenerate the top level domain list from the current repository.
5. Fill one row per top level domain.
6. Mark domains as `none` when integration would be false to the domain.
7. Record evidence for every `partial`, `complete`, and `blocked` row.
8. Extract only the rows with real needed integration into implementation work.
9. Keep adapter work separate from authoritative domain behavior.
10. Update the assessment if a new top level domain is added during the change.

## Required Output

An Assessment By Domain artifact must include:

- Concern definition.
- In scope list.
- Out of scope list.
- Domain assessment table.
- Gaps and follow ups.
- Explicit non integration notes for domains where `none` is not obvious.
- Evidence date.
- Domain snapshot command output or a link to the source used for the snapshot.

## Anti Patterns

- Treating a substrate as the owner of every domain behavior it touches.
- Adding integration because a domain exists rather than because the domain needs it.
- Hiding missing integration behind vague layer language.
- Calling a concern complete without saying which domains are not started, partial, complete, blocked, or not needed.
- Making a CLI, API, or formatting adapter the authoritative implementation of domain truth.
- Reusing a table from an old assessment without regenerating the domain list.

