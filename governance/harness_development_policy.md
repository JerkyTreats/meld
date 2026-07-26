# Harness Development Policy

Date: 2026-07-26
Status: active
Scope: use of the interactive runtime harness — session boots, walks, waiting-on declarations, the served substrate, and external consumers — during development

## Intent

The harness exists so runtime behavior is diagnosed from durable evidence rather than reproduced by hand or worked around with test orchestration. This policy fixes the rules that keep it truthful: how sessions boot, how stalls are observed, what the served boundary guarantees, and what a consumer may assume.

The delivery record lives in [Runtime Harness Plan](../design/plan/integration/runtime_harness_plan.md); the frozen requirements register lives in [Agent-Native Debugger Requirements](../design/plan/integration/agent_native_debugger_requirements.md). This policy governs usage, not delivery.

## The Substrate Boundary

- Meld ships no visualizer. The product surface ends at the served, machine-readable `/v1` contract over loopback HTTP, whose JSON bodies are the existing contract types verbatim.
- Every consumer — a terminal client, a dashboard, the t3code preview application — is external and non-authoritative. A consumer that renders what the substrate does not serve has a consumer defect. A consumer need is never an argument for widening the substrate.
- External consumers use only the published contract. No private imports from product crates, no direct store opens, no parsing of prose fields as if they were identities.
- Live and playback surfaces are byte-consistent: a sealed session replayed through the same handlers serves the same bytes. A divergence is a harness defect, never something a consumer compensates for.

## Session Discipline

- Every session boots through the product staged initialization pipeline, scoped to its registration subset. A harness that initializes differently from the product debugs a system that does not exist.
- The default boot lands in a temporary root. Pointing a session at an existing product data root requires the explicit unsafe flag, and the caller owns the consequences of a reused root.
- A session is an artifact. Bug reproduction is a sealed manifest — stimuli, step schedule, closing watermarks — not a prose recipe. Hand a manifest, not instructions.
- Steps take injected time and bounded budgets. No wall clock inside a session.

## Observation Discipline

- Stimuli enter only through the canonical event append and public domain command ports. The harness never pokes stores, never manufactures records, never forces cursor advances.
- A stall observed through the harness is a finding to diagnose at root cause. Working around a stall with synthetic inputs, seeded state, or test orchestration is prohibited — the specimen tests exist precisely because the happy-path fixture hid the stall.
- The walks are substrate derivations, never consumer-side logic, so independent consumers cannot diverge on an answer. The provenance walk explains why a record exists; the eligibility walk explains why a record does not. Absence is an answer: unresolvable references surface as truthful cuts distinguishing an absent record from an out-of-scope store.

## Waiting-On Declarations

- Emission is observational and derives only from state the tick already computed. A declaration never gates, reorders, or fails semantic work.
- Whenever a selector finds nothing eligible, the report says what would change that. Quiet is a statement, not an omission.
- Condition vocabulary is domain-owned and frozen as exported constants in each domain's `waiting` module. Emitters and consumers compile against the same constant, so a rename breaks the build instead of silently changing the substrate's answer. New conditions land as new constants with rustdoc stating what would make the work eligible.

## Durable Encoding Rules

- The report store encoding is bincode, which is not self-describing. New fields on durably stored report records append at the end only, and every shape change ships a frozen compatibility mirror plus a test that decodes real pre-field bytes. The legacy fallback rejects trailing bytes so a tail-localized decode failure can never be served as a legacy record.
- Served report reads fence to one session by sequence floor. A reused product root never serves a previous boot's declarations as the current run's state. Only a sealed playback mount lifts the fence, because a sealed session's retained history is the session.

## Runtime Corrections Under The Harness

- Corrections to runtime behavior — the flywheel-ignition lane and its successors — are validated live through a harness session before fresh review. The exit evidence is the gap visibly clearing in the served projections, not a unit test alone.
- The bounded convergence proof runs with the harness attached; the harness observes and never drives semantic work.

## Review Guidance

- Findings that request productization around the harness — an in-product dashboard, a resident daemon, a server-side condition registry, authentication layers on the loopback substrate — are disposed under the freeze principle: durable primitives, not enterprise-grade productization.
- Findings that a projection presents state the durable records cannot confirm are blocking. Truthfulness is the harness's one non-negotiable.

## Related Documentation

- [Runtime Harness Plan](../design/plan/integration/runtime_harness_plan.md)
- [Agent-Native Debugger Requirements](../design/plan/integration/agent_native_debugger_requirements.md)
- [Storage Substrate Decision Record](../design/plan/integration/storage_substrate_decision_record.md)
- [Commenting Policy](commenting_policy.md)
