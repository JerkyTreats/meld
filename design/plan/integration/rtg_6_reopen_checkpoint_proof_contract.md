# RTG-6 Reopen Checkpoint Proof Contract

Date: 2026-06-14
Status: proposed
Scope: durable reopen checkpoints for the first `docs_freshness` runtime proof

## Purpose

This plan defines the `RTG-6` contract required before the durable runtime host is implemented.

The first proof must show that progress survives planned host loss. The host may coordinate bounded turns, but correctness state must live in domain stores.

`RTG-6` is complete when the first proof can drop all opened host values at required boundaries, reopen stores from the product root, and continue by querying domain-owned durable state.

## Relationship To `RTG-5`

`RTG-5` supplies the deterministic `docs_freshness` fixture and identity contract.

`RTG-6` does not define new semantic ids. It uses the `RTG-5` subject, belief key, seed agent, curation rule, method, task network id, session id, worker id, artifact type, publication event type, and stable id derivations.

The reopen tests may store only fixture constants and the product root across a checkpoint. Goal records, task network state, publications, event records, promoted evidence, belief views, mutation commands, and cursors must be reloaded or recomputed after reopen.

## Checkpoint Model

Each checkpoint has four phases.

1. A bounded domain operation writes durable state through the owning command or append boundary.
2. Every touched store is flushed. Per-network task stores are flushed before the product boundary flush.
3. All opened stores and host-like values are dropped.
4. Stores are reopened from `ProductStorageLayout`, and assertions read through domain query APIs.

Worker reports are diagnostic evidence. They may show that a resumed tick made progress, but they are not correctness state.

## Required Checkpoints

### `after_goal_acceptance`

Required writes:

- graph seed records are present in the world model graph store
- seed agent and subscription are present in the agent store
- low confidence belief view is present in the belief store
- agent curation decision is persisted before execution receives the command
- execution goal store contains the active accepted goal

Stores to reopen:

- world model store
- execution goal store
- product stores opened by `OpenProductStores`

Durable assertions after reopen:

- expected goal id exists in execution storage
- lifecycle is `Active`
- source command id equals the fixture goal command id
- source identity equals the fixture goal source identity
- agent decision exists by dedupe key
- active goal query returns only the expected `docs_freshness` goal

Later host report assertions:

- goal curation tick reports one committed decision
- goal acceptance tick reports one committed goal lifecycle output
- no fatal errors are present

Forbidden local state:

- `AgentGoalCommand`
- active `Goal`
- curation output
- belief view
- subscription cursor

### `after_pending_publication`

Required writes:

- active goal has been reloaded from execution storage
- planning has composed the docs method
- task network command boundary accepted a task mutation
- task claim is recorded
- task outcome is recorded
- pending publication exists in task network state

Stores to reopen:

- execution goal store
- task network store for `network-docs`
- task artifact store when task artifacts are written
- product stores opened by `OpenProductStores`

Durable assertions after reopen:

- task status is succeeded
- outcome exists in task network state
- exactly one pending publication exists
- publication event type is `execution.task.succeeded`
- publication payload includes the required `docs_patch` artifact type

Later host report assertions:

- planning tick reports a committed task network proposal
- task network tick reports one accepted mutation
- task worker tick reports one committed outcome
- no fatal errors are present

Forbidden local state:

- planning result
- task network state clone
- dispatch claim
- task outcome
- publication id selected before reopen

### `after_publication_append_before_satisfaction`

Required writes:

- publication bridge appends the task outcome event idempotently
- task network marks the publication published only after append succeeds

Stores to reopen:

- event spine
- task network store for `network-docs`
- world model store
- execution goal store
- product stores opened by `OpenProductStores`

Durable assertions after reopen:

- event spine contains the publication event
- event record id matches the task network publication id
- task network publication is marked published
- published state carries the event spine sequence
- docs evidence can be rebuilt from the reopened event record
- promoted evidence ingestion updates the docs freshness belief
- satisfaction curation decision is persisted before execution mutation
- final execution goal lifecycle is satisfied at the fixture satisfaction review sequence

Later host report assertions:

- publication bridge report attempts one item
- publication bridge report commits one item
- resumed evidence ingestion reports durable progress
- satisfaction curation reports a committed decision before goal mutation
- no fatal errors are present

Forbidden local state:

- publication object
- event record
- promoted evidence
- updated belief view
- satisfaction mutation command
- last event cursor

## Durable Assertions

All assertions after a checkpoint must read through owning stores.

- event facts come from `EventStore`
- graph anchors come from `TraversalStore` or `TraversalQuery`
- belief views come from `BeliefStore` or `BeliefQuery`
- curation decisions come from `AgentStore` or `AgentQuery`
- goal lifecycle comes from `PersistentGoalSetStore`
- task and publication state come from `SledTaskNetworkStore`

The product root and fixture constants are the only correctness inputs allowed to cross checkpoint boundaries.

## Forbidden Host Local State

Future host code must not store semantic progress in local fields that replace domain cursors.

Forbidden examples:

- accepted goal object
- current task network state clone
- pending publication object
- last appended event record
- promoted evidence record
- current belief view
- satisfaction mutation command
- host-owned source cursor

Allowed examples:

- product root path
- fixture or configuration constants
- diagnostic report list
- checkpoint label

## Pre Host Contract Tests

The pre-host tests simulate the required boundaries with existing domain APIs.

Required tests:

- `docs_freshness_reopens_after_goal_acceptance_from_product_stores`
- `docs_freshness_reopens_after_pending_publication_from_product_stores`
- `docs_freshness_reopens_after_publication_append_before_satisfaction`

These tests prove the durable state shape before `MeldRuntimeHost` exists.

## Future Runtime Host Wiring

When `MeldRuntimeHost` lands, the host test must reuse the same fixture and checkpoint expectations.

The host flow must:

1. load from a product root
2. run bounded turns to `after_goal_acceptance`
3. flush touched stores
4. drop the host
5. reload from the same product root
6. run bounded turns to `after_pending_publication`
7. flush and reload again
8. run bounded turns to `after_publication_append_before_satisfaction`
9. flush and reload again
10. converge to satisfied goal lifecycle

The final assertion reads the satisfied lifecycle from execution storage.

## Exit Criteria

`RTG-6` is complete when:

- this contract exists
- fixture contracts pin ids and checkpoint sequences used by reopen tests
- each checkpoint test reopens stores from `ProductStorageLayout`
- assertions after reopen use domain query APIs
- pending and published publication states survive reopen
- satisfaction curation decision is persisted before execution mutation
- focused `docs_freshness` reopen tests pass
