# Docs Freshness Flywheel Next Iteration Report

Date: 2026-06-22
Status: historical proposal, superseded for runtime completion
Scope: current goal and next code iteration for one visible `docs_freshness` flywheel

Authority: [Runtime Completion Ground Map](runtime_completion_ground_map.md) and [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) replace this report for current implementation. Its code inventory remains useful evidence, but its activation-first and one-goal-to-one-satisfaction framing is not the current operational-parity objective.

## Current Goal

The current goal is to make one real docs freshness goal move through the cognitive flywheel from durable initialization to durable satisfaction.

This is not yet the general conversational front end. The future front end should translate user intent into deterministic records, assess capability coverage, describe the plan, and initialize a long horizon goal only after confirmation.

The first product visible slice is narrower:

```text
docs freshness activation config
-> durable directive and seed agent
-> belief subscription
-> stale or missing docs freshness belief
-> active goal
-> scan if workspace state is missing or stale
-> docs writer task network
-> publication
-> evidence ingestion
-> belief revision
-> satisfaction decision
```

The important property is not that docs freshness can run. Existing workflow paths can already do useful docs work. The important property is that the work exists because the cognitive substrate created and pursued a durable goal.

## Current State

The implementation is close in primitives and incomplete in product wiring.

Docs writer execution exists today through the legacy workflow task path. It can resolve workspace context, fan out traversal, call provider capabilities, write frames, and emit task outcome data.

Workspace scan and watch exist today, but scan is a CLI initialization style operation rather than a first class capability in a task network. The next slice should make scan an explicit step with inputs, outputs, effects, and dependencies.

World model belief, evidence ingestion, curation, goal creation, publication replay, and satisfaction have concrete tests. The durable reopen tests prove the diagnostic flywheel mechanics, but the proof is still fixture shaped.

Execution planning exists, but the docs freshness method is not yet bound to the real docs writer package as the production path. The current planning proof is enough to validate mechanics, not enough to satisfy the intended product use case.

Product runtime assembly exists, but `meld runtime run` still supervises lifecycle handles. It does not yet start the concrete semantic actors needed to pursue the docs freshness goal end to end.

## Architectural Conclusion

This should be treated as a focused wiring and ownership iteration, not a workflow rebuild.

The existing workflow system should remain the docs writer adapter for this slice. The cognitive runtime should own why the work exists, which goal is active, when scan is needed, what evidence was published, and whether the goal is satisfied.

Capabilities are the verbs. The nouns must remain durable object references and task artifacts. The current lowering TODO is valid because collapsing a `DomainObjectRef` into only `object_id` violates the principle that the object travels with the plan.

## Next Code Iteration

The next iteration should produce one executable docs freshness flywheel from config to satisfaction.

| Step | Work Item | Owner Boundary | Expected Result |
| --- | --- | --- | --- |
| 1 | Add `workspace_scan` capability contract | `workspace` plus execution capability catalog | Scan becomes a normal task step, not a preflight blocker |
| 2 | Add single docs freshness activation file | product assembly parses and validates, domains own writes | A folder scoped docs freshness activation can be loaded deterministically and split into owner-scoped runtime inputs |
| 3 | Add bootstrap runtime for directive and seed agent | world model agent domain | Directive, seed agent, and belief subscription are registered idempotently |
| 4 | Bind docs freshness method to real docs writer package | execution planning and task package adapter | Planner output triggers the existing docs writer task path |
| 5 | Start bounded semantic runtime handles | supervisor starts, domains execute | Bootstrap, curation, planning, publication, evidence replay, and satisfaction can tick under runtime |
| 6 | Add CLI activation surface | CLI adapter only | A user can point Meld at a folder and activate docs freshness |
| 7 | Add end to end product test | integration proof | Reopen checkpoints prove state durability across the whole path |

## Scan Capability Contract

`workspace_scan` should be modeled as a capability with explicit contract shape.

| Contract Area | Proposed Shape |
| --- | --- |
| Inputs | workspace root, optional target selector, scan policy, session id |
| Outputs | scan summary, workspace snapshot ref, root node ref, observed node refs |
| Effects | read filesystem, update workspace tree, produce publication candidates or append requests through owned runtime port |
| Dependencies | filesystem access, ignore policy, workspace store, event publication adapter |
| Non Goals | provider calls, docs writing, belief decisions, goal mutation |

The capability may produce event candidates or publication records, but canonical event append should stay behind the runtime publication boundary. Hidden event publication inside arbitrary capability code would make effects harder to reason about and replay.

## Persistence Model

The first slice should persist every decision that matters for replay and diagnosis.

| Durable Record | Owner | Purpose |
| --- | --- | --- |
| directive record | world model agent | stable user intent shell |
| seed agent record | world model agent | durable actor serving the directive |
| belief subscription | world model agent | connects agent to docs freshness belief key |
| belief family config snapshot | world model belief | explains confidence and freshness decisions |
| active goal | execution goals | durable work request |
| task network command and journal | execution task network | deterministic execution plan and progress |
| publication record | execution publication | durable handoff to event spine |
| event envelope | events | canonical cross domain fact |
| promoted evidence | world model belief | reason belief changed |
| satisfaction decision | world model agent plus execution goals | reason the goal closed |

The activation file may be narrow, but the records created from it should already use the intended durable boundaries. Root assembly parses the file, validates the DTO, and passes typed input packages to runtimes. Runtimes do not parse the activation file and do not depend on its file format.

## Definition Of Done

The next code iteration is complete when a focused command or integration test starts from one docs freshness activation file and a folder path, then reaches durable goal satisfaction without fixture only shortcuts.

The proof should show:

```text
activation file loaded
-> validated activation split into owner-scoped runtime inputs
-> bootstrap wrote or confirmed directive, agent, and subscription
-> missing or stale workspace state requested workspace_scan
-> scan produced workspace object refs
-> low docs freshness belief created an active goal
-> planner lowered the goal into docs writer task network work
-> docs writer task outcome published to the event spine
-> evidence ingestion revised docs freshness belief
-> satisfaction runtime closed the goal
-> reopen checkpoints recover the same state
```

## Known Risks

The `DomainObjectRef` lowering TODO should be fixed before broadening beyond the first docs freshness path. Keeping only `object_id` will make mixed domain plans fragile.

The workflow adapter boundary must stay explicit. Workflow can execute the docs writer package, but workflow should not become the source of cognitive goal truth.

Event publication authority must remain clear. Capabilities can report artifacts and requested effects, while publication runtimes append canonical events.

Directive identity is still not fully represented as an independent durable record in product flow. The next slice needs at least the thin directive shell defined by the physical configuration requirements.

## Recommended First Patch Set

Patch one should add the `workspace_scan` capability contract, registration, and characterization tests around existing scan behavior.

Patch two should add the single docs freshness activation file loader, validated activation DTO, owner-scoped runtime input packages, and world model bootstrap runtime that writes directive, seed agent, and subscription records idempotently.

Patch three should bind the docs freshness planning method to the existing docs writer task package and prove the task network command shape.

Patch four should assemble the bounded runtime handles under `meld runtime run` and add the end to end reopen proof.

This sequence keeps the work close to current primitives while producing the visible flywheel the architecture is meant to demonstrate.
