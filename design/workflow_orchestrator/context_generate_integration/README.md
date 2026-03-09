# Context Generate Integration

Date: 2026-03-09
Status: active

## Intent

Refactor `context generate` so it consumes ordered target plans and executes context generation as one capability inside workflow orchestration.

## Primary Questions

- what orchestration logic must remain in `context`
- what planning logic should move out of `context`
- how does recursive generation stay stable during migration
- how does existing CLI behavior remain predictable while orchestration shifts upward

## Initial Requirements

- keep context generation focused on context artifact production and retrieval
- preserve current recursive behavior through a compatibility adapter during migration
- allow workflow to submit target plans rather than force `context` to derive them
- preserve current frame generation quality and metadata guarantees
- keep `context` dependent only on workflow public contracts when running workflow backed execution

## Migration Boundary

`context generate` should become a strong capability consumer without remaining the hidden owner of global orchestration policy.

## Related Areas

- [Ordering Capability](../ordering_capability/README.md)
- [Capability Contract](../capability_contract/README.md)
- [Migration Plan](../migration_plan/README.md)
