# `pandaPIengine`

- Upstream: <https://github.com/panda-planner-dev/pandaPIengine>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: C plus plus, branch `master`, commit `810f043`

## Role

- `pandaPIengine` is the solver core in the PANDA family.
- It focuses on search, progression, heuristics, and optional backend variants after parsing and grounding are already done.

## Layout

- Top level source is under `src/`.
- Core problem and search state structures live in `src/Model.*` and `src/ProgressionNetwork.*`.
- Search orchestration lives in `src/SearchEngine.cpp`.
- Planner interaction helpers live in `src/interactivePlanner.*`, `src/Invariants.*`, `src/VisitedList.*`, and `src/Util.*`.
- Build composition is declared in `src/CMakeLists.txt`, which wires in sublibraries such as `search`, `symbolic_search`, `heuristics`, `translation`, and `intDataStructures`.

## HTN Structure

- The repo is structured as an engine layer with a narrow executable front door.
- `SearchEngine.cpp` exposes multiple algorithm families such as progression, SAT, BDD, interactive mode, and translation mode, which shows that the same grounded HTN model can feed several solving backends.
- The model and progression network types are shared infrastructure for those backends.
- Heuristics are not embedded into one monolithic search file. They are surfaced as pluggable subsystems with compile-time feature flags and runtime selection.

## Design Considerations

- This repo assumes a multi-stage toolchain. Parsing and grounding are intentionally outside the engine boundary.
- Optional dependencies such as CPLEX and CUDD are compile-time switches, so advanced heuristic and symbolic modes are treated as backend capabilities rather than baseline requirements.
- The layout shows a strong separation between domain model, search policy, heuristic families, and low-level data structures.
- This is a good example of HTN infrastructure where solver variation happens inside one engine binary, while format translation happens elsewhere in the toolchain.

## Cross Repo Takeaways

- `pandaPIengine` pairs naturally with `pandaPIparser` and benchmark corpora like `ipc2020-domains`.
- It exemplifies the parser-to-grounder-to-engine split that appears several times in this set.
- It also shows a recurring HTN pattern: stable shared model structs plus interchangeable search backends.
