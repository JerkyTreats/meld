# PDS W04 Assignment And Startup Activation Design

Date: 2026-08-16
Status: accepted for authorized implementation
Mapped workstream: `W04`
Scope: separate standing package assignment from physical activation and publish isolated assignment-local runtime generations at normal process startup

## Objective

Define the exact startup-only activation protocol for several assignments over installed package receipts.

This packet does not implement hot replacement, graceful retirement, interrupted recovery, passive external delivery, or participant replacement. Those belong to `D06`.

## Ownership

| Concern | Owner |
| --- | --- |
| desired assignment and non-secret activation config | config |
| exact package receipt and owner refs | theory |
| aggregate preparation and startup generation publication | runtime composition |
| owner-local activation state and readiness | each activation contributor |
| exact contract and implementation inventory | capability contributors |
| assignment-local catalogs and executor registry | capability plus runtime composition |
| Agent activation and current authority judgment | world-model Agent |
| execution policy, Goal, task, operation, and dispatch authority | execution |
| stable product role supervision | supervisor |

The router is not a live activation participant after exact package resolution.

## Assignment Contract

```rust
struct StewardshipAssignmentV1 {
    assignment_id: String,
    package_receipt_id: String,
    principal_id: String,
    agent_id: String,
    subject: DomainObjectRef,
    perspective_id: String,
    branch_id: String,
    requested_authority_ref: String,
    principal_grant_ref: String,
}
```

Assignment identity hashes every field except `assignment_id` in canonical field order.

The assignment is standing semantic intent. It contains no workspace path, provider, credential, endpoint, executable, placement, resource limit, runtime handle, or current-generation state.

Several assignments may cite one package receipt or one subject. Declaration order grants no priority.

## Physical Activation Contract

```rust
struct StewardshipActivationV1 {
    activation_id: String,
    assignment_id: String,
    bindings: BTreeMap<String, PhysicalBindingRef>,
    selected_implementations: BTreeMap<CapabilityContractRevisionRef, String>,
    placement: AdapterPlacement,
    isolation_requirements: RuntimeIsolationRequirements,
    operational_limits: OperationalLimits,
}

enum PhysicalBindingRef {
    WorkspaceRef(String),
    ProviderRef(String),
    CredentialRef(String),
    EndpointRef(String),
    ExecutableRef(String),
    ConfigRef(String),
}
```

Activation identity hashes non-secret binding refs, selected implementation refs, placement, isolation requirements, and operational limits with the assignment id. Secret values never enter identity, diagnostics, receipts, or events.

Credential content changes under the same ref are detected through an owner binding revision and create a new generation under `D06`. Startup reads one exact binding revision per ref.

## Owner Requirement Contract

```rust
trait TheoryActivationContributor: Send + Sync {
    fn owner_domain(&self) -> &str;

    fn requirements(
        &self,
        package: &OwnerPackageView,
        assignment: &StewardshipAssignmentV1,
    ) -> Result<OwnerActivationRequirements, ActivationDiagnostic>;

    fn prepare(
        &self,
        package: &OwnerPackageView,
        assignment: &StewardshipAssignmentV1,
        bindings: &OwnerActivationBindings,
    ) -> Result<PreparedDomainActivation, ActivationDiagnostic>;

    fn readiness(
        &self,
        prepared: &PreparedDomainActivation,
        generation: &ActivationGenerationRef,
    ) -> Result<OwnerReadinessReceipt, ActivationDiagnostic>;
}
```

The owner package view exposes owned routes and explicit required refs only. Owner bindings contain only ids requested by that contributor.

Requirements declare:

- required and optional binding ids
- selected exact capability refs owned by the contributor
- isolation requirements
- participant descriptions
- lifecycle dependencies
- readiness contract refs
- admission policy refs

Root verifies structural closure but does not interpret owner meaning.

## Prepared Activation Closure

```rust
struct PreparedActivationClosureV1 {
    prepared_id: String,
    assignment: StewardshipAssignmentV1,
    activation: StewardshipActivationV1,
    package_receipt_id: String,
    package_content_hash: String,
    owner_receipts: Vec<PreparedDomainActivationRef>,
    capability_closure: PreparedCapabilityClosureRef,
    participant_plan: ActivationParticipantPlanV1,
    binding_revision_refs: Vec<OwnerBindingRevisionRef>,
    effective_authority_inputs: EffectiveAuthorityInputRefs,
}
```

Prepared identity hashes every exact ref except `prepared_id`.

The closure is committed last after every owner receipt, exact selected invoker, binding revision, isolation realization, lifecycle dependency, and authority input resolves. It is inert. It exposes no invoker through a live assignment registry and opens no admission.

## Startup Participant Plan

```rust
struct ActivationParticipantPlanV1 {
    plan_id: String,
    participants: Vec<ActivationParticipantSpec>,
}

struct ActivationParticipantSpec {
    participant_id: String,
    owner_domain: String,
    kind: ParticipantKind,
    required: bool,
    depends_on: BTreeSet<String>,
    readiness_contract_ref: String,
    wake_contract_ref: String,
    safe_point_contract_ref: String,
    stop_contract_ref: String,
}

enum ParticipantKind {
    BoundedActor,
    DurableOperationAdapter,
    PassiveSource,
}
```

`W04` starts only participants required by docs startup activation. Passive source and external operation behavior is specified now as open plan vocabulary but not implemented until `W06`.

Plan identity hashes sorted participant specs. Dependency cycles and missing dependencies fail preparation.

## Startup Generation

```rust
struct ActivationGenerationV1 {
    generation_id: String,
    generation_number: u64,
    assignment_id: String,
    activation_id: String,
    prepared_id: String,
    expected_prior_generation: Option<String>,
    status: StartupGenerationStatus,
    readiness_receipts: Vec<OwnerReadinessReceiptRef>,
}

enum StartupGenerationStatus {
    Starting,
    Ready,
    Current,
    FailedBeforeCurrent,
}
```

Generation id hashes assignment id, activation id, prepared id, and generation number. Startup uses generation number one and expects no prior generation.

Current publication is one compare-and-swap over the assignment head from absent to generation id. Constructor success, handle creation, or partial readiness cannot advance the head.

## Startup Protocol

1. Resolve all desired assignments in deterministic config-key order.
2. Derive and validate exact assignment and activation ids.
3. Resolve exact package receipt and owner package views.
4. Collect owner requirements read-only.
5. Resolve only requested bindings and their exact revisions.
6. Resolve current effective-authority inputs without treating availability as a grant.
7. Prepare owner-local activation records idempotently.
8. Prepare the exact assignment-local capability closure.
9. Commit the prepared activation closure last.
10. Create generation one in `Starting` state.
11. Create required startup handles in explicit dependency strata.
12. Obtain every required owner readiness receipt.
13. Mark the generation `Ready`.
14. Compare and swap the assignment head from absent to this generation.
15. Publish the immutable capability catalog and executor registry with the current head.
16. Open new-work admission for the assignment.

Independent assignments may execute these steps concurrently. Their durable ids, stores, catalogs, bindings, and diagnostics remain separate.

## Assignment-Local Runtime View

```rust
struct CurrentAssignmentRuntime {
    assignment_id: String,
    generation_id: String,
    package_receipt_id: String,
    agent_runtime_ref: String,
    belief_scope_ref: String,
    capability_catalog: Arc<CapabilityCatalog>,
    executor_registry: Arc<CapabilityExecutorRegistry>,
    execution_scope_ref: String,
    admission_ref: String,
}
```

No mutable global executor registry contains invokers from several assignments. Shared immutable contract revisions are allowed. Invoker instances, grants, bindings, beliefs, Goals, task networks, operations, and admission remain assignment scoped.

A selected identical contract may bind different implementation refs in two activations without identity collision.

## Persistence

Runtime composition owns these external product database trees:

```text
pds_assignments_v1
pds_activations_v1
pds_prepared_activations_v1
pds_activation_generations_v1
pds_assignment_heads_v1
```

Assignment, activation, and prepared records are immutable by content identity. Generation records use append-only transitions with expected prior state. Assignment heads use compare and swap.

Owner-local preparation, readiness, belief, Agent, and execution state remains in owner stores.

## Startup Failure Matrix

| Failure point | Durable residue | Live authority | Startup response |
| --- | --- | --- | --- |
| assignment validation | none | none | reject only that desired assignment |
| exact package resolution | desired config only | none | report exact receipt failure |
| requirement collection | diagnostics only | none | exact retry safe |
| binding resolution | diagnostics only | none | fail affected activation only |
| owner preparation | inert owner records | none | retry same preparation or mark abandoned |
| capability preparation | inert owner and invoker drafts | none | discard process-local invokers, retain exact diagnostics |
| prepared closure commit | owner records may exist | none | retry commit after exact verification |
| handle creation | generation and partial handles | none | stop created handles and mark failed before current |
| readiness | generation and readiness subset | none | stop handles and mark failed before current |
| current-head compare and swap | ready non-current generation | none | report conflict and keep admission closed |

Failure of one activation never stops or removes a successfully current unrelated assignment.

## Provider Optionality

The docs contributor requests a provider binding only when the selected implementation set includes a provider-backed contract.

- deterministic docs activation can become current without any provider config
- model-backed docs activation fails locally when its required provider binding is missing
- provider failure does not alter another activation's catalog or head
- provider id and credential revision remain activation data, not package data

## Verification Design

Focused tests:

```text
config::stewardship::assignment::tests::identity_excludes_physical_bindings
config::stewardship::activation::tests::identity_excludes_secret_values
theory::activation::tests::prepared_closure_is_inert
theory::activation::tests::missing_owner_receipt_prevents_closure
runtime::activation::tests::startup_publishes_only_after_required_readiness
runtime::activation::tests::current_head_uses_expected_absent_fence
runtime::activation::tests::declaration_order_does_not_change_assignment_results
runtime::activation::tests::overlapping_assignments_have_distinct_runtime_views
runtime::activation::tests::one_failed_activation_leaves_another_current
capability::tests::same_contract_binds_distinct_assignment_local_invokers
capability::tests::prepared_invokers_are_not_globally_visible
runtime::activation::tests::deterministic_docs_activation_needs_no_provider
```

Reopen test:

```text
tests::pds_w04_startup_activation::assignments_and_current_heads_survive_reopen
```

Static checks:

```sh
rg -n "BTreeMap.*CapabilityInvoker|global.*executor" src/runtime src/capability.rs src/capability
rg -n "provider_id" src/theory.rs src/theory/package.rs src/theory/receipt.rs
rg -n "docs_freshness|dependency_security" src/runtime/activation.rs src/theory/activation.rs
```

The expected result is no global mutable invoker map, no provider in package contracts, and no expression branches.

## Acceptance Criteria

- package, assignment, activation, prepared closure, and generation ids are distinct
- package identity excludes every physical binding
- prepared closure is complete and inert
- admission opens only after all required readiness receipts resolve
- several assignments resolve independently of declaration order
- overlapping assignments retain separate grants, bindings, invokers, beliefs, Goals, tasks, and operations
- the same exact contract can bind different implementations per activation
- one failed activation leaves another assignment healthy
- deterministic activation requires no provider
- assignment, activation, prepared closure, generation, and head survive reopen

## Rejection Criteria

Reject implementation if it:

- activates a package merely because it installed
- includes secrets or physical paths in package identity
- publishes invokers before readiness closure
- stores all assignment invokers in one mutable global registry
- treats declaration order as priority
- allows capability availability to grant authority
- lets one activation failure abort unrelated current assignments
- implements hot replacement or interrupted recovery in `W04`

## Decision Ledger

- Assignment is standing semantic intent and activation is physical realization.
- Runtime composition owns prepared closure, generations, and current heads.
- Generation one uses an expected-absent current-head fence.
- Prepared closure is complete but inert.
- Capability catalogs and executor registries are assignment-local.
- Startup readiness precedes current publication and admission.
- Hot lifecycle behavior remains in `D06`.

## Completion Condition

`D04` is delivered. `W04` may implement assignment and startup activation without further cross-domain design decisions.
