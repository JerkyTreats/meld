# Sensory Substrate

Date: 2026-04-12
Status: active
Scope: natural runtime model for observation, lowering, and promotion in `sensory`

## Thesis

To each domain be true.

The natural substrate for `sensory` is not `task` and `capability`.
It is a family of parallel stream compilers that lower high-frequency raw signal into slower typed observations suitable for shared replay and curation.

The key idea is:

- raw signal stays close to the source
- lowering resolves jitter, burst, and continuity
- promoted observations become semantic facts suitable for the shared spine

`sensory` compiles.
It does not decide belief.

## Natural Runtime Shape

The substrate should look like:

- modality workers
- local buffers and backpressure
- lowering stages
- promotion gates
- typed observation publishers

This is closer to stream processing and online estimation than to deliberate workflow execution.

## Core Primitives

- raw lane
  fast, lossy-allowed, source-local signal flow
- lowering IR
  typed intermediate observations with observed time, integration window, confidence, and provenance
- promotion rule
  threshold or state-machine rule that decides when an intermediate observation becomes a shared semantic fact
- modality worker
  source-specific observer such as workspace, git, runtime, or future media sensors
- track or continuity state
  transient state used to preserve identity across noisy observations before promotion

## Materialization Posture

`sensory` may materialize stable lowered observations for short-lived local use.
It should not treat every raw pulse as canonical shared truth.

The durable shared materialization boundary is promotion into the spine.
Before that point, most sensory state should be:

- transient
- bounded
- replayable only if explicitly retained
- free to use loss-tolerant techniques

## What The Substrate Must Support

- multi-rate flows
- modality isolation
- duplicate suppression
- hysteresis and threshold crossing
- late arrival handling close to the source
- source-local throttling
- explicit provenance at promotion time

## What Should Not Be Forced Into This Substrate

- world belief maintenance
- planner-facing truth
- task repair logic
- provider execution
- human-facing context views

Those belong to other domains.

## Relationship To Spine

`spine` is not the sensory substrate.
It is the shared temporal ledger that receives promoted sensory facts.

That means:

- raw lanes stay outside the spine
- lowering IR usually stays outside the spine
- promoted semantic observations enter the spine

## First Slice

- workspace watch lowering from churn into stable workspace delta observations
- git change lowering into reusable typed observations
- one promotion contract shared by those first modalities
- one replay story for promoted facts only

## Read With

- [Sensory Domain](README.md)
- [Spine Concern](../spine/README.md)
- [Observe Merge Push](../observe_merge_push.md)
- [Further Research Prompts](../further_research_prompts.md)
