# Learner Mastery Router Pressure Specification

Date: 2026-08-18

Status: discovery router exercise aligned to canonical PDS boundary

Scope: fan learner mastery through the proposed `theory::router` package, assignment, activation, isolation, and admission contracts

## Purpose

Test whether learner mastery can compile into domain-owned routed components without adding learner branches to the router or root runtime. This document records a plausible package shape and the theory pressure it exposes. It is not an implementation plan or frozen schema.

## Layering

The principal approves learner, curriculum scope, desired mastery conditions, autonomy, budget, escalation, and verification posture. The package expands those selections into exact semantic components. Installation preserves inert meaning. Assignment supplies principal, learner scope, perspective, branch, and grants. Activation supplies physical bindings and selected implementations. Answers, beliefs, activities, and outcomes remain runtime or external state.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Owner meaning | Structural requirements |
| --- | --- | --- | --- |
| `learning-policy` | `learning.policy.v1` | concept identity, prerequisite semantics, assessment evidence classes, progression and protected-load policy | none |
| `mastery-belief` | `world-model.belief-family.v1` | mastery question over learner and skill | `learning-policy` |
| `prerequisite-belief` | `world-model.belief-family.v1` | readiness question over prerequisite edges | `learning-policy` |
| `misconception-belief` | `world-model.belief-family.v1` | explicit competing misconception hypotheses | `learning-policy` |
| `retention-belief` | `world-model.belief-family.v1` | decay and delayed recall question | `learning-policy` |
| `transfer-belief` | `world-model.belief-family.v1` | independent use in unfamiliar contexts | `learning-policy` |
| `learning-outcomes` | `world-model.outcome-mapping.v1` | admitted diagnostic, activity, retention, and transfer evidence | all belief components and `learning-policy` |
| `learning-curation` | `world-model.agent-curation-rule.v1` | observe, tolerate, teach, defer, or escalate posture | all belief components |
| `learning-condition` | `world-model.agent-maintained-condition.v1` | breach and restore semantics for readiness and progression | `learning-curation` and all belief components |
| `learning-strategy` | `world-model.strategy-theory.v1` | settlement, prospective evidence, instructional action, outcome, and constraint meaning | `learning-condition` and `learning-policy` |
| `learning-capability-*` | activation capability contribution | one exact published contract per scorer, retrieval, diagnostic, teaching, or escalation action | compatible learning action classes |
| `learning-authority` | assignment governance selection | requested action classes and explicit denials | `learning-policy` |

`learning.policy.v1` is a candidate domain-owned route, not an established common router route. If existing world-model owner schemas can carry all learning evidence semantics through narrow public contracts, the dedicated route may shrink. The router must not interpret its body either way.

The router proves structural closure over semantic components and exact owner refs. Activation proves that its capability snapshot offers contracts compatible with required learning action classes. Strategy, learning, capability, and execution owners prove semantic compatibility through public contracts.

## Owner Semantics And Cross Links

The learning owner defines concept identity, prerequisite graph meaning, rubric lineage, evidence classes, assistance context, and progression policy. The belief owner defines belief revision contracts but does not invent what an assessment proves. The Agent owner defines standing response, maintained-condition mechanics, construction policy, and candidate judgment without inventing educational meaning. Strategy owns candidate construction and verification. Execution owns exact capability contracts, effect claims, and dispatch authority checks.

Cross-owner checks need narrow public views:

- each belief family cites learning-owned dimensions and admissible evidence classes
- the outcome mapping cites exact belief revisions and learning result classes
- the maintained condition cites exact planner-facing belief projections
- Strategy cites exact condition and capability refs
- authority policy names action classes compatible with exact capability contracts

No root validator should compare rubrics, mastery thresholds, or progression meaning.

## Assignment And Activation

A candidate assignment binds:

```text
exact package receipt
+ educator or institutional principal
+ exact learner and curriculum scope
+ instructional perspective and branch
+ requested authority ref
+ principal grant ref
```

The learning-platform learner identity and course revision are source bindings, not package identity. A second educator may assign the same package to the same learner under a distinct perspective and grant.

Candidate activation bindings include learning-platform read access, artifact access, content catalog, assessment service, tutoring provider, notification endpoint, and bounded resource budget. Each owner contributor receives only its declared binding refs. Preparation produces inert closure. Runtime objects start with admission closed, prove readiness, and become live only through conditional current-generation publication.

An activation may be unable to realize a remote tutoring capability while still installing the package successfully. It fails closed rather than silently selecting an unapproved provider or narrowing verification.

## Capability And Authority Closure

Instructional action requires the complete chain:

```text
learning owner defines action meaning
→ activation supplies compatible exact capability contracts
→ Strategy selects one current contract in a candidate
→ candidate lineage pins the contract
→ assignment requests the action class
→ principal and current policy grant it
→ execution revalidates the intersection at dispatch
```

Access to a model that can score, teach, or message does not authorize grading, credentialing, sensitive profiling, or disclosure. Those action classes remain absent or explicitly denied.

## Observation And Result Admission

A request-response diagnostic carries assignment, receipt, activation generation, participant incarnation where available, exact capability ref, durable operation key, dispatch claim, and attempt id. A retry keeps the semantic operation key and creates a new attempt.

A learning-platform event is passive delivery. It carries an admitted subscription, source cursor or revision, adapter delivery id, authentication lineage, assignment, activation generation, and participant incarnation. It does not invent an execution claim.

Before admission, the learning owner validates learner and concept scope, item and rubric revision, assistance context, completeness, source class, privacy labels, and lineage. Canonical append preserves the admitted claim. Only then may belief and outcome owners consume it.

Late retention evidence may remain valuable after an activity ends. It is not automatically current after activation replacement or scope change. Owner policy must classify it as rejected, historical-only, or eligible under an explicit late-result rule.

## Isolation And Lifecycle Pressure

Learner mastery requires strong normative, binding, state, admission, and replay isolation even when content indexes or provider infrastructure are shared. Prompt caches, model conversations, artifacts, and learner identifiers must not cross assignments.

The long outcome horizon pressures lifecycle semantics. A steward may be actively idle while waiting for a future assessment or retention deadline. Justified idle requires durable wait refs and a viable wake path, not merely a healthy process.

The lifetime review proposes separating assignment-wide activation generation from participant incarnation. That distinction matters here because restarting one learning-platform connector should not invalidate an unrelated local scorer when bindings and semantics are unchanged. The exact split remains an unresolved bridge against earlier restart-always-advances-generation language.

Deactivation closes new admission and fences subscriptions. Later platform events retain retired lineage. Retirement cannot promise that a remote callback will never arrive.

## Interaction With Other Stewards

A learner steward may exchange narrow public facts with curriculum, accessibility, scheduling, or wellbeing stewards. It must not reach into their internal stores.

The learning steward can publish a request for educator review or a claim that prerequisite evidence is insufficient. It cannot diagnose health, change accommodations, award credentials, or let another steward silently rewrite its mastery state. Shared evidence retains source provenance and each assignment performs its own admission and interpretation.

## Router Falsification Findings

The example falsifies the design if:

- the router needs a learner-specific branch
- a package capability implicitly grants grading or disclosure authority
- a platform score is admitted as mastery without owner semantics
- a remote model session becomes hidden learner state required for replay
- delayed retention evidence cannot cite the original decision-time package and activity lineage
- shared provider placement merges learner scope, policy, or state
- one connector restart forces unrelated participant replacement without semantic need
- the maintained condition reduces to completing a lesson workflow

The example weakens the case for PDS when fixed adaptive sequencing produces equivalent learning, safety, and audit outcomes.

## Theory-Set Implications

### Common Candidate Mechanism

The existing common shape remains plausible: exact routed semantic components, owner validation, package receipt closure, assignment-scoped authority, inert preparation, readiness-gated activation, activation-local capability snapshots, Strategy selection, durable operation lineage, passive delivery lineage, domain admission, and later outcome evidence.

Delayed verification strengthens the need to keep task completion separate from domain restoration and to retain decision-time semantic lineage beyond one activation generation or participant incarnation.

### Owner-Specific Meaning

Concept identity, prerequisite relations, rubric semantics, assistance context, mastery and misconception evidence classes, progression policy, and retention interpretation belong to learning-owned theory or explicit learning contracts. They should not become generic fields in `theory::router`.

### Unresolved Bridge

The theory set does not yet settle:

- whether learning needs a dedicated policy route or can link through existing owner contracts
- how a coherent assessment and content revision view is named
- which late learner observations remain admissible after scope or consent changes
- how participant incarnation relates to activation generation
- how privacy deletion duties coexist with durable decision lineage
- which authority owns learner identity merge, correction, and withdrawal

## Source Trail

- [Use-case narrative](README.md)
- [PDS Router design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation and runtime portability](../../isolation_and_runtime_portability.md)
- [Runtime lifecycle and quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
- [Use-case decomposition](../../use_case_decomposition.md)
- [Compilation layer span](../compilation_layer_span.md)
