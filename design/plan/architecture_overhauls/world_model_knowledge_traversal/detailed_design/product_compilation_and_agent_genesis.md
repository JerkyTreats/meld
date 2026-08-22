# Product Compilation And Agent Genesis Detailed Design

Date: 2026-08-22

Slice: `WMR-DD-05`

Status: active-slice design candidate

Implementation authorization: none

## Decision

PDS compiles one principal-selected product into exact native-owner revisions, complete receipts for every selected package, one product compilation receipt, one situated assignment, a finite declared Agent topology, Agent-owned genesis receipts, and one inert prepared activation closure.

PDS owns product definition and lineage. It does not interpret native component bodies, own Agent cognition, construct situated Plans, grant effective authority, publish a runtime generation, or participate in ordinary reconciliation work.

## Product Path

```text
principal selection
-> product declaration revision
-> exact linked semantic package
-> complete native-owner validation
-> idempotent native-owner installation
-> exact package installation receipt for every selected package
-> complete product compilation receipt
-> situated assignment and declared Agent topology
-> Agent-owned records and subscription requests
-> source-owner subscription acceptance and genesis publications
-> complete Agent topology receipt
-> physical activation inputs and Capability preparation receipt
-> structural participant plan
-> inert prepared activation closure
```

## Product And Package Separation

A product declaration records the reusable stewardship composition selected by a principal. A semantic package is the linked structural carrier for exact owner-routed components. The declaration may select several packages or one package may support several product declarations. Neither identity substitutes for the other.

Compilation resolves every exact import and route, validates structural requirements, and asks each native owner to validate its own component. All validation completes before installation begins. Installation is content-idempotent and may reuse an existing immutable owner revision.

One complete package installation receipt proves one selected package. A product becomes selectable only when one product compilation receipt binds the exact product revision, exact selected package set, and complete installation receipt for every selected package under one compilation policy revision. Any missing or extra package receipt leaves the product unselectable. Immutable owner revisions left by an interrupted installation are safe local facts but are not proof that a package or product is complete.

## Native Ownership

The router knows component identity, route identity, import closure, structural requirements, owner receipt identity, and cross-component link declarations. It does not parse Belief meaning, Curation operations, Strategy constraints, maintained conditions, security semantics, docs semantics, or Capability behavior.

Each owner controls its schema, validation, installed revision, compatibility decision, predecessor lineage, and runtime interpretation. Events and Traversal remain product-blind. PDS cannot create a universal runtime object that bypasses those owners.

## Assignment And Authority

One assignment binds the exact product compilation receipt and product revision to the principal, subject, perspective, branch, declared Agent topology, requested authority, and grant lineage. Physical implementation selections, credentials, placement, isolation, and operational limits belong to separate activation inputs.

Requested authority is not effective authority. The prepared closure cites product-supported authority classes, principal request and grant lineage, current restriction inputs, and selected enforcement bindings. Agent judgment and Execution enforcement remain later owner decisions.

Secret values are resolved by the physical binding owner and are never package or assignment identity inputs.

## Declared Agent Topology

The product declaration chooses a finite topology rather than relying on one global cardinality rule. A product may declare one multi-concern Agent or several specialized Agents. Every topology position has a stable identity, required installed owner revisions, subject, perspective, branch, directive, observation scope, maintained conditions, required subscriptions, and structural participant reference.

Changing the number or meaning of topology positions creates a successor product or assignment position. It cannot silently mutate an existing Agent identity.

## Agent Genesis Boundary

Root initialization is a composition adapter. It presents one exact genesis intent per topology position to Agent-owned contracts. Agent validates the complete identity body, installed revisions, predecessor relationship, assignment lineage, and required subscription declarations before accepting a record revision.

An existing Agent with the same identifier is reusable only when its immutable identity body and lineage match. A changed subject, perspective, branch, topology position, provenance, or assignment is a durable conflict.

For each topology position, these milestones remain distinct:

| Milestone | Owner evidence | Does not prove |
| --- | --- | --- |
| Agent record accepted | Agent record revision and acceptance receipt | required subscriptions exist |
| subscription requests durable | Agent-owned natural-key requests name exact source owners, source contracts, keys, scopes, and cursor policies | source owners accepted them |
| source subscriptions accepted | each named source or Belief owner records accepted, duplicate, rejected, or conflicted consumer decisions | first source or Belief view is readable |
| genesis publication prepared | deterministic Agent outbox operation | Event append |
| genesis Event durable | Event append receipt | Graph catch-up or Belief readability |
| topology genesis complete | complete set of Agent-owned genesis receipts | participant readiness or current generation |

The Agent genesis receipt binds the record revision, complete source-owner subscription acceptance set, exact installed owner revisions, assignment, topology position, and genesis publication operation. A durable request alone is insufficient. Each named source or Belief owner owns its acceptance, rejection, conflict, and restart position under its exact contract revision. The complete topology receipt exists only when every declared position has one non-conflicted Agent receipt and every required subscription has a source-owner acceptance receipt.

Agent lifecycle labels such as registered or operational cannot substitute for this aggregate. Runtime admission remains closed until `WMR-DD-06` establishes a current activation generation.

## Capability And Participant Preparation

Packages may co-distribute Capability contracts and compatibility expectations. Activation preparation asks Capability to produce one inert receipt over the exact selected contract, offer, implementation, binding, compatibility-policy, placement, and limit revisions. PDS never treats co-distribution or selection as current availability.

The Capability preparation receipt proves only that the selected realization inputs are complete and mutually compatible under the cited policy. It does not prove that an offer is currently available, a binding is reachable, a participant is ready, or realization succeeded. Missing, stale, incompatible, unauthorized, and conflicted selections produce Capability-owned negative receipts and remain restartable from exact revisions.

The participant plan names exact structural participants and their preparation, readiness, wake, safe-point, and stop contract references. Its dependency graph may prove preparation order, physical realization prerequisites, and reverse drain order. It cannot encode the semantic order from observation through Belief, planning, Curation, Execution, and returned evidence. That order remains in native owner data and Strategy Plans.

## Inert Prepared Closure

Preparation closes only when one immutable aggregate binds:

- exact product declaration and complete product compilation receipt over every selected package installation receipt
- situated assignment and physical activation-input identity
- complete Agent topology receipt
- every required native-owner preparation reference
- exact inert Capability preparation receipt and binding revisions
- exact structural participant plan
- effective-authority input references without claiming the effective decision
- predecessor or expected-prior preparation position

The closure is inert. It is not a lifecycle intent, activation generation, readiness aggregate, current head, admission fence, or proof that any participant is running.

## Direct Product Proof

### Docs Freshness

The principal selects a docs stewardship product. Its linked package routes docs observation meaning, Graph and Traversal vocabulary, Belief evidence mapping, maintained condition, Curation rules, Strategy construction constraints, and Capability contract expectations to their native owners. The assignment declares the exact subject, perspective, branch, topology, and grant lineage. Agent genesis binds the installed docs-specific revisions without storing a shadow PDS cognition record.

An already-correct README later closes through owner observation and Agent judgment with no Task. A missing README may later produce Curation and Execution products. Neither path requires PDS participation after activation realization.

### Dependency Security

The security product routes manifest and advisory observation meaning, inventory and evidence mappings, security maintained conditions, mitigation constraints, verification obligations, and Capability compatibility expectations to their native owners. Its topology and required participants may differ from docs freshness while using the same structural compilation mechanics.

Inventory, advisory applicability, assessment, mitigation, and verification remain distinct security-owner meanings. The PDS package does not reduce them to generic success or reuse docs-specific theory.

## Restart And Upgrade

Restart resolves the selection, exact selected package set, package and import closures, installed owner revisions, complete package receipts, product compilation receipt, assignment, Agent records, subscription requests, source-owner subscription acceptance receipts, genesis publication operations, topology receipt, activation inputs, Capability preparation receipt, participant plan, and prepared closure by exact identity. It resumes from the first missing consumer acceptance position and never infers completeness from command history.

Upgrade effects follow changed identities. Changed owner meaning creates a successor owner revision; content-identical owner revisions remain reusable. Changed package composition creates a new package receipt. A changed selected package set or product revision creates a new product compilation receipt. Every selected product revision or topology change creates a successor assignment. Changed physical bindings create a successor activation input. Any changed prepared input creates a successor prepared closure. Historical Plans, Tasks, observations, Beliefs, and Agent decisions retain the prior exact lineage. `WMR-DD-06` owns the visible generation boundary and safe replacement.

## Deferred Boundaries

- lifecycle acceptance of the prepared closure
- participant incarnation and owner readiness
- current-generation publication and runtime admission
- steady-state waits and wakes
- replacement, late delivery, quiescence, and retirement
- exact Rust types, schema placement, storage, APIs, and migration sequence

No deferred boundary is borrowed as product readiness in this slice.
