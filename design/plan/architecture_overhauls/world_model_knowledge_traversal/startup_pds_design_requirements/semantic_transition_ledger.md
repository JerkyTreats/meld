# Startup PDS Semantic Transition Ledger

Date: 2026-08-23

Status: independent requirements evidence

## Purpose

This ledger projects the Startup PDS through the accepted World Model Reconciliation handoffs. It adds no new transport authority. It names the exact producer and consumer position required for the product proof.

## Transition Spine

| Edge | Producer and durable product | Consumer acceptance | Identity and retry | Wait and wake | Fence |
| --- | --- | --- | --- | --- | --- |
| `SPDS-H01` | PDS product revision and complete package compilation receipt | assignment accepts exact product and one-Agent topology | equal package set and owner receipts reuse compilation | missing owner receipt waits on exact installation successor | product revision and compilation policy |
| `SPDS-H02` | assignment, topology receipt, Capability preparation, participant plan, and inert closure | lifecycle accepts one preparing generation | equal closure and request key reuse lifecycle decision | incomplete preparation waits on exact owner receipt | assignment, expected prior head, closure |
| `SPDS-H03` | realized participant set and native readiness receipts | lifecycle publishes current generation and open admission epoch | publication is conditional on exact prior head | incomplete parity or readiness waits on owner successor | generation, incarnation set, epoch |
| `SPDS-H04` | current generation and open admission epoch | Agent records deterministic nonce instance and standing eligibility | equal epoch derives equal nonce | closed epoch waits on exact open epoch | assignment, Agent, generation, epoch |
| `SPDS-H05` | Event coverage, Graph position, and exact initial `TraversalCut` | standing Curation accepts `assess_startup_nonce` | equal rule, nonce, cut, and perspective derive equal operation | incomplete coverage waits on Event or Graph successor | nonce, cut, generation, epoch |
| `SPDS-H06` | terminal standing result and Curation publications | Events appends deterministic result publications | retry reuses Curation result and Event identities | missing append receipt wakes on Event authority | Curation result, source cut, epoch |
| `SPDS-H07` | standing mismatch publication through configured route | Belief commits immutable mismatch revision | evidence identity binds Event, route, key, and predecessor | absent route or unsettled evidence waits on exact successor | nonce key, perspective, generation, epoch |
| `SPDS-H08` | mismatch Belief and Agent input position | Agent incepts deterministic Goal | equal nonce and maintained condition reuse Goal decision | stale or incomplete evidence waits on exact revision | Agent, nonce, directive, generation, epoch |
| `SPDS-H09` | complete native revisions and construction request | Planner returns complete `PlannerCut` or refusal | equal source set and policy yield equal cut | refusal waits on each named successor | Goal, Agent, authority, generation, epoch |
| `SPDS-H10` | Goal and complete construction request | Strategy returns immutable Plan | equal frozen inputs yield equal semantic products and Plan | no runtime wait exists inside Strategy | Goal, cut, catalogs, policies |
| `SPDS-H11` | verified Plan revision | Agent records Plan judgment | equal Plan and context reuse judgment | rejected or stale Plan waits on successor cut or policy | Agent, Goal, Plan, epoch |
| `SPDS-H12` | eligible emitter Task and fresh Planner check | Agent records complete Task authorization | equal Task and authority input reuse authorization | unmet dependency or stale cut waits on exact successor | Task, Plan, authority, generation, epoch |
| `SPDS-H13` | Agent-authorized Task | Execution Goal Set records accepted, duplicate, rejected, or conflicted admission | Task authorization and consumer request key remain distinct | in-flight handoff wakes on exact admission receipt | Agent, Goal, Task, generation, epoch |
| `SPDS-H14` | accepted admission and unified Task Network mutation | dispatch records claim, attempt, and external operation | deterministic lowering and operation identities prevent duplicate effect | binding or readiness waits on exact Execution successor | admission, Capability binding, epoch |
| `SPDS-H15` | exact generic nonce emitter request | Events returns append receipt for deterministic Event kind `nonce` | retry looks up or appends the same record identity | Event authority unavailable wakes on exact authority recovery | nonce, subject, correlations, fence, producer contract |
| `SPDS-H16` | Event record at exact ledger sequence | Graph cursor covers all nonce projection writes | replay is idempotent by record and projection revision | Graph lag waits on exact sequence | ledger, sequence, route, generation, epoch |
| `SPDS-H17` | occurrence-rich nonce `TraversalResult` under successor cut | Agent accepts Event visibility milestone | equal cut and Plan dependency reuse absorption | missing or stale cut waits on projection successor | Plan, dependency, nonce, epoch |
| `SPDS-H18` | eligible confirmation operation and Agent authorization | Curation records intake decision | authorization and operation identities remain distinct | missing authorization waits on exact Agent position | Agent, Goal, Plan, operation, epoch |
| `SPDS-H19` | terminal confirmation result and semantic publications | Events, Graph, and configured Belief close independent positions | deterministic result and publication identities survive replay | each consumer waits on its exact predecessor | result, cut, route, generation, epoch |
| `SPDS-H20` | immutable realization Belief revision | Agent records exact returned milestone acceptance | Plan dependency and Belief revision derive absorption identity | stale or mismatched evidence waits on exact successor | Agent, Goal, Plan, nonce, epoch |
| `SPDS-H21` | Goal inception and returned milestone acceptance | Agent records satisfaction receipt | equal Goal and milestone reuse disposition | unresolved condition waits on exact owner milestone | Goal satisfaction contract and epoch |
| `SPDS-H22` | all native work and wait positions | lifecycle liveness projection resolves work, idle, quiescent, stalled, or interrupted | owner checkpoints remain authoritative | missing wait or wake produces stalled, not quiescent | generation, incarnation, epoch |
| `SPDS-H23` | exact native owner positions for one nonce | inspection returns repeatable nonce account | frozen position set yields equal projection | unavailable evidence remains explicit | assignment, generation, epoch, nonce, time fence |

## Relationship To Accepted Handoffs

| Startup edge family | Accepted handoff reuse |
| --- | --- |
| package, assignment, and preparation | `WMR-H20` through `WMR-H22` |
| readiness and epoch publication | `WMR-H23` |
| initial cut and standing Curation | `WMR-H03` and `WMR-H05` |
| Curation publication and Belief | `WMR-H08`, `WMR-H09`, `WMR-H29`, and `WMR-H30` |
| Planner, Strategy, and Agent | `WMR-H19` and `WMR-H26` through `WMR-H28` |
| Task admission and unified Execution | `WMR-H13` through `WMR-H17` |
| nonce owner publication | `WMR-H01` and `WMR-H02` |
| Graph visibility and Agent progression | `WMR-H04`, `WMR-H19`, and `WMR-H31` |
| waits, recovery, and retirement | `WMR-H24` and `WMR-H25` |
| inspection | `WMR-H18` |

The nonce Capability effect composes two existing boundaries. Execution dispatch reaches one owner implementation. That implementation produces one `WMR-H01` owner publication. No new Event or Execution handoff class is required.

## Independent Position Rules

These positions may occur in either order and may disagree temporarily:

- nonce Event append and Execution terminal outcome
- Event Graph visibility and Execution outcome publication
- Graph visibility and configured Belief settlement
- Belief settlement and Agent milestone acceptance
- Agent Goal satisfaction and Execution uncertain-effect closure
- nonce satisfaction and activation-wide quiescence

No implementation may replace these pairs with one combined status.

## Failure Localization

The success-critical path is the shortest declared chain from nonce eligibility to Agent satisfaction. The inspection projection reports the first absent, stale, conflicted, unavailable, or unproved consumer position on that chain.

Parallel obligations remain visible. For example, an Event may return and satisfy the Agent while Execution still reconciles a lost provider callback. Inspection reports both rather than downgrading the valid satisfaction receipt or hiding the unresolved operational effect.

## Terminality

The nonce account has no new authoritative terminal state. It projects native outcomes.

`satisfied` requires an Agent satisfaction receipt. `pending` means the next declared owner position is not yet terminal. `stalled` means a required wait, wake, route, binding, or consumer is unresolved. `conflicted` means exact identities disagree. `superseded` means a successor admission epoch or product lineage has become current.

Timeout is not a semantic terminal class. It is an inspection observation over a deadline wake and the still-current native positions.
