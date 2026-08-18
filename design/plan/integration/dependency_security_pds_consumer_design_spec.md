# Dependency Security PDS Consumer Detailed Design Specification

Date: 2026-08-18
Status: design input with only the bounded `W05` and `W06` subset authorized
Scope: dependency-security domain theory, external security runtime adapter, canonical observations, belief inputs, capabilities, and event-mediated cooperation through the PDS Router

## Authorization Boundary

This document is not a complete authorized implementation contract. Only the bounded dependency-security truth slice and lifecycle contract named in [PDS Authorized Implementation Workstreams](pds_implementation_workstreams.md) may be implemented.

Production source selection, full scanner or monitor integration, remediation, repository mutation, broader cross-steward behavior, and completion of the dependency-security expression require real design work and explicit reauthorization under [PDS Design-Gated Continuations](pds_design_gated_continuations.md).

The semantic boundary is fixed in [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md). The package owns security vocabulary, evidence meaning, norms, and Strategy semantics. Activation supplies exact capabilities. Agent and Strategy produce situated cognition from current state and independently owned policy inputs.

## Objective

Define dependency security as a self-contained domain consumer of the [PDS Router](pds_router_design_spec.md) under the [canonical runtime lifecycle and quiescence design](../../cognitive_architecture/runtime_lifecycle_and_quiescence.md). A user-defined package may attach a complete vulnerability-management runtime, but the dependency-security domain remains the authority that interprets dependency inventory, advisory coverage, applicability, violations, remediation feasibility, and subsequent verification.

The request term domain security is represented here as `dependency-security`. CVE is one advisory identity system. It is not broad enough to name dependency inventory, non-CVE advisories, source coverage, remediation, and verification.

## Consumer Position

```text
external security runtime
→ dependency-security physical adapter
→ dependency-security canonical products
→ canonical Meld events and evidence
→ beliefs
→ independent Agents
→ Goals and capabilities
```

The external runtime may be a local process, library, sidecar, hosted service, or persistent monitor. Placement does not change package identity, result schema, owner validation, authority, evidence classes, or lineage obligations. Exact result values may differ when external source revisions differ.

Meld owns durable cognition, authority decisions, Goal lifecycle, task state, execution lineage, and canonical event history. The external runtime does not become a second Meld runtime.

## Use-Case Trace

The normalized [dependency security use case](../../persistent_domain_stewardship/examples/dependency_security/README.md) and its [router specification](../../persistent_domain_stewardship/examples/dependency_security/router_spec.md) are the use-case evidence for this workstream.

They place the following router and runtime behavior in the next iteration:

- one package routes security, world-model, and execution theory to their owners
- bounded clean posture requires explicit inventory, advisory, coverage, currency, and completeness evidence
- request attempts and passive source deliveries retain different lineage
- local scanner, subprocess, shared service, and persistent monitor placements preserve one domain result contract
- activation generation, participant incarnation, durable operation, attempt, cursor, and source revision remain distinct
- external results become canonical only after dependency-security admission
- source advance creates an observation opportunity rather than a direct belief mutation
- interrupted recovery and retired callbacks remain fenced

The wider example corpus does not expand dependency security into a universal source, graph, projection, disclosure, approval, or remediation framework. Source coherence is exercised here only through dependency-security-owned inventory and advisory admission. Future domains may reuse generic revision carriers while retaining their own sufficiency rules.

## Domain Ownership

Dependency security owns:

- dependency declaration and resolved-component meaning
- package ecosystem and version identity
- dependency coupling and remediation scope
- advisory identity and source provenance
- source coverage and source currency
- vulnerability applicability and affected-range interpretation
- severity and policy threshold meaning
- bounded clean, violated, unknown, and stale assessment posture
- remediation availability and feasibility meaning
- security verification requirements
- security violation and resolution domain products
- adapter interpretation of scanner and monitor output
- dependency-security theory routes and capability implementations

Dependency security does not own:

- workspace file identity or storage
- code editing or pull request authorship
- docs correctness
- canonical event append and replay
- belief revision mechanics
- Agent judgment
- effective authority calculation
- Goal, task, or execution state
- external advisory database internals
- global steward scheduling

## Package Composition

One dependency-security PDS package uses several routes. This is the intended router pressure test.

```text
dependency-security package manifest
├── dependency-security.policy.v1
├── world-model.belief-family.v1
├── world-model.belief-family.v1
├── world-model.outcome-mapping.v1
├── world-model.agent-curation-rule.v1
├── world-model.agent-maintained-condition.v1
├── world-model.strategy-theory.v1
├── Strategy semantic theory
└── governance semantics
```

The package manifest routes semantic components only. Each owner validates and installs its own body. The compatibility truth slice may still route exact capability and authority bodies to preserve its existing contract while the runtime split is implemented.

An illustrative component index:

| Component id | Route | Owner meaning |
| --- | --- | --- |
| `dependency-policy` | `dependency-security.policy.v1` | source set, coverage, severity, currency, and assessment rules |
| `security-coverage-belief` | `world-model.belief-family.v1` | whether required knowledge coverage is adequate |
| `security-posture-belief` | `world-model.belief-family.v1` | current bounded dependency-security posture |
| `security-outcomes` | `world-model.outcome-mapping.v1` | typed domain results admitted as belief evidence |
| `security-curation` | `world-model.agent-curation-rule.v1` | standing response posture |
| `security-condition` | `world-model.agent-maintained-condition.v1` | adequately assessed and no actionable violation |
| `security-strategy` | `world-model.strategy-theory.v1` | observation, feasibility, and verification action choices |
| `security-capability-*` | activation capability contribution | one exact published contract per activation-selected capability |
| `security-authority` | assignment governance selection | requested observation and security-evaluation actions |

## Dependency Security Policy Component

The domain-owned routed body has a versioned schema.

```rust
struct DependencySecurityPolicySpec {
    policy_id: String,
    subject_kind: String,
    ecosystems: Vec<String>,
    inventory_policy: InventoryPolicySpec,
    advisory_sources: Vec<AdvisorySourcePolicySpec>,
    coverage_policy: CoveragePolicySpec,
    severity_policy: SeverityPolicySpec,
    currency_policy: CurrencyPolicySpec,
    applicability_policy: ApplicabilityPolicySpec,
    remediation_policy: RemediationPolicySpec,
    verification_policy: VerificationPolicySpec,
}
```

The policy is state-free. It names required sources and rules but does not contain the latest inventory, advisory database revision, current finding, belief, Goal, or task state.

The dependency-security route handler validates and installs this body in an append-only owner registry. It returns one exact `TheoryRevisionRef` through the router receipt.

## Physical Activation Requirements

The dependency-security activation contributor derives physical requirements from the installed policy, assignment inputs, and activation-selected capability contracts.

Potential binding ids:

```text
workspace
security_runtime
advisory_source_credentials
repository_host
resolver_executable
verification_profile
```

Only selected behavior makes a binding required. A local OSV-style scanner may need `workspace` and `security_runtime` but no provider. A persistent hosted monitor may need credentials and an endpoint. A deterministic resolver may need an executable ref. Package identity remains the same across equivalent placements.

Credentials remain references in config and activation records. They never enter package bodies, package receipts, canonical events, evidence, or diagnostics.

The contributor also declares isolation requirements independently of placement. A local linked scanner may provide cooperative binding and result-admission isolation but cannot claim hard memory, crash, filesystem, or network containment. A subprocess may satisfy declared OS-enforced grants. A hosted monitor may satisfy host separation while still requiring explicit tenant, credential, callback, cursor, and source-revision isolation.

Two assignments may use the same exact policy component and external service while retaining distinct principals, subjects, grants, binding namespaces, adapter cursors, capability catalogs, activation generations, participant incarnations, and canonical assessment lineage.

The contributor returns an exact lifecycle participant plan. A typical security activation declares a bounded owner participant for domain admission, an operation adapter for scans and acquisitions, and a passive-source participant when advisory advance arrives independently. Each participant names its readiness dependencies, structural wake references, owner safe-point reference, stop reference, and required status.

Readiness proves exact adapter, binding, policy, source, cursor, and result-admission identity as applicable. Process existence alone is not readiness. A security participant may report active idle only with a durable reason and a resolvable workspace, timer, source-cursor, delivery, or operation-completion wake path.

## External Runtime Port

The adapter consumes a transport-neutral domain port.

```rust
trait DependencySecurityRuntimePort {
    fn dispatch_observe_inventory(
        &self,
        context: &AdapterInvocationContext,
        request: InventoryObservationRequest,
    ) -> Result<DependencySecurityDispatchReceipt, AdapterFailure>;

    fn dispatch_acquire_advisories(
        &self,
        context: &AdapterInvocationContext,
        request: AdvisoryAcquisitionRequest,
    ) -> Result<DependencySecurityDispatchReceipt, AdapterFailure>;

    fn dispatch_assess(
        &self,
        context: &AdapterInvocationContext,
        request: SecurityAssessmentRequest,
    ) -> Result<DependencySecurityDispatchReceipt, AdapterFailure>;

    fn dispatch_evaluate_remediation(
        &self,
        context: &AdapterInvocationContext,
        request: RemediationFeasibilityRequest,
    ) -> Result<DependencySecurityDispatchReceipt, AdapterFailure>;

    fn dispatch_verify(
        &self,
        context: &AdapterInvocationContext,
        request: SecurityVerificationRequest,
    ) -> Result<DependencySecurityDispatchReceipt, AdapterFailure>;

    fn poll_completion(
        &self,
        context: &AdapterInvocationContext,
    ) -> Result<DependencySecurityPollOutcome, AdapterFailure>;
}

enum DependencySecurityPollOutcome {
    Pending,
    Complete(DependencySecurityRuntimeCompletion),
}

enum DependencySecurityRuntimeCompletion {
    Inventory(DependencySecurityRuntimeResult<InventoryRuntimeResult>),
    Advisory(DependencySecurityRuntimeResult<AdvisoryRuntimeResult>),
    Assessment(DependencySecurityRuntimeResult<SecurityRuntimeAssessment>),
    Remediation(DependencySecurityRuntimeResult<RemediationRuntimeResult>),
    Verification(DependencySecurityRuntimeResult<SecurityRuntimeVerification>),
}

struct DependencySecurityDispatchReceipt {
    operation_key: String,
    attempt_id: String,
    activation_generation_id: String,
    participant_incarnation_id: String,
    adapter_implementation_ref: String,
    adapter_dispatch_ref: String,
}
```

Transport results are not canonical facts. The dependency-security adapter validates identity, coverage, source revision, scope, and result completeness before producing domain-owned canonical products.

Process exit status, HTTP success, task completion, or pull request creation never becomes a security verdict by itself.

Before dispatch, the operation adapter durably records the operation key, attempt identity, activation generation, participant incarnation, exact request, and admission policy. Dispatch returns a transport receipt within one bounded step. That receipt proves only adapter acceptance and does not prove that an effect began or a result exists. Completion arrives through a later poll, authenticated delivery, or reconciliation observation and is admitted by the dependency-security domain. A short local implementation may return completion on its first poll without changing this lifetime boundary.

## Runtime Result Envelope And Admission

Every request-response result returns a transport-neutral envelope before domain admission.

```rust
struct DependencySecurityRuntimeResult<T> {
    assignment_id: String,
    package_receipt_id: String,
    activation_id: String,
    capability_contract_ref: String,
    policy_revision: TheoryRevisionRef,
    operation_key: String,
    attempt_id: String,
    participant_id: String,
    participant_incarnation_id: String,
    activation_generation_id: String,
    adapter_implementation_ref: String,
    response_identity: String,
    observed_scope: Vec<DomainObjectRef>,
    source_revisions: Vec<ExternalSourceRevisionRef>,
    completeness: AdapterCompleteness,
    value: T,
}
```

The durable operation key remains stable across retry. Each attempt id, activation generation, and participant incarnation identify one physical realization. A timeout after dispatch is ambiguous. Retry either preserves the operation key under a new attempt or performs an explicit status or source reconciliation first.

Persistent monitor delivery uses a separate authenticated ingress rather than fabricating a capability invocation.

```rust
struct DependencySecuritySourceDelivery {
    assignment_subscription_ref: String,
    package_receipt_id: String,
    policy_revision: TheoryRevisionRef,
    participant_id: String,
    participant_incarnation_id: String,
    activation_generation_id: String,
    adapter_delivery_id: String,
    adapter_implementation_ref: String,
    source_cursor: String,
    source_revision: ExternalSourceRevisionRef,
    authenticated_envelope: Vec<u8>,
}
```

The dependency-security adapter rejects a response or delivery before canonical append when assignment, subject, policy revision, source revision, activation generation, participant incarnation, operation lineage, subscription, or completeness does not match the admitted request and scope.

A callback from a retired generation remains rejected or historical-only under explicit policy. It never silently advances current security posture. Shared external cursor, cache, or portfolio state cannot substitute for assignment-scoped Meld lineage.

## Canonical Domain Products

### Dependency inventory

```rust
struct DependencyInventorySnapshot {
    snapshot_id: String,
    subject: DomainObjectRef,
    manifest_refs: Vec<DomainObjectRef>,
    lockfile_refs: Vec<DomainObjectRef>,
    components: Vec<ResolvedDependencyComponent>,
    observed_workspace_revision: String,
    adapter_identity: String,
    observed_at: ReferenceTime,
}
```

Components retain package ecosystem, package identity, exact resolved version, declaration origin, and dependency path. Coupled dependencies may name one domain remediation scope while remaining distinct components.

### Advisory snapshot

```rust
struct AdvisoryKnowledgeSnapshot {
    snapshot_id: String,
    source_revisions: Vec<AdvisorySourceRevision>,
    covered_ecosystems: Vec<String>,
    advisories: Vec<NormalizedAdvisoryRecord>,
    acquired_at: ReferenceTime,
}
```

The normalized record retains every source advisory identity and may include a CVE id as an alias. Normalization never discards source provenance or collapses conflicting source claims into one untraceable record.

### Security assessment

```rust
enum DependencySecurityPosture {
    Unknown,
    CoverageInsufficient,
    CleanWithinCoverage,
    Violated,
    Stale,
    Conflicted,
}

struct DependencySecurityAssessment {
    assessment_id: String,
    subject: DomainObjectRef,
    inventory_snapshot_ref: String,
    advisory_snapshot_ref: String,
    policy_revision: TheoryRevisionRef,
    posture: DependencySecurityPosture,
    findings: Vec<SecurityFinding>,
    coverage: AssessmentCoverage,
    assessed_at: ReferenceTime,
}
```

`CleanWithinCoverage` is explicitly bounded. It is never serialized or presented as universally safe.

### Actionable violation

```rust
struct ActionableDependencyViolation {
    violation_id: String,
    subject: DomainObjectRef,
    assessment_ref: String,
    finding_refs: Vec<String>,
    affected_component_refs: Vec<String>,
    remediation_scope_refs: Vec<DomainObjectRef>,
    policy_revision: TheoryRevisionRef,
    advisory_source_refs: Vec<String>,
}
```

This product is a fact. It contains no requested assignee, command, provider prompt, branch mutation, or pull request instruction.

### Remediation feasibility

```rust
enum RemediationFeasibilityPosture {
    Unknown,
    Feasible,
    Coupled,
    NoKnownFix,
    ResolverConflict,
    PolicyBlocked,
}

struct RemediationFeasibilityAssessment {
    assessment_id: String,
    violation_ref: String,
    candidate_version_changes: Vec<DependencyVersionChange>,
    required_scopes: Vec<DomainObjectRef>,
    resolver_revision: String,
    posture: RemediationFeasibilityPosture,
}
```

Feasibility is security and dependency domain ground for planning. It is not authorization to edit files.

### Resolution assessment

```rust
struct DependencySecurityResolutionAssessment {
    resolution_id: String,
    violation_ref: String,
    change_evidence_refs: Vec<String>,
    new_inventory_snapshot_ref: String,
    new_advisory_snapshot_ref: String,
    verification_result_refs: Vec<String>,
    policy_revision: TheoryRevisionRef,
    posture: DependencySecurityPosture,
}
```

A commit ref may appear in `change_evidence_refs`. The resolved posture still requires new inventory and advisory assessment evidence.

## Bounded Negative Semantics

No finding is not automatically a clean result.

`CleanWithinCoverage` requires:

```text
exact subject scope
+ exact dependency inventory snapshot
+ exact advisory source set
+ exact advisory source revisions
+ declared ecosystem coverage
+ applicable severity policy
+ source currency satisfied
+ no admitted applicable finding at or above threshold
```

Missing inventory, unsupported ecosystem, unavailable source, stale source revision, incomplete transitive graph, or adapter truncation produces `Unknown`, `CoverageInsufficient`, or `Stale` as appropriate.

Generic closed-world negation must not be used to infer this posture. Dependency security produces a positive categorical assessment product that explicitly records the bounded negative proof.

## Evidence Non-Substitution

The domain keeps these evidence classes distinct:

| Evidence class | Proves | Does not prove |
| --- | --- | --- |
| inventory observation | dependencies and versions observed in one scope | advisory status |
| advisory acquisition | source content and revision acquired | applicability to subject |
| security assessment | policy interpretation over exact inventory and advisory refs | code change applied |
| resolver feasibility | candidate graph can or cannot resolve | vulnerability resolved |
| commit or pull request | code or package references changed | new graph is clean |
| build and test verification | selected software checks passed | advisory absence |
| subsequent security assessment | bounded current security posture | universal future safety |

Outcome mapping components must reject one evidence class satisfying another class requirement.

## Domain Events

Dependency security publishes canonical products through the existing event authority.

Recommended event type identities:

```text
dependency_security.inventory_observed.v1
dependency_security.advisory_snapshot_observed.v1
dependency_security.assessment_completed.v1
dependency_security.actionable_violation_assessed.v1
dependency_security.remediation_feasibility_assessed.v1
dependency_security.resolution_assessed.v1
dependency_security.source_advanced.v1
```

The event payload carries the canonical domain product intact. Event envelope metadata adds event identity, sequence, producer, time, and provenance. It does not repeat product fields beside the product.

An external monitor webhook is sensory input. The dependency-security adapter validates and translates it before canonical append.

## Belief Families

At least two epistemic concerns remain separate.

### Coverage belief

Question:

```text
Does this subject have adequate current dependency and advisory coverage under the installed policy?
```

Inputs include inventory snapshots, advisory source revisions, adapter completeness, supported ecosystems, and currency facts.

### Security posture belief

Question:

```text
What bounded dependency-security posture is supported for this subject under the installed policy?
```

Inputs include exact assessment products. Raw scanner matches and task completion do not directly settle this belief.

A remediation-feasibility belief may remain separate if Strategy requires durable comparison across resolver revisions. It must not be folded into security confidence merely to avoid another family.

## Maintained Condition

The standing condition is conceptually:

```text
coverage is adequate and current
and
security posture is clean within declared coverage
or
an explicit authorized risk posture satisfies the installed policy
```

The first proof may omit risk acceptance and treat any actionable violation as breached. Unknown, insufficient, stale, or conflicted posture cannot satisfy the condition.

## Capability Contracts

Dependency security publishes atomic capabilities.

| Capability | Inputs | Output | Effect class |
| --- | --- | --- | --- |
| `dependency_inventory_observe` | subject and workspace binding | inventory snapshot | read and emit |
| `advisory_knowledge_acquire` | source policy and credential refs | advisory snapshot | external read and emit |
| `dependency_security_assess` | inventory, advisories, policy | security assessment | emit |
| `dependency_remediation_evaluate` | violation, inventory, resolver binding | feasibility assessment | read and emit |
| `dependency_security_verify` | prior violation, changed subject, verification policy | resolution assessment | read and emit |

Each capability contract is published by the dependency-security domain into execution-owned product inventory. Activation selects the exact contract and the dependency-security capability contributor binds its physical invoker.

Observation, assessment, feasibility, and verification remain distinct capabilities even when one external executable implements all of them.

## Code Change Boundary

Dependency security does not publish a generic capability that edits dependency manifests, commits code, or opens a pull request.

If a complete external runtime can perform those actions, its code-change behavior must attach through a developer or code-change domain route and capability contract. That domain owns patch meaning, repository mutation, commit evidence, and pull request behavior.

Dependency security may publish candidate version changes and remediation scopes. Those are inputs to an independent developer steward, not commands.

## Event-To-Belief Cooperation

```text
dependency-security assessment
→ actionable violation fact
→ security Agent belief and maintained-condition breach

same actionable violation fact
→ developer Agent belief revision
→ developer Goal under independent authority
→ code change and commit evidence

changed dependency graph fact
→ dependency-security observation opportunity
→ new inventory and advisory assessment
→ bounded resolution fact

relevant workspace and public behavior changes
→ docs Agent belief revision
→ docs maintained condition evaluates independently
```

No package names another steward as a required next step. Each assignment declares which domain facts its own belief mappings consume.

The producer does not grant consumer authority. A developer Agent must already possess effective authority to modify code and create a pull request.

## Subject And Scope Discovery

The assignment begins with one bounded subject such as a workspace or repository. Dependency security derives ground subjects from admitted inventory:

```text
workspace assignment scope
→ manifest and lockfile observations
→ resolved component subjects
→ coupled remediation scope subjects
```

The domain owns derivation and stable identity of dependency and remediation subjects. The world-model Agent domain owns subscription and delivery mechanics.

Derived subject discovery must be bounded by assignment scope, deterministic from admitted inventory, idempotent, and retractable when inventory supersedes a component. It must not scan arbitrary neighboring repositories or subscribe globally by package name.

Whether this uses family-within-scope subscription or explicit eligible-subject publication remains an implementation decision. The public contract must preserve the domain-derived subject ref and assignment lineage either way.

## Source Advance And Wake

A later advisory revision can invalidate a prior clean or no-fix assessment without a workspace change.

The adapter publishes `dependency_security.source_advanced.v1` only after observing and validating a new exact source revision. This event creates an observation opportunity for affected assignments. It does not directly mutate their beliefs to violated or clean.

Elapsed-time currency requires an explicit semantic time signal and installed currency policy. Wall-clock polling behavior is adapter operation, not hidden belief logic.

The first implementation may support explicit source revision advance before time-based expiry. It must not claim elapsed-time freshness without the required event source.

## External Runtime Persistence

A complete external security runtime may retain:

- advisory databases
- component portfolio data
- its own scan cache
- webhook cursor
- operational retry state
- vendor-specific project identity

These remain external source truth or adapter operational state. Meld records exact external identity, source revision, admitted canonical domain product, provenance, and relevant adapter version.

Restart of Meld must reconstruct its own beliefs, Goals, and task state from Meld stores. It may reconnect to the external runtime for new observations but must not depend on hidden external Agent decisions.

## Dependency Security Storage

The domain requires append-only exact theory and canonical product stores under the external product storage root.

Conceptual concerns:

```text
dependency_security/theory
dependency_security/inventory
dependency_security/advisories
dependency_security/assessments
dependency_security/feasibility
dependency_security/verification
dependency_security/adapter_cursors
```

Domain records cite package receipt, policy revision, assignment, activation generation, adapter implementation, operation or passive-delivery lineage, source revisions, and canonical event refs where applicable. They do not duplicate full event envelopes or world-model belief state.

No store lives under the target workspace.

## Requirements By Domain

### Dependency security

| Requirement | Contract |
| --- | --- |
| `DS-R01` | publish and consume `dependency-security.policy.v1` through the router |
| `DS-R02` | own dependency inventory, advisory, assessment, feasibility, and verification schemas |
| `DS-R03` | keep package policy state-free and install it by exact revision |
| `DS-R04` | expose a transport-neutral external runtime port |
| `DS-R05` | validate external results before canonical admission |
| `DS-R06` | represent unknown, insufficient, clean within coverage, violated, stale, and conflicted posture distinctly |
| `DS-R07` | preserve advisory source and inventory revision lineage |
| `DS-R08` | preserve evidence classes without substitution |
| `DS-R09` | derive bounded dependency and remediation subjects from admitted inventory |
| `DS-R10` | publish actionable violation as a fact rather than a steward command |
| `DS-R11` | require subsequent assessment evidence before publishing resolution |
| `DS-R12` | retain external placement independence |

### Theory and configuration

| Requirement | Contract |
| --- | --- |
| `DS-R13` | install all package components through owner routes and one package receipt |
| `DS-R14` | bind workspace, security runtime, credentials, resolver, and verification refs only when required |
| `DS-R15` | keep package identity independent of physical bindings |
| `DS-R16` | permit several security package assignments without declaration-order priority |

### Events and world model

| Requirement | Contract |
| --- | --- |
| `DS-R17` | append canonical dependency-security products through existing event authority |
| `DS-R18` | map only admitted domain assessments into security posture belief evidence |
| `DS-R19` | keep coverage and posture concerns distinct |
| `DS-R20` | preserve unknown and stale as non-satisfying maintained-condition states |
| `DS-R21` | wake reassessment from explicit source advance or workspace change facts |
| `DS-R22` | use existing Agent, belief, Strategy, and Goal mechanics |

### Capability and execution

| Requirement | Contract |
| --- | --- |
| `DS-R23` | publish atomic inventory, acquisition, assessment, feasibility, and verification capabilities |
| `DS-R24` | bind exact invokers through the domain capability contributor |
| `DS-R25` | keep code mutation and pull request capabilities outside dependency-security ownership |
| `DS-R26` | preserve effective authority checks at admission, planning, and dispatch |
| `DS-R27` | declare exact effect targets for manifest, lockfile, source service, and external runtime interactions |
| `DS-R28` | let execution arbitrate conflicting effects without changing semantic plan edges |

### Workspace and developer consumers

| Requirement | Contract |
| --- | --- |
| `DS-R29` | consume workspace observations through public workspace contracts |
| `DS-R30` | never make workspace interpret dependency or advisory semantics |
| `DS-R31` | expose remediation candidate products for independent developer consumers |
| `DS-R32` | consume changed dependency graph facts for subsequent verification |

### Runtime isolation and admission

| Requirement | Contract |
| --- | --- |
| `DS-R33` | keep durable operation key, attempt id, activation generation, and participant incarnation as separate identities |
| `DS-R34` | expose authenticated passive source ingress for persistent monitor delivery |
| `DS-R35` | preserve assignment-scoped tenant, binding, cursor, catalog, and assessment lineage over shared runtimes |
| `DS-R36` | reject mismatched or incomplete external results before canonical append |
| `DS-R37` | classify callbacks from retired generations without silent current admission |
| `DS-R38` | preserve historical interpretation replay while the external runtime is offline |
| `DS-R39` | distinguish physical filesystem and network grants from semantic capability effects |
| `DS-R40` | publish an exact expected participant plan with readiness, wake, safe-point, stop, dependency, and required-status refs |
| `DS-R41` | keep external scan and acquisition dispatch bounded by persisting durable operation state before dispatch |
| `DS-R42` | admit external completion only through later owner validation under exact generation and incarnation lineage |
| `DS-R43` | distinguish active idle, quiescent, stalled, fenced quiescent, stopped, and interrupted security states |
| `DS-R44` | keep recovery admission closed until operations, deliveries, cursors, wake registrations, and owner state reconcile |
| `DS-R45` | permit equivalent participant restart under the same activation generation only through a new incarnation and readiness proof |
| `DS-R46` | prove assignment quiescence from the complete participant plan and owner receipts without requiring a cross-store transaction |

## Requirement Trace To Assessment

| Assessment requirement | Security requirements |
| --- | --- |
| `AR-10` | `DS-R01` through `DS-R12` |
| `AR-11` | `DS-R04`, `DS-R05`, `DS-R12`, `DS-R17`, `DS-R22` |
| `AR-12` | `DS-R10`, `DS-R31`, `DS-R32` |
| `AR-13` | `DS-R08`, `DS-R11`, `DS-R32` |
| `AR-16` | `DS-R01`, `DS-R03`, `DS-R13` |
| `AR-18` | `DS-R03`, `DS-R12`, `DS-R22` |
| `AR-19` | `DS-R12`, `DS-R14`, `DS-R15`, `DS-R35`, `DS-R39` |
| `AR-20` | `DS-R13`, `DS-R24`, `DS-R31`, `DS-R35` |
| `AR-21` | `DS-R33` through `DS-R46` |

## Expected Domain-First Code Shape

```text
src/dependency_security.rs
src/dependency_security/contracts.rs
src/dependency_security/theory.rs
src/dependency_security/inventory.rs
src/dependency_security/advisory.rs
src/dependency_security/assessment.rs
src/dependency_security/remediation.rs
src/dependency_security/verification.rs
src/dependency_security/capability.rs
src/dependency_security/adapter.rs
src/dependency_security/events.rs
src/dependency_security/persistence.rs
```

Behavior may be split further only when implementation evidence requires it. No `mod.rs` file is used.

## Verification And Acceptance Criteria

### Router consumption

- `DS-AC01` one dependency-security package installs components through dependency-security, world-model, and execution routes
- `DS-AC02` root theory and runtime code contains no dependency-security body types or route branches
- `DS-AC03` exact historical security policy resolves after a newer package revision installs
- `DS-AC04` one package activates against a local fake runtime and a serialized fake runtime with identical canonical results

### Assessment truth

- `DS-AC05` missing advisory source coverage cannot produce clean posture
- `DS-AC06` stale advisory revision cannot produce current clean posture
- `DS-AC07` no findings with complete declared coverage produces clean within coverage
- `DS-AC08` an applicable above-threshold finding produces violated posture
- `DS-AC09` conflicting source interpretation remains conflicted and preserves both sources
- `DS-AC10` unsupported ecosystems remain coverage insufficient

### Evidence separation

- `DS-AC11` scanner task success alone creates no clean evidence
- `DS-AC12` resolver feasibility cannot satisfy security verification
- `DS-AC13` commit evidence cannot satisfy resolution
- `DS-AC14` subsequent inventory and advisory assessment can satisfy bounded resolution
- `DS-AC15` every resolution cites the prior violation, new inventory, source revisions, policy revision, and verification refs

### Independent stewardship

- `DS-AC16` actionable violation event contains no developer command or authority grant
- `DS-AC17` a developer assignment may independently consume the violation fact
- `DS-AC18` absence of a developer assignment leaves the security fact durable without creating code work
- `DS-AC19` changed dependency graph wakes security reassessment without a manual workflow edge
- `DS-AC20` docs freshness receives only relevant workspace facts and is not directly called by security

### Runtime and durability

- `DS-AC21` restart resumes from existing Meld events, beliefs, Goals, and domain cursors
- `DS-AC22` external runtime storage is not copied into Meld by default
- `DS-AC23` external runtime outage produces adapter failure or unknown posture, never clean
- `DS-AC24` no dependency-security state is written under the target workspace
- `DS-AC25` source advance creates an observation opportunity before any posture change

### Capabilities and authority

- `DS-AC26` selected atomic capabilities bind by exact contract identity
- `DS-AC27` unselected code-mutation behavior in an external tool is unreachable through the security contributor
- `DS-AC28` provider-free scanner activation requires no model provider
- `DS-AC29` denied effective authority prevents acquisition or assessment dispatch as declared by policy
- `DS-AC30` coupled remediation scopes preserve full domain object identity for execution effect arbitration

### Isolation and external runtime behavior

- `DS-AC31` two assignments over one subject under distinct policy revisions produce separate assessments and never share grants, bindings, catalogs, or current state
- `DS-AC32` inventory supersession retracts derived subject eligibility idempotently without rewriting prior history or another assignment
- `DS-AC33` a result with mismatched subject, assignment, policy revision, activation generation, operation key, attempt id, or source revision is rejected before canonical append
- `DS-AC34` a shared external runtime cursor or cache cannot substitute for assignment-scoped Meld lineage
- `DS-AC35` concurrent inventory and advisory revision advances cannot produce clean posture from a mixed snapshot
- `DS-AC36` retry after participant or activation-generation change preserves one operation key and records a new attempt with exact generation and incarnation lineage
- `DS-AC37` a retired-generation callback remains rejected or historical-only and never silently changes current posture
- `DS-AC38` historical interpretation replay succeeds with the external runtime offline
- `DS-AC39` required hard filesystem or network isolation rejects a cooperative in-process realization
- `DS-AC40` scanner readiness proves exact adapter and source identity rather than process existence alone
- `DS-AC41` graceful retirement checkpoints assignment-scoped advisory and workspace cursors before the generation is retired cleanly
- `DS-AC42` interrupted scan recovery preserves the operation key, unresolved effect state, source lineage, and retired generation fence

### Lifecycle and recovery

- `DS-AC43` a clean zero-finding result cannot prove quiescence while a required participant or structural wake path is absent
- `DS-AC44` a fully idle security activation proves each expected participant has a durable wait reason and resolvable wake path
- `DS-AC45` crash after durable operation creation but before dispatch resumes or retires the operation without duplicate semantic work
- `DS-AC46` crash after external dispatch but before admission reconciles the ambiguous attempt before retry or canonical append
- `DS-AC47` an equivalent operation-adapter restart creates a new participant incarnation under the same activation generation only after readiness closes
- `DS-AC48` a callback from an older participant incarnation cannot alter current posture without explicit historical admission policy
- `DS-AC49` source advance wakes the passive-source participant without a manual workflow edge
- `DS-AC50` recovery keeps admission closed until operation, delivery, source cursor, wake registration, and owner checkpoints reconcile
- `DS-AC51` fenced quiescence aggregates owner receipts over the exact participant plan while each owner persists its own safe point

## Explicit Deferrals

- scanner and advisory provider selection
- production polling interval
- package signing
- reachability analysis beyond the selected adapter contract
- risk acceptance workflow
- automatic code mutation domain design
- pull request provider selection
- cross-steward conflict policy
- mandatory remote process isolation
- universal ecosystem and package identity standardization

## Rejection Criteria

Reject an implementation that adds:

- CVE-specific variants to core language or event enums
- one scanner success boolean as security posture
- one opaque security bot capability covering observation through code mutation
- direct dependency-security writes to belief, Agent, Goal, or task stores
- direct calls from the security Agent to developer or docs Agents
- clean posture inferred from missing findings
- resolution inferred from commit or pull request creation
- hidden adapter state required to replay Meld decisions
- package credentials or endpoints in semantic package identity

## Completion Condition

The consumer design is satisfied when a user-defined dependency-security package installs through the PDS Router, binds a complete external runtime through a domain adapter, produces provenance-rich bounded assessments, drives existing Meld cognition through typed evidence, and cooperates with independent stewards through durable facts without owning their actions.
