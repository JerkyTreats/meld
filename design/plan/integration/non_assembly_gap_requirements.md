# Non Assembly Gap Fix Requirements

Date: 2026-06-06
Status: proposed
Scope: contract and domain fixes that unblock end to end assembly for the minimal runtime flywheel

## Purpose

This document breaks out the gaps that prevent the runtime flywheel from being assembled end to end, while avoiding the assembly work itself.

The target outcome is a set of narrow requirements that can be implemented and tested inside existing domains before introducing the durable runtime host. These requirements focus on handoff contracts, publication, belief evidence, satisfaction review, and failure behavior.

## Relationship To Assembly

These fixes are prerequisites for assembly. They should not create the runtime host, long running worker graph, CLI entrypoint, or integrated service loop.

Each fix should leave behind a clear callable contract and a focused test so assembly can later wire the pieces together without inventing semantics at the boundary.

## In Scope

- Convert curated agent goal commands into execution goal commands.
- Publish task network outcomes to the event spine through an explicit bridge.
- Convert relevant execution outcomes into belief evidence through configured mappings.
- Review active goals against updated world state and satisfy goals through execution contracts.
- Define failure behavior so failed work cannot accidentally satisfy goals.

## Out Of Scope

- A complete durable runtime host.
- A background worker scheduler.
- A CLI command for the flywheel.
- Storage migrations outside the contracts needed for these gaps.
- Broad refactors of world model, execution, or language domains.

## Selection Rule

A gap belongs in this document when assembly would otherwise need to make a semantic decision that should live in a domain contract.

A gap does not belong here when it is only about call order, worker lifetime, dependency injection, or service startup.

## Requirement Summary

| ID | Gap | Primary Domain | Required Proof |
| --- | --- | --- | --- |
| NAG-1 | Producer-neutral goal acceptance | execution | Implemented at [goal API](../../../crates/meld-execution/src/goals/api.rs) and proven by `producer_neutral_goal_acceptance_stores_active_plannable_goal` |
| NAG-2 | Outcome publication bridge | execution | Implemented at [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) and proven by `publication_bridge_appends_pending_task_outcome_once` |
| NAG-3 | Outcome fact to belief evidence | world model | Implemented at [promoted evidence ingestion](../../../crates/meld-world-model/src/belief/ingestion.rs), [outcome evidence mapper](../../../src/execution/outcome_evidence.rs), and proven by `docs_writer_success_promotes_configured_freshness_evidence` |
| NAG-4 | Satisfaction review | world model agent plus execution boundary | Implemented at [agent curation](../../../crates/meld-world-model/src/agent/curation.rs), [goal mutation adapter](../../../src/execution/goal_mutation.rs), and proven by `agent_satisfaction_curation_marks_goal_satisfied_only_after_world_state_match` |
| NAG-5 | Failure outcome contract | execution | Implemented across [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs), [outcome evidence mapper](../../../src/execution/outcome_evidence.rs), [agent satisfaction curation](../../../crates/meld-world-model/src/agent/curation.rs), and proven by `failure_outcome_does_not_satisfy_goal` |

## NAG-1 Producer-Neutral Goal Acceptance

World model agent curation can emit an `AgentGoalCommand` when a belief crosses a rule threshold. Integration maps that producer-specific output to `GoalAcceptanceRequest`. Execution accepts the neutral request through `GoalSetApi.accept_goal`, stores goals through `AddGoalCommand`, and planning only accepts active goals.

Status: `implemented`

The implemented fix is an explicit producer-neutral execution acceptance API. Producer-specific validation stays in world model, and the integration boundary maps curation output into execution's request shape.

### Requirements

- Add a callable execution facade that accepts `GoalAcceptanceRequest`.
- Keep `AgentGoalCommand` validation in world model producer contracts.
- Map `AgentGoalCommand` to `GoalAcceptanceRequest` in integration boundary code, not execution core.
- Reject neutral requests with empty command id, empty agent id, or non ground target.
- Convert `GoalLifecycle::Proposed` to `GoalLifecycle::Active` at the execution acceptance boundary.
- Preserve idempotency by deriving a stable execution command id from the curation command id or by reusing the curation command id directly.
- Set `GoalCommandMetadata.source_identity` from the curation dedupe identity when available.
- Duplicate delivery of the same neutral request must return the existing execution goal outcome.
- Execution must not import world model producer types.

### Acceptance Proof

- A low confidence curation rule emits a proposed docs freshness goal.
- Integration maps the curation output into a neutral acceptance request.
- Goal acceptance stores one active execution goal.
- Replaying the same acceptance request does not create a second execution goal.
- Planning accepts the stored goal.

Suggested focused test name:

```text
producer_neutral_goal_acceptance_stores_active_plannable_goal
```

Implementation evidence:

- [goal API](../../../crates/meld-execution/src/goals/api.rs)
- [goal acceptance test](../../../tests/integration/goal_acceptance.rs)
- `cargo test --test integration_tests producer_neutral_goal_acceptance_stores_active_plannable_goal`

## NAG-2 Outcome Publication Bridge

Task network storage can create pending `Publication` records for task outcomes. The event spine is the shared source for subsequent world state reduction and belief reassessment.

Status: `implemented`

The implemented fix is an explicit callable bridge from retryable task outcome publications to the event spine.

### Requirements

- Add a publication worker or callable bridge that reads pending task outcome publications.
- Convert each publication into a canonical event spine envelope.
- Use a deterministic event id or record identity derived from the publication id so retry is idempotent.
- Preserve the publication event type and serialized payload.
- Preserve enough task identity to connect the event back to the task run and artifacts.
- Mark a publication as `Published` only after append succeeds and returns an event cursor.
- On append failure, retain a retryable state and record the error through the publication contract.
- Do not require the future durable runtime host to know how task outcome payloads become events.

### Acceptance Proof

- A recorded task success creates one pending publication.
- Running the bridge appends exactly one event to the event spine.
- Re-running the bridge does not append a duplicate event.
- The publication is marked published only after successful append.
- A simulated append failure does not mark the publication as published.

Suggested focused test name:

```text
publication_bridge_appends_pending_task_outcome_once
```

Implementation evidence:

- [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs)
- [publication bridge test](../../../crates/meld-execution/tests/task_network_publication_bridge.rs)
- `cargo test -p meld-execution --test task_network_publication_bridge publication_bridge_appends_pending_task_outcome_once`

NAG-5 is covered by the failure outcome contract below.

## NAG-3 Outcome Fact To Belief Evidence

Docs writer success must become evidence that can refresh the docs freshness belief. The belief runtime already supports configured evidence mappings and promoted evidence records. The implemented fix maps the chosen execution outcome fact into that generic evidence path.

Status: `implemented`

The implemented fix keeps world model ingestion generic and maps docs task success to promoted evidence at the root integration boundary.

This is a callable contract, not runtime assembly. A later durable runtime host still needs to invoke the mapper and ingestion after the publication bridge delivers an applicable docs writer success event.

### Requirements

- Use configured evidence mappings rather than hardcoding docs freshness behavior into generic reducers.
- Treat docs writer success as a source kind such as `content_written` when the configured family maps that source kind.
- Build promoted evidence with a stable source cursor from the event spine cursor.
- Assign evidence to the affected subject or dirty belief key before reassessment.
- Reassess the dirty belief through the belief runtime after evidence assignment.
- Define whether task failure creates no support evidence or explicit contradictory evidence.
- Keep domain family names and source kinds as data.

### Acceptance Proof

- A docs writer success event creates promoted evidence for the configured subject.
- Reassessment updates the docs freshness belief from that evidence.
- The same event cursor cannot create duplicate effective evidence.
- A failure event follows the explicit failure evidence rule and does not count as fresh content.
- A non docs task success does not count as docs freshness evidence.

Suggested focused test name:

```text
docs_writer_success_promotes_configured_freshness_evidence
```

Implementation evidence:

- [promoted evidence ingestion](../../../crates/meld-world-model/src/belief/ingestion.rs)
- [outcome evidence mapper](../../../src/execution/outcome_evidence.rs)
- [outcome evidence test](../../../tests/integration/outcome_evidence.rs)
- `cargo test --test integration_tests docs_writer_success_promotes_configured_freshness_evidence`

## NAG-4 Satisfaction Review

Execution can mark goals satisfied through `SatisfyGoalCommand`, and the language domain can evaluate grounded goals against world state. The implemented fix is an agent satisfaction curation boundary that runs after world model updates and maps agent-authored mutation commands into execution's public satisfy API.

Status: `implemented`

Assessment and implementation evidence:

- [goal ownership boundary audit](goal_ownership_boundary_audit.md)
- [satisfaction review assessment](nag_4_satisfaction_review_assessment.md)
- [agent contracts](../../../crates/meld-world-model/src/agent/contracts.rs)
- [agent satisfaction curation](../../../crates/meld-world-model/src/agent/curation.rs)
- [goal mutation adapter](../../../src/execution/goal_mutation.rs)
- [goal acceptance integration test](../../../tests/integration/goal_acceptance.rs)

### Requirements

- Add a world model agent satisfaction path that reads active execution goals and a current world state projection.
- Evaluate each goal target with `meld_lang::evaluate` or an equivalent projection-backed belief check.
- Only call execution's satisfy API when evaluation returns satisfied.
- Use the event cursor or world state sequence as `at_seq`.
- Use a stable idempotency command id derived from the goal id and reviewed cursor.
- Leave goals active when evaluation is unsatisfied or indeterminate.
- Return diagnostics for unsatisfied and indeterminate outcomes so assembly can report progress without inventing state transitions.

### Acceptance Proof

- An active docs freshness goal remains active before supporting world state evidence exists.
- After evidence updates world state enough to satisfy the target, the agent satisfaction path marks the goal satisfied through execution's API.
- Re-running the agent satisfaction path at the same cursor is idempotent.
- Indeterminate evaluation never satisfies the goal.

Suggested focused test name:

```text
agent_satisfaction_curation_marks_goal_satisfied_only_after_world_state_match
```

Implemented proof command:

```sh
cargo test --test integration_tests agent_satisfaction_curation_marks_goal_satisfied_only_after_world_state_match
```

## NAG-5 Failure Outcome Contract

The flywheel must be able to fail without producing false satisfaction. Task outcomes already distinguish success and failure. The contract now defines what failure means to goals, evidence, and publication.

Status: `implemented`

The implemented proof keeps failure semantics at existing boundaries. Execution task outcome publication emits `execution.task.failed`, the docs freshness mapper treats that event as no support evidence, agent satisfaction curation emits no mutation while the projected goal target remains unsatisfied, and planning `NoApplicableMethod` or `Indeterminate` results provide no dispatchable composition.

### Requirements

- Task failure must publish a failure fact through the same publication bridge as success.
- Failure publication must not create support evidence for docs freshness.
- Failure must not issue `SatisfyGoalCommand`.
- No applicable method and indeterminate planning must not dispatch a task.
- The goal should remain active unless a separate explicit command transitions it to another lifecycle state.
- The failure contract must include enough detail for assembly to decide whether to retry, pause, or surface an error.

### Acceptance Proof

- A failed task run publishes a failure event.
- Belief evidence mapping follows the explicit failure rule.
- Satisfaction review leaves the goal active.
- Planning failure or no method does not create task network work.

Suggested focused test name:

```text
failure_outcome_does_not_satisfy_goal
```

Implemented proof command:

```sh
cargo test --test integration_tests failure_outcome_does_not_satisfy_goal
```

## Dependency Order

NAG-1 should land first because planning needs active execution goals.

NAG-2 should land next because later evidence and satisfaction work need durable outcome facts.

NAG-3 and NAG-4 can land after publication because they consume the published facts and updated world state.

NAG-5 should be defined before or alongside NAG-2 so failure publication and success publication share one contract.

## Exit Criteria

The non assembly gap set is complete when each requirement has a focused characterization test and no future assembly code needs to decide:

- how curated goals become execution goals
- how task outcomes reach the event spine
- how docs writer outcomes become belief evidence
- how active goals become satisfied
- how failed work avoids false satisfaction

After that point, a runtime assembly plan can focus on sequencing and worker lifecycle only.
