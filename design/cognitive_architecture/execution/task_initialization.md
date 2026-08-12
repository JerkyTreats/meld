# Task Initialization

Date: 2026-06-04
Status: active
Scope: task run seed artifacts, data flow materialization, and validation authority

## Core Position

Task initialization is the boundary between a compiled task graph and one concrete task run.

A task declares required init slots. A task run supplies init artifacts for those slots. The task executor seeds those artifacts into the task local artifact repository before it computes ready capability instances.

`TaskInitializationPayload` is the materialized task run envelope. It is not the semantic authority for artifacts. Semantic authority belongs to artifact contracts keyed by artifact type and schema version, plus source rules owned by the task network.

## Payload Role

The payload carries:

- task identity
- compiled task reference
- init artifacts
- task run context

The payload must satisfy the compiled task init slot contract before a task executor starts.

Validation requires:

- payload task id matches the compiled task id
- every supplied init slot exists in the compiled task
- each init slot is supplied at most once
- every required init slot is present
- artifact type matches the init slot contract
- schema version matches the init slot contract

The payload does not decide whether a seed object is trustworthy, semantically valid, or allowed by policy.

## Seed Objects

Init artifacts are seed objects for a task run. They are external to the task local capability graph, but once seeded they become task local artifacts produced by the synthetic task init producer.

Capability inputs can consume seed objects through task init slot wiring. After seeding, capability invocation assembly treats init artifacts and upstream capability outputs through the same artifact repository model.

## Source Tasks And Data Flow Tasks

There is no distinct task kind for source tasks and data flow tasks. Both are task network task nodes with compiled task records and required init slots.

A source task has all required init artifacts available before dispatch. Static seeds may come from lowering, workflow package triggers, goal context, target selectors, or planner supplied constants.

A data flow task has at least one required init artifact whose content is produced by an upstream task. The task can be present in the graph before that content exists, but it cannot be dispatched until the upstream artifact is available and materialized into its init payload.

The difference is source timing:

- source task: final init payload is complete before graph commit or before claim
- data flow task: final init payload is completed after upstream outcomes exist

## Init Source Plan

The task network owns the source plan for each required init slot.

Source plan kinds:

```text
StaticSeed
UpstreamArtifact
PreservedArtifact
SyntheticSeed
```

Static seeds and upstream artifacts are the foundational source plan kinds. Preserved artifacts and synthetic seeds extend the same source plan contract.

Every required init slot must have exactly one source plan. Optional init slots may have zero or one source plan unless the task contract later declares many-valued slots.

## Data Flow Materialization

Data flow materialization runs after readiness confirms dependency satisfaction and before task executor construction.

For each upstream artifact source, materialization selects an artifact from the named upstream task.

Selection requires:

- upstream task identity matches the source plan
- upstream task outcome is accepted in the task network journal
- artifact type matches the expected upstream artifact type
- artifact type matches the downstream init slot artifact type
- schema version matches the downstream init slot schema version
- artifact is not superseded or invalidated in the current task network revision

If no artifact matches, dispatch is blocked. If more than one artifact matches and no selection rule exists, dispatch is blocked.

The materialized result is an ordinary `TaskInitializationPayload` that must pass task initialization validation.

## JSON And Artifact Contracts

Artifact content is an open JSON wire shape.

JSON is the durable interchange format for task packages, task network records, fixtures, event payloads, and externally executed capabilities.

JSON is not the full validation contract.

The full validation model has layers:

- envelope validation over slot id, artifact type, schema version, uniqueness, and required presence
- schema validation for registered artifact types
- semantic validation for domain invariants
- source validation for allowed static and upstream sources
- data flow validation for graph revision and artifact provenance
- policy validation for scope, agent, workspace, and trust posture
- evolution validation for deterministic schema migration

The task network must enforce envelope and source validation before dispatch. Readiness validates each declared source's artifact type and schema version, and a blocked task narrates its per-instance reasons; semantic validators register by artifact type and schema version on the same seam.

## Dispatch Invariant

A task may be claimed only when graph dependencies are satisfied.

A claimed task may be executed only when its init payload is materialized and validated.

Claiming and materialization must preserve fencing. The task instance id, lifecycle epoch, claim id, claim revision, and network revision used for materialization must appear in the outcome path so stale outcomes cannot advance task state.

## Non Ownership

Task initialization does not own capability execution.

Task initialization does not own belief truth.

Task initialization does not own provider transport.

Task initialization does not own broad artifact policy. It invokes artifact contract and policy validators supplied by adjacent domains.

## Read With

- [Task Network](task_network.md)
- [Planning Pipeline](planning/planning_pipeline.md)
- [Execution Domain](README.md)
