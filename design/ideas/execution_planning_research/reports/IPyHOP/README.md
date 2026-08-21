# `IPyHOP`

- Upstream: <https://github.com/YashBansod/IPyHOP>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: Python, branch `main`, commit `bdda210`

## Role

- `IPyHOP` is a lightweight Python HTN and GTN planning library.
- It focuses on an in-process planner object, reusable domain registries, and decomposition-tree-aware replanning.

## Layout

- The library code lives under `ipyhop/`.
- Planner logic lives in `ipyhop/planner.py`.
- Domain registration helpers live in `ipyhop/actions.py` and `ipyhop/methods.py`.
- State and multi-goal representations live in `ipyhop/state.py` and `ipyhop/mulitgoal.py`.
- Execution, failure handling, and visualization helpers live in `ipyhop/mc_executor.py`, `ipyhop/failure_handler.py`, and `ipyhop/plotter.py`.
- Examples and tests live outside the library core in `examples/` and `ipyhop_tests/`.

## HTN Structure

- The central planner class owns the current state, task list, solution plan, solution tree, and a blacklist.
- The solution tree is stored as a `networkx.DiGraph`, which makes partial decompositions a first-class runtime object.
- Methods and actions are registered as Python callables, so the planner does not rely on an external domain language.
- Replanning and failure recovery operate over the decomposition structure instead of restarting from scratch with only a flat plan.

## Design Considerations

- `IPyHOP` is intentionally compact. Structure is built around a small reusable library API rather than a multi-tool pipeline.
- The host-language approach keeps domain authoring inside Python, which lowers tooling overhead and makes experiments easy to script.
- The use of graph objects for the decomposition tree makes planner state inspectable and reusable for execution and repair helpers.
- The repo boundary is narrow and clear: planner package in one directory, tests and demos around it.

## Cross Repo Takeaways

- `IPyHOP` is the cleanest small-library counterpart to larger systems like `shop3`.
- It shows the host-language HTN pattern in a compact modern form.
- Compared with HDDL pipelines, it trades language portability for a very direct representation of methods, actions, and replanning state.
