# Documentation Freshness Router Specification

Date: 2026-08-18
Status: discovery router exercise aligned to canonical PDS boundary
Scope: fan documentation freshness through `theory::router` and record pressure on the common theory set

## Purpose

This exercise asks whether documentation freshness can attach through domain-owned routes without teaching `theory::router` about docs bodies, workspaces, providers, or runtime execution.

It is descriptive design pressure. It does not approve implementation, schema, route names, or lifecycle traits.

The current routed package lists five exact capability components for compatibility parity. Canonically, those contracts arrive through capability contribution and activation rather than PDS semantic theory.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Installing owner | Owner meaning |
| --- | --- | --- | --- |
| `docs-freshness-belief` | `world-model.belief-family.v1` | belief domain | freshness over one declared relationship |
| `docs-outcomes` | `world-model.outcome-mapping.v1` | belief domain | typed docs products admitted as evidence |
| `docs-curation` | `world-model.agent-curation-rule.v1` | Agent domain | response to uncertainty and breach |
| `docs-condition` | `world-model.agent-maintained-condition.v1` | Agent domain | source-backed freshness condition |
| `docs-strategy` | `world-model.strategy-theory.v1` | Strategy domain | settlement, prospective evidence, action-class, outcome, and constraint meaning |
| `docs-inspect` | activation capability contribution | capability domain | inspect declared source and document scope |
| `docs-draft` | activation capability contribution | capability domain | produce bounded patch artifact |
| `docs-validate` | activation capability contribution | capability domain | validate patch and evidence map |
| `docs-publish` | activation capability contribution | capability domain | publish under exact bounded effects |
| `docs-assess` | activation capability contribution | capability domain | assess published scope after change |
| `docs-authority` | assignment governance selection | authority domain | requested observation, draft, and publication posture |
| `docs-claim-policy` | `docs.claim-policy.v1` | docs domain | valid claim classes, coverage, and verification rules |

PDS declares the required action classes and activation requirements. Capability providers independently publish exact contracts. Activation selects compatible offers under assignment bindings. The router does not derive capabilities from a steward profile or infer a provider.

## Router-Owned Semantics

`theory::router` may own only:

- manifest and component envelope validation
- route lookup and route-version compatibility
- structural requirement closure
- exact owner install dispatch
- generic revision references
- exact package receipt closure
- historical exact resolution
- route-addressed diagnostics

It must not decode docs claim policy, inspect a workspace, choose a provider, interpret a freshness threshold, or validate a generated artifact.

## Owner Semantics And Cross Links

The docs route owns the exact claim-policy body. It validates claim classes, declared source evidence, coverage obligations, and owner invariants before installing an append-only revision.

The belief route owns the `content.freshness` family and docs outcome mappings. It understands how admitted docs facts support or contradict a belief without becoming the docs artifact store.

The Agent routes own curation and maintained-condition meaning. The Strategy route owns settlement and prospective evidence semantics. Strategy owns separately admitted Methods and current problem construction. Capability and execution own exact contracts and effect declarations.

Cross links are exact references. They are not copied bodies and not symbolic current heads. Expected links include:

```text
outcome mapping
→ exact docs product schemas and belief family

maintained condition
→ exact freshness family and curation policy

strategy theory
→ exact maintained condition plus abstract action and outcome meaning

capability activation
→ exact docs claim policy, compatible action classes, and assignment bindings
```

Structural linking proves that required components exist under compatible routes. Each owner proves semantic validity of the references it interprets.

## Assignment And Activation

One assignment binds an exact package receipt to one principal and declared repository scope. The assignment records requested autonomy and authority posture without storing credentials, provider endpoints, or live handles.

Activation resolves only the bindings required by selected behavior:

```text
workspace
subject scope
Agent identity
exact docs claim policy
selected capability implementations
provider only for selected model-backed behavior
publication target only when publication is selected
```

Owner preparation installs assignment-scoped subscriptions, exact selectors, admission policies, and draft invokers. Preparation remains inert.

The activation generation then starts bounded actors and adapters as participant incarnations, proves readiness, and advances the assignment current-generation pointer through an expected-prior-generation fence. Only the current ready generation admits new claims.

## Capability And Authority Closure

Semantic package selection, capability availability, implementation binding, and effective authority stay separate.

```text
required action-class semantics
∩ compatible activation contract offer
∩ assignment request
∩ principal grant
∩ runtime restrictions
→ effective invocable behavior
```

A deterministic inspection-only assignment needs no model provider. A draft-only grant cannot reach publish or merge behavior even when one physical adapter implements all three.

Filesystem effects use exact workspace and path identity. Concurrent docs and code-change operations over overlapping files are arbitrated through execution effects, not docs-specific router logic.

## Observation Admission

Workspace notifications, source snapshots, provider responses, generated evidence maps, and validation reports are untrusted inputs until their owning adapter validates them.

Admission must verify as applicable:

- assignment and package receipt
- exact docs claim-policy revision
- source and document subject identity
- source and document revisions
- activation and activation generation
- durable operation key and attempt lineage
- adapter implementation identity
- declared coverage and completeness
- artifact and evidence-map schema

Transport success and file existence prove neither freshness nor correctness. The docs domain emits a canonical product only after validation. The event layer preserves that admitted product and provenance.

Passive workspace notifications carry subscription and delivery lineage rather than a fabricated execution claim.

## Outcome Admission

The following products remain distinct:

| Product | Meaning |
| --- | --- |
| inspection result | exact source and document facts observed |
| patch artifact | candidate content exists |
| validation report | candidate satisfies declared checks |
| publication receipt | bounded repository effect occurred |
| freshness assessment | proof-carrying current docs evidence under one policy and source revision |

Only a proof-carrying freshness assessment with independently admissible lower grounds may directly support restoration of the maintained condition. The other products may be necessary evidence but cannot substitute for it. A naked `stale_probability` does not satisfy this boundary.

## Isolation Pressure

Docs freshness demonstrates that semantic isolation does not require universal process isolation. Trusted deterministic inspection may execute in process while still requiring strong assignment, binding, effect, admission, and replay isolation.

Model-backed drafting adds external transport and credential pressure. Two assignments over one workspace must retain separate:

- principals and grants
- provider bindings and budgets
- activation-local catalogs and executors
- Agent perspectives and current beliefs
- operation and attempt lineage
- admitted artifact and outcome history

A shared repository index may share immutable cache entries keyed by exact source revision. It may not share current relationship decisions, hidden provider context, or assignment cursors.

## Lifecycle Pressure

Preparation may resolve exact theory, workspace scope, provider refs, capability selectors, and publication policy. Start creates runtime handles over durable docs state.

Readiness must prove that selected workspaces, stores, provider sessions, and admission paths match the intended activation generation. Constructor success alone is insufficient.

Quiescence closes new docs claims, brings accepted publication work to an owner safe point, persists artifacts and outboxes, drains canonical events through the required watermark, and checkpoints projections. A `no_work` tick is active idle and is not a shutdown proof.

A late provider result retains its retired generation. Docs policy classifies it as rejected, historical-only, or eligible under an explicit revision rule. It never silently updates the current freshness projection.

## Interaction With Other Stewards

Docs freshness consumes canonical domain events, never direct steward calls. A dependency-security assessment or accepted code patch may create a relevant source-change observation opportunity. Docs decides whether its own standing condition changed.

Likewise, a docs violation event can be consumed independently by a codebase-quality assignment. The event carries facts and evidence, not a command or grant.

Common observations may be deduplicated at the task or capability layer when exact scope, revision, and authority permit. Deduplication does not merge maintained conditions or outcome judgments.

## Router Falsification Findings

The example rejects or narrows the common router design if:

- the router must import docs body types to prove closure
- provider identity must enter package identity
- a package profile must imply undeclared capabilities
- one successful generator invocation can directly mark freshness restored
- historical docs decisions resolve through a mutable current policy head
- two assignments must share catalogs or Agent state to reuse a workspace index
- an inspection-only assignment cannot activate without a provider
- a late retired-generation result can enter current truth without owner policy

The current design survives these pressures when the router remains inert and owner routes perform semantic validation and admission.

## Theory-Set Implications

### Common Candidate Mechanism

The example supports common candidates for exact routed components, explicit structural requirements, package receipts, assignment-local activation, capability and authority closure, generation fencing, transport-neutral result lineage, and owner admission.

It also supports a shared distinction among passive delivery, capability attempt, repository effect, and later domain outcome.

### Owner-Specific Meaning

The docs domain alone defines source-document relationships, valid claim evidence, coverage sufficiency, document artifact semantics, and freshness restoration. Those concepts should not be generalized into router grammar.

### Unresolved Bridge

The smallest unresolved bridges are:

- how a canonical declaration selects package-defined docs profiles without exposing compiled theory
- how source-document relationships are discovered without silently expanding assignment scope
- how publication safe points relate to generic execution effect leases
- which late deterministic inspection results remain admissible after a generation change
- where activation-generation and participant-incarnation receipts and the current-generation pointer are durably owned

These are discovery questions and not blockers or approved requirements.

## Non Commitments

This exercise does not approve route names, final component schemas, provider policy, lifecycle trait shape, store layout, or a migration sequence.

## Source Grounding

- [Use-Case Definition](README.md)
- [Docs Freshness Router Refactor](../../../plan/integration/docs_freshness_pds_router_refactor_design_spec.md)
- [PDS Router Design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Activation Lifecycle Fanout](../../../plan/integration/pds_activation_lifecycle_fanout_exercise.md)
