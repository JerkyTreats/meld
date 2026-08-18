# PDS Cognition Boundary Assessment By Domain

Date: 2026-08-18
Status: accepted architecture assessment
Scope: ownership of stable PDS meaning, situated cognition, Strategy construction inputs, activation affordances, authority, and outcome reconciliation

## Concern

The current routed PDS implementation proves structural package installation and owner revision closure, but its Strategy body combines stable settlement meaning with exact capabilities, candidate evaluation policy, search bounds, projection requests, and authority requests. Documentation freshness also admits a target-shaped correctness scalar after substantial assessment has already occurred. This assessment fixes the ownership boundary needed to prevent a PDS package from precomputing or obscuring the cognition Meld is responsible for performing.

## Direct Behavior

The design must let a declaration select durable domain meaning while Meld constructs current evidence, beliefs, Goals, Strategy candidates, authorizations, tasks, and reassessments from explicit runtime inputs.

## In Scope

- PDS semantic theory
- package routing and exact revision lineage
- belief questions and proof semantics
- maintained conditions and Goal formation
- Strategy problem construction
- capability catalog ownership
- construction policy and search controls
- authority request, grant, judgment, and enforcement
- proof-carrying outcome admission
- documentation freshness and dependency-security validation

## Out Of Scope

- runtime implementation
- compatibility migration mechanics
- a final principal-facing declaration schema
- a universal domain ontology
- production dependency-security expansion
- changes to task scheduling or provider transport

## Evidence Basis

- [Canonical PDS layer](../../cognitive_architecture/persistent_domain_stewardship.md)
- [World Model Strategy](../../cognitive_architecture/world_model/strategy/README.md)
- [Strategy Search](../../cognitive_architecture/world_model/strategy/search.md)
- [PDS Semantic Interface Review](pds_semantic_interface_review.md)
- [PDS Authorized Implementation Findings](pds_authorized_implementation_findings.md)
- [Theory Router Synthesis](../../persistent_domain_stewardship/examples/theory_router_synthesis.md)
- current routed docs and dependency-security theory bodies
- current `StrategyTheoryPackage`, router, assignment, activation, capability, authority, and execution contracts

## Domain Snapshot

The current product path contains root assembly and configuration, theory routing, events, source domains, docs, dependency security, graph, belief, Agent, planner, Strategy, shared language, capabilities, execution authority, execution planning and tasks, runtime lifecycle, providers, context, workflow compatibility, session and control, storage, telemetry, branches, metadata, and workspace infrastructure.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| PDS package and theory router | own | structural manifest, routes, owner validation, exact receipts | partial | `src/theory` | router must not interpret owner cognition | retain structural ownership |
| root assembly | adapter | resolves and assembles owner products | partial | `src/runtime/assembly.rs` | root owns no domain meaning | map explicit snapshots only |
| config and declaration | consume | selects identities, scope, provider, principal | partial | `src/config/stewardship` | declaration must not embed theory bodies | select semantic package and assignment policy |
| CLI and serve | adapter | configuration and inspection entry points | not started | `src/cli`, `src/serve` | presentation is not semantic authority | no boundary change |
| events | publish | canonical append and replay | complete | `crates/meld-events` | events do not interpret PDS theory | preserve owner-admitted products |
| workspace and source domains | publish | source observations and identities | partial | `src/workspace`, `src/docs` | source truth remains source-owned | publish proof grounds |
| docs | own | claim policy, capability products, final assessment | partial | `src/docs`, `theory/docs_freshness` | docs must not own belief revision or Strategy | replace target-shaped evidence with lower grounds |
| dependency security | own | bounded inventory, advisory, coverage, posture products | partial | `src/dependency_security`, `theory/dependency_security` | security must not own belief revision or Strategy | validate refined boundary |
| graph | publish | current anchors, identity, provenance, projection inputs | partial | `crates/meld-world-model` | no PDS-owned graph state | preserve explicit projection contracts |
| belief | own | evidence admission, comparator execution, revisions | partial | `crates/meld-world-model/src/belief` | PDS may define proof meaning but not current conclusions | require proof-carrying evidence |
| Agent | own | maintained-condition evaluation, Goal curation, Strategy judgment | partial | `crates/meld-world-model/src/agent` | PDS does not authorize candidates | own construction policy selection and judgment |
| planner projection | own | ground world-state projection | partial | world-model planner contracts | PDS does not request a static universal projection | derive projection needs per Goal |
| Strategy | own | immutable problem, search, verification, candidates | partial | `crates/meld-world-model/src/strategy` | PDS does not assemble a partial problem | split semantic theory from other inputs |
| `meld-lang` | publish | propositions, operators, methods, effects, world state | complete | `crates/meld-lang` | shared IR does not choose ownership | reuse unchanged |
| capability | publish | exact contracts and implementations | partial | `src/capability`, execution capability registry | catalog availability is not package meaning | publish activation-local snapshot |
| execution authority | own | authority policy, decision validation, dispatch checks | partial | `crates/meld-execution/src/authority` | package request is not effective authority | preserve independent gates |
| execution planning and tasks | consume | exact candidate lowering and realization | partial | `crates/meld-execution/src/planning` | execution must not reinterpret Strategy | reuse authorized candidate contract |
| runtime lifecycle | consume | assignment activation, readiness, fencing, admission | partial | `src/runtime` | lifecycle is not semantic theory | carry exact activation catalog identity |
| provider | none | physical implementation binding | not needed | `src/provider` | provider choice is activation state | no PDS semantic contract |
| context and prompt context | publish | bounded frames and model input lineage | partial | `src/context`, `src/prompt_context` | context is not PDS-owned memory | future projection contract remains separate |
| workflow | none | compatibility implementation and historical Methods | not needed | `src/workflow` | workflow topology is not PDS meaning | retain explicit compatibility status |
| session and control | none | operator and process lifecycle | not needed | `src/session`, `src/control` | stewardship lifecycle remains distinct | no direct integration |
| store | none | persistence infrastructure | not needed | `src/store` | storage does not interpret theory | no direct integration |
| telemetry | observe | runtime and operation diagnostics | partial | `src/telemetry` | diagnostics are not semantic proof | expose observed cost without becoming truth |
| branches | publish | branch-scoped source state | partial | `src/branches` | branch is assignment context, not PDS core vocabulary | reuse unchanged |
| metadata | none | generic product metadata | not needed | `src/metadata` | no cognition-boundary ownership | no direct integration |
| Merkle traversal | none | workspace implementation detail | not needed | `src/merkle_traversal` | no PDS semantic role | no direct integration |

## Frozen Affected Domain Set

The affected set is PDS package and router, root assembly, configuration and assignment, events, workspace and source domains, docs, dependency security, graph, belief, Agent, planner projection, Strategy, `meld-lang`, capability, execution authority, execution planning and tasks, runtime lifecycle, context, telemetry, and branches.

Only PDS semantic contracts, Strategy input ownership, proof admission, capability publication, assignment authority, and assembly need changed behavior. Other affected domains publish or consume existing contracts unchanged.

## Pass Two Affected Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| package envelope and imports | theory router | generic manifest and exact receipts | carry owner semantic fragments without interpretation | reuse unchanged | router becomes semantic authority | `src/theory` |
| semantic theory installation | owner routes | owner decode, validate, install, verify | install stable meaning only | extend existing | state-free but precomputed cognition remains admissible | world init routes |
| declaration selection | config | identity-only declaration and generic assignment | select package, scope, perspective, requested governance | extend existing | physical catalog leaks into declaration | `src/config/stewardship` |
| runtime assembly | root runtime | complete resolved receipt and activation | combine explicit owner snapshots without semantic inference | extend existing | root reintroduces application branches | runtime assembly |
| evidence admission | belief | source mappings and comparator pipeline | consume proof grounds, not only target-shaped conclusions | extend existing | assessment output masquerades as belief | belief ingestion |
| maintained-condition curation | Agent | installed conditions and deterministic curation | create Goals from current belief only | reuse unchanged | package preselects Goal or decision | Agent runtime |
| construction policy | Agent | evaluation policy currently embedded in Strategy package | select exact policy for current Goal class | new local behavior | PDS fixes what stronger means | Agent Strategy adapter |
| projection request | planner | requested dimensions currently embedded in Strategy package | derive from Goal, theory, Method, and capability preconditions | extend existing | package freezes current cognitive view | planner public contract |
| semantic theory snapshot | Strategy | settlement snapshot inside `StrategyTheoryPackage` | retain only Goal settlement, evidence, action-class, outcome, and constraint meaning | extend existing | exact toolbox or topology reenters theory | Strategy contracts |
| capability snapshot | capability and activation | exact capabilities currently embedded in package body | publish complete activation-local semantic contract snapshot | extend existing | package identity becomes catalog identity | capability registry and activation |
| Method snapshot | Strategy | immutable Method library input | source from separately admitted Strategy templates | extend existing | reusable procedure becomes PDS mandate | Strategy search |
| evaluation and traversal | Strategy | evaluation policy and bounds in installed package | accept policy in problem and bounds in search request | extend existing | search budget becomes domain truth | Strategy search |
| candidate authority requirements | Strategy | requested authority embedded in package | derive exact requirements from selected actions | extend existing | requested action becomes grant | Strategy candidate |
| effective authority | Agent and execution | independent decision and dispatch validation | intersect candidate requirements, assignment request, grant, and restrictions | reuse unchanged | capability availability confers authority | authority contracts |
| realization | execution | exact authorized composition lowering | realize without substitution | reuse unchanged | execution invents semantic work | planning and task contracts |
| domain proof product | docs and dependency security | canonical owner products | retain subject, source revision, coverage, contradiction, currency, policy, and calculation lineage | extend existing | naked scalar crosses evidence seam | domain adapters |
| canonical publication | events | owner-admitted append | preserve product and provenance | reuse unchanged | transport success becomes truth | event append |
| outcome reassessment | belief and Agent | outcome mapping, belief revision, curation | reassess from admitted domain evidence | reuse unchanged | task completion becomes restoration | flywheel contracts |
| observed economics | telemetry and capability | partial attempt and outcome records | expose cardinality, elapsed work, retries, and revisions when Strategy-relevant | extend existing | atomic contract hides consequential procedure | implementation findings |
| perspective and branch | graph, belief, assignment | explicit identity in current contracts | remain part of every situated projection | reuse unchanged | package meaning becomes ambient process state | world-model contracts |

## Ownership Synthesis

PDS owns durable semantic supply. It declares domain vocabulary, belief questions, proof semantics, maintained norms, settlement obligations, abstract action and outcome meaning, and governance classifications.

Strategy owns construction. It assembles one immutable problem from a Goal, planner snapshot, PDS semantic theory snapshot, activation capability snapshot, separately admitted Method snapshot, Agent-selected evaluation policy, and explicit authority context. Search bounds remain request controls.

Capability and activation own the exact current affordance set. PDS may declare required action classes and activation requirements, but it does not author the active toolbox.

Belief owns current evidence admission, inference, and revision. Agent owns maintained-condition evaluation, Goal creation, construction policy selection, candidate judgment, and satisfaction. Execution owns authority revalidation, realization, task state, and effects.

Root remains an adapter that resolves exact revisions and supplies them to owner constructors. It does not infer application meaning.

## Explicit Non Integration Decisions

- PDS does not own a capability catalog.
- PDS does not own Strategy evaluation policy or search bounds.
- PDS does not own a Method library or action topology.
- PDS does not own current projection requests as a static package list.
- PDS does not grant authority.
- PDS does not emit current belief conclusions, Goals, candidates, tasks, or restoration results.
- Router structure does not inspect domain semantics.
- Provider, workflow, session, storage, metadata, and Merkle infrastructure receive no new PDS semantic ownership.

## Smallest Missing Connective Behavior

The smallest missing bridge is an owner-defined assembly contract that constructs a `StrategyProblem` from separately resolved semantic theory, capability, Method, policy, projection, and authority inputs. The current `StrategyTheoryPackage` collapses those sources into one installable body and must become a compatibility antecedent rather than the canonical public contract.

## Resolved Ownership Questions

- exact capabilities come from activation
- evaluation policy comes from Agent construction policy
- search bounds come from the search request
- projection needs are derived for the current Goal
- exact authority requirements are derived from the selected candidate
- stable settlement and proof meaning comes from PDS semantic theory
- Methods are separately admitted Strategy knowledge

## Remaining Questions

- the exact schema for stable action-class and causal meaning inside Strategy semantic theory
- how Strategy requests a second projection when candidate discovery exposes new preconditions
- which capability-internal decisions require decomposition and which may remain atomic with observed economics
- how a compatibility wrapper decomposes the existing `StrategyTheoryPackage` during migration

These questions do not reopen the ownership boundary.
