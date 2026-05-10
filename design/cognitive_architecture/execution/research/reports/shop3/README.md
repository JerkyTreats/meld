# `shop3`

- Upstream: <https://github.com/shop-planner/shop3>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: Common Lisp, branch `master`, commit `c58c86d`

## Role

- `shop3` is a full HTN planner framework with a library core, command line entry paths, domain examples, and a large test surface.
- It represents the host-language style of HTN implementation, where methods, operators, state logic, and planner extensions live directly in Lisp modules rather than in a separate parser plus engine split.

## Layout

- Core system definition lives in `shop3/shop3.asd`.
- Core planner packages live under `shop3/`, especially `planning-engine/`, `explicit-stack-search/`, `planning-tree/`, `theorem-prover/`, `unification/`, and `common/`.
- IO and language interop live under `shop3/io/`, `shop3/pddl/`, and `shop3/hddl/`.
- Executable packaging lives under `shop3/buildapp/`.
- Examples and regression coverage live under `shop3/examples/` and `shop3/tests/`.

## HTN Structure

- The codebase is organized as a planner platform first, not just a single binary.
- The ASDF system splits the code into reusable subsystems, which lets the planner core, theorem prover, unifier, plan tree tooling, and IO stack evolve as separate modules.
- There are two notable execution styles in the tree:
  - a classic planning engine under `shop3/planning-engine/`
  - an explicit stack search path under `shop3/explicit-stack-search/`
- Plan representation is first-class. `shop3/planning-tree/`, `shop3/plan-printer.lisp`, and `shop3/hddl/hddl-plan.lisp` show that output trees and plan export are treated as stable internal products, not incidental debug data.

## Design Considerations

- `shop3` keeps parsing, theorem proving, search, and plan rendering in one Lisp system, which favors deep internal integration over process boundaries.
- PDDL and HDDL support are adapters around the planner core rather than separate front ends that hand work to another executable.
- Tests and example domains sit close to the planner source, which makes the repo look like a long-lived language ecosystem rather than a one-off research artifact.
- The planner is extensible through packages and subsystems, so structure is driven by features such as plan trees, replanning, theorem proving, and export, not only by one canonical search loop.

## Cross Repo Takeaways

- `shop3` is the clearest example of a native HTN planner framework in this set.
- Compared with HDDL-centric repos, it pushes HTN structure into host-language modules rather than into an external domain compiler pipeline.
- Compared with smaller Python planners, it carries much more surrounding infrastructure for IO, packaging, and planner introspection.
