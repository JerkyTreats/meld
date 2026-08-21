# Runtime Lifecycle And Supervision Review

Date: 2026-08-20

Status: discovery evidence, implementation not authorized

## Finding

The current runtime is not missing a flywheel. It has durable domain actors, bounded stepping, replay-safe cursors, execution claims, leases, heartbeats, restart policy, event-driven wake, and truthful unresolved-binding projection. The missing behavior is the assignment-local lifecycle that proves those independently sound mechanisms form one live World Model Reconciliation program.

The sharp finding is:

```text
The supervisor owns process motion.
Domains own semantic progress and waiting.
Root composition must prove their structural closure for one activation generation.
That proof is not connected today.
```

The current code contains most of the intended lifecycle nouns. It does not yet connect them into the canonical `intend → prepare → realize → publish → work or wait → fence → drain → retire` protocol from [Runtime Lifecycle And Quiescence](../../../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md).

## Concern And Limits

This review traces activation generations, participant incarnations, actor stepping, leases, heartbeats, readiness, work, active idle, owner waits, wake viability, quiescence, stalled projection, fencing, drain, safe points, shutdown, interruption, restart, reopen, late results, and retirement.

It asks whether every producer and consumer in proposed World Model Reconciliation has a durable handoff and a shared lifecycle identity. It does not assess Strategy semantics, Curation semantics, Traversal design, PDS package syntax, implementation sequencing, migration sequencing, or final contract shapes.

The review uses current code as implementation truth and the canonical lifecycle document as target authority. Proposed reconciliation behavior is evidence for needed participation, never evidence that behavior already exists.

## Evidence Classification

Implemented means a production path calls the behavior or a public domain actor carries the behavior with durable tests.

Disconnected means an implemented primitive exists but its producer or consumer is not connected to the product lifecycle.

Compatibility means the behavior preserves the earlier flywheel, Workflow, watch-daemon, or hand-composed startup path and is not a canonical World Model Reconciliation owner.

Absent means neither a production connection nor an adequate primitive was found.

The inspection covered the complete current top-level domain set and directly relevant source under runtime, initialization, theory, world-model, events, execution, capability, provider, workspace, dependency security, and harness. Discovery stopped after the lifecycle paths named in the charter and their direct producers and consumers were inspected.

## Current Lifecycle Ground

### Operational supervision is implemented

The root supervisor has a real operational lifecycle. It persists desired state, process instances, leases, heartbeats, health snapshots, lifecycle events, restart records, shutdown state, and bounded actor reports. It acquires a lease before starting an actor, renews leases during ticks, detects expiration, applies bounded restart policy, and refuses to call a body-less required actor healthy. See [supervisor entrypoint](../../../../../../src/runtime/supervisor/entrypoint.rs), [supervisor store](../../../../../../src/runtime/supervisor/store.rs), and [bounded stepping](../../../../../../src/runtime/supervisor/stepping.rs).

Actor execution is bounded and every active actor must return a report. World-model graph replay, evidence ingestion, belief assessment, Agent curation, Execution planning, task dispatch, and publication have adapters in [runtime assembly](../../../../../../src/runtime/assembly.rs). Domain actors preserve meaningful progress in their own stores through event cursors, graph outboxes, belief leases, Agent subscription cursors, Goal records, Task Network claims, and publication outboxes.

The supervisor stores active handles in a sorted map and ticks them in runtime-identifier order. That makes the current order deterministic, but it is not the operational dependency order in the activation participant plan and it provides no visibility barrier between producers and consumers. Correctness therefore depends on durable handoffs and repeated bounded passes. Any actor coupling that silently requires same-pass producer-before-consumer order is a lifecycle defect, while an explicit owner checkpoint or event boundary is reopen-safe.

Event-driven wake is also implemented in one useful form. The foreground runtime waits on the event authority watermark and falls back to a timed heartbeat. A committed event wakes the entire supervisor loop early. See [runtime tooling](../../../../../../src/runtime/tooling.rs) and [event subscriptions](../../../../../../crates/meld-events/src/events/subscription.rs).

These mechanisms support a durable runtime. They do not yet prove an activation-wide live program.

### Activation lifecycle vocabulary is implemented but disconnected

[Theory activation](../../../../../../src/theory/activation.rs) defines an exact participant plan with owner, required status, mode, dependencies, and references for readiness, wake, safe point, and stop behavior. It also defines a prepared closure over assignment, activation, owner receipts, capability closure, participant plan, bindings, and authority inputs.

[Startup activation](../../../../../../src/runtime/activation.rs) can persist an initial assignment generation and publish it only after receiving readiness references for required participants. No production caller of `StartupActivationStore` or `publish_startup` was found. The implementation only creates generation one and only supports an absent prior head.

[Portable lifecycle](../../../../../../src/runtime/lifecycle.rs) defines lifecycle intents, participant incarnations, owner lifecycle ports, durable operations and attempts, structural wake references, owner wait receipts, liveness projection, admission fencing, and fenced-quiescence receipts. The stable lifecycle actor is supervised and can advance one submitted intent. Its only production transition is to record that exact inputs are pending. No production caller submits an intent. No implementation of the owner preparation, readiness, wake, safe-point, or stop ports was found.

The lifecycle service therefore runs beside the flywheel. It does not currently activate, supervise, fence, recover, or retire that flywheel.

### Runtime registration is not the participant plan

The active runtime set is derived by walking the first-proof runtime descriptor catalog for a selected stewardship expression. It creates thirteen registrations and classifies them as active actors or passive services. This path does not consume the prepared activation closure or its participant plan. See [registration derivation](../../../../../../src/runtime/assembly.rs) and [registration lifecycle](../../../../../../src/runtime/registration.rs).

This creates two descriptions of participant closure:

```text
Activation participant plan
Runtime registration set
```

They have no implemented identity or parity proof between them. Supervisor leases name runtime identifiers and process instances. They do not name activation generations, participant specifications, or participant incarnations. An operational restart creates a new supervisor lease and process-local handle but no durable participant incarnation.

### Initialization ends before activation

The current world initialization pipeline installs theory, creates Agent identities and subscriptions, marks the Agent operational, and appends an epistemic genesis fact. These are durable and idempotent stages. See [world initialization](../../../../../../src/init/world.rs) and [world initialization pipeline](../../../../../../src/init/world/pipeline.rs).

Actor assembly happens later by reopening stores and resolving whatever initialization left behind. Missing theory or genesis state produces body-less actors and truthful unresolved bindings. This is good failure reporting, but it is not a readiness protocol. No stage prepares owner lifecycle state, realizes participants with admission closed, collects exact readiness receipts, or publishes an activation generation as current.

Legacy initialization still installs default prompts, Agents, and Workflow files through a separate path in [legacy initialization](../../../../../../src/init.rs). That path is compatibility-only for this concern and must not become another World Model Reconciliation lifecycle.

### Active idle is implemented, quiescence is not

Domain actors emit owner-vocabulary waiting declarations through their bounded reports. Root preserves those declarations on durable action records without interpreting them. This is a sound diagnostic seam. The world-model and Execution vocabularies live in [world-model waiting](../../../../../../crates/meld-world-model/src/waiting.rs) and [Execution waiting](../../../../../../crates/meld-execution/src/waiting.rs).

The supervisor correctly projects a clean zero-work report as `ActiveIdle`. It has no canonical `Quiescent` or `Stalled` registration state. The separate lifecycle store can project quiescent or stalled from owner wait receipts and resolvable wake references, but bounded actor reports are never converted into those receipts and no wake owner resolves the references.

The operator tick account nevertheless labels a pass `quiescent` whenever all invoked actors report no work. It also labels a pass with no actors as quiescent. That is the exact rejected equivalence in the canonical lifecycle design. The correct implemented fact is active idle.

Operational health and semantic liveness are mostly separate in the core types. A clean tick becomes healthy because the actor ran successfully, while registration lifecycle becomes active idle. The conflation occurs in the operator-facing tick-account field and in the absence of an activation-wide liveness projection. A healthy supervisor can therefore report clean ticks while an unresolved standing responsibility has no viable wake path.

### Wake is viable only for one broad source

The event authority provides durable append, replay, consumer cursors, a commit watermark, and early wake for the foreground tick loop. This gives event-driven work a genuine wake path.

The canonical lifecycle also permits owner revision, deadline, passive subscription, and operator wake references. Those forms exist only as lifecycle enum values. They are not resolved by the running product. Even event wake is broad: any committed event wakes every supervised actor. The activation lifecycle does not prove that an idle participant has a relevant event subscription or a durable cursor that can make its declared wait eligible.

The harness can walk persisted waiting declarations and explain an absent result through hard-coded first-flywheel couplings. This is useful observation, not wake viability or lifecycle authority. See [eligibility walk](../../../../../../src/harness/eligibility.rs).

### Shutdown stops processes but does not retire a generation

Graceful supervisor shutdown marks the process instance stopping, calls stop on every handle, immediately observes each handle safe after its local started flag becomes false, flushes stores, releases leases, and marks the instance stopped. The configured shutdown grace is not consumed by this path. Actors have no implemented stop behavior. Store iteration order is not the reverse dependency order declared by a participant plan.

No activation admission fence precedes shutdown. No already accepted obligations are drained. No owner safe-point receipt, Execution unresolved-operation summary, passive-subscription fence receipt, participant drain receipt, aggregate fenced-quiescence receipt, or retirement receipt is collected. The current safe point proves only that root stopped calling the handle. It does not prove the owner reached a durable semantic safe point.

### Recovery is strong locally and absent globally

Several domain runtimes reopen from durable state correctly. Graph replay retains its cursor and derived-event outbox. Belief assessment recovers expired domain leases and resumes checkpoints. Agent delivery advances its subscription cursor only after durable downstream effects. Execution resumes fenced Task Network claims and publication outboxes.

The supervisor also recovers expired operational leases and recreates handles. It does not associate the new handle with a participant incarnation, close activation admission during reconstruction, reconcile prior incarnation lineage, or require owner recovery readiness before work resumes.

The portable lifecycle store can record a recovered incarnation and close admission. That behavior is isolated to its own tests. There is no aggregate recovery of activation intent, prepared closure, owner checkpoints, subscriptions, operations, participants, and incomplete lifecycle phase.

### Late-result primitives exist in the wrong aggregate

The root runtime owns experimental durable operation, attempt, late-result, and passive-delivery records. Dependency Security uses the operation store in a portable fixture to prove local and serialized adapter equivalence. Passive delivery has unit tests for retired-generation handling. Neither path is connected to supervised World Model Reconciliation.

The canonical ownership is different. Execution owns durable executable operations and attempts. A destination domain owns admission of a result into its canonical product. Root lifecycle may aggregate their receipts but must not own their semantic statuses. The current portable records are useful behavioral probes, but their placement under root runtime centralizes semantics that the canonical lifecycle explicitly leaves with owners.

## Lifecycle Concern Matrix

| Concern | Current ground | Classification | Connection verdict |
| --- | --- | --- | --- |
| Activation generation | Two durable head implementations share the same storage tree name | Disconnected | No production activation protocol selects one authority |
| Participant plan | Exact structural plan with dependencies and lifecycle references | Disconnected | Runtime registration ignores it |
| Participant incarnation | Durable record and recovery constructor | Disconnected | Supervisor handle and lease never cite it |
| Actor stepping | One bounded report per active actor per tick | Implemented | Sound operational seam |
| Tick order | Deterministic runtime-identifier order | Partial | Does not realize participant dependencies or producer visibility barriers |
| Operational lease | Durable acquisition, renewal, expiry, release, and ownership fencing | Implemented | Names runtime role, not activation participant |
| Heartbeat | Durable process health with restart inputs | Implemented | Does not claim semantic liveness |
| Structural readiness | Body resolution and unresolved-binding projection | Partial | No owner readiness receipt or activation-wide barrier |
| Work | Durable domain checkpoints and committed outputs | Implemented | Progress remains owner-local as intended |
| Active idle | Clean zero-work bounded report | Implemented | Correct local operational fact |
| Owner waits | Owner vocabulary reaches durable action reports | Partial | No durable lifecycle receipt or participant identity |
| Wake viability | Event watermark wakes the broad tick loop | Partial | No per-wait structural resolution and most wake kinds absent |
| Quiescence | Test-only projection from supplied waits and wakes | Disconnected | Operator account incorrectly equates clean ticks with quiescence |
| Stalled | Test-only projection for missing waits or broken wakes | Disconnected | No product projection or event admission |
| Admission fence | Durable generation flag in portable lifecycle store | Disconnected | Running actors and result paths do not consult it |
| Drain | No owner-wide accepted-obligation drain | Absent | Supervisor stops handles immediately |
| Safe point | Handle started flag becomes false | Compatibility | Does not cite owner checkpoint or generation |
| Shutdown | Process stop, flush, lease release, and instance closure | Implemented | Operational only, not fenced generation retirement |
| Interruption | Expired leases and durable domain reopen behavior | Partial | No assignment generation interrupted state |
| Restart | Operational policy recreates handle and lease | Implemented | No incarnation lineage or closed-admission recovery barrier |
| Reopen | Strong domain-local replay and idempotency | Implemented | No aggregate incomplete-phase reconstruction |
| Late executable result | Portable operation store can reject or retain historical result | Disconnected | Root owns experimental Execution semantics |
| Late passive delivery | Portable subscription store fences incarnation and generation | Disconnected | No production passive source uses it |
| Retirement | Fenced-quiescence record shape only | Absent | No stop receipts, release aggregation, or retired head state |

## Regenerated Domain Snapshot

The domain universe was regenerated from current source with:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

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

The separately governed crates relevant to this concern are `meld-events`, `meld-execution`, and `meld-world-model`. `meld-lang` has no runtime lifecycle role.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Legacy prompt Agent configuration | `not needed` | [legacy initialization](../../../../../../src/init.rs) | World-model Agent owns reconciliation participation | Preserve as compatibility only |
| `api` | `none` | Older progress runtime facade | `not needed` | [API](../../../../../../src/api.rs) | It does not host the canonical supervisor | None |
| `branches` | `none` | Branch identity supports event binding | `not needed` | [branches](../../../../../../src/branches.rs) | Branch identity is activation input, not lifecycle behavior | None |
| `capability` | `publish` | Exact contracts, inventories, invokers, and activation inputs exist | `partial` | [capability](../../../../../../src/capability.rs) | Not applicable | Separate availability from durable operation ownership |
| `cli` | `adapter` | Run, status, wake loop, and shutdown presentation exist | `partial` | [runtime tooling](../../../../../../src/runtime/tooling.rs) | Not applicable | Present activation liveness without deriving it |
| `compat` | `none` | Compatibility exports only | `not needed` | [compat](../../../../../../src/compat.rs) | Canonical lifecycle must not depend on compatibility | None |
| `concurrency` | `none` | Generic coordination support | `not needed` | [concurrency](../../../../../../src/concurrency.rs) | No lifecycle source truth lives here | None |
| `config` | `publish` | Assignment, activation, runtime timing, and physical bindings exist | `partial` | [stewardship config](../../../../../../src/config/stewardship.rs) | Not applicable | Keep declarations inert and exact |
| `context` | `none` | Context data is consumed by legacy and execution paths | `not needed` | [context](../../../../../../src/context.rs) | Context does not own participant lifecycle | None |
| `control` | `none` | Older orchestration state | `not needed` | [control](../../../../../../src/control.rs) | World Model Reconciliation rejects a second orchestration owner | None |
| `dependency_security` | `consume` | Portable fixture uses root operation and late-result records | `partial` | [portable security operation](../../../../../../src/dependency_security/portable.rs) | Not applicable | Preserve owner admission while removing root semantic ownership |
| `docs` | `none` | Bounded capability implementations and owner products | `not needed` | [docs](../../../../../../src/docs.rs) | Current synchronous docs work can rely on Execution operation lifecycle | None |
| `error` | `none` | Shared error mapping | `not needed` | [error](../../../../../../src/error.rs) | Error transport is not lifecycle authority | None |
| `events` | `own` | Durable authority, replay, cursors, watermark wake, and health | `partial` | [event subscriptions](../../../../../../crates/meld-events/src/events/subscription.rs) | Not applicable | Expose exact durable wake boundaries to lifecycle resolution |
| `execution` | `own` | Goal, Task Network, claims, bounded dispatch, outboxes, and reopen are durable | `partial` | [Execution crate](../../../../../../crates/meld-execution/src/lib.rs) | Not applicable | Publish owner wait, safe-point, and unresolved-operation products |
| `harness` | `observe` | Canonical boot, bounded drive, projections, and eligibility walk exist | `partial` | [harness boot](../../../../../../src/harness/boot.rs) | Not applicable | Observe activation lifecycle instead of inferring quiescence |
| `heads` | `none` | Legacy head index | `not needed` | [heads](../../../../../../src/heads.rs) | Compatibility source is outside current lifecycle | None |
| `ignore` | `none` | Workspace selection policy | `not needed` | [ignore](../../../../../../src/ignore.rs) | It changes scan semantics, not runtime lifecycle | None |
| `init` | `adapter` | Idempotent theory, Agent genesis, and epistemic seed pipeline | `partial` | [world initialization](../../../../../../src/init/world.rs) | Not applicable | Connect complete inert initialization to activation intent |
| `lib` | `adapter` | Root exports runtime and domain facades | `complete` | [crate surface](../../../../../../src/lib.rs) | Not applicable | Export owner contracts only when they exist |
| `logging` | `none` | Logs operational output | `not needed` | [logging](../../../../../../src/logging.rs) | Observation does not establish liveness | None |
| `merkle_traversal` | `none` | Workspace tree scanning | `not needed` | [Merkle traversal](../../../../../../src/merkle_traversal.rs) | Scan execution is a capability concern | None |
| `metadata` | `none` | Artifact metadata | `not needed` | [metadata](../../../../../../src/metadata.rs) | No participant lifecycle ownership | None |
| `prompt_context` | `none` | Prompt lineage and artifacts | `not needed` | [prompt context](../../../../../../src/prompt_context.rs) | Strategy inputs do not create lifecycle authority | None |
| `provider` | `publish` | Provider availability and execution binding exist | `partial` | [provider](../../../../../../src/provider.rs) | Not applicable | Make long-running completion obey operation lineage and owner admission |
| `runtime` | `own` | Assembly, registration, supervisor, lifecycle prototypes, delivery, status, and storage | `partial` | [runtime](../../../../../../src/runtime.rs) | Not applicable | Connect structural lifecycle without absorbing owner semantics |
| `serve` | `none` | API service hosting | `not needed` | [serve](../../../../../../src/serve.rs) | It does not own the product supervisor | None |
| `session` | `none` | Command session lifecycle | `not needed` | [session](../../../../../../src/session.rs) | Command lifetime differs from program and activation lifetime | None |
| `store` | `none` | Generic node storage | `not needed` | [store](../../../../../../src/store.rs) | Product lifecycle uses owner stores and runtime storage assembly | None |
| `task` | `none` | Root legacy Task facade | `not needed` | [task](../../../../../../src/task.rs) | Canonical Task Network lifecycle belongs to `meld-execution` | None |
| `telemetry` | `none` | Legacy session and summary observation | `not needed` | [telemetry](../../../../../../src/telemetry.rs) | Root runtime already owns operational reports | None |
| `theory` | `publish` | Participant plan and prepared activation closure exist | `partial` | [theory activation](../../../../../../src/theory/activation.rs) | Not applicable | Make this the structural input to participant realization |
| `tree` | `none` | Tree presentation and compatibility helpers | `not needed` | [tree](../../../../../../src/tree.rs) | No runtime lifecycle ownership | None |
| `types` | `none` | Shared compatibility types | `not needed` | [types](../../../../../../src/types.rs) | New lifecycle meaning must stay domain-owned | None |
| `views` | `none` | Presentation read models | `not needed` | [views](../../../../../../src/views.rs) | Read models do not own lifecycle | None |
| `workflow` | `none` | Explicitly legacy Workflow runtime | `not needed` | [workflow](../../../../../../src/workflow.rs) | Strategy is the only canonical workflow | Keep outside World Model Reconciliation |
| `workspace` | `publish` | Canonical scan publication exists beside an independent legacy watch daemon | `partial` | [workspace](../../../../../../src/workspace.rs) | Not applicable | Route canonical source wake through events or a declared passive participant |
| `world_state` | `publish` | World-model actors own durable checkpoints, waits, leases, and replay state | `partial` | [world-model crate](../../../../../../crates/meld-world-model/src/lib.rs) | Not applicable | Publish owner lifecycle products without depending on root runtime |

## Frozen Affected-Domain Set

The affected set is frozen after pass one:

```text
capability
cli
config
dependency_security
events
execution
harness
init
lib
provider
runtime
theory
workspace
world_state
```

Domains traversed by runtime are not automatically behavior-change or write scope. In particular, docs, branches, context, metadata, and prompt context may be read or acted upon without joining the lifecycle design.

## Pass Two Affected-Domain Decomposition

### Capability

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Contract inventory | Capability | Deterministic owner contributions | Publish exact selectable revisions | `reuse unchanged` | Lifecycle could mistake inventory for availability | [capability inventory](../../../../../../src/capability.rs) |
| Activation binding | Capability | Exact activation request and invoker checks exist | Supply capability closure and current availability | `extend existing` | PDS theory could absorb runtime availability | [capability runtime](../../../../../../src/capability/runtime.rs) |
| Invocation | Execution and capability owner | Bounded synchronous invocation exists | Remain behind Execution operation lineage | `extend existing` | Root lifecycle currently owns experimental operation semantics | [execution capability](../../../../../../crates/meld-execution/src/capability.rs) |

### CLI

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Foreground run | CLI adapter | Supervisor tick plus ledger wake and timed fallback | Drive the published activation generation | `adapter only` | CLI may become hidden lifecycle authority | [runtime tooling](../../../../../../src/runtime/tooling.rs) |
| Status | CLI adapter | Operational health and per-tick accounts | Present owner-derived lifecycle projection | `adapter only` | Clean tick is currently mislabeled quiescent | [runtime tooling](../../../../../../src/runtime/tooling.rs) |
| Shutdown | CLI adapter | Calls immediate operational shutdown | Request activation fence and retirement through runtime owner | `adapter only` | Adapter sequencing semantic shutdown | [runtime tooling](../../../../../../src/runtime/tooling.rs) |

### Config

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Assignment | PDS configuration | Durable identity and semantic binding shape exist | Name the standing responsibility | `reuse unchanged` | Treating assignment as a live participant | [assignment](../../../../../../src/config/stewardship/assignment.rs) |
| Activation | PDS configuration | Physical bindings, implementations, isolation, and limits exist | Supply exact activation inputs | `extend existing` | Mixing selected meaning with current authority | [activation](../../../../../../src/config/stewardship/activation.rs) |
| Runtime timing | Root configuration | Heartbeat, lease, tick, and shutdown values exist | Configure operational policy only | `reuse unchanged` | Timers becoming semantic waiting policy | [runtime assembly](../../../../../../src/runtime/assembly.rs) |

### Dependency Security

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Adapter result | Dependency Security | Result carries assignment, activation, generation, operation, attempt, source, and completeness | Validate lineage before canonical admission | `extend existing` | Root decides security result validity | [portable security operation](../../../../../../src/dependency_security/portable.rs) |
| Portable execution proof | Dependency Security | Uses root lifecycle store in a test-only fixture | Preserve behavior while moving operation truth to Execution | `adapter only` | Prototype hardens into wrong ownership | [portable security operation](../../../../../../src/dependency_security/portable.rs) |

### Events

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Append and replay | Events | Durable authority and bounded replay are mature | Carry canonical changes and recovery input | `reuse unchanged` | Root attempts to own foreign cursors | [events](../../../../../../crates/meld-events/src/events.rs) |
| Consumer progress | Consumer owner with Events registry | Owner cursor plus registry watermark exists | Publish exact durable checkpoint references | `extend existing` | Ledger treated as cursor owner | [event subscriptions](../../../../../../crates/meld-events/src/events/subscription.rs) |
| Wake transport | Events | Commit watermark wakes the whole tick loop | Resolve event-position wake viability structurally | `extend existing` | Event type interpretation moves into root | [event subscriptions](../../../../../../crates/meld-events/src/events/subscription.rs) |
| Passive delivery | Source and destination owner | Root prototype is isolated | Carry generation and incarnation lineage through admitted source delivery | `new local behavior` | Parallel passive delivery ledger | [passive delivery](../../../../../../src/runtime/delivery.rs) |

### Execution

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Goal Set | Execution | Durable Goal command, mutation, and query surfaces exist | Remain admission seam from world-model | `reuse unchanged` | Lifecycle starts deciding why Goals exist | [Execution Goals](../../../../../../crates/meld-execution/src/goals.rs) |
| Task Network work | Execution | Bounded planning, dispatch claims, terminal recording, and publication exist | Publish progress and wait products | `extend existing` | Root interprets task readiness | [Task Network](../../../../../../crates/meld-execution/src/task_network.rs) |
| Operations and attempts | Execution | Task claims and retries are durable, while generic operation prototype sits in root | Own executable operation truth and unresolved-effect summary | `extend existing` | Two operation systems diverge | [dispatch actor](../../../../../../crates/meld-execution/src/task_network/dispatch_actor.rs) |
| Safe point | Execution | Reopen-safe claims and outboxes exist | Declare accepted obligations settled or durably uncertain | `new local behavior` | Supervisor treats stopped polling as settled effects | [Task Network](../../../../../../crates/meld-execution/src/task_network.rs) |

### Harness

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Boot and drive | Harness | Uses product binding, init, assembly, and supervisor with injected time | Exercise the same activation protocol as product | `extend existing` | Harness boot remains more coherent than product boot | [harness boot](../../../../../../src/harness/boot.rs) |
| Eligibility walk | Harness | Explains preserved wait declarations | Observe owner waits and broken structural wake | `extend existing` | Hard-coded coupling becomes runtime authority | [eligibility walk](../../../../../../src/harness/eligibility.rs) |
| Projections | Harness | Builds customer views from reports and stores | Include activation-wide liveness and lineage | `extend existing` | Projection invents missing owner truth | [harness projections](../../../../../../src/harness/projections.rs) |

### Init

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Theory installation | Owner domains through root adapter | Exact installed revisions and receipts are durable | Produce inert owner preparation inputs | `extend existing` | Init becomes semantic owner | [world initialization pipeline](../../../../../../src/init/world/pipeline.rs) |
| Agent genesis | World-model through root adapter | Agent, subscription, maintained condition, and rule binding are durable | Produce owner readiness prerequisites | `extend existing` | Agent marked operational before activation authority exists | [world initialization pipeline](../../../../../../src/init/world/pipeline.rs) |
| Activation handoff | Root initialization | No stage exists | Submit exact lifecycle intent after inert closure | `new local behavior` | Initialization directly starts actors | [world initialization](../../../../../../src/init/world.rs) |
| Legacy defaults | Legacy init | Prompts, Agents, and Workflow files use a second path | Stay compatibility-only | `not needed` | Reintroduced parallel product lifecycle | [legacy initialization](../../../../../../src/init.rs) |

### Lib

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Public surface | Root crate | Reexports current domain APIs | Expose only explicit owner and lifecycle boundaries | `adapter only` | Shared types erase ownership | [crate surface](../../../../../../src/lib.rs) |

### Provider

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Availability | Provider | Provider selection, clients, and diagnostics exist | Publish current operational readiness | `extend existing` | Provider choice moves into semantic package |
| Long-running completion | Provider adapter under Execution | HTTP and generation behavior have their own limits | Return through durable operation attempt and owner admission | `extend existing` | Process timeout becomes semantic failure | [provider](../../../../../../src/provider.rs) |

### Runtime

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Assembly | Root runtime composition | Opens scoped stores, binds ports, and builds inert actor factories | Realize the exact participant plan | `extend existing` | Root creates semantic state | [runtime assembly](../../../../../../src/runtime/assembly.rs) |
| Registration | Root runtime composition | Stewardship expression expands to all first-proof descriptors | Represent exact participant closure and required status | `extend existing` | Second participant inventory drifts from activation plan | [runtime registration](../../../../../../src/runtime/registration.rs) |
| Supervisor | Root runtime supervisor | Strong process leases, heartbeats, steps, restart, and stop | Supervise participant incarnations under generation fence | `extend existing` | Operational health substitutes for program liveness | [supervisor entrypoint](../../../../../../src/runtime/supervisor/entrypoint.rs) |
| Activation publication | Root runtime composition | Startup-only store is not called | Publish current generation after exact readiness | `extend existing` | Competing head stores and authorities | [startup activation](../../../../../../src/runtime/activation.rs) |
| Lifecycle coordination | Root lifecycle | Rich record vocabulary and a one-transition actor exist | Aggregate owner receipts without interpreting them | `extend existing` | Root owns operations, waits, or result validity | [portable lifecycle](../../../../../../src/runtime/lifecycle.rs) |
| Waiting and wake | Owners plus root projection | Waiting declarations persist, event wake runs, viability is absent | Resolve structural references and project liveness | `extend existing` | Root parses owner condition vocabulary | [runtime contracts](../../../../../../src/runtime/contracts.rs) |
| Shutdown and retirement | Root plus owner domains | Immediate handle stop and flush | Fence, drain, aggregate safe points, then stop | `extend existing` | Stop flag mistaken for semantic safe point | [supervisor entrypoint](../../../../../../src/runtime/supervisor/entrypoint.rs) |
| Passive delivery | Root prototype | Generation-aware isolated store exists | Route lineage while destination owns admission | `extend existing` | Second event or operation system | [passive delivery](../../../../../../src/runtime/delivery.rs) |
| Self observation | Runtime | Threshold crossings become durable events | Admit operational failures as world-model evidence | `reuse unchanged` | Heartbeat samples flood cognition | [self observation](../../../../../../src/runtime/self_observation.rs) |

### Theory

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Participant plan | PDS linker with owner contributions | Exact structural shape and acyclic dependency validation exist | Be the sole activation participant declaration | `extend existing` | Plan encodes Steward semantic choreography | [theory activation](../../../../../../src/theory/activation.rs) |
| Prepared closure | PDS linker with owner receipts | Exact inert closure exists | Supply immutable lifecycle preparation input | `extend existing` | Receipt references accepted without owner resolution | [theory activation](../../../../../../src/theory/activation.rs) |

### Workspace

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Scan and publication | Workspace | Scan produces publication candidates for owning append runtime | Publish canonical source changes through Events | `reuse unchanged` | Workspace writes world-model state directly | [workspace scan](../../../../../../src/workspace/scan.rs) |
| Watch daemon | Legacy workspace runtime | Independent loop reaches legacy Workflow task path | Stay outside canonical reconciliation lifecycle | `not needed` | A second flywheel remains active | [watch runtime](../../../../../../src/workspace/watch/runtime.rs) |
| Passive source role | Workspace or external source adapter | No activation participant connection | Declare readiness, source cursor, wake, fence, and stop when continuously active | `new local behavior` | Root owns workspace source semantics | [workspace watch](../../../../../../src/workspace/watch.rs) |

### World State

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Graph replay | World-model graph | Durable cursor, projection, derived outbox, and bounded catch-up | Publish readiness, wait, wake, and safe point | `extend existing` | Root owns graph cursor | [graph runtime](../../../../../../crates/meld-world-model/src/world_state/graph/runtime.rs) |
| Evidence ingestion | Belief | Durable event cursor and mapping lineage | Publish owner lifecycle products | `extend existing` | Event authority owns belief progress | [evidence ingestion](../../../../../../crates/meld-world-model/src/belief/evidence_ingestion.rs) |
| Belief assessment | Belief | Durable queue, lease recovery, revision, and checkpoint | Publish owner lifecycle products | `extend existing` | Supervisor lease replaces assessment lease | [belief assessment](../../../../../../crates/meld-world-model/src/belief/assessment.rs) |
| Agent participants | Agent | Durable subscription, curation, satisfaction, and step sequence | Publish readiness, waits, and safe points per Agent | `extend existing` | Runtime registration loses Agent cardinality |
| Strategy and Curation participants | World-model owners | Proposed reconciliation changes their work products | Join the same bounded lifecycle without a root semantic sequence | `new local behavior` | Root lifecycle becomes another cognitive runtime |

## Ownership And Boundary Synthesis

Runtime supervision is not the owner of World Model Reconciliation. It is the owner of process and participant motion. The owner domains remain responsible for eligibility, progress, wait meaning, durable checkpoints, result admission, and safe-point meaning.

Root composition has one missing responsibility: it cannot presently prove that the current activation generation has exactly the required participant incarnations, exact readiness receipts, complete owner waits, viable structural wakes, and exact safe-point closure. That connective responsibility is structural. It does not require root to understand why a belief is dirty, why an Agent wants a Goal, why a Task is ready, or why a domain result is valid.

Actor order must remain an operational scheduling choice. Producer and consumer correctness has to cross durable owner products and visibility checkpoints, so a restart, a changed schedule, or a consumer tick before its producer merely yields an owner-declared wait. The participant dependency graph may govern readiness and reverse drain order. It must not become semantic sequencing for reconciliation.

The strongest existing seam is the combination of the exact participant plan, the registration set, bounded actor reports, supervisor lease ownership, and owner-persisted checkpoints. They are independently useful. Their identities do not meet.

Several independent loops therefore exist:

```text
Root supervisor lease and tick loop
Portable activation-lifecycle actor
World initialization stages
Event watermark wake loop
Domain-local replay, lease, claim, and outbox recovery
Legacy workspace watch and Workflow loop
```

Domain-local loops are not themselves a defect. They are where semantic progress belongs. The defect is that activation, supervision, waiting, wake, and retirement do not aggregate their owner receipts under one generation. The legacy watch and Workflow loop is different: it is a competing product loop and should remain explicitly outside the canonical architecture.

## Separated Scopes

### Runtime or operating path

The complete operating path includes config, theory, init, runtime, events, world-state, Execution, capability, provider, workspace sources, CLI, and harness observation.

### Behavior that must change for canonical closure

Current evidence supports behavior changes in runtime composition and lifecycle projection, initialization handoff, owner lifecycle publication from events, world-model and Execution, capability and provider operation lineage, and continuous workspace source participation where enabled.

Dependency Security is an owner-admission proof affected by operation placement. CLI, harness, config, theory, and the crate surface carry or present the behavior but do not become semantic owners.

### Likely implementation units

The evidence points to the existing runtime activation, lifecycle, assembly, registration, supervisor, delivery, and tooling seams; theory activation; world initialization; owner wait and bounded-report adapters in `meld-world-model` and `meld-execution`; event wake resolution; provider and source adapters; and harness projection. This is an evidence map, not an implementation plan or authorization.

## Explicit Non-Integration Decisions

PDS routing must leave the live path after exact owner components and activation inputs are resolved.

Root runtime must not interpret owner waiting-condition strings, decide Goal or Task eligibility, admit foreign results, own world-model cursors, or own executable operation semantics.

Heartbeats, clean actor ticks, empty queues, and process presence must remain operational evidence. None proves program liveness.

`meld-lang` does not need lifecycle guards. It supplies language entities and remains permissive.

Legacy Workflow, root Task compatibility, default Agent and prompt initialization, and the workspace watch Workflow loop must not be used to complete the World Model Reconciliation lifecycle.

Docs, branches, context, metadata, and prompt context do not need direct lifecycle contracts merely because runtime work reads or changes them.

## Smallest Missing Connective Behavior

The smallest missing behavior is an assignment-local structural lifecycle projection and coordinator that correlates, without reinterpreting:

```text
exact activation generation
exact participant plan
current participant incarnations and supervisor leases
owner readiness receipts
owner progress and wait receipts
structurally resolvable wake references
owner safe-point and unresolved-operation receipts
participant stop and lease-release receipts
```

This is not a new cognitive loop. It is the proof that the existing domain loops form one live program and can be replaced, interrupted, recovered, or retired without losing why work exists.

## Unresolved Questions

The current evidence does not settle whether runtime registration becomes the realized form of the participant plan or remains a separate projection with a required parity receipt.

It does not settle how one product expression expands into several Agent-specific participant instances without losing stable role identity.

It does not settle whether activation intent is submitted by PDS lifecycle management, an initialization adapter, or another root application service.

It does not settle the durable shapes of owner readiness, wait, safe-point, stop, and wake-resolution products.

It does not settle how activation replacement coexists with long-running executable operations whose external effect remains uncertain.

It does not settle whether all passive source delivery should use the event authority or whether some sources require a distinct subscription transport that still admits canonical Events.

## Confidence

Confidence is high for the central finding that operational runtime primitives are substantial while activation-wide lifecycle closure is disconnected. The absence claims are supported by direct call-site searches across the relevant source and tests.

Confidence is high that clean-tick quiescence is currently mislabeled, shutdown is operational rather than fenced retirement, and supervisor restarts do not create participant incarnations.

Confidence is medium for likely change ownership at provider and workspace boundaries because exact future remote and passive source shapes remain unresolved in World Model Reconciliation.

No contract drafting, migration sequencing, or implementation authorization is implied by this assessment.
