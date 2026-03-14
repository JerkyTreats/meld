# `ChatHTN`

- Upstream: <https://github.com/hhhhmmmmm02/ChatHTN>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: Python, branch `main`, commit `f795a0a`

## Role

- `ChatHTN` is a small experimental hybrid that layers LLM calls over a PyHOP-style symbolic planner.
- It is a script-oriented prototype rather than a general HDDL toolchain.

## Layout

- The planner substrate lives in `pyhop.py`.
- LLM integration lives in `openAINewVersion.py`.
- Domain definitions and domain runners are paired as top-level files such as `logisticsDefinitions.py` with `logistics.py`, and similar pairs for household robot and search and rescue.
- There is no package boundary or deep module tree. The repo is intentionally flat.

## HTN Structure

- Domain knowledge is encoded directly in Python functions for operators, axioms, method variants, and verification helpers.
- The README describes a manual import order, which shows that the system is assembled as a lightweight script stack.
- The LLM layer is positioned as a helper around symbolic methods rather than as a replacement for the planner core.
- Each domain file acts as both model and experiment harness, so the repo structure mirrors the paper workflow closely.

## Design Considerations

- This repo optimizes for fast experimentation with hybrid symbolic and approximate planning.
- Flat structure keeps the feedback loop short, but also means there is little separation among planner core, domain authoring, and experiment runner.
- The architecture depends on host-language method definitions, not on an external planning language.
- It is a strong example of how HTN plus LLM prototypes often start from small embedded planners instead of from large HDDL ecosystems.

## Cross Repo Takeaways

- `ChatHTN` is structurally closest to `IPyHOP` and the host-language side of `shop3`, not to parser-heavy HDDL repos.
- It shows a recurring pattern for hybrid work: keep the symbolic planner tiny and explicit, then add model calls around missing or ambiguous decomposition steps.
- It is useful as a contrast case when comparing infrastructure-heavy HTN stacks with prototype-first research code.
