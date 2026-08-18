# PDS W05 Dependency-Security Truth-Slice Design

Date: 2026-08-18
Status: accepted for authorized implementation
Mapped workstream: `W05`
Scope: one bounded read-only dependency-security package over deterministic Cargo inventory and advisory fixtures

## Objective

Prove that the routed package, assignment, activation, belief, Agent, Strategy, and execution contracts support a second domain with truthful bounded negative evidence.

This slice is a domain and contract proof. It is not a production dependency-security product.

The slice also acts as a falsification case for the [PDS Semantic Interface Review](pds_semantic_interface_review.md). Dependency security confirms that domain-owned belief questions and bounded proof semantics must cross the interface. The [canonical PDS architecture](../../cognitive_architecture/persistent_domain_stewardship.md) rejects probability-shaped conclusions and exact capability-bearing Strategy bodies as the generic pattern.

## Authorization Boundary

The slice includes:

- deterministic fixture-backed Cargo dependency inventory
- deterministic fixture-backed advisory knowledge
- explicit coverage and currency
- assessment and independent calculation verification
- local and fake serialized adapters over one result contract
- owner admission before canonical append
- coverage and posture beliefs
- one maintained condition and read-only activation capability set

The slice excludes:

- production Cargo metadata or lockfile resolution
- real OSV, CVE, GitHub, Snyk, scanner, or hosted service integration
- source credential behavior
- remediation feasibility
- version selection or dependency update planning
- repository mutation, commit, branch, or pull-request behavior
- persistent advisory monitoring
- cross-steward commands

## Ownership

Dependency security owns inventory interpretation, advisory normalization within the fixture schema, coverage, currency, applicability, posture, calculation verification, adapter admission, domain events, and its capability implementations.

Workspace owns fixture file bytes and workspace source identity only. Events carry admitted products unchanged. World model owns belief, maintained conditions, Agent judgment, Goals, and Strategy. Execution owns exact capability contracts, authority enforcement, task state, and dispatch behavior.

## Domain-First Code Shape

```text
src/dependency_security.rs
src/dependency_security/contracts.rs
src/dependency_security/policy.rs
src/dependency_security/inventory.rs
src/dependency_security/advisory.rs
src/dependency_security/assessment.rs
src/dependency_security/verification.rs
src/dependency_security/adapter.rs
src/dependency_security/capability.rs
src/dependency_security/events.rs
src/dependency_security/persistence.rs
src/dependency_security/theory.rs
```

`src/dependency_security.rs` declares its children directly. No `mod.rs` file is introduced.

## Maintained Subject

The proof subject is one exact Cargo dependency graph fixture under a workspace subject.

```rust
struct DependencySecuritySubjectV1 {
    subject: DomainObjectRef,
    ecosystem: PackageEcosystem,
    inventory_scope: InventoryScopeV1,
}

enum PackageEcosystem {
    Cargo,
}

struct InventoryScopeV1 {
    manifest_ref: DomainObjectRef,
    lockfile_ref: DomainObjectRef,
    include_transitive: bool,
}
```

Schema version one requires `Cargo`, one manifest ref, one lockfile ref, and `include_transitive` equal to true. The adapter consumes fixture documents that already contain resolved components. It does not parse an arbitrary real workspace.

## Policy Component

The package routes one `dependency-security.policy.v1` body.

```rust
struct DependencySecurityPolicyV1 {
    policy_id: String,
    subject_kind: String,
    ecosystem: PackageEcosystem,
    required_advisory_source_id: String,
    required_coverage: CoverageRequirementV1,
    currency: CurrencyRequirementV1,
    severity_threshold: SeverityThresholdV1,
    verification: VerificationRequirementV1,
}

struct CoverageRequirementV1 {
    require_complete_inventory: bool,
    require_all_components_covered: bool,
    require_transitive_dependencies: bool,
}

struct CurrencyRequirementV1 {
    maximum_source_age_seconds: u64,
    maximum_inventory_age_seconds: u64,
}
```

Reference time is injected into deterministic requests. The policy never reads wall time directly.

The authorized fixture policy requires complete transitive inventory, coverage for every component, one exact fixture source, and no finding at or above the configured threshold.

## Package Composition

| Component id | Route | Meaning |
| --- | --- | --- |
| `security-policy` | `dependency-security.policy.v1` | bounded source, coverage, currency, severity, and verification rules |
| `security-coverage-belief` | `world-model.belief-family.v1` | current knowledge sufficiency |
| `security-posture-belief` | `world-model.belief-family.v1` | bounded current posture |
| `security-outcome-mapping` | `world-model.outcome-mapping.v1` | admitted security products to evidence |
| `security-curation-rule` | `world-model.agent-curation-rule.v1` | Goal curation for observation and assessment |
| `security-maintained-condition` | `world-model.agent-maintained-condition.v1` | coverage adequate and posture clean within coverage |
| `security-strategy` | `world-model.strategy-theory.v1` | compatibility aggregate carrying stable security Strategy semantics |
| `security-authority` | `execution.authority-policy.v1` | compatibility governance input for read and emit effects only |
| `security-capability-inventory` | activation capability contribution | observe fixture inventory |
| `security-capability-advisories` | activation capability contribution | acquire fixture advisory knowledge |
| `security-capability-assess` | activation capability contribution | calculate bounded posture |
| `security-capability-verify` | activation capability contribution | independently verify calculation integrity |

The authority policy contains no filesystem write, repository write, process mutation, or remote side effect.

## Canonical Inventory

```rust
struct DependencyInventorySnapshotV1 {
    snapshot_id: String,
    subject: DependencySecuritySubjectV1,
    workspace_revision: String,
    manifest_content_hash: String,
    lockfile_content_hash: String,
    components: Vec<ResolvedDependencyComponentV1>,
    completeness: InventoryCompleteness,
    observed_at: ReferenceTime,
}

struct ResolvedDependencyComponentV1 {
    component_id: String,
    ecosystem: PackageEcosystem,
    package_name: String,
    resolved_version: String,
    source_identity: String,
    dependency_paths: Vec<Vec<String>>,
}

enum InventoryCompleteness {
    CompleteTransitive,
    Incomplete { reasons: Vec<String> },
}
```

Snapshot identity hashes every semantic field except `snapshot_id`. Components sort by package name, resolved version, source identity, and component id. Dependency paths sort lexically.

The adapter proposes a raw inventory. Dependency security validates the exact scope, component uniqueness, version presence, source identity, completeness declaration, and content hashes before storing the canonical snapshot.

## Canonical Advisory Knowledge

```rust
struct AdvisoryKnowledgeSnapshotV1 {
    snapshot_id: String,
    source_id: String,
    source_revision: String,
    covered_ecosystem: PackageEcosystem,
    covered_components: Vec<ComponentCoverageV1>,
    advisories: Vec<NormalizedAdvisoryV1>,
    completeness: AdvisoryCompleteness,
    acquired_at: ReferenceTime,
}

struct ComponentCoverageV1 {
    package_name: String,
    source_identity: String,
}

struct NormalizedAdvisoryV1 {
    source_advisory_id: String,
    aliases: Vec<String>,
    package_name: String,
    affected_ranges: Vec<VersionRangeV1>,
    severity: SeverityV1,
}

enum AdvisoryCompleteness {
    CompleteForDeclaredCoverage,
    Incomplete { reasons: Vec<String> },
}
```

Snapshot identity hashes all semantic fields except `snapshot_id`. Coverage and advisories use deterministic sort order. Conflicting records retain distinct source advisory ids and are not silently merged.

## Assessment Contract

```rust
enum DependencySecurityPosture {
    Unknown,
    Insufficient,
    CleanWithinCoverage,
    Violated,
    Stale,
    Conflicted,
}

struct DependencySecurityAssessmentV1 {
    assessment_id: String,
    subject: DependencySecuritySubjectV1,
    inventory_snapshot_ref: String,
    advisory_snapshot_ref: String,
    policy_revision: TheoryRevisionRef,
    reference_time: ReferenceTime,
    posture: DependencySecurityPosture,
    findings: Vec<SecurityFindingV1>,
    coverage: AssessmentCoverageV1,
    reasons: Vec<AssessmentReasonV1>,
}
```

Posture selection is deterministic in this precedence order:

1. `Unknown` when a required exact input is absent or unresolved.
2. `Conflicted` when applicable fixture records disagree in a way the bounded policy cannot resolve.
3. `Stale` when required inventory or advisory knowledge exceeds policy currency.
4. `Insufficient` when inventory or advisory coverage is incomplete.
5. `Violated` when one or more applicable findings meet the severity threshold.
6. `CleanWithinCoverage` when every required proof is present and no threshold finding applies.

An assessment may retain findings while its overall posture is stale, insufficient, or conflicted. Presentation must never shorten `CleanWithinCoverage` to an unqualified safe or clean state.

## Bounded Negative Proof

`CleanWithinCoverage` requires all of:

```text
exact subject and Cargo scope
exact complete transitive inventory snapshot
exact fixture advisory source and revision
coverage record for every inventory component
complete advisory declaration for that coverage
inventory and advisory currency under injected reference time
no unresolved source conflict
no applicable finding at or above threshold
```

Silence, empty adapter output, transport success, process exit, task success, or missing advisory records cannot establish clean posture.

## Calculation Verification

Verification proves calculation integrity only. It does not prove remediation or future safety.

```rust
struct DependencySecurityVerificationV1 {
    verification_id: String,
    assessment_ref: String,
    inventory_snapshot_ref: String,
    advisory_snapshot_ref: String,
    policy_revision: TheoryRevisionRef,
    independently_computed_posture: DependencySecurityPosture,
    independently_computed_finding_refs: Vec<String>,
    checks: Vec<VerificationCheckV1>,
    verified: bool,
}
```

The verify capability reloads exact canonical inputs, recomputes assessment through a separate pure entry point, and compares posture, findings, coverage, and reasons. A verification over changed inputs is invalid rather than a new security posture.

## Adapter Contract

Both authorized adapters implement one port:

```rust
trait DependencySecurityTruthAdapter: Send + Sync {
    fn observe_inventory(
        &self,
        request: InventoryObservationRequestV1,
    ) -> Result<DependencySecurityAdapterResultV1, AdapterFailure>;

    fn acquire_advisories(
        &self,
        request: AdvisoryAcquisitionRequestV1,
    ) -> Result<DependencySecurityAdapterResultV1, AdapterFailure>;
}

struct DependencySecurityAdapterResultV1 {
    assignment_id: String,
    package_receipt_id: String,
    activation_id: String,
    activation_generation_id: String,
    operation_key: String,
    attempt_id: String,
    adapter_implementation_ref: String,
    response_identity: String,
    observed_scope: Vec<DomainObjectRef>,
    source_revisions: Vec<String>,
    completeness: AdapterCompleteness,
    value: DependencySecurityAdapterValueV1,
}
```

The local adapter reads typed fixture values in process. The fake serialized adapter sends and receives canonical JSON bytes through an in-memory or loopback test transport. It does not start a real external scanner.

Both adapters must produce byte-equivalent semantic `value` after decoding the same fixture. Transport and response identities may differ and remain lineage, not domain meaning.

## Owner Admission

Adapter output becomes canonical only through dependency-security admission.

Admission validates:

- assignment, package, activation, and generation match the durable operation
- capability contract and adapter implementation were selected for the activation
- operation and attempt are admitted and not retired
- subject and scope equal the request
- source revision equals the requested fixture source
- completeness is explicit
- raw value validates under the exact policy revision
- response identity has not already produced a conflicting canonical product

Admission either returns one exact canonical product ref, returns unchanged for an identical retry, or rejects without event append.

## Canonical Events

The slice publishes:

```text
dependency_security.inventory_observed.v1
dependency_security.advisory_snapshot_observed.v1
dependency_security.assessment_completed.v1
dependency_security.assessment_verification_completed.v1
dependency_security.source_advanced.v1
```

The event payload carries one canonical product intact. The event envelope supplies canonical sequence, producer, subject, causation, and provenance.

`source_advanced` carries a new fixture source revision and creates an observation opportunity. It does not mutate a belief directly.

## Belief And Maintained Condition

Coverage and posture remain separate belief families.

Coverage belief consumes only admitted inventory, advisory, and assessment coverage products. Posture belief consumes only admitted assessment and calculation-verification products.

The maintained condition is satisfied only when:

```text
coverage belief supports adequate and current
and
posture belief supports clean within coverage
and
the latest required calculation verification is verified
```

Unknown, insufficient, stale, conflicted, or violated cannot satisfy it.

## Capability Effects

| Capability | Effect contract |
| --- | --- |
| inventory observe | read fixture and emit raw observation for owner admission |
| advisory acquire | read fixture source and emit raw observation for owner admission |
| assess | read exact canonical products and emit assessment |
| verify | read exact canonical products and emit calculation verification |

All four are read-and-emit behavior. None declares workspace mutation, repository mutation, process control, credential change, or remote write.

## Persistence

Dependency security owns versioned external product trees:

```text
dependency_security_policy_revisions_v1
dependency_security_inventories_v1
dependency_security_advisories_v1
dependency_security_assessments_v1
dependency_security_verifications_v1
dependency_security_adapter_admissions_v1
```

Products are immutable by content identity. Admission dedupe keys include assignment id, operation key, response identity, product kind, and canonical product id.

No data is written under the target workspace.

## Fixture Set

```text
tests/fixtures/pds/dependency_security/v1/
├── clean/
├── violated/
├── unknown/
├── insufficient_inventory/
├── insufficient_coverage/
├── stale_inventory/
├── stale_advisories/
├── conflicted/
└── tampered_response/
```

Each fixture contains a typed Cargo inventory document, advisory snapshot document, policy document, injected reference time, and expected canonical products.

## Verification Design

Focused tests:

```text
dependency_security::policy::tests::policy_is_state_free_and_exact
dependency_security::inventory::tests::inventory_identity_is_order_independent
dependency_security::advisory::tests::advisory_identity_retains_source_revision
dependency_security::assessment::tests::posture_precedence_is_deterministic
dependency_security::assessment::tests::empty_findings_do_not_imply_clean
dependency_security::assessment::tests::clean_requires_complete_current_coverage
dependency_security::verification::tests::verification_recomputes_exact_inputs
dependency_security::adapter::tests::local_and_serialized_values_are_equivalent
dependency_security::admission::tests::transport_success_is_not_canonical
dependency_security::admission::tests::tampered_or_duplicate_conflicting_response_is_rejected
dependency_security::events::tests::source_advance_is_observation_opportunity
tests::pds_w05_dependency_security::package_runs_read_only_loop
tests::pds_w05_dependency_security::provider_free_activation_reaches_truthful_posture
```

Static checks:

```sh
rg -n "dependency_security|DependencySecurity" src/runtime src/theory crates/meld-lang/src
rg -n "write|commit|pull_request|remediat" src/dependency_security/capability.rs
rg --files src/dependency_security src | rg '/mod\.rs$'
```

The first search must find no application branches. The second must find no mutation or remediation capability. The third must find no new `mod.rs` file.

## Acceptance Criteria

- one package installs through dependency-security, world-model, and execution routes
- all six postures have deterministic fixtures
- clean posture requires exact complete current coverage evidence
- local and fake serialized adapters admit the same canonical product shapes
- adapter or transport success cannot establish posture
- calculation verification cannot be mistaken for remediation verification
- source advance creates an observation opportunity only
- the maintained condition cannot be satisfied by unknown, stale, insufficient, conflicted, or violated state
- provider-free execution is complete
- root and generic crates contain no dependency-security branches
- no code mutation or pull-request effect exists

## Rejection Criteria

Reject implementation if it:

- reads a real workspace or external advisory service as the acceptance path
- calls no findings clean without explicit coverage
- collapses inventory, advisory, assessment, and verification into one result
- treats calculation verification as vulnerability resolution
- allows adapter output to bypass owner admission
- adds dependency-security variants to Meld Lang or generic runtime contracts
- includes remediation feasibility or repository mutation
- requires a provider

## Decision Ledger

- The proof ecosystem is Cargo only.
- Inventory and advisory inputs are typed deterministic fixtures.
- Reference time is injected.
- Overall posture follows an exact precedence order.
- Verification proves calculation integrity, not remediation.
- The assembled Strategy problem has an activation-local snapshot of four read-only capabilities.
- Local and fake serialized adapters share one admitted result contract.
- Production integrations remain outside the slice.

## Completion Condition

`D05` is delivered. `W05` may implement the bounded dependency-security truth slice without further cross-domain design decisions.
