# Execution Goal Set

The Goal Set is Execution's public intake and lifecycle boundary for executable obligations. It stores Goal-attributed complete Tasks and their execution-facing state.

A Goal remains a desired state owned by its originating authority. Execution stores enough Goal identity and attribution to rank work, report outcomes, and support curation. It does not own the full Strategy Plan or universal Goal satisfaction.

## Admission

Each admission carries a stable admission identity, Goal identity, Agent or producer identity, one complete Task, authority, priority, idempotency key, and lineage.

Several Task admissions may contribute to one Goal over time. One Task may also be cohered with compatible work that contributes to several Goals. Admission identity therefore remains distinct from Goal identity and Task identity.

## Admission Lifecycle

Authorized producers may add, suspend, resume, withdraw, or supersede their admissions. Execution preserves completed effects and protects its own in-flight state. A producer cannot mutate another producer's authority or Execution-owned history.

## State

Execution-facing state distinguishes admitted, blocked, planned, active, completed, failed, withdrawn, and superseded. These states describe executable processing. They do not declare epistemic Goal satisfaction.

## Coherence

The Goal Set is the input universe for Execution Planning. Compatible admissions may share operational work when Capability contract, inputs, revision scope, effects, authority, and result attribution permit reuse.
