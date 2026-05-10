# HTN Codebase Structure Report

## Scope

- This workspace inspects two local PDF sources:
  - [`Advancements in Hierarchical Task Network Planning Research Since 2020.pdf`](Advancements%20in%20Hierarchical%20Task%20Network%20Planning%20Research%20Since%202020.pdf)
  - [`Hierarchical Task Networks and Goal Oriented Action Planning for Modern Agentic Systems.pdf`](Hierarchical%20Task%20Networks%20and%20Goal%20Oriented%20Action%20Planning%20for%20Modern%20Agentic%20Systems.pdf)
- Open source repos referenced in the HTN tooling section of the research survey are summarized under [`reports/`](reports).
- This report focuses on structure, boundaries, design choices, and implementation shape.
- This report does not score code quality.

## Related Design Work

- [`Workflow Orchestrator Roadmap`](../../design/workflow_orchestrator/README.md) — current Meld workflow design that now uses this research set as its HTN alignment reference

## Repo Reports

- [`shop3`](reports/shop3/README.md) — native Common Lisp planner framework with multiple planner subsystems
- [`pandaPIengine`](reports/pandaPIengine/README.md) — grounded HTN engine with several backend styles
- [`pandaPIparser`](reports/pandaPIparser/README.md) — parser and compiler front end for hierarchical models
- [`pandaPIpgrRepairVerify`](reports/pandaPIpgrRepairVerify/README.md) — verification and recognition utility over grounded HTN models
- [`ToadPlanningSystem`](reports/ToadPlanningSystem/README.md) — translation-first system that compiles HTN into automata and classical artifacts
- [`HDDLGym`](reports/HDDLGym/README.md) — environment and learning stack built around HDDL domains
- [`HDDL-Parser`](reports/HDDL-Parser/README.md) — parser, semantic checker, metadata extractor, and language server
- [`IPyHOP`](reports/IPyHOP/README.md) — compact host-language HTN and GTN library with decomposition-tree replanning
- [`thtn`](reports/thtn/README.md) — temporal and multi-robot HTN planner with parser plus scheduler split
- [`ChatHTN`](reports/ChatHTN/README.md) — script-first hybrid of symbolic HTN and LLM prompting
- [`sh`](reports/sh/README.md) — Scala planner platform with API and service layers
- [`ipc2020-domains`](reports/ipc2020-domains/README.md) — benchmark corpus that defines shared HDDL layout conventions

## Consensus Patterns

- Parser and engine are often separate products. `pandaPIparser`, `HDDL-Parser`, and the parser half of `thtn` treat syntax, normalization, and semantic checks as their own layer.
- Shared HTN model structs are common in C plus plus toolchains. `pandaPIengine`, `pandaPIpgrRepairVerify`, and `ToadPlanningSystem` all revolve around stable model and task-network data structures that feed several tasks.
- Translation-first architectures are a major family. `ToadPlanningSystem` and parts of the PANDA toolchain compile HTN into other substrates instead of solving only in native HTN space.
- Host-language planners form a second family. `shop3`, `IPyHOP`, and `ChatHTN` encode methods and actions directly in the implementation language and keep the planner embedded in the runtime.
- Service and application wrappers are common when HTN leaves the lab. `sh` exposes planning behind library and HTTP entry points, while `HDDLGym` wraps hierarchical models as environments for simulation and learning.
- Benchmarks shape implementation structure. `ipc2020-domains` reinforces the separation of total-order and partial-order support, plus conventions for domain and problem pairing.

## Structural Families

- Native planner frameworks
  - `shop3`
  - `IPyHOP`

- Parser and validation front ends
  - `pandaPIparser`
  - `HDDL-Parser`
  - `thtn`, parser side

- Grounded solver and post-processing stack
  - `pandaPIengine`
  - `pandaPIpgrRepairVerify`

- Translation and compilation systems
  - `ToadPlanningSystem`

- Service, environment, and hybrid application layers
  - `sh`
  - `HDDLGym`
  - `ChatHTN`

- Benchmark substrate
  - `ipc2020-domains`

## Design Considerations Across Codebases

- Input language choice drives structure.
  - HDDL and HPDL repos usually split parsing from planning.
  - Host-language repos usually merge domain authoring with planner runtime.

- Ordering discipline changes repo shape.
  - Total-order focused systems are more likely to compile into classical or automata-based forms.
  - Partial-order capable systems often keep richer task-network structures visible in the core model.

- Grounded versus lifted boundaries are explicit.
  - PANDA and TOAD style systems make grounding a pipeline boundary.
  - `IPyHOP` and `shop3` keep more semantics in host-language methods and runtime objects.

- Validation is now a separate architectural concern.
  - `HDDL-Parser` and `pandaPIparser` show that parsing is no longer only a preprocessing step.
  - Semantic checks, metadata extraction, and editor tooling are treated as durable infrastructure.

- Application context adds another layer above planning.
  - `sh` adds API and storage.
  - `HDDLGym` adds environment state, multi-agent coordination, and RL policies.
  - `ChatHTN` adds an approximate decomposition helper around a small symbolic core.

## High Level Synthesis

- There is no single dominant HTN codebase shape.
- The ecosystem clusters into a few repeatable architectures:
  - embedded planners in a host language
  - parser plus validator front ends
  - grounded engines with optional solver backends
  - translation pipelines into non-HTN substrates
  - service or simulation wrappers around hierarchical models
- The strongest cross-repo consensus is not one algorithm. It is a pipeline view of HTN work: model authoring, parsing, validation, normalization, solving or compilation, then analysis or integration.

## Local Clone Layout

- Repos are stored under [`repos/`](repos).
- Reports are stored under [`reports/`](reports).
- The master synthesis is this file.
