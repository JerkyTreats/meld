# Theory Declaration Lowering Assessment By Domain

Date: 2026-08-13
Status: implementation input
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Scope: Theory Elevation Step 2

> Historical assessment of the implemented compatibility path. Use [PDS Cognition Boundary Assessment](pds_cognition_boundary_domain_assessment.md) and [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) for current ownership.

## Concern

Replace the expression-shaped docs freshness configuration and direct docs runtime composition with one declaration-shaped selection and a data-driven lowering path. Root config, initialization, CLI, and runtime assembly may select, route, and bind domain products, but they must not choose semantic behavior by expression name. The change must preserve the complete receipt and exact lineage delivered by Step 1.

## Direct Behavior

A canonical stewardship declaration names an expression, target, subject, agent, provider, and installed theory identities. Configuration lowers declarations into physical bindings. Initialization installs the selected image. Runtime assembly resolves the active exact receipt, activates Strategy from the owner package, and binds executable implementations by exact capability contract identity.

## In Scope

- a generic declaration collection in root configuration
- compatibility lowering from the existing docs freshness table
- source-aware validation for arbitrary declaration ids
- deterministic physical binding resolution for declarations
- target-based selection for world initialization and CLI runtime composition
- owner-side activation of an installed Strategy package
- capability implementation binding by exact contract identity
- removal of direct docs PDS composition from production runtime assembly
- declaration-selected belief-family routing into belief-context hydration
- retirement of stale docs Method, available-action, and realization source artifacts
- parity for the existing docs freshness runtime

## Out Of Scope

- standing maintained conditions from Step 3
- authority grants and restrictions from Step 4
- the CVE expression from Step 5
- settled Strategy replay from Step 6
- a final conversational or visual authoring surface
- hot replacement inside an assembled runtime
- a generic semantic registry that owns foreign domain bodies
- revival of Method or workflow routing for docs freshness

## Maturity Envelope

This step establishes the smallest canonical declaration and lowering seam required to remove expression dispatch. It does not freeze the eventual principal-facing PDS language. Package presets, authority posture, maintained conditions, assignment, and activation records remain governed by their later program steps.

## Evidence Basis

- current repository at branch `implementation/theory-durability-symmetry`
- [Theory Elevation Program](theory_elevation_program.md)
- [PDS Theory Runtime Layer Map](pds_theory_runtime_layer.md)
- [Bottom-Up Runtime Review](../../persistent_domain_stewardship/reviews/bottom_up_runtime_review.md)
- [Bidirectional Review Synthesis](../../persistent_domain_stewardship/reviews/bidirectional_synthesis.md)
- `src/config/stewardship/selection.rs`
- `src/config/stewardship/binding.rs`
- `src/init/world/tooling.rs`
- `src/runtime/assembly.rs`
- `src/docs/pds.rs`
- `crates/meld-world-model/src/agent/strategy.rs`

Applicable policy is [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md). Compatibility work follows [Compatibility Shim Policy](../../../governance/compatibility_shim_policy.md).

## Regenerated Domain Snapshot

The snapshot was regenerated with:

```sh
find src -mindepth 1 -maxdepth 1 -printf '%f\n' | sort
find crates -mindepth 1 -maxdepth 2 -type f -name Cargo.toml -printf '%h\n' | sort
```

The root domain set contains agent, api, branches, capability, cli, concurrency, config, context, control, docs, error, events, execution, harness, heads, ignore, init, logging, merkle traversal, metadata, prompt context, provider, runtime, serve, session, store, task, telemetry, tree, types, views, workflow, workspace, and world state. Independently owned workspace domains are meld events, meld execution, meld lang, and meld world model.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| agent | none | Root agent configuration is separate from stewardship Agent records | not needed | `src/agent.rs` | Declaration lowering does not change root agent profiles | none |
| api | none | Context facade is used after runtime composition | not needed | `src/api.rs` | No declaration contract crosses the API facade | none |
| branches | none | Branch identity remains a physical runtime input | not needed | `src/branches.rs` | Declaration meaning is branch independent in this step | none |
| capability | adapter | Root capability surface re-exports execution contracts | partial | `src/capability.rs` |  | expose exact implementation activation without expression matching |
| cli | adapter | CLI checks the docs freshness field directly | partial | `src/cli/runtime_assembly.rs` |  | select bindings by target from generic declarations |
| concurrency | none | Runtime limits are below declaration meaning | not needed | `src/concurrency.rs` | No coordination contract changes | none |
| config | own | One docs-specific optional field owns the only selection shape | partial | `src/config/stewardship/selection.rs` |  | add canonical declaration collection and compatibility lowering |
| context | consume | Belief-context hydration hardcodes the docs family id | partial | `src/context/belief_context.rs` |  | accept the family identity selected by the declaration |
| control | none | Control orchestration is outside the dynamic Strategy path | not needed | `src/control.rs` | No declaration lowering relationship | none |
| docs | publish | Docs publishes exact capability contracts and executors, but its PDS composer also activates Strategy | partial | `src/docs/capability.rs`, `src/docs/pds.rs` |  | retain docs capability ownership and remove production PDS composition |
| error | adapter | Existing config and assembly errors carry diagnostics | complete | `src/error.rs` |  | reuse stable error surfaces |
| events | none | Event authority receives runtime products after lowering | not needed | `src/events.rs` | Declaration lowering adds no event contract | none |
| execution | consume | Planning and dispatch consume exact catalogs and invokers | complete | `src/execution.rs`, `meld-execution` |  | reuse unchanged |
| harness | consume | Fixtures construct physical bindings and theory snapshots directly | partial | `src/harness.rs`, `tests/integration/harness_survey_fixture.rs` |  | preserve explicit fixture injection while moving production callers |
| heads | none | Legacy frame heads are unrelated | not needed | `src/heads.rs` | No declaration relationship | none |
| ignore | none | Ignore policy is capability-local runtime input | not needed | `src/ignore.rs` | No declaration relationship | none |
| init | adapter | World init resolves exactly one docs selection and loads owner bodies | partial | `src/init/world/tooling.rs` |  | select one generic binding by addressed target |
| logging | none | Logging only observes runtime behavior | not needed | `src/logging.rs` | No new observability contract is required | none |
| merkle traversal | none | Traversal executes below capability implementations | not needed | `src/merkle_traversal.rs` | No declaration relationship | none |
| metadata | none | Metadata schemas are unchanged | not needed | `src/metadata.rs` | No declaration relationship | none |
| prompt context | none | Prompt artifacts remain capability behavior | not needed | `src/prompt_context.rs` | No declaration relationship | none |
| provider | consume | Physical binding resolves a provider identity and runtime executors consume it | complete | `src/provider.rs` |  | reuse unchanged |
| runtime | adapter | Receipt resolution is generic, but hydration calls docs PDS directly | partial | `src/runtime/theory.rs`, `src/runtime/assembly.rs` |  | activate owner products without expression dispatch |
| serve | none | Server composition has no stewardship selection path in scope | not needed | `src/serve.rs` | No current direct relationship | none |
| session | none | Session identity is runtime state | not needed | `src/session.rs` | No declaration relationship | none |
| store | none | Product storage layout already owns theory storage | not needed | `src/store.rs`, `src/runtime/storage.rs` | No new persistence group is required | none |
| task | adapter | Package triggers carry physical execution inputs but not the selected belief family | partial | `src/task/package.rs`, `meld-execution` package contracts |  | extend the existing trigger with an optional family binding used only by enabled belief hydration |
| telemetry | observe | Existing diagnostics expose unresolved composition | complete | `src/telemetry.rs` |  | reuse unchanged |
| tree | none | Tree utilities are capability internals | not needed | `src/tree.rs` | No declaration relationship | none |
| types | none | Shared root types do not own declaration semantics | not needed | `src/types.rs` | No new shared primitive is required | none |
| views | none | Presentation models do not expose declaration state in this step | not needed | `src/views.rs` | Inspection is deferred beyond the lowering seam | none |
| workflow | adapter | Registered workflow preparation forwards package trigger inputs | partial | `src/workflow/executor.rs` |  | forward no binding for ordinary compatibility routes and fail truthfully if belief hydration is enabled without one |
| workspace | consume | The target root is resolved as physical scope | complete | `src/workspace.rs`, config storage path resolver |  | reuse unchanged |
| world state | consume | Projection consumes requested dimensions after activation | complete | `src/world_state.rs` |  | reuse unchanged |
| meld events | none | Event contracts remain unchanged | not needed | `crates/meld-events` | No declaration contract crosses this owner | none |
| meld execution | consume | Exact capability catalog and authorization lineage already drive realization | complete | `crates/meld-execution` |  | reuse unchanged |
| meld lang | none | Goal and proposition contracts remain unchanged | not needed | `crates/meld-lang` | Step 3 owns maintained-condition lowering | none |
| meld world model | own | Strategy package validation exists, but activation into the Agent runtime template is docs-owned today | partial | `crates/meld-world-model/src/agent/strategy.rs`, `src/docs/pds.rs` |  | move package activation to the Agent and Strategy boundary |

## Frozen Affected Domain Set

The affected set is capability, cli, config, context, docs, error, execution, harness, init, provider, runtime, task, telemetry, workflow, workspace, world state, meld execution, and meld world model.

Changed behavior is limited to capability activation, config declaration lowering, declaration-selected belief-context hydration, docs executor publication, initialization selection, runtime composition, CLI routing, package trigger binding, and meld world model Strategy activation. The remaining affected domains are traversed or characterized and are reused unchanged.

## Pass Two Affected Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| declaration collection | config | one optional docs freshness selection | carry arbitrary named declarations | new local behavior | freezing a universal PDS language too early | `src/config/stewardship/selection.rs` |
| compatibility selection | config | existing docs freshness table | lower into the canonical declaration shape | extend existing | legacy shape becoming the extension model | `src/config/stewardship/selection.rs` |
| source attribution | config | origin capture assumes one docs table | attribute arbitrary declaration fields | extend existing | losing actionable config diagnostics | `src/config/merge/service.rs` |
| physical binding | config | resolves one docs selection | resolve all declarations deterministically | extend existing | environment-dependent or ambiguous target selection | `src/config/stewardship/binding.rs` |
| runtime selection | cli | direct optional docs field check | choose at most one binding for the addressed workspace | adapter only | silently choosing among colliding declarations | `src/cli/runtime_assembly.rs` |
| initialization selection | init | direct single-selection resolution | choose exactly one binding for the explicit target | adapter only | initializing a different declaration than addressed | `src/init/world/tooling.rs` |
| authored source loading | init | bodies selected by package ids | retain identity-based owner loading | reuse unchanged | expression folder conventions leaking into runtime | `src/init/world/source.rs` |
| receipt resolution | runtime | exact package resolution is expression-neutral | retain exact active receipt resolution | reuse unchanged | current-head fallback | `src/runtime/theory.rs` |
| Strategy activation | meld world model | docs PDS builds an Agent Strategy template | activate installed package against supplied subject and Agent | extend existing | docs or root owning Strategy semantics | `src/docs/pds.rs`, `crates/meld-world-model/src/agent/strategy.rs` |
| executable implementation binding | docs | docs PDS registers five invokers | register matching exact docs contracts and reject identity drift | extend existing | binding by expression name instead of contract identity | `src/docs/capability.rs` |
| product activation routing | runtime | hydration calls docs PDS directly | compose built-in capability publishers and require exact coverage | adapter only | root becoming semantic authority | `src/runtime/assembly.rs` |
| belief-context family routing | context | hydration queries one compiled docs family id | accept the selected family id as a physical query input and preserve it in the seeded bundle | extend existing | context choosing application vocabulary | `src/context/belief_context.rs` |
| package trigger binding | task and meld execution | trigger carries target, Agent, provider, and frame identities | carry the optional selected family identity for enabled belief hydration | extend existing | a second declaration surface emerging in task contracts | `crates/meld-execution/src/task/package/contracts.rs`, `src/task/package.rs` |
| dispatch route propagation | runtime and workflow | stewardship route seed omits family identity | copy the selected family from the physical binding through dispatch preparation | adapter only | fallback to a compiled family when the binding is absent | `src/runtime/assembly.rs`, `src/runtime/ports.rs`, `src/cli/route.rs` |
| planning catalog | meld execution | consumes exact capability contracts | accept receipt-built catalog unchanged | reuse unchanged | substituting a current contract | `crates/meld-execution/src/planning/runtime.rs` |
| execution dispatch | meld execution | resolves invokers by type and version | consume exact implementation registry unchanged | reuse unchanged | invoker and contract drift | `crates/meld-execution/src/capability/invocation.rs` |
| provider binding | provider | validated provider identity and overrides | supply the same process-local binding | reuse unchanged | provider settings entering semantic identity | `src/provider.rs` |
| harness injection | harness | direct snapshots and bindings support characterization | retain explicit non-production injection | reuse unchanged | compatibility injection becoming production authority | `src/runtime/assembly.rs` |
| assembly diagnostics | error and telemetry | stable unresolved diagnostics already exist | preserve stable failure codes | reuse unchanged | lowering failures becoming silent absence | `src/runtime/theory.rs` |
| physical workspace scope | workspace | canonical target and external storage root | retain current validation | reuse unchanged | declaration lowering creating workspace-local semantic state | `src/config/stewardship/binding.rs` |
| belief projection | world state | consumes package requested dimensions | retain activated package dimensions | reuse unchanged | root restating domain dimensions | `src/runtime/assembly.rs` |

## Ownership And Boundary Synthesis

Config owns the canonical declaration shape, source attribution, compatibility lowering, and physical binding. Meld world model owns activation of the installed Strategy package into the Agent runtime input. Docs owns construction of docs capability invokers and validation of its exact claim policy. Context owns belief-bundle hydration but receives the family identity from the selected physical binding through the existing package trigger. Meld execution continues to own capability catalogs, invoker lookup, planning, and realization.

Root CLI, init, and runtime assembly only select by target, call owner contracts, verify complete exact coverage, and route the resulting products. They must not branch on expression names or reconstruct semantic body fields.

The smallest missing connective behavior is a declaration collection, two owner-facing activations, and propagation of the selected family identity through the existing package trigger. No new semantic body contract is needed for planning, task execution, events, or world state.

## Explicit Non-Integration Decisions

- No Method, available-action, realization, or workflow path is restored.
- No event schema or declaration event is added.
- No maintained condition or authority model is inferred from the current selection.
- No generic theory registry interprets owner bodies.
- No runtime actor learns expression names.
- No capability implementation is selected by expression name.
- No belief family is selected by context or workflow application vocabulary.
- No new product store is created.

## Boundary Risks

- Treating the minimal declaration as the final principal-facing language would exceed current maturity.
- Keeping docs PDS as the production composer would leave the root dispatch defect intact.
- Moving docs claim semantics into root capability activation would violate domain ownership.
- Selecting the first declaration for a target would make configuration order authoritative. Ambiguity must fail.
- Accepting an exact contract without a matching executable implementation would create a latent dispatch failure. Assembly must fail the binding instead.

## Unresolved Questions

No ownership question blocks this vertical. The future generic form for expression-specific semantic dependencies remains intentionally open for the CVE proof. Step 2 preserves the Step 1 receipt inventory and removes expression dispatch without claiming that the six-unit docs image is the final package manifest schema.
