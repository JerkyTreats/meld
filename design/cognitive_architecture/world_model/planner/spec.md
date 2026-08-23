# World Model Planner Specification

Status: canonical evergreen design

Scope: immutable reasoning cuts for Agent and Strategy

## Purpose

Planner creates one complete and replayable source-consistency boundary for a bounded reasoning question. Its canonical root product is `PlannerCut`.

Planner is a projection domain inside the world model. It assembles exact native-owner revisions. It does not select actions, construct a Strategy Plan, authorize work, mutate a Goal, or produce Execution Planning input.

## Ownership

| Concern | Owner | Planner relationship |
| --- | --- | --- |
| graph objects and relation occurrences | Graph and Traversal | consume one exact `TraversalCut` |
| evidence settlement | Belief | bind exact immutable revisions |
| mechanisms and expected effects | Causation | bind exact causal revisions |
| changing operating conditions | Regime | bind exact regime revisions |
| directive and maintained condition | Agent and directive owner | bind exact context |
| Capability contracts | Capability owners and catalog authority | bind the constructible catalog revision |
| Epistemic Operation contracts | Curation | freeze the available operation catalog beside the cut |
| source consistency and projection | Planner | own assembly, refusal, identity, and derived views |
| Plan construction | Strategy | consume the complete frozen context |
| Plan judgment and progression | Agent | decide what may proceed |
| operational lowering | Execution | remain outside Planner |

## Assembly Request

A `PlannerCutAssemblyRequest` names:

- one decision context and ground Goal reference
- Traversal roots, allowed relation families, direction, and bounds
- temporal, transaction, horizon, branch, subject, and exclusion scope
- Agent perspective and authority scope
- required Belief, Causation, and Regime source families
- required Capability catalog position
- projection and completeness policy revision
- activation generation and request idempotency identity

The request declares completeness before source selection. A caller may request a deliberately narrow cut. Planner cannot silently narrow a broader request because a source is absent or expensive.

## Source Inventory

| Source | Required position | Invalid position |
| --- | --- | --- |
| graph structure | exact `TraversalCut`, frontier, provenance, owner revisions, and truncation state | missing owner revision, unacceptable truncation, or scope conflict |
| Belief | exact revision set with evidence and invalidation lineage | missing, stale, conflicted, or perspective-mismatched revision |
| Causation | exact claim, mechanism, effect, and assumption revisions required by policy | unidentified or unavailable required mechanism |
| Regime | exact posterior, changepoint, segment, mixture, and stress revisions required by policy | missing or policy-incompatible regime input |
| directive context | exact directive and maintained-condition revision | expired, superseded, or out-of-scope context |
| Capability catalog | exact catalog and contract revisions visible to Strategy | missing body or incompatible revision |
| scope and time | exact subject, branch, temporal, transaction, horizon, and exclusion policy | incoherent or unsupported scope |
| perspective and authority | exact Agent lens, grants, and activation generation | absent, stale, or incompatible authority |
| projection policy | exact selection, hydration, risk, conflict, and completeness policy | unknown or non-replayable policy |

The Curation operation catalog and Strategy construction policy are frozen beside the `PlannerCut` in the Strategy construction request. They define available means rather than observed world state.

## PlannerCut

A `PlannerCut` binds:

- cut identity and assembly request identity
- one exact occurrence-rich `TraversalCut`
- exact Belief revisions and invalidation conditions
- exact Causation and Regime revisions required by policy
- exact directive and maintained-condition revisions
- exact Capability catalog and contract revisions
- temporal, transaction, horizon, branch, subject, exclusion, and perspective scope
- Agent lens, authority grants, and activation generation
- projection and completeness policy revision
- hydration handles pinned to exact owner revisions

The cut is immutable. Equal normalized inputs and equal policy produce equal cut identity. A changed source, scope, authority fence, generation, or policy produces a successor cut.

## Traversal

Planner uses Traversal to discover relation-rich context from the declared roots. The query preserves exact relation occurrences, path lineage, owner qualification, currentness, and frontier state.

Traversal resource exhaustion is visible. A truncated result satisfies the request only when the request policy explicitly permits that exact truncation class. Otherwise Planner refuses assembly.

Planner never treats graph reachability as evidence. It binds owner publications and Belief revisions without replacing their semantic authority.

## Derived WorldModelView

`WorldModelView` is an optional decision-shaped projection under one complete `PlannerCut`. It may organize actionable beliefs, risks, conflicts, assumptions, sensitivities, and pinned hydration handles for one scoped question.

```text
PlannerCut
-> zero or more WorldModelView projections
```

The view cannot choose a different source set, advance a cursor, weaken cut completeness, or replace the cut identity in Strategy lineage. Equal cut identity, decision context, and projection revision yield equal view meaning.

## Assembly Refusal

Planner emits either one complete `PlannerCut` or one `PlannerCutRefusal`.

A refusal names every known source that is:

- missing
- stale
- conflicted
- unauthorized
- out of scope
- unacceptably truncated
- unavailable under the required projection policy

Planner does not emit a partial cut as complete. Strategy cannot repair cut consistency through live reads or implicit omission.

## Strategy Construction Handoff

Agent supplies Strategy with:

- one ground Goal and Agent identity
- one complete `PlannerCut`
- exact Curation operation catalog revision
- exact Strategy construction and verification policy revision
- predecessor Plan and completed causal history when reconstructing
- pure search and explanation bounds

Strategy may hydrate only through handles already pinned to exact owner revisions. It cannot issue live reads, advance a cursor, or wait for runtime state while constructing the Plan.

The `StrategyPlan` cites the `PlannerCut` as its frozen context root. Optional `WorldModelView` products remain derived evidence within that root.

## Currentness Check

Before Agent authorizes an eligible Plan product, it may request a fresh assembly for the same decision context.

Equal frozen and current cut identities satisfy the source-currentness fence. A successor cut or refusal invalidates eligibility and gives Agent evidence for a wait, suspension, or successor Strategy request.

Planner performs the reassembly. Agent owns the comparison decision. Strategy does not mutate the old Plan. Execution never receives either cut.

## Identity And Replay

Durable Planner products distinguish:

- assembly request identity
- `TraversalCut` identity
- native source revision identities
- `PlannerCut` identity
- derived `WorldModelView` identity
- currentness-check identity owned by Agent

Replay resolves the exact source revisions and policy named by the cut. A missing historical source or incompatible schema produces an explicit unavailable result. Replay cannot substitute current data for a historical revision.

## Lifecycle

Planner is ready when every source class and assembly policy required by the request can be resolved.

A wait names exact missing source positions, owner revisions, projection coverage, policy, authority, or generation. Its wake names the durable successor that can change assembly eligibility.

Every cut and refusal is fenced by request, scope, branch, perspective, authority, activation generation, and projection policy.

Planner-local quiescence means no accepted assembly request is eligible through the observed source positions and every unresolved request has a durable wait with a viable wake. It says nothing about Strategy completion, Agent progression, Execution, or activation-wide quiescence.

## Public Operations

| Operation | Result |
| --- | --- |
| `assemble_planner_cut` | complete `PlannerCut` or explicit `PlannerCutRefusal` |
| `project_world_model_view` | derived `WorldModelView` or projection refusal under one cut |
| `resolve_planner_cut` | exact stored or reconstructed cut account |
| `compare_cut_currentness` | structural equality or successor evidence for Agent judgment |

Names describe contract roles. They do not require these exact Rust identifiers.

## Invariants

- one complete cut is the root source-consistency identity for Strategy
- every source retains native-owner identity and semantic authority
- relation occurrence identity and provenance survive projection
- scope, time, branch, perspective, authority, and generation are explicit
- incomplete or conflicting source state produces refusal
- derived views cannot weaken or replace cut completeness
- equal normalized inputs and policy produce equal identity
- Strategy receives frozen context and never repairs it through live reads
- Agent owns currentness judgment and successor requests
- Execution receives no Planner product

## Non Ownership

Planner does not own Strategy search, Plan verification, Agent authority, Goal satisfaction, Curation authorship, Belief settlement, Capability registration, Task admission, Task Network state, dispatch, or runtime supervision.

## Read With

- [Planner Overview](README.md)
- [Graph And Traversal](../graph/README.md)
- [Belief](../belief/README.md)
- [Causation](../causation/README.md)
- [Regime](../regime/README.md)
- [Strategy](../strategy/README.md)
- [Agent](../agent/README.md)
- [World Model Public Interface](../public_interface.md)
