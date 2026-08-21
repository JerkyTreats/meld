# Agent Plan Progression

Plan progression is the Agent-owned reconciliation of an authorized Strategy Plan. The name Curation is reserved for the separate epistemic authorship domain.

## Progress State

Agent records the exact Plan revision and one state for each desired condition and product. Product states distinguish blocked, eligible, authorized, published, completed, invalidated, and superseded.

## Eligibility

A product becomes eligible when its dependency conditions are admitted as satisfied under the Plan's declared perspective and frozen assumptions remain valid. Agent evaluates eligibility from durable owner results rather than process-local callbacks.

## Publication

For an Epistemic Operation, Agent publishes an authorization envelope to Curation. For a Task, Agent publishes a Goal-attributed Task admission to Execution. Both publications carry stable idempotency and lineage keys.

## Reconciliation

Agent reassesses progression after every relevant admitted fact. It never treats command acceptance as semantic completion. Curation completion, execution completion, observation, belief settlement, and Goal satisfaction remain distinct facts.

When a premise changes, Agent requests a successor Plan and preserves already completed causal history. It does not ask Execution to interpret or revise the Strategy Plan.
