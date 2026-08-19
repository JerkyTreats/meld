# PDS W01 Router Substrate Design

Date: 2026-08-16
Status: accepted for authorized implementation
Mapped workstream: `W01`
Scope: inert package loading, structural routing, exact owner installation refs, package receipts, and historical resolution

## Objective

Define the exact `theory::router` substrate that installs state-free package meaning without activating runtime authority or importing owner semantics.

## Ownership

`theory` owns:

- package source materialization and canonical identity
- route identity and immutable route catalog assembly
- structural requirements and exact direct imports
- deterministic dispatch order
- generic installed-component refs
- package receipt construction, persistence, and exact resolution
- structural diagnostics

Each route-owning domain owns body decoding, validation, installation, canonical revision identity, exact resolution, semantic-link validation, and owner diagnostics.

Init is a caller. Runtime is an exact receipt consumer. Neither owns router behavior.

## Domain-First Code Shape

```text
src/theory.rs
src/theory/contracts.rs
src/theory/package.rs
src/theory/router.rs
src/theory/receipt.rs
src/theory/registry.rs
src/theory/resolution.rs
src/theory/error.rs
```

`src/theory.rs` declares these child modules. No `mod.rs` file is introduced.

## Package Source Contract

```rust
struct PdsPackageManifestV1 {
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
    owner_component_id: String,
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

Schema version one supports exact direct imports. Transitive meaning is reached only through the imported receipt graph. Floating imports and requirement cycles are rejected.

Description is non-semantic in schema version one. Unknown fields are rejected.

## Identity Rules

Package content identity hashes this canonical tuple:

```text
manifest schema version
package id
package version
sorted exact import package and receipt ids
sorted component ids
each owner component id
each route identity
each component schema version
each canonical source hash or exact published identity
sorted structural requirements
```

Canonicalization uses a versioned deterministic serializer. Source file order, description, filesystem path, install sequence, workspace, credentials, provider, endpoint, and placement do not participate.

Package id, package version, component id, owner domain, component kind, and published identity are nonempty normalized UTF-8 strings. Schema version one permits ASCII letters, digits, dash, underscore, and dot. The implementation rejects normalized collisions.

`component_id` is package-local structure. `owner_component_id` is the semantic id the route owner validates and installs. Keeping both explicit avoids making package-local naming part of an owner registry contract.

`PublishedExact` is materialized through the compiled publisher inventory before owner validation. Its declared publisher id, component id, and exact content identity participate in package identity. The resolved bytes are verified against that exact compiled publication and do not replace the declared identity tuple.

## Route Contract

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

Route version governs the handler protocol. Component schema version governs the owner body. A compiled handler may accept a bounded schema range. The catalog contains one handler per exact route id.

## Handler Boundary

```rust
trait TheoryRouteHandler: Send + Sync {
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

The validation token is opaque, process-local, handler-bound, catalog-bound, package-bound, component-bound, and source-hash-bound. The router carries it intact and never serializes it.

Handlers are constructed with narrow owner command and query ports. They never receive `OpenProductStores` or another owner's mutable command port.

## Immutable Route Catalog

`TheoryRouteCatalog` is assembled before package input is loaded.

Catalog construction:

1. Collect compiled owner handler publications.
2. Sort by exact route id.
3. Reject duplicate route ids even when contracts appear identical.
4. Reject an empty owner domain, component kind, or zero route version.
5. Derive one catalog assembly identity from sorted route contracts and handler contract versions.
6. Freeze the catalog behind immutable query access.

A package cannot publish a handler or claim owner identity.

## Structural Validation Order

The router performs every locally decidable validation before the first owner write:

1. Materialize the manifest and bounded component bytes.
2. Reject absolute paths, parent traversal, symlink escape, mutable URLs, oversized files, and source-hash mismatch.
3. Reject duplicate ids and normalized collisions.
4. Compute package content identity.
5. Resolve and verify every exact direct import.
6. Resolve every route and accepted component schema.
7. Enforce route cardinality.
8. Resolve every structural requirement.
9. Reject requirement cycles.
10. Obtain owner validation tokens for every component.

Owner install begins only after step ten succeeds for the complete package.

## Installation Saga

Owner installation order is sorted by route id and component id.

```text
all local and owner validation complete
→ install each owner component idempotently
→ verify every exact owner ref
→ validate owner semantic links
→ build generic package receipt
→ append receipt
→ optionally advance package head through expected-prior compare and swap
```

An owner append may remain after a later failure. It is invisible to package consumers because no package receipt cites it. Exact retry reuses that owner revision.

There is no cross-store rollback and no owner-global current head participates in package resolution.

## Installed Component And Receipt Contracts

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

struct PdsPackageInstallationReceiptV1 {
    receipt_id: String,
    package_id: String,
    package_version: String,
    package_content_hash: String,
    manifest_schema_version: u32,
    exact_imports: Vec<InstalledExactPackageImport>,
    components: Vec<InstalledTheoryComponentRef>,
    installed_at_seq: u64,
}
```

Receipt identity includes every field except `receipt_id` and `installed_at_seq`. Imports sort by package id then receipt id. Components sort by route id then component id.

`source_content_hash` proves exact supplied bytes. `owner_revision.content_hash` proves canonical owner meaning. Both are retained.

## Persistence

The theory domain opens two versioned trees in the external product database:

```text
pds_package_receipts_v1
pds_package_heads_v1
```

Receipt keys are content-derived receipt ids. Receipt values are immutable canonical bytes. Reinstalling identical bytes is unchanged. Existing different bytes under the same id are corruption.

Head keys are package ids. A head value contains package version, receipt id, and head revision. Head advancement uses expected-prior compare and swap. Historical resolution never reads a head.

Package source is read-only input. No theory state is stored under the target workspace.

## Exact Resolution

```rust
struct ResolvedPdsPackage {
    receipt: PdsPackageInstallationReceiptV1,
    components_by_route: BTreeMap<TheoryRouteId, Vec<InstalledTheoryComponentRef>>,
    exact_imports: BTreeMap<String, ResolvedImportedPdsPackage>,
}
```

Exact resolution verifies receipt identity, exact imports, sorted component closure, route availability, and every owner ref through its current compiled route handler. It returns refs only. Root never receives decoded owner bodies.

A missing current handler for a historical route fails closed with `route_unavailable`. A missing owner revision fails with `owner_revision_missing`. Neither falls back to a current owner head or current package version.

## Error Contract

Router diagnostics carry:

```rust
struct TheoryRouterDiagnostic {
    code: String,
    package_id: Option<String>,
    component_id: Option<String>,
    route: Option<TheoryRouteId>,
    owner_code: Option<String>,
    message: String,
}
```

Stable codes include:

- `package_source_invalid`
- `package_identity_mismatch`
- `exact_import_missing`
- `exact_import_mismatch`
- `route_unavailable`
- `component_schema_unsupported`
- `route_cardinality_invalid`
- `component_requirement_missing`
- `component_requirement_cycle`
- `owner_validation_failed`
- `owner_install_failed`
- `owner_revision_missing`
- `owner_link_invalid`
- `package_receipt_conflict`
- `package_receipt_corrupt`
- `package_head_conflict`

Owner diagnostics remain owner-authored and are nested without translation into generic semantic claims.

## Verification Design

Focused tests:

```text
theory::package::tests::package_identity_ignores_source_order
theory::package::tests::source_escape_and_hash_mismatch_fail_before_dispatch
theory::registry::tests::duplicate_route_publication_is_rejected
theory::router::tests::all_validation_precedes_owner_install
theory::router::tests::partial_owner_install_remains_unselectable
theory::router::tests::retry_reuses_exact_owner_revision
theory::router::tests::semantic_link_failure_prevents_receipt_commit
theory::receipt::tests::receipt_identity_is_order_independent
theory::receipt::tests::corrupt_receipt_fails_closed
theory::resolution::tests::historical_resolution_uses_exact_refs
theory::resolution::tests::missing_owner_revision_has_no_head_fallback
theory::resolution::tests::missing_historical_route_fails_closed
```

Static checks:

```sh
rg -n "DocsClaimPolicy|DependencySecurity|docs_freshness|dependency_security" src/theory.rs src/theory
rg -n "OpenProductStores" src/theory.rs src/theory
rg --files src/theory src | rg '/mod\.rs$'
```

The expected result for the first two searches is empty.

## Acceptance Criteria

- one manifest routes at least three components through fake owner handlers
- missing and duplicate routes fail before owner writes
- owner partial installation cannot be selected without a package receipt
- exact reinstall is idempotent
- package and receipt identities are independent of source order
- historical exact resolution fails closed on missing refs
- install starts no actor, opens no credential, and registers no invoker
- root and theory contain no application body types

## Rejection Criteria

Reject implementation if it:

- decodes owner bodies in theory
- receives unrestricted owner stores
- uses a cross-store rollback claim
- resolves historical meaning through current heads
- permits package-controlled handler registration
- includes workspace, credentials, provider, endpoint, or placement in package identity
- executes package-provided code
- activates runtime behavior during installation

## Decision Ledger

- The top-level owner is `theory`, with `router` as behavior.
- Schema version one supports exact direct imports.
- Requirement cycles are rejected.
- Route contracts may accept bounded owner schema ranges.
- Package visibility is receipt-scoped and commit-last.
- Package heads are selection conveniences only.
- The generic resolved package contains refs, not decoded bodies.

## Completion Condition

`D01` is delivered. `W01` may implement this router substrate without further cross-domain design decisions.
