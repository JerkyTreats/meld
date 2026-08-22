# PlannerCut And Plan Transition Ledger

Date: 2026-08-22

Slice: `WMR-DD-03`

Status: active detailed design

Implementation authorization: none

## Purpose

This ledger separates source consistency, causal meaning, Agent authority, consumer delivery, and result progression. It is a seam account, not a shared runtime schema or a cross-domain status protocol.

Equal immutable inputs and equal semantic policy yield equal identities. A changed source revision, policy, semantic body, authority fence, or predecessor creates a successor rather than mutating an accepted product.

## Source And Construction Identities

| Identity | Owner | Meaning | Stable inputs | Must remain distinct from |
| --- | --- | --- | --- | --- |
| Goal identity | Agent and shared Goal language | one durable desired proposition under Agent authority | Agent, proposition, scope, priority, provenance | desired condition and product identity |
| `TraversalCut` identity | Graph and Traversal | one bounded occurrence-rich graph source cut | owner revisions, query, scope, bounds, policy | complete `PlannerCut` |
| source revision identity | native semantic owner | one immutable Belief, Causal, Regime, directive, Capability, or other admitted revision | owner-specific identity and lineage | Planner projection identity |
| `PlannerCut` identity | Planner | one complete source-consistency boundary for reasoning | exact source revision set, temporal and branch scope, perspective, authority, projection policy | `WorldModelView` and Strategy Plan |
| `WorldModelView` identity | Planner | one decision-shaped projection derived under a `PlannerCut` | cut identity, decision context, projection revision | source-cut currentness authority |
| construction request identity | Agent to Strategy | one ground Goal and exact frozen construction context | Goal, `PlannerCut`, Curation operation catalog, Strategy policy, predecessor history | Plan identity and Agent authorization |
| Plan family identity | Strategy | one causal reconciliation lineage for one Goal under one Agent | Agent, Goal, directive lineage | Plan revision identity |
| Plan revision identity | Strategy | one immutable semantic Plan body over one construction request | Plan family, request, stable condition, product, and dependency identities, assumptions, predecessor | Agent Plan judgment |
| desired condition semantic identity | Strategy | one proposition and satisfaction meaning reusable across successor Plans | Plan family, proposition, contribution, scope, satisfaction semantics | Plan-revision selection reference and durable Goal |

## Product And Authority Identities

| Identity | Owner | Meaning | Stable inputs | Must remain distinct from |
| --- | --- | --- | --- | --- |
| Task semantic product identity | Strategy from Capability contracts | one independently complete executable discharge product reusable only when its full meaning is equal | Plan family, desired-condition identities, Capability contract revisions, bound inputs, internal dependency graph, expected outcome and milestone | Plan-revision selection reference, authorization, and Task Network identity |
| Epistemic Operation semantic product identity | Strategy from Curation catalog | one independently bounded epistemic discharge product reusable only when its full meaning is equal | Plan family, desired-condition identities, Curation operation revision, exact target cut, bounds, requested authorship, completion and publication policy, authority requirements | Plan-revision selection reference, Task, and Curation acceptance identity |
| dependency semantic identity | Strategy | one causal or information edge between semantic conditions, products, and exact milestones | Plan family, producer identity, consumer identity, milestone requirement, semantics | Plan-revision selection reference and runtime schedule edge |
| Plan selection reference identity | Strategy | selection of one semantic condition, product, or dependency into one immutable Plan revision | Plan revision, selected semantic identity, role and ordering | semantic product identity |
| Plan judgment identity | Agent | one durable decision to admit or reject a Plan revision as a basis for progression | Agent, Goal, Plan revision, frozen context, directive, judgment policy | product authorization |
| Planner currentness check identity | Agent using Planner | one product-eligibility comparison between the Plan frozen cut and a fresh complete reassembly result | Agent, Goal, decision context, Plan frozen cut, assembly request, returned cut or refusal | `PlannerCut` and product eligibility |
| product eligibility identity | Agent | one evaluation of a selected product against current dependencies and fences | admitted Plan, selection reference, milestone set, Planner currentness check, authority, generation | Strategy verification |
| product authorization identity | Agent | permission for one exact eligible selected product | Agent, Goal, Plan revision, selection reference, complete product body, context, authority scope, generation, idempotency | Plan judgment and consumer acceptance |
| Curation acceptance identity | Curation | durable acceptance or rejection of an authorized Epistemic Operation | authorization, operation identity, Curation fence | Agent authorization and Curation result |
| Execution admission identity | Execution | future durable acceptance or rejection of one authorized complete Task | authorization, Task identity, Execution fence | Agent authorization and Task Network work |
| milestone acceptance identity | Agent | durable absorption of one owner milestone for one Plan dependency | Plan revision, dependency, owner product, exact owner position, Agent input cursor | owner result and Goal satisfaction |
| Goal disposition identity | Agent | one lifecycle judgment under owning satisfaction semantics | Goal, admitted evidence, Plan history, directive policy | product completion |

## PlannerCut Source Inventory

| Source | Required position | Completeness responsibility | Refusal when invalid |
| --- | --- | --- | --- |
| graph structure | exact `TraversalCut`, result, frontier, truncation, provenance, and hydrated owner revisions | Graph and Traversal producer plus Planner assembly policy | missing owner revision, unacceptable truncation, or scope conflict |
| Belief | exact revision set for every required key with evidence and invalidation lineage | Belief owner and Planner source-selection policy | missing, stale, conflicted, or perspective-mismatched required revision |
| Causation | exact causal claim, effect, mechanism, and assumption revisions required by the decision context | Causation owner | unidentified or unavailable required mechanism input |
| Regime | exact posterior, changepoint, segment, mixture, and stress revisions required by policy | Regime owner | missing or policy-incompatible regime input |
| directive context | exact Agent directive and maintained-condition revision | Agent and directive owner | expired, superseded, or out-of-scope authority |
| Capability catalog | exact catalog and contract revisions visible for Task construction | Capability owners and catalog authority | missing required contract body or incompatible revision |
| Curation operation catalog | exact constructible operation revisions available to Strategy | Curation | missing operation semantics or foreign-owner vocabulary request |
| scope and time | exact subject, temporal, transaction, horizon, branch, and exclusion policy | Agent request and Planner policy | incoherent or unsupported scope |
| perspective and authority | exact Agent lens, perspective, grants, and activation generation | Agent and authority owners | absent, stale, or incompatible authority |
| projection policy | exact Planner selection, hydration, risk, conflict, and completeness policy revision | Planner | unknown policy or non-reproducible projection |

Planner emits a complete immutable cut or an explicit assembly refusal. It never silently drops a required source or asks Strategy to repair cut consistency.

## Plan Closure And Cardinality

One Plan revision contains zero or more desired conditions, zero or more complete Tasks, zero or more bounded Epistemic Operations, and exact dependency edges.

A terminal Strategy construction position requires every desired condition to be one of:

- already satisfied under exact admitted owner evidence
- discharged by at least one independently closed product and declared milestone path
- decomposed into closed prerequisite conditions
- explicitly unresolved through a bounded observation or epistemic path that names what later evidence can settle it

A Plan may contain no discharge products when the Goal is already established. It may contain several Tasks and several Epistemic Operations. No product identity stands for the whole Plan.

## Milestone Registry

| Milestone kind | Owning evidence | What it may discharge | Forbidden inference |
| --- | --- | --- | --- |
| Curation terminal | exact `EpistemicOperationResult` identity and disposition | dependency on operation attempt or unchanged closure | Graph or Belief visibility |
| Graph visible | exact projection position covering required result publications | dependency on structural availability | evidence admission or Agent acceptance |
| Belief revision | exact belief key and immutable revision derived from named sources | dependency on configured settlement | Goal satisfaction or Agent absorption |
| Agent accepted | exact milestone acceptance identity | dependency requiring authority-bearing reconciliation | Planner or consumer work completion |
| Task admission | future Execution acceptance receipt | dependency on durable consumer admission | Task realization |
| Task outcome | future Execution owner outcome identity | dependency on operational terminality | external state truth or Goal satisfaction |
| owner observation | exact semantic owner publication and visibility position | dependency on observable world effect | Belief or correctness settlement |
| Goal disposition | exact Agent lifecycle decision under owning semantics | root Goal completion only | product completion from another owner |

Every dependency names one milestone kind, exact producer product, acceptance rule, and required position. Generic `completed`, an Event append, and a process callback are invalid milestone specifications.

## Agent Plan And Product States

### Plan judgment

| Decision | Meaning | Authority effect |
| --- | --- | --- |
| admitted | Plan is a valid current basis for progression | no product is yet authorized |
| rejected | Plan is unsuitable under current directive, authority, or risk policy | no product may progress |
| superseded | a successor Plan revision is current | old uncommitted products lose eligibility |

### Product progression

| State | Meaning | Required durable evidence |
| --- | --- | --- |
| blocked | at least one exact dependency or fence is unsatisfied | blocking milestone identities and current input positions |
| eligible | all declared dependencies and freshness checks pass | eligibility identity under one Plan revision |
| authorized | Agent granted authority to one exact product | product authorization identity |
| published | authorization was durably offered to the consumer and has an exact retry position | producer decision and in-flight retry position, not consumer acceptance |
| accepted | consumer durably accepted the exact authorization and product | Curation acceptance receipt or future Execution admission receipt |
| completed | the exact Plan-declared owner milestone was accepted | milestone acceptance identity naming kind and owner position |
| invalidated | a premise, authority, scope, or generation fence changed | invalidating source revision and decision |
| superseded | a successor Plan no longer selects the product | successor Plan and disposition |

The word completed has no meaning without its stored milestone kind and exact owner evidence.

## Authorization Sequence

```text
Strategy verification
-> Agent Plan judgment
-> product eligibility evaluation
-> product authorization
-> durable publication and retry position
-> consumer acceptance
-> owner milestone
-> Agent milestone acceptance
-> next eligibility, successor Plan, or Goal disposition
```

No arrow may be skipped by reusing an earlier identity. Plan admission is consent to progress the Plan, not blanket authority for its products.

## Successor Rules

- a changed frozen source, strategy policy, selected semantic body, or predecessor produces a new Plan revision
- Strategy may retain a semantic product identity only when its complete conditions, inputs, dependencies, authority requirements, expected outcome, and milestone remain equal
- every successor creates new Plan selection references for retained semantic products, and every authorization remains bound to one exact Plan revision and selection reference
- completed owner facts remain history and are referenced by the successor
- uncommitted products may be retained, replaced, added, or removed
- authorized or published products may be invalidated for future reliance, but their historical authorization and any owner outcome are never erased
- a successor Plan cannot retroactively change what milestone an earlier authorization required

## Lifecycle Projection

| Claim | Exact position |
| --- | --- |
| Planner ready | all required source revisions and assembly policy are available and coherent |
| Strategy ready | ground Goal, complete `PlannerCut`, operation catalog, construction policy, and predecessor history are frozen |
| Agent ready | Plan revision, directive, authority, generation, relevant input cursors, and completed Planner currentness check are available |
| wait | explicit missing Planner reassembly result, Plan, milestone, consumer receipt, authority, or successor condition |
| wake | exact Planner reassembly receipt, result position, consumer receipt, authority decision, deadline, or successor request |
| fence | Agent, Goal, Plan revision, product, context, branch, perspective, authority, and activation generation |
| restart | durable Agent judgments, progression states, Planner currentness checks, input cursors, consumer receipts, and owner milestones |
| local quiescence | no eligible Agent progression exists through the observed inputs and every published authorization has a retry or accepted consumer position |
| not implied | Execution completion, external observation, full activation quiescence, or safe retirement |

## Deferred Fields

`WMR-DD-04` must close Execution admission, independent Task Network realization, Task outcome, returned semantic-owner observation, and exact return milestones.

`WMR-DD-06` must close activation-wide readiness, aggregate waiting, quiescence, fencing, and safe retirement.
