# World Model Reconciliation Source Runtime Groundmap

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-08-26

Status: reconstructed first-cut evidence

Concern: deliver the historical `SI-01` owner-publication-to-`TraversalCut` outcome through the one live runtime route, without restoring the withdrawn additive implementation or leaving raw workspace Event attachments as a second owner-knowledge authority.

## Scope And Authority

Direct behavior: one exact workspace source revision becomes a retryable owner publication, passes through the existing Event authority and Graph runtime, and is read through an occurrence-preserving immutable cut by a production branch owner-inspection route.

In scope:

- workspace scan and watch publication
- neutral Event carriage
- the existing Graph reducer, store, runtime, and query facade
- branch graph inspection and its CLI adapter
- replay, restart, lag, incomplete cut, bounds, and equal-endpoint occurrence proof

Out of scope:

- Curation
- Belief settlement changes
- Planner and Strategy changes
- Agent progression
- Execution changes
- PDS genesis and lifecycle
- Startup
- Docs Freshness and Dependency Security product migration
- legacy Workflow retirement

Authority comes from the user goal to implement `SI-01`, the accepted architecture, the [single-runtime recovery](world_model_reconciliation_single_runtime_recovery.md), and the [Runtime Invariants](../../../../governance/runtime_invariants.md). Source edits still require approval of the reconstructed cut recorded below.

Discovery used the full budget of twelve batched inspections and twenty current source items. Confidence is high for the workspace, Event, Graph, runtime assembly, and branch inspection route. Confidence is medium for unrelated owner routes, which remain outside the first cut.

## Domain Universe

The current domain universe comes from `src/lib.rs`, the Cargo workspace, and the public owner modules in `meld-events` and `meld-world-model`.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| agent | none | none | not needed | `src/agent.rs` | no Agent judgment belongs in this cut | none |
| api | adapter | generic routing only | complete | `src/lib.rs` | API owns no graph meaning | preserve thin routing |
| branches | consume | consumes generic graph facts and flattened relations | partial | `src/branches/query.rs` | none | add exact owner inspection and keep structural inspection semantically separate |
| capability | none | workspace scan capability emits candidate artifacts | not needed | `src/workspace/capability.rs` | capability execution does not own publication acceptance | preserve artifact behavior |
| cli | adapter | invokes workspace scan and structural branch inspection | partial | `src/workspace/commands.rs` and `src/branches/query.rs` | none | surface durable publication failures and expose exact owner-cut inspection |
| compat | none | no accepted `SI-01` compatibility authority | not needed | `src/lib.rs` | no second writer or graph reader is justified | none |
| concurrency | none | none | not needed | `src/lib.rs` | no ownership relationship | none |
| config | none | supplies roots only | not needed | `src/runtime/storage.rs` | configuration does not interpret publication meaning | none |
| context | none | publishes separate operational anchor events | not needed | `graph/source_intent.rs` | first cut selects workspace as the one real owner publisher | preserve unchanged |
| control | none | none | not needed | `src/lib.rs` | no ownership relationship | none |
| dependency security | observe | no owner publication runtime | not started | accepted `SI-01` proof | product migration is later | use only a dissimilar test specimen |
| docs | observe | no owner publication runtime | not started | accepted `SI-01` proof | product migration is later | use only a docs test specimen |
| events | adapter | carries producer payload, objects, relations, identity, replay, and cursor | complete | `crates/meld-events/src/events.rs` | Events must not own publication semantics | reuse unchanged |
| execution | none | publishes operational artifact anchors | not needed | `graph/source_intent.rs` | Execution semantics are outside this cut | preserve unchanged |
| harness | observe | reads current anchors and supplies proof fixtures | partial | `src/harness/walk.rs` | harness is not product authority | update only if public contract compilation requires it |
| heads | none | no direct selected behavior | not needed | `src/lib.rs` | no ownership relationship | none |
| ignore | none | shapes workspace enumeration | complete | `src/workspace/scan.rs` | remains workspace policy | reuse unchanged |
| init | none | no selected behavior | not needed | `src/lib.rs` | product initialization is a later cut | none |
| logging | none | diagnostics only | not needed | `src/lib.rs` | no ownership relationship | none |
| merkle traversal | none | no selected behavior | not needed | `src/lib.rs` | workspace scan already owns enumeration | none |
| metadata | none | no selected behavior | not needed | `src/lib.rs` | no ownership relationship | none |
| prompt context | none | no selected behavior | not needed | `src/lib.rs` | no ownership relationship | none |
| provider | none | no selected behavior | not needed | `src/lib.rs` | no ownership relationship | none |
| runtime | own | assembles one Graph runtime over one Event authority and one traversal store | complete | `src/runtime/assembly.rs` and `src/runtime/storage.rs` | none | reuse the existing assembly without a new actor |
| serve | none | no selected behavior | not needed | `src/lib.rs` | no ownership relationship | none |
| session | adapter | `ProgressRuntime` exposes durable and best-effort Event append | partial | `src/telemetry/sessions/service.rs` | session owns append mechanics only | workspace must select idempotent durable append |
| store | publish | workspace node records hold the authoritative source revision material | partial | `src/workspace/scan.rs` | none | bind publication identity to the exact tree revision |
| task | none | no selected behavior | not needed | `src/lib.rs` | task execution is outside this cut | none |
| telemetry | adapter | workspace CLI currently appends publication candidates best effort | partial | `src/workspace/commands.rs` | telemetry does not decide owner completeness | switch owner publication to durable idempotent append |
| theory | none | no selected behavior | not needed | `src/lib.rs` | PDS theory is outside this cut | none |
| tree | publish | produces exact deterministic workspace nodes and root revision | complete | `src/workspace/scan.rs` | none | reuse as reconstruction source |
| types | none | shared primitives only | not needed | `src/lib.rs` | no ownership relationship | none |
| views | none | no selected behavior | not needed | `src/lib.rs` | no ownership relationship | none |
| workflow | none | no selected behavior | not needed | `src/lib.rs` | Workflow is not a compatibility target | none |
| workspace | own | creates deterministic Event candidates but uses best-effort publication | partial | `src/workspace/events.rs`, `src/workspace/scan.rs`, and `src/workspace/commands.rs` | none | author exact owner operation and repair publication on restart |
| world state Graph and Traversal | own | projects allowlisted raw Event attachments and exposes flattened generic reads | partial | `graph/reducer.rs`, `graph/store.rs`, and `graph/query.rs` | none | validate owner payload, preserve occurrences, build cuts, and retire workspace raw-attachment authority |
| meld language | none | no selected behavior | not needed | Cargo workspace | formal goal language is outside this cut | none |

## Frozen Affected Domain Set

The affected domains are:

- workspace
- tree and workspace storage
- telemetry session append adapter
- Events
- world state Graph and Traversal
- root runtime assembly
- branches
- CLI

Docs Freshness and Dependency Security are proof specimens only. They are not behavior-change domains in this cut.

## Pass Two Affected Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| source enumeration | workspace | deterministic tree and ordered node records | derive one exact revision, scope, object set, relation occurrences, and completeness receipt | extend existing | Graph must not reconstruct workspace meaning | `src/workspace/scan.rs` |
| publication identity | workspace | candidate sequence has no durable owner operation identity | derive stable operation and Event record identity from root revision and enumeration rule, carried as `world_state.owner_publication.v1` under the workspace Event domain | new local behavior | timestamp or process order must not enter identity | `src/workspace/events.rs` |
| CLI publication | workspace and telemetry adapter | best-effort append can silently lose the owner operation, and an unchanged scan returns no retry candidate | reconstruct the current operation for unchanged scans, use durable idempotent append, and return failure | extend existing | telemetry must remain a thin append adapter | `src/workspace/commands.rs` and `src/workspace/scan.rs` |
| watch recovery | workspace | changed-tree facts are emitted best effort and initial restart build emits none | reconstruct and retry the current exact revision during initial watch build | extend existing | restart must not invent a prior revision | `src/workspace/watch/runtime.rs` |
| neutral carriage | Events | intact producer payload, stable record identity, append receipt, replay, and cursor already exist | carry the workspace operation unchanged | reuse unchanged | Event validation must not interpret workspace semantics | `crates/meld-events/src/events.rs` |
| owner admission | Graph | allowlisted workspace events become facts from raw attachments | require and validate the typed owner operation on `world_state.owner_publication.v1` while keeping `domain_id` equal to the semantic owner | extend existing | envelope hints cannot become semantic authority | `graph/reducer.rs` |
| operational anchors | Graph | snapshot selection drives the current snapshot anchor | retain anchor behavior as a separate operational responsibility | reuse unchanged | anchor currentness must not imply owner-cut completeness | `graph/source_intent.rs` |
| owner projection persistence | Graph | no occurrence or completeness projection exists | persist the intact projected operation in the existing world-model database | new local behavior | schema must remain inside the canonical Graph store | `graph/store.rs` |
| immutable cut | Graph and Traversal | generic live reads use implicit currentness and flatten equal relations | select exact revisions, receipts, scope, projection position, policy, and bounds | new local behavior | incomplete and truncated states must stay explicit | `graph/query.rs` |
| legacy workspace attachment read | Graph | raw workspace attachments are admitted as graph meaning | stop admitting new raw workspace knowledge and treat historical rows as nonauthoritative compatibility material | adapter only | a permanent second semantic reader would violate runtime singularity | `graph/reducer.rs` |
| graph runtime order | Graph runtime | projection, derived outbox, flush, cursor advance, and reporting are ordered | include owner projection before cursor advance and preserve crash repair | extend existing | false cursor progress after partial persistence | `graph/runtime.rs` |
| product assembly | root runtime | one Graph runtime already shares one Event authority and traversal store | reuse unchanged and prove through `ProductRuntimeAssembly` | reuse unchanged | test-only Graph construction cannot substitute for product proof | `src/runtime/assembly.rs` |
| owner cut selection | Graph and Traversal | no public exact selection contract exists | bind explicit owner requirements and scope to the `latest_complete` policy at one durable Graph position | new local behavior | a global latest revision or hidden fallback would make currentness implicit | `graph/query.rs` |
| branch owner inspection | branches | public product path exposes structural facts and flattened relations only | add `graph-owner-walk` as one production exact-cut route with occurrence-rich results | new local behavior | structural inspection must never satisfy owner completeness | `src/branches/query.rs` |
| structural branch inspection | branches | reads generic operational and historical graph facts | remain available but exclude typed workspace owner publications and disclaim cut completeness | reuse unchanged | inserting new owner material into both stores would recreate parallel authority | `src/branches/query.rs` |
| CLI presentation | CLI | presents structural branch graph products | add owner-cut identity, status, frontier, truncation, occurrence identity, and lineage presentation | adapter only | adapter must not infer completeness | branch tooling and presentation routes |

## Current Canonical Route

```text
workspace tree and node records
-> workspace Event candidates
-> best-effort ProgressRuntime append
-> Event authority
-> GraphRuntime replay
-> raw object and relation attachment admission
-> TraversalStore facts and relation indexes
-> generic TraversalQuery
-> structural branch neighbors and walk inspection
```

The route is canonical for current generic graph inspection. It is not sufficient for `SI-01` because publication loss is silent, relation occurrences collapse, completeness is absent, currentness is implicit, and the branch consumer cannot name one immutable cut.

## Intended Canonical Route

```text
exact workspace tree revision
-> deterministic owner publication operation
-> durable idempotent append or restart reconstruction
-> unchanged Event authority
-> existing GraphRuntime replay
-> typed `world_state.owner_publication.v1` workspace admission with hint parity validation
-> intact owner projection plus operational anchor projection
-> immutable bounded TraversalCut and TraversalResult
-> production `graph-owner-walk` branch inspection
```

## Superseded Surface

The completed cut must retire these authorities for new workspace knowledge:

- best-effort workspace owner publication in the CLI scan path
- unchanged CLI scans that cannot reconstruct the current owner operation
- watch startup that does not reconstruct the current publication
- Graph admission of raw workspace object and relation attachments as owner-qualified knowledge
- any insertion of new typed workspace owner material into the structural fact and relation route

Historical Event records and graph rows may remain readable only through structural inspection, anchor provenance, and forensic tests as explicitly nonauthoritative compatibility material. They may not originate new owner publications or satisfy a complete cut.

## Persistent State And Compatibility

The authoritative workspace source remains the workspace node store plus exact Merkle root revision. Events remains the durable publication ledger. Graph adds owner projection records inside the existing world-model database and keeps its identity-bearing cursor.

No new crate, service, actor, event authority, graph authority, or standalone store is required. One Graph-owned record family and index are required inside the existing store.

Historical workspace Events lack owner completeness and occurrence identity. They cannot be upgraded into truthful owner publications. They remain replayable for the existing snapshot anchor and structural historical inspection only. A fresh or unchanged workspace scan, or watch restart, produces the first authoritative typed revision. Exact queries report the owner missing until that happens.

## Scope Separation

Runtime path domains:

- workspace
- telemetry session append
- Events
- Graph runtime
- Traversal
- branches
- CLI

Behavior-change domains:

- workspace
- Graph and Traversal
- branches
- CLI adapter

Likely write units:

- workspace event and scan publication
- workspace CLI and watch recovery
- world-model graph contracts, reducer, store, query, runtime, and exports
- branch owner-query contracts and tooling presentation
- focused integration, replay, restart, property, and fuzz proof
- source program records and receipts

Events and root runtime assembly are on the product path but should remain source-unchanged unless implementation evidence disproves the existing seam.

## Explicit Non-Integration Decisions

- no Curation or Belief behavior enters this cut
- no Planner or Agent consumer is claimed
- no Docs Freshness or Dependency Security runtime is migrated
- no Execution publication runtime interprets workspace owner products
- no universal owner registry or ontology is introduced
- no Event payload semantics move into Events
- structural graph inspection remains distinct and cannot read typed owner publications or satisfy cut completeness
- no second Graph runtime or traversal store is introduced
- no old Workflow path is used as compatibility

## Confidence And Remaining Decision

The current and intended routes are grounded strongly enough to define a bounded vertical cut. The remaining decision is authority, not architecture: the reconstructed cut below must receive explicit user approval before source edits because the historical `WMR-SI-01` activation and gate are permanently withdrawn.
