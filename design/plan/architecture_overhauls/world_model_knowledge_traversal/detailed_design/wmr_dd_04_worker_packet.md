# WMR-DD-04 Detailed Design Worker Packet

Date: 2026-08-22

Slice: `WMR-DD-04`, Executable Admission And Observation Return

Status: historical worker packet, delivery complete

Implementor: Codex detailed-design implementor

## Product Increment

Close one complete executable vertical from an Agent-authorized Task through Execution admission, durable realization, outcome publication, semantic-owner observation, independent downstream visibility, and Agent milestone absorption.

The slice proves design behavior only. It does not implement runtime contracts.

## Accepted Maturity Envelope

Posture: `exploratory`

Obligation floor: `operational durability for incumbent records and recovery seams`

Hard limits:

- zero source-code changes
- zero new crates, workspace members, stores, services, background runtimes, migration systems, or compatibility systems
- preserve Execution ownership of executable realization and uncertain effects
- preserve semantic-owner authority over returned observations
- preserve Events as neutral carriage
- do not reactivate legacy Method search or Workflow as canonical planning
- do not design PDS compilation, Agent genesis, or activation-wide lifecycle closure

## Direct Product Proof

The missing README trace must show one complete Task accepted without re-planning, realized through exact claims and attempts, published as an Execution outcome, re-observed by workspace and docs owners, and absorbed through exact owner milestones. Task success alone cannot prove README correctness or Goal satisfaction.

The dependency-security trace must preserve mitigation attempt, inventory observation, advisory applicability, assessment, and verification as distinct products.

## Active Scope

- `WMR-H13` Agent authorization to Execution admission
- `WMR-H14` Execution admission to Task Network
- `WMR-H15` durable route selection to package or claimed-task dispatch
- `WMR-H16` ready Task to exact claim and attempt
- `WMR-H17` Execution outcome to Events
- `WMR-H32` Execution outcome to semantic-owner observation
- `WMR-H33` returned owner observation to Events
- `WMR-H34` returned owner Event to Graph visibility
- `WMR-H35` returned owner Event to configured Belief revision
- `WMR-H36` return-derived Belief revision to Agent acceptance

## Owned Write Scope

- this worker packet
- `execution_admission_and_observation_transition_ledger.md`
- `execution_admission_and_observation_return.md`
- `../world_model_reconciliation_handoff_ledger.md`
- `../delivery_gates/wmr_dg_04_execution_admission_and_observation_return.md`
- later review, Gate Acceptance, program-ledger, and folder-index evidence for this slice

## Existing Seams

- accepted `WMR-DD-03` Task authorization envelope and progression contract
- Execution Goal admission, lowering, Task Network journal, claims, outcomes, and publication outboxes
- deterministic Event append and independent Graph, Belief, and Agent consumer positions
- accepted `WMR-DD-01` owner publication and completeness contract
- accepted `WMR-DD-02` configured Belief settlement

## Forbidden Changes

- Rust types, storage schemas, API signatures, migrations, or implementation sequencing
- Strategy planning inside Execution
- Task outcome interpreted as semantic-owner truth
- Event append interpreted as Graph, Belief, Agent, or Goal completion
- process-local route state treated as a durable handoff
- activation generation aggregation, global quiescence, or retirement design

## Review And Gate

Frozen Delivery Gate: [WMR-DG-04 revision 1](../delivery_gates/wmr_dg_04_execution_admission_and_observation_return.md)

Integrated review: one dedicated read-only subagent initial recommendation and one bounded verification after accepted corrections.

Gate Acceptance: one distinct read-only subagent recommendation over the exact integrated candidate.

Commit expectation: one accepted delivery commit before `WMR-DD-05` activation.
