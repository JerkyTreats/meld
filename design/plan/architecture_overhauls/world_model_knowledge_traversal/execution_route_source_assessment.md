# WMR-VC-04 Execution Route Source Assessment

Date: 2026-09-04

Status: corrected, accepted, closed, committed, and pushed for exact candidate `WMR-VC-04-R1::5308f0ba6ce873bd1e00b4ba283d60eefc9df996f9ba261461fc44929c86c430`

Source baseline: `713ca3ee`

## Corrected Decision

`WMR-VC-04` carries the real accepted `WMR-VC-03` Docs Freshness Task through canonical Execution and returns its exact operational terminal position to Agent.

The earlier frozen trace incorrectly assumed `workspace_scan`. The accepted Task actually contains exactly these Capability types:

- `docs.inspect_scope`
- `docs.draft_patch_set`
- `docs.validate_patch_set`
- `docs.publish_patch_set`
- `docs.assess_published_scope`

The correction leaves PDS selection, the accepted Task body, and product theory unchanged. It does not construct a workspace candidate and does not substitute any separately authored Task.

## Runtime Ground

Agent already owns Planner cut assembly, immutable Strategy Plan construction, judgment, Task eligibility, fresh authorization, durable progress, and Plan milestone acceptance.

Execution owns consumer admission, exact validation, canonical-only lowering, the single Task Network, claims, capability invocation, durable operational outcomes, and neutral publication. Public commands cannot fabricate admission or insert attributed nodes. Canonical lowering durably seals each complete node, and dispatch verifies that exact executable content. Graph independently projects eligible appended Events. Agent accepts only the Plan-declared Execution terminal position.

The retired Execution planner duplicated semantic authority by selecting Goals, projecting world state, searching Methods, realizing actions, and lowering an `ExecutionComposition`. The retired package-plan handoff also bypassed durable Task Network claim selection. Neither survives the corrected candidate.

## Corrected Product Trace

```text
accepted Agent Plan Task with five Docs capabilities
-> fresh Agent Task authorization and exact authority decision
-> durable Execution Task admission
-> direct consumer validation
-> direct lowering of the admitted Task body
-> one attributed region in the single Task Network
-> fenced production dispatch of admitted Capability instances
-> durable operational terminal outcome
-> neutral Execution Event publication when publishable
-> independent Graph catchup over the appended Event
-> Plan-declared ExecutionTerminal acceptance by Agent
```

The exact proof endpoint is the durable `AgentMilestoneAcceptance.owner_position_id` equal to the admitted Task region terminal `Outcome.outcome_id`.

Execution operational completion, Event append, Graph visibility, and Agent milestone acceptance remain distinct positions. Operational success or failure does not establish Docs correctness, Belief settlement, Goal satisfaction, or owner truth.

## Responsibility Disposition

| Responsibility | Disposition | Surviving owner |
| --- | --- | --- |
| Agent Task selection and authorization | preserve | Agent and Strategy |
| Task consumer admission | move to canonical name | `execution.task_admission` |
| direct Task lowering | preserve | Task admission |
| attributed graph insertion and exact node sealing | preserve as canonical-only | Task admission and Task Network |
| Execution Goal writer and store | delete | none |
| Execution world projection | delete | none |
| Method search and action realization | delete | none |
| `ExecutionComposition` lowering | delete | none |
| package-plan handoff and dispatch | delete | none |
| durable Task Network command route | preserve | Task Network |
| durable claim dispatch | preserve | Task dispatch |
| neutral outcome publication | preserve | Execution publication |
| exact Graph projection | preserve where an Event exists | Graph runtime |
| terminal milestone acceptance | preserve | Agent |
| generic Task and explicit Workflow persistence | preserve | Task and Workflow domains |
| historical planning record decoding | retain as read-only | named Task Network legacy decoder |

## Deferred Owner Return

This slice does not realize `workspace_scan`, publish `workspace_event_candidates`, append an owner Event, or claim owner Graph visibility. Those positions are deferred until a real admitted Task produces an owner artifact through an authorized owner route.

No new workspace Capability contribution, product inventory entry, owner publication source, tests, or documentation remain in this candidate solely for that deferred route.

## Persistence Assessment

The configured Execution root was inspected before retirement. The legacy Goal database and current Task stores contained no application records requiring a writable compatibility path. Historical Task Network data remains readable through a named decoder. Any network containing retired planning lineage is write-fenced.

Canonical startup no longer opens or creates the legacy Goal database. Current mutation constructors emit `source_task_id` and current Task lineage constructors cannot emit retired composition, Goal, Method, or planning-frame fields.

## Proof Obligations

- root registers `execution.task_admission` exactly once
- root registers neither `execution.planning` nor `execution.goal_set`
- public commands cannot create admission decisions or attributed graph regions
- the real accepted Task is the admission source
- exactly five Docs Capability nodes enter one attributed region
- exact full-node content is durably sealed and rechecked before direct claim, fresh invocation, and resumed invocation
- no Method search, semantic repair, or synthetic workspace Task occurs
- claim dispatch retains live policy and activation-generation fences
- artifacts persist before terminal outcome recording
- neutral publication remains separate from operational completion
- Graph catchup remains separate from Event append
- Agent terminal acceptance cites the exact operational outcome
- explicit Workflow behavior remains green and cannot carry Agent authority without admission
- historical Task Network records replay through the named read-only decoder

## Exclusions

The slice does not authorize PDS changes, Task replacement, workspace owner return, Docs truth, Belief settlement, Goal satisfaction, lifecycle redesign, Startup work, product migration, `WMR-VC-05`, commit, push, or deployment.
