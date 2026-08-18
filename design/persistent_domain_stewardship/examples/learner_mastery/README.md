# Learner Mastery Steward

Date: 2026-08-18

Status: discovery example aligned to canonical PDS boundary

Scope: persistent mastery, prerequisite, misconception, retention, and progression stewardship for one declared learner scope

## Working Intent

Maintain an evidence-grounded view of what an assigned learner can do independently, which prerequisites remain ready, which misconceptions are plausible, and whether learning transfers beyond the immediate exercise.

The steward remains responsible after one answer, lesson, or course session ends. It chooses when to observe, teach, tolerate uncertainty, defer progression, or ask an educator for help.

## Domain Boundary

The bounded domain contains:

- one learner identity or a declared cohort partition
- selected concepts, skills, and prerequisite relations
- assessment items, practice episodes, explanations, and artifacts
- mastery, misconception, retention, and transfer questions
- declared instructional policy, progression policy, and protected learning load

The steward may form evidence-backed learning beliefs and select instructional activities inside its assignment. It does not own official grades, credentials, accommodations, medical or psychological conclusions, discipline, curriculum accreditation, or institutional enrollment.

Stable learner and course identities should come from the learning platform or another declared identity authority. The learning management system, assessment service, credential store, and institutional policy repository remain external systems of record for the facts they own. Meld preserves admitted source claims and lineage. It does not turn a derived mastery belief into an official score.

Two assignments may concern the same learner under distinct educators, courses, perspectives, or grants. Shared evidence does not merge their policies, authority, current beliefs, or instructional decisions.

## Standing Condition

The candidate maintained condition is:

```text
required prerequisite readiness is supported by sufficiently current evidence
and suspected misconceptions remain explicit until differentiated
and progression stays within declared mastery, load, and policy bounds
and important claims eventually receive retention or transfer evidence
```

One correct response can restore neither durable mastery nor transfer. A successful activity only creates new evidence for later evaluation.

## Observations And Evidence

Candidate observations include answers, solution steps, explanations, projects, assessment results, response timing, hint use, later recall, and transfer to unfamiliar problems.

Evidence needs more than a value. Admission should preserve:

- learner and concept scope
- source and assessment identity
- item and rubric revision
- response time and valid time
- whether assistance, collaboration, or hints were available
- delivery lineage and source revision
- provenance and access labels

Official score records are authoritative only for the recorded score under their stated rubric. They are evidence, not authority, for durable mastery. Generated questions and model judgments are likewise source claims until the learning owner validates their schema, scope, rubric, and evidence class.

Silence is not failure. Missing retention evidence means unknown retention, not forgotten material. Conflicting modalities may support several misconception hypotheses rather than one forced conclusion.

## Observation And Intervention Choices

The steward may choose to:

- ask a diagnostic question
- request an explanation or worked step
- select a discriminating exercise
- inspect a prior artifact
- retrieve prerequisite evidence
- teach or demonstrate
- vary representation
- assign bounded practice
- revisit prerequisite material
- defer progression
- recommend educator review

These actions differ in learner effort, information value, delay, and risk of overloading or cueing the answer. The package should express their meaning and constraints. It should not encode one fixed lesson workflow.

## Outcome Semantics

Immediate correctness is weak verification. Stronger evidence arrives through later unprompted recall, transfer, independent performance, and stable prerequisite use.

Outcomes are delayed and confounded by outside instruction, collaboration, fatigue, item familiarity, motivation, accessibility, and changing curricula. Later success may increase confidence that an intervention helped. It cannot prove that one activity caused mastery. Later failure may reveal weak retention without proving the original intervention was harmful.

Outcome records should therefore distinguish activity completion, admitted learning evidence, maintained-condition restoration, and uncertain attribution.

## Authority And Safety

A principal such as an educator, guardian under applicable policy, learner, or institution grants the assignment. Effective authority is the intersection of the exact assignment request, principal grant, current policy, and dispatch-time restrictions.

Typical authority may permit bounded content retrieval, diagnostic selection, practice assignment, and recommendations. Grading, credentialing, accommodation changes, sensitive profiling, external disclosure, and high-impact progression decisions remain denied or separately approved.

The steward must not infer broader authority from access to a capable tutoring provider. Learner records and provider prompts require assignment-scoped binding and disclosure controls.

## Why PDS

Spaced repetition, knowledge tracing, fixed adaptive sequencing, or an assistant with memory remains the required baseline.

PDS is justified only when the use case needs a standing responsibility across concepts and sessions, selective evidence acquisition, explicit uncertainty and contradiction, reconstructable instructional choices, bounded authority, and later retention or transfer feedback. If fixed sequencing performs equivalently in correctness, cost, safety, and inspectability, this example should narrow to that baseline.

## Physical Runtime Variants

The same semantic package could use:

- a trusted in-process scorer for deterministic item formats
- a subprocess for artifact analysis
- a shared local content index with learner-scoped queries
- a remote tutoring or model service
- a persistent learning-platform connector that delivers assessment changes

Placement does not alter package identity or evidence meaning. Each realization must still prove binding, disclosure, resource, admission, and replay isolation. A persistent connector must retain subscription, cursor, source revision, activation, and delivery lineage. A remote provider response never becomes mastery truth merely because transport succeeded.

## Explicit Non Commitments

This example does not commit:

- a final user-facing profile syntax
- a universal mastery scale or inference algorithm
- one assessment ontology
- autonomous grading or credentialing
- a required model provider
- a specific process or service topology
- causal attribution of learning gains to one intervention
- one storage owner for assignment or activation lifecycle

The route names in the companion specification are candidate uses of the proposed router contract, not frozen public schemas.

## Read With

- [Router pressure specification](router_spec.md)
- [Expression catalog](../pds_expression_catalog.md)
- [Compilation layer span](../compilation_layer_span.md)
- [Use-case decomposition](../../use_case_decomposition.md)
- [PDS Router design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation and runtime portability](../../isolation_and_runtime_portability.md)
- [Runtime lifecycle and quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
