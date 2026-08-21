# Runtime Lifecycle And Quiescence

Scope: canonical lifecycle, quiescence, liveness, interruption, recovery, and wake semantics across the Meld runtime

## Thesis

Meld is quiescent when no work is currently eligible and every standing responsibility retains a complete durable path to future eligibility. Quiescence is therefore a property of a live program, not a synonym for process silence, an empty queue, a successful heartbeat, or shutdown.

A runtime is braindead when its operational machinery remains present but a standing responsibility has lost that path. Leases and heartbeats establish operational ownership. Domain records, checkpoints, waiting conditions, wake paths, and durable operations establish program liveness.

The runtime projects lifecycle from those owner truths. It does not replace them with one central state machine.

## Lifecycle Layers

Meld keeps four lifecycle layers distinct.

### Program Lifetime

The program is the durable set of responsibilities, selected meaning, authority, subscriptions, domain progress, Goals, execution state, and admitted evidence that explains why Meld acts.

Program lifetime survives process restart. No live process, external service, hidden queue, or adapter session is the sole source of why work exists or what remains unresolved.

### Activation Lifetime

An activation binds one standing responsibility to exact runtime meaning, authority, physical bindings, selected capabilities, and isolation requirements.

An activation generation is the assignment-wide live-authority fence for one exact realization closure. Changes to authority, bindings, selected implementation, isolation realization, or activation meaning create a new activation generation.

### Participant Lifetime

A runtime participant is one actor, operation adapter, passive source, process, client session, or other live realization required by an activation.

A participant incarnation identifies one operational ownership lifetime. Reconstructing an equivalent participant after process loss creates a new incarnation without changing activation generation. A changed activation input requires a new activation generation instead.

### Operation And Delivery Lifetime

A durable operation identifies one semantic unit of capability work across retries. An attempt identifies one dispatch through one participant incarnation.

A passive subscription identifies one admitted observation path. A passive delivery identifies one source notification under that subscription and source cursor.

Operation, attempt, subscription, and delivery identities do not substitute for activation generation or participant incarnation.

## Runtime States

### Working

A required participant advances an owner checkpoint, commits a domain product, or durably dispatches an operation.

### Active Idle

The latest bounded observation found no eligible work. Admission remains open. Active idle is operational evidence only and does not prove quiescence.

### Quiescent

All required participants are present or durably represented, no work is currently eligible, and every wait is justified by an owner-declared condition with a viable wake path. A blocked condition qualifies only when it retains a durable escalation or operator wake path.

Quiescent runtime remains available. A matching durable change wakes ordinary domain selection without process restart or manual semantic sequencing.

### Stalled

A standing durable responsibility exists but required participant closure, operation ownership, eligibility, or wake viability is broken. A stalled runtime may remain operationally healthy.

Stall is a derived liveness fact. It does not grant the supervisor authority to interpret domain meaning or restart semantic work.

### Fenced Quiescent

Admission is closed for an exact activation generation. Every accepted obligation is settled, reconciled, or durably unresolved. Each affected owner has recorded an exact safe point.

Fenced quiescence is the boundary for activation replacement, deactivation, and graceful shutdown. It does not require simultaneous writes across domain stores.

### Stopped

No participant incarnation owns live work for the activation generation. Operational leases are released and new work requires a new start or activation transition.

### Interrupted

Runtime continuity was lost without complete graceful retirement. Admission remains closed during recovery. Durable records preserve completed work, unresolved effects, stale participant lineage, and the phase that recovery must resume.

## Ownership

Each domain owns:

- its work eligibility
- its durable checkpoints and progress
- its waiting-condition vocabulary
- the domain meaning of readiness and safe point
- admission of external results into canonical domain products
- reconstruction of its state from its durable records

The world model owns:

- graph and belief consumer progress
- durable Agent decisions and Plan progression
- Strategy construction requests and results
- Curation operation intake, publication, and result progress
- waits for exact belief, graph, Curation, execution, and Goal conditions

Sensory and product domains own observation cadence, source revision, semantic admission, and publication progress for their declared products.

Execution owns:

- durable operations
- dispatch claims and attempts
- effect and idempotency lineage
- unresolved external effects
- task and Goal operational lifecycle

The event authority owns:

- canonical append and replay
- consumer cursors and watermarks
- durable wake transport for admitted changes

The runtime supervisor owns:

- process and participant ownership leases
- heartbeats and operational health
- bounded actor stepping
- operational restart and shutdown signaling

Root runtime composition owns:

- exact participant closure
- activation preparation and current-generation publication
- structural readiness and lifecycle dependency closure
- aggregation of owner lifecycle receipts
- lifecycle projection for users and operators

Root runtime never interprets domain waiting conditions, decides domain result validity, or owns foreign progress cursors.

## Expected Participation

Every activation declares an exact participant plan before it becomes live. The plan identifies:

- participant identity and owner domain
- required or optional status
- actor, operation-adapter, or passive-source mode
- operational readiness dependencies
- owner readiness, wake, safe-point, and stop contract references
- selected runtime realization

The plan is structural. It contains no foreign domain state and no semantic sequence of Steward behavior.

Current participant observations are compared with the exact plan. A missing required participant is never projected as quiescence.

## Waiting And Wake

An idle domain participant declares what durable condition would make work eligible. The declaration remains in the owner's vocabulary and names a structural wake reference.

A wake reference identifies an observable boundary such as:

- an event ledger position advancing
- an owner record revision advancing
- a durable deadline arriving
- an admitted passive subscription delivering
- explicit operator action

Domains publish read-only resolution for their waiting conditions. Root verifies structural wake viability without interpreting the condition's domain meaning.

Absence of eligible work without an owner wait declaration is not sufficient for quiescence. A wait whose required wake owner or subscription is absent is stalled rather than quiescent.

## Durable External Work

Long-running or externally completed work never occupies one supervisor step until completion.

The bounded dispatch transition:

1. persists the durable operation
2. records one attempt and participant incarnation
3. hands work to the runtime adapter
4. returns a bounded report

Completion arrives through a later bounded poll or admitted passive delivery. The destination domain validates activation, participant, operation, attempt, source, scope, completeness, and evidence lineage before constructing a canonical product.

Short synchronous capabilities may complete within one bounded step. Both shapes use the same authority and result-admission boundaries.

## Activation Protocol

Activation follows one structural protocol:

```text
intend
prepare
realize
publish
work or wait
fence
drain
retire
```

### Intend

Persist one idempotent lifecycle intent against an exact standing responsibility and expected prior activation generation.

### Prepare

Owner domains create idempotent preparation records. Aggregate prepared closure proves complete inert preparation but grants no live authority.

### Realize

Create participant incarnations with admission closed. Acquire operational ownership and reconstruct domain state.

### Publish

Collect exact readiness receipts and conditionally publish one activation generation as current against the expected prior generation. Current publication is the live-authority boundary.

### Work Or Wait

Participants perform bounded domain work, dispatch durable operations, admit passive deliveries, or report owner-declared waits. Quiescence is derived from complete participation and viable wake paths.

### Fence

Close new admission for the exact current activation generation. Existing operations and deliveries retain their original lineage.

### Drain

Continue only already accepted obligations. Each owner settles, reconciles, or durably records uncertainty and then emits its safe-point receipt.

### Retire

Consume the aggregate fenced-quiescence receipt, stop participant incarnations in reverse operational dependency order, release leases, and record the activation generation as retired.

Operational dependencies express readiness and drain order only. Steward cooperation remains event-mediated cognition and never becomes lifecycle choreography.

## Fenced Quiescence Receipt

Fenced quiescence is an aggregate over exact owner products:

```text
admission fence receipt
+ owner safe-point receipts
+ execution unresolved-operation summary
+ passive-subscription fence receipts
+ participant drain receipts
→ aggregate quiescence receipt
```

Each owner receipt identifies its own durable checkpoint and lineage. The aggregate proves closure of the declared participant set. It does not claim a cross-store transaction or simultaneous physical persistence.

Retirement then combines the aggregate quiescence receipt with participant stop receipts and lease-release receipts. Fenced quiescence proves that live ownership can stop safely. Retirement proves that it did stop.

A remote callback may still arrive after retirement. Its retired activation generation and participant incarnation remain visible, and owner admission policy determines whether it is rejected, historical only, or otherwise admissible without relabeling it current.

## Recovery

Recovery replays the ordinary activation protocol from durable records. It is not a second reconciliation authority.

A fresh process reconstructs:

- exact activation intent and current-generation pointer
- prepared closure and expected participant plan
- owner checkpoints, outboxes, subscriptions, and safe points
- durable operations, attempts, claims, and unresolved effects
- participant incarnations and operational leases
- the incomplete lifecycle phase

Admission remains closed until required participants have reconstructed their owner state, reconciled prior incarnation lineage, and produced exact recovery readiness.

The runtime resumes the interrupted phase idempotently. External operational state may assist reconciliation, but it cannot be the sole source of program intent, progress, or uncertainty.

## Liveness Projection

The lifecycle projection compares durable owner truths. It may report working, active idle, quiescent, stalled, fenced quiescent, stopped, or interrupted.

The projection is read-only. It does not become authority for domain progress, automatic retry, Goal lifecycle, or result admission.

Operational health and program liveness remain separate. Restart policy responds to operational failure. Semantic stalls enter the normal event, belief, Agent, and execution loop as owner-admitted facts when action is warranted.

## PDS Relationship

PDS package attachment installs inert domain meaning and creates no runtime lifecycle.

A PDS assignment names one standing use of exact package meaning under a principal, scope, and authority context. Activation binds that assignment to physical resources and runtime participants.

The PDS router resolves exact owner components and leaves the live path. It does not supervise participants, interpret waiting conditions, sequence Agent reconciliation, or admit domain results.

One stable activation-lifecycle service coordinates assignment-local activation generations beneath the role-based root supervisor. The service remains a structural control plane. Domain actors, execution, and events remain the cognitive data plane.

## Architectural Rejections

Reject lifecycle designs that:

- equate a clean zero-work tick with quiescence
- treat heartbeat success as program liveness
- make process memory the sole home of pending work
- hold a supervisor step open for long-running external completion
- use one identity for activation generation, participant incarnation, and operation attempt
- reopen admission before recovery reconciliation
- require one simultaneous transaction across domain stores
- make root interpret domain wait or safe-point meaning
- let a remote callback bypass owner result admission
- turn operational dependencies into a manual Steward workflow
- create a second cognitive runtime inside PDS lifecycle management

## Read With

- [Cognitive Architecture](README.md)
- [Persistent Domain Stewardship](persistent_domain_stewardship.md)
- [Multi-Domain Event Ledger](events/multi_domain_spine.md)
- [Agent Runtime Surface](world_model/agent/runtime_surface.md)
- [Task Network](execution/task_network.md)
- [Observation Wait Semantics](execution/planning/observation_wait_semantics.md)
