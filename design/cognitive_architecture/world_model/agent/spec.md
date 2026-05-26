# Agent Spec

Date: 2026-05-02
Status: active
Scope: perspective-heavy assembly point inside `world_model`

The Agent is the perspective where the other world model domains become action-relevant for one consumer. While `graph`, `belief`, `causation`, `regime`, and `planner` are process-heavy concerns that reduce facts, settle posteriors, estimate effects, detect structural shifts, and project planner-facing reads, `agent` is data-heavy. Its main job is to bind one perspective to those other domain outputs and produce one coherent world-model view for that perspective.

The Agent does not replace those domains. It consumes them.

## Position

The payoff for Agent is not one more inference loop. It is sparse, divergent perspective state over a shared substrate.

Many Agents should be able to:

- share one event and graph foundation
- consume the same belief family with different trust policy
- consume the same causal summaries with different relevance
- consume the same regime signals with different sensitivity
- receive distinct planner-facing projections without duplicating the lower layers

This is where `En Masse and At Will` becomes concrete.

## Domain Types

The core Agent domain types are:

- `Agent`
  one stable consumer perspective over the world model
- `AgentPerspective`
  the scoped interpretation context for one Agent
- `AgentBeliefLens`
  the Agent-scoped selection of belief families and filters
- `AgentCausalLens`
  the Agent-scoped selection of causal summaries and intervention questions
- `AgentRegimeLens`
  the Agent-scoped sensitivity to structural uncertainty and break risk
- `AgentPlannerLens`
  the assembled planner-facing projection for one Agent

These are lens types over shared lower-domain state, not duplicate world facts.

## Data Model

The Agent data model consists of:

- `AgentIdentity`
  stable identity and ownership metadata
- `PerspectiveKey`
  the primary key for one perspective over the world model
- `BeliefPolicy`
  trusted evidence channels, tolerated uncertainty, and relevance thresholds
- `ObservationScope`
  what evidence channels and object spaces this Agent is expected to observe
- `BranchScope`
  which branch-local world slices this Agent is allowed to consume
- `BeliefRefs`
  references to relevant belief views
- `CausalRefs`
  references to relevant causal summaries
- `RegimeRefs`
  references to relevant regime summaries
- `PlannerViewRef`
  reference to the current planner-facing world-model projection
- `ExecutionHandoffRef`
  reference to the shaped handoff consumed by `execution`
- `CalibrationProfile`
  history of outcomes that changes how this Agent interprets evidence

These should mostly be references, filters, thresholds, and policy state. They should not duplicate the heavy internal state owned by the other domains.

## Pipelines

The core Agent pipeline stages are:

- perspective resolution
  determine which shared world-model outputs are relevant to one Agent
- belief lens assembly
  filter and rank belief outputs for that perspective
- causal lens assembly
  select causal summaries and open effect questions relevant to that perspective
- regime lens assembly
  project structural uncertainty into Agent-specific sensitivity
- planner lens assembly
  assemble the final planner-facing world-model view for that Agent
- execution handoff publication
  publish one shaped view for downstream `execution`

These are assembly stages. They should consume outputs from the more process-heavy domains rather than re-running their logic.

## What Agent Consumes

Agent should consume:

- graph identity and hydration handles from `graph`
- posterior state and observation opportunities from `belief`
- effect summaries and confounder warnings from `causation`
- regime sensitivity and break risk from `regime`
- assembled action-relevant summaries from `planner`

Agent should not consume raw reducer internals when shaped outputs exist.

## What Agent Owns

Agent owns:

- perspective
- trust and evidence policy
- tolerance for uncertainty
- branch and observation scope
- relevance over shared world-model outputs
- one shaped handoff to `execution`

Agent does not own:

- graph truth
- belief settlement
- causal estimation
- changepoint inference
- runtime task dispatch

## Core Design Rule

Agent is the owner of perspective, not the owner of truth.

That means:

- lower domains do heavy world-model work once
- Agent instances consume and lens that work many times
- divergence across Agents should mostly live in data and projections, not in duplicate pipelines

## First Slice

The first Agent slice should prove:

- one Agent over shared graph and belief substrate
- one perspective key
- one belief lens
- one planner lens
- one shaped execution handoff

It does not need shared planning, inter-Agent negotiation, or execution-owned policy semantics.

## Read With

- [World Model Agent](README.md)
- [World Model Belief](../belief/README.md)
- [Belief Spec](../belief/spec.md)
- [Graph Spec](../graph/spec.md)
- [Causal Spec](../causation/spec.md)
- [Regime Spec](../regime/spec.md)
- [Planner Spec](../planner/spec.md)
