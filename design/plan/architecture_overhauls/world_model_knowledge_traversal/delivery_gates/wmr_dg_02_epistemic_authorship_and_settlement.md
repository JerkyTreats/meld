# WMR-DG-02 Epistemic Authorship And Settlement Gate

Date: 2026-08-21

Gate identifier: `WMR-DG-02`

Revision: 1 frozen

Active slice: `WMR-DD-02`

Intended handoff: constructible Curation operation and independently visible result positions to `WMR-DD-03`

Gate owner: Codex separate cross-deliverable acceptance lane

Exception authority: user

## Coherence Horizon

The gate begins with the immutable `TraversalCut` accepted by `WMR-DG-01` plus either an installed standing Curation rule or the Curation-owned acceptance side of an authorized Plan product. It ends at independent Graph visibility and, only for an installed evidence route, one exact Belief revision position.

Agent Plan construction, Plan authorization production, durable Plan progression, Agent result acceptance, complete `PlannerCut` assembly, Strategy construction, Execution, root activation, and lifecycle aggregation are outside the horizon.

The planned invocation boundary may define what Curation accepts. It may not claim that `WMR-DD-03` already produces or progresses that authorization.

## Candidate Deliverables

- [worker packet](../detailed_design/wmr_dd_02_worker_packet.md)
- [epistemic operation transition ledger](../detailed_design/epistemic_operation_transition_ledger.md)
- [epistemic authorship and settlement design](../detailed_design/epistemic_authorship_and_settlement.md)
- [handoff and lifecycle ledger](../world_model_reconciliation_handoff_ledger.md)
- this frozen Gate Definition
- integrated design-review receipt

Named upstream artifacts:

- accepted [WMR-DG-01 receipt](wmr_dg_01_acceptance_receipt.md)
- [semantic transition ledger](../detailed_design/semantic_transition_ledger.md)
- [owner publication to TraversalCut design](../detailed_design/owner_publication_to_traversal_cut.md)
- canonical Curation and Belief architecture

## Handoff Edges

Active closure edges:

- `WMR-H05`, `TraversalCut` to standing Curation
- `WMR-H06`, Traversal products to configured Belief
- `WMR-H08`, Curation terminal result to Events
- `WMR-H09`, Events to configured Belief
- `WMR-H29`, Curation result Event to Graph visibility
- `WMR-H30`, Curation result Event to configured Belief revision

Declared deferred-consumer edges:

- `WMR-H07`, Agent Plan authorization to planned Curation
- `WMR-H10`, Belief revision to Agent
- `WMR-H26`, Belief revision to `PlannerCut` assembly
- `WMR-H31`, Curation-derived Belief revision to Agent acceptance

## Lifecycle Claims

- standing work is accepted against an exact rule, Agent perspective, source cut, and bounded selection position
- planned work is accepted only through an exact authorization envelope whose production remains deferred
- terminal Curation state is durable before Event publication is reported complete
- Event append, Graph visibility, configured Belief settlement, and Agent acceptance remain independent positions
- replay uses stable operation, result, semantic publication, and Event identities
- wake references are structural positions such as a new source cut, rule revision, authorization identity, projection position, or configured Belief dependency
- fences include Agent, perspective, branch, activation generation, rule or operation revision, and source-cut identity
- restart resumes from durable Curation and consumer positions without treating replay as fresh authority
- quiescence is Curation-local and cannot imply Graph, Belief, Agent, or activation-wide quiet

## Criteria

| Criterion | Cross-deliverable claim | Acceptable evidence | Forbidden substitution | Blocking standard |
| --- | --- | --- | --- | --- |
| `WMR-DG-02-C01` | standing and planned invocation use one Curation semantic authority and one operation and result grammar | invocation comparison plus transition ledger | two Curation owners joined only by similar names | any duplicated or ambiguous semantic authority blocks |
| `WMR-DG-02-C02` | every operation is bounded by exact authority, perspective, rule or operation revision, roots, cut, query bounds, vocabulary, and completion meaning | operation identity account and proof traces | a live graph query or generic Agent identity | any unbound input or output authority blocks |
| `WMR-DG-02-C03` | standing intake closes while planned intake names an exact Curation acceptance position and keeps Agent production deferred | `WMR-H05`, `WMR-H07`, and invocation table | claiming planned authorization exists | false downstream readiness blocks |
| `WMR-DG-02-C04` | applied, unchanged, abstained, incomplete, rejected, conflicted, and operationally failed outcomes terminate observably without implying Goal satisfaction | outcome table and result identity | silence, timeout, Event absence, or Task completion | any non-observable terminal path blocks |
| `WMR-DG-02-C05` | Curation-owned semantic products retain perspective, provenance, currentness, and foreign-owner limits | authorship and supersession account | generic edge meaning or ledger order as truth | semantic ownership leakage blocks |
| `WMR-DG-02-C06` | terminal result durability and neutral Event append close as separate positions | `WMR-H08` and transition ledger | append receipt as operation execution proof | missing or conflated positions block |
| `WMR-DG-02-C07` | Graph visibility closes at a projection position at or beyond the exact result Event | `WMR-H29` and visibility barrier table | Event sequence alone | no independent Graph barrier blocks |
| `WMR-DG-02-C08` | configured Belief settlement requires an explicit eligible route and exact revision position | `WMR-H06`, `WMR-H09`, and `WMR-H30` | universal Event interpretation or graph reachability | implicit evidence admission blocks |
| `WMR-DG-02-C09` | Agent and Planner acceptance remain named downstream positions without being borrowed as current completion | `WMR-H10`, `WMR-H26`, and `WMR-H31` | Gate acceptance as Agent acceptance | false consumer closure blocks |
| `WMR-DG-02-C10` | the already-correct docs trace reaches epistemic closure without Goal or Task creation | direct proof trace | current write-pipeline success | requiring executable work blocks |
| `WMR-DG-02-C11` | missing docs is represented by a positive bounded non-realization assessment over complete scope | direct proof trace | absence of a relation occurrence | inferred absence blocks |
| `WMR-DG-02-C12` | dependency security preserves inventory, advisory, assessment, and verification owner meaning | dissimilarity trace | Curation-authored foreign truth | owner collapse blocks |
| `WMR-DG-02-C13` | replay, feedback, idempotency, currentness, and supersession rules prevent self-triggered unbounded derivation | recovery and fixed-point account | Event deduplication alone | an unbounded or oscillating path blocks |
| `WMR-DG-02-C14` | waits, wakes, fences, restart, and local quiescence cite exact structural positions | handoff ledger lifecycle rows | process ticks, empty queues, or absent consumers | any false lifecycle claim blocks |
| `WMR-DG-02-C15` | the candidate stays inside the accepted design envelope | worker packet, scope report, and diff | future implementation need as present authority | source work or architectural expansion blocks |

Every criterion is blocking. Acceptance requires every criterion to pass or carry a user-authorized exception.

## Acceptance Inputs

- this frozen revision
- exact candidate manifest and digest
- accepted upstream Gate Receipt
- integrated design-review receipt
- criterion-level evidence
- affected handoff entries
- authorized exceptions, if any

## Acceptance Budget

- one initial acceptance pass
- one frozen violation set
- one program-owner disposition
- one bounded remediation cycle
- one verification pass limited to failed criteria and correction-caused regressions

The acceptance owner may judge only these criteria. The owner may not redesign Curation, select remediation, waive a criterion, authorize expansion, or activate `WMR-DD-03`.

## Gate Output

Use only `accepted`, `rejected`, or `not eligible`.

An accepted receipt establishes that `WMR-DD-03` may rely on a constructible Curation operation grammar, exact terminal results, independent Graph and configured Belief visibility positions, and explicit deferred Agent acceptance positions. It does not authorize `WMR-DD-03`.
