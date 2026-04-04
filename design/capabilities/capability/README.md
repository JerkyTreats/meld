# Capability 

![Screenshot](./screenshot-2026-03-30_05-46-06.png)

A capability is a domain-owned contract for one typed unit of behavior.
It is the durable boundary between domain functionality and orchestration.
In HTN language, it is closest to an atomic task, but the durable abstraction here is the contract, not the executor internals.

An example is `context_generate` as pure functionality.
To surface that functionality to task compilation and later control execution, the domain publishes an explicit capability contract with stable identity, scope, bindings, inputs, outputs, and effects.

The concrete capability still lives under the owning domain, such as `context`.
The common shape described here gives `capability`, `task`, and `control` one shared vocabulary.

## Contract Shape

The first slice should treat a capability contract as a declarative record.
That record should be rich enough for graph compilation and simple enough to stay above function-signature details.

```rust
struct CapabilityTypeContract {
    capability_type_id: CapabilityTypeId,
    capability_version: CapabilityVersion,
    owning_domain: DomainId,
    scope_contract: ScopeContract,
    binding_contract: Vec<BindingSpec>,
    input_contract: Vec<InputSlotSpec>,
    output_contract: Vec<OutputSlotSpec>,
    effect_contract: Vec<EffectSpec>,
    execution_contract: ExecutionContract,
}
```

### Identity

- `capability_type_id`
- `capability_version`
- `owning_domain`

Identity must be stable across compiled tasks and execution records.
Versioning belongs on the contract, not buried in implementation details.

### Scope Contract

- `scope_kind`
- `scope_ref`
- scope validation rules
- optional scope fan-out policy

Scope tells orchestration where the capability is valid.
Examples include workspace, node, thread, turn, frame, or Merkle tree scope.

### Binding Contract

- required named bindings
- binding value kind such as literal, config ref, policy ref, agent ref, or provider ref
- validation rules
- whether the binding affects deterministic identity

Bindings cover non-artifact inputs that still matter to execution and compiled task identity.
Examples include provider choice, agent choice, generation policy, and traversal strategy.

### Input Contract

- input slot id
- accepted artifact type ids
- artifact schema version range
- required or optional
- one or many cardinality

Input slots are how upstream outputs become downstream requirements.
This is the part task compilation and later control execution use for artifact handoff validation.

### Output Contract

- output slot id
- artifact type id
- artifact schema version
- guaranteed or conditional

Output slots are explicit produced artifacts.
They replace hidden return-shape assumptions.

### Effect Contract

- effect kind such as read, write, append, emit, or acquire
- effect target such as active head, frame store, workflow state, or telemetry bus
- exclusivity rule when ordering matters without artifact flow

Not every dependency is an artifact dependency.
For score 5 and 4 runtime capabilities, effect metadata is what lets task compilation and later control ordering reason about writes such as `FrameWrite`, `HeadSet`, and `WorkflowStateWrite` without leaking storage internals into the graph.

### Execution Contract

- execution class such as inline, queued, or session-scoped
- completion semantics
- retry class
- cancellation support when relevant

This section describes execution-facing behavior at the contract level.
It should not expose adapter internals or transport details.

## Appropriate Abstraction Level

The contract should describe what orchestration must know, not how the domain code is implemented.

- good level: typed scope, typed bindings, typed input slots, typed output slots, declared effects
- too low: raw function signatures, adapter helper objects, storage structs, transport handles
- too high: one vague `run` shape that hides artifact compatibility and ordering rules

For the first slice, score 5 and 4 capabilities should converge on this one shared shape.
That gives task compilation and later control execution one uniform model for the runtime critical path without forcing lower-value admin capabilities into the same maturity level on day one.

## Runtime Note

Concepts such as trigger, payload manager, function adapter, and output manager can still exist as implementation helpers inside a domain.
They should not be the durable contract surface consumed by task compilation or control.

## See Also

- [Capabilities by domain](by_domain.md) — section-per-domain sketch of future `src/<domain>/capability.rs` surfaces
- [Domain Architecture](../domain_architecture.md) — current module boundary direction for capability and task
- [Control Design](../../control/README.md) — higher-order owner of plan and graph execution
