# Standing Maintained Condition Assessment By Domain

Date: 2026-08-13
Status: implementation input
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Scope: Theory Elevation Step 3

## Concern

Represent the standing responsibility behind stewardship as a first-class durable maintained condition that survives individual Goal lifecycles and causes zero or more transient Goals as world state diverges and recovers. The Agent domain must own the condition, its exact installed revision, its breach interpretation, and its Goal lineage. Root initialization and runtime assembly may install, resolve, and route that owner contract but may not reconstruct its semantics.

## Direct Behavior

One selected maintained condition is installed as an exact Agent-owned revision and included in the complete stewardship receipt. Agent curation grounds the condition against the observed subject, evaluates the current projection, emits no Goal while the condition holds, and emits or reopens a transient Goal when it is breached. Every such Goal cites the maintained-condition identity that caused it.

## In Scope

- a minimal Agent-owned maintained-condition contract
- append-only exact maintained-condition revisions
- selection and complete-receipt identity
- authored docs maintained-condition theory
- initialization installation and Agent binding
- runtime exact resolution and curation activation
- Goal provenance naming the causing maintained condition
- compatibility parity for older threshold-only curation fixtures
- bounded lifecycle proof for hold, breach, restore, and later drift

## Out Of Scope

- authority grants or restrictions from Step 4
- the CVE expression from Step 5
- settled Strategy replay from Step 6
- multiple maintained conditions per Agent
- temporal stability windows, hysteresis, or scheduling policy
- generalized principal-facing declaration syntax
- condition discovery or automatic assignment
- hot replacement inside an assembled runtime

## Maturity Envelope

This step establishes the smallest durable standing-condition seam required by the running docs vertical. One Agent has one selected maintained condition. The condition reuses existing proposition and priority contracts, existing belief delivery, existing projection evaluation, and existing Goal lifecycle epochs. It does not freeze a universal stewardship package schema or invent authority and temporal policy before the second expression tests those needs.

## Evidence Basis

- current repository on branch `implementation/theory-durability-symmetry`
- [Theory Elevation Program](theory_elevation_program.md)
- [PDS Architectural Invariants](../../persistent_domain_stewardship/architectural_invariants.md)
- [PDS Code Expression Map](../../persistent_domain_stewardship/code_expression_map.md)
- `crates/meld-world-model/src/agent/contracts.rs`
- `crates/meld-world-model/src/agent/curation.rs`
- `crates/meld-world-model/src/agent/curation_registry.rs`
- `crates/meld-lang/src/goal.rs`
- `src/init/world/pipeline.rs`
- `src/runtime/theory.rs`
- `src/runtime/assembly.rs`

Applicable policy is [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md). Compatibility work follows [Compatibility Shim Policy](../../../governance/compatibility_shim_policy.md).

## Regenerated Domain Snapshot

The current domain set was regenerated with:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
find crates -mindepth 1 -maxdepth 2 -type f -name Cargo.toml -printf '%h\n' | sort
```

The root domain set contains agent, api, branches, capability, cli, compat, concurrency, config, context, control, docs, error, events, execution, harness, heads, ignore, init, lib, logging, merkle traversal, metadata, prompt context, provider, runtime, serve, session, store, task, telemetry, tree, types, views, workflow, workspace, and world state. Independently owned workspace domains are meld events, meld execution, meld lang, and meld world model.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| agent | none | Root Agent profiles do not own world-model stewardship Agents | not needed | `src/agent.rs` | The condition belongs to the world-model Agent record | none |
| api | none | Context facade is downstream of curation | not needed | `src/api.rs` | No maintained-condition command crosses this facade | none |
| branches | none | Branch scope is already part of Agent and belief identity | not needed | `src/branches.rs` | No branch behavior changes | none |
| capability | none | Capability contracts describe executable actions | not needed | `src/capability.rs` | A desired state is not a capability | none |
| cli | adapter | CLI delegates world initialization and runtime composition | complete | `src/cli.rs` |  | reuse unchanged |
| compat | none | Compatibility facade does not expose stewardship theory | not needed | `src/compat.rs` | No direct relationship | none |
| concurrency | none | Existing bounded actors own coordination | not needed | `src/concurrency.rs` | No new shared execution primitive | none |
| config | own | Theory selection has no maintained-condition identity | partial | `src/config/stewardship/selection.rs` |  | add one selected owner identity |
| context | none | Context consumes belief projections after curation | not needed | `src/context.rs` | Desired-state ownership does not enter prompt context | none |
| control | none | Control orchestration does not curate stewardship Goals | not needed | `src/control.rs` | No direct relationship | none |
| docs | publish | Docs source publishes belief, curation, Strategy, and claim theory | partial | `theory/docs_freshness` |  | publish the docs maintained condition as data |
| error | adapter | Existing config, storage, and theory errors carry diagnostics | complete | `src/error.rs` |  | reuse stable error surfaces |
| events | none | Existing Goal and decision paths already persist causal boundaries | not needed | `src/events.rs` | No new root event contract is required | none |
| execution | consume | Execution stores and mutates transient Goals | complete | `src/execution.rs`, `meld-execution` |  | consume the extended Goal provenance unchanged |
| harness | consume | Harness fixtures build Agent and curation theory directly | partial | `src/harness.rs` |  | preserve threshold-only compatibility and add standing-condition fixtures |
| heads | none | Legacy head indexes are unrelated | not needed | `src/heads.rs` | No direct relationship | none |
| ignore | none | Ignore policy is capability-local | not needed | `src/ignore.rs` | No direct relationship | none |
| init | adapter | Stage 2 installs curation theory and stage 3 binds the Agent | partial | `src/init/world/pipeline.rs` |  | install and bind the exact condition revision |
| lib | adapter | Root exports only selected config and runtime contracts | partial | `src/lib.rs` |  | expose no foreign semantic body |
| logging | none | Logs observe existing reports | not needed | `src/logging.rs` | No new logging contract | none |
| merkle traversal | none | Traversal is below selected capabilities | not needed | `src/merkle_traversal.rs` | No direct relationship | none |
| metadata | none | Generation metadata is unrelated | not needed | `src/metadata.rs` | No direct relationship | none |
| prompt context | none | Prompt lineage does not own stewardship intent | not needed | `src/prompt_context.rs` | No direct relationship | none |
| provider | none | Provider binding is execution infrastructure | not needed | `src/provider.rs` | Desired state is provider independent | none |
| runtime | adapter | Exact receipts omit maintained conditions and actor assembly injects only curation rules | partial | `src/runtime/theory.rs`, `src/runtime/assembly.rs` |  | resolve and route the exact owner revision |
| serve | none | Server composition has no maintained-condition surface | not needed | `src/serve.rs` | No direct relationship | none |
| session | none | Command lifecycle is not standing responsibility | not needed | `src/session.rs` | No direct relationship | none |
| store | none | Physical storage primitives do not own semantic records | not needed | `src/store.rs` | Owner registries use existing storage | none |
| task | none | Tasks realize authorized transient Goals | not needed | `src/task.rs` | Task completion remains distinct from condition restoration | none |
| telemetry | observe | Existing actor diagnostics expose unresolved bindings | complete | `src/telemetry.rs` |  | reuse unchanged |
| tree | none | Tree utilities are capability internals | not needed | `src/tree.rs` | No direct relationship | none |
| types | none | No root shared primitive is required | not needed | `src/types.rs` | Owner contracts remain in their domain crates | none |
| views | none | No presentation surface is requested | not needed | `src/views.rs` | Inspection UI remains deferred | none |
| workflow | none | Docs stewardship closes without workflow routing | not needed | `src/workflow.rs` | A condition must not become a workflow | none |
| workspace | publish | Workspace identity supplies the grounded subject | complete | `src/workspace.rs` |  | reuse unchanged |
| world state | publish | Planner projection supplies the state evaluated against the condition | complete | `src/world_state.rs` |  | reuse unchanged |
| meld events | none | Domain event contracts do not change | not needed | `crates/meld-events` | Existing causal records are sufficient | none |
| meld execution | consume | Goal storage and lifecycle mutation carry the language-owned Goal | complete | `crates/meld-execution` |  | preserve behavior under extended provenance |
| meld lang | own | Goal provenance lacks a maintained-condition cause | partial | `crates/meld-lang/src/goal.rs` |  | add explicit maintained-condition breach provenance |
| meld world model | own | Curation rule currently owns desired threshold, priority, and Goal template | partial | `crates/meld-world-model/src/agent` |  | own condition contract, registry, grounding, evaluation, and lineage |

## Frozen Affected Domain Set

The affected set is cli, config, docs, error, execution, harness, init, lib, runtime, telemetry, workspace, world state, meld execution, meld lang, and meld world model.

Changed behavior is limited to config selection, docs theory publication, initialization, runtime receipt resolution and assembly, meld lang Goal provenance, and meld world model Agent contracts and curation. The remaining affected domains publish inputs, consume Goals, expose diagnostics, or are characterized unchanged.

## Pass Two Affected Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| theory identity selection | config | selection names six exact owner bodies | name one maintained-condition body | extend existing | treating the minimal selection as final declaration syntax | `src/config/stewardship/selection.rs` |
| authored condition body | docs | threshold and Goal metadata live in curation-rule JSON | publish an intact maintained-condition body | new local behavior | docs semantics leaking into root | `theory/docs_freshness/curation_rule.docs_freshness.json` |
| source provisioning | init | validates and copies known theory kinds | validate and copy the selected maintained condition before writes | adapter only | partial source placement | `src/init/world/source.rs` |
| owner installation | init | installs exact curation and Strategy revisions | call the Agent-owned condition registry and include its revision in the receipt | adapter only | root interpreting the condition | `src/init/world/pipeline.rs` |
| Agent genesis binding | init | binds exact curation revision to the Agent record | bind the exact maintained-condition revision beside it | adapter only | Agent record and receipt drifting | `src/init/world/pipeline.rs` |
| receipt identity | runtime | complete receipt pins six semantic units | pin the maintained-condition revision | extend existing | historical activation silently using a current condition | `src/runtime/theory.rs` |
| exact resolution | runtime | resolves every receipt reference once | resolve and cross-check the condition against curation and belief dimensions | extend existing | current-head fallback | `src/runtime/theory.rs` |
| actor activation | runtime | injects exact curation rule into the Agent actor | inject exact maintained-condition binding | adapter only | root constructing Goal semantics | `src/runtime/assembly.rs` |
| condition contract | meld world model | no first-class standing desired state | own identity, dimension, condition, priority, and desired summary | new local behavior | overgeneralizing temporal policy | `crates/meld-world-model/src/agent/contracts.rs` |
| condition durability | meld world model | curation rules have append-only exact revisions | apply the same owner registry pattern | new local behavior | central theory store ownership | `crates/meld-world-model/src/agent/curation_registry.rs` |
| condition grounding | meld world model | curation constructs a threshold Goal from rule fields | ground the installed condition against the Agent subject | extend existing | caller-built Goal templates | `crates/meld-world-model/src/agent/curation.rs` |
| breach curation | meld world model | confidence below a rule threshold emits a Goal | evaluate the grounded desired proposition and emit only on breach | extend existing | task success being treated as restoration | `crates/meld-world-model/src/agent/curation.rs` |
| compatibility curation | meld world model | threshold-only fixtures and records remain active | lower the old rule fields into an ephemeral compatibility condition | extend existing | compatibility becoming the extension model | `crates/meld-world-model/src/agent/contracts.rs` |
| Goal causal provenance | meld lang | belief divergence describes observed and desired text only | name the causing maintained-condition identity | extend existing | world model inventing language-owned provenance | `crates/meld-lang/src/goal.rs` |
| Goal persistence | meld execution | stores language-owned Goal values intact | preserve new provenance without semantic branching | reuse unchanged | execution becoming condition authority | `crates/meld-execution/src/goals` |
| projection publication | world state | exact state supports proposition evaluation | supply unchanged projection | reuse unchanged | condition semantics duplicated in projection | `src/world_state.rs` |
| workspace subject | workspace | physical binding supplies canonical subject identity | supply unchanged identity | reuse unchanged | condition body carrying physical paths | `src/config/stewardship/binding.rs` |
| diagnostics | error and telemetry | missing theory and actor bindings are visible | preserve stable unresolved and corrupt classifications | reuse unchanged | silent absence | `src/runtime/theory.rs` |
| lifecycle characterization | harness | docs convergence proves satisfy and reopen | prove zero Goal while held, one transient Goal on breach, satisfaction on restore, and reopen on later drift | extend existing | counting tasks rather than condition state | `tests/integration/runtime_cli.rs` |

## Ownership And Boundary Synthesis

Meld world model owns maintained-condition meaning, validation, durable revisions, Agent binding, subject grounding, breach evaluation, and curation decisions. Meld lang owns the Goal provenance variant that records why a Goal exists. Config names the owner identity. Docs publishes the docs condition body. Root initialization and runtime assembly install, resolve, cross-check, and route exact revisions only.

Belief and world state publish the current projection. Meld execution consumes transient Goals and lifecycle mutations unchanged. CLI, error, telemetry, workspace, and harness are adapters or characterized consumers rather than semantic owners.

The smallest missing connective behavior is one Agent-owned condition revision carried by the existing selection and complete receipt, plus explicit Goal provenance. No scheduler, workflow, capability, authority, or new event substrate is required.

## Explicit Non-Integration Decisions

- A maintained condition is not a workflow, task template, Method, or capability.
- Task completion does not satisfy the maintained condition.
- Root config and assembly do not evaluate condition semantics.
- Belief does not own the desired state merely because it supplies observed confidence.
- Execution does not interpret maintained-condition identity.
- The compatibility threshold rule is not the extension model.
- Multiple conditions, hysteresis, stability windows, and authority remain deferred.

## Boundary Risks

- Leaving threshold and priority authoritative in the curation rule would create two desired-state owners.
- Omitting the condition revision from the complete receipt would break historical exactness.
- Emitting a Goal without causal identity would keep standing responsibility implicit.
- Making a satisfied Goal absorbing would violate later-drift continuity.
- Treating the condition as a timer would confuse world-state divergence with scheduling.

## Unresolved Questions

No ownership question blocks the vertical. The future shape for multiple conditions and temporal stability remains intentionally open until the second dissimilar expression supplies evidence.
