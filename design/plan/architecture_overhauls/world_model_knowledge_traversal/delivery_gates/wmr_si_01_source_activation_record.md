# WMR-SI-01 Source Activation Record

Date: 2026-08-23

Slice identifier: `WMR-SI-01`

Design source: `WMR-DD-01`

Status: authorized and ready after the commit containing this record

Authorization authority: user

Implementation started: no

Pre-correction baseline commit: `b25d4c6b`

Implementation baseline: the commit containing this record and the canonical source delivery ledger

## Correction

The first activation record incorrectly treated `meld_startup` as the first source slice. The user requirement placed Startup before Docs Freshness and Dependency Security, not before the World Model Reconciliation substrate required to run Startup.

The [Startup injection baseline](../startup_pds_design_requirements/expected_injection_baseline.md) requires implementation equivalents of `WMR-DD-01` through `WMR-DD-06` before the Startup product proof. This record restores that dependency order.

## Authorized Outcome

Implement the accepted owner-publication-to-`TraversalCut` vertical. An owner observation and bounded completeness account must become a durable typed publication, pass through neutral Event carriage, become occurrence-preserving Graph material, and return through one immutable bounded `TraversalCut` with exact owner, revision, provenance, frontier, truncation, and hydration lineage.

The direct proof must include an already-observed docs specimen, a missing-or-incomplete docs specimen that does not invent absence or expectedness, and an owner-dissimilar dependency-security specimen. This establishes an owner-neutral graph substrate without implementing Curation, Strategy, Agent progression, Execution changes, PDS genesis, lifecycle composition, or Startup.

## Accepted Design Basis

- [WMR-DG-01 Gate Definition](wmr_dg_01_owner_publication_to_frozen_cut.md)
- [retrospective WMR-DG-01 receipt](wmr_dg_01_retrospective_assurance_receipt.md)
- [owner publication detailed design](../detailed_design/owner_publication_to_traversal_cut.md)
- [semantic transition ledger](../detailed_design/semantic_transition_ledger.md)
- [canonical Graph and Traversal design](../../../../cognitive_architecture/world_model/graph/README.md)
- [source Style Assurance overlay](wmr_source_style_assurance_overlay.md)

## Active Boundary

The active source surface is owner publication, typed Event attachment production, Graph admission and projection, Traversal contracts and storage, immutable cut construction, owner-correct hydration references, and the narrow root wiring needed for direct proof.

`meld-events` remains a neutral durable carrier and should be reused unchanged unless direct source evidence proves its accepted contract cannot carry the owner publication. `meld-lang`, Curation, Belief settlement, Planner, Strategy, Agent Plan progression, Execution, PDS activation, lifecycle aggregation, Startup, Docs Freshness migration, Dependency Security migration, and legacy Workflow are outside active behavior scope.

Docs and dependency-security fixtures or current owner publications may be used to prove substrate neutrality. Their semantic workflows are not active-slice implementations.

## Source Sequence

| Slice | Product increment | Status |
| --- | --- | --- |
| `WMR-SI-01` | owner publication through immutable `TraversalCut` | authorized, awaiting corrected clean checkpoint |
| `WMR-SI-02` | bounded Curation authorship and settlement | backlog |
| `WMR-SI-03` | `PlannerCut`, heterogeneous Strategy Plan, and Agent progression | backlog |
| `WMR-SI-04` | Execution admission, unified coherence, and observation return | backlog |
| `WMR-SI-05` | PDS product compilation and Agent genesis | backlog |
| `WMR-SI-06` | activation generation and lifecycle closure | backlog |
| `WMR-SI-07` | non-blocking `meld_startup` integrated runtime proof and inspection | backlog |
| `WMR-SI-08` | Docs Freshness product migration | backlog |
| `WMR-SI-09` | Dependency Security product migration | backlog |

Each slice requires its predecessor Gate Acceptance Receipt and separate activation authority. Startup remains the first integrated PDS runtime proof and is implemented before the two product migrations.

## Delivery Sequence

```text
source implementation
-> direct owner-publication and TraversalCut proof
-> logical implementation review
-> bounded logical correction and verification
-> Style Assurance
-> WMR-SI-01 Delivery Gate Acceptance
-> handoff eligible
```

The [WMR-SI-01 source Delivery Gate](wmr_si_01_owner_publication_to_traversal_cut_gate.md) is frozen before source edits. Passing it does not activate `WMR-SI-02`.

## Stop Conditions

Pause before a new Event store, graph authority, ontology, universal record schema, runtime coordinator, compatibility path, new crate, Curation behavior, Planner behavior, Strategy behavior, Agent progression, Execution change, or product-specific semantic rule.

Pause if an owner publication cannot be made durable without a new store or protocol, if Graph admission would need to interpret owner meaning, or if `TraversalCut` completeness cannot be expressed without changing an accepted owner boundary.

## Current State

The source program starts at the existing `DomainObjectRef`, Event attachment, `GraphRuntime`, `TraversalStore`, and `TraversalQuery` seams. The first engineering task is characterization of that path against the accepted identity and durability ladder, followed by the smallest contract additions needed for durable owner publication, occurrence identity, completeness receipts, immutable cuts, and reproducible bounded results.

No source implementation has started. Source edits begin only after this correction is committed and the worktree is clean. The commit containing this record becomes the implementation baseline.
