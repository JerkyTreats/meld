# Entity First Impact Assessment For Meld Execution And Meld Events

Date: 2026-08-20

Status: evidence assessment for later canonical requirements

## Concern And Scope

This report assesses the current-code impact of replacing the one-candidate Strategy model with a world-model-owned causal Plan over Goal obligations, complete executable products, and bounded Epistemic Operations. The concern reaches Execution only where Agent submits an eligible complete executable product through the Goal Set seam. It reaches Events because Curation is expected to publish durable lifecycle facts and shared epistemic results through the canonical Event ledger.

The assessment begins with current public semantic entities, durable records, actor inputs and outputs, and owned state transitions. It then synthesizes those findings into their owning domains and crates. Proposed architecture is not treated as implemented evidence.

Execution is explicitly outside epistemic planning and Curation. It must remain ignorant of Epistemic Operations, Traversal, Belief, and Strategy reasoning. Events remains a durable carrier and replay authority. It does not acquire Curation grammar or epistemic judgment.

The direct question for Execution is narrow: can its existing Goal Set and lowering path accept each eligible complete executable product without receiving the heterogeneous Plan or re-proving world-model reasoning. The direct question for Events is narrower still: can existing append, replay, provenance, graph attachments, cursors, and idempotency carry Curation-owned products without semantic changes to the Events crate.

Out of scope are canonical requirements, implementation sequencing, migrations, acceptance criteria, Curation contract design, Traversal admission design, and removal of legacy execution workflows.

## Evidence Basis

The governing scope is the [recursive assessment charter](../assessment_charter.md). The architecture proposition is [Strategy Is Bigger Than A Task](../../strategy_plan_redesign.md). Current behavior is grounded by the [neutral code ground map](../../current_code_groundmap.md), the [Goal language review](../../reviews/goal_language_strategy_plan_review.md), the [Event-backed operation review](../../reviews/event_backed_epistemic_operations_review.md), and the [Traversal and Curation assessment](../../curation_assessment.md).

Direct evidence comes from the current crate surfaces, contracts, stores, actors, lowering, Task Network, event publication, append authority, replay, durable cursors, and their tests. Evidence links throughout this report point to the current repository rather than restating proposal text as code fact.

## Regenerated Domain Snapshots

The Execution snapshot was regenerated with:

```sh
find crates/meld-execution/src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

It returned:

```text
authority
capability
error
execution
generation
goals
lib
planning
publish
task
task_network
traversal
waiting
workflow
```

The Events snapshot was regenerated with:

```sh
find crates/meld-events/src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

It returned:

```text
error
events
lib
```

The module ownership descriptions are published by the [Execution crate surface](../../../../../../crates/meld-execution/src/lib.rs) and [Events crate surface](../../../../../../crates/meld-events/src/lib.rs).

## Meld Execution Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `authority` | `own` | Revalidates retained execution authority against an exact policy and executable Composition | `complete` for the retained execution concern | [authority contracts](../../../../../../crates/meld-execution/src/authority.rs) | not applicable | Assess whether the reduced executable seam can reuse the same decision unchanged |
| `capability` | `own` | Owns executable Capability contracts, bound instances, catalog identity, invocation, and runtime validation | `complete` | [Capability surface](../../../../../../crates/meld-execution/src/capability.rs) | not applicable | none |
| `error` | `none` | Supplies generic execution invariant errors | `not needed` | [execution errors](../../../../../../crates/meld-execution/src/error.rs) | No new error meaning is proven by the architecture proposition | none |
| `execution` | `consume` | Supplies ports used after a compiled Task reaches execution realization | `complete` | [execution ports](../../../../../../crates/meld-execution/src/execution.rs) | not applicable | none |
| `generation` | `none` | Carries provider generation DTOs for particular executable Capabilities | `not needed` | [generation contracts](../../../../../../crates/meld-execution/src/generation.rs) | Heterogeneous Strategy planning does not change provider generation semantics | none |
| `goals` | `own` | Accepts one Goal with one optional Strategy authorization and persists one authorization copy on the Goal record | `partial` | [Goal contracts](../../../../../../crates/meld-execution/src/goals/contracts.rs), [Goal acceptance](../../../../../../crates/meld-execution/src/goals/api.rs), [Goal store](../../../../../../crates/meld-execution/src/goals/store.rs) | not applicable | Resolve executable product cardinality and identity at the seam |
| `lib` | `adapter` | Re-exports Goal, planning, lowering, Task Network, and execution contracts | `partial` | [Execution crate surface](../../../../../../crates/meld-execution/src/lib.rs) | not applicable | Reflect only any proven seam contract adaptation |
| `planning` | `own` | Reads active Goal records, projects world state, either performs legacy Method planning or revalidates one authorized Composition, then lowers it | `partial` | [planning actor](../../../../../../crates/meld-execution/src/planning/runtime.rs), [Composition lowering](../../../../../../crates/meld-execution/src/planning/lowering.rs) | not applicable | Separate executable intake and lowering from world-model semantic revalidation |
| `publish` | `none` | Carries frame-head templates for workflow expansion | `not needed` | [publish templates](../../../../../../crates/meld-execution/src/publish.rs) | The proposed seam does not use frame-head publication templates | none |
| `task` | `consume` | Compiles and executes Task-local Capability graphs and persists Task artifacts and progress | `complete` | [Task surface](../../../../../../crates/meld-execution/src/task.rs), [Task contracts](../../../../../../crates/meld-execution/src/task/contracts.rs) | not applicable | none |
| `task_network` | `own` | Owns unified executable graph state, mutation, command acceptance, readiness, dispatch, outcome recording, and Event publication | `complete` | [Task Network surface](../../../../../../crates/meld-execution/src/task_network.rs), [network state](../../../../../../crates/meld-execution/src/task_network/state.rs) | not applicable | Confirm that product identity can map onto existing Composition lineage without semantic expansion |
| `traversal` | `none` | Carries workflow-backed Task package traversal expansion DTOs | `not needed` | [execution traversal DTOs](../../../../../../crates/meld-execution/src/traversal.rs) | This is executable package expansion, not world-model knowledge Traversal | none |
| `waiting` | `none` | Describes why bounded execution actors are quiet | `not needed` | [waiting declaration](../../../../../../crates/meld-execution/src/waiting.rs) | No new waiting meaning is proven at the executable seam | none |
| `workflow` | `none` | Owns the explicitly legacy profile-driven workflow system | `not needed` | [workflow surface](../../../../../../crates/meld-execution/src/workflow.rs) | The Strategy Plan proposition does not route through the legacy workflow architecture | none |

## Frozen Meld Execution Affected Set

The affected domain set is frozen as:

```text
authority
capability
execution
goals
lib
planning
task
task_network
```

This set includes domains that are necessary on the runtime path and reused unchanged. It does not imply that every listed domain requires writes.

## Meld Execution Entity Assessment

### Authority Entities

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `AuthorityPolicyRevisionRef` and `AuthorityPolicyRevision` | Execution authority | Retain exact policy content identity and installation sequence | Continue identifying the execution policy used for executable work | `reuse unchanged` | Plan authority could be confused with executable authority | [authority revisions](../../../../../../crates/meld-execution/src/authority.rs) |
| `AuthorityPolicyRegistryStore` | Execution authority | Stores exact policy revisions append-only | Continue resolving execution-owned policy revisions | `reuse unchanged` | none beyond existing storage integrity | [authority store](../../../../../../crates/meld-execution/src/authority.rs) |
| `revalidate_authority` | Execution authority | Recomputes authority over a retained Composition, subject, and exact active policy | Protect execution-owned authorization at the Task seam without interpreting the heterogeneous Plan | `reuse unchanged` | Revalidation becomes misplaced if it starts judging Strategy decomposition rather than executable actions | [Composition authority check](../../../../../../crates/meld-execution/src/authority.rs#L139) |
| `revalidate_action_authority` | Execution authority | Rechecks each Task action against retained effective authority | Continue fencing dispatched actions | `reuse unchanged` | none for the proposed split | [action authority check](../../../../../../crates/meld-execution/src/authority.rs#L170) |

Authority already owns the exact consumer-side protection that remains legitimate under the producer-boundary policy. It validates authority over executable actions and protects execution state. It does not need Plan, Curation, or epistemic semantics.

### Capability Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `CapabilityTypeContract` and its scope, binding, input, output, effect, and execution contracts | Execution Capability | Describe one executable Capability type and its mechanical invocation contract | Remain the executable vocabulary that a complete Task references | `reuse unchanged` | Adding Epistemic Operation grammar here would violate ownership | [Capability contracts](../../../../../../crates/meld-execution/src/capability/contracts.rs) |
| `BoundCapabilityInstance`, `BoundInputWiring`, and binding values | Execution Capability | Close one concrete executable Capability instance | Continue forming the Task-local executable graph | `reuse unchanged` | none for a Task-only seam | [bound instances](../../../../../../crates/meld-execution/src/capability/contracts.rs#L272) |
| `CapabilityCatalog` and exact contract revision store | Execution Capability | Resolve live executable contracts and exact content identities | Continue protecting live executable availability and identity | `reuse unchanged` | Execution must not reinterpret why Strategy selected the contract | [catalog](../../../../../../crates/meld-execution/src/capability/catalog.rs), [registry](../../../../../../crates/meld-execution/src/capability/registry.rs) |
| `CapabilityInvocationPayload`, `CapabilityExecutionContext`, and `CapabilityInvocationResult` | Execution Capability | Carry validated invocation inputs, context, lineage, and terminal result | Continue realizing compiled Task work | `reuse unchanged` | none | [runtime payload](../../../../../../crates/meld-execution/src/capability/runtime.rs), [invocation boundary](../../../../../../crates/meld-execution/src/capability/invocation.rs) |

No Capability entity needs to know whether a Task came from a mixed Strategy Plan. Epistemic operations must not be represented as executable Capabilities merely to reuse this runtime.

### Execution Port Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| provider and workspace execution ports | Execution boundary | Supply external effects required by Task and workflow runtimes | Continue serving executable Tasks only | `reuse unchanged` | Epistemic Curation must not enter through a workspace or provider execution adapter | [execution ports](../../../../../../crates/meld-execution/src/execution/ports.rs) |
| `EventPublicationPort` and `ExecutionEventContext` | Execution boundary | Publish execution lifecycle facts through an injected Event adapter | Continue publishing execution-owned facts | `reuse unchanged` | Execution events must not claim epistemic settlement | [event publication port](../../../../../../crates/meld-execution/src/execution/ports.rs) |
| `WorldModelQueryPort` and belief context ports | Execution boundary | Supply current context to legacy workflow and Task paths | No relationship to mixed Plan interpretation | `reuse unchanged` | Treating these ports as permission to re-plan would violate the seam | [world-model ports](../../../../../../crates/meld-execution/src/execution/ports.rs) |

These ports are downstream realization dependencies. Their presence on the runtime path does not establish a behavior change.

### Goal Set Entities

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `GoalAcceptanceRequest` | Execution Goal Set | Accepts one ground Goal, lifecycle policy, metadata, and one optional executable Strategy authorization | Accept only an eligible complete executable Goal and Task product, never the mixed Plan | `extend existing` | One root Goal may enable more than one executable product across reconciliation revisions | [acceptance request](../../../../../../crates/meld-execution/src/goals/api.rs#L21) |
| `ExecutionStrategyAuthorization` | Execution Goal Set | Copies Agent decision identity, Strategy candidate identity, planner snapshot, Composition, bindings, Capability identities, Method lineage, and authority decision | Retain only information needed to identify, validate, and realize the executable product | `extend existing` | Current shape exposes Strategy and planner concepts that Execution does not otherwise own | [authorization copy](../../../../../../crates/meld-execution/src/goals/contracts.rs#L35) |
| `ExecutionGoalRecord` | Execution Goal Set | Stores one Goal and one optional Strategy authorization | Represent current executable intake without becoming storage for the Strategy Plan | `extend existing` | One authorization slot makes executable product cardinality part of Goal identity | [Goal record](../../../../../../crates/meld-execution/src/goals/contracts.rs#L7) |
| `GoalCommandMetadata` and add command | Execution Goal Set | Supply command idempotency, source dedupe, and ordering for Goal insertion | Continue protecting Goal Set command state | `reuse unchanged` | A Plan revision must not accidentally collide with an earlier executable product source identity | [Goal commands](../../../../../../crates/meld-execution/src/goals/contracts.rs#L67) |
| modify command and store transition | Execution Goal Set | Replaces Goal content while deliberately preserving the existing Strategy authorization | Carry reconciled executable work only if the seam defines a distinct executable product update | `extend existing` | A newly reconciled Task cannot replace a stale authorization through current modification | [in-memory modification](../../../../../../crates/meld-execution/src/goals/store.rs#L75), [durable modification](../../../../../../crates/meld-execution/src/goals/persistent_store.rs#L170) |
| satisfy, reopen, suspend, resume, and abandon commands | Execution Goal Set | Own lifecycle transitions, epochs, and stale no-op behavior for its Goal copy | Continue managing execution-facing Goal lifecycle | `reuse unchanged` | Root Agent Goal and execution-facing Goal lifecycle may diverge unless attribution stays explicit | [lifecycle contracts](../../../../../../crates/meld-execution/src/goals/contracts.rs#L107), [store transitions](../../../../../../crates/meld-execution/src/goals/store.rs) |
| `GoalSetApi` and `GoalSetCommandStore` | Execution Goal Set | Validate transport shape and apply commands across memory and durable stores | Remain the public curation seam for executable intake | `extend existing` | Validation currently names Strategy and Composition rather than a neutral executable product | [Goal Set API](../../../../../../crates/meld-execution/src/goals/api.rs#L57) |
| `ActiveGoalQuery` and `GoalSetQuery` | Execution Goal Set | Expose active Goal records to the planning actor | Expose eligible executable records without world-model Plan state | `extend existing` | Active root Goal does not itself prove that a new Task slice is eligible | [active Goal query](../../../../../../crates/meld-execution/src/goals/query.rs) |
| `GoalCommandOutcome` and stale reasons | Execution Goal Set | Return deterministic applied, duplicate, missing, and stale outcomes | Continue reporting execution-owned state transitions | `reuse unchanged` | none | [command outcomes](../../../../../../crates/meld-execution/src/goals/contracts.rs#L160) |

The Goal Set is the one proven Execution seam under pressure. Current storage binds one executable authorization to one Goal record. A second executable product for the same root Goal is not representable through acceptance alone, and ordinary Goal modification preserves the old authorization. This is an implemented cardinality constraint, not a theoretical concern. It proves a Goal Set adaptation only if the eventual contract permits several executable products to share one execution-facing Goal identity. A contract that gives each product a distinct execution-facing Goal could reuse the present one-to-one storage shape.

Current tests confirm that guarded acceptance retains the authorization exactly. They do not establish repeated executable product intake for one root Goal. See [Goal acceptance tests](../../../../../../crates/meld-execution/tests/goals.rs#L58).

### Planning And Lowering Entities

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `PlanningRuntimeActor` | Execution planning | Reads active Goals, requests current world state, chooses the authorized or legacy planning path, lowers Composition, and submits Task Network mutations | Become or expose the bounded intake-to-lowering behavior for complete executable products | `extend existing` | The actor currently combines world-model semantic checks with execution lowering | [planning actor](../../../../../../crates/meld-execution/src/planning/runtime.rs#L247) |
| `PlanningProjectionPort` and `PlanningWorldStateProjection` | Execution planning boundary | Request a fresh Goal-scoped world-state projection before either planning path | Remain legacy input or be absent from the canonical complete-Task path | `not needed` for the canonical seam | Execution re-proves producer premises and becomes coupled to planner snapshot visibility | [projection port](../../../../../../crates/meld-execution/src/planning/runtime.rs#L60) |
| `PlanningWorldStateRequest` and `PlanningWorldStateFrameRef` | Execution planning boundary | Bind a Goal, perspective, branch, requested dimensions, and exact projection provenance | Remain provenance on the legacy planning path, not a prerequisite for accepting a complete executable product | `not needed` for the canonical seam | Planner frame identity can become a consumer guard over producer reasoning | [world-state contracts](../../../../../../crates/meld-execution/src/planning/world_state.rs) |
| `PlanningRequest` | Execution planning | Combines active Goal, projected world state, frame, and projection request | No longer needs to be the intake shape when a complete executable product is already present | `not needed` for the canonical seam | Requiring world state makes Execution aware of why Strategy chose the Task | [planning request](../../../../../../crates/meld-execution/src/planning/contracts.rs#L26) |
| `plan_goal` | Execution planning | Performs legacy Method search over a Goal and projected world state | No relationship to the canonical Strategy-authored executable path | `not needed` | A second semantic planner conflicts with Strategy as the only planning workflow | [legacy Goal planning](../../../../../../crates/meld-execution/src/planning/runtime.rs#L838) |
| `MethodLibrary`, verified entries, and Method diagnostics | Execution planning | Load and mechanically verify legacy executable Methods before selection | No relationship to the canonical Strategy-authored executable intake | `not needed` | Keeping this path visible can obscure which domain owns semantic planning | [Method library](../../../../../../crates/meld-execution/src/planning/method_library.rs) |
| available action bindings and Method realization bindings | Execution planning | Optionally reroute a selected Method to a Task package workflow rather than Composition lowering | No new relationship to heterogeneous Strategy Plans is proven | `reuse unchanged` | A realization route must not become a route for EpiOps | [action contracts](../../../../../../crates/meld-execution/src/planning/action.rs), [realization contracts](../../../../../../crates/meld-execution/src/planning/realization.rs) |
| `plan_authorized_goal` | Execution planning | Requires exact planner-frame identity, re-evaluates operator preconditions, revalidates authority, Composition structure, live Capability resolution, and contract identities | Preserve execution-local authority, structure, and live Capability checks while avoiding re-adjudication of Strategy premises | `extend existing` | Consumer-side premise evaluation violates the desired producer ownership boundary | [authorized planning](../../../../../../crates/meld-execution/src/planning/runtime.rs#L905) |
| `ExecutionComposition` | Execution planning | Carries the Goal, planner frame, Method identity, bindings, Composition, projected effects, diagnostics, and authority into lowering | Serve only as an execution-local lowering input or be adapted from the complete Task product | `extend existing` | It currently carries more world-model planning context than lowering requires | [execution composition](../../../../../../crates/meld-execution/src/planning/contracts.rs#L68) |
| lowering `Request` and `Plan` | Execution planning lowering | Convert one execution Composition into one deterministic mutation proposal with Goal and Method lineage | Continue as the Task Network lowering seam | `reuse unchanged` | The incoming executable product must map to a stable Composition identity | [lowering contracts](../../../../../../crates/meld-execution/src/planning/lowering.rs#L98) |
| `Lowerer` | Execution planning lowering | Compiles operator steps into Task nodes and maps semantic edges into executable dependencies | Continue lowering executable content only | `reuse unchanged` | EpiOps or heterogeneous Plan nodes must never reach this type | [lowerer](../../../../../../crates/meld-execution/src/planning/lowering.rs#L190) |
| lowering diagnostics | Execution planning lowering | Report missing operators, unresolved contracts, invalid data flow, and deferred non-executable language steps | Continue protecting Task Network mutation shape | `reuse unchanged` | Deferred Goal steps show that lowering is not a heterogeneous Plan executor | [lowering diagnostics](../../../../../../crates/meld-execution/src/planning/lowering.rs#L130) |
| `PlanningRuntimeActorGoalResult` and actor report | Execution planning | Report projection, planning, realization, lowering, and command outcomes | Continue reporting execution-local intake and lowering outcomes after any seam reduction | `extend existing` | Result vocabulary currently exposes legacy semantic planning stages | [actor results](../../../../../../crates/meld-execution/src/planning/runtime.rs#L131) |

The current authorized path does more than consumer shape validation. It projects world state again, requires the authorization snapshot to match the current frame, and re-evaluates every operator precondition. That behavior is an explicit ownership tension under the proposed architecture. Live Capability resolution, executable structure, and authority checks protect Execution-owned state. Re-adjudicating Strategy premises does not.

The lowerer itself already expresses the desired boundary. Its input is one executable Composition, and its output is a deterministic Task Network mutation proposal. It does not execute epistemic work or inspect Strategy theory.

### Task Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `TaskDefinition` and `TaskInitSlotSpec` | Execution Task | Define one Task-local executable Capability graph and its initialization contract | Continue representing execution-local authored work | `reuse unchanged` | Calling the heterogeneous Plan a Task would overrun this boundary | [Task definition](../../../../../../crates/meld-execution/src/task/contracts.rs#L9) |
| `CompiledTaskRecord` and dependency edges | Execution Task | Store the compiled Capability graph and derived Task-local dependencies | Continue as the unit injected into a Task Network node | `reuse unchanged` | none | [compiled Task](../../../../../../crates/meld-execution/src/task/contracts.rs#L35) |
| `TaskRunContext` and initialization payload | Execution Task | Bind run identity, trigger, and init artifacts | Continue materializing one executable run | `reuse unchanged` | none | [Task initialization](../../../../../../crates/meld-execution/src/task/init.rs) |
| `TaskCompiler` and compiler trait | Execution Task | Compile validated Task definitions against the live Capability catalog | Continue compiling each lowered executable operator | `reuse unchanged` | none | [Task compiler](../../../../../../crates/meld-execution/src/task/compiler.rs) |
| `TaskExecutor`, progress store, invocation records, and artifact repository | Execution Task | Execute, checkpoint, and persist one Task-local Capability graph | Continue realizing executable work | `reuse unchanged` | Epistemic operation state must not be added here | [Task executor](../../../../../../crates/meld-execution/src/task/executor.rs), [progress](../../../../../../crates/meld-execution/src/task/progress.rs), [artifact records](../../../../../../crates/meld-execution/src/task/contracts.rs#L72) |
| `TaskEvent` and `ExecutionTaskEventData` | Execution Task | Express Task lifecycle and artifact emission for Event publication | Continue publishing execution facts that Curation or Belief may later consume | `reuse unchanged` | Task success must not be treated as Goal satisfaction or epistemic correctness | [Task events](../../../../../../crates/meld-execution/src/task/events.rs) |
| package and expansion contracts | Execution Task | Support legacy and package-backed executable fan-out | No relationship to EpiOp execution | `reuse unchanged` | Their traversal vocabulary is operational rather than epistemic | [Task package surface](../../../../../../crates/meld-execution/src/task/package.rs), [Task expansion surface](../../../../../../crates/meld-execution/src/task/expansion.rs) |

The Task domain already has a narrow and coherent meaning: a compiled local executable graph with durable progress and artifacts. No entity here needs to absorb Strategy Plan or Curation behavior.

### Task Network Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `NetworkState` | Execution Task Network | Stores the unified executable graph, lifecycle, claims, outcomes, artifacts, and publication outbox at one revision | Continue as the single executable coordination state | `reuse unchanged` | Adding EpiOps would conflate execution and epistemic causal state | [network state](../../../../../../crates/meld-execution/src/task_network/state.rs#L27) |
| `TaskNode` and `TaskLineage` | Execution Task Network | Carry a compiled Task, initialization sources, run context, and Goal, Composition, Method, frame, operator, Capability, and authority lineage | Continue carrying executable provenance | `reuse unchanged` | Future executable product identity must map without importing the whole Plan | [Task node lineage](../../../../../../crates/meld-execution/src/task_network/state.rs#L111) |
| `DependencyEdge`, origin, and kind | Execution Task Network | Encode executable ordering, artifact flow, and deferred conditional dependencies | Continue coordinating How and When among executable Tasks | `reuse unchanged` | Strategy causal edges that cross Task and EpiOp owners must remain outside this network | [network dependencies](../../../../../../crates/meld-execution/src/task_network/state.rs#L215) |
| `Set`, `Mutation`, `Inject`, and commit contracts | Execution Task Network | Accept deterministic graph mutation proposals under revision and state-hash preconditions | Continue accepting lowerer output | `reuse unchanged` | none | [mutation contracts](../../../../../../crates/meld-execution/src/task_network/mutation.rs) |
| command `Request`, `Command`, and `Response` | Execution Task Network | Serialize single-writer mutation, claim, outcome, cancellation, and publication transitions | Continue owning Task Network writes | `reuse unchanged` | none | [command boundary](../../../../../../crates/meld-execution/src/task_network/command.rs) |
| journal records and memory and sled stores | Execution Task Network | Persist commands append-only and reduce them into deterministic network state | Continue protecting the unified executable network | `reuse unchanged` | none | [journal](../../../../../../crates/meld-execution/src/task_network/journal.rs), [stores](../../../../../../crates/meld-execution/src/task_network/store.rs) |
| `ReadySet` and readiness diagnostics | Execution Task Network | Compute executable eligibility from committed dependencies and artifacts | Continue ranking executable readiness independently of Strategy reasoning | `reuse unchanged` | It must not evaluate epistemic dependencies absent from the admitted Task slice | [readiness contracts](../../../../../../crates/meld-execution/src/task_network/state.rs#L315), [readiness behavior](../../../../../../crates/meld-execution/src/task_network/readiness.rs) |
| Task initialization materialization | Execution Task Network | Resolve static seeds and upstream artifacts into one validated Task initialization payload | Continue preparing admitted executable nodes | `reuse unchanged` | none | [initialization](../../../../../../crates/meld-execution/src/task_network/initialization.rs) |
| dispatch `Request`, `Claim`, `Outcome`, and status | Execution Task Network | Fence workers, record terminal Task outcomes, and carry artifact records | Continue executing admitted Tasks | `reuse unchanged` | Terminal Task outcome is not proof of root Goal achievement | [dispatch contracts](../../../../../../crates/meld-execution/src/task_network/dispatch.rs) |
| dispatch actor and command port | Execution Task Network | Claim ready nodes, invoke Tasks, and submit terminal outcomes through the command boundary | Continue operating over Task Network readiness only | `reuse unchanged` | It must remain unaware of Strategy Plan eligibility and EpiOp state | [dispatch actor](../../../../../../crates/meld-execution/src/task_network/dispatch_actor.rs) |
| `Publication` and `PublicationState` | Execution Task Network | Keep retryable Event publication in durable network state | Continue publishing execution-owned outcome facts | `reuse unchanged` | An append receipt proves Event durability, not downstream epistemic visibility | [publication outbox](../../../../../../crates/meld-execution/src/task_network/outcome.rs) |
| publication bridge and runtime report | Execution Task Network | Build graph-decorated execution Events, append idempotently, and mark exact receipts | Continue as the execution result handoff to Events | `reuse unchanged` | Consumers must interpret the event after their own cursor barriers | [publication bridge](../../../../../../crates/meld-execution/src/task_network/publication.rs), [publication runtime](../../../../../../crates/meld-execution/src/task_network/runtime.rs) |
| aggregate package outcome and publication | Execution Task Network | Publish a complete package-level executable result while retaining per-folder artifacts and subjects | Remain an execution-owned result product where package execution is used | `reuse unchanged` | Aggregate execution evidence must not replace Curation claim-level correctness | [aggregate outcome](../../../../../../crates/meld-execution/src/task_network/aggregate.rs), [aggregate publication](../../../../../../crates/meld-execution/src/task_network/aggregate_publication.rs) |
| package-step, Composition-step, and terminal recording contracts | Execution Task Network | Support bounded package execution, Composition-backed invocation, and durable package terminal outcomes | Continue as execution-local realization and compatibility behavior | `reuse unchanged` | None of these contracts may progress epistemic Plan nodes | [package step](../../../../../../crates/meld-execution/src/task_network/package_step.rs), [Composition step](../../../../../../crates/meld-execution/src/task_network/composition_step.rs), [terminal recording](../../../../../../crates/meld-execution/src/task_network/terminal_recording.rs) |

The Task Network already owns the exact Execution invariant described by the architecture proposition. It coordinates executable priority, ordering, parallelism, dispatch, and outcome publication. No current entity needs knowledge of the world-model Plan or its epistemic branches.

### Execution Crate Surface Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Goal Set exports | Execution crate surface | Expose Goal acceptance, records, stores, lifecycle commands, and authorization copy | Expose any narrowly adapted executable intake contract | `adapter only` | A broad re-export can make legacy Strategy vocabulary appear canonical | [Goal exports](../../../../../../crates/meld-execution/src/goals.rs) |
| planning and lowering exports | Execution crate surface | Expose both semantic execution planning and Composition lowering | Preserve the lowering seam while distinguishing legacy planning | `adapter only` | Consumers may select the legacy semantic planner accidentally | [planning exports](../../../../../../crates/meld-execution/src/planning.rs), [crate re-exports](../../../../../../crates/meld-execution/src/lib.rs) |
| Task and Task Network exports | Execution crate surface | Expose compiled Task and executable network contracts | Continue unchanged | `reuse unchanged` | none | [Task surface](../../../../../../crates/meld-execution/src/task.rs), [Task Network surface](../../../../../../crates/meld-execution/src/task_network.rs) |

## Meld Execution Domain Synthesis

### Goals

The Goal Set is the intake owner and the clearest possible Execution write boundary. It currently admits one Goal and one exact Strategy authorization, then stores that authorization in a single optional field. This works for the current one-candidate model. It does not directly express several independently enabled executable products across revisions of one world-model Plan.

That does not make a Goal Set storage change unconditional. The eventual executable-product cardinality contract decides it. Several products beneath one execution-facing Goal require a Goal Set adaptation. One distinct execution-facing Goal per product may reuse the current storage cardinality, though the root Goal attribution and lifecycle relationship would then need to be explicit at the producer seam.

The pressure is cardinality and vocabulary, not epistemic semantics. Execution still needs a Goal attribution, executable body, deterministic product identity, Capability identities, and effective authority. It does not need the heterogeneous Plan, Curation operations, belief state, Strategy theory, or epistemic dependencies.

### Planning

Planning currently contains two different behaviors. One is a legacy semantic planner that selects Methods for unauthorised active Goals. The other revalidates an Agent-authorized Strategy Composition and lowers it into Task Network mutations.

The lowerer is aligned with the proposed boundary and can remain an Execution-owned translation from a complete executable product into the unified Task Network. The authorized runtime is only partly aligned. Its live Capability, executable structure, and authority checks protect Execution. Its planner-frame equality and operator-precondition reassessment re-prove the producer's semantic decision and couple Execution back to world-model state.

### Task

Task remains a compiled Task-local Capability graph. It owns execution progress, invocations, artifacts, and Task lifecycle facts. The proposed architecture creates no evidence for adding EpiOps, Curation rules, Strategy Plan state, or Goal satisfaction logic here.

### Task Network

Task Network remains the unified operational graph for How and When. It lowers accepted executable work into nodes, dependencies, readiness, claims, dispatch, outcomes, and Event publication. Its existing Composition lineage can preserve executable source identity if the intake mapping remains stable. No Task Network semantic redesign is proven.

### Authority, Capability, And Ports

Authority and Capability remain Execution-owned mechanical truth. They protect executable action scope, live contract identity, invocation shape, and dispatch. Execution ports remain realization adapters. These domains are part of the runtime path but are not write scope for the proposed architecture.

## Meld Execution Crate Synthesis

The thesis survives contact with Execution code, with one qualified seam pressure.

Execution can remain a simple machine that accepts complete executable work, lowers it into one Task Network, and publishes outcomes. Task compilation, Task Network mutation, readiness, dispatch, artifact persistence, Event publication, Capability invocation, and execution authority are reusable without epistemic changes.

The current Goal Set and authorized planning actor still reflect the older assumption that one Agent Goal owns one Strategy candidate and one Composition. They also retain Strategy and planner vocabulary that the consumer does not need. Mixed Strategy Plans make that representation incomplete when reconciliation enables another executable product for the same root Goal.

The proven scope is therefore limited to the Goal Set intake and the authorized intake-to-lowering path. A Goal Set storage adaptation remains conditional on the chosen executable-product cardinality. The intake path's re-evaluation of world-model premises is a direct conflict with the proposed producer boundary if that path remains canonical. In both cases, the evidence supports reduction rather than stronger enforcement. Execution should protect its own Goal storage, executable structure, live Capability availability, authority, and Task Network state. It should not re-evaluate world-model premises or receive epistemic work.

No evidence supports changes to Task semantics, Task Network semantics, Capability semantics, execution ports, Task result Events, provider generation, package expansion, legacy workflow internals, or the execution-local traversal DTOs.

## Meld Events Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `error` | `none` | Owns typed storage and authority failures | `not needed` | [Events errors](../../../../../../crates/meld-events/src/error.rs) | Curation publication uses existing error categories and proves no new Events error meaning | none |
| `events` | `own` | Owns producer-neutral envelopes, graph attachments, provenance, ledger identity, append, replay, subscription, watermarks, cursors, migration, and observability | `complete` | [Events domain](../../../../../../crates/meld-events/src/events.rs), [authority](../../../../../../crates/meld-events/src/events/authority.rs) | not applicable | none |
| `lib` | `adapter` | Re-exports the complete canonical Event authority surface | `complete` | [Events crate surface](../../../../../../crates/meld-events/src/lib.rs) | not applicable | none |

## Frozen Meld Events Affected Set

The affected domain set is frozen as:

```text
events
lib
```

Both domains are reused unchanged by the proposition. No Events crate write is proven.

## Meld Events Entity Assessment

### Event Contract Entities

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `EventEnvelope` | Events | Carries producer identity, stream, type, idempotency key, object coordinates, relations, source provenance, and producer-owned payload | Carry Curation lifecycle facts and promoted Curation semantic products without interpreting them | `reuse unchanged` | Generic JSON must not become implicit Curation grammar | [Event envelope](../../../../../../crates/meld-events/src/events.rs#L153) |
| `DomainObjectRef` | Events contracts | Validates a stable domain, object kind, and object identity coordinate | Coordinate Curation-owned expected entities and assessments for Traversal | `reuse unchanged` | Coordinate shape does not carry truth, perspective, currentness, or authority | [object reference](../../../../../../crates/meld-events/src/events/contracts.rs#L30) |
| `EventRelation` | Events contracts | Validates one producer-named directed relation and its endpoints | Carry Curation-owned epistemic edges | `reuse unchanged` | Events cannot validate materiality, realization, coverage, or semantic ownership | [relation contract](../../../../../../crates/meld-events/src/events/contracts.rs#L86) |
| `EventRecord` and `EventRecordRef` | Events | Add canonical sequence and ledger identity to immutable producer records | Provide durable result identity and exact source references | `reuse unchanged` | A sequence orders occurrences but does not establish semantic currentness | [record contracts](../../../../../../crates/meld-events/src/events.rs#L121) |
| `EventProvenance` | Events | Carries canonical source record references on derived Events | Cite the frozen source facts used by Curation | `reuse unchanged` | Provenance must not be confused with semantic proof sufficiency | [provenance](../../../../../../crates/meld-events/src/events.rs#L139) |
| genesis identity helpers | Events with producer ownership | Provide idempotent identities for seeded first knowledge and rebuilt projections | Remain available for genuine seed facts, not ordinary EpiOp results | `reuse unchanged` | Misusing genesis identity could collapse distinct rule or source-cut results | [genesis constructors](../../../../../../crates/meld-events/src/events.rs#L112) |

`EventEnvelope` already has every transport feature proven necessary by the architecture proposition. The missing EpiOp vocabulary, perspective, bounds, rule revision, terminal semantics, and currentness belong in typed Curation products carried by the envelope.

### Append And Ledger Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `LedgerIdentity` and `LedgerCursor` | Events | Bind records and positions to one canonical sequence space | Continue fencing Curation publication and replay | `reuse unchanged` | Cross-ledger references are rejected and cannot be silently merged | [ledger identity](../../../../../../crates/meld-events/src/events/identity.rs), [cursor](../../../../../../crates/meld-events/src/events/authority.rs#L93) |
| `AppendMode`, `AppendDisposition`, and `AppendReceipt` | Events authority | Distinguish plain from producer-keyed idempotent append and return identity-bearing durability | Publish deterministic EpiOp lifecycle and semantic result records | `reuse unchanged` | Idempotency prevents duplicate records for one key but does not define Curation identity | [append contracts](../../../../../../crates/meld-events/src/events/authority.rs#L56) |
| `EventAppendCapability` | Events authority | Validates provenance and genesis identity, then durably appends or accepts best-effort work | Accept producer-owned Curation envelopes through the same authority | `reuse unchanged` | Events must not add Curation semantic validation | [append capability](../../../../../../crates/meld-events/src/events/authority.rs#L375) |
| `EventAuthority` | Events authority | Aggregates one writer, store, ledger identity, registry, durable cursors, and observability | Continue as the sole ledger owner | `reuse unchanged` | A separate Curation ledger or direct Traversal write would create a second system | [authority aggregate](../../../../../../crates/meld-events/src/events/authority.rs#L40) |
| `EventAuthorityContract` and durable append requests | Events remote seam | Expose transport-neutral append, replay, subscription, watermark, and observations | Carry Curation envelopes locally or remotely without contract expansion | `reuse unchanged` | Transport must not gain domain semantics | [remote authority contract](../../../../../../crates/meld-events/src/events/remote.rs#L31) |

Current tests prove that idempotent duplicate append returns the original sequence and a duplicate disposition. See [idempotent append test](../../../../../../crates/meld-events/tests/event_authority.rs#L264). That is sufficient infrastructure for deterministic producer publication, but the producer must still derive correct operation and result identities.

### Replay And Consumer Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `ReplayRequest` and `EventPage` | Events authority | Return bounded ordered pages against an identity-checked frozen coverage range | Supply Curation and downstream consumers with replayable change discovery | `reuse unchanged` | Reaching the page tip is not proof that another projection caught up | [replay contracts](../../../../../../crates/meld-events/src/events/authority.rs#L102) |
| `EventReplayCapability` | Events authority | Performs bounded replay and newest-page reads | Continue feeding bounded pull consumers | `reuse unchanged` | No Event directly invokes Curation code | [replay capability](../../../../../../crates/meld-events/src/events/authority.rs#L451) |
| `SubscriptionPollRequest` and `EventSubscriptionCapability` | Events authority | Poll replay immediately, wait on watermark, then replay once more | Remain an optional wake-up optimization over replay | `reuse unchanged` | Subscription must not replace durable cursor correctness | [subscription capability](../../../../../../crates/meld-events/src/events/authority.rs#L480) |
| `ConsumerCursorState` and `DurableConsumerCursor` | Events consumer | Persist monotonic per-consumer absorption only after consumer-owned state is durable | Allow a future Curation consumer to own its exact Event position | `reuse unchanged` | Advancing before Curation state commit would lose work | [durable cursor contract](../../../../../../crates/meld-events/src/events/consumer.rs) |
| `EventConsumerRegistryCapability` and cursor positions | Events authority | Store authoritative durable positions and mirror them for lag observation | Continue reporting Curation and Traversal lag separately | `reuse unchanged` | One consumer cursor cannot stand in for another consumer's visibility | [consumer registry capability](../../../../../../crates/meld-events/src/events/authority.rs#L529) |
| `EventWatermark` and watermark capability | Events authority | Report committed and durable tip positions and support bounded waits | Continue exposing ledger durability barriers | `reuse unchanged` | Ledger watermark is not graph, Belief, or Agent visibility | [watermark capability](../../../../../../crates/meld-events/src/events/authority.rs#L506) |

Durable cursor tests prove monotonic advancement, restart survival, identity binding, and separation between authoritative positions and observational mirrors. See [durable cursor tests](../../../../../../crates/meld-events/tests/durable_consumer_cursor.rs). These mechanics are already suitable for a Curation actor that commits domain state before advancing its own cursor.

### Observability, Compatibility, And Migration Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| health, flow, trace, lag, and session reports | Events observability | Describe bounded ledger coverage, domain flow, structural provenance, and consumer lag | Observe Curation publication and replay through existing generic dimensions | `reuse unchanged` | Structural traces do not interpret epistemic correctness | [observability contracts](../../../../../../crates/meld-events/src/events/observability.rs) |
| compatibility aliases | Events compatibility | Preserve pre-extraction progress-envelope names for old callers | No special relationship to Curation | `not needed` | Curation should use canonical Event contracts | [compatibility aliases](../../../../../../crates/meld-events/src/events/compat.rs) |
| legacy migration contracts | Events migration | Translate and cut over frozen old ledgers with explicit identity evidence | No current migration is authorized or proven | `not needed` | Architecture assessment must not manufacture a ledger migration | [migration contracts](../../../../../../crates/meld-events/src/events/migration.rs) |
| test-support contracts | Events test support | Expose low-level stores, writers, cursors, and observability only for tests and fuzzing | No runtime relationship | `not needed` | Test fixtures are not product contracts | [test support](../../../../../../crates/meld-events/src/events/test_support.rs) |

### Events Crate Surface Entities

| Entity family | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| canonical Event and authority re-exports | Events crate surface | Publish envelopes, records, graph attachments, append, replay, subscriptions, watermarks, cursors, and remote contracts | Give Curation the existing producer and consumer capabilities | `reuse unchanged` | Adding Curation-specific exports would shift grammar ownership into Events | [Events crate surface](../../../../../../crates/meld-events/src/lib.rs) |
| error re-exports | Events crate surface | Publish current storage and authority failures | Continue unchanged | `reuse unchanged` | none | [Events errors](../../../../../../crates/meld-events/src/error.rs) |

## Meld Events Domain Synthesis

The Events domain already owns the full durable carrier needed by bounded Epistemic Operations. A Curation producer can create a typed operation or result, serialize its producer-owned body into `EventEnvelope`, attach stable objects and relations, cite exact source records, derive a deterministic record id, and append idempotently. A Curation consumer can replay bounded pages, commit its own state, and advance its own durable cursor.

Events does not need to know whether a record represents expected README F, a material source claim, a coverage assessment, an accepted EpiOp request, abstention, or terminal incompleteness. Those meanings remain Curation-owned.

The existing mechanics also expose the real causal barriers. Durable append, Traversal materialization, Belief reconciliation, and Agent observation are separate positions. No change to Events can collapse those distinct owner milestones into one completion flag.

## Meld Events Crate Synthesis

No `meld-events` behavior change is proven.

The architecture proposition adds new producers and consumers of the Event spine, not a new responsibility to the spine. `EventEnvelope`, `DomainObjectRef`, `EventRelation`, source provenance, idempotent append, ordered replay, durable consumer cursors, watermarks, and generic observability are all reusable unchanged.

The likely writes belong in the future Curation producer and in world-model Traversal admission, not in `meld-events`. A second command bus, graph mutation path, or Curation-specific envelope would duplicate existing durable mechanics and violate the producer-owned semantics boundary.

## Cross-Crate Ownership Boundary

The entity pass exposes one newly sharp boundary:

```text
world-model Agent and Strategy
-> complete executable product with Goal attribution and authority
-> Execution Goal Set
-> execution-local validation and lowering
-> Task Network
-> execution result Event
-> Events ledger
-> independent world-model consumers
```

The current code calls the executable handoff an `ExecutionStrategyAuthorization`. That name and body preserve the old one-candidate architecture. The architectural boundary is more neutral: Execution receives an executable product authorized by Agent. Strategy lineage may remain provenance, but it is not the consumer contract's semantic center.

The boundary does not extend from Curation to Execution. Curation uses Events as its publication and replay spine directly through a world-model-owned runtime. Execution result Events and Curation result Events coexist in the ledger as products owned by different domains.

## Separated Scopes

### Runtime Path

The complete runtime path includes Execution authority, Capability, Goal Set, authorized intake, Composition lowering, Task compilation, Task Network mutation, readiness, dispatch, Task artifacts, execution result publication, Events append, replay, cursors, and later independent world-model consumers.

### Behavior That Must Change

Current evidence proves behavior pressure only at the Execution Goal Set and authorized intake-to-lowering boundary. The one-authorization-per-Goal record and its preservation across Goal modification cannot directly represent a later executable product enabled by reconciliation under the same execution-facing Goal identity. Whether that requires a Goal Set change depends on the eventual executable-product cardinality contract. The current authorized planning path also re-evaluates Strategy premises against a fresh world-state projection.

No behavior change is proven inside Events.

### Likely Writes

Likely Execution writes are constrained to the authorized intake and lowering seam plus public exports. Goal Set contracts, storage, and the active executable query enter likely write scope only if several executable products share one execution-facing Goal identity. A small lineage adaptation may be needed only if executable product identity cannot map onto the existing Composition identity.

No likely write is identified in Execution authority, Capability, Task, Task Network, Task result publication, provider generation, workflow, waiting, package traversal, or Events.

### Adapters

Root composition must map an Agent-authorized executable Plan slice into the Execution-owned intake shape. That adapter must not become the owner of product identity, Goal meaning, authority, or Task semantics.

The Events crate surface already exports the producer and consumer capabilities needed by a future Curation runtime. Root wiring may supply those capabilities, but no Events adapter change is proven.

### Reuse Unchanged

Execution authority policies, action revalidation, Capability contracts and invocation, Task definitions and compilation, Task execution and artifacts, Task Network state and mutations, readiness, dispatch, outcome recording, publication outbox, execution Event building, and aggregate package publication are reusable unchanged.

Events envelope, object and relation contracts, record and provenance contracts, ledger identity, append authority, idempotency, replay, subscription, watermarks, consumer cursors, remote contract, and observability are reusable unchanged.

## Explicit Non-Integration Findings

Execution does not receive the heterogeneous Strategy Plan.

Execution does not receive or execute Epistemic Operations.

Task Network does not coordinate dependencies between Tasks and EpiOps.

Execution does not need Traversal, Curation rules, Belief revisions, Strategy theory, or Plan progression state.

Task success does not establish root Goal satisfaction or documentation correctness.

The legacy Method planner is not evidence that Execution should remain a semantic planner for the new architecture.

Execution's package traversal DTOs are unrelated to world-model knowledge Traversal.

Events does not validate Curation semantics, perspective, rule identity, bounds, currentness, or terminal meaning.

An Event append receipt does not prove Traversal, Belief, or Agent visibility.

Durable cursor authority does not decide when a consumer's semantic state is complete. The consumer decides when its own state is durable enough to advance.

No Events migration, new Event envelope, new graph coordinate, new relation type registry, or Curation-specific command protocol is proven.

## Evidence Confidence

Confidence is high for current Goal Set cardinality, exact Strategy authorization storage, modification behavior, active Goal selection, authorized planning revalidation, Composition lowering, Task Network ownership, Task outcome publication, Event envelope semantics, append idempotency, replay bounds, ledger identity, and durable consumer cursor behavior. These findings come from current contracts, stores, actors, and tests.

Confidence is high that no Task Network or Events semantic change is required by the proposition. Both domains already expose the needed owner boundaries and durable mechanics.

Confidence is moderate on the exact future Execution intake shape. Current evidence proves that the one-authorization slot is incomplete for repeated executable products under one execution-facing Goal, but it does not prove that the eventual design will use that cardinality. The architecture has not selected whether each executable product receives a derived execution-facing Goal, a product queue beneath one Goal, or another explicit cardinality contract.

Confidence is moderate on whether `ExecutionComposition` remains the execution-local lowering DTO or is replaced by a more neutral Task product copy. Its current fields can carry executable work, but several fields preserve world-model planning context that lowering does not need.

## Unresolved Questions

The evidence does not decide whether each eligible executable product carries the full root Agent Goal, a stable reference to it, or a distinct execution-facing Goal identity.

The evidence does not decide whether a new executable product replaces, appends beneath, or coexists with an earlier product for the same root Goal.

The evidence does not decide which Agent authorization identity must survive on Task Network lineage after Strategy Plan reconciliation.

The evidence does not decide whether the canonical Execution path retains `ExecutionComposition` as an internal lowering DTO or accepts another complete Task-shaped product and adapts locally.

The evidence does not decide whether legacy `plan_goal` remains callable for compatibility after the canonical Strategy seam is established. Its removal is not required to validate the new boundary.

The evidence does not decide whether planned Curation requests use a direct typed port, a Curation queue, or an accepted-request Event. That question belongs to world-model Curation. Every option can reuse the current Events spine for discovery, recovery, and shared result publication.

This report stops at evidence and impact. It does not authorize requirements, sequencing, migration, or implementation.
