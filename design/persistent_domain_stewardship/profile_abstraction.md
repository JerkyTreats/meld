# Steward Profile Abstraction

Date: 2026-08-18
Status: aligned design exploration
Scope: customer-facing interaction layer above full stewardship-package and runtime configuration

> The canonical cognition boundary is fixed in [Persistent Domain Stewardship](../cognitive_architecture/persistent_domain_stewardship.md). Profiles select intent over stable PDS semantics. They do not cause exact capabilities, Methods, construction policy, search controls, or effective authority to become package-owned cognition.

## Problem

A complete stewardship package may need to represent:

- domain vocabulary
- observation and projection routes
- belief families
- perspective and curation policy
- standing objectives
- action-class and outcome semantics
- outcome verification
- governance and authority requirements
- package scenarios
- assignment scope
- physical activation bindings

Exposing that representation directly to customers would make routine stewardship curation difficult and would leak cognitive-runtime mechanics into the product interface.

PDS is intended to be the interaction layer through which users create and refine stewards. The interaction model should therefore express user intent rather than reproduce the full compiled state.

## Proposed Layering

```text
Domain package
    expert-authored operational domain theory
        ↓ exposes a curated public surface
Steward profile
    customer-authored intent and policy
        ↓ binds to
Stewardship assignment
    principal, scope, requested authority, and grant lineage
        ↓ deploys through
Stewardship activation
    connectors, credentials, providers, runtime placement, quotas
        ↓ compiles to
Strategy problem and runtime assembly
    combines the semantic image with separately owned live inputs
```

## Layer Definitions

### Domain package

Defines the stable vocabulary, evidence meaning, norms, action classes, outcomes, and governance constraints for one steward family.

Typical authors:

- Meld maintainers
- integration partners
- domain experts
- advanced customer platform teams

The package may include low-level evidence, comparator, action-class, outcome, and governance declarations. Exact capability offers come from activation and reusable Methods come from separately admitted Strategy knowledge.

### Steward profile

Expresses the customer-visible choices for one steward configuration.

Typical authors:

- repository owners
- service owners
- teachers
- game designers
- portfolio administrators
- governance administrators

A profile should not require understanding belief leases, task networks, event projections, comparator weights, or runtime actors.

### Assignment

Binds a profile to:

- a principal
- a concrete scope
- requested authority and principal grant lineage
- lifecycle state
- package and profile revisions

### Activation

Binds an assignment to physical resources:

- sensors and external systems
- credentials
- provider implementations
- capability adapters
- runtime placement
- quotas and operational budgets

### Compiled image

Contains all domain facet outputs, linked symbols, registration plans, governance requirements, and lineage.

It is inspectable but not routinely hand-authored.

## Common Customer Profile Shape

The working hypothesis is that most stewardship profiles can be expressed through eight dimensions.

```text
StewardProfile
    =
steward type
+ scope
+ desired conditions
+ sensitivity
+ autonomy
+ budget
+ escalation
+ verification
```

### Steward type

Selects a package and steward template.

Examples:

```text
software.performance
software.persistence
service.reliability
learning.technical_tutor
game.faction
portfolio.thesis
```

### Scope

Identifies the bounded subjects to steward.

Examples:

```text
service payments
repository meld
workspace /services/payments
learner account:42
faction northern-alliance
portfolio retirement
```

### Desired conditions

Selects package-defined objectives rather than requiring raw `meld-lang` propositions.

Examples:

```text
maintain interactive API health
maintain tested recovery
maintain documentation freshness
maintain prerequisite mastery
maintain mandate risk bounds
```

### Sensitivity

Selects how readily the steward investigates, intervenes, or waits.

A package-defined preset may expand into:

- breach threshold
- restore threshold
- evidence freshness
- hysteresis
- confidence requirement
- stability window
- observation budget
- escalation interval

Examples:

```text
relaxed
balanced
strict
safety_critical
```

The terms are package-defined. PDS does not assign universal meanings to `strict` or `balanced`.

### Autonomy

Selects an authority posture within the package's supported actions.

Candidate common levels:

```text
observe
recommend
draft
execute_reversible
execute_bounded
```

Organization policy and assignment grants may further restrict the selected level.

### Budget

Expresses limits relevant to the package:

- compute
- model tokens
- money
- test runtime
- external API quota
- human attention
- intervention frequency
- risk exposure

Profiles should normally select named budgets such as `minimal`, `standard`, or `intensive` rather than edit all counters individually.

### Escalation

Declares when and where control transfers.

Examples:

- insufficient evidence
- repeated failed intervention
- authority boundary
- harmful outcome
- budget exhaustion
- unresolved conflict
- objective breach exceeding a time limit

### Verification

Selects what evidence is required before the steward considers the condition restored.

Examples:

```text
fast
standard
release_grade
human_reviewed
retention_tested
```

The package expands the selected verification profile into domain-owned outcome requirements.

## Public Profile Surface

Each package should expose a curated customer-facing surface.

Candidate representation:

```rust
struct ProfileSurfaceSpec {
    steward_templates: Vec<StewardTemplateRef>,
    scope_parameters: Vec<ProfileParameter>,
    objectives: Vec<ProfileObjective>,
    sensitivity_presets: Vec<ProfilePreset>,
    autonomy_levels: Vec<ProfileAutonomy>,
    budget_profiles: Vec<ProfileBudget>,
    verification_profiles: Vec<ProfileVerification>,
    escalation_options: Vec<ProfileEscalation>,
    advanced_overrides: Vec<ProfileOverridePoint>,
}
```

The profile surface acts as the package's public API.

The package may contain many internal belief families, action classes, outcomes, and governance constraints while exposing only a small number of meaningful choices.

## Expression Options

## Option A: Full package YAML

Customers edit the complete package representation.

### Advantages

- one representation
- straightforward serialization
- no separate profile compiler

### Costs

- exposes deep runtime mechanics
- difficult to maintain
- high risk of invalid cross-references
- poor progressive disclosure
- authority changes may be hidden among incidental fields

### Current assessment

Not recommended as the primary customer interface.

It remains useful for package authors, fixtures, and compiled diagnostics.

## Option B: Simplified profile YAML

Example:

```yaml
steward:
  type: software.performance
  name: payments-performance

scope:
  service: payments

maintain:
  - objective: interactive_api
    sensitivity: strict

autonomy: draft
budget: standard
verification: release_grade

approval_required:
  - merge
  - production_deploy

escalate_to: platform-reliability
```

### Advantages

- fast first implementation
- easy API representation
- form generation and round-trip are feasible
- package schema can provide completion and validation

### Costs

- nested advanced expressions become cumbersome
- YAML remains syntax-sensitive
- users may gradually demand low-level escape hatches

### Current assessment

Recommended as an initial encoding, not necessarily the final conceptual language.

## Option C: HCL-like profile language

Example:

```hcl
steward "payments_performance" {
  use = software.performance

  scope {
    service = "payments"
  }

  maintain "interactive_api" {
    sensitivity = "strict"
  }

  autonomy    = "draft"
  budget      = "standard"
  verification = "release_grade"
}
```

### Advantages

- readable block structure
- parser and expression tooling exist
- better advanced composition than YAML

### Costs

- still resembles infrastructure configuration
- expression features may encourage programming in profiles

### Current assessment

Credible implementation substrate for a profile language.

## Option D: Purpose-built Steward Profile Language

Example:

```text
steward "Payments performance"
    using software.performance

scope service "payments"

maintain interactive_api strict

autonomy draft
budget standard
verify with release_grade

require approval for
    merge
    production_deploy

escalate unresolved to team "platform-reliability"
```

### Advantages

- user-facing vocabulary
- strong semantic diagnostics
- compact representation
- semantic diff can be natural

### Costs

- requires parser, formatter, language server, migration, and tooling
- premature syntax design may freeze the wrong abstraction

### Current assessment

Preferred long-term conceptual representation, but a custom parser should be deferred until the canonical profile model is proven.

## Option E: Code-first SDK

### Advantages

- strong typing
- tests and refactoring
- suitable for package authors and advanced platform teams

### Costs

- profiles become software projects
- arbitrary code harms deterministic compilation
- excludes non-developer curators

### Current assessment

Appropriate for expert package authoring, not the primary customer profile.

## Option F: Generated visual editor

The package surface generates forms and guided configuration.

### Advantages

- low learning burden
- invalid combinations can be prevented
- authority and budget changes can receive prominent treatment
- simulation and previews can be integrated

### Costs

- requires textual canonical representation for review and version control
- bulk and advanced edits are cumbersome

### Current assessment

Likely primary customer UX over a structured profile representation.

## Option G: Natural-language-only configuration

### Advantages

- minimal interaction cost

### Costs

- ambiguous
- non-deterministic
- difficult to diff and replay
- unsafe for authority changes
- interpretation may drift across model versions

### Current assessment

Not recommended as canonical state.

## Option H: Conversational editor over structured profiles

Natural language proposes deterministic structured edits.

```text
User request
    ↓
candidate profile mutation
    ↓
package-aware validation
    ↓
semantic diff
    ↓
explicit approval
    ↓
canonical profile revision
```

### Current assessment

Recommended interaction surface when paired with a textual canonical profile.

## Semantic Diff

Line diffs are insufficient for consequential profile changes.

A semantic diff should state:

```text
Steward: Payments performance

Objective changes:
- Documentation stewardship enabled.

Sensitivity changes:
- Performance changed from Balanced to Strict.
- Evidence maximum age changed from 24 hours to 2 hours.
- Restore now requires 3 healthy revisions.

Authority changes:
- Draft pull requests are now permitted.
- Merge remains approval-required.
- Production deployment remains prohibited.

Resource changes:
- Daily test budget increased from 60 to 90 minutes.
```

Authority, scope, and budget changes should be separated from ordinary tuning changes.

## Profile Curation Lifecycle

1. Select a package and steward template.
2. Bind a subject scope.
3. Select objectives, sensitivity, autonomy, budget, escalation, and verification.
4. Validate against package semantics, activation capabilities, and organization policy.
5. Preview effective observations, candidate actions, authority, and expected cost from the assembled runtime inputs.
6. Simulate against historical scenarios where possible.
7. Approve and activate.
8. Refine through structured edits, forms, or conversational proposals.

## Advanced Overrides

Packages may expose bounded override points.

Example:

```text
override concern compatibility {
    evidence max_age 2h
    restore after 3 healthy revisions
}
```

Overrides should produce warnings when they leave package-recommended operating envelopes.

Users should not normally configure:

- comparator weights
- posterior parameters
- graph-projection routes
- task-network mechanics
- retry state machines
- actor cadence
- evidence leases
- context serialization

## Recommended Architecture

The current recommendation is a layered hybrid:

```text
Expert package SDK and typed package source
        ↓ exposes curated profile surface
Small structured Steward Profile Language
        ↓ edited through
text + generated GUI + conversational refinement
        ↓ compiles to
assignment and activation plans
        ↓ links to
full stewardship image and domain registrations
```

No source syntax is selected by this proposal.

## Required Experiments

The same steward should be represented through:

- simplified YAML
- HCL-like syntax
- a generated form
- a conversational edit flow

The experiment should test:

- first-time creation
- authority change
- scope refinement
- additional objective
- verification change
- escalation rule
- advanced override
- package upgrade
- semantic diff
- round-trip preservation

See [Evaluation Plan](evaluation_plan.md).
