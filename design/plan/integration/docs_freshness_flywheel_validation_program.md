# Docs Freshness Flywheel Validation Program

Date: 2026-07-14
Status: complete for first-turn validation
Scope: prove one useful cognitive flywheel turn before further reliability platform work
Branch: `production-cognitive-runtime-closure`

## Why This Program Exists

The prior production closure program optimized durability, recovery, authority, and process hosting before the product loop had been demonstrated. That ordering is now superseded.

The controlling question is simple: can Meld observe stale documentation, create a goal, plan useful work, run the real docs writer, publish the result, revise its belief, and satisfy the goal?

Until the answer is yes, reliability work is admitted only when a concrete failure blocks that path.

## Objective

Deliver one executable `docs_freshness` vertical with this path:

```text
activation document and target workspace
-> initial stale or uncertain observation
-> belief revision
-> active docs freshness goal
-> planner projection and method selection
-> real docs writer task package
-> real provider-backed task execution
-> task outcome publication
-> evidence ingestion and revised belief
-> satisfied goal
```

The first proof may run in one process with a deterministic actor order. It must exercise the real domain implementations and real provider adapter, but it does not need to prove every restart or concurrency boundary.

## Validation Result

The answer to the controlling question is yes.

The runtime now completes one real `docs_freshness` turn through supervisor ticks. It starts from activation, appends the allowed initial observation, revises belief, hydrates the agent, curates a goal, projects planner state, lowers the selected method into a task network, scans the workspace, calls the real four-turn docs writer path through a deterministic local provider, writes the target README, publishes the task outcome, ingests the resulting evidence, revises belief again, and satisfies the original goal.

The same vertical is exposed through:

```text
meld runtime run --activation path/to/activation.toml
```

The command path is covered by an integration proof that asserts the provider calls, final documentation, published task outcome, revised belief, and satisfied goal.

## Success Gate

The vertical is successful when one focused integration test proves all of the following:

- it starts from the checked-in activation document and a real fixture workspace
- one explicit environment observation supplies the initial stale or uncertain signal
- the world model creates the active goal
- execution planning lowers that goal into the configured docs writer package
- the existing workspace scan and docs writer capability path runs
- the deterministic provider observer records a real provider call
- execution records a successful outcome and publishes it through the event authority
- the evidence and satisfaction actors revise belief and satisfy the same goal
- the final documentation artifact is present and contains expected generated content
- the test driver does not directly insert the goal, planning mutation set, task outcome, success evidence, or satisfaction mutation

The initial environment observation is an allowed validation seam because the sensory domain is deferred. It must enter through a public event, graph, or belief input contract. Direct test writes to internal belief tables are not allowed.

## Delivery Sequence

### Slice V1 — Semantic Path Starts

Status: complete

Keep the concrete Wave 3 actor factories that directly advance the loop. Supply the minimum initial observation needed to produce the first belief revision and operational agent.

Completion gate:

- activation reaches a real belief revision
- hydration reaches operational state
- goal curation creates one active goal

### Slice V2 — Real Task Dispatch

Status: complete

Reuse the existing task package path instead of designing a new reliability subsystem. Add one execution actor or adapter that performs this bounded sequence:

```text
read ready task
-> claim through task network authority
-> materialize initialization
-> run existing task executor and capability registry
-> record outcome through task network authority
```

Reuse `prepare_registered_workflow_task_run`, `execute_task_to_completion`, the built-in docs writer package, workspace scan capabilities, provider capabilities, and existing outcome helpers.

Completion gate:

- planner-produced task work reaches the existing docs writer implementation
- provider observer sees the expected calls
- task network contains a real successful outcome with real artifacts

### Slice V3 — Close The Loop

Status: complete

Run publication, evidence ingestion, assessment, and satisfaction through their concrete actors until the original goal is satisfied.

Completion gate:

- one actor-driven integration proof reaches final goal satisfaction
- no direct semantic writes exist in the proof driver after the initial observation
- actor reports provide enough diagnostics to locate a blocked stage

### Slice V4 — Product Entry Point

Status: complete

Expose the proven path through the smallest user-facing foreground command. Reuse current activation and runtime surfaces where possible.

Completion gate:

- a user can point Meld at a workspace and activation document
- the command runs or clearly reports the same proven vertical

### Slice V5 — Failure-Driven Hardening

Status: complete for failures exposed by the first-turn proof

Review failures actually observed while building and running V1 through V4. Add only the recovery behavior needed for those failures or for one immediately credible user risk.

Each hardening item must name:

- the observed failure
- the user-visible consequence
- the smallest corrective contract
- the focused regression test

The validation work exposed and corrected these blocking failures:

- initial assessment waited forever when no prior belief revision existed
- the agent store conflated event-ledger sequence with bootstrap-domain sequence
- planned scan output and the existing docs writer input contract required a narrow route adapter
- target README selection was ambiguous until joined through the exact target frame producer
- invalid foreground run settings could mutate product state before returning an error
- the command proof needed enough bounded runtime and full closure assertions

No broader recovery subsystem was added.

## Explicit Post-Validation Deferrals

The following work remains deferred until product use demonstrates its priority:

- exhaustive crash and reopen matrices
- recovery index migration for unreleased actor formats
- invocation journals and unknown external outcome state machines
- claim expiry and legacy claim operator reconciliation
- comprehensive artifact replay hardening
- independent actor-order convergence across cloned roots
- competing-supervisor proofs
- full operator action vocabulary
- detached daemon launch
- control IPC and event authority IPC
- endpoint authentication and reconnect protocols
- broad shutdown barrier hardening

Fresh review identified the first credible hardening candidates without making them acceptance gates for this validation:

- a failure after a durable task claim but before outcome recording can leave that task in `Running`
- a recorded route failure does not yet make the bounded foreground command return a failed flywheel result
- the workspace snapshot artifact is currently a readiness and provenance reference while execution reads the materialized workspace authority
- the activation observation proves one turn and does not replace a recurring sensory loop

Existing reliable primitives may be reused. Already completed work does not need to be removed merely because it exceeds the minimum. New work in these areas is blocked unless it directly unblocks the vertical.

## Parallelization

Use at most three implementation lanes beside the root integrator:

| Lane | Scope | Strength |
| --- | --- | --- |
| world model path | initial observation, first belief, hydration, goal creation | highest available |
| execution path | planning output to existing docs writer task execution | highest available |
| product proof | actor driver, provider observer, final assertions | strong |

Do not create separate reliability, migration, operator, or documentation lanes before V3 passes. Documentation remains a root responsibility at each slice boundary.

## Quality Bar

Every slice keeps normal engineering discipline:

- domain boundaries remain intact
- no `mod.rs`
- public contracts receive appropriate Rustdoc
- focused tests cover the changed behavior
- formatter, diff hygiene, check, and strict Clippy pass
- the end-to-end proof is rerun after every integrated slice

Full workspace tests run at each slice closeout. They are not a substitute for the product proof.

## Documentation Rule

This document records the completed first-turn validation after accepted W3A work. The earlier production closure program and ledger remain historical evidence for accepted work and a deferred reliability backlog. Their unfinished Wave 3 through Wave 6 gates do not block this result.

The plan index and delivery ledger point here as the controlling validation record.
