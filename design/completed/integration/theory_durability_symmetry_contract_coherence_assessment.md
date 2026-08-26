# Theory Durability Symmetry Contract Coherence Assessment

Date: 2026-08-12
Status: reconciled for phased implementation
Evidence basis: `implementation/theory-durability-symmetry` at `28542ac4` plus the uncommitted implementation design
Scope: Theory Elevation Step 1 contract reuse and cross-domain coherence

> Historical assessment of the Step 1 compatibility aggregate. Its recommendation to preserve the complete `StrategyTheoryPackage` is superseded by [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md).

## Concern And Scope

The behavior under assessment is durable installation, exact runtime activation, and historical resolution of the semantic bodies used by docs freshness. This review asks whether the implementation design introduces contracts only where ownership or durability requires a new product, and whether existing public contracts can be extended without duplicating truth or moving authority into root adapters.

In scope:

- revision reference and installed revision shapes
- Strategy activation and capability publication boundaries
- executable capability identity and catalog behavior
- claim policy revision ownership
- complete-install receipt and runtime snapshot boundaries
- lineage fields that cross world-model and execution boundaries
- initialization, storage, runtime, config, CLI, and harness integration

Out of scope:

- implementation sequencing and acceptance test detail
- generic declaration lowering and root expression dispatch removal
- maintained conditions, authority grants, and CVE freshness
- a second expression, Method revival, settled replay, and hot replacement

Applicable method is the canonical assessment-by-domain skill. Contract choices also follow [Runtime Invariants](../../../governance/runtime_invariants.md) and the domain architecture rules in the repository `AGENTS.md`.

## Regenerated Domain Snapshot

The root domain set was regenerated from the current repository with:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

The independently owned workspace domains were regenerated from the workspace member list in `Cargo.toml`.

Root domains:

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
tree
types
views
workflow
workspace
world_state
```

Workspace domains:

```text
meld-events
meld-execution
meld-lang
meld-world-model
```

## Contract Reuse Audit

| Proposed contract | Existing seam | Decision | Reason |
| --- | --- | --- | --- |
| World-model revision references | `TheoryRevisionRef` | extend existing | Its contract already states that it is generic across world-model theory kinds |
| Belief-family installed revision | `BeliefFamilyRevision` | extend existing | Tighten validation and corruption behavior without replacing the public product |
| Curation-rule identity | `TheorySelection`, `AgentCurationRuleConfig`, and `AgentCurationRuleBinding` | reuse selection identity, retain binding only as a shim | Pass the selected id into installation and avoid changing the canonical rule body and its existing hashes |
| Curation-rule installed revision | no durable registry product | new local behavior | Installation sequence and exact historical resolution are new owner behavior |
| Outcome-mapping installed revision | `OutcomeMappingSetConfig` and `OutcomeMappingInput` | extend existing around a new registry record | The config already owns `mapping_id`; the input needs the existing exact revision reference |
| Strategy activation config | `StrategyTheorySnapshot`, `StrategyCapability`, `StrategyEvaluationPolicy`, and `StrategySearchBounds` | replace with one installed aggregate over existing products | A new lifecycle aggregate is justified, but the delivered runtime contracts remain unchanged |
| Capability view config | `StrategyCapability` inside the Strategy problem | remove proposal and fold into Strategy theory | Root capability has no authority for a Strategy-facing semantic view |
| Strategy installed revision | existing Strategy product family | add `StrategyTheoryPackage` and its registry record | The package carries existing products intact and excludes dynamic Goal and planner state |
| Exact executable capability reference | `CapabilityRef` and `CapabilityTypeContract::content_identity` | add one execution-owned exact reference | `CapabilityRef` selects type and version but does not pin content, while `meld-lang` must not gain durability-specific meaning |
| Durable capability registry | `CapabilityCatalog` | add append-only exact storage and reuse the catalog as the activated projection | Different content under one type and version is rejected as version drift |
| Claim policy body | `DocsClaimPolicy` | extend existing | Validation and content identity already exist on the canonical docs product |
| Claim policy installed revision | no durable registry product | new local behavior | Historical resolution needs owner storage metadata without moving policy meaning into root runtime |
| Complete installation receipt | `SelectedStewardshipPackage` | new receipt carrying the existing selection intact | A cross-store commit barrier is new, but expression and stable identities must not be copied into sibling fields |
| Resolved runtime theory | `StewardshipTheoryBindings` | add a complete snapshot and narrow the existing binding | Current bindings mix optional theory, executors, planning compatibility, and dispatch routes |
| Strategy authorization lineage | `StrategyAuthorization` | extend existing | Add the exact Strategy revision only; selected capability identities already live on the candidate |
| Execution authorization lineage | `ExecutionStrategyAuthorization` | extend existing | Preserve the Strategy revision and reuse existing exact capability identities |
| Belief theory lineage | `TheoryRevisionRef`, `OutcomeMappingInput`, `EvidenceItem`, and `EvidenceRejection` | extend existing | Belief revisions already reach mapping lineage through durable evidence ids, so provenance does not copy it |

## Contract Corrections

The implementation design should use six durable semantic units:

1. belief family
2. curation rule
3. outcome mapping
4. complete Strategy theory package
5. executable capability contract
6. docs claim policy

The complete `StrategyTheoryPackage` carries the existing settlement snapshot, Strategy capabilities, evaluation policy, search bounds, and requested projection dimensions intact. `StrategyCapability` remains the world-model receiver-owned projection. Each entry already carries the exact execution contract content identity in `contract_id` and the type and version selector in its operator resolution.

This removes the separate capability-view contract, root capability registry, capability theory database, capability-view selection id, and capability-view lineage field. It also removes the proposed `StrategyActivationConfig` without refactoring the delivered Strategy runtime contracts.

Within the world model, all registries reuse `TheoryRevisionRef`. Execution defines one exact capability contract reference because its content identity has no equivalent in `meld-lang::CapabilityRef`. Docs defines one exact claim-policy reference beside `DocsClaimPolicy`. These owner-specific references are justified by dependency direction and do not create a shared theory vocabulary.

The installation receipt is a new root operational contract. It carries `SelectedStewardshipPackage` intact, adds exact owner references, and derives its selection key and receipt identity through methods. It does not repeat expression or stable selection fields beside the package. Its active head is the only activation pointer added by Step 1.

Physical storage remains coarse at current product maturity. World-model revisions use the existing world-model database. Execution, docs, and receipt trees share one new external theory database while retaining distinct owner APIs.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Root agent configuration does not own cognitive curation records | `not needed` | `src/agent.rs` | Agent theory lives in `meld-world-model` | none |
| `api` | `none` | Root facade has no theory authority | `not needed` | `src/api.rs` | No new public operation is required | none |
| `branches` | `none` | Branch identity is reused unchanged | `not needed` | `src/branches.rs` | Theory selection is not branch catalog behavior | none |
| `capability` | `adapter` | Reexports execution contracts and binds process-local executors | `partial` | `src/capability.rs`, `src/capability/runtime.rs` |  | Expose the exact execution reference if required and keep executor binding process-local |
| `cli` | `adapter` | Directly composes docs theory from authored files and constants | `partial` | `src/cli/runtime_assembly.rs` |  | Route receipt resolution into runtime assembly |
| `compat` | `none` | Root compatibility exports are unrelated | `not needed` | `src/compat.rs` | Serialized domain shims stay with their owners | none |
| `concurrency` | `none` | Existing synchronization remains sufficient | `not needed` | `src/concurrency.rs` | Composition immutability removes a new coordination need | none |
| `config` | `adapter` | Selection names three stable theory identities | `partial` | `src/config/stewardship/selection.rs`, `src/config/stewardship/binding.rs` |  | Extend existing selection and package values with Strategy and claim-policy ids |
| `context` | `none` | Context products carry no theory authority | `not needed` | `src/context.rs` | No direct relationship | none |
| `control` | `none` | Control plans do not resolve stewardship theory | `not needed` | `src/control.rs` | No direct relationship | none |
| `docs` | `own` | Owns claim policy and publishes compiled docs Strategy inputs and executors | `partial` | `src/docs/claim_validation.rs`, `src/docs/pds.rs` |  | Own claim-policy revisions and authored docs publications without owning Strategy contracts |
| `error` | `none` | Existing error adapters can carry owner failures | `not needed` | `src/error.rs` | Stable runtime diagnostics stay in runtime contracts | none |
| `events` | `none` | Event replay transports outcomes unchanged | `not needed` | `src/events.rs` | Theory registries are not event-ledger semantics | none |
| `execution` | `none` | Root execution facade owns no capability theory | `not needed` | `src/execution.rs` | Workspace execution ownership is assessed separately | none |
| `harness` | `observe` | Existing proof surfaces can observe assembled convergence | `complete` | `src/harness.rs` |  | Reuse unchanged without a new lineage query |
| `heads` | `none` | Legacy head storage is unrelated | `not needed` | `src/heads.rs` | The receipt store owns its own active pointer | none |
| `ignore` | `none` | Ignore rules do not affect external theory stores | `not needed` | `src/ignore.rs` | No direct relationship | none |
| `init` | `adapter` | Provisions three bodies and installs one revision kind | `partial` | `src/init/world/source.rs`, `src/init/world/pipeline.rs` |  | Coordinate owner installs and commit the receipt last |
| `lib` | `none` | Existing module exports are sufficient | `not needed` | `src/lib.rs` | New public products remain under owner modules | none |
| `logging` | `none` | Logs are not durable lineage | `not needed` | `src/logging.rs` | No direct relationship | none |
| `merkle_traversal` | `none` | Traversal behavior is unrelated | `not needed` | `src/merkle_traversal.rs` | No direct relationship | none |
| `metadata` | `none` | Metadata does not own theory lineage | `not needed` | `src/metadata.rs` | No direct relationship | none |
| `prompt_context` | `none` | Prompt inputs are not installed theory | `not needed` | `src/prompt_context.rs` | No direct relationship | none |
| `provider` | `none` | Provider identity is physical binding | `not needed` | `src/provider.rs` | Provider state remains outside theory revisions | none |
| `runtime` | `adapter` | Accepts loose optional theory and route bindings | `partial` | `src/runtime/assembly.rs`, `src/runtime/storage.rs` |  | Open owner stores, resolve one receipt, and freeze one complete snapshot |
| `serve` | `none` | Served presentation needs no new authority | `not needed` | `src/serve.rs` | Harness projection can be reused | none |
| `session` | `none` | Command lifecycle is independent | `not needed` | `src/session.rs` | Session state is not theory state | none |
| `store` | `none` | Generic persistence primitives interpret no domain body | `not needed` | `src/store.rs` | Owner stores may reuse sled directly | none |
| `task` | `none` | Task records already trace to Goal authorization | `not needed` | `src/task.rs` | Exact theory need not be copied onto each task | none |
| `telemetry` | `none` | Telemetry is not historical authority | `not needed` | `src/telemetry.rs` | Existing diagnostics suffice | none |
| `tree` | `none` | Filesystem tree behavior is unrelated | `not needed` | `src/tree.rs` | No direct relationship | none |
| `types` | `none` | Shared root types must not become a theory vocabulary | `not needed` | `src/types.rs` | Owner-specific references remain with owners | none |
| `views` | `none` | Presentation views do not own Strategy capability meaning | `not needed` | `src/views.rs` | No direct relationship | none |
| `workflow` | `none` | The live Strategy path does not use workflows | `not needed` | `src/workflow.rs` | Method and workflow restoration is out of scope | none |
| `workspace` | `none` | Workspace remains stewarded source truth | `not needed` | `src/workspace.rs` | Installed theory remains outside the target | none |
| `world_state` | `none` | Root projection facade adds no theory ownership | `not needed` | `src/world_state.rs` | World-model changes remain behind its crate contracts | none |
| `meld-events` | `none` | Append and replay are unchanged substrate | `not needed` | `crates/meld-events/src` | No event contract change is required | none |
| `meld-execution` | `own` | Owns exact contract content identity and runtime catalog lookup | `partial` | `crates/meld-execution/src/capability/contracts.rs`, `crates/meld-execution/src/capability/catalog.rs` |  | Add historical contract resolution and activate exact receipt-selected bodies |
| `meld-lang` | `none` | `CapabilityRef` selects type and version for semantic operators | `not needed` | `crates/meld-lang/src/operator.rs` | Durability-specific content identity does not belong in the primitive language | none |
| `meld-world-model` | `own` | Belief family revisions exist while curation, mapping, and Strategy bodies remain embedded or injected | `partial` | `crates/meld-world-model/src/belief/registry.rs`, `crates/meld-world-model/src/agent/contracts.rs`, `crates/meld-world-model/src/strategy/contracts.rs` |  | Extend its revision vocabulary and add the three missing owner registries |

## Frozen Affected-Domain Set

The affected-domain set is frozen as:

```text
capability
cli
config
docs
harness
init
runtime
meld-execution
meld-world-model
```

The changed authoritative domains are `docs`, `meld-execution`, and `meld-world-model`. Root capability, config, init, runtime, and CLI remain adapters. The harness remains observation-only.

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Contract exposure | `capability` | Reexports execution capability products | Expose exact execution references without defining another identity | `adapter only` | Root becoming a second capability owner | `src/capability.rs` |
| Executor binding | `capability` | Registers live executors against an in-memory catalog | Bind only executors whose contracts match the resolved exact catalog | `extend existing` | Persisting executors or claim policy in the generic registry | `src/capability/runtime.rs` |
| Runtime command routing | `cli` | Builds docs PDS values directly | Delegate selection and exact resolution | `adapter only` | CLI retaining compiled semantic fallback | `src/cli/runtime_assembly.rs` |
| Stable selection | `config` | `TheorySelection` names three owner identities | Add Strategy theory and claim policy identities | `extend existing` | Reintroducing a separate capability-view identity | `src/config/stewardship/selection.rs` |
| Selected package | `config` | `SelectedStewardshipPackage` carries expression and stable ids | Preserve this value intact in the installation receipt | `extend existing` | Receipt duplicating package truth | `src/config/stewardship/binding.rs` |
| Claim policy | `docs` | `DocsClaimPolicy` validates and hashes its body | Install and resolve exact historical policy revisions | `extend existing` | Root runtime interpreting policy fields | `src/docs/claim_validation.rs` |
| Docs publication | `docs` | `pds` composes Strategy views and executors | Publish authored Strategy package input and concrete capability contracts | `extend existing` | Docs becoming owner of generic Strategy or execution contracts | `src/docs/pds.rs` |
| Executor construction | `docs` | Policy-bearing executors accept `DocsClaimPolicy` | Construct them from the exact resolved policy product | `extend existing` | Silent fallback to compiled policy | `src/docs/capability.rs` |
| Lineage observation | `harness` | Existing walk and manifest inspect runtime state | Observe assembled convergence without a new contract | `reuse unchanged` | Harness becoming a theory query authority | `src/harness/walk.rs`, `src/harness/manifest.rs` |
| Source provisioning | `init` | Copies and validates three authored bodies | Provision the Strategy package and claim policy through existing preflight behavior | `extend existing` | Partial source writes appearing activated | `src/init/world/source.rs` |
| Owner installation | `init` | Invokes belief-family installation | Invoke each owner and commit the receipt only after exact cross-validation | `extend existing` | Root implementing semantic validation | `src/init/world/pipeline.rs` |
| Store assembly | `runtime` | Opens grouped stores under external product storage | Add one shared physical theory store with owner-specific trees and APIs | `extend existing` | Physical separation being mistaken for semantic ownership | `src/runtime/storage.rs` |
| Theory resolution | `runtime` | Accepts optional loose bodies | Resolve the complete receipt and carry owner products intact | `adapter only` | Root interpreting theory or consulting owner heads | `src/runtime/assembly.rs` |
| Runtime snapshot | `runtime` | `StewardshipTheoryBindings` mixes theory and physical routes | Add one complete immutable theory snapshot and narrow the old binding to compatibility | `new local behavior` | Treating live executors as persisted theory | `src/runtime/assembly.rs` |
| Exact contract identity | `meld-execution` capability | `CapabilityTypeContract` supplies `content_identity` | Add a typed exact reference derived from the existing contract | `extend existing` | Extending `meld-lang::CapabilityRef` with durability concerns | `crates/meld-execution/src/capability/contracts.rs` |
| Historical contract resolution | `meld-execution` capability | `CapabilityCatalog` is in-memory lookup by type and version | Add append-only exact storage and build the existing catalog from receipt-selected revisions | `new local behavior` | Allowing content drift under one type and version | `crates/meld-execution/src/capability/catalog.rs` |
| Authorized realization | `meld-execution` planning | Revalidates exact capability content ids from authorization | Reuse that identity list against the receipt-selected catalog | `extend existing` | Adding a second capability identity list | `crates/meld-execution/src/planning/runtime.rs`, `crates/meld-execution/src/goals/contracts.rs` |
| Revision vocabulary | `meld-world-model` belief | `TheoryRevisionRef` is documented as reusable across theory kinds | Add validation and reuse it across world-model registries | `extend existing` | Creating one reference type per world-model subdomain | `crates/meld-world-model/src/belief/registry.rs` |
| Curation rule | `meld-world-model` Agent | Selection owns the stable id and binding embeds body plus hash | Pass the selected id to installation, register the unchanged body, and cite its exact reference | `extend existing` | Changing the body would invalidate existing hashes | `crates/meld-world-model/src/agent/contracts.rs` |
| Outcome mapping | `meld-world-model` belief | Mapping config self-identifies but runtime input carries only stable id | Add owner registry and pass the exact existing revision reference | `extend existing` | New mapping reference vocabulary drifting from `TheoryRevisionRef` | `crates/meld-world-model/src/belief/outcome/mapping.rs` |
| Strategy theory | `meld-world-model` Strategy | Snapshot, capabilities, evaluation, and bounds already form public products | Add one installed package that carries those products intact | `new local behavior` | Refactoring the delivered search contracts for persistence | `crates/meld-world-model/src/strategy/contracts.rs` |
| Strategy authorization | `meld-world-model` Agent | Authorization carries candidate and evaluation policy identity | Add exact Strategy revision and reuse candidate capability identities | `extend existing` | Copying a separate capability-view identity | `crates/meld-world-model/src/strategy/contracts.rs` |
| Belief lineage | `meld-world-model` belief | Family revision and durable evidence ids are already cited | Add mapping references to evidence and rejection records only | `extend existing` | Copying reachable mapping references into belief provenance | `crates/meld-world-model/src/belief/contracts.rs` |

## Ownership And Boundary Synthesis

`meld-world-model` owns four revision families: belief family, curation rule, outcome mapping, and the complete Strategy theory package. `meld-execution` owns executable capability contract revisions and the activated exact catalog. `docs` owns claim-policy revisions and publishes docs-authored instances of world-model and execution contracts.

Docs publishes authored bodies to the world-model and execution install boundaries, then consumes the resolved claim policy while constructing concrete executors. World-model Strategy consumes exact execution contract references through `StrategyCapability`. Execution consumes the authorized exact capability identity list without resolving world-model bodies.

Root initialization owns orchestration only. Root runtime owns receipt selection, exact owner routing, and the immutable process-local aggregate. Root capability exposes execution products and binds live executors but owns no durable semantic view. Config and CLI carry identities and routes only. The harness reuses its existing observation surface.

The smallest missing connective behavior is one complete installation receipt over typed owner references. It is necessary because owner writes do not share a transaction. It does not justify owner activation heads, a generic registry, a root semantic body, or another capability projection.

Runtime path domains include docs, capability, world model, execution, task network, events, provider, workspace, runtime, CLI, and harness. Changed authoritative behavior is limited to docs, `meld-execution`, and `meld-world-model`. Likely file changes also include root config, init, runtime, CLI, and capability adapters, but traversal does not make those domains owners.

Events, provider, workspace, task-network behavior, and `meld-lang` are reused unchanged on the product path.

## Explicit Non-Integration Decisions

- Do not create `CapabilityStrategyViewConfig` or a root capability theory registry.
- Do not create `StrategyActivationConfig` or refactor existing Strategy runtime contracts. Add one lifecycle-specific installed package over them.
- Do not add durability fields to `meld-lang::CapabilityRef`.
- Do not create separate world-model reference types for curation, mapping, and Strategy.
- Do not duplicate exact capability identities on execution authorization because `capability_contract_ids` already carries them.
- Do not copy the selected expression and stable ids out of `SelectedStewardshipPackage` into the receipt.
- Do not persist executors, provider bindings, physical scope, or runtime routes as theory.
- Do not add theory fields to task records when Goal authorization already supplies the lineage path.
- Do not add mapping revision lists to belief provenance when evidence ids already provide the path.
- Do not add owner activation heads beyond the compatibility belief-family head.
- Do not add a generic lineage query or new harness product contract.
- Do not make events, the harness, CLI, config, or runtime assembly semantic authorities.

## Evidence Basis

The contract audit is grounded in current world-model registry, Strategy, Agent, outcome mapping, and belief contracts; execution capability, catalog, authorization, and planning contracts; and root capability, config, docs, initialization, runtime assembly, and storage contracts.

## Unresolved Questions

No ownership or contract-shape question blocks implementation design. The installed aggregate is named `StrategyTheoryPackage`, and its registry result is named `StrategyTheoryRevision`.

The compatibility duration for embedded `AgentCurationRuleBinding` remains governed by migration evidence. It does not change the target contract: new Agent records cite an exact `TheoryRevisionRef`, while the embedded body exists only to read and migrate old records.
