# Agent

Agent is the authority-bearing world-model entity that reconciles a directive with admitted world state. It owns Goal judgment and the durable progression of authorized Strategy Plans.

Agent does not perform Strategy search, author graph edges directly, or execute Tasks.

## Responsibilities

Agent grounds directives into Goals, decides whether observed divergence matters, requests Strategy construction, judges exact Plans, authorizes individual Plan products, and reconciles progression when knowledge or outcomes change.

Agent routes eligible Epistemic Operations to Curation. It routes eligible complete Tasks to the Execution Goal Set. It preserves causal lineage across both paths and never exposes the full heterogeneous Plan to Execution.

## Durable Progression

For each authorized Plan, Agent records product eligibility, authorization, publication, completion, invalidation, successor lineage, and Goal satisfaction state. Repeated delivery is idempotent.

A new belief revision, Curation result, execution outcome, or product-domain fact may cause reassessment. Agent may continue the Plan, authorize newly eligible products, suspend products whose premises no longer hold, or request a successor Plan from Strategy.

## Authority

Agent authorization proves that a Strategy product is permitted for this Agent and Goal. It does not prove that Curation has authored the requested knowledge or that Execution has produced the requested effect. Those owners publish their own results.

- [Directive Grounding](directive_grounding.md)
- [Genesis And Activation](genesis_and_activation.md)
- [Plan Progression](goal_curation.md)
- [Runtime Surface](runtime_surface.md)
- [Agent Contract](spec.md)
