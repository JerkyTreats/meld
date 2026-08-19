# Portfolio Thesis And Risk Router Pressure Specification

Date: 2026-08-18

Status: discovery router exercise aligned to canonical PDS boundary

Scope: fan portfolio thesis and risk stewardship through the proposed `theory::router` contracts without treating PDS as a trading authority

## Purpose

Test exact semantic routing, external system-of-record boundaries, confidential assignment isolation, coherent source snapshots, and strict separation among research belief, recommendation, approval, and execution.

## Layering

The principal approves portfolio and thesis scope, mandate constraints, research freshness, recommendation autonomy, and verification posture. The package compiles those selections into exact owner components. Assignment binds principal, legal or advisory perspective, scope, and grants. Activation supplies data, provider, calculation, and notification bindings. Positions, orders, executions, market data, thesis revisions, and later outcomes remain runtime or external state.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Owner meaning | Structural requirements |
| --- | --- | --- | --- |
| `portfolio-policy` | `portfolio.policy.v1` | portfolio identity, mandate source, instrument lineage, coherent snapshot, thesis and recommendation policy | none |
| `thesis-support-belief` | `world-model.belief-family.v1` | support for declared thesis claims | `portfolio-policy` |
| `invalidation-belief` | `world-model.belief-family.v1` | explicit invalidation and counterevidence risk | `portfolio-policy` |
| `exposure-belief` | `world-model.belief-family.v1` | exposure under an exact portfolio and pricing basis | `portfolio-policy` |
| `liquidity-belief` | `world-model.belief-family.v1` | liquidity risk under declared horizon and method | `portfolio-policy` |
| `freshness-belief` | `world-model.belief-family.v1` | decision relevance and source freshness | `portfolio-policy` |
| `portfolio-outcomes` | `world-model.outcome-mapping.v1` | admitted research, recommendation, approval, execution, and later outcome evidence | all belief components and `portfolio-policy` |
| `portfolio-curation` | `world-model.agent-curation-rule.v1` | research, tolerate, recommend, review, or escalate posture | all belief components |
| `portfolio-condition` | `world-model.agent-maintained-condition.v1` | current evidence, visible invalidation, coherent risk, and authority separation | `portfolio-curation` and all belief components |
| `portfolio-strategy` | `world-model.strategy-theory.v1` | settlement, prospective evidence, research action, outcome, and constraint meaning | `portfolio-condition` and `portfolio-policy` |
| `portfolio-capability-*` | activation capability contribution | exact contracts for source retrieval, snapshot read, calculation, recommendation, or review request | compatible portfolio action classes |
| `portfolio-authority` | assignment governance selection | requested research and recommendation actions with explicit execution denials | `portfolio-policy` |

`portfolio.policy.v1` is a candidate owner route. It names meaning that cannot safely be inferred by a generic router, especially system-of-record roles, snapshot coherence, mandate revision, and recommendation versus order intent.

The default activation profile does not offer an order-submission capability. A future separately governed assignment and activation could offer one exact contract, but capability closure and external control would still be required.

## Owner Semantics And Cross Links

The portfolio owner defines portfolio, account, instrument, thesis, mandate, snapshot, and recommendation semantics. Source adapters report custodian, broker, compliance, filing, and market claims without becoming their owner. The belief owner hosts exact belief revisions. Strategy selects among research and bounded action meanings. Execution owns contracts, operation identity, effect claims, and dispatch fences.

Required public cross-links include:

- belief families cite exact portfolio dimensions and evidence classes
- exposure results cite an exact calculation method and coherent input set
- maintained conditions cite current mandate and source-freshness rules
- Strategy cites exact capability contracts and cannot invent order authority
- outcome mapping distinguishes recommendation, approval, external execution, and realized observation
- authority policy names explicit denied and approval-gated action classes

The router proves that refs close. Owners decide what a position snapshot, mandate rule, or invalidation condition means.

## Assignment And Activation

A candidate assignment binds:

```text
exact package receipt
+ portfolio or thesis scope
+ principal and legal or advisory perspective
+ branch and decision horizon
+ requested authority ref
+ exact principal grant ref
```

Custodian account ids and current credentials are activation bindings, not package identity. Distinct assignments over one portfolio remain valid and cannot gain precedence from declaration order.

Candidate activation bindings include primary-document access, market and reference data, custodian or accounting read access, approved risk calculators, research provider, compliance query endpoint, and notification sink. Each owner sees only declared refs. Confidential holdings must not leak into a shared research or model adapter through ambient product access.

Preparation installs no live invoker. Readiness must prove authenticated source access, exact selected implementation identity, source-cursor recovery where relevant, and closed admission before the generation can become current.

## Capability And Authority Closure

Research retrieval and recommendation require exact contract selection, an approved implementation, principal grant, current policy, and dispatch-time authority. Portfolio read access is distinct from permission to transmit holdings to another provider.

Any future order request adds stronger closure:

```text
exact order-intent capability
+ mandate-compatible proposed effect
+ current compliance decision
+ principal or delegated approval
+ external venue controls
+ durable idempotent operation key
+ authoritative order and execution reconciliation
```

The existence of broker credentials, a trading client, or a favorable belief grants none of these.

## Observation And Result Admission

Request-response research or calculation carries assignment, exact package receipt, activation generation, participant incarnation where available, capability ref, durable operation key, claim, and attempt id.

Persistent market, custodian, or compliance feeds use admitted subscription, source sequence, delivery id, authentication lineage, assignment, activation generation, and participant incarnation. They do not fabricate execution claims.

The portfolio owner validates scope, account and instrument identity, source role, revision, timestamps, corporate-action basis, currency and units, completeness, and snapshot coherence before canonical admission. Derived risk additionally cites exact method and inputs.

External execution reports remain claims from the authoritative venue or order system. Meld may reconcile them but cannot create or correct an execution merely through an internal belief update.

Late data may alter historical interpretation without being current enough for a new recommendation. A source correction must preserve supersession rather than rewrite decision-time evidence.

## Isolation And Lifecycle Pressure

Portfolio stewardship requires strong authority, binding, state, admission, effect, and replay isolation. Shared public filings and market caches may be acceptable. Holdings, mandates, recommendations, credentials, approvals, and order intent require tighter namespace and disclosure controls.

One assignment can remain actively idle while waiting for a filing, mandate change, market session, or review. Every wait requires a durable wake path. A healthy data process with a lost portfolio subscription is braindead, not quiescent.

Credential or endpoint changes should advance assignment-wide activation generation because they change the realization. A mechanically equivalent participant restart may only need a new participant incarnation after admission-closed reconciliation. This refinement remains unresolved across the current proposal set.

An ambiguous external action cannot be retried under a fresh semantic identity. It retains the durable operation key and either reconciles authoritative venue state or dispatches a new attempt under explicit idempotency rules.

## Interaction With Other Stewards

Potential peers include compliance, tax, treasury, liquidity, research, and execution stewards. They communicate through explicit contracts such as a current restriction claim, review request, approved recommendation, or authoritative execution observation.

The thesis steward must not inspect another steward's private store or treat that steward's recommendation as an execution grant. Conflict among mandate, liquidity, and thesis objectives remains visible and goes through declared governance rather than package order.

## Router Falsification Findings

The example falsifies the design if:

- the router interprets instruments, mandates, thesis claims, or risk methods
- a current package head reinterprets an old recommendation
- individually valid but incoherent snapshots produce an authoritative clean posture
- capability availability or broker binding implies trade authority
- a shared service crosses confidential portfolio or grant boundaries
- transport success becomes an execution fact
- later profit is treated as proof that a prior thesis was correct
- historical replay depends on a live market endpoint or hidden provider state
- retry after ambiguous order effect can duplicate economic intent

The example narrows to conventional research, risk, optimization, and workflow tools if PDS adds no measurable decision reconstruction or adaptive evidence value.

## Theory-Set Implications

### Common Candidate Mechanism

Exact package receipts, assignment authority, activation-local catalogs, source and operation lineage, owner admission, conditional generation publication, and outcome records remain common. This example especially supports generic source-coherence references and effect reconciliation without asking the router to understand finance.

### Owner-Specific Meaning

Instrument identity, portfolio snapshot coherence, mandate semantics, thesis invalidation, risk calculation basis, liquidity horizon, recommendation meaning, and execution-system authority belong to portfolio and external domain contracts.

### Unresolved Bridge

The theory set does not yet settle:

- whether `portfolio.policy.v1` is one route or several domain-owned facets
- how independent source revisions form a coherent decision-time view
- where confidential-data disclosure authority intersects capability authority
- how external approval and compliance receipts join effective authority
- how participant incarnation relates to activation generation
- which corrected or late market observations may update current versus historical projections
- the generic boundary between effect idempotency and source-of-record reconciliation

## Source Trail

- [Use-case narrative](README.md)
- [PDS Router design](../../../completed/integration/pds_router_design_spec.md)
- [Isolation and runtime portability](../../isolation_and_runtime_portability.md)
- [Runtime lifecycle and quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
- [Use-case decomposition](../../use_case_decomposition.md)
- [Compilation layer span](../compilation_layer_span.md)
