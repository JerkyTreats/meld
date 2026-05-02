# World Model Agent

Date: 2026-05-01
Status: active
Scope: perspective-scoped agent identity and world-model ownership above belief and below execution commitment

## Thesis

`world_model/agent` defines the Agent as a world-model concern before it becomes an execution concern.

The Agent is the fulcrum from belief to action.
It does not dispatch tasks or own runtime control.
It owns the perspective that determines which facts are trusted, which uncertainty matters, which regime concerns are relevant, and which planner-facing world-model view should be published for that perspective.

This area exists because shared graph truth is not the same as shared belief.
Many Agents may consume one shared event and graph substrate while producing different belief views, regime sensitivities, and action-relevant projections.

## Boundary

`world_model/agent` owns:

- agent identity as a world-model perspective anchor
- perspective-scoped belief ownership
- evidence policy and trust profile
- observation scope and branch scope
- regime sensitivity profile
- planner-facing world-model view assembly for one perspective
- execution handoff inputs as shaped world-model outputs

`world_model/agent` does not own:

- task graphs
- dispatch
- continuation
- repair
- runtime control state
- provider execution
- canonical event append

## Relationship To Other World Model Domains

`graph` is shared substrate.
It does not become agent-private because one Agent distrusts or ignores part of it.

`belief` produces perspective-scoped posterior state.
`agent` defines which perspective is being served and which belief views should be assembled for it.

`causation` may expose effect summaries that differ by the intervention and measurement assumptions relevant to the Agent.

`regime` may expose structural uncertainty that one Agent treats as central and another treats as tolerable.

`planner` remains the final projection layer that turns these concerns into action-relevant world-model reads.

## Relationship To Execution

The Agent is not the execution runtime.

Within `world_model`, the Agent owns epistemic perspective.
Within `execution`, the Agent becomes a consumer identity, goal owner, or policy context that receives shaped world-model views and commits to work.

The boundary is:

- `world_model/agent` decides how the world should be interpreted for this perspective
- `execution` decides what work should run for that perspective

## ECS Note

The ECS interpretation for this domain lives in [Agent ECS](ECS.md).

The important point is that Agent should consume the heavier systems of the other world model domains and assemble them into one perspective-scoped handoff, rather than re-owning their internal logic.

## Core Design Rule

The Agent should be the owner of perspective, not the owner of truth.

That means:

- graph truth stays shared and replayable
- belief may diverge by perspective
- regime sensitivity may diverge by perspective
- planner-facing world-model views may diverge by perspective
- execution still receives one shaped view per consuming perspective

## Multi-Agent Requirement

This domain is where the world model satisfies the `En Masse and At Will` requirement.

The architecture should support:

- one shared event and graph substrate
- many Agent entities over that substrate
- sparse and divergent attached belief state
- independent planner-facing projections
- stable replay and audit across all Agents

The main payoff of an ECS-shaped internal substrate is here:
many Agents can share one identity and provenance foundation while carrying sparse, divergent, mutable world-model state without forcing one rigid record for every perspective.

## First Slice

The first slice should remain narrow.

It should define:

- one Agent identity anchored to `DomainObjectRef`
- one explicit perspective key
- one evidence and trust policy surface
- one branch and observation scope surface
- one path from belief views to planner-facing projection for that Agent

It should defer:

- full multi-Agent synchronization strategy
- shared planning between Agents
- execution-owned goal and policy semantics
- any requirement that other crates adopt ECS vocabulary

## Read With

- [World Model Domain](../README.md)
- [World Model Vision](../VISION.md)
- [World Model Planner](../planner/README.md)
- [Agent ECS](ECS.md)
- [World Model Belief](../belief/README.md)
- [Belief Microarchitecture](../belief/microarchitecture.md)
- [Knowledge Graph ECS Decision Memo](../belief/knowledge_graph_ecs_decision_memo.md)
- [Execution Domain](../../execution/README.md)
