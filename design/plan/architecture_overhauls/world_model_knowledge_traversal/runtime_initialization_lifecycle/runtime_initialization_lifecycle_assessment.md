# World Model Reconciliation Runtime, Initialization, And Lifecycle Assessment

Date: 2026-08-20

Status: final discovery assessment, implementation not authorized

## Problem Statement

Meld has already built most of the runtime mechanisms needed for a durable reconciliation program. Events can be appended and replayed. Traversal, Belief, Agent, and Execution preserve their own cursors and progress. Actors run under bounded work budgets. The supervisor owns leases, heartbeats, restart policy, and shutdown. Theory installation, Agent genesis, assignment identity, activation identity, participant plans, readiness references, structural wake references, and fenced-quiescence records all exist in some form.

The failure is not a missing flywheel mechanism. The failure is that these mechanisms do not yet prove they are one flywheel.

Today, a configured product can install exact theory, create an Agent, construct runtime factories, start supervised actors, and report clean ticks without one durable account proving that all of those facts belong to the same PDS assignment and activation generation. A producer may publish a durable fact while the intended consumer has no lifecycle-visible position, relevant wake path, or generation fence. The supervisor can restart each actor while no aggregate authority proves that the complete reconciliation program reconstructed the same semantic and physical position.

The harness makes this risk concrete. It constructs semantic actor factories before world initialization runs, then retains those factories after initialization succeeds. The manifest can report successful theory installation and Agent genesis while the later supervisor still holds stale or body-less bindings. This is a locally truthful boot sequence that does not establish globally truthful readiness.

That is the failure mode the earlier flywheel efforts exposed. Independent runtime logic can be sound and still fail to form a live product.

## Thesis

The World Model Reconciliation flywheel is not a scheduler. It is lifecycle closure over independently owned durable transitions.

PDS declares what product should exist. Native owners install and realize their exact portions. Root composition proves that one activation generation has the complete expected participant set, owner-issued readiness, durable producer and consumer positions, viable structural wakes, and a reversible fence and drain path. Only then does it publish that generation as current.

Once current, semantic progress remains decentralized. Product domains observe. Events carries. Traversal materializes. Curation authors epistemic products. Belief selectively admits evidence. Agent reconciles. Strategy constructs a Plan. Execution realizes only eligible executable products. Each owner decides its own eligibility, progress, waiting meaning, result admission, and safe point.

```text
PDS product expression
-> exact owner theory revisions
-> assignment
-> activation and exact participant plan
-> owner preparation and Agent genesis
-> participant realization with admission closed
-> owner readiness closure
-> current generation publication

current generation
-> independent owner work
-> durable owner products
-> consumer visibility milestones
-> owner waits with structural wake references
-> Agent reconciliation
```

Root lifecycle does not sequence cognition. It verifies structural closure. Participant dependencies may govern preparation, readiness, and reverse drain order. They must never become a hidden Workflow that tells Agent, Strategy, Curation, Belief, or Execution how to perform semantic work.

## The Required Runtime Property

Every producer-to-consumer edge in World Model Reconciliation needs one complete lifecycle account:

```text
producer-owned durable commit
+ consumer-owned cursor or exact query
+ deterministic idempotency identity
+ required visibility milestone
+ owner-defined wait
+ structurally resolvable wake
+ activation and freshness fence
+ durable restart source
+ safe retirement behavior
```

This is stronger than Event append and weaker than centralized orchestration.

Event append proves that Events accepted a record. Traversal materialization, Belief admission, Agent acceptance, and Plan progression are later and independent milestones. The Plan dependency must name the milestone it actually needs. Root may confirm that a named milestone and wake reference have an owner and a live transport. It must not decide what the milestone means.

The operating standard should therefore be order independence. A consumer may run before its producer and report an owner-defined wait. A process may restart between commit and consumption and resume from durable positions. Actor identifier order, process timing, or repeated polling may affect latency. None may affect correctness.

## What The Current Runtime Gets Right

The assessment strongly supports the user's intuition that the runtime logic itself is well founded.

Theory routing validates owner components before installation, delegates semantic judgment to native owners, records exact immutable receipts, and withholds the selectable package head when aggregate installation is incomplete. Content identities make retries converge without inventing a central ontology.

Assignments and activations are separate exact values. The assignment captures package, principal, Agent, subject, perspective, branch, requested authority, and grant lineage. The activation captures physical bindings, implementations, placement, isolation, and operating limits. `PreparedActivationClosureV1` can bind those values to owner preparation, Capability closure, participant structure, bindings, and authority inputs while remaining inert.

World initialization has explicit ordered stages and durable idempotent products. Agent registration, theory bindings, Belief subscriptions, and the genesis Event all have stable identities. The implementation stops on error and preserves completed owner work for a later retry.

The supervisor is a real operational owner. It acquires leases before actor start, persists reports before projecting health, renews leases, detects expiration, applies bounded restart policy, refuses to call body-less required actors healthy, and separates durable domain progress from supervisor state.

The semantic actors also recover locally. Traversal retains its Event cursor and derived publication outbox. Belief ingestion writes semantic state before advancing its cursor. Belief assessment recovers its own leases and checkpoints. Agent records a decision before crossing its sink seam and advances its subscription only after the sink receipt is durable. Execution preserves Goal state, Task Network mutations, claims, outcomes, and publication outboxes.

Events already supplies the neutral durable carrier the redesign needs. Its ordered append, idempotent identity, replay, cursors, watermarks, and independent consumer positions should be preserved.

These are the foundation. The assessment does not recommend replacing them.

## Where Initialization Is Disconnected

Current Meld has four separately credible constructions:

```text
PDS package installation

world initialization and Agent genesis

root runtime registration and supervision

activation generation publication
```

No production path proves that they cite one package receipt, assignment, activation, participant plan, Agent topology, owner theory image, and runtime generation.

`WorldInitPipeline` can install theory, create Agent state, and append the first epistemic Event. Its stages can also be selected independently. Agent genesis marks an Agent operational after one active Belief subscription. It does not prove that the view is readable, Traversal reached the needed Event position, Planner can create a frozen projection, Strategy inputs resolve, future Curation intake is available, or the complete participant set is ready.

The Agent becomes operational before the genesis Event is appended. A failed final stage can therefore leave a locally operational Agent without the Event intended to begin its first observation path. A retry can safely append that Event, but there is no aggregate incomplete-initialization state that keeps the Agent inert until the downstream milestone is visible.

Existing Agent identity is also only partially reconciled. Repeated genesis can migrate selected theory revision bindings while retaining an older subject, perspective, branch, directive, provenance, or product lineage under the same Agent id. A future initialization contract must distinguish idempotent reconstruction from silent identity drift.

`PreparedActivationClosureV1` and the exact participant plan are strong structural inputs, but their current construction sites are tests. Runtime registration is instead derived from a fixed first-proof descriptor catalog. The derived registrations do not carry assignment, generation, participant, readiness, wake, safe-point, or stop identities.

`StartupActivationStore` can publish a first generation after supplied readiness references exist, but it has no production caller. It accepts non-empty opaque receipt strings rather than resolving owner, generation, participant incarnation, or selected realization. It supports only generation one and has a publication crash window between head change and the separately stored current record.

The newer activation lifecycle store is a second disconnected authority. It defines intents, incarnations, operations, waits, wakes, admission, liveness projection, and fenced quiescence. The running lifecycle actor only marks an unseen intent as waiting for exact inputs. No production caller submits the intent, and no native owner implements its preparation, readiness, wake, safe-point, or stop ports. Both activation stores use the same assignment-head tree name without sharing one generation record or publication transaction.

The missing behavior is one durable assignment-local initialization account. It must correlate exact package, assignment, activation, Agent genesis, participant plan, realization, owner readiness, and current publication without absorbing the semantic bodies owned by those domains.

## Where Runtime Connectivity Is Disconnected

The full producer and consumer review found strong local handoffs and several decisive breaks.

Workspace observation is not a supervised participant in the stewardship-derived runtime set. The long-lived watcher remains a separate daemon that can still execute legacy Workflow directly. The ordinary scan path builds useful canonical Events, but its common publication route is best-effort telemetry append rather than a durable observation operation with a restart cursor and activation lineage. A reconciliation program cannot remain live if its primary source is outside its participant closure.

Traversal catch-up does not generally make an already assessed configured Belief eligible again. Initial assessment may read a graph anchor. Later reassessment is driven by Belief dirty keys, and current dirtying primarily comes from Evidence assignment rather than graph anchor or relation revision. A new workspace or Curation product may therefore be durable and graph-visible without producing the Belief revision that would wake Agent.

Future standing and planned Curation have no runtime entrance yet. There is no durable Epistemic Operation acceptance state, Curation cursor, result outbox, terminal account, or restart position. Events can carry the future products, but Event replay cannot itself grant current authority or prove that a replayed operation should run again.

The future heterogeneous Plan has the same missing progression boundary. Agent currently persists one executable Strategy authorization and one Goal Set handoff. There is no active Plan identity, revision lineage, per-product eligibility, milestone cursor, or durable reconciliation state for Task and Epistemic Operation products.

Execution is locally durable after the Goal Set seam, but one root package-route handoff remains process memory. Planning writes a selected package route into an in-process map that dispatch and aggregate publication later read. Restart reconstructs the map only if planning selects the same route again. Durable package progress limits duplicate effects, but process memory remains the only immediate explanation for unresolved dispatch and aggregate publication work.

These are not arguments for making root runtime understand workspace claims, Belief mappings, Curation rules, Plans, or Tasks. They are evidence that each owner must expose a durable handoff and lifecycle position at its existing seam.

## Waiting, Wake, And Quiescence

Current bounded actor reports preserve owner-defined waiting declarations. This is a good observational boundary. The declaration contains condition, optional subject, and detail in the producer's vocabulary. Root does not interpret it.

Those declarations are not lifecycle waits yet. `OwnerWaitReceiptV1` separately carries generation, participant incarnation, owner checkpoint, and structural wake references. No production adapter connects bounded reports to those receipts. `OwnerWakePort` has no production implementation. The liveness projector therefore has no running product input.

The foreground loop provides one useful broad wake. A new Event ledger commit can wake the entire supervisor before its heartbeat interval. Other changes, including a Belief revision, Agent store revision, Execution Goal insertion from another process, Task Network mutation, provider completion, activation intent, or restored participant binding, wait for periodic polling unless their producer also appends an Event. Direct-store derived Events may also rely on fallback polling because they do not advance the same writer watermark.

The current system is notification-assisted polling. It is not complete wake closure.

Polling is acceptable as a latency fallback and heartbeat mechanism. It is not sufficient evidence of liveness. A quiet participant must name at least one durable condition that can make work eligible, and root must be able to establish that every structural wake reference resolves to a present owner or transport.

The current supervisor correctly calls a clean zero-work actor `ActiveIdle`. Foreground tooling separately calls an all-no-work pass quiescent, including a pass with no actors. That label is false under the canonical lifecycle. Quiescence requires complete required participation, no eligible work, owner checkpoints, complete waits, and viable wakes. Missing participants or broken wake paths mean stalled or incomplete, not quiet.

## Scheduling Must Not Become Semantics

The supervisor stores handles in a sorted map and steps every active actor once in runtime-id order. The order is deterministic but has no relationship to the participant dependency plan or data visibility. Execution actors may run before world-model actors in a pass. Within world-model, a consumer may run before the producer that would make it eligible.

That is acceptable only because all meaningful handoffs are supposed to be durable. The consumer sees no eligible work, records a truthful wait, and advances on a later pass after the required owner milestone is visible.

Actor tick order must remain an operational choice. Optimizing the order may reduce latency. Using it as causal correctness would recreate the disconnected flywheel under a more fragile name.

The participant dependency graph has a narrower role. It can prove preparation order, physical readiness, and reverse drain order. It cannot encode the semantic sequence from observation through belief, planning, and execution. That sequence belongs to owner data and Plan dependencies.

## Recovery, Fencing, And Retirement

Domain-local restart is one of the current architecture's strengths. Activation-local restart is absent.

The supervisor can recover expired operational leases and rebuild handles. It does not create a new participant incarnation under the same activation generation, close admission during reconstruction, reconcile the prior owner's checkpoint, or require owner recovery readiness before work resumes.

Graceful shutdown is also operational. It asks each handle to stop, checks a local safe-for-flush flag after the started flag clears, flushes stores, releases leases, and closes the process instance. The configured grace period does not govern a semantic drain. Shutdown order does not follow reverse participant dependencies.

No activation admission fence precedes that shutdown. No owner reports its last accepted work boundary. No unresolved external effect summary is collected. No passive source subscription is fenced. No aggregate fenced-quiescence receipt proves that the generation can retire without losing why work exists.

The target lifecycle should preserve the current supervisor role and add structural aggregation around owner products:

```text
close generation admission
-> stop accepting new owner work
-> drain already accepted obligations
-> collect owner safe-point and unresolved-operation receipts
-> fence passive delivery
-> stop participants in reverse structural dependency order
-> release leases
-> publish retired generation
```

Execution continues to own executable operations and uncertain effects. Curation owns Epistemic Operation state and publication. Product sources own their observation cursors. Root lifecycle correlates their receipts and refuses retirement when the structural closure is incomplete. It does not reinterpret the receipts.

## PDS And World Model Reconciliation Lifecycle

This assessment reinforces the corrected PDS framing.

PDS productizes Meld. World Model Reconciliation operates the product. Runtime lifecycle realizes and maintains the connection between the two.

PDS supplies exact semantic packages, product declaration lineage, assignment, activation inputs, expected Agent topology, and structural participant requirements. Native owners install and operate their portions. The activation generation names one physically realized, current instance of that configured product.

PDS leaves the live cognitive path after realization. It does not participate in every Event, Belief revision, Plan transition, Epistemic Operation, Task, or Agent judgment. Runtime lifecycle likewise does not become a PDS interpreter. It consumes exact structural identities and owner receipts.

This produces a clean lifecycle sequence:

```text
PDS declares
-> owners install
-> initialization materializes Agent state
-> activation realizes participants
-> owners prove readiness
-> root publishes current
-> World Model Reconciliation operates
-> root fences and aggregates retirement
```

## Recursive Domain Assessment

The three specialist reviews regenerated the current root and crate domain universes, classified all domains before decomposition, froze their affected sets, and then traced the affected entities one level.

The complete root universe is recorded in the [initialization review](reviews/initialization_and_genesis_review.md#pass-one-domain-sweep), [runtime lifecycle review](reviews/runtime_lifecycle_and_supervision_review.md#pass-one-domain-sweep), and [producer and consumer review](reviews/producer_consumer_connectivity_review.md#pass-one-domain-sweep). Together they preserve explicit non-integration findings for legacy Agent, Workflow, control, compatibility, language, logging, session, generic storage, and other domains that must not gain lifecycle authority.

The recursively synthesized concern map is:

| Owning area | Affected entities | Lifecycle relationship | Assessment posture |
| --- | --- | --- | --- |
| PDS theory and config | package receipt, owner revisions, assignment, activation, prepared closure, participant plan | supply exact inert product and structural inputs | extend connection, preserve semantic ownership |
| root initialization | selected stages, Agent genesis, subscription creation, genesis Event | produce one reconstructable semantic initialization account | behavior change |
| root runtime composition | registration set, actor factories, binding slots, startup activation, lifecycle service | realize the exact participant plan and bind one generation | major connective change |
| root supervisor | instances, leases, bounded steps, reports, health, restart, shutdown | retain operational ownership and correlate participant incarnations | extend existing without semantic interpretation |
| runtime lifecycle | intents, generations, incarnations, waits, wakes, admission, quiescence, retirement | become one production structural authority instead of a parallel prototype | major connective change |
| workspace and other product sources | scan operation, watch subscription, observation Events, source cursor | join participant closure as durable observation producers | product-owned extension |
| Events | append receipt, replay sequence, consumer cursor, watermark | carry exact records and expose durable visibility positions | reuse unchanged |
| Traversal | reducer cursor, promoted occurrence, source cut, graph query | publish graph visibility and future Curation admission | extend existing narrowly |
| Belief | Evidence cursor, dirty key, assessment checkpoint, revision | publish readiness, wait, wake, and graph-driven eligibility | extend existing |
| Agent | Agent record, subscription, decision, future active Plan progression | own situated authority, progress, reconciliation, and lifecycle receipts | major World Model Reconciliation change |
| Strategy | immutable problem, candidate, future Plan revision | remain pure construction and reconstruction | major World Model Reconciliation change, no actor lifecycle |
| future Curation | standing work, accepted EpiOp, operation state, terminal result, publication outbox | become a bounded durable participant with Event publication | new world-model behavior |
| Capability and provider | catalog closure, selected invoker, external attempt lineage | publish generation-bound availability and execution-owned result facts | narrow readiness and realization seam |
| Execution | Goal Set receipt, planning cursor, Task Network revision, claims, outboxes, waits | consume complete Tasks and publish owner lifecycle receipts | reuse semantics, narrow lifecycle participation |
| harness, CLI, serve, telemetry | manifests, status accounts, eligibility walk, user projection | observe exact lifecycle truth | adapter and evidence only |
| `meld-lang` | current nouns, verbs, and pure helpers | no runtime lifecycle relationship | reuse unchanged |

Runtime participation is much broader than behavior change. Behavior change is much broader than likely core write scope. Those sets must remain separate in later requirements.

## Frozen Affected Scope

The combined operating path includes PDS config and theory, initialization, root runtime, workspace and other product observation sources, Events, Traversal, Belief, Agent, Strategy, future Curation, Capability and provider realization, Execution, and observational adapters.

The behavior proven incomplete is narrower:

```text
one aggregate initialization lineage
one generation-bound participant realization
owner-issued readiness, wait, wake, safe-point, and stop products
durable observation participation
graph-driven downstream eligibility
future Curation intake and recovery
future Agent Plan progression
truthful activation liveness
fenced drain and retirement
```

The likely architectural write concentration is root `init`, root `runtime`, theory activation, world-model Agent, Belief, Traversal, Strategy, and future Curation. Workspace requires a specific durable source-participation seam. Harness and runtime presentation need adapter changes so they report the new truth rather than infer it.

Events and `meld-lang` require no semantic change. Execution requires no epistemic change and no new Strategy guard. Its only proven lifecycle relationship is to publish its own readiness, waits, safe points, and unresolved-operation state, plus removal or containment of the process-memory package handoff when it is the sole explanation of durable work. Capability and provider impact should remain limited to exact realization readiness and Execution-owned operation lineage.

Docs and Dependency Security do not need lifecycle logic merely because their products participate. Their role is to publish owner-shaped theory and observations. The root participant plan binds the corresponding runtime source or Capability adapter without moving product semantics into lifecycle.

Legacy Workflow, root legacy Agent, control orchestration, command session lifetime, logging, views, generic storage, prompt context, and language grammar are explicit non-integration areas.

## Architecture Recommendation

Adopt activation-local lifecycle closure as a required framing for World Model Reconciliation requirements.

Do not build another central flywheel actor. Connect the existing actors through one exact activation generation and owner-issued lifecycle products. The current generation publication becomes the admission boundary. Owner checkpoints and structural wake references become the basis of liveness. Fencing and owner safe points become the basis of replacement and retirement.

Converge the two disconnected activation authorities conceptually into one production lifecycle. The evidence does not yet choose the final type or store, but it rejects two assignment heads and an unrelated registration set as simultaneous authorities.

Treat the exact participant plan as the structural source for runtime realization. A registration may be its realized root projection, but the two need a stable identity and parity proof. Registration must not be independently inferred from a fixed catalog when the activated product declares a different topology.

Keep owner contracts one-way. Producers prove their own semantic products. Consumers verify transport shape and their local state. Root lifecycle verifies identity, completeness, generation, and wake resolvability. It does not add semantic guards around foreign products.

Make every runtime connection satisfy the lifecycle edge account before calling the product ready. This standard should be applied to the sixteen concrete handoffs in the [producer and consumer connectivity review](reviews/producer_consumer_connectivity_review.md#complete-reconciliation-handoff-trace) when canonical requirements are drafted.

Make restart and replacement first-class acceptance evidence. A successful fresh boot is insufficient. The proof must survive process loss after each durable handoff, reconstruct the same generation without actor-order assumptions, reject or classify late results, and retire only after fenced closure.

## Requirements Evidence Produced

This assessment supports later requirements in five areas without drafting them now.

The first is initialization closure. Requirements can now demand exact lineage from PDS receipt through Agent genesis, prepared activation, owner readiness, and current publication.

The second is participant realization. Requirements can demand parity between the exact participant plan, runtime registration, supervisor lease, and participant incarnation.

The third is connection completeness. Requirements can demand a producer commit, consumer position, visibility milestone, wait, wake, generation fence, restart source, and retirement behavior for every reconciliation edge.

The fourth is truthful liveness. Requirements can distinguish active idle, quiescent, stalled, interrupted, fenced, and retired without interpreting owner semantics.

The fifth is lifecycle evidence. Requirements can demand harness proofs for fresh activation, lagged consumers, missed wake, crash between handoffs, interrupted external effects, generation replacement, late delivery, and fenced retirement.

## Unresolved Design Edges

The exact storage and public contract for the aggregate lifecycle authority remain open. Evidence establishes the role and rejects the current split authority. It does not select one of the existing record families unchanged.

The exact Agent topology declared by PDS remains open. That decision controls participant cardinality and whether some participants are shared across Agents.

The first readiness milestone for an Agent remains open. Genesis Event append, Traversal catch-up, Belief readability, planner projection, and Agent acceptance are distinct candidates. The required milestone should follow the first work the activated product promises.

Standing Curation may consume an Event cursor, graph revision cursor, durable work index, or a combination. Current evidence proves the lifecycle properties it needs but not the optimal intake contract.

The mechanism that makes a configured Belief eligible after a relevant graph change remains open. Traversal publication, Event mapping, and a Belief-owned dependency index preserve different boundaries.

Long-running provider work needs an exact relationship among Execution operation identity, activation generation, attempt incarnation, external idempotency, late-result classification, and retirement. Execution must own the semantic answer while lifecycle aggregates its receipt.

## Evidence And Review Quality

The [initialization and genesis review](reviews/initialization_and_genesis_review.md) performed strongly. It followed exact identity from package materialization through Agent genesis and both activation stores. Its most useful correction was showing that the harness ordering can report successful initialization while retaining factories bound before initialization. Confidence is high on current behavior and medium-high on the affected scope because future Agent topology remains open.

The [runtime lifecycle and supervision review](reviews/runtime_lifecycle_and_supervision_review.md) performed strongly. It separated operational process supervision from activation-wide semantic liveness, verified the deterministic but non-causal tick order, and proved that current shutdown and restart do not fence or reconstruct a generation. Its focused lifecycle, assembly, and supervisor suites passed ninety tests. Confidence is high.

The [producer and consumer connectivity review](reviews/producer_consumer_connectivity_review.md) performed strongly. It traced sixteen handoffs through initialization and the complete reconciliation loop, keeping every visibility milestone independent. Its most important additions were the unsupervised workspace source, graph-to-Belief invalidation gap, disconnected wait-to-wake path, and process-memory execution handoff. Confidence is high on implemented paths and intentionally medium where Curation and heterogeneous Plan contracts do not exist yet.

Together, the reviews answer the lifecycle question with consistent evidence from three directions. Current components are mostly sound. Their complete activation-local composition is not yet implemented.

## Assessment Boundary

This work produced discovery evidence and an architecture framing for later canonical requirements. It did not authorize contracts, migration, implementation sequencing, or code changes.

The recommendation is sharp enough to guide the next specification phase: World Model Reconciliation must be designed as one generation-bound, restart-safe network of owner-controlled durable transitions. Anything less risks another set of good runtimes that never become one product.
