# Portfolio Thesis And Risk Steward

Date: 2026-08-15

Status: discovery example

Scope: persistent research thesis, invalidation, mandate, exposure, liquidity, and interacting risk stewardship

## Working Intent

Maintain reconstructable research theses and a current risk view for an assigned portfolio while keeping belief, recommendation, approval, and execution authority separate.

The strongest initial expression supports research and risk stewardship. Autonomous live trading is deliberately outside the initial fit claim.

## Domain Boundary

The bounded domain contains:

- declared portfolios, accounts, positions, exposures, and mandate constraints
- research theses, claims, evidence, counterevidence, and invalidation conditions
- assets, issuers, risk factors, liquidity questions, and decision horizons
- recommendations, approvals, and prospective outcome records

The portfolio custodian, accounting platform, broker, order management system, and compliance system remain external systems of record for positions, cash, orders, executions, restrictions, and approvals within their authority. Primary filings and published records remain source claims under their own revisions. Market and reference-data vendors own their delivered observations, not Meld conclusions.

Meld may preserve admitted snapshots, form research beliefs, calculate derived exposure through an owner-approved method, and record decisions. It must not overwrite authoritative position or execution state with an inferred projection.

Assignments may overlap one portfolio under distinct mandates, principals, legal entities, research perspectives, or advisory roles. Shared market data does not merge confidential holdings, mandate constraints, grants, or current recommendations.

## Standing Condition

The candidate maintained condition is:

```text
decision-relevant thesis evidence is current enough for the declared horizon
and invalidating evidence remains visible
and position, exposure, liquidity, and mandate evidence has coherent source lineage
and research confidence never substitutes for execution permission
```

The condition can be breached by stale research, unresolved counterevidence, incoherent portfolio snapshots, mandate pressure, or insufficient liquidity evidence. Restoration requires admitted evidence and policy evaluation, not a favorable price move.

## Observations And Evidence

Candidate observations include filings, issuer publications, primary research, counterevidence, market and reference data, custodian positions, cash, orders, executions, mandate revisions, compliance restrictions, and risk measurements.

Admission should preserve:

- source identity and licensing or access class
- instrument, issuer, portfolio, and account scope
- publication, effective, observation, and receipt time
- source revision or market-data sequence
- position and cash snapshot identity
- pricing and reference-data basis
- calculation method and input lineage for derived risk
- confidence, contradiction, and correction lineage

A clean risk result built from mismatched position and market snapshots is not authoritative merely because every input was individually valid. The owner needs either a declared coherent cut or an explicit bounded mixed-snapshot uncertainty state.

Research summaries are derived artifacts. They never replace primary evidence. Later price movement is evidence about realized outcomes, not retrospective proof that a thesis was epistemically sound when formed.

## Observation And Intervention Choices

The steward may choose to:

- retrieve primary evidence
- inspect counterevidence
- refresh a stale source
- reconcile instrument or issuer identity
- acquire a coherent portfolio snapshot
- recalculate exposure or liquidity under a declared method
- update thesis state
- draft or revise a recommendation
- request compliance or principal review
- tolerate a declared risk within policy

A separately governed deployment might submit a bounded action request to an execution system. That is a distinct capability, authority, approval, and result-admission path. This example does not make it part of the default steward.

## Outcome Semantics

Recommendation completion does not prove thesis quality or portfolio improvement. Outcomes may arrive after days or years and are confounded by market regime, position sizing, execution quality, external cash flows, later decisions, and events outside the thesis.

The outcome model should preserve prospective decision-time evidence, the recommendation actually made, any approval and execution records from their systems of record, later portfolio observations, and explicit attribution limits.

A profitable outcome does not erase ignored invalidating evidence. An adverse outcome does not by itself prove that a mandate-compliant risk reduction was wrong. Outcome learning must not reward unsafe authority expansion.

## Authority And Safety

The principal may authorize research retrieval, risk calculation, recommendation drafting, and review requests. Access to portfolio state is itself scoped sensitive authority.

Order submission, trade approval, exception approval, leverage changes, cash movement, and mandate modification require separately declared grants and external controls. Package installation, capability discovery, or access to broker credentials grants none of them.

Any future live-action path should preserve independent compliance checks, external limits, approval state, idempotent order intent, ambiguous-effect reconciliation, and authoritative execution reports. The PDS belief engine is not the final risk control or ledger.

## Why PDS

A research database, risk engine, optimizer, fixed rebalance rule, and alerting system remain the credible baseline.

PDS is justified only if it adds reconstructable decision-time theses, counterevidence and belief decay, active research choice, policy-aware tolerate or escalate behavior, separation of belief from action authority, and prospective outcome history. It does not claim predictive edge. If a deterministic optimizer and governance workflow provide equivalent results, they remain preferable.

## Physical Runtime Variants

The same package could activate through:

- an in-process deterministic exposure calculator
- a subprocess for licensed research or risk analysis
- a shared sidecar for market and reference data
- remote research and model providers
- persistent custodian, compliance, or portfolio-monitor connectors

Every placement must preserve confidential assignment scope, exact source revisions, owner-scoped credentials, request and delivery lineage, and destination-domain admission. Shared market data may be safe while shared holdings or recommendations are not. A dedicated process does not prove credential, source, or tenant isolation.

## Explicit Non Commitments

This example does not commit:

- autonomous live trading
- a prediction model or source of investment edge
- one portfolio accounting or risk methodology
- a universal instrument ontology
- one market-data vendor or broker
- a final profile syntax
- a specific runtime placement
- causal attribution from one recommendation to later return
- Meld as a custodian, broker, compliance ledger, or authoritative book of record

The companion route map remains a discovery artifact over proposed contracts.

## Read With

- [Router pressure specification](router_spec.md)
- [Expression catalog](../pds_expression_catalog.md)
- [Compilation layer span](../compilation_layer_span.md)
- [Use-case decomposition](../../use_case_decomposition.md)
- [PDS Router design](../../../completed/integration/pds_router_design_spec.md)
- [Isolation and runtime portability](../../isolation_and_runtime_portability.md)
- [Runtime lifecycle and quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
