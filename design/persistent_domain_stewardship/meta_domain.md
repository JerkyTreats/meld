# PDS Meta-Domain

Date: 2026-07-16  
Status: proposed  
Scope: options for integrating Persistent Domain Stewardship across Meld while preserving domain authority

## Concern

Persistent Domain Stewardship is cross-cutting by construction. A configured steward may involve:

- sensory promotion
- event publication
- graph projection
- belief assessment
- Agent perspective and curation
- planning and goals
- task networks and capabilities
- outcome evidence
- authority and approval
- user-facing lifecycle and inspection

Cross-cutting scope does not imply cross-cutting authority.

The design problem is to provide one coherent PDS feature set without creating a super-domain that owns the semantics and state of every participating domain.

## Constraint: To Each Domain Be True

PDS must preserve these ownership rules:

- events own canonical append, sequencing, and replay
- source domains own source truth and promoted observation semantics
- the world model owns graph materialization, evidence, beliefs, and planner projection
- the Agent domain owns perspective and normative judgment
- execution owns goals, planning, task networks, dispatch, and outcome publication
- capabilities own typed executable behavior
- root `meld` owns application assembly and concrete adapters

PDS may reference, compose, and project these domains. It must not replace their authority.

## Candidate PDS Source Truth

A PDS meta-domain could truthfully own:

- stewardship package identity and version
- customer profile identity and revision
- assignment identity and lifecycle
- activation identity and lifecycle
- linked domain-facet inventory
- package imports and cross-domain symbolic links
- facet preparation and activation receipts
- package, profile, assignment, and activation hashes
- stewardship lineage identifiers
- package upgrade coordination
- semantic profile diffs
- unified stewardship inspection projections

It should not own:

- observations
- graph anchors
- belief revisions
- act-versus-tolerate decisions
- execution goals
- task state
- capability results
- domain outcome interpretation
- authority grants

## Option A: Central Package-Schema Meta-Domain

PDS centrally defines source and compiled schemas for domain vocabulary, observations, beliefs, charters, actions, outcomes, governance, and scenarios.

### Advantages

- one compiler and validation model
- coherent package authoring experience
- straightforward canonical package representation
- simple documentation and tooling story

### Costs

- PDS must understand every participating domain's internal configuration model
- domain schema changes require PDS schema changes
- central types may become the de facto authority for belief, Agent, and execution semantics
- extracted domains risk depending on PDS or sharing PDS-owned abstractions
- one package schema may flatten meaningful domain differences

### Status

Existing `package_model.md` substantially explores this option.

It remains useful as a complete operational-domain-theory model, but it should not be treated as the only integration architecture.

## Option B: Federated Stewardship Facets

Each participating domain owns one or more stewardship facet schemas, validators, compilers, and activation behavior.

PDS owns the package manifest, profile surface, linking, assignment, activation coordination, receipts, and inspection.

```text
PDS package
    ├── sensory facet
    ├── world-model facet
    ├── Agent facet
    ├── execution facet
    ├── governance facet
    └── package/profile metadata
```

PDS links opaque compiled facets through declared exports, imports, requirements, and lineage.

### Advantages

- domain ownership remains explicit
- domain validation evolves with the owning domain
- PDS core avoids importing domain internals
- new domains can add facets without extending a universal PDS schema
- connectors provide a clear integration boundary

### Costs

- cross-domain linking and version coordination become more complex
- package diagnostics may span several domain compilers
- common authoring concepts require a stable profile-surface abstraction
- activation becomes a durable multi-domain saga rather than one transaction

### Current recommendation

This is the current preferred direction because it aligns most directly with Meld's domain-isolation rules.

It remains a recommendation pending implementation experiments.

## Option C: Root Product Composition Only

PDS remains a root `meld` application concern.

Root configuration and adapters directly register belief families, Agents, methods, capabilities, and event routes without a distinct PDS control-plane domain.

### Advantages

- minimal new architecture
- direct access to current application wiring
- appropriate for an initial proof
- avoids premature crate extraction

### Costs

- package identity and lifecycle may remain implicit
- PDS behavior can become scattered across adapters
- root `meld` may accumulate domain-specific assembly branches
- inspection and upgrades become difficult to standardize
- the customer-facing stewardship model may lack a stable source truth

### Status

This is a plausible first-slice implementation strategy even if the long-term design uses federated facets.

## Option D: PDS As Runtime Orchestrator

PDS directly runs the observe, believe, curate, plan, execute, and verify sequence.

### Advantages

- one visible loop
- simple conceptual ownership

### Costs

- duplicates cognitive-runtime responsibilities
- centralizes foreign domain state
- conflicts with current crate boundaries
- makes PDS a second planner and execution coordinator

### Status

Rejected as a target architecture.

PDS may coordinate activation and present a unified lifecycle, but the cognitive data plane remains domain-owned.

## Recommended Layering

```text
User interaction
    Steward Profile Language / GUI / conversational editor
        ↓
PDS control plane
    package, profile, assignment, activation, linking, lifecycle
        ↓
Stewardship facet protocol
    domain-owned compile, prepare, activate, inspect
        ↓
Cognitive runtime data plane
    sensory, events, graph, belief, Agent, execution, capabilities
        ↓
PDS stewardship projection
    correlated user-facing status and history
```

## PDS Control Plane

The candidate control plane owns:

```text
Package
Profile
Assignment
Activation
CompiledStewardshipImage
FacetReceipt
StewardshipContext
UpgradePlan
StewardshipProjection
```

It coordinates registration through domain connectors.

It must not mutate domain stores directly.

## Cognitive Data Plane

After activation, normal runtime operations should not synchronously route through PDS.

```text
observation
→ event
→ graph and belief
→ Agent decision
→ execution
→ outcome event
```

PDS receives domain events or queries domain projections to build the user-facing stewardship view.

## Stewardship Projection

One open question is whether an episode is:

- authoritative PDS state;
- an Agent-owned lifecycle projected by PDS;
- a PDS correlation view over belief, Agent, goal, task, and outcome events.

The current recommendation is to treat the cross-domain episode as a projection unless implementation evidence establishes a distinct PDS-owned coordination state.

The projection may show:

- assignment and objective
- current belief and uncertainty
- open investigation
- proposed or approved intervention
- active goal and task work
- verification state
- restoration, tolerance, failure, or escalation

The underlying domain records remain authoritative.

## Cross-Domain References

PDS should use stable identifiers and lineage rather than shared mutable objects.

Candidate shared context:

```rust
struct StewardshipContext {
    package_hash: PackageHash,
    profile_id: ProfileId,
    assignment_id: AssignmentId,
    activation_id: ActivationId,
    charter_id: Option<CharterId>,
    concern_id: Option<ConcernId>,
    objective_id: Option<ObjectiveId>,
    episode_id: Option<EpisodeId>,
}
```

This context may be attached at domain boundaries such as commands, events, execution-goal wrappers, task lineage, approval requests, and outcome evidence.

It should not be copied into every internal object.

## Integration Invariant

For every PDS concept, the proposal should identify exactly one of:

- PDS source truth
- participating-domain source truth
- PDS projection of domain truth
- root adapter
- external source truth

No concept should be simultaneously authoritative in PDS and a participating domain.

## Open Questions

- Is PDS substantial enough to justify an extracted crate?
- Which domain facets are generic enough to share a protocol?
- Does each domain compile its own facet or only validate registrations produced by PDS?
- Who owns package scenario execution?
- Is the stewardship episode authoritative or projected?
- How are partial activation and compensation represented?
- How much domain detail may appear in a customer-facing semantic diff?

See [Facet Protocol](facet_protocol.md), [Assessment By Domain](assessment_by_domain.md), and [Open Decisions](open_decisions.md).