# Stewardship Facet Protocol

Date: 2026-07-16  
Status: proposed  
Scope: candidate connector between the PDS control plane and domain-owned stewardship semantics

## Purpose

Persistent Domain Stewardship needs a cross-domain integration mechanism.

The mechanism should allow one package and profile to activate observation, belief, Agent, execution, outcome, and governance behavior without making PDS authoritative for those domains.

The Stewardship Facet Protocol is one candidate solution.

It is not an accepted API.

## Facet Definition

A stewardship facet is a domain-owned contribution to a stewardship package.

Examples:

- a sensory observation requirement
- an event-to-graph projection plan
- a belief-family definition
- an Agent charter and concern policy
- an execution method library
- a context-projection policy
- an outcome-evidence mapping
- an authority-requirement declaration

The owning domain defines the facet's schema, validation, compilation, activation, and inspection semantics.

PDS links facets and coordinates lifecycle.

## Why Facets

A single universal package schema has useful coherence but risks making PDS the owner of foreign domain concepts.

Facets permit this split:

```text
PDS understands:
    package identity
    facet identity
    imports and exports
    requirements
    activation order
    receipts
    lineage

Domain understands:
    facet payload
    semantic validity
    local registration
    runtime behavior
    local status
```

## Candidate Source Package

```rust
struct StewardshipPackageSpec {
    manifest: PackageManifest,
    profile_surface: ProfileSurfaceSpec,
    imports: Vec<PackageImport>,
    facets: Vec<FacetSourceRef>,
    bindings: Vec<CrossDomainBinding>,
    scenarios: Vec<StewardshipScenarioRef>,
}
```

The package may be authored as one physical file or several modules.

Physical layout does not determine ownership.

## Facet Source Reference

```rust
struct FacetSourceRef {
    facet_id: FacetId,
    domain_id: DomainId,
    facet_kind: FacetKind,
    schema_version: SchemaVersion,
    source: SourceLocation,
}
```

Candidate facet kinds include:

```text
domain_model
observation
projection
belief
agent_charter
context_projection
method_library
capability_requirements
outcome_verification
governance
```

Facet kinds may be domain-specific rather than one globally frozen enum.

## Compiled Facet Envelope

PDS should not require access to every compiled domain payload.

Candidate envelope:

```rust
struct CompiledFacetEnvelope {
    facet_id: FacetId,
    domain_id: DomainId,
    facet_kind: String,
    schema_version: SchemaVersion,
    facet_hash: FacetHash,

    payload: OpaqueDomainPayload,

    exports: Vec<SymbolExport>,
    imports: Vec<SymbolImport>,
    activation_requirements: Vec<ActivationRequirement>,
    requested_authority: Vec<AuthorityRequirement>,
    diagnostics: Vec<FacetDiagnostic>,
}
```

PDS links the envelope.

The domain owns the opaque payload and may deserialize it only through its own versioned contract.

## Symbols

Facets communicate through exported and imported symbols rather than shared internal objects.

Example exports:

```text
object_type: software.service
belief_family: performance.health
action_class: performance.run_benchmark
method: performance.investigate_regression
artifact_type: performance.benchmark_report
verification_profile: release_grade
```

Example imports:

```text
Agent charter imports belief_family: performance.health
Agent concern imports action_class: performance.run_benchmark
outcome facet imports artifact_type: performance.benchmark_report
profile surface imports verification_profile: release_grade
```

## Symbol Linker

The PDS linker validates:

- symbol existence
- namespace and version compatibility
- cardinality requirements
- declared cross-domain bindings
- package import closure
- activation requirement closure
- package-level lineage

The linker does not validate the internal semantic meaning of a belief comparator or execution method.

## Connector Options

## Option A: Domain compiler and runtime connector

Each domain exposes compile, prepare, activate, deactivate, migrate, and inspect operations.

Conceptual interface:

```rust
trait StewardshipFacetConnector {
    fn descriptor(&self) -> FacetDescriptor;

    fn compile(
        &self,
        source: FacetSource,
        links: &LinkContext,
    ) -> Result<CompiledFacetEnvelope>;

    fn prepare(
        &self,
        facet: &CompiledFacetEnvelope,
        activation: &ActivationContext,
    ) -> Result<PreparedFacetReceipt>;

    fn activate(
        &self,
        prepared: &PreparedFacetReceipt,
    ) -> Result<ActiveFacetReceipt>;

    fn deactivate(
        &self,
        active: &ActiveFacetReceipt,
    ) -> Result<()>;

    fn inspect(
        &self,
        active: &ActiveFacetReceipt,
    ) -> Result<FacetStatus>;
}
```

### Advantages

- strongest domain ownership
- version and validation remain local
- PDS core remains independent of domain internals

### Costs

- broad connector lifecycle
- distributed diagnostics
- complex multi-domain upgrades

## Option B: PDS compiler plus domain validator

PDS compiles a generic package representation. Domains validate and register the resulting plans.

### Advantages

- simpler central authoring and tooling
- less domain compiler infrastructure

### Costs

- PDS must understand more foreign semantics
- central schema evolves with all domains

## Option C: Root adapters only

PDS produces generic registration commands. Root `meld` adapters translate them into domain-specific calls.

### Advantages

- suitable for an initial implementation
- no new connector API in extracted crates

### Costs

- root can become the hidden semantic owner
- adapter proliferation
- difficult portability and testability

## Current Recommendation

Use Option C for a tightly bounded first proof if necessary, but preserve contracts that can evolve toward Option A.

Do not freeze one universal PDS schema before testing federated domain compilation.

## Compilation Lifecycle

Candidate package compilation:

```text
parse package and profile
    ↓
resolve package imports
    ↓
route facet sources to owning domain compilers
    ↓
collect compiled facet envelopes
    ↓
link imports and exports
    ↓
resolve profile presets and overrides
    ↓
validate activation and authority requirements
    ↓
run package and domain conformance scenarios
    ↓
produce CompiledStewardshipImage
```

## Compiled Stewardship Image

```rust
struct CompiledStewardshipImage {
    identity: CompiledPackageIdentity,
    profile_surface: CompiledProfileSurface,
    facets: Vec<CompiledFacetEnvelope>,
    symbol_table: LinkedSymbolTable,
    activation_graph: ActivationDependencyGraph,
    requested_authority: Vec<AuthorityRequirement>,
    scenario_summary: ScenarioSummary,
    provenance: CompilationProvenance,
}
```

The image is deterministic and content-addressed.

## Assignment And Activation

The package image is reusable.

An assignment binds:

- profile
- principal
- concrete subject scope
- authority grant
- lifecycle

An activation binds:

- external sources
- credentials
- provider and capability implementations
- runtime placement
- operational budgets

This separation remains a recommendation rather than a settled requirement.

## Activation Receipts

Each domain returns a durable receipt.

```rust
struct ActiveFacetReceipt {
    activation_id: ActivationId,
    facet_id: FacetId,
    domain_id: DomainId,
    facet_hash: FacetHash,
    local_registration_refs: Vec<DomainObjectRef>,
    activated_at_seq: u64,
    status: FacetActivationStatus,
}
```

PDS stores receipt identity and lifecycle.

The domain remains authoritative for the referenced local registrations.

## Activation Saga

A multi-domain activation cannot assume one transaction.

Candidate lifecycle:

```text
Draft
→ Compiled
→ Linked
→ Preparing
→ Prepared
→ Activating
→ Active
```

Failure and terminal states:

```text
CompileFailed
LinkFailed
PrepareFailed
ActivationDegraded
Compensating
Suspended
Retired
```

Candidate process:

1. Resolve package, profile, assignment, and activation inputs.
2. Calculate effective authority.
3. Ask each domain to prepare its registrations.
4. Persist preparation receipts.
5. Activate in dependency order.
6. Persist active receipts.
7. Emit activation lifecycle events.
8. Start or wake generic runtime actors.

If one domain fails, PDS records the failure and coordinates compensation. It does not directly mutate another domain's store.

## Deactivation And Upgrade

A package upgrade may change:

- exported symbols
- belief semantics
- Agent policy
- methods
- authority requests
- outcome verification
- activation requirements

Candidate upgrade process:

```text
compile new image
→ compare semantic and authority diff
→ prepare migrations per domain
→ obtain required approval
→ activate at a sequence boundary
→ retain old image and receipts for replay
```

Domains own local migration plans.

PDS owns cross-domain ordering and status.

## Inspection

A facet connector should provide a user-safe status projection:

```rust
struct FacetStatus {
    facet_id: FacetId,
    domain_id: DomainId,
    lifecycle: FacetLifecycle,
    registrations: Vec<RegistrationSummary>,
    diagnostics: Vec<FacetDiagnostic>,
    source_refs: Vec<DomainObjectRef>,
}
```

PDS combines statuses into one stewardship projection without becoming authoritative for domain state.

## Security

- PDS must not execute arbitrary facet source code while parsing.
- Domain compiler and connector implementations are trusted plugins or built-in code.
- Package-requested authority remains a request.
- Activation uses the intersection of package request, principal grant, runtime policy, and current restrictions.
- Dispatch must enforce authority independently of package activation.
- Facet payloads and receipts require version and hash validation.

## Required Experiments

1. Compile the documentation steward using a central schema.
2. Compile the same steward using domain-owned facets.
3. Add a game-faction steward without adding new PDS core fields.
4. Upgrade one belief facet while preserving profile intent.
5. Change one activation binding without changing assignment identity.
6. Simulate a failed execution-facet activation after belief and Agent preparation.
7. Verify exact-hash replay across package upgrade.

## Falsification

The facet model should be rejected or simplified if:

- every connector merely forwards identical generic structures;
- cross-domain linking is more complex than the domain isolation it preserves;
- package authors must understand domain internals regardless of the facet boundary;
- root adapters remain the real semantic authority;
- non-software packages still require changes to PDS core types.

See [Meta-Domain](meta_domain.md) and [Open Decisions](open_decisions.md).