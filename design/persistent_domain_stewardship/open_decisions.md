# PDS Open Decisions

Date: 2026-07-16  
Status: proposed decision register  
Scope: unresolved architectural choices in Persistent Domain Stewardship

## Purpose

This register prevents recommendations and illustrative schemas from becoming implicit decisions.

Each entry records:

- options
- current recommendation
- why the decision remains open
- evidence needed
- major consequences

## D01: PDS Location

### Options

1. Root `meld` product module only.
2. Extracted `meld-stewardship` control-plane crate.
3. Pure design and package tooling with no persistent runtime domain.
4. Split `meld-stewardship-spec` plus root runtime coordination.

### Current recommendation

Start in root `meld` for the first proof while maintaining contracts that permit later extraction.

### Why open

The size and independent source truth of PDS are not yet proven.

### Evidence needed

- first package/profile activation
- package upgrade
- second domain integration
- persistence needs for assignments and receipts

## D02: Package Ownership Model

### Options

1. Central PDS-owned package schema.
2. Federated domain-owned facets linked by PDS.
3. Root adapter configuration without a package meta-model.

### Current recommendation

Federated facets, with a root-composed first implementation if required.

### Why open

A central schema may be materially simpler, while facets may introduce excessive linking complexity.

### Evidence needed

Implement the same documentation steward through both models and compare change surface and diagnostics.

## D03: Customer Source Representation

### Options

1. Full package YAML.
2. Simplified profile YAML.
3. HCL-like profile language.
4. Purpose-built Steward Profile Language.
5. Code-first SDK.
6. Generated visual forms.
7. Conversational editor over structured state.

### Current recommendation

Typed canonical profile model, initially encoded as simplified YAML, with visual and conversational editing. Defer a custom parser.

### Why open

The common profile model has not been tested through real authoring tasks.

### Evidence needed

Profile-authoring experiment described in `evaluation_plan.md`.

## D04: Package, Profile, Assignment, And Activation

### Options

1. Four distinct records.
2. Package and profile distinct; assignment includes activation.
3. Package only plus external application config.

### Current recommendation

Keep all four conceptually distinct; permit a combined first-slice storage representation.

### Why open

Separating records improves lifecycle clarity but increases implementation and user-visible concepts.

### Evidence needed

- same profile activated in two environments
- connector rotation without normative change
- profile revision without deployment change

## D05: Standing Objective Ownership

### Options

1. PDS owns objective runtime state.
2. Agent owns objective runtime state; PDS owns declaration lineage.
3. Objective is reconstructed from the profile and current belief.
4. Split PDS assignment declaration with Agent-owned evaluation state.

### Current recommendation

Split declaration and evaluation: PDS links the declaration; Agent owns normative evaluation.

### Why open

Objective lifecycle may require durable PDS coordination beyond Agent curation.

### Evidence needed

Repeated breach/restoration proof and package upgrade while objective is active.

## D06: Stewardship Episode Ownership

### Options

1. Authoritative PDS episode record.
2. Agent-owned episode lifecycle.
3. PDS projection over domain events.
4. Split authoritative coordination record plus derived details.

### Current recommendation

Begin with a derived projection; add authoritative state only when coordination cannot be reconstructed.

### Why open

Approvals, verification windows, and package upgrades may require explicit coordination state.

### Evidence needed

- multi-goal episode
- failed intervention followed by alternate method
- external restoration
- reopen proof

## D07: Domain Vocabulary

### Options

1. Central PDS symbol registry.
2. Domain facets export and import symbols.
3. Runtime-only strings with conventions.

### Current recommendation

Domain-owned exports linked through a PDS symbol table.

### Why open

A central registry could simplify tooling and package discovery.

### Evidence needed

Cross-domain package with software and organization or game concepts.

## D08: Facet Connector Scope

### Options

1. Full compile, prepare, activate, deactivate, migrate, inspect protocol.
2. Compile and validate only; root owns activation.
3. Generic registration plans interpreted by root adapters.
4. No common protocol; package compiler calls domain APIs directly.

### Current recommendation

Design toward the full protocol, implement the smallest subset required by the first slice.

### Why open

The full lifecycle may be premature before package activation exists.

### Evidence needed

Partial activation and package upgrade prototypes.

## D09: Context Projection

### Options

1. Separate PDS package module.
2. World-model facet.
3. Context-domain facet.
4. Capability input-binding concern.
5. Method-local declaration.

### Current recommendation

Treat it as an explicit package concept compiled to a world-model/context/capability contract; final owner remains open.

### Why open

Context projection crosses graph reads, belief selection, prompt artifacts, and capability inputs.

### Evidence needed

Documentation, performance, and learner methods with different context needs.

## D10: Governance Ownership

### Options

1. PDS governance facet owns policy.
2. External organization policy system owns grants and approvals.
3. Execution owns all enforcement.
4. Split request, grant, and enforcement.

### Current recommendation

Split:

- package/profile request authority;
- principal and organization policy grant authority;
- planning filters;
- dispatch enforcement.

### Why open

The repository does not yet have one complete governance domain.

### Evidence needed

Denied-action, approval, budget, and emergency restriction scenarios.

## D11: Outcome Verification Ownership

### Options

1. PDS outcome engine.
2. World-model evidence mappings plus Agent satisfaction.
3. Execution interprets task outcomes semantically.
4. Domain-specific evaluator facets.

### Current recommendation

Domain outcome facets produce evidence through the existing world-model path; Agent evaluates restoration. PDS correlates status.

### Why open

Some verification windows may need explicit PDS coordination.

### Evidence needed

Task success without restoration, harmful result, and delayed outcome scenarios.

## D12: Workflow End State

### Options

1. Workflows remain first-class applications alongside PDS.
2. Workflows become compatibility methods.
3. Workflow stages are incrementally absorbed into task-network execution.
4. Workflows are replaced after parity.

### Current recommendation

Use workflows as compatibility methods and migrate semantics only where the target domain is ready.

### Why open

The workflow runtime remains proven infrastructure; planner and task-network replacement value is not fully demonstrated.

### Evidence needed

Documentation steward parity and runtime-cost comparison.

## D13: Package Scenario Ownership

### Options

1. PDS owns a cross-domain deterministic scenario runner.
2. Each domain owns facet conformance tests; PDS links results.
3. Scenarios remain integration tests in Rust.

### Current recommendation

Hybrid domain conformance plus PDS integration scenarios.

### Why open

A generic scenario language may become another large DSL.

### Evidence needed

Three dissimilar package scenario suites.

## D14: Runtime Actor Instantiation

### Options

1. PDS activation directly starts generic runtime actors.
2. Supervisor discovers durable work from domain stores.
3. Root application starts a fixed actor set independent of packages.
4. One isolated runtime process per assignment.

### Current recommendation

Fixed generic actor kinds with package/assignment-scoped durable registrations.

### Why open

The current runtime assembly does not yet prove the optimal multiplicity and isolation model.

### Evidence needed

Multiple concurrent assignments and restart/reopen behavior.

## D15: Stewardship Lineage Transport

### Options

1. Embed one full lineage struct in all records.
2. Attach lineage only to commands and events.
3. Use event relations and reconstruct lineage.
4. Store a compact lineage id resolved through PDS.

### Current recommendation

Compact neutral lineage at public boundaries plus event relations where sufficient.

### Why open

Full propagation may create pervasive coupling and storage overhead.

### Evidence needed

End-to-end explanation query and package-upgrade replay.

## D16: Package Upgrade Semantics

### Options

1. In-place assignment upgrade at a sequence boundary.
2. New assignment and activation identity.
3. Parallel old/new activation with migration period.
4. Domain-specific policy only.

### Current recommendation

PDS coordinates a sequence-bound upgrade while each domain owns local migration. Major incompatible changes may require a new assignment.

### Why open

No package compiler or assignment runtime exists yet.

### Evidence needed

Belief comparator change, method change, authority change, and activation-only change.

## D17: PDS Crate Public API Timing

### Options

1. Define crate contracts before the first implementation.
2. Implement in root and extract after two package proofs.

### Current recommendation

Implement in root and extract only after stable source truth and contracts emerge.

### Why open

Premature extraction may freeze a speculative meta-model.

## Decision Review Process

A decision entry should be updated when:

- an implementation experiment completes;
- a domain assessment changes;
- a selected option affects public contracts;
- a recommendation is promoted to an implementation plan;
- an option is rejected.

Accepted decisions should move into an authoritative design or implementation plan rather than silently changing the proposal status.