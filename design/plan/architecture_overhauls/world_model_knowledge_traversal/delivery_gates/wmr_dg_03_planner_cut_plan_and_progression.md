# WMR-DG-03 PlannerCut, Plan, And Progression Gate

Date: 2026-08-22

Gate identifier: `WMR-DG-03`

Revision: 1 frozen

Active slice: `WMR-DD-03`

Intended handoff: one complete authorized Task envelope with Plan lineage to `WMR-DD-04`, while Curation authorization and Agent milestone progression close inside this slice

Gate owner: Codex separate cross-deliverable acceptance lane

Gate recommender: dedicated read-only subagent distinct from the integrated review recommender

Exception authority: user

## Coherence Horizon

The gate begins with a ground Goal, the exact Graph and Belief positions accepted by `WMR-DG-01` and `WMR-DG-02`, canonical Causation and Regime revisions, exact directive and Capability catalog revisions, Curation's accepted constructible operation grammar, and exact scope, authority, and projection policy.

It ends when Agent has durably judged one immutable Plan revision, separately authorized exact eligible products, closed the Curation authorization handoff, produced an exact deferred Execution Task envelope, absorbed declared owner milestones, and recorded successor or Goal disposition.

Execution admission, lowering, Task Network realization, returned owner observation, PDS compilation, root activation, and aggregate lifecycle are outside the horizon.

## Candidate Deliverables

- [worker packet](../detailed_design/wmr_dd_03_worker_packet.md)
- [PlannerCut and Plan transition ledger](../detailed_design/planner_cut_and_plan_transition_ledger.md)
- [PlannerCut, Strategy Plan, and Agent progression design](../detailed_design/planner_cut_strategy_plan_and_agent_progression.md)
- [handoff and lifecycle ledger](../world_model_reconciliation_handoff_ledger.md)
- this frozen Gate Definition
- integrated design-review receipt and its subagent recommendation

Named upstream artifacts:

- accepted [WMR-DG-01 receipt](wmr_dg_01_acceptance_receipt.md)
- accepted [WMR-DG-02 receipt](wmr_dg_02_acceptance_receipt.md)
- [owner publication to TraversalCut design](../detailed_design/owner_publication_to_traversal_cut.md)
- [epistemic authorship and settlement design](../detailed_design/epistemic_authorship_and_settlement.md)
- canonical Planner, Strategy, Agent, Causation, Regime, and public-interface architecture

## Handoff Edges

Active closure edges:

- `WMR-H04`, `TraversalCut` to immutable `PlannerCut`
- `WMR-H07`, Agent Plan authorization to planned Curation
- `WMR-H10`, Belief revision to Agent
- `WMR-H11`, Agent Goal and cut to Strategy
- `WMR-H12`, Strategy Plan revision to Agent progression
- `WMR-H19`, admitted result to Agent reconciliation
- `WMR-H26`, Belief revision to `PlannerCut` assembly
- `WMR-H27`, Causation and Regime revisions to `PlannerCut` assembly
- `WMR-H28`, directive and Capability revisions to `PlannerCut` assembly
- `WMR-H31`, Curation-derived Belief revision to Agent acceptance

Declared deferred-consumer edge:

- `WMR-H13`, Agent authorization to Execution admission

## Lifecycle Claims

- Planner assembles one exact immutable cut or returns an explicit refusal with missing, conflicting, stale, or unauthorized source positions
- Strategy construction and verification are pure over one frozen request and do not own waits or runtime progression
- Agent persists Plan judgment before product progression and persists product authorization before handoff
- every dependency names an owner product and exact milestone kind rather than generic completion
- waits cite missing source revision, unsatisfied milestone, invalidated premise, unavailable consumer receipt, or successor need
- wakes cite exact source, result, revision, receipt, authority, or deadline positions
- fences bind Agent, Goal, Plan revision, product, context, branch, perspective, authority, and activation generation
- restart resumes from durable Agent decisions, progression positions, input cursors, and consumer receipts
- local quiescence does not imply Execution completion, activation quiescence, or safe retirement

## Criteria

| Criterion | Cross-deliverable claim | Acceptable evidence | Forbidden substitution | Blocking standard |
| --- | --- | --- | --- | --- |
| `WMR-DG-03-C01` | `PlannerCut` binds exact Traversal, Belief, Causation, Regime, directive, Capability, scope, authority, temporal, branch, perspective, and projection-policy revisions | source inventory and identity account | latest reads, timestamps, or one aggregate cursor | any unbound Strategy input blocks |
| `WMR-DG-03-C02` | incomplete, stale, conflicting, or unauthorized sources produce an explicit Planner refusal rather than a partial cut presented as complete | assembly refusal table and proof traces | Strategy-side repair or implicit omission | false completeness blocks |
| `WMR-DG-03-C03` | `PlannerCut` is the source-consistency root and `WorldModelView` is only a derived decision projection under that cut | vocabulary reconciliation | two independent currentness authorities | competing cut identity blocks |
| `WMR-DG-03-C04` | Strategy consumes one frozen request and constructs a pure immutable Plan whose semantic identity is deterministic | Plan identity and construction account | runtime search state, live reads, or reviewer confidence | mutable or ambient Plan meaning blocks |
| `WMR-DG-03-C05` | every desired condition is satisfied, connected to a closed product, or explicitly unresolved through a bounded observation path | Plan closure rules and proof traces | ranking score or generic prospective evidence | unclosed causal meaning blocks |
| `WMR-DG-03-C06` | each Task is complete and independently identified, and one Goal or Plan may contain several Tasks | product and cardinality ledger | one executable Composition for the whole Plan | collapsed Task cardinality blocks |
| `WMR-DG-03-C07` | each Epistemic Operation conforms to the accepted Curation grammar and remains distinct from Task and Goal | operation grounding account and `WMR-H07` | Strategy-invented relation meaning or Execution routing | owner or product collapse blocks |
| `WMR-DG-03-C08` | every dependency cites an exact owner milestone such as Curation terminal, Graph visible, Belief revision, Agent acceptance, Task outcome, or owner observation | milestone registry and Plan traces | generic completed, Event append, or process callback | ambiguous discharge blocks |
| `WMR-DG-03-C09` | Plan family, revision, predecessor, condition, product, and successor identities preserve completed history without rewriting it | transition ledger and reconstruction account | in-place Plan mutation | lost lineage or rewritten fact blocks |
| `WMR-DG-03-C10` | Plan judgment and per-product authorization are distinct durable Agent decisions | Agent decision table | whole-Plan approval as product authority | authority overreach blocks |
| `WMR-DG-03-C11` | product eligibility rechecks exact dependencies, source freshness, authority, and generation before authorization and handoff | progression rules and lifecycle account | historical eligibility or Plan verification | stale authorization blocks |
| `WMR-DG-03-C12` | planned Curation closes from Agent authorization through the accepted Curation consumer position without replay granting authority | `WMR-H07` and direct proof | generic command Event or Plan identity alone | incomplete Curation handoff blocks |
| `WMR-DG-03-C13` | the Execution producer envelope contains one complete Task with Goal, Plan, product, context, authority, and idempotency lineage while consumer admission remains deferred | `WMR-H13` and downstream handoff | heterogeneous Plan, Epistemic Operation, or Strategy search state | wrong consumer product blocks |
| `WMR-DG-03-C14` | Agent absorbs exact Curation, Graph, Belief, and other declared milestones before progression and keeps Goal satisfaction separate | `WMR-H10`, `WMR-H19`, `WMR-H31`, and transition ledger | Event append, Task completion, or Curation completion as universal satisfaction | false progression or satisfaction blocks |
| `WMR-DG-03-C15` | already-correct docs can close with no executable product, while missing docs can produce a mixed multi-product Plan with exact milestone order | docs proof traces | existing fixed write pipeline | unnecessary Task or missing closure blocks |
| `WMR-DG-03-C16` | dependency security uses the same Plan grammar without collapsing inventory, advisory, assessment, mitigation, observation, and verification ownership | dissimilarity proof | scan success as applicability or mitigation as resolution | foreign-owner or workflow collapse blocks |
| `WMR-DG-03-C17` | waits, wakes, fences, restart, and local quiescence cite exact structural positions | handoff ledger lifecycle rows | process ticks, empty queues, or absent consumers | false lifecycle claim blocks |
| `WMR-DG-03-C18` | the candidate stays inside the accepted design envelope | worker packet, scope report, and diff | future implementation need as present authority | source work or architectural expansion blocks |

Every criterion is blocking. Acceptance requires every criterion to pass or carry a user-authorized exception.

## Recommendation And Authority

The integrated review recommender and Gate Acceptance recommender are read-only subagents. Each recommendation must cite the exact candidate, remain inside its packet, and return findings or criterion verdicts without editing artifacts, prescribing expansion, waiving criteria, or authorizing the next slice.

Codex remains the program owner, freezes findings or violations, assigns dispositions, creates receipts, and holds commit authority.

## Acceptance Inputs

- this frozen revision
- exact candidate manifest and digest
- accepted upstream Gate Receipts
- integrated design-review receipt and subagent recommendation
- criterion-level evidence
- affected handoff entries
- authorized exceptions, if any

## Acceptance Budget

- one initial subagent recommendation
- one frozen violation set
- one program-owner disposition
- one bounded remediation cycle
- one subagent verification recommendation limited to failed criteria and correction-caused regressions

The Gate Acceptance recommendation must use only `accepted`, `rejected`, or `not eligible`. It does not itself issue the program receipt.

## Gate Output

An accepted receipt establishes that `WMR-DD-04` may rely on an exact complete Task envelope, several-Tasks-per-Plan cardinality, Plan and Goal lineage, Agent authority, frozen context, idempotency, and a declared consumer acceptance position. It does not authorize `WMR-DD-04`.
