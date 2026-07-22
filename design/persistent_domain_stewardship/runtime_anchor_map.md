# Runtime Anchor Map

Date: 2026-07-16  
Status: proposed analysis  
Runtime evidence branch: `runtime-operator-visibility`  
Scope: mapping PDS concepts to the implemented cognitive-runtime surfaces

## Purpose

The PDS proposal was initially developed from architecture documents.

The `runtime-operator-visibility` branch implements the major contracts required for a durable cognitive flywheel. This document maps proposed PDS concepts to those code anchors.

The mapping is not a quality audit and does not claim that the branch is fully assembled as an autonomous production runtime.

It identifies:

- reusable existing contracts
- specialized first-slice implementations
- missing PDS concepts
- likely package or facet compilation targets
- early specializations that could restrict future steward packages

## Closure Levels

The branch demonstrates three different forms of closure.

| Closure | Assessment |
|---|---|
| Domain contract closure | Most observe, belief, Agent, goal, planning, task-network, and publication contracts exist |
| Durable integration closure | Integration tests can drive the end-to-end sequence and prove reopen behavior |
| Autonomous semantic actor closure | Incomplete; root runtime assembly still requires generic actor activation beyond the first concrete handles |

PDS should target the existing contracts rather than introduce parallel runtime mechanisms.

## End-To-End Runtime

```text
source observation
    ↓
canonical event ledger
    ↓
graph reduction
    ↓
evidence ingestion and belief revision
    ↓
planner-facing world state
    ↓
Agent curation
    ↓
execution goal
    ↓
method selection and planning
    ↓
composition lowering
    ↓
task-network dispatch
    ↓
capability execution
    ↓
outcome publication
    ↓
evidence ingestion
    ↓
belief revision and satisfaction curation
```

## Anchor Matrix

| PDS concept | Runtime anchor | Current character | PDS implication |
|---|---|---|---|
| Canonical event history | `crates/meld-events/src/events/*` | generic and durable | reuse unchanged; add neutral stewardship lineage only where useful |
| Domain object references | `crates/meld-events/src/events/contracts.rs` | string-backed neutral identity | candidate package-level type validation over existing wire contracts |
| Observation capability | `crates/meld-execution/src/capability/*` plus root adapters | generic capability contracts; source adapters remain concrete | observation facets should resolve to existing capabilities or sensory adapters |
| Graph projection | `crates/meld-world-model/src/world_state/graph/*` | generic store and reducer with specialized event interpretation | package/facet route registration should replace fixed source-intent matches over time |
| Belief family | `crates/meld-world-model/src/belief/contracts.rs` and runtime | close to declarative PDS target | domain belief facet can compile directly into existing family configuration |
| Evidence ingestion | `crates/meld-world-model/src/belief/ingestion.rs` | generic durable path | outcome and observation facets should produce promoted evidence through this path |
| Belief assessment | `crates/meld-world-model/src/belief/runtime.rs` | generic runtime | no PDS belief engine required |
| Planner world state | `crates/meld-world-model/src/planner/*` and `meld-lang::WorldState` | shared typed read boundary | stewardship objectives and methods should lower to this language |
| Agent identity and subscription | `crates/meld-world-model/src/agent/contracts.rs`, `registration.rs`, `subscription.rs` | first-slice subject- and belief-bound records | assignments may materialize one or more existing Agent records |
| Agent curation | `crates/meld-world-model/src/agent/curation.rs` and `runtime.rs` | threshold-based first policy | candidate pluggable curation-policy facet; threshold remains first implementation |
| Execution goal | `crates/meld-execution/src/goals/*` | durable execution-owned commitment | retain; add stewardship lineage in wrapper metadata rather than changing goal semantics immediately |
| Method library | `crates/meld-execution/src/planning/method_library.rs` | loadable methods with source provenance | package-scoped method namespace and hash are candidate additions |
| Planning runtime | `crates/meld-execution/src/planning/runtime.rs` | bounded planner actor | reuse; PDS activates and scopes method inventory rather than owning planning |
| Composition lowering | `crates/meld-execution/src/planning/lowering.rs` | maps methods to task-network mutations | package compiler must validate the currently executable `meld-lang` subset |
| Task network | `crates/meld-execution/src/task_network/*` | durable task graph, claims, outcomes, publications | reuse; propagate stewardship lineage and projection status |
| Capability contract | `crates/meld-execution/src/capability/contracts.rs` | typed bindings, artifacts, side effects, execution classes | action facets should import capability requirements rather than embed implementations |
| Task outcome publication | `crates/meld-execution/src/task_network/publication.rs` | generic execution event publication | outcome verification should consume events through domain-owned evidence mappings |
| Workflow compatibility | `crates/meld-execution/src/workflow/*` and task package support | mature production orchestration | candidate workflows-as-methods migration path; not yet a decided endpoint |
| Runtime supervisor | root runtime and integration design under `design/plan/integration/*` | generic lifecycle with incomplete semantic actor assembly | activation should instantiate generic runtime actors, not profile-specific kinds |
| Full docs-freshness proof | `design/plan/integration/docs_freshness_*` and integration tests | implicit package assembled in code and fixtures | first migration target for PDS package/profile compilation |

## Implicit Documentation Steward Package

The implemented docs-freshness proof already distributes package-like declarations across several locations.

### Package identity and constants

Integration fixture constants define:

- Agent identity
- belief dimension and predicate
- evidence policy
- action threshold
- method identity
- artifact identity
- publication event identity
- task-network and worker identity

### Belief facet

The fixture supplies a declarative family containing:

- graph and content evidence schemas
- source mappings
- Bayesian comparator factors
- prior
- freshness
- planner projection

### Agent facet

Agent registration and curation configuration supply:

- subject
- branch
- perspective
- directive
- subscription
- action threshold
- goal priority and source

### Execution facet

The fixture constructs:

- capability catalog entries
- one `refresh_docs` method
- operator preconditions and effects
- artifact and resolution criteria

### Outcome facet

A docs-specific replay adapter interprets successful task events as promoted documentation evidence and invokes the generic belief path.

### Scenario facet

Integration and reopen tests provide imperative conformance scenarios.

PDS should consolidate and compile these declarations without replacing the underlying runtimes.

## Missing PDS Anchors

The branch does not provide explicit runtime records for:

- package identity and package hash
- customer profile and profile revision
- stewardship assignment
- physical activation
- domain-facet receipts
- standing stewardship objective
- recurring stewardship episode
- package-scoped context projection
- effective authority calculation
- cross-domain stewardship lineage
- semantic profile diff
- unified stewardship projection

These are the principal proposed PDS additions.

## Specialized Anchors That Need Generalization

## Fixed graph source interpretation

Current graph reduction recognizes a bounded set of source events.

Candidate trajectory:

```text
package/domain facet
→ validated event-to-graph projection plan
→ existing graph reducer and store
```

Do not move graph truth into PDS.

## Docs-specific outcome evidence replay

The first proof contains a docs-specific execution-event-to-evidence adapter.

Candidate trajectory:

```text
outcome facet
→ domain-owned event/artifact-to-evidence mapping
→ existing belief ingestion
```

Avoid one root adapter per steward profile.

## Workspace requirement in product execution context

Current product execution wiring includes workspace-specific dependencies.

Candidate trajectory:

- capability-specific ports;
- installed domain-adapter registries;
- activation-selected source adapters.

Non-software steward packages should not require workspace types.

## Fixed belief-context bundle

The implemented generation read path is specialized around workspace subjects and docs freshness.

Candidate PDS addition:

```text
ContextProjectionSpec
    subject selector
    graph traversal
    belief families
    perspective and branch
    provenance inclusion
    ranking and retention
    item and byte budgets
    output artifact schema
```

Ownership remains open between the world-model facet, capability input binding, and a separate PDS package concept.

## Threshold coupling

Belief projection and Agent action thresholds are aligned in the first proof.

PDS should permit multiple charters to consume one belief family using different:

- breach thresholds
- restore thresholds
- uncertainty tolerances
- observation policies
- inaction costs
- authority postures

## Method provenance

The method library records method source information but does not carry package namespace or import identity.

Candidate addition:

- package hash
- facet or module identity
- method version
- profile/assignment visibility
- authority class
- outcome-contract reference

## Runtime Actor Assembly

The runtime supervisor design enumerates generic flywheel actors, but the first branch does not yet provide concrete semantic handles for every actor.

PDS activation should eventually:

```text
compiled image
+ assignment
+ activation
    ↓
owner-scoped registrations
    ↓
generic actor instances
```

Do not create runtime kinds such as `docs_freshness_planner` or `performance_agent`.

## Proposed Package Lowering Targets

| Package/profile declaration | Candidate receiving domain |
|---|---|
| sensor and observation requirement | sensory or source adapter |
| event-to-graph mapping | world-model graph |
| evidence schema and family | belief |
| perspective and concern binding | Agent |
| standing-objective policy | Agent or PDS/Agent split; open |
| method and operator | execution planning |
| capability requirement | execution capability catalog |
| context projection | world model/context/capability boundary; open |
| outcome evidence mapping | source or world-model evidence facet |
| authority requirement | policy/governance and execution enforcement |
| activation runtime needs | root assembly and supervisor |
| semantic status | PDS projection over domain status |

## Candidate Lineage

A minimal lineage reference could be propagated across public domain boundaries:

```rust
struct StewardshipLineageRef {
    package_hash: String,
    profile_id: String,
    assignment_id: String,
    activation_id: String,
    charter_id: Option<String>,
    concern_id: Option<String>,
    objective_id: Option<String>,
    episode_id: Option<String>,
}
```

Likely attachment points:

- Agent decision command
- execution-goal record metadata
- planning composition metadata
- task lineage
- outcome publication
- promoted verification evidence

Internal domain records need not all carry the full structure.

## Conclusions

The runtime branch supports the PDS direction because most cognitive mechanisms already have durable owners and public contracts.

PDS implementation should concentrate on:

- declaration and linking
- profile abstraction
- assignment and activation
- standing mandate and episode semantics
- registration routes
- lineage
- authority coordination
- unified inspection

It should not create parallel observe, belief, Agent, planning, task, or outcome runtimes.

See [Candidate Implementation Requirements](candidate_implementation_requirements.md).