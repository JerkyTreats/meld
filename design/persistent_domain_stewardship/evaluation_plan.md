# PDS Evaluation Plan

Date: 2026-07-16  
Status: proposed  
Scope: experiments required to validate or falsify the PDS abstractions before implementation commitments

## Purpose

The PDS proposal contains several attractive abstractions:

- persistent domain steward
- operational domain theory
- steward profile
- assignment and activation
- federated domain facets
- standing objectives and episodes
- package compiler and linker
- workflow migration

The proposal should not be accepted based on conceptual coherence alone.

This plan defines experiments that test usability, domain isolation, runtime fit, package generality, migration value, and upgrade behavior.

## Evaluation Principles

1. Compare against a simpler baseline.
2. Measure authoring and maintenance, not only runtime behavior.
3. Use dissimilar domains to test the common form.
4. Treat package and profile upgrade as first-class scenarios.
5. Separate customer interaction from expert package authoring.
6. Test domain isolation by recording kernel and crate changes required by each package.
7. Preserve failure evidence when an abstraction does not generalize.

## Experiment 1: Profile Expression

### Goal

Determine whether a small customer-facing profile can express stewardship intent without exposing package mechanics.

### Candidate representations

- simplified YAML
- HCL-like syntax
- purpose-built textual prototype
- generated form
- conversational editor producing structured mutations

### Common tasks

1. Create a software-performance steward from a preset.
2. Narrow its scope from repository to service.
3. Add documentation as a secondary objective.
4. Change sensitivity from balanced to strict.
5. Change autonomy from recommend to draft.
6. Add an approval requirement.
7. Increase compute budget by 20 percent.
8. Change verification from fast to release-grade.
9. Add escalation after repeated failure.
10. Apply one advanced threshold override.

### Measures

- time and operations required
- syntax and semantic error count
- concepts the author must understand
- ability to explain effective behavior
- semantic-diff quality
- round-trip fidelity
- package-upgrade experience

### Success indication

Most normal tasks require only profile-surface concepts. Deep package mechanics remain unnecessary.

### Falsification

If common changes repeatedly require low-level evidence, comparator, method, or route editing, the profile abstraction is insufficient.

## Experiment 2: Central Schema Versus Federated Facets

### Goal

Compare the current central package model with the proposed domain-facet model.

### Package

Documentation freshness steward.

### Central-schema implementation

One PDS schema defines observation, belief, Agent, method, outcome, and governance sections.

### Facet implementation

Each participating domain compiles its own facet and exports or imports linked symbols.

### Measures

- lines and types in PDS core
- lines and types in participating domains
- number of cross-domain dependencies
- diagnostic clarity
- package compile complexity
- package upgrade complexity
- ability to reuse existing runtime contracts directly

### Success indication for facets

PDS core remains stable while domains validate their own semantics, and the additional linker complexity is bounded.

### Falsification

If all facets use effectively identical generic structures and domain compilers merely forward data, the central schema may be more appropriate.

## Experiment 3: Three-Domain Generality

### Goal

Test whether the common package and profile form extends beyond software.

### Packages

1. Software performance steward.
2. Game faction steward.
3. Learner steward.

### Required shared profile dimensions

- steward type
- scope
- desired conditions
- sensitivity
- autonomy
- budget
- escalation
- verification

### Measures

- PDS core schema changes per package
- new `meld-lang` grammar required
- new runtime actor kinds required
- new domain-specific branches in root assembly
- profile concepts that fail to transfer
- package facets that remain domain-specific

### Success indication

Packages add domain facets and adapters without changing PDS core semantics or cognitive-runtime grammar.

### Falsification

If each package needs a bespoke lifecycle, planner, or PDS schema extension, the common form is superficial.

## Experiment 4: Runtime Lowering

### Goal

Prove that package/profile declarations can compile into existing cognitive-runtime anchors.

### First target

Documentation freshness.

### Required lowering

- belief family registration
- Agent registration and subscription
- curation policy
- method library entry
- capability requirements
- outcome evidence route
- package and assignment lineage
- activation of generic runtime actors

### Measures

- existing runtime code changed
- root adapter code added
- profile-specific branching
- declarations still hard-coded in tests or fixtures
- stages manually invoked by integration code

### Success indication

The package-driven path closes the same loop without a PDS-specific planner, belief engine, or task executor.

## Experiment 5: Standing Objective And Episode

### Goal

Determine whether objective and episode require authoritative PDS state or can be projected.

### Scenario

1. Objective begins healthy.
2. Belief enters breach.
3. Observation goal is created.
4. Intervention goal follows.
5. First method fails.
6. Alternate method succeeds mechanically.
7. Verification remains inconclusive.
8. External change restores the condition.
9. Later evidence opens a second breach.
10. Runtime restarts between stages.

### Compare

- authoritative PDS episode state
- Agent-owned state
- projection from domain events
- split coordination and projection model

### Measures

- reconstruction accuracy
- duplicated state
- conflict handling
- reopen behavior
- number of cross-domain commands
- upgrade behavior during an open episode

## Experiment 6: Authority Boundary

### Goal

Verify that profile intent and capability availability cannot expand authority.

### Scenarios

- profile requests draft authority but principal grants observe-only
- method matches but action is prohibited
- budget exhausted after planning but before dispatch
- emergency restriction narrows an active assignment
- approval granted for one action only
- package upgrade requests broader authority

### Success indication

Planning and dispatch independently enforce the same effective-authority result, and semantic diff highlights authority changes.

## Experiment 7: Package Upgrade

### Goal

Test independent semantic and physical changes.

### Upgrade classes

- additive profile option
- belief comparator change
- breach/restore preset change
- method replacement
- outcome verification change
- requested authority increase
- connector credential rotation
- provider change
- source-scope expansion

### Compare

- package revision
- profile revision
- assignment revision
- activation revision

### Measures

- which changes require approval
- which require belief reassessment
- which require episode migration
- which retain the same assignment identity
- replay correctness across sequence boundaries

## Experiment 8: Workflow Migration

### Goal

Determine whether stewardship should become application truth above existing workflows.

### Baselines

- current docs-writer workflow as application
- workflow wrapped as a PDS compatibility method
- workflow stage chain lowered into task composition
- native PDS package using planner-selected methods

### Measures

- behavior parity
- configuration complexity
- execution-code reuse
- duplicate authority
- retry and repair clarity
- ability to tolerate or gather evidence instead of execute
- ability to share methods across profiles

### Falsification

If PDS adds no meaningful behavior above a workflow for the target use case, workflow remains the appropriate application model.

## Experiment 9: Context Projection

### Goal

Find the correct ownership and declaration model for bounded context hydration.

### Methods

- documentation generation
- performance investigation
- learner lesson generation

### Candidate ownership

- world-model facet
- context-domain facet
- capability input binding
- method-local specification
- separate PDS package concept

### Measures

- reuse across capabilities
- package authoring complexity
- provenance and replay
- budget enforcement
- dependency direction
- non-model capability impact

## Experiment 10: Semantic Inspection

### Goal

Test whether a user can understand what one steward is doing without reading domain stores.

### Required questions

- What is this steward responsible for?
- What evidence is current?
- Why is the objective considered breached?
- Why did it observe rather than act?
- What authority does it have?
- Which method was selected and why?
- What work is active?
- What evidence is required for restoration?
- Which package and profile revision applied?
- What changed after an upgrade?

### Success indication

The unified projection answers these questions while linking to authoritative domain detail.

## Evidence Capture

Each experiment should record:

- package and profile source
- compiled image and hashes
- domain-facet diagnostics
- runtime registrations
- event trace
- semantic diff
- user or author observations
- required code changes
- rejected abstraction or schema changes

## Promotion Criteria

The PDS proposal is ready for an authoritative implementation plan when:

1. one package compiles and runs through existing runtime contracts;
2. one dissimilar package requires no PDS kernel extension;
3. the profile abstraction proves simpler than the package model;
4. ownership of objectives and episodes is resolved;
5. authority enforcement is demonstrated;
6. package upgrade and exact-hash replay are demonstrated;
7. workflow migration value is measured rather than assumed;
8. Assessment By Domain identifies no unresolved authority collision.

## Negative Results

Negative results should narrow the design rather than be hidden.

Examples:

- retain a central schema if facets add no useful isolation;
- retain YAML if a custom language offers insufficient benefit;
- keep workflows first-class for deterministic applications;
- omit episodes if domain records provide adequate projection;
- keep PDS in root if no independent source truth emerges;
- limit PDS to software domains if non-software packages require incompatible semantics.

The proposal succeeds by identifying the correct abstraction boundary, not by preserving every current hypothesis.