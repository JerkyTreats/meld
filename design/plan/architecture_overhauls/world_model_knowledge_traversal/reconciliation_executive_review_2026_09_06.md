# Executive Review: Reconciliation Recovery And Startup Proof

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-09-06

Status: source review for executive-agent disposition; no independent runtime acceptance

Original reviewed snapshot: `7e03bf6920348741a43afa9ffb97ef9c0e1b7606`

Targeted follow-up snapshot: `128ab164a9fb5e878ab6b35e3f3c4160de0de526`

Active implementation account: [Reconciliation Outcome Audit](reconciliation_outcome_audit.md)

## Purpose And Authority

The user requested this review beside the active outcome audit for the executive agent. It records findings, evidence limits, and proposed regression and completion proofs. It does not supersede the audit, restore the withdrawn delivery program, freeze implementation order, or grant additional authority. Executive disposition belongs in the active implementation account rather than another parallel ledger.

The original review concerned `7e03bf6`. Before this document was committed, the branch advanced to `128ab164`. The two finding paths were rechecked at that exact successor. Other progress at the successor is identified as source evidence or an implementation-account report, not a newly executed full audit. Evidence links below are pinned so later changes cannot silently alter what was reviewed.

## Assessment

The program-level cause of frozen drift has been addressed. The [historical source ledger](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/design/plan/architecture_overhauls/world_model_knowledge_traversal/world_model_reconciliation_source_delivery_program_ledger.md#L1-L3) withdraws its binding slice boundaries and completion claims. The [outcome audit](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/design/plan/architecture_overhauls/world_model_knowledge_traversal/reconciliation_outcome_audit.md) retains compatible sharing, live successor reconciliation, no-action behavior, semantic return, and complete Startup closure as implementation obligations. Shared-owner changes are no longer excluded merely because an earlier slice was accepted.

That change in authority is not evidence that every outcome now works. The current finding is narrower: stale-Plan recovery can still create a first Execution admission, and Startup confirmation can be discarded when standing evidence already satisfies its target. Production nonce emission has progressed, but emission is not the complete flywheel.

### Corrections Present In Source

The [Agent driver](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/agent/actor.rs#L1020-L1458) selects the admitted Plan lineage and calls successor construction when the current cut changes. It preserves completed owner history and records successor admission with predecessor supersession. Historical authorization ordering no longer selects the permanent Plan.

Fresh [Task authorization](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/agent/actor.rs#L1760-L2040) compares against the persisted Plan's `planner_cut_id`. Existing epistemic authorizations without Curation acceptance re-enter currentness and authorization checks before resubmission. These correct the previously identified fresh-authorization comparison and query-only Curation retry path, subject to the separate Execution recovery finding below.

[Strategy search](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/strategy/search.rs#L17-L232) supports no-work satisfied Plans and confirmation-only successors backed by completed Task history. The Agent traverses unfinished products by dependencies rather than universally requiring the first epistemic operation followed by the first Task. These are connected shared primitives, not proof of arbitrary heterogeneous planning or complete product behavior.

### Progress Since The Original Review

The [successor commit](https://github.com/JerkyTreats/meld/commit/128ab164a9fb5e878ab6b35e3f3c4160de0de526) and active audit report a shipped `meld.startup` package, ordinary factory selection of epoch preparation, and a composed supervisor test that emits one nonce through production Task admission and dispatch without calling the emitter directly. The implementation also separates the execution subject from the observation subject and the issuing Agent from the authority-granting principal.

Accordingly, the earlier statement that ordinary factory selection and the Startup package are missing applies to `7e03bf6`, not this successor. The successor still explicitly reports planned confirmation, Goal closure, minimal runtime binding, and full epoch-restart closure as incomplete. Its production-emission test was not independently executed during this review.

## ER-01: Stale Reconstruction Can Create A First Execution Admission

Priority: P1

Evidence status: source-derived failure path rechecked at `128ab164`; no executed reproducer

### Behavior

An authorized Task that never reached Execution can acquire its first admission after the Agent has determined that the Task's Plan premises changed.

In [Agent `select_plan`](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/agent/actor.rs#L1179-L1340), changed cut identity blocks predecessor eligibility. The method then calls `reconcile_execution` for predecessor Task authorizations while attempting to observe already-admitted effects.

[Agent `reconcile_execution`](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/agent/actor.rs#L1939-L1983) checks the activation and authority fence, then calls `execution.advance`. The [production adapter](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/src/runtime/ports.rs#L810-L1018) implements `advance` with `allow_intake = true`. When the admission is absent, it calls `TaskAdmissionApi::admit`. Its separate `observe` method uses `allow_intake = false` and returns no position rather than creating an admission.

```text
Plan P0 is constructed against cut C0.
Agent persists Task authorization A0.
Interruption occurs before Execution admission.
Admitted knowledge changes to C1 within the same generation and epoch.
Agent detects stale P0 and starts reconstruction.
Predecessor observation calls advance on A0.
The adapter creates the missing admission for P0's old Task.
```

This concerns a knowledge change within unchanged authority. The separate closed-epoch historical-return path already uses read-only observation and does not eliminate this same-epoch window. A new authorization check is insufficient when recovery can submit an existing authorization without passing that check.

### Requested Executive Disposition

Separate observing an existing admission or unresolved effect from creating the first admission of an undelivered authorization. Stale-Plan reconciliation should not create new intake as a side effect of observation. Preserve admitted predecessor work and exact historical outcomes without asking Execution to interpret Strategy premises.

The existing read-only observation port provides a candidate boundary. An absent predecessor admission must also permit successor construction or an explicit justified disposition, rather than creating a permanent wait for work that was never submitted.

### Proposed Regression Proof

Persist a valid Plan and Task authorization, interrupt before consumer admission, and close and reopen the stores. Advance a semantically relevant owner revision while retaining the same Agent, generation, epoch, and authority policy. Drive ordinary Agent ticks through the production Execution port.

Assert that the old authorization creates no admission, node, claim, or effect; predecessor eligibility becomes invalid; and successor construction or explicit refusal is durable. A valid successor may create work only under its own current-cut authorization. Separately exercise an already-admitted predecessor and show that its outcome is still observed under original lineage without duplicate execution. Also retain recovery coverage for an undelivered authorization whose Plan remains current.

## ER-02: Positive Standing Evidence Can Bypass Required Confirmation

Priority: P1 for the complete Startup contract

Evidence status: conditional source-derived path rechecked at `128ab164`; complete Startup failure not independently reproduced

### Behavior

A satisfied target can cause an unfinished planned-confirmation obligation to disappear during successor construction.

[Strategy `confirmation_successor`](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/strategy/search.rs#L121-L232) returns no confirmation candidate when the Goal target already evaluates as satisfied. `search_successor` falls back to ordinary `search`, whose satisfied branch constructs a Plan with no products or dependencies. The [Agent selection path](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/agent/actor.rs#L1179-L1340) also supplies no Curation operations when the target is satisfied.

[Agent `accept_satisfied_plan`](https://github.com/JerkyTreats/meld/blob/128ab164a9fb5e878ab6b35e3f3c4160de0de526/crates/meld-world-model/src/agent/actor.rs#L1342-L1408) checks live authority, the exact current cut, and target truth before recording satisfaction. That path does not independently require acceptance of Startup's outstanding planned-confirmation milestone.

If standing realization settles the target before the planned operation is accepted, these paths can replace the required confirmation with an empty satisfied Plan. The active outcome audit explicitly identifies this remaining obligation; this is not a claim that the implementation already advertises full Startup completion.

### Requested Executive Disposition

Represent the required confirmation and satisfaction evidence through the owning Goal, Agent, and product contracts. Preserve valid no-work closure for a genuinely maintained condition while ensuring that reconstruction cannot discard an unfulfilled product-specific completion obligation. Do not globally require execution, add a Startup coordinator, or interpret generic Graph visibility as confirmation.

### Proposed Regression Proof

Run the installed Startup product through nonce emission, then let standing Curation and configured Belief settle positive realization while delaying planned-confirmation acceptance. Trigger successor construction and assert that Goal satisfaction remains absent and the required confirmation remains represented.

Resume the exact confirmation path and assert separate durable authorization, Curation intake and result, required Graph and Belief positions, Agent milestone acceptance, and Goal satisfaction. Reopen around those boundaries and confirm idempotency. Keep a separate already-maintained-condition test that creates no unnecessary Task.

## Startup Proof Boundary

The implementation account now reports production emission from ordinary package preparation, reopen, lifecycle admission, and supervisor operation. It separately reports executable crash recovery and epistemic return tests. Those results must not be added together as though one connected end-to-end run established every transition.

A completion run should establish this connected trace through ordinary installed owners:

```text
Installed product and exact prepared revisions
-> current generation and open admission epoch
-> Agent-owned epoch specification and exact observation
-> complete source coverage and bounded non-realization
-> admitted mismatch and Agent-owned Goal
-> exact Planner cut and Strategy Plan
-> separate Task authorization and Execution admission
-> nonce Capability, durable Event, and Graph visibility
-> separately authorized planned confirmation
-> configured realization evidence and Agent milestone acceptance
-> separate Goal satisfaction
-> successor epoch requiring a distinct nonce
```

The harness may inject faults and inspect owner records. It should not author the Goal, call the nonce emitter, insert confirmation results, or directly write satisfaction. Running ordinary bounded supervisor ticks is not itself manual semantic sequencing.

The evidence should expose exact product, Agent, Goal, Plan, cut, authorization, admission, Event, Curation result, Belief revision, generation, and epoch identities. Same-epoch replay should retain one semantic nonce effect. A successor epoch must not accept predecessor evidence as its own satisfaction. Delayed Graph or Belief, retained uncertain effects, and interruption before consumer acceptance should leave explicit owner positions rather than a combined success status.

A nonce round trip proves this path for its exact product and epoch. It does not prove compatible sharing, arbitrary Plan search, Docs correctness, Security applicability, global health, or activation-wide quiescence.

## Remaining Outcome Coverage

The active audit retains O27 compatible sharing and independent discharge, broader product-dependency and owner-milestone progression, Docs semantic observation and no-action closure, the native Security trace, and lifecycle recovery obligations. These remain unfinished outcomes rather than accepted exclusions. Retain that distinction when recording completion.

The successor audit also records provisional workspace-shaped Startup initialization and unnecessary workspace or provider dependencies. Removing those couplings is still part of the minimal runtime product, not a reason to create a second activation route.

## Verification Limits And Handoff

This review performed repository reads and targeted source tracing. It changed no runtime source and executed no Rust tests. The review environment exposed neither `cargo` nor `rustc`; direct retrieval from GitHub failed DNS resolution. This is a verification limitation, not a build failure of Meld.

The author reports 1,857 passing workspace tests at `7e03bf6` and 1,859 at `128ab164`, across 52 suites with zero failures and three ignored, plus passing lint and formatting. These are attributed checkpoint reports, not independently reproduced results. The earlier chat's zero-workflow observation came from a helper restricted to pull-request-triggered runs; it must not be interpreted as proof that no CI of any kind ran.

The executive agent should disposition ER-01 and ER-02 against its current working tree in the active outcome audit, recording a fixing commit and executable regression evidence or the exact counterevidence. If later work already changes a cited path, preserve the reviewed snapshot and explain the new disposition rather than treating this document as an immutable implementation requirement.

Assessment at the follow-up snapshot: delivery restrictions are superseded; connected reconciliation primitives and production nonce emission have advanced; stale first admission and confirmation bypass require disposition; complete Startup flywheel acceptance remains unproved by this review.
