# Reasoning Reconciliation Source Assessment

Date: 2026-08-29

Candidate slice: `WMR-VC-03`

Mode: design

Readiness: `approval-ready`

Implementation authority: none

Investigation budget: twelve batched inspection calls and twenty additional source items

Budget used: ten batched inspection calls and nineteen additional source items

Confidence: high for current ownership, runtime wiring, and the proposed cutover boundary

## Architectural Thesis

The current runtime turns a belief threshold into one Strategy `Composition`, attaches that candidate to an Agent Goal command, stores the Goal in Execution, and later lets Execution perform another planning pass. The accepted architecture instead requires Planner to freeze the reasoning context, Strategy to construct one heterogeneous Plan, Agent to own Goal and Plan progression, and each consumer to receive only its exact authorized product.

`WMR-VC-03` should replace the current projection, candidate, and split Agent curation route in one vertical. Its direct proof ends when a mixed Plan drives one planned Epistemic Operation through the real Curation consumer and the accepted milestone makes one complete Task eligible. It must not publish that Task to Execution before the Execution admission vertical exists.

## Concern And Scope

The assessed behavior begins with one active Agent, one admitted Goal, the accepted exact workspace and Curation graph cut, configured Belief revisions, installed directive and construction data, and current authority. It follows immutable reasoning-cut assembly, pure heterogeneous Plan construction, Agent Plan judgment, one planned Curation authorization, durable Curation acceptance, exact result reconciliation, and the resulting next-product eligibility decision.

In scope are current Planner projection ownership, Strategy candidate construction, Agent Goal formation and satisfaction control, Agent-owned durable Goal and Plan progression, planned Curation intake, root composition, waiting and restart, and the removal of every superseded real caller on that route.

Out of scope are Execution Task admission, Goal Set replacement, Task Network lowering, Capability invocation, returned semantic-owner observation, PDS compilation, activation lifecycle redesign, Startup proof, product migration, new Causation or Regime source domains, and generalized compatibility machinery.

The decision context is deliberately limited before source selection. It requires exact Traversal, Belief, directive, Capability, Curation catalog, scope, authority, branch, perspective, generation, and policy revisions. Causation and Regime are not silently omitted. Their source positions are explicitly not required by this first installed reconciliation policy because no current source authority supplies them.

Evidence is grounded in current source, the accepted `WMR-VC-01` and `WMR-VC-02` receipts, the single-runtime recovery amendment, Runtime Invariants, and the accepted Planner, Strategy, Agent, and Curation architecture.

## Current Product Route

```text
Belief revision and graph query
-> PlannerProjectionOutput
-> threshold Agent curation decision
-> StrategyCandidate over one Composition
-> StrategyAuthorization attached to AgentGoalCommand
-> root CurationGoalExecutionPort
-> Execution Goal Set
-> Execution PlanningRuntimeActor
-> Task Network
-> Agent satisfaction curation and Goal mutation
```

Current source anchors are `PlannerQuery`, `StrategyCandidate`, `AgentGoalCurationActor`, `AgentSatisfactionCurationActor`, `CurationGoalExecutionPort`, `ExecutionGoalCommandPort`, and the root registrations for `world_model.agent_goal_curation`, `world_model.satisfaction_curation`, and `execution.planning`.

The route has three ownership defects relative to accepted architecture.

- `PlannerProjectionOutput` is a live latest-state projection rather than the exact immutable consistency root consumed by Strategy.
- `StrategyCandidate` is one executable `Composition`, so it cannot represent several Tasks, Epistemic Operations, or exact owner milestones.
- Agent persists threshold decisions while Execution stores the Goal and performs the next semantic planning pass, so Agent does not own one durable Plan progression history.

## Intended Product Route

```text
active Agent and admitted Agent-owned Goal
-> complete deliberately limited PlannerCut or explicit refusal
-> pure Strategy construction and verification
-> immutable mixed StrategyPlan
-> durable Agent Plan judgment
-> exact Epistemic Operation eligibility and authorization
-> durable planned Curation acceptance and terminal result
-> exact Agent milestone acceptance
-> complete Task becomes eligible
-> wait for future Execution admission consumer
```

The product increment is observable without a placeholder consumer. The planned Curation operation reaches the existing Curation authority, publishes through the accepted Event and Graph route, and returns an exact durable result to Agent progression. The complete Task remains a Strategy product under one Plan revision and is not submitted through the superseded Goal-command adapter.

## Regenerated Domain Snapshot

The domain universe comes from current workspace source and the canonical WMR ledger. It includes every semantic owner or product boundary on the full reconciliation program path. Root technical folders are represented by their owned product domain rather than counted as independent architecture domains.

| Domain | Current source ground |
| --- | --- |
| PDS and theory selection | root initialization selects installed Agent, Belief, Strategy, and Curation theory revisions |
| workspace and semantic product owners | workspace publishes accepted owner facts and later products retain observation authority |
| Events | one durable append and replay authority |
| Graph and Traversal | one owner projection, immutable cut, and occurrence-rich bounded query authority |
| Belief | one evidence admission, revision, and query authority |
| Causation | accepted design owner with no current source domain on this route |
| Regime | accepted design owner with no current source domain on this route |
| Planner | current live `PlannerProjectionOutput` and `PlannerQuery` projection route |
| Curation | accepted standing operation store and actor with no planned authorization intake |
| Strategy | current pure `StrategyCandidate` search and verification over one `Composition` |
| Agent | current registration, activation, subscription, threshold decision, sink receipt, and split curation actors |
| Execution Goal Set | current durable Goal and Strategy authorization recipient |
| Execution Planning and Task Network | current semantic planning, lowering, dispatch, and outcome authority |
| Meld Language | shared pure Goal, Method, Composition, and world-state values |
| runtime lifecycle | root supervision, leases, bounded ticks, waiting, and activation generation |
| root composition | current store, theory, port, actor, and runtime registry wiring |
| inspection and Startup | current diagnostics and later complete route proof |
| Docs Freshness | current concrete Strategy and Execution specimen |
| Dependency Security | current dissimilar future migration specimen |
| legacy Workflow | incumbent product path outside this vertical |

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| PDS and theory selection | publish | publish | partial | installed Strategy, maintained-condition, Agent, and Curation data reach root composition | none | reuse selected revisions and do not change compilation |
| workspace and semantic product owners | publish | publish | complete | accepted owner publications and exact cuts from `WMR-VC-01` | none | reuse unchanged |
| Events | publish | publish | complete | accepted append and replay authority from both prior verticals | none | reuse unchanged |
| Graph and Traversal | publish | publish | complete | exact owner cuts and Curation projection accepted by prior gates | none | reuse exact cut contracts |
| Belief | publish | publish | complete | configured immutable revisions and query route | none | reuse revision bodies and lineage |
| Causation | none | none | not needed | no current source owner or installed current policy input | first installed decision context excludes it explicitly | no source work |
| Regime | none | none | not needed | no current source owner or installed current policy input | first installed decision context excludes it explicitly | no source work |
| Planner | own | own | partial | current projection is deterministic but not a complete immutable reasoning cut | none | replace projection authority with cut assembly and refusal |
| Curation | consume | own | partial | standing Curation exists but accepts no Agent-authorized planned operation | none | extend existing store and actor with planned intake |
| Strategy | own | own | partial | pure candidate search exists but produces one executable composition | none | replace candidate grammar with immutable heterogeneous Plan |
| Agent | own | own | partial | Agent owns decisions but not durable Goals, Plans, or unified progression | none | replace split curation route with one reconciliation actor |
| Execution Goal Set | none | consume | not needed | current Goal command adapter is the superseded output of Agent curation | no Task admission is authorized in this slice | stop new Agent Goal publication and leave Execution redesign to `WMR-VC-04` |
| Execution Planning and Task Network | none | own | not needed | current actor consumes Execution Goals and performs semantic planning | Task consumption and replacement are the next vertical | no new Task publication and no Execution source work |
| Meld Language | publish | publish | complete | Goal and Composition values remain pure shared vocabulary | none | reuse unchanged unless exact serialization support is unavoidable |
| runtime lifecycle | consume | consume | partial | supervisor supports bounded durable actors but current Agent work is split | none | replace two Agent participants with one bounded reconciliation participant |
| root composition | adapter | adapter | partial | root currently binds Planner projection and Agent Goal sinks into Execution | none | compose exact owners and remove superseded ports and registrations |
| inspection and Startup | none | observe | not needed | complete Startup proof remains a later slice | current direct integration proof is a focused root test | preserve current diagnostics only |
| Docs Freshness | observe | consume | partial | current fixture proves the incumbent candidate and Execution route | product migration is later | use only as a mixed-plan semantic specimen |
| Dependency Security | observe | none | partial | accepted design supplies a dissimilarity specimen | product migration is later | use only for grammar and owner-separation proof |
| legacy Workflow | none | own | not needed | no current caller overlap with this focused world-model cut was found | retirement requires its own complete inventory | no change |

## Frozen Affected Domain Set

The affected set is frozen as:

- PDS and theory selection
- workspace and semantic product owners
- Events
- Graph and Traversal
- Belief
- Planner
- Curation
- Strategy
- Agent
- Meld Language
- runtime lifecycle
- root composition
- Docs Freshness
- Dependency Security

Execution Goal Set, Execution Planning and Task Network, inspection and Startup, Causation, Regime, and legacy Workflow are explicit non-integrations for this slice. Their presence on the wider runtime path does not grant write scope.

## Pass Two Affected Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| installed revision selection | PDS and theory selection | exact installed revisions already reach root | supply Planner and Strategy policy and catalog identities | reuse unchanged | root must not invent missing revisions | current theory selection and registries |
| owner observation | workspace and semantic owners | immutable owner receipts and cuts | remain the source of observed facts | reuse unchanged | Curation must not claim owner truth | accepted `WMR-VC-01` receipt |
| durable carriage | Events | deterministic append and replay | carry Curation output only | reuse unchanged | Event identity must not become semantic authority | accepted `WMR-VC-02` receipt |
| exact source cut | Graph and Traversal | immutable occurrence-rich `TraversalCut` | become one bound Planner source position | reuse unchanged | derived projection must not replace cut identity | Planner architecture and accepted cut proof |
| immutable evidence | Belief | exact revision bodies and invalidation lineage | become bound Planner source positions and Agent wake inputs | reuse unchanged | latest lookup must not silently change a Plan | Belief query and accepted settlement proof |
| assembly request | Planner | subject and dimension live query | bind declared required owner revisions before selection | new local behavior | optional fields could create false completeness | Planner specification |
| assembly refusal | Planner | warnings can accompany partial projection | return one explicit refusal for missing, stale, conflicting, or unauthorized input | new local behavior | Strategy-side repair would split authority | Planner specification |
| cut and derived view | Planner | `PlannerProjectionOutput` is the current root | make `PlannerCut` canonical and any view subordinate | replace existing | retaining both public roots creates parallel currentness authority | recovery amendment |
| planned intake | Curation | standing rule selection creates its own operation | accept one complete Agent authorization and exact operation | extend existing | Agent replay must not grant Curation acceptance twice | Curation contracts architecture |
| operation realization | Curation | durable admission, result, outbox, and receipts exist | reuse terminal and publication mechanics for planned operations | extend existing | planned and standing operations must share one Curation authority | accepted `WMR-VC-02` implementation |
| Plan construction | Strategy | candidate search returns one `Composition` | construct immutable conditions, Tasks, Epistemic Operations, and dependencies | replace existing | a Plan must not become a Task Network | Strategy architecture |
| verification and identity | Strategy | candidate verification and identity are pure | verify closure, lineage, exact context, milestones, and stable identity | extend existing | semantic product and Plan revision identities must not be circular | accepted detailed design |
| Goal ownership | Agent | threshold decisions target Execution Goal storage | persist admitted Goals in Agent storage | new local behavior | retaining new Goal writes to Execution would preserve split ownership | Agent architecture |
| Plan judgment | Agent | candidate authorization is attached to a Goal command | persist admitted, rejected, and superseded Plan judgments separately from product authority | new local behavior | whole-Plan approval must not authorize every product | Agent plan progression design |
| product progression | Agent | two actors separately create and satisfy Goals | own dependency, currentness, authorization, receipt, milestone, and successor states | replace existing | split actors or process callbacks would create dual progression | recovery amendment |
| pure values | Meld Language | shared Goal and Composition types | continue as value vocabulary only | reuse unchanged | moving Plan grammar into the language crate would erase domain ownership | Meld Language architecture |
| bounded participation | runtime lifecycle | root leases two Agent actors | supervise one durable Agent reconciliation actor | extend existing | empty polling must not be called quiescence | runtime lifecycle architecture |
| storage and ports | root composition | binds Agent Goal commands and mutations to Execution | bind Planner, Strategy, Agent, and planned Curation contracts only | adapter only | a compatibility forwarding writer would retain the old route | Runtime Invariants |
| Docs Freshness specimen | Docs Freshness | current Strategy package produces one candidate | prove a mixed Plan and exact milestone order without migration | reuse unchanged | fixture proof must not be called product migration | accepted Strategy design |
| Dependency Security specimen | Dependency Security | design-only dissimilar path | prove grammar does not collapse owner distinctions | reuse unchanged | scan status must not become applicability or mitigation truth | accepted Strategy design |

## Ownership And Boundary Synthesis

Planner owns exact source consistency and refusal. Strategy owns pure Plan construction and verification. Agent owns Goals, Plan judgment, per-product authority, durable progression, and milestone acceptance. Curation owns acceptance and realization of the authorized Epistemic Operation. Events and Graph carry and project Curation products without inheriting their meaning. Belief owns whether mapped results become admitted evidence.

The smallest missing connective behavior is one Agent reconciliation participant that persists its own Goal and Plan history, calls Planner and Strategy through their public contracts, sends one exact planned operation to Curation, and resumes only from durable consumer and owner positions.

The root remains composition authority only. It may adapt carrier shapes, but it may not construct a `PlannerCut`, choose Plan products, judge currentness, or infer milestone completion.

## Separated Scopes

Domains on the runtime path:

- PDS and theory selection
- workspace and semantic product owners
- Events
- Graph and Traversal
- Belief
- Planner
- Curation
- Strategy
- Agent
- runtime lifecycle
- root composition

Domains whose behavior must change:

- Planner
- Curation
- Strategy
- Agent
- runtime lifecycle registration
- root composition adapters

Likely implementation units:

- `crates/meld-world-model/src/planner.rs` and behavior modules beneath `planner`
- `crates/meld-world-model/src/strategy.rs` and behavior modules beneath `strategy`
- `crates/meld-world-model/src/agent.rs` and behavior modules beneath `agent`
- `crates/meld-world-model/src/curation.rs` and focused planned-operation behavior beneath `curation`
- `crates/meld-world-model/src/lib.rs`
- `src/runtime/assembly.rs`, `src/runtime/ports.rs`, `src/runtime/storage.rs`, and focused runtime contracts
- focused world-model and root integration tests
- focused property and fuzz targets where durable replay and public deserialization risk applies

This is a write envelope, not a promise that every listed file changes.

## Superseded Surface Inventory

The completed source change must remove or make non-authoritative every real caller of:

- `PlannerProjectionOutput` as the reasoning consistency root
- `PlannerQuery::project_current_world_state` on Agent progression
- `StrategyCandidate` and `StrategyAuthorization` as the Agent planning product
- `AgentGoalCurationActor`
- `AgentSatisfactionCurationActor`
- `CurationGoalExecutionPort`
- Agent-originated `ExecutionGoalCommandPort` and `ExecutionGoalMutationPort` writes
- root registrations `world_model.agent_goal_curation` and `world_model.satisfaction_curation`

Durable readers for prior Agent decision and sink receipt records may remain only when current stored-data evidence requires them. They may not originate new Goals, Plans, product authorizations, or Execution mutations. The implementation review must inventory public exports, exclusive tests, root registrations, persisted trees, and reopen behavior before declaring deletion complete.

## Maturity Envelope

```yaml
maturity:
  posture: first slice
  obligation_floor: operational durability for accepted graph, belief, curation, and agent records
  confidence: high
  evidence:
    - two accepted vertical runtime proofs
    - current real root Agent and Execution wiring
    - accepted Planner, Strategy, Agent, and Curation architecture
    - rejected additive source attempt and single-runtime recovery
  user_override: none
  direct_product_proof: one mixed Plan drives a planned Curation operation through durable acceptance and exact Agent milestone progression
  hard_limits:
    new_crates: forbidden
    new_durable_stores: forbidden
    new_cross_domain_protocols: forbidden beyond the frozen Planner, Strategy, Agent, and Curation contracts
    new_workspace_dependencies: forbidden
    parallel_implementors: forbidden
  tripwires:
    changed_production_files: 24
    added_production_lines: 4500
  investigation_budget:
    inspection_calls: 12
    source_files_beyond_named_design_and_policy: 20
  approval_gates:
    - explicit source authorization
    - explicit approval of Agent-owned durable reconciliation trees inside the existing world-model database
    - frozen WMR-VC-03-DG revision 1
  review_budget:
    review_owner: primary integrated implementation review lane
    reviewer_lanes: one
    initial_passes: one
    verification_passes: one
    reviewer_inspection_calls: 16
    reviewer_additional_source_files: 24
```

The tripwires are provisional but evidence-grounded. They allow a larger cut than `WMR-VC-02` because this slice must delete and replace three incumbent authorities across one real root route. Crossing either threshold pauses integration for user disposition.

## Expansion Decisions

One expansion is proposed for approval with source authorization: Agent-owned durable Goal, Plan, judgment, progression, consumer receipt, and milestone trees inside the existing world-model database.

The direct product behavior cannot survive restart or keep Plan judgment distinct from product authorization using the current Agent threshold-decision trees alone. Reusing Execution Goal storage would preserve the ownership defect. A new database, crate, service, or generic protocol is unnecessary.

Planner cut bodies and Strategy Plans should be stored through the Agent reconciliation family when they are part of durable Agent history. Planner and Strategy remain pure owners of construction contracts and do not gain background runtimes or independent stores.

No other expansion is approved or proposed.

## Direct Proof And Stop Conditions

The direct root proof must demonstrate:

- an exact deliberately limited `PlannerCut` over accepted Graph, Belief, directive, catalog, scope, authority, branch, perspective, generation, and policy positions
- explicit refusal for every required missing, stale, conflicting, or unauthorized input
- deterministic mixed `StrategyPlan` identity with one complete Task and one bounded Epistemic Operation
- durable Plan judgment distinct from product eligibility and authorization
- planned Curation acceptance before semantic execution
- terminal Curation result and publication recovery across reopen
- exact Agent milestone acceptance without inferring Graph or Belief completion from Curation terminality
- the Task becoming eligible only after the declared milestone
- no Task or heterogeneous Plan publication to Execution
- no new Agent Goal or mutation write through the superseded Execution adapters
- restart resuming from durable Agent and Curation positions without duplicate judgment, authorization, acceptance, or publication

Stop before editing if source authorization or the durable-tree approval is absent. Stop during implementation if the slice needs a new crate, dependency, database, service, Event authority, Graph authority, Belief consumer, Causation domain, Regime domain, Task admission, Task Network change, product migration, or compatibility writer.

## Test Quality Posture

Focused examples must cover complete construction, already-satisfied closure, mixed products, bounded refusal, invalid authority, stale currentness, dependency blocking, planned Curation rejection, accepted failure, incomplete result, successor Plan lineage, and restart at each durable publication boundary.

Property proof is required for deterministic cut, Plan, product, judgment, authorization, and milestone identities plus stable ordering and replay idempotency. Focused fuzz targets are required for public durable Planner, Strategy, Agent progression, and planned Curation deserialization or equivalent state-machine risk. The real root proof and complete sequential workspace suite remain required.

## Explicit Non-Integration Decisions

- no Task admission or Execution source change
- no continued Agent Goal publication into Execution
- no PDS compilation or activation lifecycle redesign
- no Causation or Regime source placeholder
- no new Event grammar owned by Events
- no Graph or Belief direct write from Planner, Strategy, Agent, or Curation
- no Docs Freshness or Dependency Security product migration
- no Workflow retirement
- no compatibility forwarding writer
- no source restoration from the archived additive attempt

## Unresolved Questions

No architecture question blocks approval-ready status. Source activation still requires the user to approve the proposed Agent-owned durable tree family and authorize `WMR-VC-03` implementation. Exact internal tree names and the smallest semantics-preserving file split belong to implementation inside the frozen gate.

## Assessment Result

`WMR-VC-03` is approval-ready as a bounded replacement vertical. It is not active. The proposed [Delivery Gate](delivery_gates/wmr_vc_03_reasoning_reconciliation_gate.md) may be frozen as revision 1 only after explicit source authorization and durable-tree approval.
