# Documentation Freshness Stewardship

Date: 2026-08-18
Status: discovery example aligned to canonical PDS boundary
Scope: persistent stewardship of declared source and documentation relationships

## Working Model

A documentation freshness steward maintains evidence that selected documentation still describes its declared source, API, workflow, and audience. It observes change, decides whether existing evidence remains sufficient, acquires more evidence when needed, and proposes bounded repair when the maintained condition is breached.

The steward does not treat document generation as proof of freshness. A written artifact, a successful tool run, and a verified source-backed document are three different facts.

This example is the local and tightly composed end of the PDS runtime spectrum. It tests whether one semantic package can use trusted in-process behavior for deterministic inspection and an external model provider for selected drafting behavior without changing package meaning.

## Domain Boundary

The docs domain owns:

- source-to-document claim policy
- evidence-map and verification product meaning
- document artifact and publication state
- validation of docs runtime output before admission
- the meaning of stale, current, incomplete, and contradicted documentation

The world-model domains own belief families, evidence mapping, Agent curation, maintained conditions, and Strategy semantics. Execution owns capability contracts, claims, effects, and authority enforcement. Theory owns package routing and exact receipt closure.

The steward does not own source truth, merge authority, repository history, provider transport, or generic planning semantics.

## Standing Condition

For each declared source-document relationship:

```text
required source coverage is represented
and current verification cites the active source revision
and admitted claims satisfy the selected docs claim policy
and no unresolved contradiction breaches the policy threshold
```

Unknown coverage or stale verification does not satisfy the condition. A non-material source change may be tolerated only through an explicit policy-supported decision with durable rationale.

## Subjects And Scope

Primary subjects include:

```text
repository
workspace subtree
source artifact
API or workflow
document
document section
source-document relationship
source revision
document revision
```

Assignment scope selects the repository or subtree and the declared relationships. Discovery of new candidate relationships may produce owner facts, but it does not silently expand the assignment.

## Observations And Evidence

Useful observations include:

- source and document changes
- exact source and document revisions
- source-to-claim evidence maps
- bounded inspection reports
- generation and validation reports
- human acceptance, rejection, or correction
- later repository changes that invalidate prior verification

Evidence retains source identity, revision, relationship, policy revision, assignment, activation generation, and producer lineage. Generated text is evidence of an available artifact, not evidence of correctness.

## Beliefs

The core belief family is `content.freshness`. Supporting concerns include source-document coverage and verification freshness.

The useful question is not merely whether the document changed recently. It is how strongly current admitted evidence supports that the document accurately represents its declared subject and audience.

## Observation Choices

The steward may:

- inspect changed source regions
- compare source and document lineage
- hydrate prior accepted docs decisions
- generate a bounded evidence map
- run deterministic documentation checks
- request human clarification when source intent is ambiguous

Observation remains selective. A full repository regeneration is one possible method, not the semantic definition of freshness.

## Actions

Candidate interventions include:

- tolerate a proven non-material change
- regenerate a bounded document or section
- update a source-to-claim evidence map
- validate a proposed patch set
- publish an artifact to an isolated branch
- open a draft change
- request review

Mutation, publication, pull-request creation, and merge remain distinct effects with distinct grants.

## Outcomes

Mechanical completion may report that a file was written or a draft change was opened. Restoration requires later admitted evidence that:

- the artifact schema is valid
- declared scope is covered
- claims cite current source evidence
- validation passed under the exact claim policy
- the `content.freshness` belief entered its restore region

A rejected draft can still be useful outcome evidence. It may revise action cost, method suitability, or the interpretation of an ambiguous source-document relationship.

## Authority

The effective action set is the intersection of assignment request, principal grant, compatible activation capabilities, runtime policy, and current restrictions. The package defines governance and action-class meaning but does not grant or select exact behavior.

Typical authority boundaries are:

| Action | Typical posture |
| --- | --- |
| inspect declared workspace scope | automatic observation |
| run bounded deterministic checks | bounded execution |
| use selected model provider | explicit provider and budget grant |
| create an isolated patch | draft authority |
| publish a draft branch | separate publication authority |
| merge or deploy | outside the default steward grant |

The same workspace under two assignments does not imply shared authority, provider credentials, Agent perspective, or current belief state.

## Why PDS

The simpler baseline is scheduled documentation generation plus a docs-writing workflow. That baseline should remain preferred when every relevant source change deterministically regenerates one known artifact and fixed validation is sufficient.

PDS earns its cost when the responsibility needs selective evidence acquisition, persistent source-document lineage, uncertainty-aware freshness, bounded repair, conflict with concurrent repository change, independent verification, and durable outcome history.

## Physical Runtime Variants

The same semantic package may admit several placements:

| Placement | Useful behavior | Required proof |
| --- | --- | --- |
| linked local adapter | deterministic inspection and evidence mapping | path scope, panic containment, result validation |
| owned subprocess | converter or documentation generator | executable identity, environment allowlist, filesystem grants |
| remote request service | model-backed drafting or validation | endpoint identity, credential isolation, response lineage, output admission |
| shared local sidecar | reusable repository index | assignment namespace, revision identity, queue fairness, crash sharing |

Provider-free inspection must remain possible when only deterministic capabilities are selected. Placement and provider bindings are activation concerns and do not participate in package identity.

## Interaction With Other Stewards

Docs freshness may consume admitted facts about source changes, dependency graph changes, API changes, or accepted patches. It does not receive commands from security or codebase-quality stewards.

Its own admitted stale-document fact may independently wake a code-change steward. The receiving assignment must already have its own maintained condition and authority.

## Explicit Non Commitments

This example does not approve:

- a final PDS package schema
- one universal documentation ontology
- automatic merge authority
- mandatory model use
- one process topology
- hidden scope expansion from repository discovery
- direct steward-to-steward commands
- successful generation as proof of freshness
- a final activation lifecycle trait

## Router Exercise

The package fanout and theory pressure are developed in [Router Specification](router_spec.md).

## Source Grounding

- [Documentation Freshness Catalog Entry](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [Docs Freshness Use Case](../../../use_cases/docs_freshness.md)
- [Software Quality Stewardship](../software_quality.md)
- [Docs Freshness Router Refactor](../../../plan/integration/docs_freshness_pds_router_refactor_design_spec.md)
- [PDS Router Design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Activation Lifecycle Fanout](../../../plan/integration/pds_activation_lifecycle_fanout_exercise.md)
