# `ipc2020-domains`

- Upstream: <https://github.com/panda-planner-dev/ipc2020-domains>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: benchmark corpus, branch `master`, commit `9e31324`

## Role

- `ipc2020-domains` is a benchmark repository, not a planner implementation.
- It matters structurally because it defines the domain and problem organization that many HTN tools target.

## Layout

- Total-order benchmarks live in `total-order/`.
- Partial-order benchmarks live in `partial-order/`.
- Each domain has its own folder with domain files, problem files, and in some cases generator scripts or supporting assets.
- Validation and feature checks live in `tests/`.
- The root README is the catalog and track index.

## HTN Structure

- The main architectural choice is data layout rather than executable code.
- Separating total-order and partial-order suites at the top level makes planner capability boundaries explicit.
- Per-domain folders work as mini packages that hold both the canonical HDDL artifacts and any custom generation logic needed to reproduce instances.
- The repo standardizes file placement and naming enough to act as a de facto interoperability contract across planners, parsers, and validators.

## Design Considerations

- Benchmark layout becomes part of the HTN ecosystem architecture because many tools hard-code assumptions about domain and problem pairing.
- Keeping generators close to the domain definitions preserves provenance for instances and supports reproducible evaluation.
- The split by ordering discipline reflects a core modeling distinction that often propagates into solver design and parser support.
- This repo is a reminder that HTN structure across codebases is shaped by shared corpora as much as by algorithm design.

## Cross Repo Takeaways

- `ipc2020-domains` is the common data substrate for several repos in this set.
- It explains why many implementations isolate front ends, validators, and grounded-model pipelines around HDDL assets.
- Even without much executable code, it is one of the strongest structural influences on the rest of the ecosystem.
