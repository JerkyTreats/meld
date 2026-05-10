# `ToadPlanningSystem`

- Upstream: <https://github.com/toad-planner-dev/ToadPlanningSystem>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: C plus plus, branch `master`, commit `25aaad5`

## Role

- `ToadPlanningSystem` is a translation-oriented HTN system for total-order problems.
- It turns grounded HTN models into automata-based artifacts and then writes classical planning output such as SAS or PDDL.

## Layout

- Entry orchestration lives in `main.cpp`.
- Shared HTN model types live in `htnModel/`.
- Translation and automata logic live in `translation/`.
- Output writers live in `ModelWriter.*` and `SASWriter.*`.
- Lower-level helpers live in `utils/`.
- Older experiments and retired code live in `deprecated/`.

## HTN Structure

- The repo is organized as a compilation pipeline.
- The main program reads a grounded HTN problem, converts methods into grammar rules, performs rule analysis, builds a DFA with top-down or bottom-up strategies, then writes a classical planning artifact.
- Translation classes such as `CFGtoFDAtranslator`, `CFtoRegGrammarEnc`, `StateBasedReachability`, and `HeuFaDist` reveal that grammar and automata construction are the center of the architecture.
- The planner does not bundle a classical solver. It prepares artifacts for external tooling such as Fast Downward style workflows.

## Design Considerations

- This repo assumes a grounded input generated elsewhere, which keeps parsing outside the system boundary.
- OpenFST is treated as a core dependency, so automata operations are not an incidental add-on.
- The translation pipeline is explicit in the top-level layout, which makes the code easier to read as a sequence of compilation stages rather than as one generic planner loop.
- Output generation is a first-class concern because the real execution target is another planning stack.

## Cross Repo Takeaways

- `ToadPlanningSystem` is the clearest translation-first codebase in the set.
- It shares grounded-model assumptions with parts of the PANDA ecosystem, but its architecture is built around automata compilation rather than around native HTN search.
- This repo highlights one major HTN implementation family: use hierarchy to compile into a non-HTN substrate, then solve there.
