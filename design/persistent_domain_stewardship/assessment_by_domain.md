# Persistent Domain Stewardship Assessment By Domain

Date: 2026-08-16
Status: historical proposal assessment
Evidence basis: PDS proposal branch plus current cognitive-runtime design and implementation mapping
Scope: candidate integration of PDS across Meld domains

> This artifact preserves the initial proposal breadth map. It is not architecture or implementation authority. Use [Canonical Persistent Domain Stewardship](../cognitive_architecture/persistent_domain_stewardship.md) for the semantic constraint and [Current Architecture Overhauls](../plan/README.md) for the unresolved ownership questions.

## Concern Definition

Persistent Domain Stewardship proposes a user-facing application model for long-lived, evidence-grounded stewards.

The concern spans package and profile declaration, assignment, activation, lineage, standing mandates, observation and intervention configuration, outcome verification, and unified inspection.

The assessment determines which Meld domains require a PDS contract and which must remain uninvolved.

## In Scope

- package and profile representation
- cross-domain linking
- assignment and activation
- domain-owned facet registration
- standing-objective and episode ownership options
- package lineage
- user-facing stewardship projection
- workflow compatibility
- authority and approval requirements

## Out Of Scope

- replacing canonical event storage
- replacing sensory workers
- replacing graph or belief state
- replacing Agent curation
- replacing planning or task execution
- centralizing provider or workspace truth
- forcing PDS integration into every domain

## Integration Levels

This assessment uses the repository policy values:

- `none`
- `observe`
- `publish`
- `consume`
- `own`
- `adapter`

## Domain Assessment

| Domain | Needed integration | Current integration | Completeness | PDS proposal | Non-integration rationale / follow-up |
|---|---|---|---|---|---|
| PDS control plane | own | design only | not started | package, profile, assignment, activation, linking, receipts, lineage, semantic diff, unified projection | Candidate new meta-domain; crate extraction remains open |
| root `meld` | adapter | product assembly already wires all runtime domains | partial | load sources, discover facet connectors, bind concrete adapters, expose CLI/API/views | Must not become PDS semantic authority |
| config | adapter | loads application configuration | partial | locate package/profile/activation sources and organization policy | Parsing location may remain root even if schemas live elsewhere |
| CLI | adapter | none specific | not started | validate, compile, inspect, assign, activate, diff, simulate | Presentation only |
| API | adapter | none specific | not started | package/profile/assignment/activation operations and projections | Must call PDS and domain contracts rather than stores directly |
| views | adapter | none specific | not started | user-facing stewardship read model | View is not domain truth |
| events | publish / consume substrate | canonical ledger exists | partial | carry stewardship lineage and domain lifecycle events | Event crate must not interpret PDS semantics |
| sensory | consume facet | promoted observations exist | partial | validate sensor requirements and activation bindings | PDS does not own polling, lowering, or promotion |
| workspace | optional consume facet | first source-domain implementation | partial | software package may bind workspace identities and observation adapters | Non-software stewards should not depend on workspace |
| branches | optional consume facet | branch-scoped state exists | partial | software profiles may expose branch scope | Not a universal PDS requirement |
| context | optional consume facet | context frames and generation exist | partial | context-projection or hydration facet for model-backed methods | Ownership of projection declarations remains open |
| prompt context | optional consume facet | provider inputs and lineage exist | partial | compile profile/package context policy to prompt-context requirements | Not needed for non-model capabilities |
| provider | adapter | root-owned registry and clients | partial | activation binds provider implementation and quota | Provider is not a stewardship semantic domain |
| world-model graph | consume facet | fixed projection routes exist | partial | validate and register package event-to-graph projection intents | Domain owns projection semantics and graph state |
| belief | consume facet | belief-family configuration exists | strong partial | compile or register epistemic families, evidence mappings, priors, freshness, projection | Belief family must remain separate from steward-specific preference |
| Agent | consume facet; possible objective owner | first threshold curation exists | partial | perspective, concern bindings, standing-objective evaluation, act/tolerate/observe/escalate policy | Objective and episode ownership remain open |
| `meld-lang` | consume shared IR | proposition and method types exist | partial | package and domain compilers lower supported constructs to existing IR | PDS should not add domain-specific grammar by default |
| goals | publish / consume | execution goal records exist | partial | include stewardship lineage and objective/episode references | Goals remain execution-owned transient commitments |
| execution planning | consume facet | method library and planning substrate exist | partial | package-scoped methods, action classes, authority requirements, outcome references | PDS does not own search or plan repair |
| task network | publish / consume | durable graph and lifecycle exist | partial | propagate stewardship lineage; expose status to projection | PDS does not own readiness or continuation |
| capability | consume requirement | typed catalog and contracts exist | partial | resolve package action requirements to installed capabilities | Capability availability is not authority |
| workflow | compatibility / method source | production orchestration exists | partial | wrap workflows as methods, then classify and migrate semantics | Workflow may remain first-class if experiments disprove the migration thesis |
| governance / policy | consume requirements; authority owner outside package | design fragmented | not started | calculate and enforce effective authority, approval, budgets, prohibitions | Package may request authority but cannot grant it |
| session | observe | command lifecycle exists | not started | operator-facing linkage only if needed | Stewardship lifecycle should not replace session lifecycle |
| concurrency | consume requirement | shared limits exist | partial | activation and profile budgets may compile to runtime resource claims | Infrastructure remains authoritative |
| store | none / infrastructure | persistence primitives exist | not needed | PDS and domain stores use ordinary persistence contracts | Store layer should not interpret stewardship meaning |
| telemetry | observe | runtime status exists | partial | package and activation diagnostics, actor health, cost summaries | Telemetry is not stewardship truth |
| logging | observe | existing | not needed | include package and assignment identifiers where useful | No PDS semantic ownership |
| metadata | optional publish / consume | product metadata exists | open | packages may refer to domain metadata through explicit facets | Avoid turning generic metadata into a hidden package registry |
| ignore | none | path-selection infrastructure | not needed generally | software source adapters may consume it indirectly | No direct PDS contract |
| merkle traversal | none / capability implementation | workspace-specific traversal | not needed generally | software observation or context capability may reference it | Must not become global PDS concept |
| heads | none / compatibility | legacy compatibility | not needed | no new integration | Explicit non-integration |
| types / lib exports | adapter | crate surfaces exist | open | export only stable PDS contracts after model validation | Avoid premature public API freeze |

## Candidate Ownership Summary

### PDS-owned

- package/profile/assignment/activation identity
- linked facet inventory
- package and activation lifecycle
- registration receipts
- semantic profile diff
- package upgrade coordination
- stewardship lineage identifiers
- unified stewardship projection

### Domain-owned

- observation promotion
- event facts
- graph projections
- evidence and belief semantics
- perspective and normative decisions
- goals and planning
- task and capability execution
- outcome evidence meaning
- authority grants and enforcement

### Root-adapter-owned

- source loading
- concrete connector discovery
- credentials and environment binding
- CLI/API/presentation
- concrete domain port implementations

## High-Risk Boundaries

### PDS and belief

Risk: belief-family schemas move into PDS and become centrally authoritative.

Candidate boundary: belief owns the facet schema and compiler; PDS links family exports to Agent concern imports.

### PDS and Agent

Risk: PDS evaluates objective breach and restore independently of Agent policy.

Candidate boundary: Agent owns normative evaluation; PDS owns declaration lineage and may project lifecycle.

### PDS and execution

Risk: PDS becomes a second planner or workflow runtime.

Candidate boundary: execution facet registers methods and requirements; PDS only coordinates activation and lineage.

### PDS and governance

Risk: package-requested authority becomes effective authority.

Candidate boundary: external or runtime policy grants authority; planning filters and dispatch enforces.

### PDS and source domains

Risk: a universal object model erases source authority.

Candidate boundary: domain facets export stable symbols and mappings while source systems remain authoritative.

## Domains With Explicit Non-Integration

The following should not receive direct PDS semantics unless a later concrete use case demonstrates need:

- persistence primitives
- logging implementation
- generic error mapping
- ignored-path infrastructure
- legacy head compatibility
- low-level Merkle traversal

These may be used by participating adapters without becoming PDS facets.

## Follow-Up Requirements

1. Define a candidate stewardship facet protocol.
2. Decide whether objective and episode state are authoritative or projected.
3. Add package/profile/assignment/activation separation.
4. Define package lineage propagation through Agent, goal, task, and outcome boundaries.
5. Test whether a new non-software domain can integrate without modifying PDS core types.
6. Repeat this assessment when implementation creates or extracts a PDS domain.

## Assessment Conclusion

PDS can remain true to Meld's domains if it is constrained to control-plane composition and inspection.

The federated-facet option currently provides the clearest ownership model, but root-only composition remains a credible first implementation.

A central PDS schema that directly owns foreign domain semantics remains an explored option rather than the assumed target.
