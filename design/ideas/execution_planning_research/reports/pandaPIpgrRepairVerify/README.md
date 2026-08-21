# `pandaPIpgrRepairVerify`

- Upstream: <https://github.com/panda-planner-dev/pandaPIpgrRepairVerify>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: C plus plus, branch `master`, commit `59df3fa`

## Role

- `pandaPIpgrRepairVerify` is a transformation and verification utility around grounded HTN models.
- Its tasks are plan and goal recognition, repair scaffolding, and plan verification rather than core search for a fresh plan from scratch.

## Layout

- Entry dispatch lives in `main.cpp`.
- Shared HTN model types live in `htnModel/`.
- Prefix and recognition encodings live in `prefEncoding/`.
- Verification support lives in `verifier/`.
- Example assets and benchmark data live under `example-pgr/`, `example-verify/`, and `benchmarks/`.

## HTN Structure

- The executable selects a task mode such as `pgrfo`, `pgrpo`, `verify`, or `cyk` and then constructs an encoding from a grounded model and a trace.
- This means the repo sits after parsing and grounding in the pipeline.
- HTN structure is reused here as an analyzable graph or prefix encoding substrate rather than as a direct search space.
- The split between `htnModel/` and `prefEncoding/` shows a clean line between shared domain representation and task-specific compilation logic.

## Design Considerations

- Verification and recognition are modeled as separate applications over the same grounded HTN data structures.
- The repo uses a focused executable with mode dispatch rather than separate binaries for each research task.
- Benchmarks and examples are stored with the code, which reflects an evaluation-driven structure common in planning research repos.
- The design favors reuse of model readers and encoders across several post-planning tasks.

## Cross Repo Takeaways

- This repo shows that HTN ecosystems often have a second layer of tools after solving, especially for verification and recognition.
- Its use of shared `htnModel` concepts is close to `ToadPlanningSystem`, which points to a recurring grounded-model core reused by multiple utilities.
- It complements `pandaPIengine` by covering analysis tasks that sit beside, not inside, the main planner.
