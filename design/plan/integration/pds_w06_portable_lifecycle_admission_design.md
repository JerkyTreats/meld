# PDS W06 Portable Lifecycle And External Admission Design

Date: 2026-08-16
Status: accepted for authorized implementation
Mapped workstream: `W06`
Scope: assignment-local activation lifecycle, durable external operations, passive delivery, recovery, quiescence, and owner result admission exercised by local and fake serialized dependency-security adapters

## Objective

Implement the canonical runtime lifecycle beneath one stable supervised activation service without moving domain progress, waiting, safe-point, or result meaning into root.

The dependency-security adapter remains a contract proof. This packet does not authorize a production scanner or service integration.

## Canonical Relationship

```text
root supervisor
→ stable activation lifecycle service
→ assignment activation generation
→ owner participant incarnations
→ durable operations or passive subscriptions
→ owner-admitted canonical products
```

The router leaves the live path after exact package resolution.

## Ownership

| Concern | Owner |
| --- | --- |
| lifecycle intent, prepared closure, current head, aggregate receipts | runtime composition |
| stable process role and participant leases | supervisor |
| work eligibility, waits, readiness, safe points, domain checkpoints | each domain owner |
| durable operations, attempts, claims, ambiguous effects | execution |
| passive subscriptions, source cursors, and raw delivery authentication | source owner and adapter |
| canonical result admission | destination domain |
| append, replay, consumer cursors, and event wake | event authority |
| read-only lifecycle projection | runtime presentation and harness |

## Stable Service Boundary

Root supervisor gains one stable role:

```text
runtime.pds_activation_lifecycle
```

The role owns a bounded command loop over durable lifecycle intents. It does not dynamically register assignment roles with the root supervisor.

Each bounded step may advance one durable lifecycle transition, dispatch one bounded operation, poll one ready completion, admit one delivery, or record one owner wait. It may not block on external completion.

## Lifecycle Intent

```rust
struct ActivationLifecycleIntentV1 {
    intent_id: String,
    request_key: String,
    assignment_id: String,
    desired_activation_id: String,
    expected_prior_generation: Option<String>,
    action: LifecycleAction,
}

enum LifecycleAction {
    Activate,
    Replace,
    Deactivate,
    Recover,
}
```

Intent identity hashes request key, assignment, desired activation, expected prior, and action. Exact retry is unchanged. A conflicting request key fails.

The intent is persisted before owner work begins.

## Identity Model

```text
package receipt
→ assignment
→ activation
→ activation generation
→ participant
→ participant incarnation
→ durable operation and attempt

participant incarnation
→ passive subscription and delivery
```

Exact contracts:

```rust
struct ParticipantIncarnationV1 {
    incarnation_id: String,
    generation_id: String,
    participant_id: String,
    adapter_implementation_ref: String,
    lease_ref: String,
    status: ParticipantIncarnationStatus,
}

struct DurableOperationV1 {
    operation_key: String,
    assignment_id: String,
    generation_id: String,
    capability_contract_ref: CapabilityContractRevisionRef,
    semantic_request_hash: String,
    status: DurableOperationStatus,
}

struct OperationAttemptV1 {
    attempt_id: String,
    operation_key: String,
    incarnation_id: String,
    dispatch_claim_ref: String,
    adapter_dispatch_ref: Option<String>,
    status: OperationAttemptStatus,
}

struct PassiveSubscriptionV1 {
    subscription_id: String,
    assignment_id: String,
    generation_id: String,
    participant_id: String,
    incarnation_id: String,
    source_id: String,
    admitted_cursor: String,
    status: PassiveSubscriptionStatus,
}

struct PassiveDeliveryV1 {
    delivery_id: String,
    subscription_id: String,
    incarnation_id: String,
    source_cursor: String,
    source_revision: String,
    payload_hash: String,
}
```

Operation key is stable across retry and equivalent participant replacement. Attempt id changes for every dispatch. Delivery never fabricates an operation or attempt.

## Owner Lifecycle Ports

One universal callback does not own domain meaning. Runtime coordinates separate narrow ports:

```rust
trait OwnerPreparationPort {
    fn prepare(&self, request: OwnerPrepareRequest) -> Result<OwnerPrepareReceipt, OwnerLifecycleDiagnostic>;
}

trait OwnerReadinessPort {
    fn readiness(&self, request: OwnerReadinessRequest) -> Result<OwnerReadinessReceipt, OwnerLifecycleDiagnostic>;
}

trait OwnerWakePort {
    fn resolve_wait(&self, wait_ref: &OwnerWaitRef) -> Result<ResolvedOwnerWait, OwnerLifecycleDiagnostic>;
}

trait OwnerSafePointPort {
    fn safe_point(&self, request: OwnerSafePointRequest) -> Result<OwnerSafePointReceipt, OwnerLifecycleDiagnostic>;
}

trait OwnerStopPort {
    fn stop(&self, request: OwnerStopRequest) -> Result<OwnerStopReceipt, OwnerLifecycleDiagnostic>;
}
```

Each port is implemented by an owner adapter and receives only owner state. Runtime verifies exact refs and participant coverage without interpreting the receipt body.

## Activation Generation States

| State | Admission | Required truth |
| --- | --- | --- |
| `Preparing` | closed | durable intent and owner preparation underway |
| `Prepared` | closed | complete inert closure committed |
| `Starting` | closed | generation and participant incarnations exist |
| `Ready` | closed | every required participant produced readiness |
| `Current` | open | assignment head points to generation |
| `ActiveIdle` | open | latest bounded observation found no eligible work |
| `Quiescent` | open | all participants represented and every wait has viable wake |
| `Stalled` | unchanged | required participant, eligibility, operation ownership, or wake is broken |
| `Quiescing` | closed | drain and safe-point collection underway |
| `FencedQuiescent` | closed | all accepted obligations settled or durably unresolved |
| `Stopping` | closed | participants stop in reverse dependency strata |
| `Stopped` | closed | leases released and generation retired cleanly |
| `Interrupted` | closed | runtime continuity lost without complete retirement |
| `FailedBeforeCurrent` | closed | generation never gained authority |

Active idle and quiescent are projections over current owner receipts. They are not mutable domain authority states.

## Activation And Replacement Protocol

Activation follows:

```text
persist intent
→ resolve exact inputs
→ collect owner requirements
→ resolve bindings and authority inputs
→ prepare owners and capability closure
→ commit prepared closure
→ create generation and participant incarnations
→ acquire leases and start by dependency strata
→ collect readiness
→ compare and swap current head
→ open admission
```

Replacement follows:

```text
prepare new generation with admission closed
→ close old generation admission
→ drain old generation
→ commit old fenced-quiescence receipt
→ start and prove new readiness
→ compare and swap old head to new generation
→ stop and retire old participants
→ open new admission
```

No two mutating generations are current concurrently. Preparation and process warmup may overlap because they carry no new authority.

A credential, endpoint, selected implementation, authority, binding, isolation, or activation-policy change creates a new generation. Equivalent reconstruction after process loss creates a new participant incarnation under the same generation after recovery readiness.

## Durable External Operation

Bounded dispatch:

1. Derive or resolve one stable semantic operation key.
2. Persist the operation before external contact.
3. Persist an attempt against the current generation and incarnation.
4. Acquire one execution dispatch claim.
5. Hand the exact request to the adapter.
6. Persist the adapter dispatch ref or an ambiguous dispatch state.
7. Return from the bounded step.

Completion:

1. Poll or receive the result in a later bounded step.
2. Resolve operation, attempt, generation, incarnation, adapter, request, and source lineage.
3. Reconcile ambiguous dispatch before retry.
4. Ask the destination owner to admit the result.
5. Append the canonical domain product only after owner admission.
6. Complete the operation and execution claim with admitted product refs.

A timeout after dispatch is ambiguous. It never becomes a retryable failure until reconciliation proves the prior attempt did not produce an effect or accepted result.

## Passive Source Ingress

Passive delivery is admitted through a distinct path:

```text
authenticated adapter envelope
→ exact active subscription
→ generation and incarnation fence
→ monotonic or owner-valid source cursor
→ destination owner validation
→ canonical product
→ canonical event append
```

No execution claim or attempt is created for a passive delivery. Owner admission still verifies assignment, package, policy, subject, scope, source revision, completeness, and dedupe identity.

The `W06` proof uses a fake serialized source advance delivery. It does not poll a real advisory service.

## Waiting And Wake

Every required idle participant produces:

```rust
struct OwnerWaitReceiptV1 {
    wait_ref: OwnerWaitRef,
    generation_id: String,
    incarnation_id: String,
    owner_checkpoint_ref: String,
    reason_code: String,
    wake_refs: Vec<StructuralWakeRef>,
}
```

Structural wake kinds are event position, owner revision, durable deadline, passive subscription, and operator action.

Runtime resolves that each wake ref has a current owner or admitted source path. It does not interpret `reason_code`. Missing required participant, missing wait receipt, or unresolvable wake produces `Stalled`, not `Quiescent`.

## Fenced Quiescence

Fencing first closes new admission for the exact generation.

The aggregate receipt is:

```rust
struct FencedQuiescenceReceiptV1 {
    receipt_id: String,
    assignment_id: String,
    generation_id: String,
    admission_fence_ref: String,
    owner_safe_point_refs: Vec<String>,
    unresolved_operation_summary_ref: String,
    passive_subscription_fence_refs: Vec<String>,
    participant_drain_refs: Vec<String>,
}
```

Receipt identity hashes every field except `receipt_id` with sorted ref collections.

An ambiguous external effect may remain as a durable unresolved operation. Fenced quiescence requires no unrecorded in-memory obligation, not elimination of all uncertainty.

After the receipt commits, participants stop in reverse dependency strata, owner stores flush through their own boundaries, leases release, and a retirement receipt records clean stop.

## Interrupted Recovery

On reopen the activation service reconstructs:

- lifecycle intent and incomplete phase
- exact prepared closure and participant plan
- current generation head
- owner checkpoints, waits, safe points, outboxes, subscriptions, and cursors
- durable operations, attempts, claims, and unresolved effects
- prior participant incarnations and lease state

Admission remains closed.

Recovery creates new incarnations for equivalent participants, reconciles every old-incarnation operation and delivery path, and collects recovery-readiness receipts. Only then may it resume the interrupted phase or reopen current admission.

Changed activation inputs require replacement with a new generation rather than recovery under the old one.

## Late Result Policy

Every destination owner declares one policy for each product kind:

```rust
enum LateResultDisposition {
    Reject,
    HistoricalOnly,
}
```

`W06` does not permit a retired result to advance current posture. `HistoricalOnly` may persist the raw admitted historical product and lineage but may not publish it as current belief evidence or complete a current operation.

The bounded dependency-security proof uses `Reject` for inventory and assessment operation results and `HistoricalOnly` for authenticated source-advance notifications whose source revision remains useful as historical provenance.

No result is relabeled with a current generation or incarnation.

## Liveness Projection

The read-only projection contains:

```rust
struct AssignmentLifecycleProjectionV1 {
    assignment_id: String,
    generation_id: Option<String>,
    state: ProjectedLifecycleState,
    participant_statuses: Vec<ProjectedParticipantStatus>,
    unresolved_operation_count: u64,
    waiting: Vec<OwnerWaitRef>,
    broken_wake_refs: Vec<StructuralWakeRef>,
    last_transition_ref: String,
}
```

Telemetry and harness render this projection. They do not write lifecycle state or establish readiness, quiescence, safe point, or result validity.

## Persistence

Runtime composition and execution use versioned external product trees:

```text
pds_activation_intents_v1
pds_activation_generations_v1
pds_participant_incarnations_v1
pds_assignment_heads_v1
pds_admission_fences_v1
pds_quiescence_receipts_v1
pds_retirement_receipts_v1
pds_passive_subscriptions_v1
pds_passive_deliveries_v1
```

Durable operations, attempts, dispatch claims, tasks, and unresolved effects remain in execution-owned stores. Owner checkpoints and canonical products remain in owner stores.

Every store path resolves under the external product root.

## Failure Matrix

| Failure point | Durable residue | Authority | Required transition |
| --- | --- | --- | --- |
| before exact resolution | intent | none | correct or terminate intent |
| owner preparation | idempotent owner records | none | retry missing owner or abandon inert preparation |
| after prepared closure | exact inert closure | none | resume start or retire preparation |
| participant start | incarnation and possible lease subset | none | stop or abandon leases and retry or replace |
| readiness | readiness subset | none | remain closed, stop, or retry according to owner diagnostic |
| before current publication | ready non-current generation | none | compare expected prior, publish, or retire |
| after current publication | current head and accepted work | current until fence | close admission and recover or replace |
| external dispatch timeout | ambiguous attempt | existing accepted operation only | reconcile before retry |
| passive cursor conflict | rejected delivery | unchanged | owner resolves cursor or source resync |
| graceful drain timeout | quiescing generation and unresolved work | no new authority | mark interrupted with durable uncertainty |
| process loss | stale leases and old incarnations | closed during recovery | create new incarnations after reconciliation |
| late callback | retired lineage | none | reject or historical-only admission |
| final store flush uncertainty | exact safe-point refs with uncertain boundary | none | reopen and verify owner watermarks |

## Local And Fake Serialized Proof

The same `D05` truth contract runs through:

- a local in-process adapter with immediate later-step completion
- a fake serialized adapter using canonical JSON requests, dispatch receipts, later completion, and passive source delivery

Both use identical package receipt, assignment, policy, capability contract, operation, owner admission, and canonical product shapes. Only activation implementation ref, participant incarnation, attempt, transport receipt, and delivery identity differ.

The fake adapter supports controlled timeout, duplicate completion, out-of-order completion, crash, restart, cursor conflict, and late callback fixtures.

## Verification Design

Focused tests:

```text
runtime::activation::tests::intent_retry_is_idempotent
runtime::activation::tests::expected_prior_fences_concurrent_replacement
runtime::activation::tests::readiness_precedes_current_publication
runtime::activation::tests::replacement_fences_old_before_new_publication
runtime::activation::tests::participant_restart_changes_incarnation_not_generation
runtime::activation::tests::binding_change_requires_new_generation
runtime::lifecycle::tests::zero_work_without_complete_waits_is_stalled
runtime::lifecycle::tests::complete_wait_and_wake_closure_is_quiescent
runtime::lifecycle::tests::fenced_quiescence_accepts_durable_unresolved_effect
execution::operation::tests::operation_persists_before_dispatch
execution::operation::tests::ambiguous_attempt_reconciles_before_retry
runtime::delivery::tests::passive_delivery_creates_no_execution_attempt
runtime::delivery::tests::retired_generation_delivery_never_advances_current
runtime::recovery::tests::admission_stays_closed_until_reconciliation
runtime::recovery::tests::reopen_resumes_incomplete_lifecycle_phase
tests::pds_w06_portable_lifecycle::local_and_serialized_products_are_equivalent
tests::pds_w06_portable_lifecycle::late_callback_is_rejected_or_historical_only
```

Static checks:

```sh
rg -n "sleep|block_on|wait_for_completion" src/runtime/activation.rs src/dependency_security/adapter.rs
rg -n "docs_freshness|dependency_security" src/runtime/activation.rs src/runtime/lifecycle.rs
rg -n "current.*=.*result|result.*generation.*current" src/runtime src/dependency_security
rg --files src/runtime src/dependency_security | rg '/mod\.rs$'
```

No blocking external wait, expression branch, silent result relabeling, or new `mod.rs` file is allowed.

## Acceptance Criteria

- one stable supervised service coordinates every assignment generation
- lifecycle intent is durable and idempotent
- current publication uses expected-prior fencing
- activation generation, participant incarnation, operation, attempt, subscription, and delivery ids are distinct
- operation persists before dispatch
- passive delivery creates no fabricated execution attempt
- readiness and recovery keep admission closed until owner reconciliation
- zero work without full participant and wake closure is not quiescent
- ambiguous effects reconcile before retry
- replacement closes old admission before new current publication
- late results retain retired lineage and never advance current state
- local and fake serialized adapters yield the same canonical owner products
- all runtime state remains outside the target workspace

## Rejection Criteria

Reject implementation if it:

- dynamically turns each assignment into a root supervisor role
- holds a bounded actor step open for external completion
- uses one id for generation, incarnation, operation, and attempt
- equates process health or zero work with quiescence
- reopens admission before recovery readiness
- retries an ambiguous effect without reconciliation
- fabricates an execution attempt for passive source delivery
- allows telemetry or harness to establish lifecycle truth
- relabels an old result as current
- selects or integrates a production security service

## Decision Ledger

- One stable supervised activation service owns structural lifecycle coordination.
- Assignment participants remain beneath the role-based root supervisor.
- Lifecycle uses separate owner preparation, readiness, wake, safe-point, and stop ports.
- External work always has a durable operation before dispatch.
- Passive delivery has subscription and delivery lineage, not execution-attempt lineage.
- Replacement fences old admission before new publication.
- Retired results are rejected or historical only in the authorized slice.
- The proof uses local and fake serialized adapters only.

## Completion Condition

`D06` is delivered. `W06` may implement portable lifecycle and external admission without further cross-domain design decisions.
