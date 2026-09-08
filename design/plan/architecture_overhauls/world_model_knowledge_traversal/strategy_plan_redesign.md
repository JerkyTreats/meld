# Strategy Is Bigger Than A Task

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-08-20

Status: architecture redesign proposal, implementation not authorized

## Problem Statement

Meld currently knows how to change documentation. It does not know how to discover that the documentation was already correct.

That sounds like a small missing branch. It is not.

The installed docs freshness theory gives Strategy one way to establish freshness. Inspect the source scope. Draft README patches. Validate those patches. Publish them. Assess the published result.

```text
inspect
-> draft
-> validate
-> publish
-> assess
```

The branch excludes current README files from its inspection evidence. The assessment step requires a publication receipt. The only path to epistemic confidence therefore passes through an executable write pipeline.

Imagine `README F` already exists and is completely correct. The missing work is epistemic. Meld needs to establish that folder F requires this README, determine which source claims matter, connect those claims to the README claims, and assess the exact file under the exact workspace snapshot. None of that requires changing the workspace.

The current branch cannot express that answer. It must write before it can know.

This is the design flaw in current Strategy. `StrategyCandidate` contains one executable `Composition` and a prospective evidence route. The evidence route says that knowledge should arrive later, but it is not causal work in the plan. It cannot run before a Task, eliminate a Task, gate a Task, or verify a Task.

The system has confused Task Construction with Plan Construction.

## Thesis

Strategy is Plan Construction.

An Agent gives Strategy a Goal. Strategy decomposes that Goal into the conditions that must become established and selects closed causal products that may establish them. Some products change the external world. Some products change the admitted knowledge of that world.

The first product is a Task. Execution owns its realization.

The second product is a Bounded Epistemic Operation. Curation owns its realization.

```text
Agent Goal
-> Strategy Plan
   -> desired condition
      -> Bounded Epistemic Operation
   -> desired condition
      -> Task
   -> desired condition
      -> Bounded Epistemic Operation
```

Task is not Plan. Epistemic Operation is not Task. Neither product is itself a Goal.

Goal is the shared language of desired state. Task and Epistemic Operation are different means of discharge.

## Goal Stays Neutral

The current `meld-lang` Goal already points in the right direction. It carries an Agent, a desired proposition, priority, provenance, and lifecycle. It does not name an executor.

That neutrality matters. The same desired condition may need no external action when the world is already aligned, or may need a Task when observation proves a mismatch. Encoding Execution or Curation into the Goal would force the answer before Strategy had reasoned about the world.

Task and Epistemic Operation should therefore share Goal reference, target proposition, contribution, and satisfaction-evidence language. They should not share operation grammar, admission, execution, or storage.

A Strategy Plan is also more than a set of Goals. A set cannot explain that claim mapping enables writing, that publication enables verification, or that a successful epistemic assessment makes writing unnecessary. The plan must be a causal graph containing desired conditions, proposed discharge products, and dependencies between them.

The root Goal remains the durable Agent commitment. Intermediate desired conditions remain Strategy obligations unless they need independent authorization, maintenance, reuse, or lifecycle. This prevents every planning thought from becoming a new durable Goal.

## Strategy Constructs And Reconstructs

Current Strategy is a pure bounded function over one immutable problem. That is a strength worth keeping.

Strategy should not become the actor listening to every Event. It should become the pure constructor and reconstructor used by continuous reconciliation.

Agent owns the root Goal, authorization, and active plan progression. New admitted knowledge may invalidate an assumption, satisfy an obligation, make a Task eligible, or make a Task unnecessary. Agent then asks Strategy to reconstruct the plan against a new frozen snapshot while retaining plan lineage.

```text
Event
-> Traversal and Belief
-> Agent reassessment
-> Strategy reconstruction
-> new authorized plan revision
```

Strategy owns the semantic plan body. Agent owns whether that plan is pursued and when a changed epistemic position requires a new revision. This makes Strategy the reconciliation engine without pretending that the pure search function is itself a runtime scheduler.

## Two Closed Construction Problems

Strategy has two isolated construction problems.

Task Construction closes Capability identity, bindings, required artifacts, executable dependencies, authority, and expected external outcomes. The product is complete enough for Agent judgment and later Execution lowering.

Epistemic Operation Construction closes Agent perspective, input knowledge cut, root objects, traversal bounds, Curation rule identity, admissible output vocabulary, terminal meaning, and provenance. The product is complete enough for Agent judgment and later Curation execution.

Strategy can ground both products because each executing domain publishes the contract Strategy is allowed to reason about. Strategy does not invent Capability behavior. It must not invent Curation meaning either.

A bounded operation promises a terminal account, not success. It may complete with new knowledge, no change, abstention, bounded incompleteness, rejection, or failure. Tasks have the same distinction between termination and Goal achievement.

## Curation Has Two Entrances

Standing Curation exists before a Strategy Plan.

An Agent specification may say that every material folder requires a correct README. Standing Curation applies that rule to known folders and authors the expected README entities and requirement connections. This creates the expected-versus-observed mismatch from which Agent can form a Goal.

Planned Curation begins after a Goal exists.

Strategy may determine that claim materiality must be established before writing, or that the published README must be assessed before satisfaction. It constructs a Bounded Epistemic Operation and places that product in the plan.

Both entrances use the same Curation semantics and publication path. The difference is initiating authority. Standing work begins from an installed Agent specification. Planned work begins from an Agent-authorized Strategy Plan.

## Events Carry Epistemic Operations

The Event spine should carry the durable lifecycle and results of epistemic operations.

It should not define their language.

`EventEnvelope` already carries producer identity, event type, idempotency identity, object coordinates, relations, source provenance, and producer-owned payload. Events owns durable append, ordering, replay, and cursors. It deliberately does not know what a Curation rule means.

That is exactly the boundary needed here.

```text
Agent authorizes an Epistemic Operation
-> accepted-operation Event
-> Curation replays the Event
-> Curation executes against a frozen cut
-> terminal-operation Event
-> promoted Curation knowledge Events
-> Traversal materialization
-> optional Belief admission
-> Agent reassessment
```

The accepted-operation Event records that authority was granted. It is not a replayable imperative whose mere presence grants fresh authority forever. Curation owns a durable operation identity and state, so replay discovers unfinished work without repeating settled work.

Every shared semantic result should be appended as a Curation-owned Event. An operation that changes nothing or abstains still needs a terminal Event when later plan work depends on knowing that it finished.

This reuses the existing ingestion spine and avoids a second graph-write, notification, and recovery system. Other consumers can replay the same canonical result. Traversal can materialize its objects and edges. Belief can install an explicit mapping when the result is evidence for a belief dimension. Agent can react to the resulting revision.

An append receipt is not enough to satisfy every epistemic dependency. The result may be durable in Events while Traversal, Belief, or Agent still trails behind. A Strategy Plan dependency must name the milestone it actually requires: durable Curation result, graph materialization through a sequence, Belief revision, or Agent acceptance.

## The Cost Of The Event Spine

Event-backed Curation is not free.

Durable append, replay, graph projection, Belief ingestion, and Agent delivery create several visibility barriers. That is slower than returning a local function result. The repository has no measurements proving whether the difference matters for epistemic operations.

The gain is that each result has one ordered identity, provenance, replay, idempotent recovery, and independent fanout. A synchronous shortcut would still need to reproduce those properties or eventually reconcile with the authoritative Event. For shared knowledge that may wake several Agents and belief families, the durable path is the simpler system even when it is not the shortest call stack.

Feedback is the harder cost. Curation reads a graph that includes Curation results and then publishes more results into that graph. Every operation must bind a frozen source cut, deterministic identity, exact rule revision, and finite traversal bound. Curation must own supersession and currentness. Traversal cannot infer that a later edge invalidates an earlier one.

Perspective also cannot disappear inside a generic relation. Expected README F may belong to one Agent specification. The Curation product must retain that perspective and rule lineage even though `DomainObjectRef` and `EventRelation` do not carry those meanings themselves.

## What Execution Receives

The heterogeneous Strategy Plan never enters Execution.

When a Task becomes eligible and Agent authorizes it, Agent publishes only the executable Goal slice and the complete Task through the Execution Goal Set seam. Execution lowers that Task into its unified Task Network and remains ignorant of epistemic operations, Traversal, Curation rules, Strategy reconciliation, and why the Goal exists.

When an Epistemic Operation becomes eligible, Agent publishes it through the Curation path. Curation remains ignorant of Task Network state.

The root Goal and Strategy Plan remain in world-model. Results from both sides return as observations and may cause Agent to request a reconciled plan.

This preserves the sacred seam. It also retires the assumption that publishing one Goal package to Execution means publishing the whole plan. Execution receives the complete executable product it needs. It does not receive the complete cognitive product that produced it.

## Docs Freshness After The Redesign

Folder F enters the world model through workspace observation.

Standing Curation applies the Agent rule and authors expected README F. It also connects source claims to the expected README through requirement and materiality relations.

Curation compares expected README F with the observed workspace snapshot.

If an observed README realizes the expectation and its claims cover the required source claims, Curation publishes that bounded assessment. Belief reconciles it. Agent may judge the Goal satisfied. No Task is constructed.

If the README is absent or incorrect, the mismatch becomes explicit. Strategy constructs a plan containing the necessary Task and the epistemic operations needed to prepare or verify it. Agent routes each eligible product to its owner.

After Execution changes the workspace, workspace observation publishes the new file revision. Curation assesses realization and claim coverage under the new snapshot. The assessment enters Events, Traversal, and any configured Belief path. Agent closes or reconciles the plan from admitted knowledge rather than from Task completion.

```text
expected README F
+ observed workspace snapshot
-> Curation assessment

assessment says aligned
-> epistemic closure

assessment says mismatch
-> Strategy Plan
-> Task plus later Epistemic Operation
-> observation
-> reassessment
```

Now the system can do the simple thing. It can discover that the README was already correct.

## Redesign Boundary

The substantial redesign belongs inside world-model.

Strategy changes from one executable candidate to a heterogeneous causal Plan. Curation becomes a distinct domain with bounded operation contracts, execution, durable state, and Event publication. Agent gains active plan authorization and progression. Traversal admits promoted Curation products and preserves occurrence provenance. Planner provides the relation-rich frozen view Strategy needs.

The docs producer must expose independently addressable observed claims and verification products so Curation has real entities to connect. This is the specific product-side change proven by the docs freshness case.

Events remains the durable carrier and replay spine. It does not gain Curation semantics. Execution remains the consumer of executable Goal and Task products only. Task Network remains unchanged in meaning. `meld-lang` remains permissive shared vocabulary rather than a grammar enforcer.

## Open Edges

The architecture still needs exact answers for plan identity, plan revision lineage, and the point at which an intermediate Strategy obligation becomes a durable Agent Goal.

Agent authorization may bind the entire Plan, each enabled product, or both at different scopes. The safe shape likely needs plan-level consent plus product-level freshness judgment, but current evidence does not settle that contract.

The plan progression boundary must name exact epistemic milestones rather than a generic completed state. Raw Curation completion, graph visibility, Belief revision, and Agent acceptance are different events.

The public Curation contract must define its operation vocabulary, stable identity, perspective scope, bounds, supersession, and terminal results. The Event envelope cannot supply those semantics.

These are design edges inside the proposed model. They do not weaken its central split.

## Evidence

The [neutral current code ground map](current_code_groundmap.md) records implemented behavior without selecting this architecture.

The [Goal language and Strategy Plan review](reviews/goal_language_strategy_plan_review.md) validates shared desired-state language while keeping Goals distinct from their discharge products.

The [Event-backed epistemic operation review](reviews/event_backed_epistemic_operations_review.md) validates the Event spine as durable carrier and recovery mechanism while rejecting EventEnvelope as semantic grammar.

The [Traversal and Curation assessment](curation_assessment.md) grounds the separation between graph reads and epistemic authorship.

The [bounded epistemic operations refinement](bounded_epistemic_operations.md) preserves the earlier discovery step that exposed the mixed causal plan.

The prose shape follows the problem-first style of [Building A Music Engine](https://app.notion.com/p/35db05a0a5d54fe3a0c7dcba15cd7f1f), used as a style reference rather than architectural evidence.
