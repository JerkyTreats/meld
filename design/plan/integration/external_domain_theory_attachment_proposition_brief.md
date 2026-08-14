# External Domain Theory Attachment Proposition Brief

Date: 2026-08-13
Status: discovery proposition, not implementation authority
Scope: define the candidate boundary between Meld and independently owned typed domain theory before Step 5 design begins

## Proposition

Treat each stewardship expression as an exact, externalizable typed domain theory package that attaches to Meld through owner-defined installation and activation contracts.

The package supplies domain meaning. Physical adapters supply environmental observations and actions. Meld supplies durable cognition, authority, planning, execution, and replay.

External means outside Meld core ownership. It does not require a separate repository, crate, process, or service. Runtime placement is an activation choice and is not part of package identity.

```text
principal declaration
+ exact domain theory package
+ activation bindings
→ installed stewardship image
→ Meld runtime
```

## Design Change

The current installed image has exact revisions, but its shape still requires docs-owned bodies. The proposed change makes the image open to exact owner contributions while retaining a small common Meld contract.

The common contract carries only semantics that every stewardship expression must share:

- package identity and exact revision
- standing assignment scope
- maintained condition and settlement references
- required capability contracts
- requested authority
- activation requirements
- complete installation evidence

Owner contributions carry meanings that Meld must not generalize:

- domain vocabulary and relations
- belief and evidence semantics
- domain verdicts and invalidation rules
- candidate ground and remediation scope
- outcome interpretation
- owner-specific policy revisions

The root runtime links exact references and verifies completeness. It does not parse owner bodies, branch on expression names, or acquire owner vocabulary.

## Package, Adapter, And Runtime Boundary

The semantic package is state-free. It defines durable meaning and exact references, but it does not own the live ledger, belief state, Agent decisions, Goals, tasks, authority decisions, or revision cursors.

An adapter binds that meaning to a physical environment. It may read an advisory source, parse a manifest, invoke a resolver, execute verification, or publish a change. Adapters publish typed observations and consume typed capability requests. They do not become alternate planners or canonical stores.

Meld remains the sole owner of runtime cognition and coordination. Canonical state stays in Meld or in an explicitly authoritative external source. Restart and replay do not depend on hidden adapter memory.

```text
domain theory package
        ↓ exact revision and owner adapter
typed verdicts, propositions, capability contracts
        ↓
Meld ledger → beliefs → Agent → Strategy → Execution
```

## Isolation Posture

Semantic isolation is required. Meld core contracts must remain free of dependency-security, docs, or future domain vocabulary.

Executable isolation is permitted where failure containment, credentials, dependency conflicts, language choice, or release cadence justify it. The same adapter contract should be usable in process, in a subprocess, or through a remote transport without changing domain semantics.

A persistent runtime isolate for every package or assignment is not yet justified. Making it mandatory now would freeze process lifecycle, authentication, reconnect, backpressure, leases, upgrade, and observability protocols before the product has evidence that those protocols are needed.

Package identity must therefore remain independent of process identity. The likely isolation unit is an executable adapter or activation binding, not the semantic package itself.

## Relationship To Synthesis

Synthesis is not a dependency or deliverable of this proposition. Its proposed runtime isolation model is useful evidence that capability implementations can live behind stable contracts.

The proposition keeps that future available without importing Synthesis as an architectural prerequisite. If later evidence supports synthesized or remote implementations, they should satisfy the same activation and capability contracts as local adapters.

## Evidence In The Current Product

Steps 1 through 4 already provide exact theory revisions, complete receipts, declaration-selected activation, maintained conditions, and authority enforcement. Generic Agent, Strategy, and execution contracts carry no docs vocabulary.

The remaining docs coupling sits mainly at the installed image and activation edges. Fixed theory slots, direct docs capability contribution, one concrete subject, and a universal provider requirement prevent a truly dissimilar expression from attaching cleanly.

Current event and capability contracts provide partial support for a transport-neutral boundary. The live registries, actor roster, and store access remain single-process. This is evidence for preserving isolation as an option, not for declaring a distributed runtime now.

## Step 5 Consequence

The CVE freshness workstream should use this proposition as a design hypothesis:

1. Refactor only the proven docs-shaped attachment seams.
2. Express dependency-security as typed domain theory with physical advisory, manifest, resolver, and verification adapters.
3. Run the CVE proof through existing Meld cognition and execution.
4. Refine the common boundary by comparing docs freshness and CVE freshness before freezing it.

This is a `refactor → CVE → refine` discovery sequence. It is not approval for a general plugin platform or an implementation plan.

## Long Tail Consequences

If the proposition holds, Meld core can stabilize around transport-neutral cognitive and execution contracts while domain packages evolve on independent release and dependency schedules. Domains can isolate credentials and unstable libraries without moving their meaning into Meld.

The cost is a stricter link boundary. Exact package compatibility, owner validation, activation completeness, and typed failure must be visible. Opaque extension payloads would only move expression dispatch into an unsafe decoding layer.

Process isolation cannot repair a weak semantic contract. A perfectly isolated adapter that returns untyped success still erases advisory coverage, resolver feasibility, verification, and authority distinctions.

## Falsification Conditions

The proposition should be rejected or revised if Step 5 shows that:

- a domain package must directly own Meld ledger, belief, Goal, or task state
- domain semantics cannot cross the attachment boundary without core vocabulary branches
- local and isolated adapter placements require different semantic results or authority models
- exact owner extensions cannot be validated without root understanding their bodies
- package identity must vary with deployment topology
- CVE freshness requires a second planner, event loop, effect lock system, or canonical store

## Validation Signal

The proposition becomes a strong freeze candidate only after two dissimilar expressions install through the same root shape, retain exact replayable semantics, and use the same runtime ownership model.

Step 5 should also prove that one adapter contract can be serialized and exercised across a test boundary. That is a boundary conformance test, not a commitment to persistent process isolation.

Evidence from multiple assignments, restart behavior, real credential boundaries, or conflicting dependency stacks is required before choosing mandatory isolate granularity.

## Non Commitments

This proposition does not select:

- a package file format
- a separate repository or crate boundary
- a dynamic loading mechanism
- a daemon or sidecar topology
- a remote transport
- a package marketplace
- a universal facet lifecycle
- a Synthesis implementation
- a final owner extension envelope
- a final Agent scope discovery contract

## Working Design Signal

Freeze the semantic attachment boundary before freezing runtime topology.

```text
domain theory owns meaning
adapter owns physical translation
Meld owns cognition and durable runtime state
activation owns placement
```

This preserves the runtime-isolate idea as a real architectural option while requiring Step 5 to prove that isolation belongs in the mandatory product shape.

## Read With

- [Theory Elevation Program](theory_elevation_program.md)
- [CVE Freshness Step 5 Discovery Ground Map](cve_freshness_step_5_discovery_ground_map.md)
- [CVE Freshness Use Case](../../use_cases/cve_freshness.md)
- [PDS Theory Runtime Layer](pds_theory_runtime_layer.md)
- [Synthesis Overview](../../cognitive_architecture/execution/synthesis/README.md)
