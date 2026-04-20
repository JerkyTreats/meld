# Belief

Date: 2026-04-20
Status: active
Scope: confidence, revision, contradiction, and settlement inside `world_state`

## Thesis

`belief` answers whether the current anchor should still be trusted.

Traversal can tell the system what is current.
Belief tells the system whether current is still credible.

## What Belief Owns

- confidence and uncertainty
- contradiction handling
- supersession as belief revision
- calibration from later outcomes
- perspective quality over time

## What Belief Consumes

Belief consumes the outputs of traversal plus new evidence from execution, workspace, and later sensory domains.

That means belief should build on:

- current anchors
- lineage chains
- provenance bundles
- later evidence that confirms or weakens those anchors

## Current Status

The repo has a real traversal substrate today.
It also has a legacy claim projection for generation outcomes and artifact availability.

It does not yet have a fully explicit belief layer with confidence, contradiction, calibration, and curation semantics.

This area is where curation, settlement, and possible ECS-shaped internals belong after traversal is stable.

## Read With

- [World State Domain](../README.md)
- [Graph](../graph/README.md)
- [Curation In Belief](curation.md)
- [Knowledge Graph ECS Decision Memo](knowledge_graph_ecs_decision_memo.md)
- [Observe Merge Push](../../observe_merge_push.md)
