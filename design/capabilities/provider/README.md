# Provider Capability Design

Date: 2026-04-03
Status: active
Scope: provider domain refactor for capability-ready execution and service interaction

## Intent

Define the provider-domain boundary needed by the capability and task architecture.

The provider domain should own interaction with external model services.
That includes execution optimization for already-ready work.
It does not include task readiness, dependency order, or target derivation.

## Core Position

`provider` should own a `ProviderExecutor`.

That executor is a pure processor for provider-ready work.
Its job is to send requests to model services as efficiently and safely as possible.

That means batching and throttling belong here.
Those are transport and service-interaction concerns.
They are not task orchestration concerns.

## Upstream And Downstream Split

The intended split is:

- `context`
  prepares what to generate
- `task` or `control`
  decides what work is ready
- `provider`
  executes ready work against external providers

The rule is simple:

- upstream decides readiness
- provider decides execution strategy

## Definitive Boundary

### Provider Should Own

- provider binding resolution
- runtime override application
- client construction
- execution admission for ready requests
- compatibility grouping for batchable requests
- provider-lane throttling
- concurrency limits tied to provider service behavior
- retry and backoff policy for provider interaction
- response correlation to stable request identity
- provider-specific error normalization
- provider usage and timing capture

### Provider Should Not Own

- task dependency inspection
- graph readiness
- business priority policy
- target traversal
- frame persistence
- metadata construction for generated frames
- workflow branching
- orchestration policy above the ready set

## Contract Shape

Recommended first-slice contracts:

- `ProviderExecuteRequest`
  stable request identity, provider binding, prompt payload, execution hints, expected response shape
- `ProviderExecuteBatch`
  a provider-compatible set of execute requests
- `ProviderExecuteResult`
  stable request identity, normalized output, usage, timing, finish reason, provider metadata, normalized failure
- `ProviderExecutor`
  the domain contract that accepts ready requests and returns correlated results

The key property is stable request identity.
Provider execution may reorder or batch internally for efficiency.
Upstream layers still need deterministic correlation when results come back.

## Batching Rule

Batching is appropriate at the provider layer when requests share a provider-compatible execution class.

That compatibility class may include fields such as:

- provider name
- model
- endpoint family
- request mode
- runtime override fingerprint
- response contract shape

The provider executor may regroup ready requests within one compatibility class to reduce network round trips or improve backend utilization.

The provider executor must not invent new work or cross provider-incompatible boundaries just to increase batch size.

## Throttling Rule

Throttling also belongs at the provider layer.

Reason:

- provider rate limits are properties of external services
- provider concurrency ceilings are transport realities
- provider backoff policy depends on provider error classes

This means throttling should be keyed by provider-owned execution lanes rather than by context agent identity.

## Current Code Findings

The current codebase shows the need for this refactor clearly.

- [provider_execution.rs](/home/jerkytreats/meld/src/context/generation/provider_execution.rs)
  currently lives under `context` and owns provider preparation plus completion execution
- [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
  currently throttles by agent, which is the wrong long-term execution key for service interaction
- [generation.rs](/home/jerkytreats/meld/src/provider/generation.rs)
  already owns provider execution binding and runtime override contract types
- [provider.rs](/home/jerkytreats/meld/src/provider.rs)
  already owns provider clients, provider factory logic, and the registry

So the direction is not to invent a new concern.
It is to move existing provider-execution responsibility into the provider domain and make the contract provider-native.

## Refactor Direction

### Phase P0

Name the boundary and stop treating provider execution as context-owned behavior.

Required outcomes:

- `context` is understood as request preparation and result finalization
- `provider` is understood as service execution
- queue or control layers stop owning provider-specific execution policy

### Phase P1

Introduce provider-native execution contracts.

Required outcomes:

- provider execution no longer depends on `ContextApi`
- provider execution no longer depends on context queue event types
- provider execution accepts provider-ready request records and returns normalized results

### Phase P2

Move batching and throttling into provider execution.

Required outcomes:

- provider lanes are keyed by provider service identity
- rate limiting is provider-aware
- batching happens by provider compatibility class
- retries use provider-normalized failure classes

### Phase P3

Shrink context around the provider handoff.

Required outcomes:

- `context` prepares prompts and generation inputs
- `context` hands a provider-ready request to `provider`
- `context` consumes normalized results and persists generated frames when needed

## Relation To Context Refactor

This provider design is the missing counterpart to the context refactor.

The context refactor should not end at a single monolithic `context_generate` call that still performs transport work internally.
It should end at a provider handoff.

That implies this conceptual flow:

1. `context` prepares generation input
2. `provider` executes the ready request or batch
3. `context` validates and persists the generation result

This keeps `context` domain-pure while giving `provider` the responsibility to optimize interaction with model services.

## Relation To Task And Control

`task` and `control` decide what work is ready.
`provider` decides how that ready work is sent.

During the refactor window, `control` also owns batch barriers and batch release for current ordered execution flows.
`provider` remains below that line.

This preserves the intended architecture:

- capability contracts remain domain-owned
- task and control retain ordering authority
- provider owns service-execution optimization
- context stops carrying hidden orchestration and transport concerns

## Decision Summary

- add a provider-domain executor concern
- place batching in `provider`
- place throttling in `provider`
- keep readiness and dependency order outside `provider`
- refactor context so provider execution is a handoff, not an internal side path
