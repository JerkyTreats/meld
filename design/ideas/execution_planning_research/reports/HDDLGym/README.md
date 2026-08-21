# `HDDLGym`

- Upstream: <https://github.com/HDDLGym/HDDLGym>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: Python, branch `main`, commit `f19db8f`

## Role

- `HDDLGym` is an application and experimentation stack that wraps HDDL domains as Gym-style environments for multi-agent planning and learning.
- It combines parsing, environment state management, planner utilities, learning code, visual tooling, and domain assets in one repo.

## Layout

- Core environment and planner helpers live in `src/`.
- GUI and visual editing tools live in `HDDL_GUI/`.
- RL environment support and bundled dependencies live in `jaxmarl/` and `overcooked/`.
- Domain assets and many problem files are stored inside the repo alongside the code.
- Packaging is light. The repo reads like a research workspace more than a small standalone library.

## HTN Structure

- `src/hddl_env.py` builds an `HDDLEnv` around a parsed domain and problem pair.
- `src/hddl_parser.py` turns HDDL text into an environment dictionary that becomes the shared runtime model.
- `src/central_planner.py` and `src/central_planner_utils.py` implement method choice, valid action generation, combination checking, and action application.
- `src/learning_methods.py` and `src/learning_methods_lifted.py` layer PPO-style policy learning over that HTN-aware action space.
- The GUI stack adds graph views, method editing, plan rendering, and helper tooling for interactive model work.

## Design Considerations

- HTN structure is embedded into an environment runtime model rather than isolated behind a separate planner binary.
- The repo blurs the line between planning and learning. The same parsed hierarchy feeds both symbolic planner helpers and neural policies.
- Multi-agent coordination is central to the structure, which makes agent state, belief, and valid joint action generation core architectural concerns.
- The codebase carries both runtime logic and many assets, so data layout is part of the architecture, not just supporting material.

## Cross Repo Takeaways

- `HDDLGym` represents the environment-centric HTN family.
- Unlike `pandaPIengine` or `ToadPlanningSystem`, it does not separate parsing, solving, and application into strict process boundaries.
- It shows how HDDL can become the substrate for simulation and policy learning rather than only for traditional plan generation.
