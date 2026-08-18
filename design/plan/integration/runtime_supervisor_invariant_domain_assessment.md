# Runtime Supervisor Goal, Invariants, And Domain Assessment

Date: 2026-08-17
Status: frozen assessment for disposition
Scope: runtime supervision of the authorized routed docs path through `W06`
Maturity envelope: correct the confirmed live failure with the smallest coherent ownership repair; do not add generalized reliability machinery, new Strategy scope, or dependency-security implementation

## Concern And Scope

The routed docs run proved that one provider-backed task can occupy a supervisor tick long enough for unrelated participant leases to expire. Useful task outcomes can then be hidden by a later shutdown lease error. This assessment defines what the supervisor is for, derives the minimum invariants required by that goal, and checks every current product domain for concrete violations or required relationships.

The direct behavior under assessment is runtime participation while long external work is active, including dispatch, lease maintenance, interruption, recovery handoff, shutdown, and operator reporting.

In scope:

- independent participant ownership and liveness
- bounded supervisor stepping
- durable handoff of queued or external work
- operational interruption and owner reassessment
- shutdown truthfulness
- lifecycle identity and late-result fencing already authorized by `W06`
- owner-derived quiescence

Out of scope:

- changing Strategy search, cost learning, or candidate diversity
- fixing docs capability cost declarations
- verification evidence retention
- dependency-security behavior or its design-gated continuation
- a generalized scheduler, distributed control plane, or new reliability framework
- implementation sequencing and migration planning

The current evidence basis is the retained live-run account in [PDS Authorized Implementation Findings](pds_authorized_implementation_findings.md), current code inspection, the authorized [PDS W06 Portable Lifecycle And External Admission Design](pds_w06_portable_lifecycle_admission_design.md), and the canonical [Runtime Lifecycle And Quiescence](../../cognitive_architecture/runtime_lifecycle_and_quiescence.md).

Applicable repository policy requires domain-first ownership, explicit cross-domain contracts, thin adapters, and compatibility characterization before removal of an old path.

## Frozen Supervisor Goal

The runtime supervisor keeps the declared participant set independently owned, observable, bounded, and safely interruptible, then hands operational discontinuity back to the owning domains without deciding, replaying, invalidating, or obscuring semantic work.

This goal gives the supervisor a narrow authority:

- it may start, lease, observe, fence, restart, stop, and report participant incarnations
- it may require each participant interaction to return within a bounded operational transition
- it may record that operational continuity was lost
- it may not infer task failure, Goal invalidity, result validity, retry safety, or quiescence from a lease or heartbeat alone

## Supervisor Invariants

### SUP-INV01 — Operational authority only

The supervisor owns participant lifecycle, leases, heartbeats, operational health, and lifecycle signals. Domain work eligibility, progress, result validity, and retry meaning remain with their owning domains.

### SUP-INV02 — Independent participant liveness

Work performed for one participant cannot prevent lease maintenance, health observation, or lifecycle progress for another participant.

### SUP-INV03 — Bounded supervisor interaction

One supervisor interaction either performs bounded local work or durably dispatches work and returns. A count budget is insufficient when one counted item can wait for unbounded external completion.

### SUP-INV04 — Execution class is behavioral

A capability declared `Queued` must cross a queue or durable operation boundary. It cannot execute with the completion behavior of `Inline` merely because both share an invocation interface.

### SUP-INV05 — External work is durable before contact

Before externally completed work is released, execution records a stable operation identity, activation generation, participant incarnation, attempt identity, dispatch claim, and effect state. Completion enters through a later bounded poll or delivery and owner admission.

### SUP-INV06 — Lease expiry fences no more than an incarnation

Lease expiry proves loss of current operational ownership. It does not prove semantic failure, authorize a retry, invalidate committed output, or settle an ambiguous external effect.

### SUP-INV07 — Interruption returns to owner judgment

Operational discontinuity becomes a durable interruption receipt consumable by the domain that owns the affected work. That owner reconciles its checkpoint and unresolved operations, then publishes any domain fact that should enter ordinary Meld reassessment.

### SUP-INV08 — Recovery precedes semantic replay

A replacement participant receives a new incarnation and reconstructs from owner state. No semantic effect is repeated merely because a supervisor acquired a new lease or restarted a handle.

### SUP-INV09 — Committed truth is monotonic

Already committed domain products and task outcomes remain authoritative. A later heartbeat, lease, flush, or shutdown failure is reported on its own axis and cannot replace the semantic outcome.

### SUP-INV10 — Shutdown records every terminal axis

Shutdown closes admission, requests owner safe points, records unresolved work, flushes owner stores, records stopped or interrupted lifecycle state, and releases leases. Failure to write a final heartbeat cannot prevent lease release or erase the rest of the shutdown account.

### SUP-INV11 — Lifecycle identities stay distinct

Activation generation, participant incarnation, durable operation, attempt, task claim, and provider dispatch identities cannot substitute for one another. Late completion admission checks the identities relevant to the owning domain.

### SUP-INV12 — Quiescence is owner-derived

Quiescence requires complete participant representation plus owner-declared waits with viable wake paths. One zero-work pass, a healthy heartbeat, or an empty supervisor queue never proves quiescence.

### SUP-INV13 — Observation has no execution authority

CLI output, served status, telemetry, harnesses, health projections, and diagnostic reports can expose lifecycle truth. They cannot resume work, advance foreign cursors, admit results, or decide semantic progress.

## Regenerated Domain Snapshot

The domain universe was regenerated from `src/lib.rs`, the root module contracts, and workspace members in `Cargo.toml`. The independently owned workspace crates are represented under their product owners:

- `meld-events` under events
- `meld-execution` under execution, capability, and task
- `meld-world-model` under agent and world state
- `meld-lang` as Meld Lang

The pass retains every root product module as a row. Compatibility wrappers and adapters remain visible so traversal is not mistaken for write scope.

## Pass One — Complete Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| agent | consume | ordinary admitted evidence can drive Agent work | complete | `meld-world-model` Agent actors | no direct lease knowledge is allowed | decompose as reuse |
| api | none | stateless context API | not needed | `src/api.rs` | supervisor change does not alter context reads or writes | none |
| branches | none | branch identity and migration | not needed | `src/branches.rs` | no participant lifecycle authority | none |
| capability | publish | execution class and cancellation support are declared | partial | `ExecutionContract` | declaration exists but is not enforced by dispatch | decompose |
| cli | observe | runtime results and status are rendered | partial | `src/runtime/tooling.rs`, `src/cli.rs` | presentation must stay non-authoritative | decompose |
| compat | none | compatibility reexports | not needed | `src/compat.rs` | no current supervisor relationship | none |
| concurrency | none | node-level access safety | not needed | `src/concurrency.rs` | participant scheduling is outside this lock domain | none |
| config | none | general product and stewardship configuration | not needed | `src/config.rs` | supervisor timing and desired runtime state are owned by runtime assembly types | none |
| context | none | frame behavior and context generation | not needed | `src/context.rs` | context is traversed only inside task execution | none |
| control | none | legacy control orchestration and projection | not needed | `src/control.rs` | supervisor does not require a control-domain sequence | none |
| dependency security | none | intentionally not assessed | not needed | user authorization boundary | design-gated work is excluded from this concern | none |
| docs | publish | provider-backed docs capabilities declare `Queued` | complete | `src/docs/capability.rs` | docs does not own supervisor scheduling | decompose as reuse |
| error | none | shared error types | not needed | `src/error.rs` | runtime can represent lifecycle axes in its own contracts | none |
| events | publish and consume | durable append, replay, and wake transport exist | complete | `meld-events`, `src/events.rs` | events must not interpret interruption | decompose as reuse |
| execution | own | claims and outcomes are durable, external operation handoff is absent | partial | dispatch actor and runtime ports | execution owns operation and retry truth | decompose |
| harness | observe | reads runtime and domain products | complete | `src/harness.rs` | retention defects are a separate finding | decompose as reuse |
| heads | none | legacy frame head index | not needed | `src/heads.rs` | no supervisor authority | none |
| ignore | none | workspace scan exclusions | not needed | `src/ignore.rs` | no supervisor authority | none |
| init | none | installs initial world and package state | not needed | `src/init.rs` | lifecycle repair does not change initialization meaning | none |
| logging | observe | emits process diagnostics | complete | `src/logging.rs` | diagnostics remain non-authoritative | decompose as reuse |
| merkle traversal | none | bounded workspace capability behavior | not needed | `src/merkle_traversal.rs` | local task behavior is not lifecycle ownership | none |
| metadata | none | frame metadata contracts | not needed | `src/metadata.rs` | no supervisor authority | none |
| prompt context | none | prompt artifacts and lineage | not needed | `src/prompt_context.rs` | no lifecycle contract change | none |
| provider | adapter | executes external model requests for queued capabilities | partial | `src/provider.rs`, provider capability | cancellation and durable attempt context are not carried through the live dispatch path | decompose |
| runtime | own | leases, synchronous stepping, lifecycle primitives, shutdown, status | partial | supervisor entrypoint, assembly, lifecycle, tooling | confirmed source of cross-participant starvation and terminal masking | decompose |
| serve | observe | exposes read-only product status | complete | `src/serve.rs` | served views must not acquire semantic authority | decompose as reuse |
| session | none | session lifecycle and events | not needed | `src/session.rs` | supervisor repair is participant-scoped, not session-scoped | none |
| store | none | legacy node record index | not needed | `src/store.rs` | runtime and execution already own their stores | none |
| task | own | task artifacts, claims, progress, and outcomes are durable | partial | `src/task.rs`, execution task network | claimed invocation is awaited to completion and unresolved external effect is not explicit | decompose |
| telemetry | observe | task and capability telemetry exists | complete | `src/telemetry.rs` | telemetry is evidence only | decompose as reuse |
| theory | none | package identity and routing | not needed | `src/theory.rs` | selected theory identity is owner state, not supervisor input | none |
| tree | none | filesystem Merkle state | not needed | `src/tree.rs` | no supervisor authority | none |
| types | none | shared filesystem types | not needed | `src/types.rs` | no lifecycle behavior | none |
| views | none | compatibility view surface | not needed | `src/views.rs` | no lifecycle behavior | none |
| workflow | none | legacy workflow path | not needed | `src/workflow.rs` | authorized routed docs path uses execution task-network dispatch | none |
| workspace | none | workspace commands, status, and watch | not needed | `src/workspace.rs` | no change to workspace truth or watch ownership | none |
| world state | consume | belief and Strategy can react to admitted domain facts | complete | `meld-world-model` bounded actors | no direct lease interpretation is allowed | decompose as reuse |
| Meld Lang | none | authored language and policy binding | not needed | `meld-lang` | lifecycle repair does not change authored semantics | none |

## Frozen Affected-Domain Set

The affected set is frozen after pass one:

- runtime
- execution
- task
- capability
- provider
- docs
- events
- world state
- agent
- CLI
- harness
- logging
- serve
- telemetry

This is a traversal set, not an implementation file list. Several domains are present only to preserve an existing contract or observation boundary.

## Pass Two — Affected-Domain Decomposition

### Runtime

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| participant ownership | runtime supervisor | leases prevent duplicate runtime owners | lease only fences participant incarnation | extend existing | lease may be mistaken for a task claim | supervisor store and entrypoint |
| actor stepping | runtime supervisor | actors run sequentially in one synchronous tick | each participant remains independently maintainable | extend existing | one long step starves all following actors | `RuntimeSupervisor::tick` |
| bounded contract | runtime contracts | budget limits item count only | external wait cannot occupy the call | extend existing | a single item can exceed every lease | `WorkBudget`, `ActorBoundedStep` |
| lifecycle timing | runtime assembly | lease defaults to fifteen minutes to tolerate provider I/O | timing expresses failure detection, not task duration | extend existing | raising the duration can hide the architecture failure | `RuntimeLifecycleConfig::default` |
| activation lifecycle | root runtime composition | operation, attempt, incarnation, late-result, and quiescence primitives exist | connect primitives to active execution and supervisor lineage | extend existing | a second competing lifecycle truth could emerge | `ActivationLifecycleStore` |
| restart | runtime supervisor | expired lease can remove and recreate a handle | restart only replaces operational incarnation | extend existing | handle restart can trigger unreconciled semantic replay | `evaluate_restart_policies` |
| shutdown | runtime supervisor | final heartbeat precedes lease release and can abort shutdown | record stopped or interrupted state and release ownership independently | extend existing | bookkeeping can mask committed work | `request_shutdown` |
| quiescence projection | runtime tooling | all zero-work actions are labeled quiescent | aggregate owner waits and wake viability | extend existing | diagnostic shorthand becomes false lifecycle truth | `build_tick_account` |
| operator result | runtime tooling | shutdown error replaces successful tick result | report semantic progress, runtime interruption, and shutdown separately | extend existing | a single error string erases a multi-axis outcome | runtime run command |

Confirmed violations: `SUP-INV02`, `SUP-INV03`, `SUP-INV06`, `SUP-INV08`, `SUP-INV09`, `SUP-INV10`, and `SUP-INV12`.

### Execution

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| task selection and claim | execution | claims are durable and epoch-fenced | retain as task ownership, distinct from operation attempts | reuse unchanged | claim identity could be reused as external attempt identity | dispatch actor |
| capability scheduling | execution | one invoker path handles every execution class | enforce the published execution class | extend existing | dispatch adapter may silently collapse `Queued` to `Inline` | capability registry and task executor |
| durable operation | execution | lifecycle primitives exist outside the live dispatch path | persist operation and attempt before external release | extend existing | duplicated or ambiguous provider effects after interruption | runtime lifecycle and dispatch actor |
| completion admission | execution | artifacts and terminal outcomes are recorded after invocation returns | accept later result against generation, incarnation, operation, and attempt | extend existing | stale completion may enter the current task | task network terminal recording |
| interruption reconciliation | execution | a running claim is resumed by invoking it again | reconcile unresolved external attempts before retry | new local behavior | restart can duplicate external cost or effects | `execute_claimed_task` |
| domain publication | execution | task outcomes can already publish through the event boundary | publish owner-admitted interruption or revised outcome only after reconciliation | extend existing | supervisor facts could bypass execution judgment | execution event bridge |

Confirmed violations: `SUP-INV04`, `SUP-INV05`, `SUP-INV07`, `SUP-INV08`, and `SUP-INV11`.

### Task

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| compiled task progress | task | durable task progress and artifacts survive restart | remain canonical semantic progress | reuse unchanged | runtime lifecycle could duplicate task progress | task artifact repository |
| claimed invocation | task network | waits for a complete capability graph before terminal recording | release queued work and return before completion | extend existing | claim remains running with no explicit external state | claimed-task invoker |
| terminal outcome | task network | outcome follows artifact persistence | admit only reconciled later completion | extend existing | shutdown error can obscure but must not replace outcome | terminal recording |

Confirmed violation: claimed task completion currently spans unbounded provider work, violating `SUP-INV03` and `SUP-INV05`.

### Capability

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| contract publication | capability owner | execution class, retry class, completion semantics, and cancellation support are published | preserve as the cross-domain scheduling contract | reuse unchanged | execution may treat metadata as descriptive only | `ExecutionContract` |
| registry binding | capability | contracts reach bound invocation records | make scheduling data available at release point | extend existing | contract can be lost between catalog and executor | capability invocation |
| invocation context | execution capability runtime | deadline, cancellation key, and priority fields exist | carry attempt lineage and enforce supported controls where applicable | extend existing | declared controls can remain inert metadata | `CapabilityExecutionContext` |

Confirmed violation: published execution semantics do not affect live dispatch, violating `SUP-INV04`.

### Provider

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| model request adapter | provider | async request completes inside caller invocation | execute behind a durable operation attempt | extend existing | adapter may become owner of task meaning | provider API and capability |
| cancellation | provider | capability contracts advertise support | bind cancellation to the exact operation attempt | extend existing | process cancellation could target a later attempt | provider capability contract |
| result return | provider | result returns directly to the awaiting task | return an attempt-scoped result for owner admission | extend existing | late result may lack generation and attempt fence | provider invocation path |

Confirmed violation: provider completion is awaited inside the supervisor step and no live attempt lineage reaches the adapter, violating `SUP-INV03`, `SUP-INV05`, and `SUP-INV11`.

### Docs

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| capability contract | docs | provider-backed draft and validation declare `Queued` | preserve truthful declaration | reuse unchanged | execution may ignore domain-owned contract | docs capability contracts |
| capability body | docs | draft and validation legitimately perform multiple provider requests | run as queued operation payload outside supervisor step | reuse unchanged | moving tactical policy into supervisor would violate ownership | docs capability and claim validation |
| result admission | docs | task artifacts represent completed docs products | retain docs validation before canonical publication | reuse unchanged | runtime may confuse provider return with valid docs product | docs artifacts |

No docs-owned supervisor invariant is violated. The live runtime violates the execution behavior that the docs contract already requests.

### Events

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| durable transport | events | append and replay carry admitted facts | carry owner-admitted interruption and recovery facts | reuse unchanged | raw heartbeat noise could pollute semantic evidence | event ledger |
| wake | events | ledger progress wakes the runtime loop between fallback ticks | remain a structural wake reference | reuse unchanged | wake transport could be mistaken for task authority | runtime tooling event wait |
| consumer progress | events | event cursors remain event-owned | never let supervisor own or repair them | reuse unchanged | root runtime could centralize foreign progress | event health contracts |

No event-domain change is required. The missing publication belongs to execution after owner reconciliation, not to the supervisor or event authority.

### World State

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| evidence ingestion | world model | consumes durable admitted events in bounded steps | ingest execution-owned interruption facts normally | reuse unchanged | raw lease state could bypass evidence admission | belief ingestion |
| belief assessment | world model | revises beliefs from admitted evidence | reassess only after owner publication | reuse unchanged | supervisor could be treated as semantic oracle | belief actors |
| Strategy | world model | evaluates current immutable problem inputs | reconsider work only when ordinary inputs change | reuse unchanged | supervisor repair could smuggle in unauthorized Strategy work | Strategy runtime |

No world-state change is required by the confirmed supervisor failure. `PDS-IF03` is an upstream missing handoff, not proof that Strategy or belief machinery is incomplete.

### Agent

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| attention and curation | Agent | bounded actors consume current world-model state | react to owner-admitted interruption through the normal loop | reuse unchanged | direct supervisor commands would become cognition choreography | Agent runtime actors |
| authorization | Agent | authorizes semantic execution under current evidence | preserve authority across operational restart only when owner state says it remains valid | reuse unchanged | a new lease could be mistaken for new authorization | Agent execution handoff |

No Agent change is required. The supervisor must not command reassessment directly.

### CLI

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| command result | runtime tooling with CLI presentation | shutdown failure becomes the terminal command error | present semantic progress, interruption, and shutdown truth without collapsing them | adapter only | adapter could invent semantic success | runtime tooling and CLI rendering |
| status view | CLI | renders supervisor snapshots | continue read-only mapping | reuse unchanged | output wording can overstate quiescence or failure | runtime status rendering |

The confirmed violation is in runtime result composition. CLI remains an adapter.

### Harness

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| runtime observation | harness | reads product and supervisor records | expose the separated lifecycle axes | reuse unchanged | harness success criteria could redefine product truth | interactive harness |
| evidence retention | harness | failure retention is incomplete in the live verification fixture | separate concern under `PDS-IF09` | not needed | expanding this assessment would mix verification with runtime repair | findings register |

No supervisor behavior belongs in the harness.

### Logging

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| diagnostics | logging | emits runtime messages | preserve distinct operational axes | reuse unchanged | logs could be mistaken for durable lifecycle state | tracing configuration |

No change is required.

### Serve

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| status substrate | serve | exposes read-only runtime and product views | expose corrected runtime projections after owner changes | reuse unchanged | served endpoint could become a command path | served sources and listener |

No change is required.

### Telemetry

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| capability observation | telemetry | records task and capability activity | carry operation and attempt references when supplied by owners | reuse unchanged | metrics could be used as correctness state | telemetry contracts |

No behavior change is required for supervisor correctness.

## Ownership And Boundary Synthesis

The supervisor owns operational continuity. Execution owns external operation truth, interruption reconciliation, and retry safety. Task owns semantic progress and terminal outcomes. Capability owners publish execution behavior. Provider remains an external adapter. Events carry only facts admitted by their owner. World state, Agent, and Strategy consume those facts through their existing cognition path.

The smallest missing connective behavior is a two-stage handoff:

```text
supervisor fences participant incarnation
-> runtime records operational interruption receipt
-> execution relates the receipt to durable operations and attempts
-> execution reconciles unresolved effect state
-> execution resumes, retries, rejects, or publishes an admitted fact
-> ordinary event, belief, Agent, and Strategy processing continues
```

The supervisor does not need enough knowledge to choose the final execution action. It needs enough identity to report which participant incarnation lost continuity. Execution needs the durable operation and attempt lineage that is currently absent from the live route.

### Runtime Path Domains

The full runtime path is:

```text
runtime configuration and assembly
-> supervisor
-> execution and task
-> capability and provider
-> docs
-> events
-> world state
-> Agent
```

CLI, harness, logging, serve, and telemetry observe this path.

### Domains With Changed Behavior

Current evidence supports changed behavior only in:

- runtime supervisor and runtime tooling
- runtime activation lifecycle integration
- execution scheduling, durable operation, completion admission, and recovery
- task claimed-invocation release and later completion
- capability runtime binding of declared execution behavior
- provider operation-attempt adapter context
- runtime-owned timing configuration after the unbounded wait is removed

Docs, events, world state, Agent, CLI rendering, harness, logging, serve, and telemetry are traversed but are expected to reuse their existing ownership boundaries.

### Files Likely To Change

This is an evidence-based write-scope forecast, not an implementation plan:

- `src/runtime/supervisor/entrypoint.rs`
- `src/runtime/contracts.rs`
- `src/runtime/assembly.rs`
- `src/runtime/lifecycle.rs`
- `src/runtime/tooling.rs`
- `crates/meld-execution/src/task_network/dispatch_actor.rs`
- `crates/meld-execution/src/capability/runtime.rs`
- `crates/meld-execution/src/capability/invocation.rs`
- `src/runtime/ports.rs`
- the provider adapter seam selected by execution

`src/docs/capability.rs` and `src/docs/claim_validation.rs` are evidence for long-running behavior, not presumed write targets.

## Explicit Non-Integration Decisions

- The supervisor will not inspect Goal, belief, Strategy, docs content, task result validity, or provider response meaning.
- Lease expiry will not directly append a semantic event. Execution first admits the operational fact against its own durable state.
- Events will not gain supervisor or retry policy.
- World state, Agent, and Strategy will not gain direct supervisor callbacks.
- Docs tactical procedure will not move into runtime supervision.
- A longer lease will not be accepted as closure for an unbounded supervisor step.
- A background task alone will not be treated as durable operation ownership.
- Harness retention and generated README review remain separate verification findings.
- Dependency-security work remains outside this assessment and authorization boundary.

## Confirmed Violation Map

| Finding | Violated invariants | Primary owner for disposition |
| --- | --- | --- |
| `PDS-IF01` | `SUP-INV02`, `SUP-INV03` | runtime supervisor and execution scheduling |
| `PDS-IF02` | `SUP-INV09`, `SUP-INV10` | runtime shutdown and result composition |
| `PDS-IF03` | `SUP-INV06`, `SUP-INV07`, `SUP-INV08` | runtime interruption receipt and execution reconciliation |
| `PDS-IF04` | `SUP-INV03`, `SUP-INV04`, `SUP-INV05`, `SUP-INV11` | execution, task, capability runtime, and provider adapter |
| `PDS-IF11` | `SUP-INV12`, `SUP-INV13` | runtime tooling |

## Unresolved Ownership Questions

These questions remain because current code does not prove the smallest seam. They do not block the assessment and should be answered before implementation planning:

- Whether `ActivationLifecycleStore` should become execution-owned persistence or remain root-owned storage behind an execution contract
- Whether one docs capability invocation is the durable operation unit, or whether each provider request needs its own child operation for honest ambiguity and cancellation
- Which existing provider adapter can accept operation and attempt lineage without making provider own execution policy
- Whether interruption receipts belong in the supervisor store with an execution query port, or in a runtime lifecycle store shared through an explicit contract
- How shutdown represents a participant that cannot supply a safe point without converting unresolved work into a false terminal failure
- Which current runtime status schema can carry separate semantic, operational, and shutdown axes without an unnecessary compatibility expansion

## Assessment Stop

The goal, invariants, affected-domain set, confirmed violations, ownership boundaries, and unresolved seams are now frozen for the current concern. This document intentionally stops before implementation planning, acceptance-test design, migration sequencing, or code changes.
