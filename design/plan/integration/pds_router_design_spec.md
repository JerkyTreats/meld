# PDS Router Detailed Design Specification

Date: 2026-08-18
Status: aligned implementation and migration design
Scope: one package entry, domain-owned theory routes, exact package installation, assignment, activation, and independently owned capability attachment

## Objective

Define the PDS Router as the single semantic attachment point for external user-defined stewardship packages. The router accepts one package entry, dispatches each component to a route published by its owning domain, proves complete exact installation, and exposes the installed package to runtime activation without interpreting domain bodies.

This specification resolves the requirements from [PDS Router Assessment By Domain](pds_router_domain_assessment.md). It does not define dependency-security or docs component bodies. Those are consumer specifications.

Runtime activation, quiescence, interruption, and recovery conform to [Runtime Lifecycle And Quiescence](../../cognitive_architecture/runtime_lifecycle_and_quiescence.md). The router remains the inert semantic attachment boundary within that lifecycle.

The [PDS Activation Lifecycle Fanout](pds_activation_lifecycle_fanout_exercise.md) applies that canonical contract across the existing runtime, supervisor, domain, and consumer boundaries.

The semantic ownership boundary is fixed in [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md). The router installs PDS meaning. Strategy problem assembly separately supplies activation capabilities, admitted Methods, construction policy, authority context, projection requests, and search controls. Existing package-carried capability and authority components below describe migration compatibility, not the canonical semantic package shape.

This document records a candidate implementation design above the active theory-to-runtime contract. Exact owner installation and resolution are grounded in implemented primitives. Package imports, generic assignments, activation receipts, transport placement, and isolation requirement encoding remain proposed until the promotion evidence in the PDS proposal corpus is satisfied.

## Next Iteration Consumer Boundary

The next implementation iteration has two delivery consumers:

- [docs freshness](../../persistent_domain_stewardship/examples/documentation_freshness/README.md), exercised through its [router specification](../../persistent_domain_stewardship/examples/documentation_freshness/router_spec.md) and [refactor design](docs_freshness_pds_router_refactor_design_spec.md)
- [dependency security](../../persistent_domain_stewardship/examples/dependency_security/README.md), exercised through its [router specification](../../persistent_domain_stewardship/examples/dependency_security/router_spec.md) and [consumer design](dependency_security_pds_consumer_design_spec.md)

These two consumers are the delivery gate for the common router, assignment, activation, capability, admission, isolation, and lifecycle contracts in this specification. A common mechanism enters the next iteration only when one or both consumers exercise it or when it is necessary to keep their implementation domain-open.

The wider [PDS example router corpus](../../persistent_domain_stewardship/examples/README.md) is architectural pressure evidence. It may strengthen a rejection criterion, show that an owner extension point must remain open, or falsify a supposedly generic field. It does not by itself require a new common route, manifest field, runtime service, store, lifecycle state, or owner in the next iteration.

This boundary keeps future PDS creation package driven. A later roleplay, lore, learning, portfolio, faction, reliability, or maintenance steward should normally arrive as a dedicated package plus domain-owned routes and activation contributions. It should not require root branches or retroactive application fields in the common manifest.

The next iteration therefore does not implement a universal perspective projection route, evidence-mapping route, disclosure-policy route, graph-projection route, or context-hydration route. Those remain candidate owner contracts. Current envelopes and public owner boundaries must avoid preventing them.

## Use-Case Trace Rule

Every current-iteration router requirement and acceptance criterion must trace to docs freshness, dependency security, or a generic closure property directly needed to install and activate those packages.

Wider examples use this classification:

| Classification | Effect on the next iteration |
| --- | --- |
| delivery requirement | exercised by docs freshness or dependency security and included in implementation scope |
| extension-shape evidence | protects an owner boundary or generic envelope without implementing the wider feature |
| deferred use-case theory | remains in its example package and router specification until that domain becomes a delivery consumer |

An extension-shape finding may prevent a closed enum or application-specific root branch. It may not add speculative executable behavior.

### Current Iteration Change Ledger

| Common change | Docs freshness trace | Dependency-security trace | Scope treatment |
| --- | --- | --- | --- |
| route catalog, owner dispatch, exact package receipt | `DF-R11`, `DF-R12`, `DF-R21` | `DS-R01`, `DS-R13` | implement once and prove with both packages |
| assignment separate from physical activation | `DF-R14` | `DS-R14`, `DS-R15` | implement common envelopes with owner-scoped bindings |
| exact capability contribution and selection | `DF-R04`, `DF-R05`, `DF-R18` | `DS-R23`, `DS-R24` | implement common contributor and activation-local catalogs |
| owner result admission and evidence non-substitution | `DF-R07`, `DF-R24` | `DS-R05`, `DS-R08`, `DS-R11`, `DS-R18` | implement common lineage carriers while owners retain meaning |
| optional provider and placement-neutral activation | `DF-R09` | `DS-R12`, `DS-R14` | prove deterministic local and external realizations without changing package identity |
| passive source-delivery lineage | none | `DS-R34`, `DS-R37` | implement for the security monitor without fabricating execution attempts |
| activation generation, participant incarnation, and durable operation lineage | `DF-R36` through `DF-R39` | `DS-R33`, `DS-R41` through `DS-R45` | implement as shared runtime mechanics exercised by both consumers |
| readiness, wake, quiescence, and interrupted recovery | `DF-R33` through `DF-R40` | `DS-R40` through `DS-R46` | implement only the common lifecycle mechanics required by these participant plans |
| perspective projection, disclosure, graph hydration, and generic evidence mapping | none | none | retain extension seams and defer implementation |
| shared observation deduplication and universal approval workflow | none | none | defer without adding common runtime behavior |

## Architectural Decision

The top-level code domain is `theory`. The attachment behavior is `theory::router`. PDS is the architectural package and stewardship program, not a generic runtime owner.

The public architectural name is PDS Router.

```text
PDS package
→ theory router
→ domain route handlers
→ exact owner revisions
→ installed package receipt
→ assignment
→ physical activation
→ Meld runtime
```

The router is analogous to a public API router only at the control boundary. It owns addresses, dispatch, closure, receipts, and diagnostics. Each destination domain owns payload meaning, validation, persistence, exact resolution, and runtime use.

The product attachment façade may compose source loading, the router, receipt persistence, assignment selection, and activation. `theory::router` itself remains the narrow route lookup, structural closure, and owner-dispatch behavior.

```text
package source adapter
→ materialized content-addressed package
→ theory router
→ theory receipt store
→ runtime activation service
→ assignment-local runtime participants
```

## Governing Invariants

1. One package has one manifest entry point.
2. One package may contain many components.
3. Every component names exactly one published theory route.
4. The route owner alone interprets the component body.
5. Package installation is state-free theory installation, not runtime activation.
6. Owner installs are append-only, exact, and idempotent.
7. The package receipt is committed after every owner revision resolves exactly.
8. Package identity excludes workspace, credentials, provider, endpoint, process, and transport placement.
9. Assignment identity is separate from package identity.
10. Physical activation is separate from assignment and package identity.
11. Capability availability and activation selection are separate from PDS theory and effective authority.
12. Root code never switches on package id, expression name, route id, or owner domain.
13. Several installed packages and several active assignments are valid.
14. Router success proves installation closure only. It does not prove a maintained condition.
15. Runtime placement does not change package identity or confer semantic authority.
16. Every active realization is assignment scoped and activation-generation fenced.
17. External results remain untrusted until the destination domain admits them.
18. Shared runtime machinery never merges principal grants, bindings, selected capabilities, or runtime state.

## Terminology

| Term | Meaning | Owner |
| --- | --- | --- |
| package entry | manifest path or equivalent source handle supplied by the caller | caller and `theory` adapter |
| package manifest | common package identity, component index, and exact imports | `theory` |
| routed component | common envelope containing route identity and an intact owner body | `theory` envelope, destination domain body |
| theory route | published address and version contract for one owner component kind | destination domain |
| route handler | owner adapter that validates, installs, and verifies its body | destination domain |
| package receipt | exact package identity plus sorted exact owner revision refs | `theory` |
| assignment | standing use of one package receipt by one principal over one scope | root configuration and Agent activation boundary |
| activation | physical bindings and executable implementations for one assignment | configuration plus owning adapters |
| activation generation | assignment-wide live-authority fence over exact bindings, selected implementations, and isolation realization | runtime composition plus owner receipts |
| participant incarnation | one operational ownership lifetime for an actor, process, client, source, or adapter under an activation generation | supervisor plus owning adapter |
| durable operation | one semantic capability unit preserved across retries and participant replacement | execution |
| attempt | one dispatch of a durable operation through one participant incarnation | execution plus owning adapter |
| passive delivery | one authenticated source notification under an admitted subscription and participant incarnation | source owner plus destination domain |
| capability contributor | domain publisher of executable contracts and invoker factories | capability-owning domain |

## Source Package Contract

The common source model is intentionally small.

```rust
struct PdsPackageManifest {
    schema_version: u32,
    package_id: String,
    package_version: String,
    description: Option<String>,
    imports: Vec<ExactPackageImport>,
    components: Vec<PdsComponentEntry>,
}

struct ExactPackageImport {
    package_id: String,
    receipt_id: String,
}

struct PdsComponentEntry {
    component_id: String,
    route: TheoryRouteId,
    component_schema_version: u32,
    content: ComponentContentRef,
    requires: Vec<ComponentRequirement>,
}

enum ComponentContentRef {
    Embedded { canonical_bytes: Vec<u8> },
    RelativeFile { path: String, content_hash: String },
    PublishedExact {
        publisher: String,
        id: String,
        content_identity: String,
    },
}

struct ComponentRequirement {
    package: PackageRequirementTarget,
    component_id: String,
    required_route: Option<TheoryRouteId>,
}

enum PackageRequirementTarget {
    SelfPackage,
    ExactImport { package_id: String, receipt_id: String },
}
```

`RelativeFile` paths are resolved relative to the manifest directory. Absolute paths, parent traversal, symlink escape, and mutable network references are rejected. `PublishedExact` names canonical semantic content already published by an owning domain. Existing docs and dependency-security migrations also use it for exact capability contracts as a compatibility lowering. Canonical activation obtains exact capability offers from contributors and selects them outside package semantics. A later transport may provide the same canonical semantic bytes through another source adapter without changing the package contract.

The manifest does not contain a universal belief section, Agent section, Strategy section, docs section, or security section. Those are routed component bodies.

## Package Identity

The canonical package content identity is derived from:

```text
manifest schema version
package id
package version
sorted exact import receipt ids
sorted component ids
each component route
each component schema version
each canonical component content hash or exact published identity
sorted structural component requirements
```

Description text does not participate in identity for manifest schema version one. A future schema may make descriptive material semantic, but host policy cannot vary identity for the same manifest schema. Operational installation sequence never participates.

Canonicalization rejects duplicate package ids within one install request, duplicate component ids, unsorted duplicate requirements, unknown manifest fields when strict mode is selected, and any body whose observed hash differs from the declared content hash.

## Theory Route Contract

Routes use owner-qualified stable identity.

```rust
struct TheoryRouteId {
    owner_domain: String,
    component_kind: String,
    route_version: u32,
}

struct TheoryRouteContract {
    route: TheoryRouteId,
    accepted_component_schema: VersionRange,
    package_cardinality: RouteCardinality,
    handler_contract_version: u32,
}

enum RouteCardinality {
    AtMostOne,
    Many,
}
```

Route cardinality constrains occurrences within a package only when the route is used. It never makes a route mandatory for every package. Structural component requirements and owner semantic validation establish which components a specific package needs.

Illustrative serialized routes:

```text
world-model.belief-family.v1
world-model.outcome-mapping.v1
world-model.agent-curation-rule.v1
world-model.agent-maintained-condition.v1
world-model.strategy-theory.v1
execution.capability-contract.v1  compatibility migration route
execution.authority-policy.v1  compatibility migration route
docs.claim-policy.v1
dependency-security.policy.v1
```

The route version describes the handler contract. The component schema version describes the owner body. They evolve independently.

## Route Handler Boundary

Every route handler is constructed by its owning domain with narrow domain installation and resolution ports.

```rust
trait TheoryRouteHandler {
    fn contract(&self) -> TheoryRouteContract;

    fn validate(
        &self,
        source: &RoutedComponentSource,
        package: &PackageLinkView,
    ) -> Result<OwnerValidationToken, OwnerRouteDiagnostic>;

    fn install(
        &self,
        source: &RoutedComponentSource,
        validation: OwnerValidationToken,
        installed_at_seq: u64,
    ) -> Result<InstalledTheoryComponentRef, OwnerRouteDiagnostic>;

    fn verify(
        &self,
        reference: &InstalledTheoryComponentRef,
    ) -> Result<(), OwnerRouteDiagnostic>;

    fn validate_links(
        &self,
        reference: &InstalledTheoryComponentRef,
        package: &InstalledPackageLinkView,
    ) -> Result<(), OwnerRouteDiagnostic>;
}
```

`OwnerValidationToken` is an opaque non-durable owner product carrying the route, component id, source hash, package content identity, handler contract version, and catalog assembly identity. The router carries it intact between validation and installation. It may not inspect the token or copy owner fields beside it. The owner verifies that the token still matches the supplied source, package graph, and handler instance before installation. A token cannot be replayed against another package, catalog, handler instance, or process lifetime.

`PackageLinkView` exposes package identity, route-addressed component metadata, structural requirements, and exact import receipts. It does not expose foreign raw bodies to handlers. `InstalledPackageLinkView` exposes route-addressed exact owner refs after installation. A handler that needs foreign meaning receives an explicit public query contract from that foreign domain when the handler is constructed.

Handlers never receive `OpenProductStores`. They receive only owner registry commands and resolvers captured during construction.

## Route Catalog

`TheoryRouteCatalog` is an immutable installation-time registry assembled before package input is read.

It must:

- reject duplicate route identity
- reject conflicting owner identity
- return routes in deterministic order
- expose contracts without exposing handler internals
- resolve exactly one handler for every component
- remain independent of installed package and expression identity

Route discovery is product composition, not package-controlled dynamic loading. The first implementation registers compiled domain handlers. A later isolated adapter may publish the same route contract through a transport, but that is not part of this design.

Transport never grants route ownership. A trusted compiled owner proxy publishes the route, authenticates any remote implementation, and remains responsible for validation, exact owner installation, diagnostics, and result admission. A package or endpoint cannot mint owner identity by repeating an owner string.

Any future transport-backed install uses a deterministic operation id derived from package identity, component id, route, and source hash. A timeout is an unknown install result. Retry under the same operation id returns the same exact owner ref or a conflict diagnostic.

## Structural And Semantic Linking

Linking has two bounded layers.

### Structural linking owned by theory

The router validates:

- exact package imports exist and match receipt identity
- component ids are unique within a package
- every route exists
- every route accepts the component schema version
- route occurrence cardinality is satisfied
- every component requirement resolves to one component
- every required route identity matches the target component
- component reference cycles are either rejected or explicitly allowed by a future manifest version
- package and component content hashes are deterministic

The first design rejects component requirement cycles.

Exact imports form a frozen package graph. Requirements may address the current package or one declared direct import. The router keys imported components by exact receipt id and never consults an imported package head. Transitive import traversal is available only through the exact imported receipt graph. It does not flatten imported components into local package identity.

### Semantic linking owned by destination domains

Each handler validates the meaning of its own body before installation. After every component is installed and exactly resolvable, handlers validate semantic links through installed refs and explicit cross-domain public query contracts. Examples include a belief family matching an outcome mapping, Strategy action and outcome semantics matching their referenced vocabulary, or a docs policy id matching its body.

For the first compatibility consumer, every selected capability contract appears as an explicit `execution.capability-contract.v1` component using `PublishedExact`. This preserves characterized docs behavior during migration. It does not define the canonical boundary. The destination design removes those requirements from Strategy theory, obtains the exact catalog through activation contribution, and validates candidate compatibility when Strategy assembles the current problem.

No root cross-domain validator compares dimensions, policy ids, advisory semantics, capability effects, or Agent thresholds. A handler that depends on a foreign public contract consumes that contract through the owning domain public surface.

## Installation Transaction

Installation is a retry-safe saga with one final visibility commit.

```text
load one package entry
→ canonicalize manifest and component bytes
→ resolve exact imports
→ resolve every route
→ structural link validation
→ owner validation for every component
→ owner installation in deterministic route and component order
→ owner exact-resolution verification
→ owner semantic link validation through public contracts
→ package receipt construction
→ append package receipt
→ advance selected package head when requested
```

Local owner validation must complete for every component before the first owner write. Cross-domain semantic link validation runs after exact owner installation because it consumes installed public refs. Owner installation may leave exact append-only revisions behind if a later owner or link check fails. Those revisions are not selectable as an installed package until the package receipt commits. Retry converges through idempotent owner installation.

Package visibility is receipt scoped. Runtime package resolution never consults owner-global current heads. A route handler either appends an exact revision without advancing an owner-global selection, or proves that such advancement cannot affect package-scoped consumers. Owner registries that cannot satisfy either condition require a revision-only prepare port before they can participate in generic routing.

The router does not promise cross-store rollback. Package-level atomic visibility comes from committing the package receipt last. This does not claim that every present owner registry already supports an invisible prepared revision.

## Installed Component Reference

The common receipt carries route-addressed owner refs rather than opaque JSON and rather than an enum of every domain kind.

```rust
struct InstalledTheoryComponentRef {
    component_id: String,
    route: TheoryRouteId,
    component_schema_version: u32,
    source_content_hash: String,
    owner_revision: TheoryRevisionRef,
}

struct TheoryRevisionRef {
    registry: String,
    id: String,
    content_hash: String,
}
```

The router validates structural completeness of the reference. The route handler validates that `owner_revision` belongs to its registry and resolves to the installed canonical body.

`source_content_hash` records the exact supplied component bytes. `owner_revision.content_hash` records the owner canonical semantic body. They may differ when an owner performs a declared canonicalization. Neither may be silently substituted for the other.

## Package Installation Receipt

```rust
struct PdsPackageInstallationReceipt {
    receipt_id: String,
    package_id: String,
    package_version: String,
    package_content_hash: String,
    manifest_schema_version: u32,
    exact_imports: Vec<InstalledExactPackageImport>,
    components: Vec<InstalledTheoryComponentRef>,
    installed_at_seq: u64,
}

struct InstalledExactPackageImport {
    package_id: String,
    receipt_id: String,
}
```

Receipt identity includes every field except `receipt_id` and `installed_at_seq`. Components are sorted by route identity and component id. Exact imports retain both package and receipt identity and are sorted by package id and receipt id.

Receipt installation must reject:

- an empty component set
- a component absent from the source manifest
- an extra installed component
- a source hash mismatch
- a route mismatch
- a duplicate component id
- an unresolved owner revision
- an unresolved exact import
- a computed receipt identity mismatch

The current fixed `TheoryInstallationReceipt` becomes a compatibility input. The canonical target is the generic receipt above.

## Exact Resolution

Runtime resolution returns a package-level immutable view, not one monolithic cross-domain body.

```rust
struct ResolvedPdsPackage {
    receipt: PdsPackageInstallationReceipt,
    components_by_route: BTreeMap<TheoryRouteId, Vec<InstalledTheoryComponentRef>>,
    exact_imports: BTreeMap<String, ResolvedImportedPdsPackage>,
}
```

Resolution performs these checks once for one composition lifetime:

1. Resolve the selected receipt by exact id.
2. Verify receipt identity.
3. Resolve every exact import receipt and verify its retained package id.
4. Ask the matching route handler to verify every owner revision.
5. Freeze the sorted component refs in an immutable package view.

The root never receives decoded owner bodies. Runtime domain factories request refs for routes they own and resolve bodies through their existing registries.

Historical resolution always uses the receipt id carried in lineage. A current package head is only a selection convenience and is never consulted to interpret historical work.

## Assignment Contract

Installing a package does not make it active.

```rust
struct StewardshipAssignment {
    assignment_id: String,
    package_receipt_id: String,
    principal_id: String,
    agent_id: String,
    subject: DomainObjectRef,
    perspective_id: String,
    branch_id: String,
    requested_authority_ref: String,
    principal_grant_ref: String,
    lifecycle: AssignmentLifecycle,
}
```

Assignment identity derives from the exact package receipt and standing semantic bindings, including the exact requested-authority and principal-grant refs. It does not include credentials or process placement. Current runtime policy and restrictions remain independently resolved and execution revalidates their intersection at dispatch.

More than one assignment may target the same workspace or subject. Configuration resolution returns a deterministic collection instead of treating the target as singular. Conflict policy is deferred and no assignment gains priority from declaration order.

## Physical Activation Contract

```rust
struct StewardshipActivation {
    activation_id: String,
    assignment_id: String,
    bindings: BTreeMap<String, PhysicalBindingRef>,
    placement: AdapterPlacement,
    isolation_requirements: RuntimeIsolationRequirements,
    capability_selection: CapabilitySelectionRef,
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

The common activation envelope names binding refs but does not open credentials, connect endpoints, or interpret owner configuration. Route-owning activation adapters declare required binding ids and consume only their own resolved values.

Activation fails closed when any activation-selected capability implementation or owner activation requirement is unavailable. A package may install successfully on a host that cannot activate it.

Placement is operational metadata. Local library, subprocess, sidecar, and remote service placements must preserve the same domain result contract if later supported.

Placement is not an isolation claim. The activation requirement set states sharing, state, failure, secret, filesystem, network, resource, effect, and late-result constraints independently of topology. Runtime composition fails closed when the selected realization cannot satisfy those constraints.

Satisfaction records the required property, realization mode, enforcing authority, verification evidence, and activation decision per axis. A cooperative or self-asserted adapter claim cannot satisfy a required hard boundary. Semantic effect targets do not substitute for physical filesystem grants, and logical work budgets do not claim hard compute or memory enforcement.

See [PDS Isolation And Runtime Portability](../../persistent_domain_stewardship/isolation_and_runtime_portability.md) for the proposal-level requirement model and unresolved encoding choices.

## Placement-Independent Runtime Isolation

Every activation is assignment scoped over one exact package receipt. Its runtime realization receives only:

- the owner-scoped package view required by its contributor
- explicitly granted owner bindings
- selected exact capability refs
- exact assignment and effective-authority decision refs
- a bounded resource and time budget

It receives no ambient foreign stores, product-wide credential set, or another activation's capability registry.

Each request-response attempt additionally receives activation generation, participant incarnation, stable operation, and distinct attempt context. Each passive source adapter receives activation generation, participant incarnation, admitted subscription, delivery, and source-cursor context.

In-process placement provides contract isolation but is not a hostile-code security boundary. Subprocess, sidecar, and remote placement may strengthen failure or trust containment. Every placement preserves the same exact inputs, idempotency key, diagnostic envelope, owner result-admission contract, and lineage obligations.

Authority, binding, selected implementation, isolation realization, endpoint, credential, or activation replacement changes create a new activation generation. A mechanically reconstructed equivalent participant creates a new participant incarnation under the same activation generation after recovery readiness closes. It does not invalidate unrelated participants.

A result from an older activation generation or retired participant incarnation retains that identity. The destination domain rejects it, admits it as historical-only, or admits it under an explicit late-result policy. It never silently relabels the result as current-generation or current-incarnation output.

The execution lifecycle epoch anchors the stable operation key. The dispatch claim fences one attempt and may change across worker replacement. A process id, HTTP request id, or adapter-local retry counter replaces neither identity.

## Owner Activation Contribution

Installation handlers and activation contributors are separate contracts.

```rust
trait TheoryActivationContributor {
    fn owner_domain(&self) -> &str;

    fn requirements(
        &self,
        package: &OwnerPackageView,
        assignment: &StewardshipAssignment,
    ) -> Result<Vec<ActivationBindingRequirement>, ActivationDiagnostic>;

    fn prepare(
        &self,
        package: &OwnerPackageView,
        assignment: &StewardshipAssignment,
        bindings: &OwnerActivationBindings,
    ) -> Result<PreparedDomainActivation, ActivationDiagnostic>;
}
```

`OwnerPackageView` contains only routes owned by the contributor, explicit structural dependencies declared by those components, and narrow foreign-domain query handles required by owner validation. It does not expose the complete raw package graph.

Each contributor is constructed with narrow owner activation commands. Root resolves only the binding ids returned by that contributor and passes an owner-scoped binding set. There is no universal mutable activation builder and no contributor receives foreign bindings or unrestricted stores.

Preparation may append idempotent owner records keyed by activation id, but it does not publish invokers into a live catalog. Runtime assembly builds an assignment-local draft catalog and executor registry, verifies every prepared owner contribution and exact invoker closure, then commits one aggregate prepared-activation receipt last. That receipt proves complete inert preparation. It does not prove readiness or make the activation active.

```rust
struct PreparedStewardshipActivationReceipt {
    prepared_receipt_id: String,
    activation_id: String,
    assignment_id: String,
    package_receipt_id: String,
    owner_receipts: Vec<DomainActivationReceipt>,
    participant_plan: Vec<ActivationParticipantPlan>,
    selected_capability_refs: Vec<String>,
    binding_set_hash: String,
    isolation_realization_hash: String,
    prepared_at_seq: u64,
}

struct ActivationParticipantPlan {
    participant_id: String,
    owner_domain: String,
    required: bool,
    mode: ActivationParticipantMode,
    readiness_dependencies: Vec<String>,
    readiness_contract_ref: String,
    wake_contract_refs: Vec<String>,
    safe_point_contract_ref: String,
    stop_contract_ref: String,
    runtime_realization_ref: String,
}

enum ActivationParticipantMode {
    BoundedActor,
    OperationAdapter,
    PassiveSource,
}
```

Domain activation receipts prove that exact owner components were prepared against an exact assignment, non-secret binding identity set, contributor identity, and implementation version. They do not duplicate component bodies, include secret material, or prove runtime success.

Failure, timeout, or retry before the aggregate prepared commit leaves no complete prepared activation and no live authority. Append-only owner preparation records may remain and exact retry converges.

After prepared closure, runtime composition creates one generation with admission closed, starts required participant incarnations, and collects owner and adapter readiness receipts. Successful construction, process spawn, or network connection alone does not prove readiness.

```rust
struct ActiveStewardshipGenerationReceipt {
    activation_generation_id: String,
    activation_id: String,
    prepared_receipt_id: String,
    expected_prior_generation: Option<String>,
    participant_incarnation_refs: Vec<String>,
    readiness_receipts: Vec<DomainReadinessReceipt>,
    published_at_seq: u64,
}
```

Runtime composition publishes the generation as current only after every required readiness receipt resolves and the assignment current-generation pointer still matches `expected_prior_generation`. The conditional publication is the live-authority boundary. A ready but non-current generation has no authority to accept new claims or publish current results.

A failure after prepared closure but before current publication may leave inert preparation records, participant incarnation receipts, or stopped handles. Recovery resumes or retires that generation without exposing its draft catalog. Deactivation first closes admission for the exact current generation, then retires handles and owner state through their narrow lifecycle ports. It cannot revoke or mutate another assignment.

## Lifecycle Participant Contracts

Activation lifecycle uses narrow owner ports rather than one universal lifecycle contributor. A participant implements only the contracts its mode requires.

```rust
trait DomainReadinessPort {
    fn readiness(
        &self,
        generation: &ActivationGenerationRef,
        incarnation: &ParticipantIncarnationRef,
    ) -> Result<DomainReadinessReceipt, ActivationDiagnostic>;
}

trait DomainWakeResolutionPort {
    fn resolve_wait(
        &self,
        declaration: &DomainWaitingOnDeclaration,
    ) -> Result<Vec<StructuralWakeRef>, ActivationDiagnostic>;
}

trait DomainSafePointPort {
    fn close_admission(
        &self,
        generation: &ActivationGenerationRef,
    ) -> Result<AdmissionFenceReceipt, ActivationDiagnostic>;

    fn safe_point(
        &self,
        generation: &ActivationGenerationRef,
    ) -> Result<DomainSafePointReceipt, ActivationDiagnostic>;
}

trait RuntimeParticipantStopPort {
    fn stop(
        &self,
        incarnation: &ParticipantIncarnationRef,
        mode: StopMode,
    ) -> Result<ParticipantStopReceipt, ActivationDiagnostic>;
}
```

Each domain owns readiness, waiting, safe-point, and recovery meaning. Runtime composition verifies exact receipt closure and structural wake viability without interpreting owner conditions. A missing required participant, missing wait declaration, or absent wake path cannot project as quiescent.

Operational lifecycle dependencies govern readiness and reverse drain order only. They never encode observation, belief, Agent, Strategy, or Steward cooperation.

## Quiescence And Recovery

An active assignment is quiescent only when every required participant is present or durably represented, no work is eligible, and every owner-declared wait resolves to a viable wake path. A blocked condition qualifies only with a durable escalation or operator wake path. A clean zero-work tick proves only active idle.

Fenced quiescence first closes admission for one exact activation generation. Accepted work then settles, reconciles, or becomes durably unresolved. Runtime composition commits one aggregate receipt over exact owner safe points, execution unresolved-operation summary, passive-subscription fences, and participant drain receipts. The receipt does not claim a simultaneous cross-store transaction. Retirement later adds participant stop and lease-release receipts.

Recovery replays the ordinary activation protocol from durable intent, prepared closure, participant plan, owner checkpoints, operations, subscriptions, and receipts. Admission remains closed until every required reconstructed participant proves recovery readiness. An external process or service may assist reconciliation but may not be the sole source of program intent, progress, or uncertainty.

The stable role-based supervisor starts and monitors one activation-lifecycle service. That service coordinates structural assignment-local lifecycle phases. It does not own foreign cursors, domain retry policy, result admission, or cognitive sequencing.

## Capability Contribution Pattern

Theory routes and executable capability contribution are parallel but separate.

```rust
trait ProductCapabilityContributor {
    fn owning_domain(&self) -> &str;
    fn published_contracts(&self) -> Vec<CapabilityTypeContract>;
    fn bind_exact(
        &self,
        selected: &[CapabilityContractRevisionRef],
        assignment: &StewardshipAssignment,
        bindings: &OwnerActivationBindings,
        catalog: &mut ActivationLocalCapabilityCatalog,
        registry: &mut ActivationLocalExecutorRegistry,
    ) -> Result<usize, CapabilityActivationDiagnostic>;
}
```

The product capability inventory collects contracts from every compiled contributor and rejects duplicate type and version identity. Package installation may select and install exact contracts from this inventory. Activation asks every contributor to bind only matching exact selectors into draft values scoped to one activation.

Canonical capability contracts and physical implementation offers are separate catalogs. Product or administrator composition may publish zero or more offers for one exact contract. Activation selects exactly one offer for each selected contract into its local executor view. Registration changes create a new catalog generation for later activations and never mutate an active or historical view.

A package selects semantic contracts. It cannot supply a native library path, executable path, endpoint, or owner identity. Dynamic implementation registration remains trusted product or administrator behavior.

After all contributors run, runtime assembly verifies that every selected executable contract has exactly one invoker. An available invoker that is not selected remains unavailable to that assignment.

The contributor is selected by contract ownership and exact selector matching, never by package or expression identity.

## Capability And Authority Closure

Capability closure requires all of the following:

```text
domain publishes capability contract
package Strategy or action theory selects exact contract
package receipt pins exact contract revision
activation supplies required physical bindings
domain contributor binds exact invoker
assignment authority requests the action class
principal and runtime policy grant the action class
execution revalidates effective authority
```

No single step substitutes for another. Package installation does not grant authority. Product implementation availability does not add an action to Strategy. Requested authority does not prove an implementation exists.

## Invocation And Result Admission

Every capability invocation against an external or fallible adapter carries:

```text
assignment id
+ package receipt id
+ activation id and generation
+ participant id and incarnation id
+ exact capability contract ref
+ durable operation key derived from semantic task unit and lifecycle epoch
+ dispatch claim and unique attempt id
```

The owning adapter derives any transport request or process attempt from this context. A timeout after dispatch is an unknown result, not proof that no external effect occurred. Retry preserves the durable operation key under a new attempt and participant incarnation, or first performs an explicit reconciliation observation. An activation-input change additionally advances activation generation. Dispatch claim identity includes worker lineage and cannot by itself serve as a cross-worker external idempotency key.

Long-running external work never holds one supervisor step open until completion. A bounded dispatch step persists the durable operation and attempt before handing work to the adapter. A later bounded poll or passive delivery resolves the attempt through owner admission. Short synchronous capabilities may continue to complete within one bounded step.

A passive callback or source-advance notification does not fabricate an execution claim. It instead carries assignment subscription, activation generation, participant incarnation, adapter delivery identity, source cursor or revision, and authentication lineage through the sensory admission path. The destination domain still performs exact scope and result admission.

External bytes are never appended directly as canonical facts. The destination domain validates subject scope, policy and source revision, completeness, evidence class, adapter identity, and operation plus attempt or passive-delivery lineage before constructing a canonical product. Transport status remains operational evidence.

An external transport cannot publish a route or result under an owner string by assertion. The trusted owner proxy authenticates the implementation and remains the admission authority.

## Multiple Package Behavior

The router must support:

- several installed revisions of one package id
- several different installed package ids
- several assignments over one package receipt
- several assignments over one subject
- several packages contributing the same route kind when the route cardinality is per package

Package receipts are independent. Imports create explicit exact dependency edges but do not merge package identity. Runtime composition groups assignments and package views deterministically by assignment id.

Each assignment receives a distinct capability catalog, executor registry, binding namespace, authority context, and activation generation. The same exact capability contract may bind different implementation instances for different assignments. Deactivation of one assignment does not remove another assignment's invoker.

The router does not decide whether two active assignments may write the same artifact. Domain policy and execution effect arbitration own that question.

Receipt installation and current-head selection are separate outcomes. Head movement uses an expected prior receipt or reports a selection conflict. A lost head-selection race does not invalidate the newly installed exact receipt. Assignment, activation, and historical replay always cite exact receipt ids.

## Persistence And Workspace Purity

Theory persistence remains under the product external storage root.

Conceptual layout:

```text
<product-storage>/theory/
├── package_receipts
├── package_heads
├── package_manifests
└── owner registries remain in their owning stores
```

The router stores the canonical manifest record, exact import refs, component source hashes, owner refs, and receipt. It does not copy owner canonical bodies out of owner registries.

No package installation, assignment, activation, or adapter runtime state may be written beneath the target workspace. A PDS package source may be read from a workspace only when explicitly supplied by the user. Reading package source does not authorize runtime writes beside it.

## Diagnostics And Failure Contract

Stable top-level router failure codes:

| Code | Meaning |
| --- | --- |
| `pds_package_invalid` | common manifest or content identity is invalid |
| `pds_import_unresolved` | an exact imported receipt is absent or corrupt |
| `pds_route_unavailable` | no handler publishes the named route |
| `pds_route_conflict` | product composition published duplicate route identity |
| `pds_component_invalid` | an owner rejected its component body |
| `pds_component_requirement_missing` | structural component closure is incomplete |
| `pds_owner_install_failed` | owner installation did not complete |
| `pds_owner_revision_missing` | installed owner ref does not resolve |
| `pds_owner_revision_corrupt` | owner ref resolves to inconsistent content |
| `pds_receipt_incomplete` | installed refs do not close the manifest |
| `pds_receipt_corrupt` | receipt identity does not match content |
| `pds_activation_binding_missing` | a required physical binding is absent |
| `pds_capability_implementation_missing` | selected exact contract has no invoker |
| `pds_activation_incomplete` | one owner activation contribution failed |
| `pds_activation_isolation_unsatisfied` | selected placement cannot satisfy activation isolation requirements |
| `pds_activation_readiness_failed` | a required participant did not prove generation-scoped readiness |
| `pds_activation_generation_conflict` | current generation changed from the expected prior value before publication |
| `pds_activation_generation_stale` | invocation or result cites a retired activation generation |
| `pds_participant_incarnation_stale` | result cites a retired participant incarnation |
| `pds_result_lineage_mismatch` | owner result admission rejected assignment or invocation lineage |
| `pds_head_selection_conflict` | current package head changed from the expected prior receipt |

Every diagnostic carries package id when known, component id when known, route identity when known, owner diagnostic code, source location, and a human message. The common error does not parse or rewrite the owner message.

## Input Safety

Package input is untrusted data.

The first implementation must enforce:

- manifest and component byte limits
- component count and dependency depth limits
- strict relative path containment
- content hash verification before decoding
- deterministic parsing with duplicate-key rejection where supported
- no native library loading
- no command execution during validation or installation
- no credential material in package receipts or diagnostics
- no network fetch through a component content ref
- no ambient environment, workspace, store, credential, or network access for route handlers

External program execution belongs to physical activation and capability invocation under effective authority.

## Requirements By Domain

### Theory

| Requirement | Contract |
| --- | --- |
| `PDS-R01` | own the common manifest and component envelope |
| `PDS-R02` | expose one package entry loader |
| `PDS-R03` | own deterministic package content identity |
| `PDS-R04` | own route catalog uniqueness and lookup |
| `PDS-R05` | validate structural component and exact import closure |
| `PDS-R06` | dispatch intact bodies without semantic parsing |
| `PDS-R07` | commit generic package receipts after owner verification |
| `PDS-R08` | resolve current and historical package receipts exactly |
| `PDS-R09` | support more than one installed package and revision |
| `PDS-R10` | keep owner partial installs unselectable through package resolution until receipt commit |

### Route-owning domains

| Requirement | Contract |
| --- | --- |
| `PDS-R11` | publish unique route contracts |
| `PDS-R12` | own body schema and semantic validation |
| `PDS-R13` | install through append-only owner registries |
| `PDS-R14` | return route-addressed exact revision refs |
| `PDS-R15` | verify historical exact resolution |
| `PDS-R16` | publish narrow activation requirements and behavior when needed |

### Capability and execution

| Requirement | Contract |
| --- | --- |
| `PDS-R17` | aggregate domain capability publishers into product inventory |
| `PDS-R18` | reject duplicate contract type and version identity |
| `PDS-R19` | construct an activation-local exact capability snapshot from product contributions and activation selection |
| `PDS-R20` | bind only exact selected invokers during activation |
| `PDS-R21` | reject activation unless every selected contract has one invoker |
| `PDS-R22` | preserve effective authority calculation and dispatch enforcement |

### Config, init, and runtime

| Requirement | Contract |
| --- | --- |
| `PDS-R23` | config selects exact package receipts rather than fixed component slots |
| `PDS-R24` | assignment and physical activation remain separate values |
| `PDS-R25` | target resolution returns several valid assignments deterministically |
| `PDS-R26` | init invokes theory router rather than owner loaders directly |
| `PDS-R27` | runtime freezes exact package views for composition lifetime |
| `PDS-R28` | root assembly contains no package, expression, route, or owner branches |
| `PDS-R29` | physical provider binding is required only by selected components or capabilities |
| `PDS-R30` | runtime state remains in existing domain stores and ledgers |

### Compatibility

| Requirement | Contract |
| --- | --- |
| `PDS-R31` | characterize current fixed docs receipt and activation behavior before migration |
| `PDS-R32` | provide one temporary lowering from current docs selection to a synthetic package entry |
| `PDS-R33` | read historical fixed receipts until their supported data window closes |
| `PDS-R34` | require parity before deleting direct docs paths |
| `PDS-R35` | mark every shim with an explicit removal condition |

### Runtime isolation and admission

| Requirement | Contract |
| --- | --- |
| `PDS-R36` | encode activation isolation requirements independently of placement |
| `PDS-R37` | build capability catalogs and executor registries per activation |
| `PDS-R38` | expose no complete prepared activation before aggregate prepared receipt commit and no live activation before readiness closure |
| `PDS-R39` | identify every live realization by activation generation, participant incarnation, and adapter identity |
| `PDS-R40` | keep durable operation key, unique attempt id, dispatch claim, activation generation, and participant incarnation as separate external-work lineage |
| `PDS-R41` | preserve assignment, package, activation, participant, capability, operation, attempt, and passive-delivery lineage through owner admission |
| `PDS-R42` | require capability-specific idempotency or reconciliation after ambiguous effects |
| `PDS-R43` | classify late results from retired generations without silent current admission |
| `PDS-R44` | prevent shared runtime machinery from sharing grants, bindings, catalogs, or runtime state implicitly |
| `PDS-R45` | make deactivation assignment local and generation fenced |
| `PDS-R46` | separate exact receipt installation from conditional current-head selection |
| `PDS-R47` | authenticate remote owner implementations through trusted compiled owner proxies |
| `PDS-R48` | publish one ready generation through an expected-prior-generation fence before opening admission |
| `PDS-R49` | include an exact required and optional participant plan in prepared activation closure |
| `PDS-R50` | separate assignment-wide activation generation from participant incarnation and operation attempt |
| `PDS-R51` | project active idle separately from quiescence and structural stall |
| `PDS-R52` | resolve owner waiting conditions to structural wake refs without root interpreting domain meaning |
| `PDS-R53` | persist long-running external operations before dispatch and complete them through later bounded admission |
| `PDS-R54` | keep admission closed through interrupted recovery until exact readiness closure |
| `PDS-R55` | aggregate owner safe-point receipts without claiming cross-store atomicity |
| `PDS-R56` | keep the activation-lifecycle service structural and free of foreign progress or retry policy |

## Requirement Trace

| Requirement source | Router requirements |
| --- | --- |
| `AR-01` | `PDS-R01`, `PDS-R02`, `PDS-R03` |
| `AR-02` | `PDS-R07`, `PDS-R08` |
| `AR-03` | `PDS-R04`, `PDS-R06`, `PDS-R11`, `PDS-R12` |
| `AR-04` | `PDS-R05`, `PDS-R12` |
| `AR-05` | `PDS-R23`, `PDS-R24`, `PDS-R29` |
| `AR-06` | `PDS-R09`, `PDS-R25` |
| `AR-07` | `PDS-R17` through `PDS-R21` |
| `AR-08` | `PDS-R16`, `PDS-R28` |
| `AR-09` | `PDS-R11` through `PDS-R15` |
| `AR-15` | `PDS-R10`, `PDS-R13` |
| `AR-16` | `PDS-R14`, `PDS-R15` |
| `AR-17` | `PDS-R12`, `PDS-R16`, `PDS-R28` |
| `AR-18` | `PDS-R01`, `PDS-R24`, `PDS-R30` |
| `AR-19` | `PDS-R36`, `PDS-R37`, `PDS-R44`, `PDS-R45` |
| `AR-20` | `PDS-R38`, `PDS-R39`, `PDS-R46`, `PDS-R48` |
| `AR-21` | `PDS-R40` through `PDS-R43`, `PDS-R47` |
| canonical runtime lifecycle and quiescence | `PDS-R49` through `PDS-R56` |

## Migration Contract

The canonical destination removes fixed cross-owner slots from package selection and receipts. Migration must preserve current behavior through compatibility wrappers.

Required compatibility shape:

```text
legacy docs declaration
→ characterized synthetic package manifest
→ canonical router installation
→ generic package receipt
→ canonical activation
```

Historical fixed receipts remain readable through a versioned receipt decoder. New installations write only generic receipts after the router path is enabled. New code must not add fields to `TheorySelection` or `TheoryInstallationReceipt` for dependency security.

The fixed docs loader and direct docs activation call may be removed only after docs parity criteria in the consumer refactor spec pass.

## Expected Domain-First Code Shape

The shape is illustrative and must preserve modern Rust module layout.

```text
src/theory.rs
src/theory/contracts.rs
src/theory/package.rs
src/theory/router.rs
src/theory/receipt.rs
src/theory/registry.rs
src/theory/resolution.rs
src/theory/activation.rs

src/docs/theory.rs
src/docs/theory/claim_policy.rs

src/dependency_security.rs
src/dependency_security/theory.rs

crates/meld-world-model/src/belief/theory.rs
crates/meld-world-model/src/agent/theory.rs
crates/meld-world-model/src/strategy/theory.rs

crates/meld-execution/src/capability/theory.rs
crates/meld-execution/src/authority/theory.rs
```

No `mod.rs` files are introduced.

## Verification And Acceptance Criteria

### Package and route behavior

- `PDS-AC01` one manifest installs components owned by at least three different domains
- `PDS-AC02` a missing route fails before owner writes
- `PDS-AC03` duplicate route publication fails product composition
- `PDS-AC04` duplicate component identity fails package validation
- `PDS-AC05` a component schema mismatch produces route-addressed diagnostics
- `PDS-AC06` a missing structural requirement fails before owner writes
- `PDS-AC07` owner semantic rejection does not commit a package receipt

### Durability and replay

- `PDS-AC08` exact reinstall returns the existing owner revisions and package receipt
- `PDS-AC09` failure after one owner install leaves no visible package head and retry converges
- `PDS-AC10` historical package receipt resolves after a newer revision is installed
- `PDS-AC11` owner revision deletion or corruption makes package resolution fail closed
- `PDS-AC12` package receipt identity is stable across component source file order
- `PDS-AC13` no theory or activation state is written under the target workspace

### Activation and capabilities

- `PDS-AC14` a package installs on a host lacking its physical adapter but activation fails clearly
- `PDS-AC15` every selected exact capability binds exactly one invoker
- `PDS-AC16` unselected product capabilities remain absent from the assignment catalog
- `PDS-AC17` provider-free deterministic package activation requires no provider binding
- `PDS-AC18` capability availability does not bypass effective authority denial

### Domain isolation

- `PDS-AC19` router source contains no docs or dependency-security body types
- `PDS-AC20` root assembly contains no dispatch on expression, package, route, or owner strings
- `PDS-AC21` `meld-lang` contains no docs or security variants
- `PDS-AC22` route handlers receive no unrestricted cross-domain store collection
- `PDS-AC23` canonical owner bodies travel intact through router wrappers

### Multiple package behavior

- `PDS-AC24` two independent package receipts install and resolve concurrently
- `PDS-AC25` two assignments may address one subject without declaration-order priority
- `PDS-AC26` exact imports retain separate package identities and resolve historically

### Compatibility

- `PDS-AC27` current docs package behavior is characterized before lowering changes
- `PDS-AC28` legacy docs declaration and canonical package entry resolve to equivalent generic receipts
- `PDS-AC29` historical fixed receipt remains readable through the compatibility decoder
- `PDS-AC30` every compatibility entry point carries a local removal note and parity gate

### Runtime isolation and admission

- `PDS-AC31` two assignments over one subject may cite distinct package receipts without sharing catalogs, invokers, bindings, grants, or current state
- `PDS-AC32` the same exact capability contract may bind distinct implementation instances for two activations
- `PDS-AC33` failure or deactivation of one activation leaves an unrelated active assignment intact
- `PDS-AC34` one owner preparation failure exposes no partially active capability catalog and exact retry converges
- `PDS-AC35` credential rotation creates a new activation generation without changing package identity
- `PDS-AC36` a result carrying the wrong assignment, subject, activation generation, operation key, attempt id, or passive-delivery identity is rejected before canonical append
- `PDS-AC37` a late callback from a retired generation remains visibly stale and cannot silently change the current projection
- `PDS-AC38` retry after activation-generation or participant-incarnation change preserves one durable operation key, records a new attempt and exact lineage, or enters explicit reconciliation first
- `PDS-AC39` an exact imported receipt retains its meaning after a newer imported package head installs
- `PDS-AC40` concurrent head selection reports one conflict while both exact package receipts remain resolvable
- `PDS-AC41` trusted in-process and serialized external fake adapters produce equivalent owner-domain contract results from identical bounded inputs
- `PDS-AC42` same-principal and same-package sharing constraints combine by conjunction rather than overwrite
- `PDS-AC43` implementation registration creates a new catalog generation without mutating an active or historical executor view
- `PDS-AC44` required hard filesystem or network isolation rejects a cooperative in-process realization
- `PDS-AC45` historical interpretation replay succeeds while the external runtime is unavailable
- `PDS-AC46` prepared closure alone exposes no active invoker or external source admission
- `PDS-AC47` required participants must prove generation-scoped readiness before current publication
- `PDS-AC48` a stale expected-prior generation prevents publication while leaving the prior assignment healthy
- `PDS-AC49` failure after readiness but before current publication retires the non-current generation without admitting new work
- `PDS-AC50` a clean zero-work tick with a missing required participant or wake path is not projected as quiescent
- `PDS-AC51` a fully composed idle activation with owner waits and viable wake refs is projected as quiescent and wakes without restart
- `PDS-AC52` equivalent participant restart creates a new incarnation while preserving activation generation and unrelated live participants
- `PDS-AC53` binding, selected implementation, or isolation change creates a new activation generation
- `PDS-AC54` crash after durable operation creation but before dispatch resumes under the same operation key and a new attempt
- `PDS-AC55` crash after external dispatch keeps the operation unresolved or reconciles it before retry
- `PDS-AC56` recovery opens no admission until every required participant proves exact recovery readiness
- `PDS-AC57` fenced quiescence resolves exact owner safe points without a global store transaction
- `PDS-AC58` a late result from a retired participant incarnation cannot silently enter current admission

## Rejection Criteria

Reject an implementation that introduces any of the following:

- one central enum for every domain component kind
- one PDS body containing belief, Agent, Strategy, docs, and security fields
- root decoding of domain component bodies
- direct writes from theory into belief, Agent, Goal, task, or event state
- expression-name activation dispatch
- owner handlers with unrestricted product stores
- package-controlled native code loading
- floating imports resolved at activation time
- automatic provider requirement for all packages
- one generic success value for installation, activation, and runtime outcome
- one placement enum treated as proof of binding, state, failure, resource, effect, and admission isolation
- one product-wide capability catalog shared by active assignments
- transport success or process exit treated as a canonical domain result
- retry of an ambiguous effect under a new durable operation key without reconciliation
- one receipt that conflates inert preparation with live current-generation authority
- constructor, process spawn, or transport connection treated as sufficient readiness
- one identity used for activation generation, participant incarnation, and operation attempt
- a long-running external call held open inside one supervisor step
- a clean zero-work tick treated as quiescence without participant and wake closure
- a quiescence receipt that claims simultaneous persistence across owner stores
- recovery that reopens admission before owner and operation reconciliation

## Completion Condition

The router design is satisfied when two dissimilar packages install through the same manifest and receipt shape, every body remains owned and resolved by its destination domain, exact capabilities enter through activation without expression dispatch, historical package meaning remains reconstructable, and the existing Meld runtime carries both stewardship expressions without a second cognition or execution system.
