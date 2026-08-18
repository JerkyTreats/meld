# Theory Declaration Lowering Implementation Design

Date: 2026-08-13
Status: implemented
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Domain assessment: [Theory Declaration Lowering Assessment By Domain](theory_declaration_lowering_domain_assessment.md)
Completion evidence: [Theory Declaration Lowering Completion Evidence](theory_declaration_lowering_completion_evidence.md)
Scope: Theory Elevation Step 2

> Historical compatibility design. The declaration and `StrategyTheoryPackage` shape below describe the implemented antecedent. [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) supersedes any claim that exact capabilities, Strategy policy, bounds, or projection dimensions are PDS semantic theory.

## Objective

Lower named stewardship declarations into the exact receipt-backed runtime path without selecting semantics by expression name. Preserve docs freshness behavior while retiring the hand-composed production PDS image and its dead Method-era source artifacts.

## Maturity Boundary

The declaration introduced here is the smallest canonical selection required by the implemented runtime. It names expression identity, physical scope, Agent, provider, and the six Step 1 theory identities. It is not the final principal-facing PDS language.

Maintained conditions, authority, assignment, package presets, activation records, and declaration persistence remain outside this step. They enter only through their owning later program steps.

## Canonical Declaration

Root configuration accepts a deterministic map under `stewardship.declarations`. The map key is a configuration-local declaration id. Each value carries:

```text
expression
target_root
subject
agent_id
provider_id
theory identities
```

The expression is validated as a nonempty identity. Root config does not enumerate allowed expression names.

The existing `stewardship.docs_freshness` table remains a temporary compatibility reader. It validates its historical expression constraint and lowers to the same canonical declaration value with declaration id `docs_freshness`. A canonical declaration with that same id and the legacy table cannot coexist.

## Lowering Path

```text
canonical or compatibility config
  -> named stewardship declaration
  -> physical binding
  -> selected stewardship package
  -> active installation receipt
  -> exact owner revisions
  -> owner Strategy activation
  -> exact capability implementation activation
  -> existing runtime actor bindings
```

Every arrow either preserves an intact contract or calls the owner that interprets it.

## Target Selection

CLI runtime composition selects the declaration whose canonical target equals the addressed workspace. World initialization uses the same target selection. Zero matches means no stewardship composition for ordinary runtime boot and a truthful error for explicit world initialization. More than one match is always an ambiguity error.

Configuration order never chooses a steward.

## Strategy Activation

`AgentStrategyRuntimeConfig` gains an owner-side constructor from an installed `StrategyTheoryPackage`, concrete subject, and Agent identity. The constructor validates the package and builds the existing immutable Strategy problem template. Root supplies grounding values and does not inspect settlement rules, capabilities, evaluation policy, or search bounds.

The exact Strategy revision from the receipt is attached after owner activation and remains the durable authorization lineage source.

## Capability Activation

Domain capability publishers register invokers only when an exact installed contract selects their type and version. They compare content identity before registration. Root composes the built-in publishers without consulting the expression identity, then rejects every installed contract lacking an exact process implementation.

Docs owns its five invoker constructors and claim-policy validation. Meld execution continues to own catalogs, invoker lookup, planning validation, and dispatch.

Adding another expression that reuses installed capabilities changes declaration and package data only. Adding a genuinely new capability still requires its owning implementation and normal product capability publication, but never an expression branch.

## PDS Compatibility Retirement

`src/docs/pds.rs` is test-only. It remains a regression fixture for the five-capability search and lowering shape and now calls the same owner activation contracts as production.

The stale docs Method, available-action, and realization files are deleted. No loader for expression-keyed planning theory remains in root initialization.

## Acceptance Criteria

### AC1 Generic declaration selection

Canonical declarations accept arbitrary nonempty expression identities and lower in deterministic declaration-id order. Source-aware validation names the complete canonical field path. Legacy docs config lowers with characterized parity.

### AC2 Unambiguous physical binding

CLI and world initialization select by canonical target. Distinct targets remain isolated. Duplicate declarations for one target fail before stores or semantic actors are changed.

### AC3 No root expression dispatch

Production config, CLI, init, and runtime assembly contain no semantic branch that compares a canonical declaration expression to an application name. Production runtime assembly does not call `docs::pds`.

### AC4 Owner Strategy activation

Meld world model validates and activates an installed Strategy package without application vocabulary. Root passes only subject and Agent grounding inputs.

### AC5 Exact implementation binding

Docs invokers register by exact type, version, and content identity. Identity drift fails composition. Any installed contract without a process implementation remains an unresolved theory image and cannot reach dispatch.

### AC6 Compatibility retirement

The direct docs PDS composer is test-only. The stale docs Method, available-action, and realization source artifacts and their root loaders are absent.

### AC7 Runtime parity

A canonical declaration whose expression name is not `docs_freshness` installs the shipped docs theory, activates the receipt-backed runtime, closes the five-capability Strategy graph without Method or workflow routing, preserves revision A and revision B lineage, and converges to quiet passes.

### AC8 Quality gates

All changed Rust passes:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

### AC9 Declaration-selected belief context

Belief-context hydration receives the selected family identity through the existing package trigger. Production dispatch copies that identity from the physical stewardship binding. Context and workflow code contain no compiled application family fallback, and an enabled hydration request without a binding fails truthfully.

## Compatibility Removal Conditions

The legacy docs selection field may be removed after shipped configuration and all fixtures use named declarations and source-aware canonical loading coverage remains green.

The direct single-binding resolver may be removed after every caller uses all-binding or target-based selection and single-selection characterization remains green.

The test-only docs PDS fixture may be removed when equivalent owner-contract search and lowering coverage exists outside that module.

## Explicit Deferrals

- no standing maintained condition
- no effective authority model
- no CVE package or capability implementation
- no settled replay
- no final declaration authoring syntax
- no generic owner-body registry
- no hot theory replacement
