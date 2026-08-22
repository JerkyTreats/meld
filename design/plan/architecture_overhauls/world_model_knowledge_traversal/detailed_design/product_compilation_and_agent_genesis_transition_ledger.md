# Product Compilation And Agent Genesis Transition Ledger

Date: 2026-08-22

Slice: `WMR-DD-05`

Status: active detailed-design product

Implementation authorization: none

## Identity Chain

| Identity | Owner | Meaning | Stable inputs | Must remain distinct from |
| --- | --- | --- | --- | --- |
| principal selection identity | principal-facing PDS boundary | one durable selection of a product revision and situated intent | principal, product declaration revision, subject selector, requested authority, request key | effective authority and assignment |
| product declaration revision | PDS product model | one reusable product composition selected by a principal | declared concerns, topology rules, package references, product policy, predecessor | semantic package and assignment |
| semantic package identity | PDS structure | one exact linked set of owner-routed components and imports | package manifest, exact imports, route identities, structural requirements | owner-installed revisions and product declaration |
| owner component revision | native semantic owner | one immutable owner-validated meaning | owner grammar, component body, predecessor, provenance | PDS package body and runtime state |
| package installation receipt | PDS router over native receipts | complete linked proof that every required component has an exact installed owner revision | package, imports, route map, complete owner revision set | partial installation progress and selected head |
| product compilation receipt | PDS product structure | complete selectable proof for one product revision | product revision, exact selected package set, complete installation receipt for every selected package, compilation policy revision | any one package receipt and assignment |
| assignment identity | stewardship assignment owner | situated product binding for one principal and declared Agent topology | product compilation receipt, product revision, principal, subject, perspective, branch, topology, requested authority, grant lineage | activation inputs and Agent record |
| activation-input identity | stewardship activation owner | one exact physical realization request for an assignment | assignment, implementation selections, binding references, placement, isolation, limits | generation and current publication |
| Agent genesis intent | root composition adapter | idempotent request to native Agent owner for one declared Agent position | assignment, topology position, Agent identity, installed owner revisions, subject, perspective, branch, directive, provenance | Agent acceptance receipt |
| Agent record revision | Agent | durable native cognitive identity and installed theory bindings | Agent identity body, exact installed revisions, predecessor, lifecycle status | assignment and PDS shadow state |
| Agent subscription request identity | Agent | durable request for one required observation relationship | Agent, source owner, source or belief key, scope, initial cursor policy | source-owner acceptance and first readability |
| source subscription acceptance identity | named source or Belief owner | durable accepted, duplicate, rejected, or conflicted consumer decision | request, source contract revision, source key, scope, initial cursor policy | Agent request, readiness, and first readable view |
| genesis publication identity | Agent through Events | idempotent neutral publication of Agent genesis | Agent revision, assignment, product compilation receipt, deterministic publication key | Agent record acceptance and Graph visibility |
| Agent genesis receipt | Agent | complete native acceptance of one topology position | assignment, Agent revision, complete source-owner subscription acceptance set, genesis publication operation, installed revisions | runtime readiness and current generation |
| Agent topology receipt | PDS structure over Agent receipts | complete inert proof that every declared topology position has one exact Agent-owned genesis receipt | assignment, topology, complete Agent receipt set | participant readiness and generation publication |
| participant plan identity | activation preparation | exact structural realization dependency graph | assignment, declared participants, structural dependencies, readiness, wake, safe-point, and stop contract references | semantic Plan or runtime schedule |
| Capability preparation receipt | Capability | inert proof that selected contracts, offers, implementations, bindings, compatibility, placement, and limits form one exact realization input | contract revisions, offer revisions, implementation revisions, binding revisions, compatibility policy, placement, limits | current availability, readiness, and successful realization |
| prepared activation closure | activation preparation | exact inert aggregate offered to lifecycle | assignment, activation inputs, product compilation receipt, owner preparation references, Capability preparation receipt, Agent topology receipt, participant plan, binding revisions, authority inputs | lifecycle intent, generation, readiness, or current head |

## Compilation And Acceptance Positions

| Stage | Producer position | Consumer acceptance | Failure or incomplete evidence |
| --- | --- | --- | --- |
| product selection | durable selection citing one product revision | PDS resolves exact package and situated declaration inputs | unresolved or unauthorized selection receipt |
| package link | immutable package plus exact import closure | router validates structural completeness and owner routes | invalid package report naming component and route |
| owner validation | every required owner validates its component under its own grammar | installation may begin only after complete validation | owner-qualified rejection, no package receipt |
| owner installation | immutable owner revisions exist, potentially from retry or prior attempt | router verifies exact revision set and cross-component links | partial installation progress remains inert and unselectable |
| package completion | one content-derived complete installation receipt | product compilation aggregates the exact receipt under the selected package set | no receipt means the selected package set is incomplete |
| product compilation | one receipt binds the product revision and complete selected package-receipt set | assignment accepts the complete product compilation receipt | any missing or extra package receipt leaves the product unselectable |
| assignment | durable situated identity binds product compilation, principal, subject, perspective, branch, topology, and grants | Agent genesis and activation preparation resolve the exact assignment | conflict or unauthorized assignment receipt |
| Agent genesis | Agent accepts every declared topology position under its native identity and bindings, and every named source owner accepts its subscription requests | topology receipt cites complete Agent and source-owner receipts | partial topology, identity conflict, rejected subscription, or missing publication operation remains inert |
| activation preparation | exact physical inputs, Capability preparation receipt, participant plan, binding revisions, authority inputs, and owner receipts form one closure | `WMR-DD-06` lifecycle consumer may accept or reject it | prepared, conflicted, or incomplete closure evidence only |

## Owner-Routed Compilation

| Component class | Semantic owner | PDS responsibility | Owner acceptance position |
| --- | --- | --- | --- |
| observation and publication theory | workspace, docs, dependency security, or named product owner | preserve component body, imports, route, and required structural links | immutable owner revision and install receipt |
| Graph and Traversal vocabulary | Graph or Traversal owner | route exact declaration without interpreting it | owner revision accepted under canonical grammar |
| Belief family, evidence route, comparator, or policy | Belief | link exact component and dependencies | immutable Belief-owned theory revision |
| Curation standing and planned-operation theory | Curation or current compatibility owner | preserve exact route and required bindings | owner-validated revision with accepted operation meaning |
| Strategy construction theory | Strategy | link stable construction meaning, never a situated Plan | immutable Strategy theory revision |
| maintained conditions and directive | Agent | bind exact selected revisions to declared topology positions | Agent record revision and genesis receipt |
| Capability contracts and compatibility expectations | Capability | co-distribute contracts or requirements without declaring live offers | immutable contract or compatibility revision |
| product policy and authority classes | policy owner | preserve requested classifications and lineage | owner revision, not an effective grant |

## Wait, Wake, Fence, And Restart

| Edge | Wait | Wake | Fence | Restart source |
| --- | --- | --- | --- | --- |
| `WMR-H20` | package unresolved, invalid, missing a required owner acceptance, or product package set incomplete | exact import, route, component successor, owner result, package receipt, or selected package-set successor | product revision, selected package set, package hashes, import closures, route maps, owner grammar revisions, predecessor receipts | product declaration, package manifests, owner registries, partial install accounts, complete package receipts, product compilation receipt if present |
| `WMR-H21` | product compilation or assignment absent, topology incomplete, Agent identity conflicted, or source-owner subscription decision missing | exact compilation receipt, assignment, topology successor, Agent acceptance, source-owner decision, or publication operation | product revision, compilation receipt, assignment, topology position, Agent identity body, installed revisions, source contracts, perspective, branch | assignment, Agent records, subscription requests, source-owner acceptance receipts, genesis outbox, topology receipt |
| `WMR-H22` | prepared closure absent, incomplete, conflicted, or not yet accepted by lifecycle | exact owner preparation, Capability preparation receipt, Agent topology receipt, participant-plan successor, binding, or authority input | assignment, activation input, product compilation receipt, topology receipt, Capability preparation receipt, participant plan, binding revisions, authority inputs | all named native receipts, Capability preparation receipt, and inert prepared closure |

Polling may discover a missing producer position. It is not itself the wake or acceptance evidence.

## Failure And Upgrade Rules

- owner installation failure may leave immutable owner revisions, but no complete package receipt or selectable package head exists
- retry reuses content-identical owner revisions and the same complete receipt identity
- an existing Agent identity with a different subject, perspective, branch, topology position, provenance, or assignment is a conflict, never silent reuse
- a subscription request closes only through a source-owner accepted, duplicate, rejected, or conflicted decision; request persistence does not prove acceptance
- Agent record acceptance, subscription creation, genesis publication, Graph visibility, first readable Belief view, and runtime readiness remain distinct
- changed owner meaning creates a successor owner revision, while content-identical owner revisions remain reusable
- changed package composition creates a new package receipt and changed selected package set or product revision creates a new product compilation receipt
- every changed selected product revision or topology creates a successor assignment, changed physical bindings create a successor activation input, and any changed prepared input creates a successor prepared closure
- historical runtime products retain the exact prior product and owner-revision lineage

## Deferred Consumer

`WMR-H22` closes its producer side at an exact inert prepared activation closure. `WMR-DD-06` still owns lifecycle-intent acceptance, participant incarnation, owner readiness, current-generation publication, replacement, and retirement.
