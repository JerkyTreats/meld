# PDS Boundary Program Assessment By Domain

Date: 2026-08-18
Status: proposed delivery assessment
Scope: predecessor closeout, compatibility isolation, PDS cognition-boundary correction, documentation-freshness migration, and dependency-security falsification

## Concern

The current PDS implementation contains a durable generic routing and activation substrate, but its compatibility package still crosses cognition concerns through `StrategyTheoryPackage`, package-carried exact capabilities, package-carried construction policy and search controls, package-requested projection and authority, and target-shaped belief evidence. The delivery concern is how to close the superseded `W00` through `W06` program honestly, preserve its reusable substrate in a clean checkpoint, and replace the compatibility boundary through bounded product slices without centralizing owner behavior or absorbing unrelated runtime-lifecycle work.

## Direct Behavior

Documentation freshness must continue to construct and authorize a valid strategy through routed PDS semantics while the exact capability set comes from activation, construction policy comes from Agent, search bounds come from the request, projection is grounded for the current Goal, exact authority requirements come from the candidate, and belief revision consumes proof-carrying lower ground rather than a producer-supplied conclusion. Dependency security must then exercise the same ownership boundary without introducing domain branches into shared runtime code.

## In Scope

- predecessor program reconciliation and clean checkpoint
- generic PDS router and exact receipt preservation
- compatibility decomposition of `StrategyTheoryPackage`
- separately owned Strategy construction inputs
- activation-local capability snapshot publication
- Agent-owned construction policy and candidate judgment
- request-owned search controls
- candidate-derived exact authority requirements
- proof-carrying documentation-freshness evidence
- dependency-security falsification of the resulting boundary
- historical exact receipt resolution through bounded compatibility readers
- removal of canonical callers of the compatibility aggregate

## Out Of Scope

- production dependency-security integration or remediation
- general supervisor lease and shutdown repair
- portable external-operation completion beyond behavior directly required by the boundary proof
- learned Strategy evaluation or alternative-retention expansion
- settled Strategy replay
- canonical principal declaration and profile schemas
- package marketplace, signing, remote distribution, or upgrade protocol
- universal proof language, ontology, Method language, or projection framework
- new provider, workflow, session, metadata, or Merkle semantic ownership

## Evidence Basis

- [Canonical PDS Architecture](../../cognitive_architecture/persistent_domain_stewardship.md)
- [PDS Cognition Boundary Assessment](pds_cognition_boundary_domain_assessment.md)
- [PDS Authorized Implementation Workstreams](pds_implementation_workstreams.md)
- [PDS Authorized Implementation Findings](pds_authorized_implementation_findings.md)
- current `StrategyTheoryPackage`, `StrategyProblem`, Agent Strategy adapter, runtime assembly, routed package manifests, activation contribution, and belief ingestion code
- deterministic docs characterization, router, activation, dependency-security, lifecycle, and workspace test evidence

## Applicable Policy

- [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md)
- [Compatibility Policy](../../../governance/compatibility_policy.md)
- [Compatibility Shim Policy](../../../governance/compatibility_shim_policy.md)
- [Docs Style Policy](../../../governance/docs_style_policy.md)
- [Semantic Unit Preservation Policy](../../../governance/semantic_unit_preservation_policy.md)
- [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md), only if explicitly activated

## Regenerated Domain Snapshot

Evidence command:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

Snapshot:

```text
agent
api
branches
capability
cli
compat
concurrency
config
context
control
dependency_security
docs
error
events
execution
harness
heads
ignore
init
lib
logging
merkle_traversal
metadata
prompt_context
provider
runtime
serve
session
store
task
telemetry
theory
tree
types
views
workflow
workspace
world_state
```

Independently owned workspace domains on the path are `meld-events`, `meld-execution`, `meld-lang`, and `meld-world-model`.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| theory | own | generic package materialization, routing, receipt, and historical resolution | complete | `src/theory` | none | reuse without semantic decoding |
| config | consume | legacy and canonical stewardship selection plus assignment and activation identities | partial | `src/config/stewardship` | none | keep physical bindings outside package identity |
| init | adapter | central product route catalog and owner store wiring | partial | `src/init/world/routes.rs` | none | retain thin route composition |
| runtime | adapter | monolithic resolved compatibility image and activation assembly | partial | `src/runtime/theory.rs`, `src/runtime/assembly.rs` | none | replace canonical aggregate consumption |
| capability | publish | deterministic product contribution and activation preparation | partial | `src/capability.rs`, `src/capability/contribution.rs` | none | publish Strategy-visible activation snapshot |
| docs | own | claim policy, capabilities, routed package, assessment, and target-shaped freshness value | partial | `src/docs`, `theory/docs_freshness` | none | separate semantics, affordances, and proof grounds |
| dependency_security | own | policy, products, adapters, admission, verification, routes, and fixture package | partial | `src/dependency_security`, `theory/dependency_security` | none | use as second boundary consumer |
| agent | adapter | root facade over world-model Agent ownership | partial | `src/agent.rs` | none | no new semantic ownership |
| meld-world-model belief | own | generic evidence normalization, comparison, revision, and exact theory registries | partial | `crates/meld-world-model/src/belief` | none | admit proof grounds without producer conclusion substitution |
| meld-world-model Agent | own | maintained-condition curation, Strategy invocation, and authorization | partial | `crates/meld-world-model/src/agent` | none | own construction policy and current grounding |
| meld-world-model Strategy | own | `StrategyProblem`, search, verification, candidates, and compatibility package registry | partial | `crates/meld-world-model/src/strategy` | none | make independent inputs canonical |
| meld-execution capability | own | exact executable contract registry and semantic enforcement | partial | `crates/meld-execution/src/capability` | none | expose activation-selected semantics without package ownership |
| meld-execution authority | own | policy validation and dispatch enforcement | complete | `crates/meld-execution/src/authority` | none | reuse independent enforcement |
| meld-execution planning and tasks | consume | authorized candidate realization and task execution | complete | `crates/meld-execution/src/planning` | none | reuse without PDS interpretation |
| meld-events | publish | owner-admitted canonical append and replay | complete | `crates/meld-events` | none | preserve proof lineage unchanged |
| meld-lang | publish | shared propositions, Goals, operators, Methods, and compositions | complete | `crates/meld-lang` | none | reuse without PDS vocabulary branches |
| events | adapter | root facade over event append, replay, and authority | complete | `src/events.rs` | none | preserve thin routing |
| execution | adapter | root facade over execution ownership | complete | `src/execution.rs` | none | preserve thin routing |
| workspace | publish | source identity, scan, and change facts | partial | `src/workspace` | none | publish source ground only |
| branches | publish | branch-scoped source identity | complete | `src/branches` | none | preserve explicit assignment context |
| context | publish | current frame and projection lineage | partial | `src/context` | none | no PDS-owned memory |
| prompt_context | publish | provider input lineage | partial | `src/prompt_context` | none | observe only when proof needs provenance |
| telemetry | observe | attempt, timing, and runtime diagnostics | partial | `src/telemetry` | none | expose economics without becoming semantic proof |
| harness | observe | deterministic and live product evidence | partial | `src/harness`, integration tests | none | retain failure evidence outside temporary roots |
| compat | adapter | legacy declaration and historical receipt lowering | partial | `src/compat.rs`, stewardship selection | none | freeze callers and retain bounded readers |
| api | adapter | public facade wiring | not started | `src/api.rs` | none | change only if a current public type moves |
| cli | adapter | operator entry and diagnostics | not started | `src/cli.rs` | none | no new command required for first slice |
| serve | adapter | served runtime facade | not started | `src/serve.rs` | none | reuse unchanged unless proof requires inspection |
| lib | adapter | public crate exports | partial | `src/lib.rs` | none | export only owner-defined canonical contracts |
| provider | none | physical model realization | not needed | `src/provider.rs` | provider binding is activation state, not PDS semantics | none |
| workflow | none | historical and compatibility procedure | not needed | `src/workflow.rs` | PDS does not install workflow topology | preserve non-integration |
| session | none | command lifecycle | not needed | `src/session.rs` | command lifecycle is distinct from PDS semantics | none |
| control | none | orchestration and node state | not needed | `src/control.rs` | no cognition-boundary ownership | none |
| store | none | generic persistence primitives | not needed | `src/store.rs` | storage does not interpret theory | none |
| concurrency | none | shared execution limits | not needed | `src/concurrency.rs` | no direct boundary behavior | none |
| task | none | root task facade | not needed | `src/task.rs` | execution already owns realization | none |
| world_state | none | root compatibility facade | not needed | `src/world_state.rs` | world-model graph owns current projection | none |
| heads | none | compatibility head indexing | not needed | `src/heads.rs` | no cognition-boundary ownership | none |
| tree | none | tree query and mutation support | not needed | `src/tree.rs` | no cognition-boundary ownership | none |
| metadata | none | generic metadata contracts | not needed | `src/metadata.rs` | no cognition-boundary ownership | none |
| views | none | presentation-shaped read models | not needed | `src/views.rs` | no cognition-boundary ownership | none |
| types | none | shared root aliases and types | not needed | `src/types.rs` | owner contracts remain in owning modules | none |
| error | none | generic error facade | not needed | `src/error.rs` | no new error authority is required | none |
| logging | none | process logging | not needed | `src/logging.rs` | diagnostics do not establish semantic proof | none |
| ignore | none | ignored path policy | not needed | `src/ignore.rs` | no cognition-boundary ownership | none |
| merkle_traversal | none | workspace implementation detail | not needed | `src/merkle_traversal.rs` | no PDS semantic role | none |

## Frozen Affected Domain Set

The affected set is theory, config, init, runtime, capability, docs, dependency security, root Agent adapter, world-model belief, world-model Agent, world-model Strategy, execution capability, execution authority, execution planning and tasks, Meld Events, root events adapter, Meld Lang, root execution adapter, workspace, branches, context, prompt context, telemetry, harness, compatibility, API, CLI, serve, and crate exports.

Changed behavior is expected only in world-model Strategy, world-model Agent, capability publication, runtime assembly, docs evidence publication, belief admission, dependency security validation, and compatibility lowering. Other affected domains are adapters, evidence publishers, or unchanged runtime consumers.

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| package structure and receipt | theory | opaque components and exact owner refs | preserve structure without interpreting cognition | reuse unchanged | router gains semantic authority | `src/theory` |
| semantic theory installation | routed owners | owner decode, validation, registry, and query | accept stable domain meaning only | extend existing | state-free precomputed cognition remains valid | product route catalog |
| declaration and assignment | config | package, principal, subject, scope, and physical activation identities | select semantics and governance without active toolbox | extend existing | capability identity leaks into package | stewardship contracts |
| compatibility image | runtime | `ResolvedStewardshipTheory` joins all docs-shaped components | become a historical adapter over owner-scoped resolutions | extend existing | runtime remains canonical docs owner | runtime theory |
| Strategy assembly | world-model Agent and Strategy | installed package becomes a partial `StrategyProblem` | assemble separate semantic, capability, Method, policy, projection, and authority inputs | new local behavior | a renamed aggregate preserves the violation | Agent Strategy adapter |
| semantic settlement | world-model Strategy | `StrategyTheorySnapshot` already carries settlement rules | retain only stable settlement and prospective evidence meaning | extend existing | exact action topology reenters theory | Strategy contracts |
| activation capability snapshot | capability and activation | exact executable registry and invoker preparation | publish immutable Strategy-visible action semantics for one generation | extend existing | package and activation identities collapse | contribution and activation contracts |
| construction policy | world-model Agent | evaluation policy arrives inside package | select current exact policy without a new registry in the first slice | new local behavior | policy becomes ambient root default | Agent Strategy adapter |
| search controls | world-model Strategy caller | bounds arrive inside package | place bounds on `StrategySearchRequest` | extend existing | request controls become installed truth | Strategy contracts |
| projection grounding | planner and Agent | requested dimensions arrive inside package | derive current needs from Goal and available semantic preconditions | extend existing | static package projection survives | planning binding |
| candidate authority | Strategy plus Agent | requested authority arrives inside package | derive exact requests from selected capability actions | extend existing | package request is mistaken for grant | candidate and authority contracts |
| docs proof publication | docs | assessment computes freshness conclusion and proof details | publish independently admissible support, contradiction, coverage, currency, and lineage | extend existing | producer conclusion substitutes for belief | docs capability and claim validation |
| belief evidence admission | world-model belief | source mappings select scalar fields | admit proof grounds and perform belief-owned inference | extend existing | generic proof framework is invented too early | belief mapping and comparator |
| dependency-security proof | dependency security | rich assessment body plus scalar compatibility mappings | exercise the refined boundary with coverage and negative-proof semantics | extend existing | docs assumptions are generalized | security assessment and verification |
| canonical append | events | owner-admitted products with lineage | preserve products unchanged | reuse unchanged | transport success becomes truth | event append |
| realization | execution | authorized composition lowering and dispatch | consume candidate without semantic substitution | reuse unchanged | execution reconstructs Strategy | planning and task contracts |
| observed economics | telemetry and capability | partial timing and request records | preserve truthful bounds needed by current Strategy proof | extend existing | hidden procedure remains decision-relevant | implementation findings |
| compatibility decoding | compat and owner registries | historical exact package bodies | decode old aggregate without creating new callers | extend existing | migration shell becomes permanent canonical path | compatibility gates |
| product proof | harness | deterministic parity plus environment-gated live checks | retain outputs and prove boundary counterfactuals | extend existing | harness success substitutes for product behavior | PDS integration tests |
| public adapters | API, CLI, serve, init | facade and composition wiring | route changed owner contracts only | adapter only | adapter becomes authority | root modules |

## Ownership And Boundary Synthesis

The theory router remains the structural package owner. PDS owner routes publish stable semantic revisions. Capability and activation publish the exact current affordance snapshot. World-model Agent selects construction policy and judges candidates. World-model Strategy assembles and searches the immutable current problem. Execution revalidates authority and realizes the accepted candidate. Docs and dependency security publish proof-bearing owner products. Belief admits those grounds and owns current inference.

Root runtime and init remain adapters. Compatibility code may decode old bodies and assemble old views for historical reads, but no new canonical caller may depend on the aggregate.

The smallest missing connective behavior is an Agent-owned Strategy assembly seam over existing `StrategyProblem` inputs. The first slice must prove that seam with documentation freshness before any generalized new registry, store, protocol, or language is considered.

## Runtime Path, Changed Behavior, And Likely Write Scope

Runtime path domains include config, init, theory, docs, capability, runtime, world-model belief, world-model Agent, world-model Strategy, execution, events, provider, workspace, context, and telemetry.

Changed behavior is narrower: world-model Strategy contracts, Agent Strategy assembly, activation capability publication, runtime compatibility assembly, docs proof publication, belief admission, dependency-security proof mapping, and compatibility readers.

Likely first-slice write scope is limited to Strategy contracts and registry, Agent Strategy assembly, capability contribution or activation snapshot mapping, runtime theory and assembly adapters, the docs package and semantic theory body, and focused tests. Docs evidence and dependency-security files belong to later slices.

## Explicit Non-Integration Decisions

- no new PDS behavior enters provider, workflow, session, control, metadata, Merkle traversal, or generic storage
- no new application branch enters Meld Lang, root runtime, or execution planning
- no supervisor lease or shutdown repair is absorbed into the boundary program
- no production dependency-security service or repository mutation is introduced
- no learned evaluation, Strategy replay, universal proof language, or declaration schema is introduced
- no new crate, durable store, background runtime, dependency, or compatibility system is justified by this assessment

## Unresolved Questions

- whether existing `StrategyTheorySnapshot` can become the exact canonical semantic revision without a public rename
- the smallest activation-owned representation that supplies Strategy operator and outcome semantics without duplicating execution contracts
- the first-slice Agent construction policy source before a durable policy registry is justified
- the narrowest belief-owned computation that consumes docs proof grounds without creating a universal evidence language
- whether retained historical `StrategyTheoryPackage` bytes exist outside tests and local product roots

These questions are phase gates. They do not reopen the accepted ownership boundary.
