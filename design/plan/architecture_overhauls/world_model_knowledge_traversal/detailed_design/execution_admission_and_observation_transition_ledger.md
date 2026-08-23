# Execution Admission And Observation Transition Ledger

Date: 2026-08-22

Slice: `WMR-DD-04`

Status: corrected design product for `WMR-DG-04` revision 2

Implementation authorization: none

## Identity Chain

| Identity | Owner | Meaning | Stable inputs | Must remain distinct from |
| --- | --- | --- | --- | --- |
| Task semantic identity | Strategy under accepted Plan grammar | one independently complete executable product | desired condition, Capability contracts, exact inputs, internal dependencies, expected outcome | Plan revision and Execution admission |
| Agent Task authorization identity | Agent | permission to offer one Task under one Plan revision | Agent, Goal, Plan revision, selection reference, Task, frozen context, authority, generation, idempotency | Execution acceptance and Task result |
| Goal Set admission identity | Execution | one Goal-attributed accepted, duplicate, rejected, or conflicted intake decision | authorization, Task, Goal attribution, current Capability contracts, policy, generation fence | Agent authorization and operational node |
| lowering identity | Execution | deterministic realization proposal for one admitted Task | admission identity, Task body, selected installed Capability contracts | Strategy reasoning and Task Network commit |
| coherence decision identity | Execution Planning | compatibility or separation decision over one or more admitted executable regions | admission set, Capability contracts, inputs, source revisions, effects, authority, validity windows, result schemas, generation | Task meaning and Agent authorization |
| Task Network mutation identity | Task Network | one durable graph mutation set against the unified network | lowering and coherence identities, predecessor revision, command identity | Task semantic identity |
| operational node identity | Execution | one runnable realization that may retain several compatible admissions | compiled action, compatible admission set, network revision, lineage | Task, Goal, claim, and attempt |
| admission attribution identity | Execution | one contributing admission attached to one operational node and later discharge | admission, node, Task, Goal, Agent, Plan, authority, generation | shared operational outcome |
| claim identity | Execution | exclusive bounded right to attempt one operational node | operational node, complete admission attribution set, worker, network revision, lifecycle epoch | external operation identity |
| attempt identity | Execution | one fenced realization attempt | claim, activation generation, participant incarnation, attempt ordinal | operational outcome and provider callback |
| external operation identity | Execution and provider seam | idempotent uncertain effect request against one resolved target | attempt, operational node, Capability contract, exact installed binding revision, resolved Capability instance, effect target, input digest, authority | callback delivery and semantic observation |
| operational outcome identity | Execution | one durable terminal or unresolved realization account for one operational node | operational node, claim, attempt, operation, outcome class | admission discharge, Event record, and semantic-owner observation |
| admission discharge identity | Execution | one independently addressable account of how an operational outcome applies to one contributing Task admission | admission attribution, operational outcome, result compatibility, expected outcome | another admission discharge and Agent milestone acceptance |
| Execution Event identity | Execution through Events | neutral durable carriage of operational outcome, discharge set, and artifacts | outcome identity, complete discharge set, deterministic publication key | Event sequence and downstream visibility |
| owner observation identity | workspace, docs, dependency security, or another semantic owner | owner-authored observation after examining returned state or artifacts | source identity, source revision, scope, outcome lineage, completeness receipt | operational outcome and belief evidence |
| returned Event identity | semantic owner through Events | neutral carriage of the owner observation | owner publication operation and exact observation batch | Execution outcome Event |
| Graph visibility position | Graph | returned owner Event materialized through exact sequence | ledger, sequence, projection revision | Event append receipt |
| Belief revision identity | Belief | configured settlement over returned owner evidence | belief key, owner observation, route, comparator, predecessor | Graph reachability and Agent acceptance |
| Agent milestone acceptance | Agent | durable absorption of the exact Plan dependency milestone | Plan revision, dependency, owner product, owner position | Goal satisfaction |

## Position Partial Order

```text
Agent Task authorization durable
-> Goal Set admission decision durable
-> coherence decision durable
-> unified Task Network mutation durable
-> operational node claim and attempt durable
-> operational outcome and per-admission discharge durable

operational outcome and discharge set durable -> Execution Event append durable

operational outcome, source change, or owner selection rule
-> semantic-owner observation and completeness durable
-> returned owner Event append durable
-> Graph visibility durable when required
-> configured Belief revision durable when routed and required
```

Agent milestone acceptance consumes the exact position declared by the Plan dependency. Direct owner, returned Event, and Graph milestones reach Agent through `WMR-H19`; `WMR-H33` through `WMR-H35` remain the upstream owner-to-owner positions that produce those milestones. A configured Belief milestone reaches Agent through `WMR-H36`. Agent does not require traversal of every branch.

No earlier position proves a later position on the same branch. Execution Event publication and semantic-owner re-observation are independent consequences of an operational outcome and neither proves the other.

## Admission Outcomes

| Outcome | Meaning | Consumer position | Retry behavior |
| --- | --- | --- | --- |
| accepted | exact Goal-attributed Task passed consumer-owned shape, Capability, authority, and generation checks | durable Goal Set admission receipt | retry returns the same receipt |
| duplicate | the same authorization and Task were already accepted | existing admission and Task Network lineage | no duplicate realization |
| rejected | malformed, incomplete, outside Execution ownership, or unauthorized at intake | durable rejection receipt | same identity remains rejected |
| conflicted | current Capability, generation, or policy revision differs from the authorized fence | durable conflict receipt naming exact revisions | successor authorization or explicit reconciliation required |

Execution may validate its own consumer contract and current installed realization seam. It may not search for another Method, reconstruct Strategy reasoning, change the Goal, or authorize a different product.

## Coherence Outcomes

| Outcome | Meaning | Durable evidence |
| --- | --- | --- |
| distinct | admission requires its own operational region | coherence decision and separate node attribution |
| shared | all compatibility dimensions permit one operational node to serve several admissions | compatibility proof, shared node identity, complete attribution set |
| attached | a later compatible admission is attached before the shared effect fence closes | successor network mutation and admission attribution |
| refused sharing | apparent similarity fails one or more compatibility dimensions | negative coherence decision naming the mismatched dimensions |

Sharing never merges Task, Goal, Plan, Agent, authorization, milestone, or Goal-disposition identities. One shared operational outcome produces one independently addressable discharge account per admission.

## Realization Outcomes

| Outcome | Meaning | Evidence |
| --- | --- | --- |
| succeeded | the exact attempt completed under the declared outcome grammar | durable outcome, artifacts, attempt, operation, and provider receipt lineage |
| failed | the attempt reached a terminal operational failure | durable failure with effect classification |
| cancelled | Execution fenced or cancelled the attempt under declared policy | durable cancellation and unresolved-effect account |
| uncertain | the external effect may have occurred but terminal confirmation is unavailable | durable unresolved operation identity and reconciliation requirement |

Operational success plus one admission discharge proves only the Execution-owned result for that admission. It never proves semantic-owner observation, correctness, Belief settlement, Agent acceptance, or Goal satisfaction.

## Wait, Wake, Fence, And Restart

| Edge | Wait | Wake | Fence | Restart source |
| --- | --- | --- | --- | --- |
| `WMR-H13` | no exact authorization or consumer decision pending | exact Agent authorization or current contract revision | Agent, Goal, Plan, selection, Task, authority, generation | Agent authorization plus Execution admission journal |
| `WMR-H14` | admission accepted but coherence decision or unified-network mutation absent or conflicted | exact predecessor revision, coherence result, or idempotent mutation result | admission, Task, unified network, compatibility inputs, generation | admission, coherence decision, attribution set, and Task Network journal |
| `WMR-H15` | no durable or exactly reconstructible dispatch route | installed binding, routing-rule revision, route successor, or admitted Task change | Task, Capability contract, exact binding revision, exact routing-rule revision, generation | admitted Task plus durable route product or mandatory deterministic reconstruction from the same binding and routing-rule revisions |
| `WMR-H16` | dependencies or initialization inputs unresolved, claim held, or no eligible worker | exact dependency outcome, input, claim expiry, or worker availability | network revision, operational node, complete admission attribution set, claim, epoch, generation, exact installed binding revision, resolved Capability instance, and effect target | Task Network state, attributions, claims, attempts, operation receipts, installed binding revision, and effect-target reconciliation |
| `WMR-H17` | terminal outcome exists but publication receipt absent | Event append availability or retry | outcome, publication key, ledger | outcome outbox and Event append receipt |
| `WMR-H32` | outcome or declared observation source unavailable | exact artifact, source revision, or owner observation trigger | outcome lineage, owner source, perspective, branch, generation | owner source cursor and publication operation |
| `WMR-H33` | returned owner publication absent from Events | exact owner publication operation | owner revision, publication key, ledger, generation | owner outbox and Event append receipt |
| `WMR-H34` | returned Event absent from Graph position | exact Event sequence or projection retry | ledger, sequence, projection revision, generation | Event cursor and Graph projection checkpoint |
| `WMR-H35` | returned owner evidence absent from configured Belief position | exact routed evidence or predecessor revision | owner revision, route, belief key, predecessor, generation | owner evidence, route revision, and Belief revision chain |
| `WMR-H36` | required Belief milestone absent from Agent position | exact configured Belief revision | Plan dependency, Belief revision, Agent context, generation | Belief revision and Agent milestone decision |

Polling can discover eligibility but is not itself a wake or durable handoff.

## Deferred Lifecycle Fields

`WMR-DD-06` will aggregate participant incarnation, activation-wide readiness, structural wake resolvability, safe points, late-result classification across generation replacement, and retirement. This ledger fixes the owner positions those later products must reference without selecting a lifecycle store or protocol.
