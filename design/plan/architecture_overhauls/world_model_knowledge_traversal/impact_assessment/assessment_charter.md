# Recursive Impact Assessment Charter

Date: 2026-08-20

Status: frozen review charter

## Concern

Assess the current-code impact of redesigning world-model Strategy from one executable candidate into a heterogeneous causal Plan over Goal obligations, complete Tasks, and bounded Epistemic Operations. Agent retains root Goal authority and Plan progression. Strategy constructs and reconstructs the semantic Plan during reconciliation. Curation executes epistemic operations and publishes shared results through Events. Traversal materializes promoted knowledge. Execution receives complete executable Goal and Task products only.

## Terminology

Use reconciliation for the continuing process in which admitted knowledge causes Agent and Strategy to reassess and reconstruct a Plan. Do not use repair as Meld architecture terminology.

Goal names desired state. Task and Epistemic Operation are distinct discharge products. Plan is a causal graph of desired conditions, discharge products, and dependencies.

`EventEnvelope` is a durable carrier. Curation owns Epistemic Operation semantics. Traversal owns graph materialization and read, not semantic authorship.

## Caller Limits

Execution remains ignorant of Epistemic Operations, Curation, Traversal, Belief, and Strategy reasoning. Only complete executable products cross its Goal Set seam.

`meld-lang` remains permissive shared vocabulary. Do not recommend semantic enforcement there.

Do not recommend changes outside world-model unless current evidence proves a specific required boundary adaptation.

Producers own the meaning they publish. Consumers protect transport shape and state they own without re-proving producer semantics.

The assessment is evidence for later canonical requirements. It must not create those requirements or an implementation sequence.

## Evidence Packet

The primary proposal is [Strategy Is Bigger Than A Task](../strategy_plan_redesign.md).

The neutral current state is [Current Code Ground Map](../current_code_groundmap.md).

The two independent reviews are [Goal Language And Strategy Plan Review](../reviews/goal_language_strategy_plan_review.md) and [Event Backed Epistemic Operations Review](../reviews/event_backed_epistemic_operations_review.md).

The supporting domain assessment is [Traversal And Curation Assessment](../curation_assessment.md).

Reviewers must validate against current code and installed theory linked by those documents. Discovery proposals are not implemented evidence.

## Recursive Method

Begin with the complete current top-level domain set for the assigned crate. Include every domain once and record explicit `none` findings.

Within every affected domain, regenerate the current public domain-entity set from contracts, module exports, installed theory, and direct runtime products. Assess those entities before writing any domain conclusion.

Use the standard change postures `reuse unchanged`, `extend existing`, `new local behavior`, `adapter only`, and `not needed`.

After entity assessment, synthesize upward in this order:

```text
domain entities
-> owning sub-crate domain
-> owning crate
-> architecture-wide parent synthesis
```

The root synthesis owner performs the final architecture-wide step after every crate report is frozen.

At every level, separate the runtime path, behavior that must change, likely implementation writes, adapters, and reused unchanged domains.

Do not infer that every runtime participant needs code changes.

## Entity Evidence Shape

Each affected domain must record the entity, current owner, current behavior, required relationship to the proposed architecture, change posture, boundary risk, and exact evidence.

Entities are public semantic contracts, durable records, actor inputs and outputs, installed theory products, or domain-owned state transitions. Do not turn the entity pass into a private function inventory.

## Review Budget

Each crate reviewer may use up to twenty-four batched source inspections and sixty additional source items. Reaching the budget ends discovery and lowers confidence rather than silently widening the pass.

## Required Reviewer Output

Each reviewer writes one evidence document containing the complete crate domain snapshot, explicit pass-one domain sweep, frozen affected-domain set, entity-level assessments for every affected domain, upward domain synthesis, crate synthesis, separated scopes, explicit non-integration findings, evidence confidence, and unresolved questions.

The report must remain evidentiary. It must not advocate an implementation plan.
