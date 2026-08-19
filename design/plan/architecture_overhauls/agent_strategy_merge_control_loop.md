# Agent, Strategy, And Merge Control Loop

Date: 2026-08-19
Status: architectural discovery, implementation not authorized
Scope: clarify ownership of maintained intent, Goal formation, repair planning, evidence acquisition, and epistemic curation

## Finding

The intended control loop is:

```text
Directive
→ grounded belief questions
→ reconciled divergence
→ Agent Goal draft
→ Strategy candidate Composition
→ Agent authorization
→ Execution or epistemic convergence
→ Events and Merge
→ reassessment
```

Agent owns the deliberative loop. It carries perspective and maintained intent, decides whether divergence warrants a Goal, selects construction policy, judges Strategy candidates, authorizes work, and reassesses satisfaction.

Strategy is a pure per-Goal construction function invoked under the Agent's perspective, policy, world projection, Capability catalog, and authority context. It does not own a durable mutable planning entity and does not align a Directive directly.

Execution receives a Goal plus the exact authorized Composition. A bare Goal is not enough to invent operational meaning downstream.

## Current Fracture

The current runtime partly treats Strategy as a root-supplied package-era configuration rather than a clean per-Agent invocation. It also lacks a complete relation-rich planner projection and a general way to request deeper epistemic context.

More importantly, the system does not yet expose a clear generic owner for epistemic-only convergence. Strategy can recognize that a claim relation, evidence assignment, or semantic derivation is missing, but Strategy cannot author the accepted relation that would make its own Goal appear satisfied.

## Architectural Direction

Two kinds of repair must remain distinct:

```text
acquire or compute new evidence
→ Capability
→ Task
→ Task Network
→ Event

admit, integrate, or derive epistemic state
→ Merge-side owner actor
→ authoritative record
→ graph projection
→ belief revision
```

Reading a file, invoking a model for semantic conversion, scanning dependencies, or producing a verification artifact is deliberate evidence acquisition. It uses ordinary execution primitives even when it does not mutate the external world.

Attaching already admitted evidence to a belief question, deriving a deterministic relation under installed policy, or revising a belief is epistemic curation. It belongs to the graph, belief, or semantic owner under Merge authority.

When Strategy lacks context, it may identify one of two needs:

- deeper bounded projection when the information already exists
- observation work when new evidence must be acquired

Either path produces a new frozen planning snapshot before Strategy searches again.

## PDS Pressure

PDS should remain small because Meld supplies this control loop natively. For documentation freshness, PDS may state that folders require correct READMEs and define claim materiality and verification meaning. It should not encode source scanning, semantic conversion, Task topology, claim attachment mechanics, or the active toolbox.

If expressing a domain requires PDS to carry those mechanics, the missing abstraction belongs in Meld's Agent, Strategy, Merge, graph, belief, or execution architecture.

## Unresolved Architecture

- how an Agent selects and invokes Strategy without a root-owned package template
- how Goal admission binds the exact authorized Composition and current projection
- how Strategy requests deeper projection without live graph access
- which owner receives generic semantic-object and relation proposals
- how deterministic curation is scheduled, retried, and made idempotent
- where curation ends and evidence-producing computation begins
- how reassessment wakes after either execution or epistemic convergence

The core invariant is that Strategy plans repair but never writes the epistemic truth that proves its own success.
