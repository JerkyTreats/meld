# Cognitive Architecture

Date: 2026-04-12
Status: active
Scope: full loop design for sensory, world state, execution, and shared temporal coordination

## Thesis

This area defines the system as an open-world loop rather than a task-only executor.

```mermaid
flowchart LR
    O[observe] --> S[sensory]
    S --> P[event spine]
    P --> W[world state]
    W --> K[knowledge graph]
    K --> E[execution]
    E --> P
```

The durable positions are:

- full world state is never fully known
- observation is continuous and diff-native
- world state integrates observations into a temporal world model
- execution acts against the current world model and republishes outcomes
- the spine is the shared temporal substrate across all concerns

## Boundary

`cognitive_architecture` is not a replacement for `goals`, `control`, `task`, `capability`, or `provider`.

This area does own:

- the cross-domain loop definition
- the sensory and world-state seams that do not yet have durable homes
- the world-model requirement that action be grounded in current belief
- the spine requirements needed for genuine multi-process coordination

## Durable Structure

- [Observe Merge Push](observe_merge_push.md)
  founding prompt and response index
- [Knowledge Graph ECS Decision Memo](world_state/belief/knowledge_graph_ecs_decision_memo.md)
  ECS evaluation for curation internals, migration cost, and recommendation
- [Sensory Domain](sensory/README.md)
  continuous observation and diff publication
- [Sensory Substrate](sensory/substrate.md)
  stream compilation, lowering, and promotion in `sensory`
- [World State Domain](world_state/README.md)
  canonical current belief, knowledge graph projection, and world-model ownership
- [World State Graph](world_state/graph/README.md)
  current anchor selection, lineage, provenance, and graph walk
- [Temporal Fact Graph](world_state/graph/temporal_fact_graph.md)
  canonical graph model for the graph layer and spine-driven materialization
- [Graph Implementation Plan](world_state/graph/implementation_plan.md)
  phased implementation path for contracts, reducers, indexes, and planner-facing graph reads
- [Workspace FS Graph Transition Requirements](world_state/graph/workspace_fs_transition_requirements.md)
  compatibility requirements, code touchpoints, and phased lift of `workspace_fs` into graph inputs
- [Branch Federation Substrate](world_state/graph/branch_federation_substrate.md)
  canonical branch abstraction, workspace as the first branch kind, and implementation plan for federation
- [Root Federation Runtime](world_state/graph/root_federation_runtime.md)
  runtime discovery, safe migration, and user trigger flow for pre existing workspace roots
- [World State Belief](world_state/belief/README.md)
  confidence, revision, contradiction, and settlement over current anchors
- [Curation In Belief](world_state/belief/curation.md)
  merge activity and natural runtime inside `world_state/belief`
- [Knowledge Graph ECS Decision Memo](world_state/belief/knowledge_graph_ecs_decision_memo.md)
  ECS evaluation for curation internals, migration cost, and recommendation
- [Execution Domain](execution/README.md)
  world-model-aware action aligned with current execution design
- [Execution Substrate](execution/substrate.md)
  planning and deliberate action substrate for `execution`
- [Execution Control](execution/control/README.md)
  control as the coordination layer inside `execution`
- [Execution Planning](execution/control/planning/README.md)
  HTN, planning, repair, and synthesis inside `execution`
- [Events Design](events/README.md)
  shared event architecture, replay, sequencing, and telemetry refactor path
- [Spine Concern](spine/README.md)
  shared temporal substrate and sequencing constraints
- [Further Research Prompts](further_research_prompts.md)
  research queue for unresolved questions

## Read Order

1. [Observe Merge Push](observe_merge_push.md)
2. [Knowledge Graph ECS Decision Memo](world_state/belief/knowledge_graph_ecs_decision_memo.md)
3. [Sensory Domain](sensory/README.md)
4. [Sensory Substrate](sensory/substrate.md)
5. [World State Domain](world_state/README.md)
6. [World State Graph](world_state/graph/README.md)
7. [Temporal Fact Graph](world_state/graph/temporal_fact_graph.md)
8. [Graph Implementation Plan](world_state/graph/implementation_plan.md)
9. [Workspace FS Graph Transition Requirements](world_state/graph/workspace_fs_transition_requirements.md)
10. [Branch Federation Substrate](world_state/graph/branch_federation_substrate.md)
11. [Root Federation Runtime](world_state/graph/root_federation_runtime.md)
12. [World State Belief](world_state/belief/README.md)
13. [Curation In Belief](world_state/belief/curation.md)
14. [Knowledge Graph ECS Decision Memo](world_state/belief/knowledge_graph_ecs_decision_memo.md)
15. [Execution Domain](execution/README.md)
16. [Execution Substrate](execution/substrate.md)
17. [Execution Control](execution/control/README.md)
18. [Execution Planning](execution/control/planning/README.md)
19. [Spine Concern](spine/README.md)
20. [Events Design](events/README.md)
21. [Further Research Prompts](further_research_prompts.md)

## Read With

- [Execution Control](execution/control/README.md)
- [Events Design](events/README.md)
- [Multi-Domain Spine](events/multi_domain_spine.md)
- [Bayesian Evaluation Example](execution/examples/bayesian_evaluation.md)
- [Synthesis Overview](execution/control/synthesis/README.md)
- [Goals](../goals/README.md)
