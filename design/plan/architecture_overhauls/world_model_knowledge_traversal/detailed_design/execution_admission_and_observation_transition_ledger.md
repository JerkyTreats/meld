# Execution Admission And Observation Transition Ledger

Date: 2026-08-22

Slice: `WMR-DD-04`

Status: active detailed-design product

Implementation authorization: none

## Identity Chain

| Identity | Owner | Meaning | Stable inputs | Must remain distinct from |
| --- | --- | --- | --- | --- |
| Task semantic identity | Strategy under accepted Plan grammar | one independently complete executable product | desired condition, Capability contracts, exact inputs, internal dependencies, expected outcome | Plan revision and Execution admission |
| Agent Task authorization identity | Agent | permission to offer one Task under one Plan revision | Agent, Goal, Plan revision, selection reference, Task, frozen context, authority, generation, idempotency | Execution acceptance and Task result |
| Execution admission identity | Execution | one accepted, duplicate, rejected, or conflicted consumer decision | authorization, Task, current Capability contracts, Execution policy, generation fence | Agent authorization and network mutation |
| lowering identity | Execution | deterministic realization proposal for one admitted Task | admission identity, Task body, selected installed Capability contracts | Strategy reasoning and Task Network commit |
| Task Network mutation identity | Task Network | one durable graph mutation set | lowering identity, network, predecessor revision, command identity | Task semantic identity |
| Task instance identity | Execution | one runnable node realization of the semantic Task | Task identity, compiled node, network revision, lineage | claim and attempt |
| claim identity | Execution | exclusive bounded right to attempt one Task instance | task instance, worker, network revision, lifecycle epoch | external operation identity |
| attempt identity | Execution | one fenced realization attempt | claim, activation generation, participant incarnation, attempt ordinal | Task outcome and provider callback |
| external operation identity | Execution and provider seam | idempotent uncertain effect request against one resolved target | attempt, Capability contract, exact installed binding revision, resolved Capability instance, effect target, input digest, authority | callback delivery and semantic observation |
| Task outcome identity | Execution | one durable terminal or unresolved realization account | Task instance, claim, attempt, operation, outcome class | Event record and semantic-owner observation |
| Execution Event identity | Execution through Events | neutral durable carriage of outcome and artifacts | outcome identity, deterministic publication key | Event sequence and downstream visibility |
| owner observation identity | workspace, docs, dependency security, or another semantic owner | owner-authored observation after examining returned state or artifacts | source identity, source revision, scope, outcome lineage, completeness receipt | Task outcome and belief evidence |
| returned Event identity | semantic owner through Events | neutral carriage of the owner observation | owner publication operation and exact observation batch | Execution outcome Event |
| Graph visibility position | Graph | returned owner Event materialized through exact sequence | ledger, sequence, projection revision | Event append receipt |
| Belief revision identity | Belief | configured settlement over returned owner evidence | belief key, owner observation, route, comparator, predecessor | Graph reachability and Agent acceptance |
| Agent milestone acceptance | Agent | durable absorption of the exact Plan dependency milestone | Plan revision, dependency, owner product, owner position | Goal satisfaction |

## Position Partial Order

```text
Agent Task authorization durable
-> Execution admission decision durable
-> Task Network mutation durable
-> Task claim and attempt durable
-> Task outcome durable

Task outcome durable -> Execution Event append durable

Task outcome, source change, or owner selection rule
-> semantic-owner observation and completeness durable
-> returned owner Event append durable
-> Graph visibility durable when required
-> configured Belief revision durable when routed and required
```

Agent milestone acceptance consumes the exact position declared by the Plan dependency. Direct owner, returned Event, and Graph milestones reach Agent through `WMR-H19`; `WMR-H33` through `WMR-H35` remain the upstream owner-to-owner positions that produce those milestones. A configured Belief milestone reaches Agent through `WMR-H36`. Agent does not require traversal of every branch.

No earlier position proves a later position on the same branch. Execution Event publication and semantic-owner re-observation are independent consequences of a Task outcome and neither proves the other.

## Admission Outcomes

| Outcome | Meaning | Consumer position | Retry behavior |
| --- | --- | --- | --- |
| accepted | exact authorized Task passed consumer-owned shape, Capability, authority, and generation checks | durable admission receipt | retry returns the same receipt |
| duplicate | the same authorization and Task were already accepted | existing admission and Task Network lineage | no duplicate realization |
| rejected | malformed, incomplete, outside Execution ownership, or unauthorized at intake | durable rejection receipt | same identity remains rejected |
| conflicted | current Capability, generation, or policy revision differs from the authorized fence | durable conflict receipt naming exact revisions | successor authorization or explicit reconciliation required |

Execution may validate its own consumer contract and current installed realization seam. It may not search for another Method, reconstruct Strategy reasoning, change the Goal, or authorize a different product.

## Realization Outcomes

| Outcome | Meaning | Evidence |
| --- | --- | --- |
| succeeded | the exact attempt completed under the declared outcome grammar | durable outcome, artifacts, attempt, operation, and provider receipt lineage |
| failed | the attempt reached a terminal operational failure | durable failure with effect classification |
| cancelled | Execution fenced or cancelled the attempt under declared policy | durable cancellation and unresolved-effect account |
| uncertain | the external effect may have occurred but terminal confirmation is unavailable | durable unresolved operation identity and reconciliation requirement |

Task success proves only the Execution-owned outcome. It never proves semantic-owner observation, correctness, Belief settlement, Agent acceptance, or Goal satisfaction.

## Wait, Wake, Fence, And Restart

| Edge | Wait | Wake | Fence | Restart source |
| --- | --- | --- | --- | --- |
| `WMR-H13` | no exact authorization or consumer decision pending | exact Agent authorization or current contract revision | Agent, Goal, Plan, selection, Task, authority, generation | Agent authorization plus Execution admission journal |
| `WMR-H14` | admission accepted but network mutation absent or conflicted | exact predecessor revision or idempotent mutation result | admission, Task, network, generation | admission and Task Network journal |
| `WMR-H15` | no durable or exactly reconstructible dispatch route | installed binding, routing-rule revision, route successor, or admitted Task change | Task, Capability contract, exact binding revision, exact routing-rule revision, generation | admitted Task plus durable route product or mandatory deterministic reconstruction from the same binding and routing-rule revisions |
| `WMR-H16` | dependencies or initialization inputs unresolved, claim held, or no eligible worker | exact dependency outcome, input, claim expiry, or worker availability | network revision, task instance, claim, epoch, generation, exact installed binding revision, resolved Capability instance, and effect target | Task Network state, claims, attempts, operation receipts, installed binding revision, and effect-target reconciliation |
| `WMR-H17` | terminal outcome exists but publication receipt absent | Event append availability or retry | outcome, publication key, ledger | outcome outbox and Event append receipt |
| `WMR-H32` | outcome or declared observation source unavailable | exact artifact, source revision, or owner observation trigger | outcome lineage, owner source, perspective, branch, generation | owner source cursor and publication operation |
| `WMR-H33` | returned owner publication absent from Events | exact owner publication operation | owner revision, publication key, ledger, generation | owner outbox and Event append receipt |
| `WMR-H34` | returned Event absent from Graph position | exact Event sequence or projection retry | ledger, sequence, projection revision, generation | Event cursor and Graph projection checkpoint |
| `WMR-H35` | returned owner evidence absent from configured Belief position | exact routed evidence or predecessor revision | owner revision, route, belief key, predecessor, generation | owner evidence, route revision, and Belief revision chain |
| `WMR-H36` | required Belief milestone absent from Agent position | exact configured Belief revision | Plan dependency, Belief revision, Agent context, generation | Belief revision and Agent milestone decision |

Polling can discover eligibility but is not itself a wake or durable handoff.

## Deferred Lifecycle Fields

`WMR-DD-06` will aggregate participant incarnation, activation-wide readiness, structural wake resolvability, safe points, late-result classification across generation replacement, and retirement. This ledger fixes the owner positions those later products must reference without selecting a lifecycle store or protocol.
