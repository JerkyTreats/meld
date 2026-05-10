# Cognitive Architecture

Date: 2026-04-22
Status: active
Scope: full loop design for sensory, world model, execution, and shared temporal coordination

## Thesis

This area defines the system as an open-world loop rather than a task-only executor.

```mermaid
flowchart LR
    O[observe] --> S[sensory]
    S --> P[event ledger]
    P --> W[world model]
    W --> K[knowledge graph]
    K --> E[execution]
    E --> P
```

The durable positions are:

- the full world model is never fully known
- observation is continuous and diff-native
- the world model integrates observations into temporal graph and belief views
- execution acts against the current world model and republishes outcomes
- events are the shared temporal substrate across all concerns

## Boundary

`cognitive_architecture` is not a replacement for `goals`, `control`, `task`, `capability`, or `provider`.

This area does own:

- the cross-domain loop definition
- the sensory and world-model seams that do not yet have durable homes
- the world-model requirement that action be grounded in current belief
- the event requirements needed for genuine multi-process coordination

## Crate Routing

`CRATE.md` files map design ownership to the multi-crate code direction.

- [Core Crate](core/CRATE.md)
  root `meld` orchestration, CLI, config, context and provider adapters, compatibility, and runtime wiring
- [Events Crate](events/CRATE.md)
  `meld-events` event ledger, append, replay, sequence, and reference contracts
- [World Model Crate](world_model/CRATE.md)
  `meld-world-model` graph, anchors, provenance, belief, and planner-facing views
- [Execution Crate](execution/CRATE.md)
  `meld-execution` planning, control, task, capability, workflow, and provider execution

## Durable Structure

- [Crate Split Implementation Plan](PLAN.md)
  phased implementation order, gates, and dependency closure for the crate split
- [Observe Merge Push](observe_merge_push.md)
  founding prompt and response index
- [Microarchitecture Assessment By Domain](microarchitecture_assessment_by_domain.md)
  domain impact review for separating events, world model, and execution responsibilities
- [Knowledge Graph ECS Decision Memo](world_model/belief/knowledge_graph_ecs_decision_memo.md)
  ECS evaluation for curation internals, migration cost, and recommendation
- [Sensory Domain](sensory/README.md)
  continuous observation and diff publication
- [Sensory Substrate](sensory/substrate.md)
  stream compilation, lowering, and promotion in `sensory`
- [World Model Domain](world_model/README.md)
  five-layer world model ownership across graph, belief, causality, regimes, and planner-facing reads on top of upstream events
- [World Model Graph](world_model/graph/README.md)
  current anchors, lineage, provenance, traversal, branch-scoped reads, and graph surface consumed by upper world model layers
- [World Model Belief](world_model/belief/README.md)
  confidence, revision, contradiction, and settlement over current anchors
- [Causal Layer](world_model/causation/README.md)
  mechanism, intervention, confounding, and counterfactual semantics above belief
- [Regime Layer](world_model/regime/README.md)
  changepoints, recurring modes, mixture prediction, and structural stress
- [World Model Planner](world_model/planner/README.md)
  planner-facing world model projection with a strict boundary to execution authority
- [Belief Microarchitecture](world_model/belief/microarchitecture.md)
  event, world model, and execution boundaries for belief
- [Fact To Belief](world_model/belief/fact_to_belief.md)
  transition from event facts and graph anchors into evidence, belief revision, and planner view
- [Comparator Model](world_model/belief/comparator_model.md)
  Bayesian comparators, rule comparators, semantic settlement, and missing comparator policy
- [Belief Substrate](world_model/belief/substrate.md)
  event-driven curation runtime, leases, recovery, staleness, and storm handling
- [Curation In Belief](world_model/belief/curation.md)
  merge activity and natural runtime inside `world_model/belief`
- [Knowledge Graph ECS Decision Memo](world_model/belief/knowledge_graph_ecs_decision_memo.md)
  ECS evaluation for curation internals, migration cost, and recommendation
- [Execution Domain](execution/README.md)
  world-model-aware action aligned with current execution design
- [Execution Planning](execution/planning/README.md)
  HTN, planning, repair, and synthesis inside `execution`
- [Events Design](events/README.md)
  shared event architecture, replay, sequencing, and telemetry refactor path
- [Further Research Prompts](further_research_prompts.md)
  research queue for unresolved questions

## Read Order

1. [Observe Merge Push](observe_merge_push.md)
2. [Microarchitecture Assessment By Domain](microarchitecture_assessment_by_domain.md)
3. [Crate Split Implementation Plan](PLAN.md)
4. [Knowledge Graph ECS Decision Memo](world_model/belief/knowledge_graph_ecs_decision_memo.md)
5. [Sensory Domain](sensory/README.md)
6. [Sensory Substrate](sensory/substrate.md)
7. [World Model Domain](world_model/README.md)
8. [World Model Graph](world_model/graph/README.md)
9. [World Model Belief](world_model/belief/README.md)
10. [Causal Layer](world_model/causation/README.md)
11. [Regime Layer](world_model/regime/README.md)
12. [World Model Planner](world_model/planner/README.md)
13. [Belief Microarchitecture](world_model/belief/microarchitecture.md)
14. [Fact To Belief](world_model/belief/fact_to_belief.md)
15. [Comparator Model](world_model/belief/comparator_model.md)
16. [Belief Substrate](world_model/belief/substrate.md)
17. [Curation In Belief](world_model/belief/curation.md)
18. [Knowledge Graph ECS Decision Memo](world_model/belief/knowledge_graph_ecs_decision_memo.md)
19. [Execution Domain](execution/README.md)
20. [Execution Planning](execution/planning/README.md)
23. [Events Design](events/README.md)
24. [Further Research Prompts](further_research_prompts.md)

## Read With

- [Execution Planning](execution/planning/README.md)
- [Events Design](events/README.md)
- [Multi-Domain Event Ledger](events/multi_domain_spine.md)
- [Bayesian Evaluation Example](execution/examples/bayesian_evaluation.md)
- [Synthesis Overview](execution/synthesis/README.md)
- [Goals](execution/goals/README.md)
