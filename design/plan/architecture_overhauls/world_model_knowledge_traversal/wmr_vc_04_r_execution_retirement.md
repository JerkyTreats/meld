# WMR VC-04-R Execution Authority Retirement

Date: 2026-09-02

Mode: design

Status: `WMR-VC-04-R1` revision 4 accepted, closed, committed, and pushed

Behavioral baseline: `fa956e2d4927a2e8ff5618dcb710c1cf458eeff83ecc0cc46ff016766a199ce0`

## Decision

The VC-04 Task route remains the durable behavioral base. Its revision 2 Gate Acceptance is invalidated by this design amendment because the gate accepted active-route exclusivity without establishing source retirement of the superseded Execution planning authority.

This is not a finding that Task completion failed. The accepted evidence remains valid for the behavior it actually proved. Architectural completion and handoff eligibility are withdrawn until the bounded retirement successor passes review and acceptance.

## Preserved Product Behavior

`WMR-VC-04-R1` must preserve:

- fresh Agent Task authorization with exact Goal, Plan, context, authority, policy, generation, and idempotency lineage
- durable Task admission with accepted, duplicate, rejected, and stale-fence decisions
- exact Capability closure, contract, binding, alias, multiplicity, authority, and generation validation
- direct lowering that preserves the admitted Task as the sole semantic body
- one independently attributed Task Network region per admission
- live policy and generation fences before claim and invocation
- production dispatch of the exact five Docs Capability types
- durable success and failure outcomes plus graph-aware regional terminality
- neutral Execution Event publication
- independent Graph progress
- exact Agent `ExecutionTerminal` absorption
- restart, replay, reopen, and duplicate suppression
- absence of `workspace_scan` and workspace owner return
- no inference of Docs correctness, Belief settlement, Goal satisfaction, owner truth, or quiescence from operational completion

## Current And Successor Routes

The superseded authority accepts Execution Goals, projects world state, searches Methods, selects action realization, creates `ExecutionComposition`, and either lowers that composition or selects a package route.

Its public construction path includes `PlanningRuntime`, `PlanningRuntimeActor`, `PlanningProjectionPort`, `PlanningRequest`, `PlanningResult`, `ExecutionComposition`, `ExecutionCompositionLowerer`, `MethodLibrary`, realization contracts, and `TaskPackageRoutePlan`.

The verified successor route is:

```text
Agent Plan and Task authorization
-> TaskAdmissionApi
-> durable Task admission
-> TaskAdmissionRuntimeActor
-> TaskAdmissionLowerer
-> one attributed Task Network region
-> claim and production dispatch
-> neutral publication
-> Agent terminal absorption
```

The successor currently runs under the stale runtime identity `execution.planning`. The old planner is absent from that root route but remains publicly constructible and therefore remains a semantic authority under the Runtime Invariants.

## Responsibility Disposition

| Responsibility | Mode | Incumbent authority | Successor | Required disposition | Proof |
| --- | --- | --- | --- | --- | --- |
| Goal selection and world-state projection | replace | `PlanningRuntimeActor` and `PlanningProjectionPort` | complete Agent-authorized Task admission | delete actor, projection port, projection adapter, exports, callers, and exclusive tests | no construction, export, registration, store, or test path remains |
| Method search and action realization | replace | `PlanningRuntime`, `MethodLibrary`, action and realization contracts | Strategy-authored Task plus consumer validation | delete semantic planning contracts, selection, realization, exports, tests, and fuzz targets | admitted Task reaches lowering without Method or realization search |
| composition lowering | replace | `ExecutionCompositionLowerer` | `TaskAdmissionLowerer` | delete old lowerer and composition grammar; rehome the successor outside `planning` | one semantic Task body from admission through Task Network mutation |
| Execution Goal mutation | replace | `GoalSetApi`, Goal command stores, and `PersistentGoalSetStore` | `TaskAdmissionApi` and Task admission records | delete writers, readers, store opening, and passive runtime registration; leave existing on-disk files untouched | canonical startup creates no legacy Goal store and exposes no old query or mutation API |
| package-plan route | replace | `TaskPackageRoutePlan`, `PackageRouteHandoffs`, and package-plan dispatch input | durable Task Network claim dispatch | delete handoff registry, package-plan input, preparation branch, reports, and sole-purpose adapters | dispatch consumes Task Network claims only |
| runtime participant identity | move | `execution.planning` plus passive `execution.goal_set` | `execution.task_admission` | remove old identities and aliases; register the successor once | exact runtime registration and handle inventory |
| legacy Task Network decoding | retain | planning lineage and diagnostic fields in historical journals | named legacy Task Network codec | retain decoding only; current writers cannot emit the old planning meaning | historical fixtures replay and current writes exclude legacy fields |
| generic task progress and aggregate records | retain | Task and Workflow artifact persistence outside the obsolete package-plan handoff | same existing Task and Workflow products | retain unchanged; delete only obsolete package-plan root writers and wiring | explicit Workflow regression and no package-plan dispatch input |
| explicit Workflow execution | retain | direct Workflow task preparation and execution | same separate product route | preserve unchanged; it cannot accept Agent Task authority | explicit Workflow regression and caller inventory |

## Policy Trace

| Policy obligation | Responsibility | R1 deliverable | Required evidence | Exception authority |
| --- | --- | --- | --- | --- |
| [Runtime Invariants](../../../../governance/runtime_invariants.md) require one canonical authority and same-change removal for replacement | Goal planning, Method search, composition lowering, Goal mutation, package handoff, and runtime identity | migrate every real caller and remove superseded planners, writers, selectors, registrations, exports, and exclusive tests | construction, caller, registration, export, store, test, fuzz, and root-runtime inventories after deletion | user only for a named temporary noncanonical route and removal condition |
| Runtime compatibility cannot preserve a second writer, planner, actor, selector, coordinator, or decision authority | legacy Goal data and Task Network decoding | no Goal compatibility surface; Task Network historical decode only | canonical startup creates no Goal store, exposes no old API, and current Task Network writes emit no old planning meaning | user only |
| Runtime replacement requires proof through the real entrypoint after deletion | complete VC-04 Task route | reproduce the direct root Task proof after retirement | exact Task admission through production dispatch, publication, Graph progress, and Agent terminal absorption | none |
| [Contribution Policy](../../../../governance/contribution_policy.md) requires obsolete code and supporting surface to be removed | all superseded Execution planning support | delete obsolete production files, exports, adapters, tests, fixtures, and fuzz targets | final responsibility inventory and complexity delta with removal account | user only |
| Contribution compatibility comments must state why retained code remains and when it can be removed | legacy Task Network codec | document historical decode ownership and removal condition | focused comment review and replay fixture | user for stable-contract designation |

## High-Confidence Removal Inventory

Delete the old planning source after extracting any proven legacy decoder:

- `crates/meld-execution/src/planning/action.rs`
- `crates/meld-execution/src/planning/contracts.rs`
- `crates/meld-execution/src/planning/lowering.rs`
- `crates/meld-execution/src/planning/method_library.rs`
- `crates/meld-execution/src/planning/realization.rs`
- `crates/meld-execution/src/planning/runtime.rs`
- `crates/meld-execution/src/planning/world_state.rs`

Move `planning/task_admission.rs` into a Task-admission domain, then remove `planning.rs`. Move `goals/admission.rs` into that same domain, then remove the superseded Goal authority:

- `crates/meld-execution/src/goals/api.rs`
- `crates/meld-execution/src/goals/contracts.rs`
- `crates/meld-execution/src/goals/persistent_store.rs`
- `crates/meld-execution/src/goals/query.rs`
- `crates/meld-execution/src/goals/store.rs`
- `crates/meld-execution/src/goals.rs`

Remove old planning and lowering exports from `meld-execution`.

Remove root `PlanningTheoryBinding`, obsolete Method and realization fields, `PlanningFactory`, `PlanningHandle`, planning handle variants, `ExactKeyPlanningProjectionPort`, old planning report conversion, legacy Goal store opening and injection, stale harness stations, and Goals and Planning producer mappings.

Remove `PackageRouteHandoffs`, package-plan dispatch inputs, package-plan driving, sole-purpose package preparation and invocation bindings, package-plan checkpoints, and publication branches. There is no current production caller that records a package handoff.

Delete the old authority suites and their fuzz manifest entries:

- `crates/meld-execution/tests/composition_lowering.rs`
- `crates/meld-execution/tests/method_library.rs`
- `crates/meld-execution/tests/planning_contracts.rs`
- `crates/meld-execution/tests/planning_runtime.rs`
- `crates/meld-execution/tests/goals.rs`
- `crates/meld-execution/fuzz/fuzz_targets/fuzz_method_library.rs`
- `crates/meld-execution/fuzz/fuzz_targets/fuzz_planning_contracts.rs`
- `crates/meld-execution/fuzz/fuzz_targets/fuzz_planning_runtime.rs`

Rewrite Task Network fixtures that construct `ExecutionComposition` to enter through `TaskAdmissionLowerer`. Retain the composition fuzz target only if it exercises shared `meld_lang::Composition` independently of the deleted Execution planner, and rename it if its current name implies obsolete ownership.

## Compatibility Disposition

The current workspace-scoped XDG data root contains a Goal database with tree metadata but no observed Goal record payload, an empty Task Network directory, and a task-artifact database with no observed product trees. Current source exposes no required product reader of legacy Goals. R1 therefore retains no Goal reader and creates no compatibility system. It stops opening or creating the Goal database and leaves the existing on-disk directory untouched as recoverable forensic state.

If supported populated Goal records are demonstrated before source editing, that evidence is an anomaly and returns R1 to design. It does not permit the implementor to add a reader during delivery.

Current Task Network journals may contain pre-admission lineage, composition identifiers, and planning diagnostics. Historical decoding is a credible compatibility need. Move only required decoding into a named legacy Task Network codec and prevent current command submission from emitting legacy planning fields.

Generic Task and explicit Workflow progress and aggregate records remain part of separately retained products. R1 preserves their current stores and public behavior. It removes only obsolete package-plan handoff writers, dispatch inputs, and root wiring. Those retained products do not justify any old planner or Goal authority.

Every retained compatibility surface requires:

- evidence of supported persisted records or an explicit supported public read contract
- a named owning domain and real reader caller
- no writer, mutation facade, runtime registration, selector, or decision authority
- a historical decode or replay fixture
- proof that canonical startup does not create the old store or emit the old format
- a removal condition unless the reader is explicitly accepted as stable

Without that evidence, delete the surface.

## WMR-VC-04-R0 Design Amendment

This document completes `WMR-VC-04-R0` as design-only work. It:

- supersedes revision 2 architectural acceptance while preserving its behavioral evidence
- freezes the initial responsibility and removal inventory
- defines one atomic retirement successor
- leaves all source implementation, commit, push, deployment, and later-slice work unauthorized

## WMR-VC-04-R1 Product Contract

Product increment: preserve the verified VC-04 Task route while removing the superseded Execution planner, Goal writer, package-plan handoff, runtime identities, exports, and exclusive support surface so that one constructible Task admission and Task Network route owns Execution intake.

The source cut is atomic. It may not be split into separately accepted planner, Goal, or package retirement slices because any intermediate acceptance would knowingly retain parallel semantic authority.

Existing canonical Agent, Strategy, Task admission, Task Network, Capability dispatch, Events, Graph, and Agent return semantics are reused. Explicit Workflow behavior remains outside the write scope unless a direct compile dependency requires a semantics-preserving adapter change.

## R1 Acceptance Criteria

- every preserved VC-04 product proof passes with unchanged meaning
- no public old Planning runtime, Method search, action realization, composition lowerer, or package-route construction path remains
- no semantic `planning` module remains in `meld-execution`
- root registers `execution.task_admission` once and registers neither `execution.planning` nor `execution.goal_set`
- one Task admission writer and one Task Network mutation route remain
- no package-plan input, handoff registry, or package-plan dispatch branch remains
- dispatch drives durable Task Network claims only
- explicit Workflow behavior passes and cannot accept Agent Task authority
- historical Task Network fixtures replay through named read-only legacy decoding
- current writes cannot emit old Goal-driven planning records
- canonical startup does not create the legacy Goal database
- every retained compatibility surface satisfies the compatibility evidence standard
- logical review, Style Assurance, and Gate Acceptance judge one exact R1 candidate in order

## R1 Limits And Tripwires

The final limit is fifty changed production paths and twelve hundred added production lines. Production deletions are uncapped.

The initial forty-path estimate triggered a bounded design return during logical review. A rename-aware comparison from pre-retirement checkpoint `dae788ae98a546c3eafb8fc094604a86a8bdd1f6` to the exact worktree candidate establishes 49 changed production paths, 1,088 added production lines, and 8,305 removed production lines. The additional paths are the already-required atomic deletion inventory and canonical authority fences. They add no new responsibility, store, runtime, dependency, or deferred owner-return surface. The program-owner authorization to validate and fix the revised VC-04 requirements permits this evidence-backed correction before the review verdict.

Stop before implementation or return to design if:

- a compatibility writer or forwarding runtime is proposed
- an old runtime identity survives as an alias
- any canonical caller reaches old planning or package-plan logic
- historical Task Network decoding would be removed without fixture evidence
- Agent Strategy, Task meaning, Plan selection, or explicit Workflow behavior would change
- a new database, store, dependency, actor, service, or background runtime appears
- retirement would require a later cleanup slice
- retained compatibility exists only for speculative rollback
- the final path or addition limit is crossed

## Authorization State

`WMR-VC-04-R0` design amendment is complete. The program owner authorized `WMR-VC-04-R1` source implementation on 2026-09-02. The revision 3 acceptance was reopened after canonical admission integrity and planner projection retirement defects were found. Exact revision 4 candidate `WMR-VC-04-R1::5308f0ba6ce873bd1e00b4ba283d60eefc9df996f9ba261461fc44929c86c430` seals admission and attributed graph insertion behind their canonical authorities, verifies complete lowered node content at dispatch, and removes the remaining projection resource residue. It passed logical review, Style Assurance, and Gate Acceptance in order on 2026-09-04. Separate program-owner authority then committed and pushed the closed candidate. Deployment, VC-05, and every later slice remain unauthorized.
