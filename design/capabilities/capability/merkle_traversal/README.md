# Merkle Traversal Capability

Date: 2026-03-28
Status: active

## Intent

Define the capability that derives a definitive ordered Merkle node set from a Merkle scope and a traversal strategy.

## Why This Exists

Current `context generate` behavior still carries tree ordering logic inside the `context` domain.
That mixes traversal derivation with context generation execution.

This capability makes traversal derivation explicit and separate.

## Functional Definition

`merkle_traversal` takes a Merkle tree scope plus traversal bindings and emits a typed ordered Merkle node set artifact.

It does not generate context.
It does not write files.
It does not execute downstream capabilities.

## First Slice Design Decisions

The user-facing capability input must carry an explicit `traversal_strategy` field.
The first slice accepted values are `bottom_up` and `top_down`.

Internally, the first slice should represent strategy as a typed enum rather than as open-ended runtime polymorphism.
That keeps the contract explicit while keeping the implementation simple.

The output artifact for the first slice is `ordered_merkle_node_set`.
That artifact is the stable output contract consumed by downstream capabilities.

## Inputs

The first slice input contract includes tree scope reference, target selection artifact, `traversal_strategy`, and traversal policy binding.

## Outputs

The first slice output contract includes `ordered_merkle_node_set`, traversal metadata artifact, and structured observation summary.

## Contract Rules

- the contract describes traversal derivation only
- the contract does not assume downstream intent
- the contract does not encode `context generate` semantics
- the contract must support multiple traversal strategies behind one stable capability id
- the contract is strategy in, ordered Merkle node set out
- the first slice internal representation should use a closed enum for strategy selection
- the first slice should avoid trait-object-heavy strategy infrastructure unless future strategy growth makes it necessary

## Strategy Variants

The first slice supports `bottom_up` and `top_down`.

Additional strategies can be added later without changing the capability identity.

## Internal Representation

The first slice should use a typed internal representation such as `TraversalStrategy`.
That representation should be matched directly by the traversal service.

This is strategy-shaped behavior, but it does not require heavy design-pattern machinery.
The stable abstraction is the capability contract.
The initial implementation can stay as a small enum plus one algorithm per variant.

## Domain Boundary

The Merkle tree and traversal logic should live behind this capability contract.
`context_generate` must consume the ordered Merkle node set artifact rather than derive traversal internally.

## First Slice Refactor Impact

Making this capability explicit requires:

- pulling tree ordering logic out of `context generate`
- producing a typed ordered Merkle node set artifact
- moving traversal policy selection outside `context generate`
- compiling traversal as its own capability instance in the candidate capability graph

## Non Goals

- choosing why traversal is needed
- deciding which downstream capability should consume the traversal
- embedding plan graph logic into the ordering implementation
