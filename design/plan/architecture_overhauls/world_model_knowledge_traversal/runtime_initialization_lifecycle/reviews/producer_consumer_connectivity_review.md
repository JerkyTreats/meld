# World Model Reconciliation Producer And Consumer Connectivity Review

Date: 2026-08-20

Status: evidentiary discovery review, implementation not authorized

## Review Question

Can the proposed World Model Reconciliation path operate as one durable live program, or does it remain a collection of independently sound actors that happen to receive periodic ticks?

The path under review begins with product observation, proceeds through Events, Traversal, Curation, configured Belief, Agent, Strategy, Agent authorization, Curation or Execution, and returns through independently visible results to Agent reconciliation.

The review tests every handoff for a producer-owned durable product, a consumer-owned recovery position, deterministic idempotency, an exact visibility milestone, a domain-owned wait declaration, a viable wake mechanism, lag and generation fencing, and a restart source.

This review does not draft contracts, select implementation sequencing, or make lifecycle root interpret another domain's waiting semantics. It treats [Runtime Lifecycle And Quiescence](../../../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md) as the canonical lifecycle boundary, [Strategy Is Bigger Than A Task](../../strategy_plan_redesign.md) as proposed architecture, and current code as authority for implemented behavior.

The inspection budget was twelve batched source calls plus twenty additional source items. The review reached the batched-call budget after covering the named documents, the complete domain universe, initialization, runtime assembly, supervision, lifecycle storage, observation publication, Events, Traversal, Belief, Agent, Strategy handoff, Execution lowering, dispatch, publication, and focused tests. Confidence is reduced only where future Curation and heterogeneous Plan contracts do not yet exist.

## Finding

The current system has many strong local recovery loops. Events has durable append and replay. Traversal has a durable cursor and derived-event outbox. Belief evidence ingestion persists domain state before advancing its cursor. Agent persists decisions before crossing the Goal Set seam. Execution has durable Goal, Task Network, outcome, and publication state.

Those loops do not yet compose into one proven World Model Reconciliation lifecycle.

The smallest description of the gap is this:

```text
current runtime
= bounded polling over independently durable actors

required reconciliation runtime
= independently durable actors
+ explicit producer visibility milestones
+ consumer-owned wait and wake closure
+ generation-fenced participant readiness
+ restart from every durable handoff
```

Four current breaks are decisive.

Workspace observation is not part of the stewardship-derived supervised participant set. The long-lived watch path remains a separate daemon and still contains legacy Workflow execution. The ordinary scan path publishes canonical workspace envelopes through best-effort telemetry append and does not give reconciliation a durable observation operation to recover.

Traversal changes do not generally make an already assessed configured Belief eligible. Initial anchored assessment reads Traversal once. Later reassessment is driven by Belief dirty keys, and current dirty keys are created by Evidence assignment rather than by graph-anchor or relation revision. A newer workspace snapshot can therefore become visible in Traversal without producing a newer Belief revision for Agent delivery.

The implemented structural lifecycle substrate is disconnected from the running actors. Participant plans name readiness, wake, safe-point, and stop references, but production registration derivation does not consume that plan. Actor wait reports carry condition, subject, and detail only. They are never turned into owner wait receipts with structural wake references. The lifecycle liveness projector exists, but no production path feeds it. Foreground runtime separately labels a pass quiescent whenever every actor reports no work, including when no actors exist. That directly conflicts with the canonical definition of quiescence.

Future Curation and heterogeneous Strategy Plans have no implemented runtime boundary. There is no durable EpiOp request state, Curation cursor, Curation result state, Plan progression state, or Agent authorization receipt for an epistemic product. Events can carry future facts, but append alone cannot prove Traversal, Belief, or Agent visibility.

The evidence therefore supports the user concern. The runtime primitives are mostly well founded. Their current composition does not yet prove a continuously live reconciliation program.

## Current Initialization Entrance

Current world initialization deliberately separates semantic genesis from machine assembly. The root pipeline installs theory, creates the seed Agent and subscription, marks the Agent operational, and appends an idempotent unobserved-scope Event. Runtime assembly later probes durable state and leaves unresolved actors body-less rather than manufacturing state.

That is a sound local separation, but it is not yet complete participant readiness.

| Initialization handoff | Producer-owned durable product | Consumer read or cursor | Idempotency identity | Visibility milestone | Current state |
| --- | --- | --- | --- | --- | --- |
| PDS package routing to owner registries | Exact owner revisions and package installation receipt | Assembly resolves one exact receipt image and owner revisions | Component content hashes and package receipt id | Every selected owner revision resolves and validates | Implemented for the current theory image |
| Owner theory to Agent genesis | Agent record, maintained-condition binding, Curation rule binding, exact Belief subscription | Agent actor loads its record and subscriptions by Agent id | Agent id and subscription natural key | Agent record is `Operational` and subscription is durable | Implemented, but operational status is data readiness rather than participant readiness |
| Genesis to Events | Unobserved-scope Event | Traversal and Evidence ingestion ledger cursors | Deterministic scope record id under idempotent append | Event append receipt only | Implemented |
| Durable world to runtime assembly | Body-bearing or truthfully unresolved actor factory | Root assembly probes registries, Agent record, graph store, Execution stores, routes, and provider binding | Runtime id and selected stewardship identity | Factory has semantic body | Implemented as a probe, not an owner readiness receipt |
| Prepared activation to current generation | Activation generation and assignment head | Proposed activation lifecycle and supervisor | Prepared id, generation id, expected prior head | Complete required readiness receipts before current publication | Implemented as a standalone store with tests; no production caller was found |
| Current generation to actor start | Supervisor lease and started handle | Supervisor owns handle map and lease | Runtime id plus lease id | Handle started after lease | Implemented operationally, but not fenced by assignment activation generation |

The seed Agent becomes operational before the epistemic genesis Event is appended. The staged command runs in fixed order, so a complete invocation preserves the intended sequence. A caller may select only a subset of stages, and no aggregate world-ready receipt proves that all semantic stages completed before runtime start. Assembly truthfully exposes unresolved bindings, but current startup does not consume the proposed prepared activation closure as its registration authority.

Evidence is in [world initialization](../../../../../../src/init/world.rs), [world initialization pipeline](../../../../../../src/init/world/pipeline.rs), [runtime assembly](../../../../../../src/runtime/assembly.rs), [activation participant plan](../../../../../../src/theory/activation.rs), and [startup activation publication](../../../../../../src/runtime/activation.rs).

## Complete Reconciliation Handoff Trace

The table records the desired handoff from the proposed loop and the strongest current implementation ground. An absent current implementation is not treated as an invalid proposed boundary.

| Handoff | Producer-owned durable product | Consumer cursor or query | Idempotency identity | Required visibility milestone | Wait declaration | Wake mechanism | Lag or fence behavior | Restart source | Present implementation state |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Product observation to Events | Exact observation under source snapshot, source identity, and completeness boundary | Event ledger append authority | Observation identity must include source snapshot and observed entity | Canonical Event sequence assigned and durable | Producer must name pending observation or source availability | Passive source delivery, durable deadline, or operator action | Activation generation and passive subscription lineage must reject or classify late delivery | Durable observation operation and source cursor | Workspace scan builds rich envelopes, but normally submits them through best-effort plain append. Watch is outside the supervised set. Dependency Security has no canonical observation participant in the inspected runtime |
| Events to Traversal | Sequenced Event record carrying promoted objects and relations | Traversal ledger cursor `world_state.graph.reducer` | Event record id plus ledger identity and source sequence | Traversal cursor has passed the exact source sequence and projection writes are durable | No graph-specific quiet wait is emitted today | Foreground loop wakes globally on Event watermark and otherwise polls | Retention gaps are fatal and require rebuild. Producer allowlist fences admitted domains | Traversal cursor, projection indexes, derived outbox | Strongly implemented for admitted domains `workspace_fs`, `context`, and `execution`. Future `curation` Events are filtered out |
| Traversal to standing Curation | Frozen graph cut and exact rule-relevant object set | Future Curation query and source-cut cursor | Future operation identity over Agent, perspective, rule revision, roots, bounds, and source cut | Curation has durably accepted a bounded operation or standing work item against that cut | No current declaration | No current wake path from graph revision to Curation | Perspective, rule revision, and activation generation must fence eligibility | No current restart source | Not started. No Curation domain or operation store exists |
| Agent Plan to planned Curation | Agent-authorized EpiOp with Plan and product lineage | Future Curation request query or accepted-request Event cursor | Future authorization and operation identity | Curation owns durable accepted operation state | No current declaration | Proposed accepted-operation Event or a typed durable Curation queue | Replay cannot grant fresh authority. Current generation and Plan revision must be checked | No current restart source | Not started |
| Curation to Events | Terminal EpiOp result and any promoted Curation objects or relations | Event append authority | Deterministic result and semantic product ids under idempotent append | Terminal result Event durable at exact sequence | Future Curation wait on eligible operation or input change | Curation work selection plus Event watermark | Frozen cut and Curation currentness rules prevent feedback repetition | Future Curation operation state plus publication outbox | Not started. Event carrier primitives exist |
| Traversal to configured Belief | Current graph anchor, provenance, or future typed Curation result | Belief work selector and Traversal query | Belief key plus evidence identity or graph source identity | New Belief revision committed against exact source cursor | `graph_anchor_absent` or `belief_work_ineligible` | Current global tick only | Current assessment has no general graph revision fence after initial assessment | Belief store, dirty-key index, assessment checkpoint | Partial. Initial anchored assessment works. Later graph-only changes do not generally dirty the Belief key |
| Events to configured Belief | Producer Event matched by installed outcome mapping | Named durable Evidence ingestion cursor | Mapping identity plus Event publication identity derive Evidence identity | Evidence and any resulting Belief revision durable before cursor advancement | `ledger_quiet_past_cursor` or `no_mappable_events` | Foreground Event watermark wake plus periodic tick | Mapping revision is resolved by the actor. Cursor remains before failed absorption | Belief store and Event consumer cursor | Strong local implementation. Only explicitly mapped event types become evidence |
| Belief revision to Agent | Exact Belief revision head under Agent subscription key | Agent subscription cursor `last_delivered_seq` and satisfaction checkpoint | Subscription id plus revision id and sequence | Agent decision durable, required sink receipt durable, then delivery cursor advanced | `no_undelivered_revisions` or `no_pending_satisfaction_reviews` | Periodic bounded actor tick after any global Event wake | Exact key, perspective, branch, and revision sequence fence delivery | Agent store subscription cursor, decision index, satisfaction checkpoint, sink receipts | Strong local implementation. There is no Belief-owned revision notification, so non-Event Belief changes wait for periodic polling |
| Agent to Strategy | Root Goal candidate plus immutable planner projection and exact theory snapshot | Synchronous Strategy function call inside Agent delivery | Planner snapshot id, Goal id, theory revision, candidate identity | Strategy result is included in the persisted Agent decision | No independent wait because the call is synchronous | Agent delivery eligibility | Strategy sees one frozen projection. Future Plan revision fencing is absent | Agent decision only for current executable authorization | Implemented for one executable Composition. Heterogeneous Plan and reconstruction lineage are not started |
| Strategy Plan to Agent progression | Future heterogeneous Plan revision with causal obligations and proposed discharge products | Future Agent active Plan query | Plan id, Plan revision, parent revision, frozen source cut | Agent authorizes the Plan or an enabled product under exact current evidence | No current declaration | New Belief revision or product milestone should make Agent progression eligible | Stale Plan products require an explicit freshness fence | No current Plan store | Not started |
| Agent authorization to Execution Goal Set | Goal command plus complete executable Strategy authorization | Execution active Goal query | Agent command id, Agent dedupe key, Goal id | Execution Goal record applied or duplicate and Agent sink receipt durable | Agent retries a persisted decision until a receipt exists | Periodic Agent tick | Goal lifecycle epoch and authority lineage fence mutation | Agent decision and sink receipt plus persistent Execution Goal store | Strong current handoff for executable work |
| Execution Goal Set to Task Network | Authorized Composition lowering result and Task Network mutation command | Execution planning scans active Goals and Task Network state | Composition identity, lowering idempotency key, command id, network revision, state hash | Required Task nodes exist durably in the network | `no_active_goals`, `no_applicable_method`, or world-state diagnostics | Periodic bounded planning tick | Task Network revision and state hash fence mutations | Persistent Goal store and sled Task Network journal | Implemented, but current Execution planning also reprojects world state and semantically revalidates Strategy authorization. The unauthorised Method search path remains live as legacy behavior |
| Planning to package dispatch | Task package route plan | In-process handoff map read by dispatch | Deterministic plan id and derived package run id | Durable package progress or durable Task Network mutation exists | Dispatch wait declarations cover ready work, not absence of an in-memory plan handoff | Periodic bounded tick | Restart reconstructs the handoff only if planning selects the route again | Persistent Goal, planning input, and package progress are expected to reproduce the plan | Partial. The handoff itself is process memory and has no durable cursor. Publication also reads this memory to discover aggregate run bindings |
| Task Network to dispatch | Ready Task nodes and claims | Dispatch ready-set query and durable claim state | Task instance id, claim id, worker id, lifecycle epoch | Outcome and artifacts recorded durably through Task Network command | `no_ready_tasks` and dependency-specific conditions | Periodic bounded tick | Claim and lifecycle epoch fence dispatch. Same worker resumes interrupted claims | Sled Task Network state, claims, artifacts, package progress | Strong local implementation |
| Execution outcome to Events | Pending Task publication and package aggregate publication records | Publication actor scans durable outboxes and reduced run state | Deterministic publication id and Event record id under idempotent append | Event durable, then publication marked with exact append receipt | `no_pending_publications` or aggregate run not terminal | Periodic bounded tick and global Event wake | Ledger identity is checked. Append can be replayed before mark without duplication | Task Network publication outbox, aggregate publication store, package progress | Strong for per-Task publication. Aggregate discovery depends on the in-memory Plan handoff map |
| Result Event to independent visibility milestones | One canonical Event sequence | Traversal cursor, Evidence ingestion cursor, Belief revision head, Agent subscription cursor | Each consumer has its own durable identity and position | Milestones remain distinct: append, graph materialization, Belief revision, Agent acceptance | Each domain emits its own current wait vocabulary | Global Event watermark wakes the foreground loop. Domain revision wakes are not implemented | Each consumer may lag independently and must never borrow another consumer's cursor | Event ledger plus every consumer-owned store | Implemented as independent local positions. No shared milestone query or causal Plan dependency exists |
| Agent reconciliation after results | New admitted Belief revision and current Plan lineage | Agent subscription and future Plan progression query | Revision id plus future Plan revision and product identity | New Agent decision or explicit absorbed result durable | Current Agent waits on newer exact-key Belief revision | Periodic bounded tick after Event wake | Exact belief sequence works. Plan freshness and product milestone fences are absent | Agent subscription cursor and decisions | Implemented only for Goal creation and satisfaction over Belief revisions. Continuous heterogeneous Plan reconciliation is not started |

## Visibility Is Not Append

The proposed EpiOp loop depends on four independent milestones:

```text
result durable in Events
-> result materialized in Traversal
-> result admitted into configured Belief
-> resulting revision accepted by Agent
```

Current code correctly stores separate cursors for several of these stages. It does not yet give Strategy Plan progression a way to name which milestone discharges a causal dependency.

An Event append receipt contains ledger identity, sequence, and disposition. Traversal reports its own ledger cursor only after durable projection and derived publication work. Evidence ingestion owns another cursor and advances it only after Belief writes are durable. Agent owns subscription and satisfaction checkpoints. These are truthful independent facts.

The current aggregate publication path shows the danger in the other direction. It treats a durable Event append as complete only after writing the publication mark, which is sound. Yet downstream consumers still may trail. No current API allows Agent progression to wait specifically for graph visibility through sequence N or Belief revision derived from publication P.

## Wake And Waiting Assessment

Current wait declarations are useful diagnostic evidence. World-model and Execution actors name missing anchors, missing installed theory, quiet ledger positions, unmapped Event windows, absent revisions, absent Goals, unready Tasks, and pending publication conditions.

They are not yet lifecycle waits.

The root `WaitingOnDeclaration` contains only condition, optional subject, and detail. It has no owner checkpoint reference and no structural wake reference. The separate lifecycle `OwnerWaitReceiptV1` has generation, incarnation, owner checkpoint, and structural wake references, but no production adapter connects actor reports to those receipts. The `OwnerWakePort` trait has no implementation in the inspected code. The activation lifecycle projector is therefore a tested local primitive rather than the liveness projection of the running program.

The foreground loop has one broad wake source. After each complete supervisor pass, it waits on the Event writer watermark in short cancellation-responsive intervals and uses the configured tick interval as fallback. Events committed during the pass intentionally wait for the fallback interval because the baseline is captured after the pass. Direct-store derived Events also do not move the writer watermark and rely on fallback polling.

This mechanism prevents pure sleep polling for new ledger commits. It does not wake on a new Belief revision, Agent record revision, Execution Goal insertion from another process, Task Network mutation, provider completion outside Events, PDS activation intent, or restored participant binding. Those changes become visible only at the next periodic tick unless their producer also appends an Event.

The current system therefore has notification-assisted polling, not complete domain wake closure.

## Startup Order And Scheduling

Supervisor actor order is stable only because handles are stored by runtime id in a sorted map. Every active actor runs exactly once per maintenance pass. That order is lexical, not a declared causal dependency graph.

The current order can put Execution planning, publication, and dispatch before world-model graph replay, Evidence ingestion, Belief assessment, and Agent actors in one pass. This does not corrupt the durable stores. It can add several passes of lag, and it proves that runtime correctness must rest on durable selection rather than same-pass order.

Most actors satisfy that rule. The package-route handoff is the significant exception. Planning records selected package plans in an in-process map. Dispatch and aggregate publication read that map. A process restart reconstructs it only if a later planning tick selects the same route. Durable package progress deduplicates repeated execution, but the handoff's availability and aggregate discovery remain dependent on replanning and startup order.

Runtime assembly also exposes a late-bound dispatch route slot. A missing route leaves dispatch unresolved, which is truthful. The slot is process memory and first-binding-wins. It is not tied to an activation generation, so a changed physical binding cannot be proven to replace the route through a generation fence in the current composed runtime.

## Duplicate Semantic Paths

Three duplicated or legacy paths are active enough to threaten one coherent reconciliation lifecycle.

The root workspace watch daemon directly executes registered legacy Workflows when an old Agent binding names one. That is a second cognition and action path beside Agent, Strategy, and Execution.

Execution planning still supports active Goals with no Strategy authorization by searching its own Method library. For authorized Goals, it reprojects world state and revalidates Goal, frame, operator preconditions, Capability resolution, and authority before lowering. Some independent shape and freshness validation is necessary at the consumer seam, but the present actor still owns both legacy planning and authorized realization paths. The proposed reconciliation split requires those paths to be distinguished rather than treated as one semantic planner.

Current `src/agent` and `src/workflow` remain separate from `meld-world-model` Agent and Strategy. They are not needed for the proposed loop, yet watch and production dispatch adapters still reach them. This is current integration evidence, not a recommendation to modify them in this review.

## Lifecycle Contradictions

The canonical lifecycle document defines quiescence as no eligible work plus complete required participation and viable durable wake paths. Current foreground tooling calls a pass quiescent whenever all ticked actors returned no work. An empty actor set is also called quiescent. The same code preserves wait declarations only as operator narration.

Current graceful shutdown asks every handle to stop, treats `not started` as a safe point, flushes product stores, releases leases, and marks the instance stopped. It does not close assignment-generation admission, drain only accepted obligations, collect owner safe-point receipts, fence passive subscriptions, summarize unresolved Execution effects, or consume an aggregate fenced-quiescence receipt.

Current restart policy is operational. It recovers expired supervisor leases and restarts actors from their domain stores. It does not close admission while participants reconcile a prior incarnation, nor does it create a new participant incarnation under one activation generation. The standalone lifecycle store can represent these concepts but is not wired to the supervisor.

These are direct implementation contradictions with the canonical lifecycle, not evidence that the canonical lifecycle is wrong.

## Regenerated Domain Snapshot

The workspace ownership boundaries remain:

```text
meld
meld-events
meld-execution
meld-lang
meld-world-model
```

The current root domain modules are:

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

The current `meld-world-model` domains are:

```text
agent
belief
planner
strategy
waiting
world_state
```

The snapshot was regenerated from the workspace manifest and top-level Rust source files on the evidence date.

## Pass One Domain Sweep

This pass uses the root modules as the complete product domain universe and records package-owned crate domains where ownership crosses a crate boundary.

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Legacy Agent registry and Workflow binding remain reachable from watch | `not needed` | [watch runtime](../../../../../../src/workspace/watch/runtime.rs) | Canonical Agent runtime is owned by `meld-world-model` | Preserve as explicit legacy evidence |
| `api` | `adapter` | Supplies root context and production dispatch route dependencies | `partial` | [root API](../../../../../../src/api.rs), [runtime ports](../../../../../../src/runtime/ports.rs) | not applicable | Keep non-authoritative |
| `branches` | `none` | No direct lifecycle handoff found | `not needed` | [branch surface](../../../../../../src/branches.rs) | Branch meaning is already carried in world-model owner contracts | none |
| `capability` | `publish` | Publishes exact executable contracts and invokers for Strategy and Execution activation | `partial` | [runtime assembly](../../../../../../src/runtime/assembly.rs) | not applicable | Generation-bound catalog visibility remains unproven |
| `cli` | `adapter` | Runs initialization and foreground supervisor loop | `partial` | [runtime tooling](../../../../../../src/runtime/tooling.rs) | not applicable | Current quiescence projection contradicts canonical lifecycle |
| `compat` | `none` | Compatibility surface is not a reconciliation owner | `not needed` | [compat surface](../../../../../../src/compat.rs) | Compatibility may wrap paths but cannot own liveness | none |
| `concurrency` | `none` | No domain-specific reconciliation handoff found | `not needed` | [concurrency surface](../../../../../../src/concurrency.rs) | Supervisor and domain stores own current coordination | none |
| `config` | `publish` | Resolves stewardship package and physical binding used by initialization and assembly | `partial` | [config surface](../../../../../../src/config.rs) | not applicable | Exact activation generation does not fence composed bindings |
| `context` | `publish` | Publishes context-head Events admitted by Traversal and supplies execution context | `partial` | [context events](../../../../../../src/context/events.rs), [context head](../../../../../../src/context/head.rs) | not applicable | Current queue and legacy generation paths are not one supervised participant family |
| `control` | `none` | Legacy orchestration is outside proposed reconciliation | `not needed` | [control surface](../../../../../../src/control.rs) | Strategy Plan replaces manual Workflow choreography | none |
| `dependency_security` | `publish` | Owns future security observations and domain products, but no participant is composed | `not started` | [dependency security surface](../../../../../../src/dependency_security.rs) | not applicable | Observation and result publication lifecycle remains absent |
| `docs` | `publish` | Owns docs observation and Task products through current Capabilities | `partial` | [docs surface](../../../../../../src/docs.rs) | not applicable | Independently addressable observation and Curation products remain proposed |
| `error` | `none` | Shared error mapping only | `not needed` | [error surface](../../../../../../src/error.rs) | Error vocabulary does not own lifecycle | none |
| `events` | `adapter` | Root re-exports and binds canonical Event authority | `complete` | [events adapter](../../../../../../src/events.rs), [runtime ports](../../../../../../src/runtime/ports.rs) | not applicable | Preserve thin adapter role |
| `execution` | `adapter` | Root connects Agent commands, Goal Set, planning, dispatch, and publication | `partial` | [execution adapter](../../../../../../src/execution.rs), [runtime assembly](../../../../../../src/runtime/assembly.rs) | not applicable | In-memory package handoff and legacy Method route are active gaps |
| `harness` | `observe` | Preserves actor reports and derives diagnostic eligibility walks | `complete` | [eligibility walk](../../../../../../src/harness/eligibility.rs) | not applicable | Remain observational |
| `heads` | `none` | Legacy head compatibility is not a reconciliation owner | `not needed` | [heads surface](../../../../../../src/heads.rs) | Traversal anchors and owner state carry the canonical path | none |
| `ignore` | `none` | Influences observation scope but owns no runtime handoff | `not needed` | [ignore surface](../../../../../../src/ignore.rs) | Workspace observation owns publication completeness | none |
| `init` | `own` | Installs theory, creates Agent genesis, subscribes Belief, and seeds Events | `partial` | [world initialization](../../../../../../src/init/world.rs) | not applicable | No aggregate world-ready or activation-ready handoff |
| `lib` | `adapter` | Exposes crate surfaces only | `complete` | [crate surface](../../../../../../src/lib.rs) | not applicable | none |
| `logging` | `none` | Logs do not own progress or liveness | `not needed` | [logging surface](../../../../../../src/logging.rs) | Operational visibility is not a durable handoff | none |
| `merkle_traversal` | `none` | Filesystem traversal strategy is internal to observation | `not needed` | [Merkle traversal](../../../../../../src/merkle_traversal.rs) | Workspace owns the observation product | none |
| `metadata` | `none` | No direct reconciliation handoff found | `not needed` | [metadata surface](../../../../../../src/metadata.rs) | Metadata persistence is not in the proposed causal loop | none |
| `prompt_context` | `none` | Prompt lineage is an Execution implementation concern | `not needed` | [prompt context](../../../../../../src/prompt_context.rs) | Execution owns Task realization | none |
| `provider` | `consume` | Provider route realizes executable Task work | `partial` | [provider surface](../../../../../../src/provider.rs), [dispatch route](../../../../../../src/runtime/ports.rs) | not applicable | Durable external operation and callback lineage are not integrated with activation lifecycle |
| `runtime` | `own` | Composes actors, leases, ticks, reports, wakes, restart, and shutdown | `partial` | [runtime assembly](../../../../../../src/runtime/assembly.rs), [supervisor](../../../../../../src/runtime/supervisor/entrypoint.rs), [lifecycle store](../../../../../../src/runtime/lifecycle.rs) | not applicable | Structural lifecycle and actual actor waits are disconnected |
| `serve` | `adapter` | Exposes runtime status and harness reads | `partial` | [serve surface](../../../../../../src/serve.rs) | not applicable | Remain observational |
| `session` | `none` | Command session identity partitions Events but is not program lifecycle | `not needed` | [session surface](../../../../../../src/session.rs) | Program and activation lifetime outlive command sessions | none |
| `store` | `none` | Persistence primitive only | `not needed` | [store surface](../../../../../../src/store.rs) | Domain owners define recovery meaning | none |
| `task` | `consume` | Root package execution implements current dispatch route | `partial` | [task surface](../../../../../../src/task.rs), [dispatch actor](../../../../../../crates/meld-execution/src/task_network/dispatch_actor.rs) | not applicable | Current route should remain subordinate to Execution-owned durable work |
| `telemetry` | `adapter` | Carries many observation envelopes through Event append, often best effort | `partial` | [progress runtime](../../../../../../src/telemetry/sessions/service.rs) | not applicable | Product observations need a durable producer contract rather than telemetry assumptions |
| `theory` | `publish` | Owns structural package routing, prepared activation closure, and participant plan shapes | `partial` | [activation theory](../../../../../../src/theory/activation.rs) | not applicable | Participant plan is not consumed by production composition or supervisor |
| `tree` | `none` | Supports workspace observation internals | `not needed` | [tree surface](../../../../../../src/tree.rs) | Workspace owns the durable observation product | none |
| `types` | `none` | Shared root values only | `not needed` | [types surface](../../../../../../src/types.rs) | No lifecycle ownership | none |
| `views` | `none` | Presentation views do not own runtime visibility | `not needed` | [views surface](../../../../../../src/views.rs) | Consumer visibility must come from owner checkpoints | none |
| `workflow` | `none` | Legacy Workflow remains in watch and package dispatch routes | `not needed` | [workflow surface](../../../../../../src/workflow.rs), [watch runtime](../../../../../../src/workspace/watch/runtime.rs) | Strategy is the canonical workflow and Workflow must not become a reconciliation participant | Preserve as duplicate-path evidence |
| `workspace` | `own` | Builds observation Events and has a separate long-lived watch daemon | `partial` | [workspace scan](../../../../../../src/workspace/scan.rs), [workspace watch](../../../../../../src/workspace/watch/runtime.rs) | not applicable | Observation is not a supervised durable participant path |
| `world_state` | `adapter` | Root re-exports world-model traversal surface | `complete` | [world-state adapter](../../../../../../src/world_state.rs) | not applicable | Keep authority in world-model |
| `meld-events` | `own` | Owns append, replay, watermark, consumer cursor registry, and idempotent identity | `complete` | [Event authority](../../../../../../crates/meld-events/src/events/authority.rs), [subscription](../../../../../../crates/meld-events/src/events/subscription.rs) | not applicable | Current carrier can support the proposed loop |
| `meld-execution` | `consume` and `publish` | Consumes authorized executable Goals, lowers Tasks, dispatches, and publishes outcomes | `partial` | [planning runtime](../../../../../../crates/meld-execution/src/planning/runtime.rs), [publication bridge](../../../../../../crates/meld-execution/src/task_network/publication.rs) | not applicable | Package handoff and semantic path duplication remain |
| `meld-lang` | `none` | Shared permissive values only | `not needed` | [language surface](../../../../../../crates/meld-lang/src/lib.rs) | Lifecycle and wake behavior do not belong in the language | none |
| `meld-world-model` | `own` | Owns Traversal, Belief, Agent, Planner, Strategy, and wait vocabularies | `partial` | [world-model surface](../../../../../../crates/meld-world-model/src/lib.rs) | not applicable | Curation, Plan progression, graph-driven Belief invalidation, and lifecycle wait receipts are absent |

## Frozen Affected-Domain Set

The affected set is frozen as:

```text
api
capability
cli
config
context
dependency_security
docs
events
execution
harness
init
lib
provider
runtime
serve
task
telemetry
theory
workspace
world_state
meld-events
meld-execution
meld-world-model
```

The set includes runtime participants, behavior owners, adapters, and observers. Inclusion does not imply implementation change. The separate scope synthesis below prevents that promotion.

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| API context composition | `api` | Opens root services consumed by execution routes | Supply exact process-local adapters only | `adapter only` | API facade could become runtime authority | [root API](../../../../../../src/api.rs) |
| Capability publication | `capability` | Exact contracts and invokers activate into one catalog | Publish activation-generation-bound availability to Strategy and Execution | `extend existing` | Catalog change can invalidate a Plan without a wake or fence | [runtime assembly](../../../../../../src/runtime/assembly.rs) |
| Foreground run loop | `cli` and `runtime` tooling | Ticks all actors, wakes on Event watermark, polls on interval | Operate the owner-declared lifecycle without inventing quiescence | `extend existing` | Current all-no-work rule falsely claims quiescence | [runtime tooling](../../../../../../src/runtime/tooling.rs) |
| Stewardship selection | `config` | Supplies package, subject, provider, Agent, and theory ids | Bind exact activation inputs and generation lineage | `extend existing` | First-binding-wins process memory can outlive semantic selection | [config surface](../../../../../../src/config.rs) |
| Context observation | `context` | Publishes context head Events and supplies execution context | Remain an independent observation producer with durable publication | `extend existing` | Legacy generation queue may bypass reconciliation | [context head](../../../../../../src/context/head.rs) |
| Security observation | `dependency_security` | Domain Capability surface exists outside composed loop | Publish exact observations and later consume only owned Task work | `new local behavior` | Reconciliation could guess package-manager semantics | [dependency security](../../../../../../src/dependency_security.rs) |
| Docs observation | `docs` | Current Capabilities produce aggregate Task artifacts | Publish independently addressable observed claims and verification products | `extend existing` | Aggregate-only products cannot wake fine-grained Curation | [docs surface](../../../../../../src/docs.rs) |
| Event root ports | `events` | Thin wrappers around canonical authority | Carry producer-owned append and consumer-owned replay only | `reuse unchanged` | Root could confuse append with downstream visibility | [runtime ports](../../../../../../src/runtime/ports.rs) |
| Goal Set adapter | `execution` root | Maps Agent command and authorization into Execution-owned request | Preserve explicit seam and durable receipt | `extend existing` | Root mapping can silently drop future Plan lineage | [runtime ports](../../../../../../src/runtime/ports.rs) |
| Dispatch route adapter | `execution` root | Binds legacy Workflow package route after assembly | Adapt executable Task realization only | `extend existing` | Old Workflow becomes a second planning runtime | [production dispatch route](../../../../../../src/runtime/ports.rs) |
| Eligibility observation | `harness` | Reads preserved waiting declarations | Continue read-only diagnosis across owner reports | `reuse unchanged` | Observational walk could be mistaken for a wake authority | [eligibility walk](../../../../../../src/harness/eligibility.rs) |
| Theory installation | `init` | Installs exact owner revisions idempotently | Produce a complete semantic readiness account | `extend existing` | Partial stage selection can leave a plausible but incomplete world | [world init pipeline](../../../../../../src/init/world/pipeline.rs) |
| Agent genesis | `init` and world-model Agent | Creates Agent, subscription, rule, maintained condition, and operational status | Separate durable semantic readiness from live participant readiness | `extend existing` | Operational Agent status currently precedes genesis Event visibility | [Agent registration](../../../../../../crates/meld-world-model/src/agent/registration.rs) |
| Genesis Event | `init` and `meld-events` | Idempotently appends unobserved scope | Remain a canonical Event input with explicit downstream barriers | `reuse unchanged` | Append receipt may be mistaken for Belief or Agent readiness | [world init pipeline](../../../../../../src/init/world/pipeline.rs) |
| Public crate surfaces | `lib` and `world_state` | Re-export owner contracts | Expose only required public seams | `adapter only` | Re-export can bypass owner APIs | [root crate surface](../../../../../../src/lib.rs) |
| Provider execution | `provider` | Process-bound provider route called by dispatch | Report durable external operation and result lineage through Execution | `extend existing` | Callback or retry may escape activation and attempt fences | [provider surface](../../../../../../src/provider.rs) |
| Actor registration | `runtime` | Derives one fixed first-proof registry from stewardship binding | Reflect exact prepared participant closure | `extend existing` | Static catalog does not include observation or future Curation participants | [runtime assembly](../../../../../../src/runtime/assembly.rs) |
| Actor stepping | `runtime` | Lexically ordered one-step-per-pass loop | Remain order-independent through durable handoffs | `extend existing` | In-memory package handoff is order and restart sensitive | [supervisor tick](../../../../../../src/runtime/supervisor/entrypoint.rs) |
| Operational supervision | `runtime` | Leases, heartbeats, reports, restart, and simple shutdown | Preserve operational ownership distinct from program liveness | `extend existing` | Healthy actors can still form a braindead program | [supervisor](../../../../../../src/runtime/supervisor/entrypoint.rs) |
| Activation lifecycle | `runtime` | Standalone generation, operation, wait, and quiescence stores exist | Connect exact owner receipts without interpreting them | `extend existing` | A second disconnected lifecycle system already exists in code | [lifecycle store](../../../../../../src/runtime/lifecycle.rs) |
| Runtime status serving | `serve` | Presents preserved reports and status | Remain read-only | `adapter only` | Served active-idle could be presented as true quiescence | [serve surface](../../../../../../src/serve.rs) |
| Root package execution | `task` | Durable package route implements current docs execution | Remain behind Execution-owned dispatch and outcome recording | `extend existing` | Package progress and Task Network terminal state can diverge | [dispatch actor](../../../../../../crates/meld-execution/src/task_network/dispatch_actor.rs) |
| Observation event facade | `telemetry` | Supports durable, idempotent, plain, and best-effort append | Product observations require explicit durability rather than telemetry convention | `extend existing` | Accepted best-effort enqueue can be dropped or lack stable identity | [progress runtime](../../../../../../src/telemetry/sessions/service.rs) |
| Package participant plan | `theory` | Declares participant ids and lifecycle contract references | Supply structural expected participation to activation | `extend existing` | Unresolved string references are not callable owner contracts | [activation theory](../../../../../../src/theory/activation.rs) |
| Workspace scan | `workspace` | Builds source, snapshot, selection, node, and completion Events | Own exact durable observation operation and completeness | `extend existing` | Up-to-date shortcut emits no observation Events | [workspace scan](../../../../../../src/workspace/scan.rs) |
| Workspace watch | `workspace` | Separate daemon observes changes and runs legacy Workflows | Publish observations through one supervised passive-source lifecycle | `extend existing` | Process-local watcher state and legacy Workflow execution bypass the flywheel | [workspace watch](../../../../../../src/workspace/watch/runtime.rs) |
| Event append and replay | `meld-events` | Durable append, idempotency, replay, watermark, and consumer registry | Remain neutral durable carrier and wake transport | `reuse unchanged` | Generic Event cannot supply Curation semantics | [Event authority](../../../../../../crates/meld-events/src/events/authority.rs) |
| Traversal reduction | world-model `world_state` | Durable cursor, projection, source allowlist, derived outbox | Admit exact promoted Curation products and report visibility through source sequence | `extend existing` | Generic admission can create feedback loops | [Traversal runtime](../../../../../../crates/meld-world-model/src/world_state/graph/runtime.rs) |
| Traversal query | world-model `world_state` | Current anchors and bounded graph queries | Supply frozen source cuts without owning Curation meaning | `extend existing` | Query-time catch-up can hide consumer lag semantics | [Traversal query](../../../../../../crates/meld-world-model/src/world_state/graph/query.rs) |
| Configured Belief assessment | world-model `belief` | Initial anchor assessment and dirty-key reassessment | Become eligible for applicable graph and Curation revisions | `extend existing` | Current graph revision does not generally dirty a key | [Belief selection](../../../../../../crates/meld-world-model/src/belief/selection.rs) |
| Event Evidence ingestion | world-model `belief` | Exact mapping and durable Event consumer cursor | Continue state-before-cursor absorption | `reuse unchanged` | Unmapped Events advance the cursor and are unavailable if theory is installed later | [Evidence ingestion](../../../../../../crates/meld-world-model/src/belief/evidence_ingestion.rs) |
| Agent delivery | world-model `agent` | Exact-key subscription cursor and durable decisions | Consume new revisions and future Plan milestones | `extend existing` | Revision polling has no owner-revision wake | [Agent selection](../../../../../../crates/meld-world-model/src/agent/selection.rs) |
| Agent authorization | world-model `agent` | Persists executable Strategy authorization before Goal Set submission | Authorize heterogeneous Plan and each eligible discharge product under freshness | `new local behavior` | Replayed historical authorization could escape Plan revision fence | [Agent runtime](../../../../../../crates/meld-world-model/src/agent/runtime.rs) |
| Strategy construction | world-model `strategy` | Pure bounded executable candidate construction | Produce one heterogeneous Plan revision over frozen knowledge | `new local behavior` | Strategy runtime must not become its own listener or scheduler | [Strategy redesign](../../strategy_plan_redesign.md) |
| Plan progression | world-model `agent` | No current Plan store or selector | Reconcile product milestones and request reconstruction | `new local behavior` | Generic completed state would collapse independent visibility barriers | [Strategy redesign](../../strategy_plan_redesign.md) |
| Curation operation execution | future world-model `curation` | No current domain | Consume authorized bounded work and publish terminal semantic products | `new local behavior` | Event replay cannot itself grant authority or currentness | [Event-backed EpiOp review](../../reviews/event_backed_epistemic_operations_review.md) |
| Waiting vocabulary | world-model `waiting` | Diagnostic condition strings | Publish owner wait receipts with structural wake references | `extend existing` | Root must not interpret condition meaning | [world-model waiting](../../../../../../crates/meld-world-model/src/waiting.rs) |
| Execution Goal storage | `meld-execution` Goals | Durable idempotent Agent command acceptance and lifecycle epochs | Consume only complete executable products | `reuse unchanged` | Goal record currently carries Strategy internals as operational copy | [Goal Set](../../../../../../crates/meld-execution/src/goals.rs) |
| Execution lowering | `meld-execution` planning | Authorized revalidation, legacy Method search, durable Task Network mutation | Lower a complete Task without becoming Strategy | `extend existing` | Current actor mixes consumer realization with semantic planning | [planning runtime](../../../../../../crates/meld-execution/src/planning/runtime.rs) |
| Execution dispatch | `meld-execution` Task Network | Durable ready set, claims, outcomes, and retries | Continue independent How and When ownership | `reuse unchanged` | Package handoff arrives through process memory | [dispatch actor](../../../../../../crates/meld-execution/src/task_network/dispatch_actor.rs) |
| Execution publication | `meld-execution` Task Network | Durable publication outbox and idempotent Event append | Publish result and expose exact append milestone | `reuse unchanged` | Aggregate publication discovery depends on in-memory handoff | [publication bridge](../../../../../../crates/meld-execution/src/task_network/publication.rs) |

## Scope Separation

The runtime operating path touches PDS selection and theory, initialization, workspace or other product observation, Events, Traversal, Curation, Belief, Agent, Strategy, Execution, provider adapters, and observational status surfaces.

Behavior proven incomplete is narrower. It consists of observation participation and durability, complete initialization readiness, generation-bound runtime composition, Curation runtime entry and recovery, graph-driven configured Belief eligibility, Agent Plan progression, domain wait-to-wake closure, and truthful liveness and retirement projection. Current Execution also contains an in-memory package handoff and a legacy semantic planning route that prevent a single clean runtime account.

Likely write scope is not authorized and is intentionally not inferred here. Several domains on the operating path already have complete local behavior and may be reused unchanged. In particular, Events append and replay, consumer-owned cursors, Agent decision-before-sink ordering, persistent Goal admission, Task Network claims, and per-Task publication are evidence to preserve rather than reasons to rewrite those domains.

## Explicit Non-Integration Decisions

`meld-lang` should not gain lifecycle, wait, wake, or reconciliation grammar. It remains permissive shared vocabulary.

Execution should not consume Traversal, Belief, Curation, heterogeneous Plan progression, or epistemic milestone semantics. It receives a complete executable product and publishes execution outcomes.

Events should not interpret Curation rules, Strategy Plans, Belief meaning, or Agent authority. It owns durable transport and replay.

Traversal should not decide currentness, perspective applicability, Curation validity, or Belief admission.

Root runtime should not interpret foreign waiting conditions. It may verify that the structural wake references supplied by owners resolve to present participants or durable transports.

PDS routing should not sequence runtime work. It can supply exact installed meaning, assignment, activation inputs, and expected participant closure, then leave the cognitive path.

Legacy Workflow, control orchestration, old root Agent registry, heads compatibility, session lifetime, logging, views, generic storage, and Merkle traversal do not become owners of World Model Reconciliation lifecycle merely because current adapters pass through them.

## Evidence Confidence

Confidence is high that current Events, Traversal, Evidence ingestion, Agent subscription, Goal Set, Task Network, and publication components have the local durability and replay behavior stated here. Their public contracts and store ordering are explicit in code and covered by focused tests.

Confidence is high that the lifecycle connective gaps are current facts. Production searches found no adapter from `WorkerTickReport.waiting_on` to `OwnerWaitReceiptV1`, no production use of `project_liveness`, no production call to startup activation publication, no supervised workspace observation participant, no Curation domain, no Strategy Plan store, and no graph-change path that generally dirties configured Belief keys.

Confidence is medium for the exact future handoff shape around planned Curation. The proposed architecture clearly requires durable authorization, terminal results, independent visibility barriers, and replay safety, but current evidence does not select a typed command port versus an accepted-request Event as the primary request carrier.

Confidence is medium for the intended retirement of the in-memory package route. The current code explicitly describes restart by replanning as its recovery model. That model is locally coherent, but it conflicts with the stronger canonical rule that no process-local handoff may be the sole explanation of unresolved program work.

## Unresolved Evidence Questions

The current repository does not determine whether standing Curation owns a ledger cursor, a graph revision cursor, or a durable work index derived from both.

It does not determine how a Strategy Plan names downstream visibility milestones, or whether Agent records a separate acceptance receipt after observing them.

It does not determine whether Belief graph invalidation is driven by Traversal publication, an Event mapping, or a Belief-owned dependency index over graph source identities.

It does not determine how one PDS product expression divides its participant closure across one Agent or several Agents.

It does not determine whether the current activation lifecycle store and startup activation store are intended to converge into one production authority. They presently duplicate activation-generation concepts without a production bridge.

It does not determine how passive workspace observation is admitted, fenced, and recovered across activation replacement without reviving legacy Workflow scheduling.

These are the remaining discovery edges. They do not change the main evidence result: actor-local durability is strong, while full producer-to-consumer lifecycle closure is not yet implemented.

## Validation

Focused validation passed 12 lifecycle-store tests, 42 supervisor tests, 21 world-model tests, 177 Execution tests, and the integration stall specimen.

The passing tests expose the connectivity gap especially clearly. The lifecycle-store suite proves that zero work without complete waits is stalled. The supervisor suite separately proves that zero-work ticks project active idle. The stall specimen proves that wait narration reaches the diagnostic eligibility walk. All three pass because no tested production path turns the actor wait narration into lifecycle wait receipts and no production liveness projection consumes those receipts.

The lexical stepping behavior is likewise exercised only as independent actor ticks. Current supervisor tests prove that every bound actor can be stepped and reported. They do not prove a structural wake edge from one producer milestone to its consumer. The all-actor periodic pass masks that absence during a live process because every consumer eventually polls again. Restart, long tick intervals, a missing participant, or a non-Event owner revision exposes the missing connection.

Local Markdown links, prohibited prose parentheses, reconciliation terminology, trailing whitespace, and repository diff whitespace all validate.
