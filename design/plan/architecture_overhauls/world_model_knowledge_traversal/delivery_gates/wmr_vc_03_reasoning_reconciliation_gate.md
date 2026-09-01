# WMR-VC-03 Reasoning Reconciliation Delivery Gate

Date: 2026-08-29

Gate identifier: `WMR-VC-03-DG`

Revision: frozen revision 1

Status: accepted

Source baseline: `4a01416f`

Activation record: [WMR-VC-03 Source Activation](wmr_vc_03_source_activation_record.md)

Candidate slice: `WMR-VC-03`

Intended handoff: one exact Agent-owned Goal has one admitted heterogeneous Plan, one planned Curation product has reached durable consumer acceptance and exact milestone reconciliation, and one complete Task is eligible but unpublished pending `WMR-VC-04`

Gate owner: primary bounded Gate Acceptance lane

Exception authority: user

Freeze authority: explicit user implementation authorization and approval of the Agent-owned durable reconciliation trees on 2026-08-30

## Coherence Horizon

The gate begins with the exact accepted Graph and Belief positions from `WMR-VC-01` and `WMR-VC-02`, one active Agent and generation, one admitted Agent-owned Goal, installed directive and maintained-condition data, one Capability catalog revision, one Curation operation catalog revision, exact scope and authority, and one deliberately limited Planner policy.

It ends when Planner has returned one complete immutable cut, Strategy has constructed and verified one immutable mixed Plan, Agent has durably judged the Plan and separately authorized one eligible Epistemic Operation, Curation has durably accepted and completed that exact operation, Agent has accepted the declared exact owner milestone, and one complete Task has become eligible without being published to Execution.

Execution admission, Goal Set replacement, Task Network lowering and realization, Capability invocation, returned semantic-owner observation, PDS compilation, activation lifecycle redesign, Startup proof, and product migration are outside the horizon.

## Candidate Deliverables

- [source assessment](../reasoning_reconciliation_source_assessment.md)
- accepted [PlannerCut, Strategy Plan, and Agent progression design](../detailed_design/planner_cut_strategy_plan_and_agent_progression.md)
- accepted [PlannerCut and Plan transition ledger](../detailed_design/planner_cut_and_plan_transition_ledger.md)
- [delivery-design review](../reviews/wmr_vc_03_delivery_design_review_receipt.md)
- this frozen Gate Definition
- [source activation record](wmr_vc_03_source_activation_record.md)
- [implementation review receipt](../reviews/wmr_vc_03_implementation_review_receipt.md)
- [Style Assurance Receipt](../reviews/wmr_vc_03_style_assurance_receipt.md)
- [Gate Acceptance Receipt](wmr_vc_03_gate_acceptance_receipt.md)

Named upstream artifacts:

- accepted [WMR-VC-01 receipt](wmr_vc_01_gate_acceptance_receipt.md)
- accepted [WMR-VC-02 receipt](wmr_vc_02_gate_acceptance_receipt.md)
- [single-runtime recovery amendment](../world_model_reconciliation_single_runtime_recovery.md)
- [Runtime Invariants](../../../../../governance/runtime_invariants.md)
- accepted Planner, Strategy, Agent, Curation, Execution, and Meld Language architecture

## Producer And Consumer Edges

| Edge | Producer | Consumer | Required handoff |
| --- | --- | --- | --- |
| `WMR-VC-03-E01` | Graph and Traversal | Planner | exact immutable `TraversalCut`, result, frontier, provenance, and declared completeness |
| `WMR-VC-03-E02` | Belief | Planner | exact required revision bodies with evidence and invalidation lineage |
| `WMR-VC-03-E03` | installed theory and authority owners | Planner | exact directive, maintained condition, Capability catalog, Curation catalog, scope, authority, branch, perspective, generation, and policy positions |
| `WMR-VC-03-E04` | Planner | Strategy | one complete immutable `PlannerCut` or one explicit refusal |
| `WMR-VC-03-E05` | Strategy | Agent | one verified immutable heterogeneous `StrategyPlan` with exact context and predecessor lineage |
| `WMR-VC-03-E06` | Agent | Curation | one durable product authorization carrying one complete Epistemic Operation and exact Goal, Plan, context, authority, generation, and idempotency lineage |
| `WMR-VC-03-E07` | Curation | Agent | exact durable acceptance or rejection and exact terminal result position |
| `WMR-VC-03-E08` | Graph and Belief | Agent | independently observed visibility and admitted revision positions only when the Plan milestone requires them |
| `WMR-VC-03-E09` | Agent progression | future Execution admission | one complete Task remains eligible and unpublished with exact Goal, Plan, context, authority, and idempotency lineage |

## Lifecycle Claims

- activation requires one exact Agent generation and installed reconciliation policy
- Planner returns complete cut or explicit refusal and never a warning-bearing partial cut presented as complete
- Strategy construction and verification are pure over one frozen request
- Agent persists Plan judgment before product eligibility and product authorization before publication
- Curation persists acceptance before semantic execution
- publication intent persists before any Event append is treated as complete
- waits name missing owner revisions, Planner refusal, unsatisfied milestones, unavailable consumer receipts, invalidated authority, or future Execution admission
- wakes name exact revisions, cuts, results, receipts, deadlines, or successor requests
- fences bind Agent, Goal, Plan revision, product, context, branch, perspective, authority, and activation generation
- restart resumes from durable Agent and Curation records without duplicating semantic work
- local quiescence requires no eligible reconciliation transition and a durable wait for every incomplete handoff
- local quiescence does not imply Execution completion, activation quiescence, Startup satisfaction, or safe retirement

## Criteria

| Criterion | Cross-deliverable claim | Acceptable evidence | Forbidden substitution | Blocking standard |
| --- | --- | --- | --- | --- |
| `WMR-VC-03-DG-C01` | the real root runtime composes one bounded Agent reconciliation participant over canonical Planner, Strategy, Agent, Curation, Graph, Belief, and theory seams | root composition proof and runtime registry inventory | test-only constructor or optional successor adapter | no real canonical participant blocks |
| `WMR-VC-03-DG-C02` | `PlannerCut` binds every source position declared required by the installed limited policy and records Causation and Regime as explicitly not required | cut identity proof, source inventory, and policy fixture | latest reads, timestamp, aggregate cursor, or silently absent owner | any unbound required input or false completeness blocks |
| `WMR-VC-03-DG-C03` | missing, stale, conflicting, mismatched, or unauthorized required input returns one typed Planner refusal before Strategy runs | refusal matrix and root negative proof | warning-bearing partial projection or Strategy-side repair | any partial cut presented as complete blocks |
| `WMR-VC-03-DG-C04` | `PlannerCut` is the sole reasoning consistency root and any derived world-model view is subordinate to it | public export and caller inventory after deletion | retained `PlannerProjectionOutput` currentness authority | two reasoning roots block |
| `WMR-VC-03-DG-C05` | Strategy constructs and verifies one deterministic immutable Plan containing conditions, at least one complete Task, at least one bounded Epistemic Operation, exact dependencies, explanation, and frozen context | mixed-plan direct proof and identity property proof | one `Composition`, runtime search graph, or Task Network as the Plan | collapsed or mutable Plan meaning blocks |
| `WMR-VC-03-DG-C06` | Plan family, revision, condition, product, dependency, selection, and predecessor identities are non-circular and preserve completed history | identity account, successor property proof, and replay proof | in-place mutation or Plan-revision input to semantic product identity | unstable or rewritten lineage blocks |
| `WMR-VC-03-DG-C07` | Agent owns durable Goals, Plan judgments, product progression, consumer receipts, and milestone acceptance in one reconciliation family | store inventory, reopen proof, and query proof | Execution Goal storage or process memory as Agent history | split durable ownership blocks |
| `WMR-VC-03-DG-C08` | Plan judgment and per-product authorization are distinct immutable Agent decisions | record proof and negative authorization test | whole-Plan approval as product authority | authority overreach blocks |
| `WMR-VC-03-DG-C09` | eligibility rechecks exact dependencies, source currentness, authority, scope, and generation before each authorization | currentness comparison, invalidation, and stale-generation proof | Plan verification or historical eligibility | stale authorization blocks |
| `WMR-VC-03-DG-C10` | Agent publication carries one complete Epistemic Operation and durable authorization before Curation acceptance | `E06` proof, record ordering, and crash-window recovery | Plan identity, Event append, or generic command as authority | incomplete or replay-created authority blocks |
| `WMR-VC-03-DG-C11` | standing and planned work share one Curation acceptance, terminality, publication, query, and replay authority | actor and store inventory, mixed standing and planned regression | second Curation store, actor, or Event grammar | parallel Curation authority blocks |
| `WMR-VC-03-DG-C12` | Agent distinguishes Curation terminality, Graph visibility, Belief admission, and Agent milestone acceptance and progresses only on the Plan-declared kind | milestone matrix and delayed-consumer proof | Event append or Curation success as universal completion | false milestone inference blocks |
| `WMR-VC-03-DG-C13` | after the planned operation milestone is accepted, one complete Task becomes eligible but no Task, Plan, or new Agent Goal is published to Execution | root proof, Execution store non-mutation proof, and port inventory | Goal-command compatibility writer or deferred cleanup promise | premature consumer publication blocks |
| `WMR-VC-03-DG-C14` | the superseded projection, candidate, split Agent curation, and Agent-to-Execution Goal writer routes are removed from real runtime callers and public authority | source, export, registration, persistence, exclusive-test, and caller deletion inventory | dormant successor beside incumbent or reader used as writer | any equivalent incumbent route blocks |
| `WMR-VC-03-DG-C15` | reopen after each durable boundary resumes without duplicate Plan judgment, authorization, Curation acceptance, semantic execution, Event publication, or milestone acceptance | fault-injection restart matrix and exact record identities | clean restart after all publication completes | any duplicate or lost position blocks |
| `WMR-VC-03-DG-C16` | waits, wakes, fences, and local quiescence cite exact durable positions and keep future Execution admission explicit | bounded tick reports and waiting assertions | empty queue, elapsed time, or absent consumer | false liveness claim blocks |
| `WMR-VC-03-DG-C17` | Docs Freshness proves mixed Plan grammar and Dependency Security proves dissimilar owner separation without claiming product migration | focused semantic fixtures | existing fixed pipeline or scan exit status | collapsed product or owner semantics blocks |
| `WMR-VC-03-DG-C18` | Style Assurance covers comments, domain layout, formatting, strict lint, direct tests, negative tests, restart, property, durable replay, and focused fuzz applicability | satisfied receipt over exact reviewed candidate | passing workspace suite alone | incomplete engineering assurance blocks |
| `WMR-VC-03-DG-C19` | the candidate stays within approved expansion, write scope, and tripwires | source activation record, diff inventory, and complexity delta | future need or design acceptance as source authority | unauthorized expansion or tripwire breach blocks |

Every criterion is blocking. Acceptance requires every criterion to pass or carry a user-authorized exception.

## Acceptable Direct Proof

The primary proof must use the real root runtime assembly and persistent stores. It must install one limited reconciliation policy, assemble an exact cut from accepted owner positions, construct a deterministic mixed Plan, persist Agent judgment, authorize the planned operation, drive the real Curation actor, observe the exact declared milestone, and show the Task eligibility transition with no Execution mutation.

Focused unit, property, state-machine, fuzz, and fault-injection tests may establish identity, validation, and recovery claims. They cannot substitute for root composition, caller deletion, consumer acceptance, or the absence of the superseded runtime route.

## Style Assurance Requirements

Style Assurance must review the exact post-implementation-review candidate. It must apply Contribution Policy comments, domain-first module layout, modern Rust module conventions, adapter thinness, strict changed-crate lint, formatting, and risk-proportionate test quality.

Public durable Planner, Strategy, Agent progression, and planned Curation records create identity, deserialization, replay, persistence, and state-machine risk comparable to existing world-model fuzz targets. Focused property and fuzz or equivalent state-machine evidence is therefore applicable. Restart evidence must interrupt at Agent judgment, product authorization, Curation acceptance, Curation terminal persistence, Event receipt, and Agent milestone acceptance boundaries.

## Hard Limits And Tripwires

Forbidden without new user approval:

- any new crate, workspace dependency, database, service, Event authority, Graph authority, Belief consumer, Causation domain, Regime domain, or compatibility writer
- any Task admission, Task Network change, Capability invocation, product migration, PDS compilation, or activation lifecycle redesign
- any direct Graph or Belief write by Planner, Strategy, Agent, or Curation
- any restored source from the archived additive attempt without fresh line-level review

Tripwires:

- more than twenty-four production source files changed
- more than four thousand five hundred production lines added
- more than one root Agent reconciliation participant
- any retained new Agent Goal or mutation writer into Execution
- any retained equivalent `PlannerProjectionOutput`, `StrategyCandidate`, split Agent curation actor, or process-memory progression authority

Crossing a tripwire changes the slice to `awaiting approval` even when tests pass.

## Acceptance Inputs

- this definition frozen at revision 1 after source authorization
- exact candidate tree or commit range
- source activation record
- direct root product proof
- current and successor route inventory
- implementation-review receipt
- satisfied Style Assurance Receipt
- criterion-level evidence
- complexity delta
- authorized exceptions, if any

## Acceptance Budget

- one initial Gate Acceptance pass
- one frozen violation set
- one program-owner disposition
- one bounded remediation cycle
- one verification pass limited to failed criteria and correction-caused regressions

The overall verdict must be `accepted`, `rejected`, or `not eligible`.

## Gate Output

An accepted receipt establishes that the future Execution vertical may rely on one complete eligible Task with exact Agent, Goal, Plan, product, context, authority, generation, and idempotency lineage. It also establishes that planned Curation and Agent reconciliation operate through one canonical durable route.

Acceptance does not authorize `WMR-VC-04`, Execution admission, PDS, Startup, or product migration.

## Current Authority State

Revision 1 remains frozen. The initial acceptance for digest `46bd1e2b3d4435f23dbed3cf9ecaadf0a63b7b1318029f790bdcccfa50e7967a` was reopened by a later audit. The exact bounded successor digest `12285cdf7091486e69a77cc25c8fccba2ab2bea222b183d2e451d24b3adaba2f` is accepted in the [successor receipt](wmr_vc_03_successor_gate_acceptance_receipt.md). No later source slice is active, and no authority is granted to change this gate or activate `WMR-VC-04`.
