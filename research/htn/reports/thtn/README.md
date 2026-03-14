# `thtn`

- Upstream: <https://github.com/virajparimi/thtn>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: C plus plus, branch `master`, commit `84cbcb8`

## Role

- `thtn` is a timeline-based multi-robot hierarchical planner.
- It combines HDDL-style parsing with temporal reasoning, scheduling, search trees, and experiment assets.

## Layout

- The code is split into `parser/` and `planner/`.
- The parser side contains grammar files, parse-tree types, domain data structures, output helpers, and static property checks.
- The planner side contains graph structures, search structures, STN logic, timeline objects, planner orchestration, and utilities.
- Supporting experiment material lives in `data/`, `experiments/`, and `outputs/`.

## HTN Structure

- The parser is built with Flex and Bison and produces domain and task-network structures that closely resemble compiler-style HTN front ends.
- The planner adds a second layer for temporal feasibility. Files such as `planner/include/stn.hpp` and `planner/include/timelines.hpp` show that decomposition is only part of the search state.
- Search operates over task-network solutions and slot assignments, with metrics such as makespan or action count selected at runtime.
- The architecture couples hierarchy with scheduling rather than treating scheduling as a post-process.

## Design Considerations

- `thtn` keeps parsing and planning in distinct subtrees, which makes the front end reusable and keeps temporal logic out of the parser.
- The parser file set strongly resembles `pandaPIparser`, which suggests a stable structural pattern for C plus plus HDDL front ends.
- Temporal reasoning introduces additional graph and constraint layers that are absent in simpler HTN planners.
- Experiments and outputs are first-class repo content, which is typical for research planners that are evaluated across benchmark suites.

## Cross Repo Takeaways

- `thtn` is the clearest example here of HTN plus scheduling as a single architecture.
- It sits between parser-first systems and full application stacks: more structured than script prototypes, less service-oriented than `sh`.
- It shows how temporal HTN implementations often add a dedicated planner layer on top of a fairly standard HDDL front end.
