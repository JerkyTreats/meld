# Theory Durability Symmetry Assessment By Domain

Date: 2026-08-12
Status: ready for workstream initiation
Evidence basis: `implementation/docs-freshness-strategy-runtime` at `c4dcda99`
Scope: Theory Elevation Step 1

## Concern And Scope

The behavior under assessment is durable installation and exact historical resolution of every operational-theory body used by the docs freshness runtime. A runtime turn must resolve immutable revisions from the owning domains, carry those revisions through its durable lineage, and remain inspectable after a newer revision becomes current. The concern does not define a generic stewardship declaration or broaden the runtime to another expression.

In scope:

- stable theory identity and content-derived revision identity
- append-only installation and exact historical resolution
- per-domain registry ownership
- initialization through domain install commands
- immutable per-turn resolution
- lineage from curation through Strategy, capability realization, outcome interpretation, and belief
- truthful unresolved behavior when selected theory is absent or invalid
- compatibility preservation for docs freshness

Out of scope:

- generic stewardship declarations
- removal of root expression dispatch
- maintained conditions
- authority grants and policy intersection
- CVE freshness
- settled replay and Strategy promotion
- method or workflow restoration
- a central PDS registry

Applicable policy is [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md). The delivery contract is [Theory Durability Symmetry Workstream](theory_durability_symmetry_workstream.md).

## Regenerated Domain Snapshot

The root domain set was regenerated with:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

The independently owned workspace crates were regenerated from the workspace member list in `Cargo.toml`.

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

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Root adapter does not own cognitive Agent records | `not needed` | `src/agent.rs` | Agent theory and decisions live in `meld-world-model` | none |
| `api` | `none` | Context API has no theory authority | `not needed` | `src/api.rs` | No new user operation is required for Step 1 | none |
| `branches` | `none` | Branch identity is reused unchanged | `not needed` | `src/branches.rs` | Theory revision is independent of branch catalog behavior | none |
| `capability` | `publish` | Publishes content-identified contracts and process-local executors | `partial` | `src/capability/contracts.rs`, `src/capability/catalog.rs` |  | Publish durable semantic views and exact outcome bindings |
| `cli` | `adapter` | Directly composes the docs PDS and XDG mapping body | `partial` | `src/cli/runtime_assembly.rs` |  | Route selected identities to installation and resolution contracts |
| `compat` | `none` | Reexports unrelated compatibility surfaces | `not needed` | `src/compat.rs` | No compatibility shim is required beyond existing config readers | none |
| `concurrency` | `none` | Existing synchronization remains sufficient | `not needed` | `src/concurrency.rs` | Registry consistency belongs to each store contract | none |
| `config` | `adapter` | Docs-only selection names three theory identities | `partial` | `src/config/stewardship/selection.rs` |  | Extend the compatibility selection with the complete Step 1 identity set |
| `context` | `none` | Context frames are not theory state | `not needed` | `src/context.rs` | No context projection change is required | none |
| `control` | `none` | Control orchestration does not consume the docs theory image | `not needed` | `src/control.rs` | No direct relationship | none |
| `docs` | `own` | Claim policy and the complete PDS image are compiled constants | `partial` | `src/docs/pds.rs`, `src/docs/claim_validation.rs` |  | Own claim-policy revisions and consume installed foreign revisions |
| `error` | `none` | Existing domain errors can carry registry failures | `not needed` | `src/error.rs` | Stable error taxonomy is not part of this slice | none |
| `events` | `none` | Ledger mechanics are reused unchanged | `not needed` | `src/events.rs`, `crates/meld-events/src` | Theory revisions are domain records, not event-ledger semantics | none |
| `execution` | `none` | Root execution ports do not own the theory bodies | `not needed` | `src/execution.rs` | Execution ownership is assessed in `meld-execution` | none |
| `harness` | `observe` | Can display current runtime and proof state | `partial` | `src/harness.rs` |  | Reuse the existing proof surface for revision evidence only |
| `heads` | `none` | Legacy head index is unrelated | `not needed` | `src/heads.rs` | Explicit compatibility non-integration | none |
| `ignore` | `none` | Filesystem exclusion policy is unrelated | `not needed` | `src/ignore.rs` | No direct relationship | none |
| `init` | `adapter` | Provisions three bodies and installs only belief-family revisions | `partial` | `src/init/world/source.rs`, `src/init/world/pipeline.rs` |  | Coordinate validation and per-domain install commands |
| `lib` | `none` | Crate exports need no semantic authority | `not needed` | `src/lib.rs` | Stable domain APIs can remain reached through their owners | none |
| `logging` | `none` | Logs are not durable theory lineage | `not needed` | `src/logging.rs` | Diagnostics may reuse logging without changing it | none |
| `merkle_traversal` | `none` | Traversal is unrelated infrastructure | `not needed` | `src/merkle_traversal.rs` | PDS does not require Merkle theory storage | none |
| `metadata` | `none` | Frame metadata does not carry cognitive theory lineage | `not needed` | `src/metadata.rs` | No direct relationship | none |
| `prompt_context` | `none` | Prompt artifacts are runtime inputs, not theory bodies | `not needed` | `src/prompt_context.rs` | No prompt contract changes are required | none |
| `provider` | `none` | Provider binding is physical configuration | `not needed` | `src/provider.rs` | Provider identity remains outside theory revision ownership | none |
| `runtime` | `adapter` | Injects process-local theory bodies and unresolved diagnostics | `partial` | `src/runtime/assembly.rs` |  | Resolve domain revisions and freeze an immutable turn snapshot |
| `serve` | `none` | Served harness surface needs no new authority | `not needed` | `src/serve.rs` | Harness evidence can use existing presentation | none |
| `session` | `none` | Command lifecycle is independent | `not needed` | `src/session.rs` | Theory lifecycle must not replace session lifecycle | none |
| `store` | `none` | Persistence primitives do not interpret theory | `not needed` | `src/store.rs` | Domain stores may reuse sled without making store authoritative | none |
| `task` | `none` | Root task facade reuses execution contracts | `not needed` | `src/task.rs` | No task schema change is required for recoverability | none |
| `telemetry` | `none` | Telemetry is not historical semantic authority | `not needed` | `src/telemetry.rs` | Existing diagnostics are sufficient | none |
| `tree` | `none` | Filesystem tree semantics are unrelated | `not needed` | `src/tree.rs` | No direct relationship | none |
| `types` | `none` | Shared root types do not own theory | `not needed` | `src/types.rs` | No new generic root type is justified | none |
| `views` | `none` | Compatibility presentation is unrelated | `not needed` | `src/views.rs` | No new operator projection is required to complete Step 1 | none |
| `workflow` | `none` | The live Strategy route has no workflow dependency | `not needed` | `src/workflow.rs` | Restoring stale planning theory is explicitly out of scope | none |
| `workspace` | `none` | Workspace remains physical source truth | `not needed` | `src/workspace.rs` | Theory installation must not move into the target workspace | none |
| `world_state` | `none` | Root reexport adds no ownership | `not needed` | `src/world_state.rs` | World-model changes remain behind the crate contract | none |
| `meld-events` | `none` | Canonical append and replay are complete substrate | `not needed` | `crates/meld-events/src` | No event contract is needed for registry semantics | none |
| `meld-execution` | `publish` | Owns executable capability contracts and exact authorization copies | `partial` | `crates/meld-execution/src/capability/contracts.rs`, `crates/meld-execution/src/goals/contracts.rs` |  | Publish resolvable contract revisions and preserve exact realization checks |
| `meld-lang` | `none` | Provides the serializable semantic IR | `not needed` | `crates/meld-lang/src` | Existing values can be stored without language changes | none |
| `meld-world-model` | `own` | Belief families are durable; mappings and Strategy theory are process injected | `partial` | `crates/meld-world-model/src/belief/registry_store.rs`, `crates/meld-world-model/src/strategy/contracts.rs` |  | Add owner-specific registries and revision lineage |

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

The harness is observation-only. The domains with changed authoritative behavior are `capability`, `docs`, `meld-execution`, and `meld-world-model`. The `cli`, `config`, `init`, and `runtime` domains remain adapters.

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Contract identity | `capability` | Capability bodies have deterministic content identities | Publish exact bodies behind historical resolution | `extend existing` | Treating identity alone as durable resolution | `src/capability/contracts.rs` |
| Strategy view publication | `capability` | Docs code derives `StrategyCapability` values in memory | Publish a revisioned view set tied to exact contract bodies | `new local behavior` | Moving executable ownership into world-model Strategy | `src/docs/pds.rs`, `crates/meld-world-model/src/strategy/contracts.rs` |
| Executor binding | `capability` | Executors register only in process | Bind an executor to the exact installed semantic dependencies used by the turn | `extend existing` | Attempting to persist executable closures | `src/capability/catalog.rs`, `src/capability/runtime.rs` |
| Runtime command routing | `cli` | Docs selection triggers direct composition | Delegate install and resolution through domain contracts | `adapter only` | CLI becoming theory authority | `src/cli/runtime_assembly.rs` |
| Compatibility selection | `config` | Selection names belief, evidence, and curation identities | Name every Step 1 installable identity while remaining docs compatible | `extend existing` | Accidental Step 2 declaration design | `src/config/stewardship/selection.rs` |
| Claim-policy persistence | `docs` | `DocsClaimPolicy` has content identity but is compiled | Install and resolve append-only policy revisions | `new local behavior` | Centralizing docs semantics in root runtime | `src/docs/claim_validation.rs`, `src/docs/pds.rs` |
| Docs capability composition | `docs` | Composer constructs policy, views, and Strategy activation | Consume immutable installed revisions and publish concrete executors | `extend existing` | Retaining silent compiled fallback | `src/docs/pds.rs` |
| Proof projection | `harness` | Existing walk and manifest expose runtime lineage | Show or assert revision references using existing projection seams | `reuse unchanged` | Turning the harness into source truth | `src/harness/walk.rs`, `src/harness/manifest.rs` |
| Source provisioning | `init` | Three authored bodies are copied to XDG paths | Validate and provision the full Step 1 authored image | `extend existing` | Partial writes appearing activated | `src/init/world/source.rs` |
| Domain installation | `init` | Stage 2 installs only belief-family revisions | Invoke each owner install command and report every exact revision | `extend existing` | Root init implementing registry semantics | `src/init/world/pipeline.rs` |
| Theory resolution | `runtime` | Assembly injects mapping and compiled docs bodies | Resolve selected revisions through owners before actor construction | `adapter only` | Root assembly interpreting application semantics | `src/runtime/assembly.rs`, `src/cli/runtime_assembly.rs` |
| Turn snapshot | `runtime` | Process-local bindings are mutable composition inputs | Freeze all revisions used by one turn before semantic work begins | `extend existing` | Mixed-revision work when a current head changes mid-turn | `src/runtime/assembly.rs` |
| Capability contract registry | `meld-execution` | Contracts validate and hash but lack historical resolution | Expose an execution-owned revision contract for exact executable meaning | `new local behavior` | Capability and execution ownership becoming ambiguous | `crates/meld-execution/src/capability/contracts.rs` |
| Authorized realization | `meld-execution` | Execution retains exact contract ids and composition | Revalidate against the resolved exact revisions supplied by capability publication | `extend existing` | Resolving only the latest contract | `crates/meld-execution/src/goals/contracts.rs`, `crates/meld-execution/src/planning/runtime.rs` |
| Outcome mapping | `meld-world-model` belief | Mapping sets validate but are injected from files | Install, resolve, and cite exact mapping revisions | `new local behavior` | Root assembly remaining the mapping authority | `crates/meld-world-model/src/belief/outcome/interpretation.rs` |
| Strategy activation theory | `meld-world-model` Strategy | Settlement, routes, policy, and bounds are immutable values without registry lineage | Install and resolve a complete activation revision | `new local behavior` | One registry swallowing capability or docs semantics | `crates/meld-world-model/src/strategy/contracts.rs`, `crates/meld-world-model/src/agent/strategy.rs` |
| Curation lineage | `meld-world-model` Agent | Rule body and hash live on the Agent record | Cite the exact curation-rule revision on each decision | `extend existing` | Assuming the current Agent record proves historical use | `crates/meld-world-model/src/agent/contracts.rs` |
| Strategy authorization lineage | `meld-world-model` Agent | Authorization cites policy identity and exact candidate | Cite the exact Strategy and capability-view revisions used | `extend existing` | Recovering only ids whose bodies cannot be resolved | `crates/meld-world-model/src/strategy/contracts.rs` |
| Belief lineage | `meld-world-model` belief | Belief revisions cite family theory but not mapping theory | Preserve mapping revision through admitted evidence and resulting belief | `extend existing` | Losing interpretation semantics after ingestion | `crates/meld-world-model/src/belief/contracts.rs` |

## Ownership And Boundary Synthesis

`meld-world-model` owns Strategy activation theory, evidence-mapping theory, curation-decision lineage, and belief lineage. `capability` publishes the Strategy-facing semantic view, while `meld-execution` owns the executable contract body and exact realization validation. `docs` owns claim-policy meaning and the concrete capability executors that consume it.

Root `init` coordinates owner commands but does not implement registry behavior. Root `runtime` resolves and freezes domain-owned revisions but does not interpret their meaning. `config` and `cli` remain compatibility adapters. The harness observes evidence without becoming an authority.

The smallest missing connective behavior is an immutable revision-reference set carried from runtime resolution into Agent authorization, capability activation, outcome interpretation, and belief provenance.

Runtime path domains reused unchanged include events, workspace, provider, task network, and the shared language. They are not implementation write scope merely because docs freshness traverses them.

## Explicit Non-Integration Decisions

- No central PDS registry is introduced. Each semantic owner exposes its own typed install and resolution contract.
- No theory body is stored under the stewarded workspace.
- The event spine does not become the theory registry.
- The harness, CLI, runtime assembly, and config loader do not become semantic authorities.
- `meld-lang` receives no theory-specific variants.
- Method files, workflows, maintained conditions, authority grants, and the second expression remain outside Step 1.
- Exact historical resolution does not yet imply settled operational replay. Replay belongs to Theory Elevation Step 6.

## Scope Separation

Runtime path domains include docs, capability, world model, Agent, execution, task network, events, workspace, provider, runtime, CLI, and harness.

Domains with changed authoritative behavior are `capability`, `docs`, `meld-execution`, and `meld-world-model`.

Likely file scope is intentionally deferred to implementation planning. The smallest seams are the existing registry contracts, initialization stage 2, stewardship theory bindings, Agent authorization records, capability catalogs, and belief evidence provenance.

## Unresolved Questions

No ownership question blocks initiation. The workstream contract fixes the following boundaries:

- Search bounds, evaluation policy, requested projection dimensions, settlement rules, and prospective evidence routes form one world-model Strategy activation revision.
- Capability Strategy views and outcome contract bindings form a capability-published revision that references execution-owned capability contract revisions.
- Claim validation policy remains owned by `docs` and is referenced by exact policy revision from executor activation and substantive outcomes.

The generic declaration shape and long-term package schema remain intentionally unresolved until Theory Elevation Step 2 and the second-expression evidence gate.
