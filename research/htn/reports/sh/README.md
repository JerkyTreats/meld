# `sh`

- Upstream: <https://github.com/PlanX-Universe/sh>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: Scala, branch `main`, commit `2d7dddc`

## Role

- `sh` is a modular planning system with both library and web-service entry points.
- It is structured like an application-facing HTN platform, where parsing, planning, storage, and HTTP delivery are all first-class concerns.

## Layout

- Build and dependency wiring live in `build.sbt`.
- Domain and problem parsers live in `src/main/scala/org/planx/sh/parsing/hpdl/`.
- Core model types live in `src/main/scala/org/planx/sh/problem/`.
- Search and plan generation live in `src/main/scala/org/planx/sh/solving/`.
- Service and API layers live in `src/main/scala/org/planx/sh/services/` and `src/main/scala/org/planx/sh/services/rest/`.
- Storage and utility helpers live in `src/main/scala/org/planx/sh/storing/` and `src/main/scala/org/planx/sh/utility/`.
- Entry points live in `client/Client.scala` and `services/rest/HTTPServer.scala`.

## HTN Structure

- `PlanningServices` orchestrates the main path from HPDL parsing to goal preprocessing to `PlanGeneration`.
- `PlanGeneration` performs recursive decomposition and operator execution over task and operator instances.
- The planner core is embedded in a service-oriented shell rather than exposed only as a standalone CLI.
- The presence of REST routes, repository storage, and client code means the planner is designed to be consumed by other systems, not only by local experiments.

## Design Considerations

- Parsing, problem objects, search, and delivery layers are cleanly separated by package.
- The repo favors deployable interfaces. HTTP and client entry points sit next to the solver rather than in a separate wrapper project.
- HPDL parsing is part of the main system boundary, so model ingestion is integrated with service execution.
- This is a good example of HTN software shaped by integration needs, not only by planner algorithm concerns.

## Cross Repo Takeaways

- `sh` represents the service-platform family in this set.
- It differs from `shop3` by leaning toward a typed application layout and from `HDDLGym` by leaning toward service delivery rather than simulation and learning.
- It reinforces a common deployment pattern: parser plus planner core wrapped behind a stable API layer.
