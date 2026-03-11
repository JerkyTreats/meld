# Context Generate Integration

Date: 2026-03-09
Status: active

## Intent

Refactor `context generate` so it consumes ordered target plans and executes context generation as a primitive task family inside workflow orchestration.

## HTN Position

- `src/context` owns context artifact production, queue execution, frame persistence, and retrieval
- workflow owns recursive planning, ordering, cross task retry coordination, and downstream artifact handoff
- `context generate` becomes a compatibility entry that compiles into ordering plus generate primitive tasks
- context should consume workflow public contracts without regaining hidden ownership of global orchestration

## Provisional Answers

### What Remains In Context

- prompt and context artifact production remains in `src/context`
- queue lifecycle, provider calls, frame persistence, and retrieval behavior remain in `src/context`
- frame quality and metadata guarantees remain owned by `src/context`

### What Moves Out Of Context

- target expansion policy moves out of `src/context`
- ordering policy moves out of `src/context`
- cross task orchestration, publish handoff, and repair decisions move out of `src/context`
- recursive planning becomes a workflow concern that may invoke multiple primitive tasks in sequence

### Migration Stability

- recursive generation should stay stable through compatibility compilation rather than by preserving hidden orchestration inside `src/context`
- the current logic in `src/context/generation/run.rs` should be treated as transitional behavior to lift into workflow over time
- parity checks should compare old command behavior to the compiled workflow path before legacy ownership is removed

### CLI Predictability

- existing `context generate` CLI behavior should remain stable while workflow compiles the underlying task network
- command outputs should continue to summarize generation results even when the underlying execution path becomes workflow backed
- the CLI should not expose decomposition details by default, but those details should exist in workflow state and telemetry

## Initial Requirements

- keep context generation focused on context artifact production and retrieval
- preserve current recursive behavior through a compatibility adapter during migration
- allow workflow to submit target plans rather than force `context` to derive them
- preserve current frame generation quality and metadata guarantees
- keep `context` dependent only on workflow public contracts when running workflow backed execution

## Migration Boundary

`context generate` should become a strong primitive task consumer without remaining the hidden owner of global orchestration policy.

## Residual Questions

- how much target batching should remain inside context queue execution versus move into workflow task shaping
- whether first phase generation tasks should report only output artifacts or also richer observation summaries for later repair

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Ordering Capability](../ordering_capability/README.md)
- [Capability Contract](../capability_contract/README.md)
- [Migration Plan](../migration_plan/README.md)
