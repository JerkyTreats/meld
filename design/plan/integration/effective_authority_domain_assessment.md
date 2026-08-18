# Effective Authority Assessment By Domain

Date: 2026-08-13
Status: implementation input
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Scope: Theory Elevation Step 4

> Historical assessment of the implemented authority path. The current architecture moves requested authority out of Strategy theory and into the principal declaration and assignment. See [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md).

## Concern

Represent permission to execute a Strategy candidate independently from capability availability. The product must derive effective authority from the package request, principal grant, runtime policy, and current restrictions, preserve exact lineage through Agent judgment and execution, and fail closed before planning or dispatch can realize an unauthorized capability.

## Direct Behavior

One selected authority policy is installed as an exact execution-owned revision and included in the complete stewardship receipt. Strategy theory declares the action classes it requests. The Agent may authorize a verified candidate only when every required action class is requested, granted, runtime allowed, in scope, and unrestricted. Execution retains the decision and independently revalidates it during planning and dispatch.

## In Scope

- one principal per selected stewardship declaration
- one exact authority policy per installed stewardship package
- action classes identified by capability type identity
- exact subject scope matching
- explicit package request, principal grant, runtime allowance, and restriction sets
- Agent authorization lineage
- execution admission validation
- planning and dispatch enforcement
- compatibility for records without authority lineage outside exact receipt activation
- deterministic allowed and denied proofs

## Out Of Scope

- approval workflows
- time windows or expiring grants
- monetary or rate budgets
- organization hierarchy
- dynamic revocation inside an assembled process
- delegation and sub-grants
- multiple principals per declaration
- action-class taxonomy beyond capability type identity
- CVE-specific grants
- user-facing declaration grammar beyond the selected identities

## Maturity Envelope

This step establishes the smallest authority seam needed before the second expression. The current product has one concrete docs steward, exact capability contracts, exact Strategy authorization, guarded Goal admission, and deterministic planning and dispatch. It does not yet have a governance service, approval runtime, organization model, or stable multi-expression authority taxonomy. Action classes therefore reuse capability type identities and one immutable selected policy revision. Richer governance remains open until a dissimilar expression supplies evidence.

## Evidence Basis

- current repository on branch `implementation/theory-durability-symmetry`
- [Theory Elevation Program](theory_elevation_program.md)
- [PDS Architectural Invariants](../../persistent_domain_stewardship/architectural_invariants.md)
- [PDS Code Expression Map](../../persistent_domain_stewardship/code_expression_map.md)
- `crates/meld-world-model/src/strategy/contracts.rs`
- `crates/meld-world-model/src/agent/strategy.rs`
- `crates/meld-execution/src/capability/contracts.rs`
- `crates/meld-execution/src/goals/contracts.rs`
- `crates/meld-execution/src/planning/runtime.rs`
- `crates/meld-execution/src/task_network/dispatch_actor.rs`
- `src/runtime/ports.rs`
- `src/runtime/assembly.rs`
- `src/runtime/theory.rs`

Applicable policy is [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md). Compatibility work follows [Compatibility Shim Policy](../../../governance/compatibility_shim_policy.md).

## Regenerated Domain Snapshot

The current domain set was regenerated from root source modules and independently owned workspace crates. The root set contains agent, api, branches, capability, cli, compat, concurrency, config, context, control, docs, error, events, execution, harness, heads, ignore, init, lib, logging, merkle traversal, metadata, prompt context, provider, runtime, serve, session, store, task, telemetry, tree, types, views, workflow, workspace, and world state. Independently owned workspace domains are meld events, meld execution, meld lang, and meld world model.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| agent | none | Root Agent profiles expose role checks unrelated to stewardship execution | not needed | `src/agent.rs` | Stewardship Agent judgment lives in meld world model | none |
| api | none | Context facade does not admit or dispatch actions | not needed | `src/api.rs` | No authority command crosses this facade | none |
| branches | none | Branch identity already participates in product binding | not needed | `src/branches.rs` | Step 4 uses exact subject scope | none |
| capability | publish | Root docs publishes executable capability contracts and effects | complete | `src/docs/capability.rs` |  | reuse capability type identity as the first action class |
| cli | adapter | CLI delegates initialization and runtime composition | complete | `src/cli.rs` |  | reuse unchanged |
| compat | none | Compatibility facade does not own stewardship execution | not needed | `src/compat.rs` | No direct relationship | none |
| concurrency | none | Existing actors and stores own bounded coordination | not needed | `src/concurrency.rs` | No new coordination primitive | none |
| config | own | Declaration selects theory but no principal or authority policy | partial | `src/config/stewardship/selection.rs` |  | select principal and policy identity |
| context | none | Context supplies derived decision input | not needed | `src/context.rs` | Context does not grant action permission | none |
| control | none | Control orchestration does not admit stewardship actions | not needed | `src/control.rs` | No direct relationship | none |
| docs | publish | Docs publishes capability and theory bodies without authority policy | partial | `theory/docs_freshness` |  | publish the minimal docs policy and package request |
| error | adapter | Existing typed errors and diagnostics can carry denial | complete | `src/error.rs` |  | reuse stable surfaces |
| events | none | Event authority is storage authority, not action permission | not needed | `src/events.rs` | Avoid semantic collision between authority meanings | none |
| execution | adapter | Root ports copy Strategy authorization into execution | partial | `src/runtime/ports.rs` |  | carry authority lineage without deciding it |
| harness | consume | Fixtures construct Strategy and execution records directly | partial | `src/harness.rs` |  | preserve compatibility and add exact-policy fixtures |
| heads | none | Legacy head indexes are unrelated | not needed | `src/heads.rs` | No direct relationship | none |
| ignore | none | Ignore policy is capability-local | not needed | `src/ignore.rs` | No direct relationship | none |
| init | adapter | Stage 2 installs exact owner theory but no authority policy | partial | `src/init/world/pipeline.rs` |  | install policy and complete receipt last |
| lib | adapter | Root exports selected runtime contracts | partial | `src/lib.rs` |  | expose no policy semantics |
| logging | none | Logs consume actor reports | not needed | `src/logging.rs` | Existing diagnostics suffice | none |
| merkle traversal | none | Traversal is capability-local | not needed | `src/merkle_traversal.rs` | No direct relationship | none |
| metadata | none | Generation metadata does not grant execution | not needed | `src/metadata.rs` | No direct relationship | none |
| prompt context | none | Prompt lineage is not action permission | not needed | `src/prompt_context.rs` | No direct relationship | none |
| provider | none | Provider availability does not grant execution | not needed | `src/provider.rs` | No direct relationship | none |
| runtime | adapter | Assembly activates exact Strategy and capability bodies | partial | `src/runtime/theory.rs`, `src/runtime/assembly.rs` |  | resolve and route exact policy binding |
| serve | none | No authority inspection surface is required | not needed | `src/serve.rs` | Presentation remains deferred | none |
| session | none | Session lifecycle is not stewardship permission | not needed | `src/session.rs` | No direct relationship | none |
| store | none | Physical storage does not own policy meaning | not needed | `src/store.rs` | Owner registry uses existing storage | none |
| task | consume | Root task adapters eventually invoke authorized capabilities | complete | `src/task.rs` |  | execution crate enforces before invocation |
| telemetry | observe | Existing reports expose failed planning and dispatch | complete | `src/telemetry.rs` |  | reuse diagnostics |
| tree | none | Tree utilities are unrelated | not needed | `src/tree.rs` | No direct relationship | none |
| types | none | No root shared authority primitive is needed | not needed | `src/types.rs` | Shared contract belongs in meld lang | none |
| views | none | No authority UI is requested | not needed | `src/views.rs` | Inspection remains deferred | none |
| workflow | none | Native Strategy execution bypasses workflow routing | not needed | `src/workflow.rs` | Authority must not become a workflow | none |
| workspace | publish | Workspace subject supplies the exact policy scope | complete | `src/workspace.rs` |  | reuse unchanged |
| world state | none | Belief and world state do not grant action permission | not needed | `src/world_state.rs` | Keep epistemic and operational authority separate | none |
| meld events | none | Event ledger authority is unrelated to permission grants | not needed | `crates/meld-events` | Avoid extending the wrong authority structure | none |
| meld execution | own | Capability contracts, Goal admission, planning, and dispatch exist without a policy intersection | partial | `crates/meld-execution` |  | own durable policy and independent enforcement |
| meld lang | own | No shared pure authority contract or deterministic intersection exists | not started | `crates/meld-lang` |  | own neutral policy and decision value semantics |
| meld world model | consume | Agent authorizes verified Strategy candidates without permission evidence | partial | `crates/meld-world-model/src/agent/strategy.rs` |  | consume policy and attach authority decision |

## Frozen Affected Domain Set

The affected set is capability, cli, config, docs, error, execution, harness, init, lib, runtime, task, telemetry, workspace, meld execution, meld lang, and meld world model.

Changed behavior is limited to config selection, docs theory publication, initialization, runtime exact resolution and routing, shared authority values, Agent authorization, execution admission, planning, dispatch, and lineage. The remaining affected domains publish inputs, expose diagnostics, or are characterized unchanged.

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| principal and policy selection | config | declaration names Agent and theory identities | name principal and exact policy identity | extend existing | treating config as policy semantics | `src/config/stewardship/selection.rs` |
| package authority request | meld world model Strategy | package lists construction capabilities | explicitly request action classes separately | extend existing | capability vocabulary silently becoming permission | `strategy/contracts.rs` |
| neutral policy values | meld lang | shared Goal and Composition contracts exist | represent scope, grant, runtime allowance, restrictions, and effective decision | new local behavior | building governance runtime in the language crate | `meld-lang/src/lib.rs` |
| policy durability | meld execution | exact capability registry pattern exists | install and resolve one exact authority policy revision | new local behavior | central theory store owning foreign meaning | `capability/registry.rs` |
| candidate action extraction | meld execution and meld world model | Composition operators carry specific capability type identity | derive required action classes without a second catalog | reuse unchanged | adding parallel action metadata | `meld-lang::Operator` |
| Agent judgment | meld world model Agent | Strategy authorization already owns candidate approval | require an effective authority decision before exact authorization | extend existing | parallel authorization object becoming a second Agent decision | `agent/strategy.rs` |
| Goal admission | meld execution | guarded admission validates Strategy authorization | validate authority lineage and candidate action agreement | extend existing | admission pretending to know live policy | `goals/api.rs` |
| planning revalidation | meld execution | authorized planning rechecks Goal, frame, structure, preconditions, and live capabilities | recheck exact policy and effective action set | extend existing | capability availability being accepted as authority | `planning/runtime.rs` |
| task lineage | meld execution | every lowered task carries Goal, composition, operator, and capability lineage | carry the same authority decision to dispatch | extend existing | duplicating policy bodies per task | `task_network/state.rs` |
| dispatch enforcement | meld execution | dispatch invokes ready capability instances | reject invocation when lineage lacks or violates the active exact policy | extend existing | enforcement only in planning | `task_network/dispatch_actor.rs` |
| source publication | docs | authored package and executable contracts are data | publish one policy and request matching docs capability types | new local behavior | docs-specific policy logic in root | `theory/docs_freshness` |
| exact installation | init | complete receipt commits after owner installation | install policy revision and include it in the receipt | adapter only | partial receipt visibility | `src/init/world/pipeline.rs` |
| exact activation | runtime | receipt resolution freezes owner revisions | freeze and inject one policy binding into Agent, planning, and dispatch | adapter only | current-head fallback or root recomputation | `src/runtime/theory.rs` |
| port translation | execution root adapter | Strategy authorization is copied into execution DTO | carry the shared authority decision intact | adapter only | lossy reconstruction | `src/runtime/ports.rs` |
| compatibility | harness and owners | older records lack authority fields | accept missing authority only when no exact policy is activated | extend existing | compatibility becoming the production path | owner tests |

## Ownership And Boundary Synthesis

Meld lang owns only the pure shared values and deterministic intersection. Meld execution owns authority-policy durability and the independent admission, planning, and dispatch checks. Meld world model Agent consumes the exact policy and attaches the resulting decision to its existing Strategy authorization. Strategy theory owns the package request. Config selects principal and policy identities. Docs publishes one policy body. Initialization and runtime install, resolve, and route exact owner contracts.

The smallest missing connective behavior is an authority decision carried inside the existing Strategy authorization lineage. A separate authorization subsystem, approval workflow, generic role registry, or event-authority extension is not required.

## Explicit Non-Integration Decisions

- Event authority is not extended because durable append custody is not action permission.
- Root Agent roles are not reused because they authorize CLI profile behavior rather than stewardship capability invocation.
- Available actions and capability catalogs remain availability surfaces, never grants.
- The Agent does not mint policy or principal grants.
- Root assembly does not calculate effective authority.
- Execution does not reinterpret maintained conditions or Strategy quality judgment.
- No stable abstract action-class taxonomy is frozen before the CVE proof.

## Boundary Risks

- Reusing capability availability as package request or grant would violate the core invariant.
- Creating a second authorization record beside Strategy authorization would split Agent judgment lineage.
- Enforcing only at Agent curation would let stale or tampered authority pass later boundaries.
- Enforcing only at dispatch would admit Goals and plans the principal never authorized.
- Copying policy bodies into task records would duplicate authority instead of preserving a compact decision.
- Treating current restrictions as mutable without a revocation runtime would overstate product maturity.

## Unresolved Questions

No ownership question blocks this vertical. The second expression must test whether capability type identity remains a sufficient action class and whether principal grant and runtime policy need independently revised owner records. Those shapes remain intentionally unfrozen.
