# Lore And Canon Router Spec

Date: 2026-08-18
Status: discovery router exercise aligned to canonical PDS boundary
Scope: candidate lowering of lore and canon stewardship through `theory::router`

## Purpose

This exercise asks whether identity and canon maintenance can use the common PDS package and activation path while lore semantics remain owned outside theory and root assembly.

The route map is illustrative. Candidate routes not already present in the router design are marked explicitly and remain unresolved.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Owner | Meaning and required links |
| --- | --- | --- | --- |
| `lore-identity-match` | `world-model.belief-family.v1` | belief | Evidence support that mentions denote one persistent subject |
| `lore-claim-support` | `world-model.belief-family.v1` | belief | Support, contradiction, supersession, and temporal validity |
| `lore-definition-completeness` | `world-model.belief-family.v1` | belief | Profile-relative coverage of required claim classes |
| `lore-definition-freshness` | `world-model.belief-family.v1` | belief | Alignment between a definition and active accepted claims |
| `lore-conflict-state` | `world-model.belief-family.v1` | belief | Material unresolved claim or identity conflict |
| `lore-maintained-conditions` | `world-model.agent-maintained-condition.v1` | Agent | Identity stability, provenance, conflict visibility, and freshness |
| `lore-curation` | `world-model.agent-curation-rule.v1` | Agent | Observe, tolerate, draft, approve, restore, or escalate |
| `lore-strategy` | `world-model.strategy-theory.v1` | Strategy | Settlement, prospective evidence, abstract action, outcome, and constraint meaning |
| `lore-capabilities` | activation capability contribution | capability and execution | Exact contracts for source reads, extraction, queries, merge proposals, and refresh |
| `lore-authority` | assignment governance selection | Agent and execution | Distinguishes draft extraction, alias, merge, split, correction, and source mutation |
| `lore-outcomes` | `world-model.outcome-mapping.v1` | outcome and belief | Maps later references, corrections, lineage checks, and freshness evidence |
| `lore-policy` | `lore.policy.v1` | lore domain | Source authority, entity classes, claim scope, merge thresholds, and definition profiles |
| `lore-evidence-routes` | candidate `world-model.evidence-mapping.v1` | belief | Exact mappings from admitted lore products into selected families |

The package structurally links every evidence mapping and maintained condition to exact belief-family components. Strategy semantic theory names settlement, action-class, and outcome meaning. Separately admitted Methods cite exact activation capabilities. Assignment governance names requested state-changing action classes. The lore owner validates claim scope, source authority, identity policy, and definition-profile meaning.

The package carries no current entities, mentions, claims, definitions, contradictions, or merge decisions. Those remain domain runtime products.

## Owner Semantics And Cross Links

The lore owner distinguishes mention, identity candidate, accepted entity, source claim, current interpretation, and derived definition. Events preserve admitted products and provenance. Graph owns generic materialization and traversal. Belief owns declared epistemic questions. Agent owns maintained-condition curation. Execution owns action and effect lifecycle.

```text
exact source product
→ lore owner admission
→ graph projection and provenance
→ exact evidence mapping
→ belief revision
→ maintained-condition decision
→ selective observation or intervention
→ admitted lore result
→ outcome evidence
```

Graph projection registration and fact-to-event hydration are unresolved domain bridges. They are not silently introduced as settled router routes. A future route could install state-free projection theory only if the graph owner publishes such a contract.

The linker must reject a merge method that lacks an identity policy and exact effect class, a definition refresh without an evidence mapping, or an outcome mapping that cannot recover the exact source and decision lineage.

## Assignment And Activation

One assignment binds an exact package receipt to one principal, corpus subject, perspective, branch, requested authority, and principal grant. Separate private-notes and shared-canon projections remain separate assignments even when they read the same corpus.

Activation may bind a served narrative endpoint, source credential, extractor, graph query port, local corpus path, or definition generator. Only the owners that declare a binding receive it. An extractor credential does not flow into graph, belief, or unrelated assignments.

Preparation creates exact subscriptions, source refs, capability selectors, and assignment-local executor state with admission closed. Current publication occurs only after source identity, query and hydration ports, selected external adapters, and owner admission handlers prove generation-scoped readiness.

## Capability And Authority Closure

Candidate exact capabilities include:

```text
lore.read_source_passage
lore.extract_source_region
lore.query_mentions_and_relations
lore.propose_alias
lore.propose_merge
lore.execute_approved_merge
lore.execute_approved_split
lore.refresh_definition
lore.request_clarification
lore.verify_definition
```

The package declares action-class and outcome meaning, while activation supplies compatible exact contracts and binds one implementation for each current offer. Effective authority separately decides whether one invocation is allowed.

Extraction can produce provisional mentions and claims without merge authority. A definition generator can produce a candidate artifact without source mutation authority. A technical ability to merge records does not bypass principal approval or current runtime restrictions.

## Observation And Result Admission

A source watcher emits passive delivery lineage with the exact assignment subscription, activation generation, source revision, authenticated adapter identity, and cursor. It does not create an execution claim.

An invoked extraction or reconciliation action carries an exact source revision, bounded passage set, durable operation key, attempt id, activation generation, participant incarnation, capability ref, and assignment scope.

Lore owner admission validates:

- source identity, revision, and exact evidence coordinates
- corpus, perspective, and branch scope
- producer and extractor identity
- distinction among mention, candidate, claim, editorial decision, and definition
- temporal and narrative scope
- completeness and schema
- operation and attempt lineage or passive delivery lineage

A high similarity score enters as evidence for an identity candidate. It never directly commits an entity merge. HTTP success or subprocess exit zero never proves a canon claim.

## Isolation And Lifecycle Pressure

This example pressures state and replay isolation more than hard process containment. Shared indexes are plausible, but tenant namespace, source revision, assignment perspective, private annotations, and current projection must remain explicit.

An index cache may be shared only if its key and response lineage preserve the exact corpus and source revision. Hidden index updates cannot reinterpret historical decisions. A remote extractor receives only the bounded admitted passages needed for one attempt.

A source revision or credential change creates new operational lineage and may create a new activation generation without changing package identity. An equivalent restart creates a new participant incarnation. A late extraction from an older generation or incarnation may remain historical evidence but cannot silently refresh the current projection.

Historical interpretation replay reads admitted source, claim, belief, and decision records. It does not require the original extractor or source watcher to remain online.

## Interaction With Other Stewards

A roleplay continuity steward may consume accepted lore projections while maintaining its own character perspective and disclosure policy. Lore truth does not automatically become character knowledge.

Several lore stewards may maintain different branches, editorial policies, or definition profiles over one source corpus. They may share immutable source evidence but cannot share grants, current selections, merge decisions, or private annotations implicitly.

A freeform narrative database producer may own source documents, exact spans, extraction runs, and editorial acceptance. In that arrangement the lore steward consumes those products through served contracts and does not recreate a second canonical copy.

## Router Falsification Findings

The router design is weakened or falsified for this case if:

- installation requires reading the live corpus or running extraction
- theory or root must understand entity classes, aliases, claims, or merge semantics
- a graph edge becomes the sole authoritative representation of a qualified lore relation
- current owner heads reinterpret historical package or source meaning
- one shared index collapses assignment scope or private projections
- extraction confidence is treated as editorial acceptance or belief confidence
- an external result bypasses lore owner admission
- a merge or correction destroys source and decision lineage

The static fanout remains coherent. The strongest missing bridge is a domain-owned path from externally authored narrative products through generic graph projection and exact source hydration into lore evidence.

## Theory Set Implications

### Common Candidate Mechanism

- exact package and owner revision closure
- standing conditions over durable domain subjects
- selective observation before intervention
- bounded capabilities with independent authority
- perspective, branch, source, and generation lineage
- owner-admitted results and non-destructive replay
- derived-artifact freshness verified by later evidence

### Owner Specific Meaning

- entity and mention identity
- source and canon authority ordering
- qualified claim scope and valid time
- alias, merge, split, contradiction, and supersession policy
- definition profile and material freshness
- editorial acceptance

These meanings remain lore or external narrative products. They are not core PDS variants.

### Unresolved Bridge

- generic domain projection registration owned by graph
- exact event hydration across the served boundary
- ownership split between external editorial truth and Meld stewardship judgment
- incremental publication of one semantic source revision across bounded requests
- outcome proof for identity correctness before later references arrive

## Source Material

- [Use Case Readme](README.md)
- [Original Lore Transcriber Example](../lore_transcriber.md)
- [Freeform Narrative Database Constraint Case](../freeform_narrative_database.md)
- [Expression Catalog](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [PDS Router Detailed Design](../../../plan/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Stewardship Package Model](../../package_model.md)
- [Stewardship Facet Protocol](../../facet_protocol.md)
- [Proposal Status](../../proposal_status.md)
