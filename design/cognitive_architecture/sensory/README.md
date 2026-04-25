# Sensory Domain

Date: 2026-04-22
Status: active
Scope: continuous observation, modality isolation, and diff publication into shared events

## Thesis

`sensory` converts raw environmental change into typed observations.
It is the observe side of the loop.

The domain model is:

- one worker family per modality
- diff publication rather than full state replay on every cycle
- provenance and source identity attached at emission time
- independence from curation and execution correctness

## Boundary

`sensory` owns:

- modality-specific observation workers
- normalization from raw signal into typed observation artifacts
- publication of observation facts into events
- local backpressure policy close to the source

The world model owns belief updates and conflict resolution.
`events` owns durability, ordering, replay, and subscription.
`control` owns task-triggered observation use cases that already exist today.

## Current Anchors

- `workspace_scan_batch` already shows the right diff-first publication shape
- [Await Observation Semantics](../execution/control/program/await_observation_semantics.md) already defines the deliberate observation-and-branch pattern inside `execution`
- the current system is stronger at task-triggered observation than continuous background sensing

## Substrate

- [Sensory Substrate](substrate.md)
  parallel stream compilers, lowering IR, and promotion into shared events

## Required First Slice

- always-on workers for workspace, git, and other high-value modalities
- typed observation contracts that can survive replay and cross-domain reuse
- source-local throttling so high-volume sensors do not dominate events
- a clean handoff from sensory publication to curation reducers

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Sensory Substrate](substrate.md)
- [World Model Domain](../world_state/README.md)
- [Events Domain](../events/README.md)
- [Await Observation Semantics](../execution/control/program/await_observation_semantics.md)
