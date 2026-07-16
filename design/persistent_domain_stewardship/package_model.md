# Stewardship Package Model

Date: 2026-07-16  
Status: proposed option  
Scope: one candidate declarative source representation, compilation, loading, and lifecycle model for persistent domain stewardship packages

> This document explores a central package-schema option. It is not an accepted implementation contract. The competing federated-facet model is described in [PDS Meta-Domain](meta_domain.md) and [Stewardship Facet Protocol](facet_protocol.md). Read [Proposal Status And Decision Semantics](proposal_status.md) before interpreting normative language below.

## Thesis

A stewardship package may be represented as a versioned, declarative operational domain theory.

Under the central-schema option, it tells Meld:

```text
what exists
what can be observed
what can be believed
what should be maintained
what can be done
how results are verified
what authority constrains action
```

It does not tell Meld how to persist events, revise beliefs, schedule tasks, retry work, or execute capabilities. Those are cognitive-runtime responsibilities.

```text
package source
    ↓ compile
CompiledStewardshipPackage
    ↓ combine with profile, assignment, and activation
CompiledStewardshipImage
    ↓ register with existing domain authorities
Meld cognitive runtime
```

The package is one candidate declarative program model. `meld-lang` remains the shared runtime intermediate representation. Meld remains the persistent machine.

## Competing Integration Models

This document gives detailed form to **Option A: central PDS package schema**.

The proposal also considers:

- **Option B: federated domain-owned facets**, where each domain owns its facet schema, compiler, validation, activation, and inspection;
- **Option C: root product composition**, where an initial implementation directly coordinates existing domain APIs without a durable PDS meta-domain.

The current recommendation is to test the federated-facet option while permitting a root-composed first proof.

The source structures below should therefore be read as:

- a completeness checklist for operational domain theory;
- one candidate package-authoring representation;
- input to comparison experiments;
- not a final ownership decision.

## Representation Layers

The design distinguishes five conceptual representations.

### Source package

Expert-authored domain theory or a linked set of domain-owned facet sources.

The initial source format could be typed YAML or JSON backed by serializable structures. A custom language is deferred.

### Steward profile

Customer-authored intent selecting package-defined scope, objectives, sensitivity, autonomy, budget, escalation, and verification.

See [Steward Profile Abstraction](profile_abstraction.md).

### Compiled stewardship package

A canonical, content-addressed semantic package produced by a central compiler or a linker of compiled facets.

Candidate contents:

- resolved imports and symbols;
- normalized object, relation, dimension, artifact, and unit declarations;
- validated observation and evidence routes;
- compiled belief-family registrations;
- steward templates and objective templates;
- lowered `meld-lang` propositions, operators, effects, and methods;
- outcome-verification routes;
- authority requirements and governance constraints;
- package provenance and conformance-test results.

The current recommendation is that only validated, versioned package semantics are activated. The exact compilation and validation ownership remains open.

### Stewardship assignment and activation

An assignment binds profile semantics to a principal, scope, and authority grant.

An activation binds the assignment to physical sensors, connectors, credentials, providers, capability implementations, runtime placement, and quotas.

A first implementation may combine these records.

### Runtime state

Live state created after activation:

- concrete subject scope;
- observations and facts;
- graph anchors;
- evidence and belief revisions;
- objective and episode state or projections;
- goals and task-network state;
- execution attempts;
- approvals;
- measured outcomes;
- learned cost and value beliefs.

Domain runtime state remains owned by the relevant domains and refers back to package, profile, assignment, and activation lineage where required.

## Central Package Composition Option

One candidate source bundle composes independent sections:

```text
StewardshipBundle
├── manifest and imports
├── profile surface
├── domain modules
├── observation modules
├── belief modules
├── steward charters
├── context projections
├── action modules
├── outcome modules
├── governance modules
└── scenario modules
```

Conceptual Rust shape:

```rust
struct StewardshipBundleSpec {
    manifest: PackageManifestSpec,
    imports: Vec<PackageImportSpec>,
    profile_surface: ProfileSurfaceSpec,
    domains: Vec<DomainModuleSpec>,
    observations: Vec<ObservationModuleSpec>,
    beliefs: Vec<BeliefModuleSpec>,
    charters: Vec<StewardCharterSpec>,
    context_projections: Vec<ContextProjectionSpec>,
    actions: Vec<ActionModuleSpec>,
    outcomes: Vec<OutcomeModuleSpec>,
    governance: Vec<GovernanceModuleSpec>,
    scenarios: Vec<ScenarioModuleSpec>,
}
```

Under the federated-facet option, these sections become domain-owned facet payloads linked through imports and exports rather than centrally owned PDS types.

The exact source schema is intentionally not frozen.

## Manifest And Imports

The manifest establishes package identity and compatibility.

```rust
struct PackageManifestSpec {
    package_id: PackageId,
    version: SemanticVersion,
    schema_version: SchemaVersion,
    description: String,
    runtime_compatibility: RuntimeCompatibility,
}
```

Imports are explicit and version-constrained:

```yaml
imports:
  - package: software.core
    version: "^1.2"
  - package: software.git-observations
    version: "1.0.3"
  - package: software.task-capabilities
    version: "^2"
```

A candidate compiler or linker resolves imports to exact content hashes. Floating imports are not recommended for active runtime use.

A compiled package should record:

- source package id and version;
- exact imported package or facet hashes;
- compiler/linker version;
- runtime schema version;
- canonical package hash.

## Profile Surface

The package should expose a smaller public profile API rather than requiring customers to edit the full source.

Candidate surface:

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

The package defines what presets such as `strict`, `balanced`, or `release_grade` mean.

PDS provides consistent profile structure and tooling rather than universal domain meanings.

## Domain Module

Under the central-schema option, a domain module defines declarative vocabulary rather than current state.

```rust
struct DomainModuleSpec {
    module_id: ModuleId,
    object_types: Vec<ObjectTypeSpec>,
    relation_types: Vec<RelationTypeSpec>,
    attribute_types: Vec<AttributeTypeSpec>,
    artifact_types: Vec<ArtifactTypeSpec>,
    belief_dimensions: Vec<BeliefDimensionSpec>,
    scope_types: Vec<ScopeTypeSpec>,
}
```

Under the federated-facet option, the owning domain exports these symbols through its facet compiler.

### Object types

An object type may declare:

- stable type id;
- identity strategy;
- optional external system of record;
- lifecycle semantics;
- allowed attributes;
- default security classification.

```yaml
object_types:
  - id: software.module
    identity:
      strategy: external_key
      source: workspace.node_id
    lifecycle:
      created_by: sensory.workspace.node_discovered
      retired_by: sensory.workspace.node_removed
```

### Relation types

```yaml
relation_types:
  - id: software.depends_on
    from: software.module
    to: software.module
    cardinality: many_to_many
    temporal: bitemporal
```

The runtime may continue to encode identifiers as strings. Package or facet compilation can supply authoring-time type validation.

### Attributes and units

```yaml
attributes:
  - id: performance.latency_p95
    subject: software.service
    value_type: duration
    unit: millisecond
```

A candidate compiler should reject incompatible unit comparisons.

### Belief dimensions

```yaml
belief_dimensions:
  - id: test.health
    subject: software.module
    value_type: probability
    semantics: probability_healthy
```

The dimension does not define evidence or inference. Those remain belief-domain semantics.

### Artifact types

```yaml
artifact_types:
  - id: software.test_report
    schema: schemas/test_report.v1.json
```

## Observation Module

Observation declarations bind promoted sensory events to domain semantics.

```rust
struct ObservationModuleSpec {
    module_id: ModuleId,
    required_sensors: Vec<SensorRequirementSpec>,
    event_schemas: Vec<EventSchemaSpec>,
    fact_projections: Vec<FactProjectionSpec>,
    graph_projections: Vec<GraphProjectionSpec>,
    evidence_mappings: Vec<EvidenceMappingSpec>,
}
```

The declarations may specify:

- promoted observation schema;
- subject extraction and identity resolution;
- fact and graph projection;
- belief evidence mapping;
- provenance and security propagation;
- event-time and freshness semantics.

Raw high-volume signals remain inside sensory lanes.

Under the facet option, source, graph, and belief domains may own separate portions of this declaration rather than one central observation module.

## Belief Module

Belief declarations define epistemic concern families.

```rust
struct BeliefFamilySpec {
    family_id: BeliefFamilyId,
    subject_type: ObjectTypeId,
    dimension: BeliefDimensionId,
    question: String,
    evidence_schemas: Vec<EvidenceSchemaSpec>,
    source_mappings: Vec<EvidenceSourceMappingSpec>,
    comparator: ComparatorBindingSpec,
    prior: PriorSpec,
    freshness: FreshnessPolicySpec,
    conflict: ConflictPolicySpec,
    planner_projection: PlannerProjectionSpec,
}
```

A belief family answers:

```text
What should this perspective currently believe about this subject and dimension?
```

It does not answer:

```text
Does this steward want the state to change?
```

### Epistemic and normative separation

```text
BeliefFamilySpec
    epistemic semantics shared by consuming perspectives

StewardConcernBinding
    objective, tolerance, inaction cost, observation policy,
    action classes, and value policy for one charter
```

This permits several profiles to consume the same belief family differently.

The belief domain remains the candidate owner of family schema and validation under a federated model.

## Steward Charter

A charter is a reusable stewardship-role declaration.

```rust
struct StewardCharterSpec {
    charter_id: CharterId,
    perspective: PerspectiveProfileSpec,
    scope: ScopeTemplateSpec,
    concerns: Vec<StewardConcernBindingSpec>,
    lifecycle: StewardLifecyclePolicySpec,
    authority_requirements: AuthorityRequirementSpec,
    escalation: EscalationPolicySpec,
}
```

The current candidate lowering target is the world-model Agent domain.

### Concern binding

```rust
struct StewardConcernBindingSpec {
    concern_id: ConcernId,
    belief_family: BeliefFamilyId,
    objective: StewardshipObjectiveSpec,
    act_tolerate_policy: ActToleratePolicySpec,
    observation_policy: ObservationPolicySpec,
    action_classes: Vec<ActionClassId>,
    cost_belief: Option<BeliefFamilyId>,
    value_belief: Option<BeliefFamilyId>,
}
```

### Standing objective

```rust
struct StewardshipObjectiveSpec {
    objective_id: ObjectiveId,
    desired: Proposition,
    breach: Proposition,
    restore: Proposition,
    evidence_freshness: Option<Duration>,
    hysteresis: Option<HysteresisPolicy>,
    stability_window: Option<StabilityWindow>,
    inaction_cost: Option<InactionCostPolicy>,
}
```

`desired`, `breach`, and `restore` may lower to `meld-lang::Proposition`.

Objective declaration may belong to the package/PDS control plane while runtime evaluation belongs to Agent. Final ownership remains open.

### Directive handling

Free-form directives should not silently create live domain semantics.

Recommended flow:

```text
directive
    ↓
select a declared package and steward template
    ↓
propose a structured profile
    ↓
validate scope and authority
    ↓
create assignment and activation
```

## Stewardship Assignment And Activation

Candidate assignment:

```rust
struct StewardshipAssignment {
    assignment_id: AssignmentId,
    package_hash: PackageHash,
    profile_id: ProfileId,
    charter_id: CharterId,
    principal: PrincipalRef,
    scope: ScopeBinding,
    effective_authority: EffectiveAuthorityRef,
    lifecycle: AssignmentLifecycle,
}
```

Candidate activation:

```rust
struct StewardshipActivation {
    activation_id: ActivationId,
    assignment_id: AssignmentId,
    source_bindings: Vec<SourceBinding>,
    capability_bindings: Vec<CapabilityBinding>,
    provider_bindings: Vec<ProviderBinding>,
    runtime_placement: RuntimePlacement,
    quotas: Vec<QuotaBinding>,
    lifecycle: ActivationLifecycle,
}
```

Whether these remain separate records in the first implementation is open.

## Stewardship Episode

The proposal needs a user-facing episode concept, but ownership is unresolved.

Candidate record:

```rust
struct StewardshipEpisode {
    episode_id: EpisodeId,
    assignment_id: AssignmentId,
    objective_id: ObjectiveId,
    opened_at_seq: u64,
    trigger_refs: Vec<DomainObjectRef>,
    active_goal_ids: Vec<GoalId>,
    status: EpisodeStatus,
    closed_at_seq: Option<u64>,
    closure: Option<EpisodeClosure>,
}
```

Options include:

- authoritative PDS record;
- Agent-owned lifecycle;
- PDS projection over domain events;
- split coordination record and detailed projection.

The current recommendation is to begin with projection unless explicit coordination state proves necessary.

## Context Projection

A package may need to declare bounded context hydration for model-backed capabilities.

Candidate concept:

```rust
struct ContextProjectionSpec {
    projection_id: ContextProjectionId,
    subject_selector: SubjectSelector,
    graph_traversal: GraphTraversalSpec,
    belief_families: Vec<BeliefFamilyId>,
    perspective: PerspectiveRef,
    evidence_policy: ContextEvidencePolicy,
    ranking: ContextRankingPolicy,
    retention: ContextRetentionPolicy,
    output_artifact: ArtifactTypeId,
}
```

Ownership remains open among PDS, the world model, context, and capability input binding.

## Action Module

Action declarations define the operational vocabulary available to planning.

```rust
struct ActionModuleSpec {
    module_id: ModuleId,
    operators: Vec<OperatorSpec>,
    methods: Vec<MethodSpec>,
    capability_requirements: Vec<CapabilityRequirementSpec>,
    resource_claims: Vec<ResourceClaimSpec>,
    compensation: Vec<CompensationSpec>,
}
```

Operators and methods lower to existing `meld-lang` and execution contracts.

Current workflows may be imported as compatibility methods.

Capability availability remains separate from authority.

## Outcome Module

Planner effects are predictions. Outcome declarations specify how real effects become evidence.

```rust
struct OutcomeContractSpec {
    outcome_id: OutcomeId,
    action_class: ActionClassId,
    expected_effects: Vec<Effect>,
    verification_observations: Vec<ObservationRequirementSpec>,
    evaluation_window: EvaluationWindowSpec,
    success: Proposition,
    partial_success: Option<Proposition>,
    failure: Proposition,
    harmful: Option<Proposition>,
    evaluator: EvaluatorBindingSpec,
    attribution: AttributionPolicySpec,
}
```

The current recommendation is to compile outcome declarations into domain-owned evidence mappings and Agent satisfaction inputs rather than a new PDS outcome engine.

## Governance Module

Governance declarations request authority and constraints.

```rust
struct GovernanceModuleSpec {
    module_id: ModuleId,
    authority_classes: Vec<AuthorityClassSpec>,
    approval_policies: Vec<ApprovalPolicySpec>,
    budgets: Vec<BudgetPolicySpec>,
    rate_limits: Vec<RateLimitPolicySpec>,
    prohibitions: Vec<ProhibitionSpec>,
    audit_requirements: Vec<AuditRequirementSpec>,
}
```

Effective authority is proposed as:

```text
package support/request
∩ profile request
∩ principal grant
∩ organization/runtime policy
∩ current restrictions
```

Planning may filter on authority and dispatch should enforce it independently.

## Scenario Module

Scenarios are candidate package-level conformance tests.

Required scenario classes may include:

- nominal restoration;
- tolerance;
- insufficient or contradictory evidence;
- denied authority;
- unavailable capability;
- failed or harmful action;
- external restoration;
- concurrent environmental change;
- exact-hash replay;
- package upgrade.

Whether PDS owns a generic scenario language or links domain harnesses remains open.

## Candidate Compiler Or Linker

Under the central option:

```text
parse
→ resolve imports
→ resolve symbols
→ type and schema check
→ semantic validation
→ authority validation
→ lower to canonical IR
→ run scenarios
→ hash
```

Under the federated option:

```text
parse package/profile
→ route facet sources to domain compilers
→ collect compiled facet envelopes
→ link exports and imports
→ resolve profile presets
→ validate cross-domain requirements
→ run linked scenarios
→ hash compiled image
```

Candidate static failures include unresolved symbols, incompatible versions, missing evidence paths, unavailable capability requirements, absent authority requests, or outcome declarations without verification.

The final failure set belongs to the selected architecture and owning domains.

## Runtime Registration

Loading a compiled image should coordinate registration with existing authorities rather than reimplement them.

```text
observation requirements
    → sensory/source adapters

graph and evidence facets
    → world-model domains

charter and concern facets
    → Agent

methods and capability requirements
    → execution

outcome routes
    → events/world model/Agent

governance requirements
    → policy and execution enforcement
```

The package loader or facet connectors should not write live beliefs, goals, graph anchors, or task outcomes directly.

## Upgrade

A running assignment should not silently follow mutable package semantics.

Candidate upgrade flow:

```text
compile new package/image
→ semantic and authority diff
→ domain migration preparation
→ approval where required
→ sequence-bound activation
→ preserve old hashes for replay
```

Major incompatible changes may create a new assignment rather than update an existing one.

## Source Syntax

No source syntax is selected.

Options include:

- typed package YAML or JSON;
- domain package SDKs;
- CUE or similar expert composition;
- federated facet files;
- generated package authoring tools.

Customer profiles are addressed separately in [Steward Profile Abstraction](profile_abstraction.md).

## Suggested Code Routing

One candidate trajectory:

```text
root meld first slice
    package/profile loading and concrete connector wiring

possible meld-stewardship
    package, profile, assignment, activation, linking, receipts, projection

domain crates
    facet schemas, validation, registration, runtime behavior
```

No extracted crate should depend back on root `meld`.

Immediate crate extraction is not recommended until two package proofs establish stable contracts.

## Inspectability

Candidate user-facing operations include:

```text
meld stewardship package validate
meld stewardship package inspect
meld stewardship profile diff
meld stewardship assignment inspect
meld stewardship activation inspect
meld stewardship episode explain
```

Inspection should distinguish PDS-owned identity and projection from authoritative domain state.

## Non-Goals

This package model is not:

- an accepted final schema;
- a universal ontology;
- a graph-query language;
- an arbitrary programming language;
- a replacement for domain compilers or adapters;
- a place to store live beliefs or tasks;
- a direct authority grant;
- a prompt-orchestration format;
- a mechanism for bypassing `meld-lang`;
- a requirement that every domain use Bayesian inference.

## Evaluation

The central schema and federated-facet options should be compared through the experiments in [Evaluation Plan](evaluation_plan.md).

Open ownership and representation decisions are tracked in [Open Decisions](open_decisions.md).
