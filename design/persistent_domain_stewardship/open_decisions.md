# PDS Open Decisions

Date: 2026-08-18
Status: active decision register under canonical PDS architecture
Scope: unresolved architectural choices in Persistent Domain Stewardship

## Purpose

This register prevents recommendations and illustrative schemas from becoming implicit decisions.

Each entry records:

- options
- current recommendation
- why the decision remains open
- evidence needed
- major consequences

## Resolved Canonical Boundaries

The following questions are no longer open:

- PDS supplies stable semantic meaning and does not precompute situated cognition.
- activation supplies the exact capability snapshot.
- Strategy owns current problem construction and separately admitted Methods.
- Agent supplies construction policy and candidate judgment.
- planner projection requests are derived for the current Goal.
- search controls remain request-scoped.
- assignment request, principal grant, Agent judgment, and execution enforcement remain separate authority gates.

See [Canonical Persistent Domain Stewardship](../cognitive_architecture/persistent_domain_stewardship.md).

## D01: PDS Location

### Options

1. Root `meld` product module only.
2. Extracted `meld-stewardship` control-plane crate.
3. Pure design and package tooling with no persistent runtime domain.
4. Split `meld-stewardship-spec` plus root runtime coordination.

### Current recommendation

Implement the next routed package proof as a top-level `theory` domain in root `meld`, while keeping contracts narrow enough for later extraction if independent source truth emerges.

### Why open

The delivered lower layer does not require a PDS runtime domain. The proposed router has a plausible source truth for inert package structure and receipts, but two consumers have not yet proved that it warrants durable extraction.

### Evidence needed

- generic routed package activation
- package upgrade
- second domain integration
- persistence needs for assignments and receipts

## D02: Package Ownership Model

### Options

1. Central PDS-owned package schema.
2. Federated domain-owned facets linked by PDS.
3. Root adapter configuration without a package meta-model.

### Current recommendation

Domain-owned routed components under one small structural package envelope.

### Why open

A central schema may be materially simpler, while facets may introduce excessive linking complexity.

### Evidence needed

Route documentation freshness and dependency security through the same structural envelope. Reopen the central-schema option only if owner routing adds complexity without preserving a real authority boundary.

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

Keep all four conceptually distinct. Let the router iteration combine storage only where the exact lifecycle and identity distinctions remain recoverable.

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

Agent owns installed maintained-condition meaning and evaluation. PDS may own principal-facing declaration lineage and derived projection.

### Why open

Lower-layer evaluation ownership is settled. What remains open is whether approvals, multi-Goal narratives, or cross-generation upgrades require separate upper-layer coordination state.

### Evidence needed

Repeated breach and restoration are delivered for the current expression. Remaining evidence is a second expression, several assignments, and package upgrade while one maintained condition is active.

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

Theory owns structural component requirements and exact route identity. Destination domains own semantic linking through narrow public contracts. Add a broader symbol table only if two consumers need one owner-neutral symbol product.

### Why open

A broader symbol product could simplify tooling and package discovery, but it could also become a hidden universal ontology.

### Evidence needed

Cross-domain package with software and organization or game concepts.

## D08: Facet Connector Scope

### Options

1. Full compile, prepare, activate, deactivate, migrate, inspect protocol.
2. Compile and validate only; root owns activation.
3. Generic registration plans interpreted by root adapters.
4. No common protocol; package compiler calls domain APIs directly.

### Current recommendation

Implement the smallest owner route and activation-contributor subsets exercised by documentation freshness and dependency security. Generalize only after both consumers expose the same lifecycle need.

### Why open

The router and activation designs now describe the full pressure surface, but the smallest stable public contributor contract remains unproven.

### Evidence needed

Partial activation failure, exact retry, generation-scoped readiness, expected-prior publication, deactivation, credential rebinding, and package upgrade prototypes. One neighboring assignment must remain healthy throughout the failed activation case.

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

- declaration and assignment request authority;
- principal and organization policy grant authority;
- planning filters;
- dispatch enforcement.

### Why open

The current runtime has a complete minimal authority path. Approval, budget, delegation, expiry, organization hierarchy, and hot revocation do not yet have a settled owner model.

### Evidence needed

Denied action is delivered at the current maturity. Remaining evidence is approval, budget, delegation, expiry, and emergency restriction across more than one assignment.

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

Use workflows as separately admitted compatibility Methods and migrate semantics only where the target domain is ready. Workflow topology is not PDS semantic theory.

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
5. Domain-selected placement satisfying a portable activation isolation contract.
6. One stable supervised activation-lifecycle service owning assignment-local generations.
7. Dynamic assignment-generation roles registered directly with the root supervisor.

### Current recommendation

Use fixed generic Meld actor kinds and one stable supervised activation-lifecycle service for the next implementation. The service owns assignment-local activation generations and coordinates domain-owned participant incarnations. Equivalent mechanical restart creates a new incarnation under the same generation only after recovery readiness closes. Domain adapters may use different physical placements behind a portable activation contract. Dynamic assignment-generation roles do not enter the root supervisor contract.

### Why open

The current runtime assembly does not yet prove the optimal multiplicity and isolation model. Process count alone does not settle semantic, authority, binding, state, failure, resource, effect, admission, or replay isolation.

### Evidence needed

Multiple concurrent assignments, activation during active idle, generation-scoped readiness, shared and dedicated adapter placements, stale-generation and stale-incarnation result handling, and restart or reopen behavior. The [canonical runtime lifecycle and quiescence design](../cognitive_architecture/runtime_lifecycle_and_quiescence.md) fixes the shared lifecycle semantics while this decision continues to track multiplicity and placement evidence.

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

No generic routed package compiler or durable multi-assignment runtime exists yet.

### Evidence needed

Belief comparator change, method change, authority change, activation-only change, and old and new in-flight work visible across one revision boundary.

## D17: PDS Crate Public API Timing

### Options

1. Define crate contracts before the first implementation.
2. Implement in root and extract after two package proofs.

### Current recommendation

Implement in root and extract only after stable source truth and contracts emerge.

### Why open

Premature extraction may freeze a speculative meta-model.

## D18: Runtime Isolation Requirement Encoding

### Options

1. One fixed process-isolation mode for every activation.
2. One placement enum with implied isolation guarantees.
3. A portable vector of sharing, state, failure, secret, filesystem, network, resource, effect, and late-result requirements.
4. Entirely domain-specific activation policy with no common isolation vocabulary.

### Current recommendation

Explore a small portable requirement vector while leaving enforcement to runtime composition and owner adapters. Do not infer isolation guarantees from placement alone.

### Why open

Docs freshness can use trusted in-process behavior, while dependency security may use subprocesses, shared sidecars, hosted services, or persistent monitors. The common vocabulary has not yet been proven small enough to avoid becoming a universal deployment schema.

### Evidence needed

- one package activated through two placements with equivalent domain contracts
- one shared service preserving assignment and credential isolation
- one retired activation returning a late result
- one ambiguous external effect recovered through a stable operation key and distinct retry attempt
- one package revision upgrade with old and new work concurrently visible

See [PDS Isolation And Runtime Portability](isolation_and_runtime_portability.md).

## Decision Review Process

A decision entry should be updated when:

- an implementation experiment completes;
- a domain assessment changes;
- a selected option affects public contracts;
- a recommendation is promoted to an implementation plan;
- an option is rejected.

Accepted decisions should move into an authoritative design or implementation plan rather than silently changing the proposal status.
