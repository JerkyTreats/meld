# PlannerCut, Strategy Plan, And Agent Progression Detailed Design

Date: 2026-08-22

Slice: `WMR-DD-03`

Status: accepted design with corrected integrated outcome evidence

Implementation authorization: none

## Decision

Planner owns one immutable source-consistency boundary called `PlannerCut`. Strategy purely constructs and verifies one immutable heterogeneous `StrategyPlan` over a ground Goal and frozen construction context. Agent durably judges the Plan, then separately authorizes each currently eligible product and reconciles exact owner milestones.

```text
exact native-owner revisions
-> complete PlannerCut
-> pure Strategy construction and verification
-> immutable Plan revision
-> Agent Plan judgment
-> product eligibility and authorization
-> Curation acceptance or deferred Execution admission
-> exact owner milestone
-> Agent acceptance and successor progression
```

This closes the authority ambiguity left by discovery. Agent uses two decision levels. Plan judgment admits one Plan revision as the current basis for progression. Product authorization grants permission only to one exact currently eligible Task or Epistemic Operation.

## Owner Boundaries

| Concern | Owner | Design rule |
| --- | --- | --- |
| structural source cut | Graph and Traversal | supplies one accepted occurrence-rich `TraversalCut` |
| evidence settlement | Belief | supplies immutable revisions with evidence and invalidation lineage |
| mechanisms and effects | Causation | supplies exact causal revisions without letting Planner infer effects |
| structural context | Regime | supplies exact regime and stress revisions without letting Planner choose the regime |
| directive, perspective, and authority | Agent and native authority owners | binds reasoning and authorization scope |
| executable contracts | Capability owners and catalog authority | supplies exact constructible Task leaves |
| epistemic operation contracts | Curation | supplies exact constructible operation leaves |
| reasoning-cut assembly | Planner | validates source consistency and produces one complete immutable cut |
| causal Plan construction and verification | Strategy | creates desired conditions, closed products, dependencies, explanation, and lineage |
| Plan judgment and progression | Agent | admits Plans, authorizes products, absorbs milestones, requests successors, and decides Goal lifecycle |
| epistemic realization | Curation | accepts only authorized Epistemic Operations |
| executable realization | Execution | later accepts only authorized complete Tasks |

Planner does not choose products. Strategy has no durable runtime authority. Agent does not reinterpret Curation or Execution semantics. Execution never receives the heterogeneous Plan.

## Complete PlannerCut Assembly

Planner receives an assembly request with one decision context and the native-owner source positions listed in the [transition ledger](planner_cut_and_plan_transition_ledger.md). It resolves and validates exact revisions before emitting a cut.

The cut binds:

- occurrence-rich graph objects, relations, provenance, frontier, and exact `TraversalCut`
- exact Belief revisions and their invalidation conditions
- exact causal claims, effect summaries, assumptions, and mechanism revisions
- exact Regime posterior, changepoint, segment, mixture, and stress revisions required by policy
- exact directive context and maintained-condition revision
- exact Capability catalog and contract revisions visible for construction
- temporal, transaction, horizon, branch, subject, exclusion, and perspective scope
- Agent lens, authority grants, activation generation, and projection-policy revision

Curation's constructible operation catalog and Strategy construction policy are frozen beside the cut in the construction request because they define available means rather than observed world state. The resulting Plan frozen-context identity binds them with the cut.

Planner emits either one complete cut or one refusal. Refusal names every missing, stale, conflicting, unauthorized, out-of-scope, or unacceptably truncated input. A partial source set may support a deliberately limited decision context only when the assembly policy declared that limitation before source selection. It may not masquerade as the complete cut for a broader Goal.

## PlannerCut And WorldModelView

`PlannerCut` is the canonical root identity for source consistency and replay. It answers which exact native-owner revisions formed the reasoning boundary.

`WorldModelView` is retained as a Planner-owned decision projection derived under one `PlannerCut`. It may shape actionable beliefs, risks, conflicts, assumptions, sensitivities, and hydration handles for a scoped question. It does not select a different current source set, advance source cursors independently, or become the identity cited by Plan lineage instead of the root cut.

This is a containment relationship:

```text
PlannerCut
-> zero or more WorldModelView projections
```

Equal cut identity, decision context, and projection revision yield equal projection meaning. A projection refusal cannot weaken cut completeness.

## Frozen Strategy Construction Request

Agent requests Strategy construction with:

- one ground Goal and Agent identity
- one complete `PlannerCut`
- exact Curation operation catalog revision
- exact Strategy construction and verification policy revision
- exact predecessor Plan and completed causal history when reconstructing
- resource bounds for pure search and explanation

The request freezes every semantic input. Strategy may hydrate native-owner bodies only through handles already bound to the cut and exact owner revisions. It cannot issue live reads, advance a cursor, wait on runtime state, or append a product.

## StrategyPlan Closure

A Plan is an immutable heterogeneous causal graph containing:

- desired conditions and exact satisfaction meaning
- zero or more independently complete Tasks
- zero or more bounded Epistemic Operations grounded from Curation contracts
- exact causal and information dependencies
- exact owner milestone requirements
- assumptions, unresolved observation paths, explanation, frozen context, and predecessor lineage

One Goal may produce several Tasks. Each Task closes Capability identities, bound inputs, artifact flow, internal executable dependencies, expected outcome, authority requirements, and idempotency independently. Execution receives one Task at a time after Agent authorization.

Each Epistemic Operation closes against the accepted `WMR-DD-02` grammar. Strategy binds its targets, cut, bounds, vocabulary, expected result, publication policy, and authority requirements. It cannot invent Curation relation meaning.

Every desired condition must be already satisfied, connected to a closed product and milestone path, decomposed into closed prerequisites, or explicitly unresolved through a bounded path that names what later evidence can resolve it. Ranking, cost, prospective evidence, or a likely consumer outcome never substitutes for closure.

## Plan Identity, Verification, And Judgment

Plan family identity binds Agent, Goal, and directive lineage. Desired conditions, Tasks, Epistemic Operations, and dependencies have semantic identities derived without the Plan revision. Plan revision identity then binds the complete construction request, selected semantic identities, policies, and predecessor. Each selection reference binds one semantic item back to the exact Plan revision without making identity derivation circular.

Equal inputs produce stable semantic products and ordering. Private exploration metadata may vary only when it cannot alter Plan identity, product meaning, dependencies, or explanation claims used for judgment.

Strategy verification checks grounding, product closure, causal coherence, milestone typing, dependency consistency, frozen-context integrity, uncertainty handling, and explanation lineage. Verification does not grant authority.

Agent then records one immutable Plan judgment:

- admitted as the current basis for progression
- rejected under directive, authority, risk, or policy judgment
- superseded by a named successor

Admitting a Plan authorizes no product. This preserves Agent judgment over the complete causal proposal while requiring a fresh, product-specific authority decision at the moment work becomes eligible.

## Product Eligibility And Authorization

Agent evaluates one product against:

- admitted current Plan revision
- exact dependency milestones accepted so far
- frozen source and assumption validity
- current directive, scope, perspective, branch, and authority
- activation generation and product idempotency
- conflicting, superseding, or already completed work
- one newly requested complete Planner assembly result for the same decision context

The Planner currentness check returns either a complete cut or an explicit refusal. Agent records the check identity with eligibility. Equal current and frozen cut identities pass the source-freshness fence. A different cut or refusal invalidates eligibility and causes a successor request or an explicit wait. This proof reuses Planner assembly and selects no notification or runtime topology.

An eligible state is evidence, not authority. Agent records a distinct product authorization before handoff.

For Curation, the authorization envelope carries the complete immutable Epistemic Operation and binds Agent, Goal, Plan revision, selection reference, operation identity, frozen context, authority scope, generation, and idempotency. `WMR-H07` closes only when Curation returns the durable acceptance or rejection position defined by `WMR-DD-02`. An Agent retry position proves only that the handoff remains durably in flight.

For Execution, the authorization envelope binds Agent, Goal, Plan revision, Task identity, frozen context, authority scope, generation, and idempotency. It contains the complete Task and no other Plan products or private Strategy state. `WMR-DD-03` closes this producer product and leaves Execution acceptance, validation, and realization to `WMR-DD-04`.

## Milestone-Driven Progression

Every dependency declares which owner milestone discharges it. The allowed semantic classes are Curation terminal result, Graph visibility, configured Belief revision, Agent acceptance, Execution admission, Task outcome, semantic-owner observation, and Goal disposition. Later slices may refine an owner position without replacing the class with a universal status.

Agent absorbs a milestone only when its owner product, position, Plan dependency, perspective, context, and generation match. It records the absorption before enabling dependent products.

Examples:

- an Epistemic Operation may enable a Task on a configured Belief revision rather than raw Curation terminality
- a Task may enable verification only after a returned workspace observation, not after Execution success
- Goal satisfaction may require an admitted docs correctness revision, not Task outcome or README presence alone

The Plan-local state named completed therefore always cites one exact milestone kind and owner position. It is never a transport-level generic.

## Reconstruction And Successors

A new admitted source revision, invalidated premise, changed authority, changed regime, owner result, or failed product may cause Agent to request reconstruction.

Strategy receives a new complete `PlannerCut`, the predecessor Plan, and completed causal history. It returns a successor Plan revision. Completed facts and owner results remain immutable history. Uncommitted products may be retained, replaced, added, or removed. Authorized or published products may be invalidated for future reliance, but their decisions and outcomes remain visible.

A semantically identical product may retain its semantic identity only when every condition, input, dependency, authority requirement, expected outcome, and milestone remains equal. The successor creates a new Plan selection reference, and any authorization binds that exact successor Plan revision. A successor cannot relabel an earlier outcome or change the milestone that justified an earlier authorization.

This transition is the integrated observable outcome `WMR-O28`. Its proof requires both predecessor and successor cut identities, stale-eligibility invalidation, immutable Plan lineage, preserved completed history, the new Agent judgment, and any later product authorization.

## Goal Disposition

Agent owns Goal lifecycle under the Goal's satisfaction semantics and admitted evidence.

If the initial cut already establishes the desired proposition, Strategy may return a Plan with no discharge product and explicit satisfied-condition evidence. Agent may then record satisfaction if the owning Goal contract accepts that evidence.

No Plan, product authorization, Curation terminal result, Event append, Task completion, or Agent milestone acceptance universally proves Goal satisfaction. The owning satisfaction contract cites exact admitted evidence.

## Lifecycle Account

Planner readiness requires every required source and assembly policy. Strategy readiness requires a ground Goal, complete cut, exact operation catalog, construction policy, and predecessor history. Agent readiness requires the Plan, directive, authority, generation, and relevant input positions.

Waits name exact missing sources, Planner currentness results, milestones, consumer receipts, authority, or successor conditions. Wakes name completion of the exact Planner reassembly request, owner result, consumer receipt, authority decision, deadline, or reconstruction request that can change eligibility. Native source changes reach Agent eligibility through the reassembled cut rather than an assumed direct notification.

Every Agent judgment, eligibility decision, authorization, handoff, and milestone acceptance is fenced by Agent, Goal, Plan revision, product, frozen context, branch, perspective, authority, and activation generation.

Restart resumes from durable Agent decisions, Plan progression positions, Planner currentness checks, input cursors, consumer receipts, and owner milestones. A historical Plan or authorization cannot become current merely because it is replayed.

Agent-local quiescence requires no eligible progression through the observed inputs and a durable retry or consumer-acceptance position for every published authorization. It does not imply Execution completion, returned observation, activation-wide quiescence, or safe retirement.

## Product Proof

### Already-correct README

The complete cut binds expected README F, observed README revision, source and README claims, Curation coverage products, configured Belief revisions, directive context, causal and Regime inputs, and construction catalogs.

If admitted owner evidence already satisfies the desired proposition, Strategy records the condition as satisfied and constructs no Task. Agent may close the Goal only through the docs-owned satisfaction semantics and exact admitted revision. A pre-existing Goal does not force action.

### Missing or incorrect README

The cut binds positive bounded non-realization or incorrectness evidence under exact observation scope. Strategy may construct:

```text
Epistemic Operation A
-> exact required-claim milestone
-> complete Task B
-> returned workspace and docs observation milestone
-> Epistemic Operation C
-> configured correctness Belief revision
-> Agent satisfaction judgment
```

Agent can authorize A first, wait for the exact milestone declared by the dependency, then authorize B. It does not send A, C, or the full Plan to Execution. `WMR-DD-04` closes Task B admission and the returned-observation path.

### Dependency security dissimilarity

The same Plan grammar may combine an assessment operation, inventory or advisory refresh Task, mitigation Task, owner observation, and verification operation. Each product retains inventory, advisory, applicability, reachability, mitigation, and verification owner meaning.

A scan outcome does not satisfy applicability. A mitigation Task outcome does not prove exposure ended. Agent progression waits for the exact owner observation, configured Belief revision, or Agent acceptance named by the Plan.

## Downstream Handoff

`WMR-DD-04` may rely on:

- one independently complete Task identity and body
- several-Tasks-per-Plan and several-Tasks-per-Goal cardinality
- exact Agent, Goal, Plan revision, product, context, authority, generation, and idempotency lineage
- a producer authorization decision distinct from Execution acceptance
- an exact expected executable outcome and declared later owner milestones
- a strict prohibition on heterogeneous Plan or Epistemic Operation delivery to Execution

`WMR-DD-04` must still define Execution-owned acceptance, freshness and shape validation, durable admission, Task Network lowering, outcome publication, semantic-owner observation return, and milestone evidence.

## Non-Goals

- runtime types, APIs, schemas, stores, actors, scheduling, or migration
- a language-layer Strategy Plan grammar
- Execution admission or internal realization design
- PDS compilation or Agent genesis
- root runtime topology, activation-wide lifecycle, or retirement
- compatibility with legacy Workflow or current single-Composition authorization
