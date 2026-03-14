# Write Policy

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Define side effect governance for file materialization so workflow outputs do not create unnecessary churn.

## HTN Position

- write policy governs side effecting atomic tasks and feeds workflow repair decisions
- write policy should never be hidden inside the file write implementation as an implicit local rule set
- write policy must stay deterministic for the same inputs and repository state
- divergence must be a first class state and not a silent overwrite case
- write policy should compile into explicit policy ids and versions so compiled plans can be replayed and audited against the same decision rules

## Provisional Answers

### Write Versus Skip

- a workflow should write only when normalized intended bytes differ from normalized current bytes and policy allows the target scope
- identical intended and current bytes should produce an explicit skip result rather than a write
- policy based suppression should produce a distinct skip reason from content equality

### Comparison Method

- intended content should be normalized according to the write policy profile before comparison
- current repository state should be read from the managed target path immediately before the write decision
- comparison should produce a durable reason code so later repair and telemetry can explain the outcome
- the resulting decision should preserve policy id, policy version, and comparison digest in the materialization record

### Workspace Refresh

- workspace state should be refreshed on the affected managed paths immediately after successful writes
- refresh scope should stay as narrow as practical so watch and scan behavior remain efficient
- managed output metadata should be recorded so watch mode can suppress self caused churn

### Divergence Handling

- when repository state diverges from the last workflow artifact, write policy should emit `divergence_detected`
- divergence should trigger a workflow level repair or halt decision rather than an automatic overwrite
- first phase policy should prefer safety and explicit operator visibility over aggressive auto merge behavior

## Initial Requirements

- compare intended file content to current file content before writing
- write only when resulting content is materially different
- keep write policy deterministic for the same inputs and repository state
- report write reason in telemetry and workflow summaries
- define how workflow writes interact with workspace updates and future watch behavior
- version policy rules so plan replay and parity checks remain explainable across migrations

## Policy Principles

- no blind overwrite
- normalize before compare
- emit stable reason codes
- refresh managed paths after successful writes
- escalate divergence to workflow repair logic

## Residual Questions

- which normalization rules are safe to make default without hiding meaningful repository differences
- when should first phase policy allow automated retry after divergence versus require explicit operator review

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [File Write Task](../file_write_task/README.md)
- [Telemetry Model](../telemetry_model/README.md)
- [Migration Plan](../migration_plan/README.md)
- [HTN Codebase Structure Report](../../research/htn/README.md)
