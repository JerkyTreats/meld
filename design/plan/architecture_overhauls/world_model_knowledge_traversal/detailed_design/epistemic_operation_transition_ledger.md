# Epistemic Operation Transition Ledger

Date: 2026-08-21

Slice: `WMR-DD-02`

Status: accepted design product with retrospective clarification

Implementation authorization: none

## Purpose

This ledger separates the identities and durable positions crossed by bounded epistemic authorship. It is not a universal runtime record and does not prescribe storage, fields, or APIs.

Equal identities in one row mean deterministic replay of the same semantic transition. A changed authority, rule, operation, source cut, perspective, bound, or output policy creates a successor identity rather than mutating the predecessor.

## Identity Chain

| Identity | Owner | Meaning | Stable inputs | Must remain distinct from |
| --- | --- | --- | --- | --- |
| standing selection identity | Curation | one eligible application of an installed rule to an exact source cut | Agent, perspective, branch, activation generation, rule revision, subject, source cut, selection bound | planned authorization |
| planned authorization identity | Agent | permission for one exact Plan product | Agent, Goal, Plan revision, product identity, frozen context, authority scope, idempotency key | Curation acceptance and completion |
| operation identity | Curation | one bounded request as understood by Curation | invocation context, initiating authority, operation or rule revision, roots, source cut, bounds, vocabulary, completion policy | result identity |
| acceptance identity | Curation | one durable decision that either admits the operation or rejects it before admission under one fence | operation identity, evaluated authority revision, Curation policy revision, activation generation | execution and terminal result of an admitted operation |
| acceptance-rejection receipt identity | Curation | observable terminal intake outcome for an operation that was not admitted | acceptance identity, rejection class, exact failed validation, fence | result identity and semantic publication identity |
| result identity | Curation | one terminal semantic or operational disposition | accepted operation identity, exact consumed cut, Curation rule revision, outcome class | Event record identity and Graph fact identity |
| semantic publication identity | Curation | one Curation-owned expected entity, edge occurrence, assessment, withdrawal, or supersession | result identity, semantic unit identity, perspective, owner currentness policy | observed owner products |
| Event record identity | Events from Curation key | neutral durable carriage of one result or semantic publication | producer identity and deterministic publication key | Curation result meaning |
| Graph projection position | Graph | durable materialization through one Event position | ledger identity, Event sequence, projection revision | Event append receipt |
| configured evidence identity | Belief | admitted evidence under one installed mapping and exact source result | result Event identity, mapping revision, belief key, perspective | graph reachability |
| Belief revision identity | Belief | immutable settlement over exact admitted evidence | belief key, predecessor, evidence set, comparator and policy revisions | Curation result and Agent acceptance |
| Agent acceptance position | Agent | durable consumption of relevant owner result or Belief revision | Agent subscription, exact source revision, Plan or Goal context | Belief commit |
| Planner source position | Planner | exact Belief revision admitted to a later complete `PlannerCut` | Belief revision and later assembly inputs | `TraversalCut` or complete `PlannerCut` |

## Invocation Contexts

| Concern | Standing Curation | Strategy-planned Curation | Shared Curation rule |
| --- | --- | --- | --- |
| initiating authority | installed Agent specification and Curation rule | exact Agent authorization of a Plan product | initiating authority is explicit and immutable |
| bootstrap role | may produce the mismatch from which Agent forms a Goal | performs knowledge work already justified by a Plan | neither path creates a second Curation owner |
| exact input | subject plus accepted `TraversalCut` and rule revision | authorized operation plus accepted `TraversalCut` | no live or unbounded read may substitute |
| acceptance | durable admission or preadmission rejection of the standing operation | durable admission or preadmission rejection of the authorized operation | acceptance is separate from execution and an admitted operation result |
| terminal result | shared result grammar | shared result grammar | same identity, publication, replay, and currentness rules |
| downstream use | Graph and optional configured Belief visibility | Plan dependency may later cite a declared visibility milestone | Agent progression remains `WMR-DD-03` |

## Terminal Outcomes

| Outcome | Meaning | Semantic publication obligation | Completion claim |
| --- | --- | --- | --- |
| `applied` | requested Curation-owned state changed under the exact cut | publish every new, withdrawn, or superseding semantic product and the terminal result | operation terminal only |
| `unchanged` | requested Curation-owned state already held under the exact cut | publish a terminal result citing the existing semantic products | successful epistemic closure, not Goal satisfaction |
| `abstained` | policy deliberately declines authorship inside the declared bound | publish terminal reason and exact bound | terminal without desired assertion |
| `incomplete` | declared bound was exhausted without a complete answer | publish exact frontier, exclusions, failures, and cut | terminal bounded incompleteness |
| `rejected` | authority, shape, ownership, fence, or an admission precondition was invalid before work was admitted | persist an acceptance-rejection receipt naming the exact validation failure and fence, with no semantic publication | terminal intake outcome, not an accepted-operation result |
| `conflicted` | exact currentness or precondition no longer matches | publish conflicting revisions and successor eligibility | no silent retry under old identity |
| `failed` | Curation could not complete because of an operational fault | publish durable failure only when the operation reached accepted work | no semantic completion claim |

After admission, a policy decision not to author is `abstained`, a stale or incompatible currentness condition is `conflicted`, and an operational fault is `failed`. The accepted-operation terminal sequence does not reuse `rejected`.

Silence, timeout, an empty graph result, and an absent Event are never terminal outcomes.

## Position Sequence

```text
initiating authority position
-> Curation acceptance decision position
   -> preadmission rejection receipt and intake closure
   or
   -> Curation admission position
      -> terminal Curation result position
      -> Event append position
      -> Graph projection position
      -> optional configured Belief revision position
      -> deferred Agent acceptance position
      -> deferred Planner source position
```

Each arrow is a producer-consumer relationship. No earlier position implies a later one.

## Idempotency And Successors

- replaying the same accepted operation against the same exact cut and rule revision returns the same result identity
- publishing the same terminal result or semantic product reuses deterministic Event record identities
- a changed source cut, rule revision, authorization, perspective, bound, or publication policy creates a successor operation
- a `failed` standing operation remains terminal under the same identity; a successor requires a named semantic trigger such as renewed authority, an explicit retry generation, or a changed declared input
- currentness is owned by Curation through explicit validity, withdrawal, or supersession meaning
- ledger order alone does not supersede semantic products
- configured Belief settlement creates its own successor revision and never rewrites a Curation result
- Agent and Planner retain their own acceptance and assembly positions

## Feedback Boundary

Curation may consume Graph material that includes earlier Curation products. The consumed source set is frozen in the operation identity. A result cannot add its own Event or projection position to that source cut after acceptance.

Self-produced material is eligible only when the installed rule explicitly names it as input. Reaching the same semantic state returns `unchanged`; it does not generate another changed publication. A successor source cut can create successor work only when a declared dependency changed. Event replay or graph catch-up alone cannot grant new authority.

## Lifecycle Projection

| Claim | Exact position |
| --- | --- |
| ready | Curation has valid initiating authority, installed rule or accepted authorization shape, and access to the exact source cut |
| wait | no eligible standing selection, missing named source revision, unavailable exact cut, or no planned authorization |
| wake | rule revision, source owner revision, Graph projection advancement, exact planned authorization, or declared deadline; a deadline may re-evaluate eligibility but cannot by itself retry a terminal failed identity |
| fence | Agent, perspective, branch, activation generation, rule or operation revision, source cut, and authority lineage |
| restart | durable Curation acceptance and terminal positions plus Event, Graph, and configured Belief consumer positions |
| local quiescence | all accepted Curation work is terminal and no eligible work exists through the observed selection positions |
| not implied | Graph quiet, Belief quiet, Agent acceptance, activation quiescence, or safe retirement |

Before runtime implementation, Curation must select and record the allowed successor trigger for failed standing work. Until that rule is selected, failure remains terminal and no scheduler, deadline, or replay may mint a successor implicitly.

## Deferred Fields

`WMR-DD-03` must close:

- production and persistence of planned authorization
- exact Plan dependency milestone selected from result, Graph, Belief, or Agent positions
- Agent result acceptance and product progression
- Belief revision admission into complete `PlannerCut` assembly

`WMR-DD-06` must close activation-wide readiness, supervision, aggregate quiescence, and safe retirement.
