# Multiplier Harness Program

Date: 2026-07-06
Status: draft
Scope: experiment infrastructure measuring accumulated belief as an intelligence multiplier across model tiers and horizon lengths

## Purpose

Measure the project's core bet as a curve family rather than a binary. The hypothesis, stated precisely: quality of a small actor model operating on the flywheel with belief injection approaches quality of a large actor model operating stateless, as flywheel cycles accumulate — and the quality-versus-horizon slope of the flywheel configuration exceeds the stateless configuration, which plateaus or degrades.

Secondary claims the same data tests: cost per accepted artifact falls with accumulated belief, and cross-artifact contradiction count falls with belief injection.

## Experiment Matrix

Three independent axes. Every cell is a full run.

- Actor model tier: small local via the existing Ollama provider, mid hosted, large hosted.
- Substrate mode: stateless baseline re-prompted from raw source each cycle, flywheel with `belief_context` on.
- Horizon checkpoints: cycles 1, 10, and 50.

## Workload

The `docs_freshness` scenario over a fixture repository driven by a scripted mutation series: one deterministic, versioned sequence of source changes applied per cycle. The mutation script is part of the harness fixture so every cell experiences the identical evolving world. Horizon means accumulated cycles over this series, not wall time.

## Metrics

- Judged quality score per artifact, rubric-based.
- Factual consistency against the fixture workspace ground truth at the cycle the artifact was produced.
- Cross-artifact contradiction count across the run's accumulated artifacts.
- Cost: tokens and provider spend per accepted artifact, per cycle.

## Judge Protocol

- The judge model is distinct from and stronger than every actor model in the matrix. Judge and actor are never the same model.
- The rubric is a versioned harness artifact; judgments cite rubric version.
- Each judgment repeats three times with presentation order shuffled; the harness records per-item agreement and flags low-agreement items for human audit.
- A fixed sample of judged items per run receives human spot review; disagreement between human and judge is recorded as harness evidence, not silently corrected.

## Spine Schema

Experiment runs are recorded on the event spine so results are replayable projections, following the promoted-facts inclusion rule.

- Domain: `experiment`.
- Kinds: `experiment.run.registered` carrying matrix cell and fixture versions, `experiment.cycle.completed` carrying artifact refs and cost, `experiment.judgment.recorded` carrying scores, rubric version, and agreement.
- Record ids derive from run id, cycle, and item, so re-ingestion is idempotent.
- The multiplier curves are a deterministic projection over these facts. No result lives only in a notebook.

## Exit Criteria

- One complete matrix executed end to end.
- The curve family — quality by cycles, one curve per matrix cell — plotted from spine data alone.
- A written finding stating whether the small-plus-flywheel curve converges toward, parallels, or diverges from the large-stateless curve, with cost curves alongside.

## Dependencies

- Runtime activation: the flywheel must turn unattended over the fixture workspace. This is delivered by the flywheel-ignition lane and certified by the bounded convergence proof, both executing at phase five of the [Runtime Harness Plan](runtime_harness_plan.md).
- [Generation Read Path First Slice](generation_read_path_first_slice.md): the flywheel mode is meaningless without belief injection.
- Ollama provider validation against one local small model.

Naming note: this experiment harness is distinct from the runtime harness above — the runtime harness is the debugger substrate over live runtimes; this program is offline experiment infrastructure. The two share nothing but the word. Experiment runs recorded on the event spine are natural consumers of the runtime harness's served read substrate for replay and inspection.

## Out Of Scope

- Preference evidence primitives and taste-shaped belief families.
- Salience ranking beyond the first slice fixed policy.
- Workloads beyond `docs_freshness`; the second domain proof is a separate program.

## Evidence Log

None yet.
