# Runtime Completion Implementation Workstreams

Status: operational-parity rebaseline under fresh review, blocked on product decisions

Evidence date: 2026-07-16

Source requirements: [Runtime Completion Ground Map](runtime_completion_ground_map.md)

## Objective Baseline

Requested outcome:

Turn the bounded convergence requirements into implementation workstreams that maximize safe parallel delivery while preserving domain ownership, objective-scoped review, and causal end-to-end proof.

Completion evidence:

- A minimal docs freshness stewardship expression is consumed by runtime composition and advances one semantic package run across multiple bounded actor turns.
- Planning selects the existing built-in `docs_writer` task package and `docs_writer_thread_v1` workflow route.
- Existing traversal expansion fans out across a branching selected workspace tree and preserves child-finalization-before-parent-preparation dependencies.
- Existing provider-backed workflow turns and workspace publication materialize `README.md` in every actionable in-scope folder.
- Acceptance inspects the target filesystem and durable execution records rather than accepting a synthetic patch, context frame, or top-level success alone.
- Resumable package execution persists compiled fan-out, dependency, expansion, artifact, and completion progress across bounded actor ticks.
- Aggregate package completion produces evidence only after all required per-folder generation and publication work completes.
- An unsatisfactory aggregate result may cause later planning only when distinct useful work remains.
- A satisfactory aggregate result crosses the satisfaction threshold and closes the goal.
- No useful work produces quiescence without hot-looping.
- New evidence wakes a quiescent active goal.
- The foreground runtime remains available after convergence for later drift.
- Minimum foreground availability remains truthful throughout work, quiescence, satisfaction, and later wake.
- Every implementation workstream has an independent objective, write scope, dependency contract, verification gate, and fresh review lane.

Explicit non goals:

- Firm retry, backoff, failure classification, recovery, and terminality policy
- Daemon management and interprocess control
- General runtime activation documents
- Dynamic agents, generalized scheduling, and multiple concurrent stewardship expressions
- Broad observability or reliability platforms
- Implementation during this design pass

Applicable policy:

- Domain-first ownership and thin adapters from repository `AGENTS.md`
- Assessment By Domain Policy
- Runtime Governance Policy
- Storage Policy
- Semantic Unit Preservation Policy
- CLI Targeting Policy
- Compatibility Policy
- Docs Style Policy
- Domain Design Orchestration skill

Completion point:

The workstream analysis is complete when fresh spec, architecture, boundary, durability, test, and orchestration findings are incorporated. Phased program delivery is ready only after the product decisions in the requirements gate are selected and recorded, then the coordinated domain contract gate is reviewed against those fixed choices.

## Preliminary Domain Map

Cross-domain contracts are frozen through one coordinated gate but remain owned by their source or receiving domain. Parallel analysts may identify contract needs but must not independently define competing DTOs, runtime reports, record identities, or event payloads.

| Domain cluster | Owning paths | Current anchors | Requested work | Likely write scope | Dependencies in | Dependencies out | Boundary risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Stewardship expression and physical binding | `src/config.rs`, `src/config/`, `src/init.rs`, `src/runtime/storage.rs`, `src/workspace.rs` | Validated config, product storage assembly, workspace targeting | Minimal docs freshness expression, subject, stale signal, XDG config and state boundaries | Root config, init, and storage adapters | Product decisions for subject, stale signal, bootstrap, provider | Runtime registrations and source adapters | Internal role language, runtime state, or runtime sequencing leaking into the stewardship expression |
| World model convergence | `crates/meld-world-model/src/belief.rs`, `crates/meld-world-model/src/agent.rs`, `crates/meld-world-model/src/planner.rs`, `crates/meld-world-model/src/world_state.rs` | Belief runtime, goal curation, satisfaction curation, planner projection, graph replay | Bounded selection actors, durable revised state, evidence mapping, satisfaction across turns | `meld-world-model` domain modules and tests | Committed facts, stewardship package semantics | Planner-facing world state and goal mutation | Root choosing evidence probability, source kind, or satisfaction policy |
| Execution convergence | `crates/meld-execution/src/goals.rs`, `crates/meld-execution/src/planning.rs`, `crates/meld-execution/src/task_network.rs` | Active goal store, planning actor, task command and publication actors | Active-goal query, repeated planning, bounded dispatch, real capability and workflow execution, durable outcomes | `meld-execution` domain modules and tests | Planner view, method and task package, provider and workspace ports | Publication facts and execution progress | Dispatch bypassing real paths or duplicating task artifact truth |
| Runtime composition and supervision | `src/runtime/assembly.rs`, `src/runtime/contracts.rs`, `src/runtime/supervisor.rs`, `src/runtime/ports.rs` | Product assembly, bounded supervisor ticks, graph replay binding | Truthful active and passive roles, generic actor binding, quiescence and wakeup, foreground continuity | Root runtime adapters and supervisor tests | Validated config and domain actor contracts | Lifecycle reports and bounded tick driving | Supervisor or root sequencing semantic handoffs |
| Event authority and evidence cursor | `crates/meld-events/src/events/subscription.rs`, `crates/meld-events/src/events/writer.rs` | Append, bounded replay, watermarks, consumer registry | Aggregate package outcome selection through an existing durable consumer boundary | Event domain contract plus world-model evidence adapter | Canonical package outcomes | Evidence cursor progress | A second ledger or caller-managed semantic replay |
| Product convergence proof | `tests/integration/`, runtime CLI tests, domain actor tests | Manual docs freshness contract and independent component tests | Supervisor-driven bounded package proof, workflow parity, quiescence, wake, and foreground availability | Integration fixtures and tests only after domain contracts settle | All runtime workstreams | Acceptance evidence | Test-only orchestration or synthetic outcomes masking missing product wiring |

## Initial Dependency Gates

Gate A settles the minimum product intent, satisfaction and evidence granularity, first stale signal, artifact mapping direction, and bootstrap ownership. These decisions constrain configuration and the shared domain contracts.

Gate B freezes coordinated domain-owned contract skeletons for actor readiness, bounded work reports, docs outcome mapping input, stable correlation identity, and product runtime registration. No parallel lane may create an alternate source of truth for these concepts.

After Gate B, world model, execution, event evidence, and physical binding analysis can proceed in parallel. Runtime composition integrates their public contracts after each owning lane has a reviewed packet. Product convergence proof design follows those contracts but may prepare fixtures and assertions in parallel without implementing semantic handoffs.

## Agent Strength And Concurrency Rule

Concurrency and reasoning strength are separate decisions. Parallel-safe work is assigned concurrently whenever write scopes and contracts are independent. Reasoning strength is assigned from task difficulty, boundary count, ambiguity, and consequence of error.

- High strength is required for coordinated domain contract freezing, world model to execution semantics, convergence causality, runtime ownership boundaries, and cross-workstream integration review.
- Standard strength is appropriate for localized config-path analysis, mechanical activity instrumentation inventories, focused test discovery, and bounded presentation adapters when their contracts are already frozen.
- Escalation occurs only when evidence shows unresolved ambiguity, cross-domain coupling, or inability to validate the objective at the current tier.

The current collaboration interface exposes equal-strength subagents and does not expose a model selector. This design pass therefore assigns the available agents only to high-reasoning analysis lanes. Future implementation packets retain explicit strength guidance so an execution environment with model selection can enforce it.

## Requirements Decision Gate

The following product decisions must be frozen before shared implementation contracts. They are allocated to one central decision gate because allowing workstreams to decide them independently would create incompatible sources of truth.

This gate is currently unresolved and blocks implementation. It is a product requirements gate, not an implementation workstream. Phased delivery must receive the selected values as fixed inputs rather than decide them inside a worker lane.

Resolved decision:

- Docs freshness is a minimal stewardship expression of operational domain theory and contains no runtime state.
- The stewardship package owns domain semantics, including evidence policy, maintenance methods, action meaning, outcome interpretation, success measurement, authority, and governance.
- Meld derives required internal actor and passive-service bindings from that theory and the physical runtime binding.
- Product slots and slot counts do not exist. The internal descriptor registry has no product cardinality meaning.
- Flywheel docs freshness must execute the existing `docs_writer` package, traversal fan-out, provider turns, and workspace publication route. Output-equivalent replacement machinery is out of scope.
- The selected workspace subtree root is the stewardship target. The existing docs writer traversal derives per-directory execution targets at runtime. Directory inventory and package progress are runtime state, not stewardship-package content.
- One package run may span many bounded actor ticks. Multiple ticks do not require multiple complete package runs.

| Decision | Affected workstreams | Why it is central |
| --- | --- | --- |
| Satisfaction and evidence granularity | World model, execution publication, product proof | Per-folder outputs must not satisfy the selected-tree goal before aggregate package completion |
| First stale signal and bootstrap owner | Physical binding, world model, workspace observation | The first loop needs one authoritative wake source without root sequencing semantic work |
| Deterministic proof provider | Physical binding, execution dispatch, product proof | The proof must use the real provider route without external nondeterminism |
| Publication artifact mapping direction | Execution, events, world model | Execution owns outcome facts while world model owns evidence interpretation |
| Future drift goal lifecycle | World model and execution | Current stable goal identity collides with an already satisfied goal on later drift |
| CLI path targeting | Config and CLI | Policy requires an explicit exception anywhere path is not the default target |

Firm retry, backoff, failure classification, recovery, and terminality mechanics are not part of this gate.

## Coordinated Domain Contract Gate

One integration owner coordinates and freezes these skeletons before parallel implementation begins. Each canonical contract remains defined by its owning domain:

- Root owns the validated minimal docs freshness selection and its physical runtime binding without redefining the stewardship package semantics.
- Root owns registration identity and lifecycle projection for required active actors, required passive services, unresolved required bindings, active idle, unhealthy, and stopped. Catalog-only descriptors receive no runtime registration or lifecycle state.
- Each actor domain owns its positive bounded request and domain report. Root owns only translation into its lifecycle projection.
- World model owns the exact docs subject binding, belief key, outcome-to-evidence mapping input, deterministic promoted-evidence identity, consumer identity, and evidence interpretation.
- Execution owns the canonical publication outcome, artifact identity, future goal lifecycle command behavior, and task package selection boundary.
- Events owns canonical `EventRecord` values plus the durable evidence-consumer cursor and cursor mutation contract.
- Root owns runtime result and end-to-end fixture coordination.

Owning domains may expose domain-specific actor reports. Root translates those reports into the shared supervisor shape. Domain crates must not depend on root runtime contracts merely to satisfy supervision.

## Workstream One Stewardship Expression And Physical Binding

Owner: root config and product assembly adapters

Objective baseline:

- Select one minimal docs freshness stewardship expression and resolve its workspace, subject, agent, provider, and external state without exposing internal actor topology or runtime state.
- Preserve side-effect-free config loading and workspace purity.

Current anchors:

- `src/config.rs:40-65` has no minimal docs freshness selection and target expression.
- `src/config/merge/service.rs:13-23` always merges workspace config from a supplied root.
- `src/config/sources/global_file.rs:10-18` uses `HOME` instead of the existing XDG resolver.
- `src/config/workspace/storage_paths.rs:74-144` already enforces an external product root.
- `src/runtime/assembly.rs:64-83` currently exposes internal runtime identifiers as activation input.
- `src/cli/runtime_assembly.rs:32-155` is the correct physical composition boundary.

Implementation guide:

1. Add the centrally frozen minimal docs freshness selection and source-aware validation.
2. Resolve the default file through XDG config home.
3. Require explicit workspace-local config selection and preserve explicit config-file loading.
4. Add a pure physical binding resolver for workspace, subject, agent, provider, stewardship package, and external storage.
5. Produce owner-scoped domain registrations without semantic sequencing.
6. Replace empty internal-role expansion with registrations derived from the selected stewardship expression and physical binding.

Boundary rules:

- Root config must not contain evidence probability, satisfaction threshold, task package meaning, stale-signal interpretation, tick order, or test vocabulary.
- Config loading and validation must not create runtime state.
- No runtime state may be written under the target workspace.
- The twelve internal descriptors are not a user configuration schema.

Verification:

- XDG config home wins over HOME.
- Workspace config is absent unless explicitly selected.
- Invalid docs fields identify the source and field.
- Default source resolution and the validated binding remain identical across unrelated process working directories.
- Loading and validation create no state.
- Product state, supervisor state, cursors, receipts, runtime-managed artifact repositories, and artifact records remain outside the workspace.
- Task semantics publish only the intended documentation result through the workspace-owned path.
- One docs freshness selection causes runtime composition to derive only the actor and passive-service registrations required by the convergence loop.

Parallel shape:

After the central binding skeleton freezes, XDG source correction, config parsing, and storage-path tests are disjoint standard-strength tasks. Binding resolution and registration production require high strength. A fresh reviewer owns product-language purity, source-aware validation, side-effect freedom, and storage purity.

## Workstream Two World Model Convergence

Owner: `meld-world-model`

Objective baseline:

- Discover and process bounded belief, goal, evidence, and satisfaction work from durable domain state.
- Make unchanged unsatisfied state quiescent and make new evidence eligible without caller-manufactured sequences.
- Preserve exact causal revision identity for later planning.

Current anchors:

- Graph replay already provides the bounded actor pattern at `crates/meld-world-model/src/world_state/graph/runtime.rs:34-244`.
- Belief assessment persists revisions, views, evidence, assignments, and leases at `crates/meld-world-model/src/belief/runtime.rs:89`.
- Dirty belief state is durable at `crates/meld-world-model/src/belief/store.rs:606-645`.
- Goal curation persists a decision and sink receipt before advancing its cursor at `crates/meld-world-model/src/agent/runtime.rs:185-310`.
- Satisfaction persists its decision before mutation submission at `crates/meld-world-model/src/agent/runtime.rs:475-565`.
- Root currently hardcodes docs evidence policy at `src/runtime/ports.rs:472-553`.

Spec gaps:

- Belief assessment has no bounded selector for initial or outdated subjects.
- Goal curation receives caller-created deliveries rather than selecting newer revisions.
- Evidence ingestion receives caller-mapped evidence and owns no event consumer selection.
- Satisfaction receives caller-created review sequences and cannot distinguish unchanged absorbed state from newly eligible state.
- Planner projection does not query the exact configured belief key.
- Later drift derives the same goal identity that execution already treats as a satisfied duplicate.

Implementation guide:

1. Add exact-key revision and planner projection queries.
2. Add deterministic bounded selection for initial assessment and dirty keys.
3. Implement a bounded belief assessment actor.
4. Add world-model-owned event replay, cursor-reporting, and typed outcome mapping ports that accept the intact canonical `EventRecord`.
5. Implement bounded evidence selection and ingestion with cursor advancement only after durable domain progress.
6. Add bounded goal-delivery eligibility from subscription and belief revision state.
7. Add satisfaction trigger eligibility derived from durable goal, belief revision, decision, and receipt state.
8. Add only the smallest new satisfaction checkpoint record if existing indexes cannot derive eligibility without scanning.
9. Implement bounded goal curation and satisfaction actors.

Durability and idempotency:

- Existing belief revisions, views, dirty state, subscriptions, decisions, sink receipts, graph cursor, and event consumer cursor remain authoritative.
- Applicable evidence must be durable before the authority cursor advances.
- Promoted-evidence identity must derive deterministically from canonical publication and mapping identity so replay cannot create distinct evidence.
- Events owns durable cursor mutation while world model owns the consumer identity and advancement request after its state is durable.
- Non-applicable understood events may advance the cursor.
- Reopen after evidence commit and before cursor commit must replay idempotently without a second assignment, revision, or confidence increase.
- An unchanged low-confidence revision becomes ineligible after one absorbed satisfaction review.
- A missing mutation receipt keeps the same trigger replayable without inventing a new review sequence.

Parallel shape:

After the central subject, mapping, report, and future-drift contracts freeze, three high-strength sublanes can run concurrently:

- Belief selection and exact planner causality
- Evidence mapping and event cursor ordering
- Goal delivery and satisfaction eligibility

One integration owner reserves parent exports, shared domain report adapters, and registration constructors. Fresh reviewers independently verify bounded revision progress, evidence cursor ordering, and no-hot-loop satisfaction eligibility. Instrumentation and mechanical scans are standard-strength follow-up tasks.

Verification:

- `cargo test -p meld-world-model`
- Reopen tests for every actor checkpoint
- First evidence remains below threshold and second distinct evidence crosses it
- Second planner projection carries the second belief revision and changed source references
- Repeated unchanged ticks attempt and commit no work
- New revision wakes a quiescent active goal
- Static scans reject execution internals, root logging dependencies, `mod.rs`, hardcoded docs policy in generic modules, and unbounded actor entry points

## Workstream Three Execution Planning And Docs Writer Selection

Owner: `meld-execution` planning, goals, lowering, and task package boundaries

Objective baseline:

- Read active goals, consume exact revised world state, select the existing built-in `docs_writer` task package and `docs_writer_thread_v1` route, and submit bounded task-network work without duplicating prior materialization.

Current anchors:

- `PlanningRuntimeActor` already reads active goals and processes an optional bounded limit at `crates/meld-execution/src/planning/runtime.rs:207-274`.
- Each request retains a planner frame and world-state request at `crates/meld-execution/src/planning/runtime.rs:312-318`.
- Existing tests prove repeated identical planning skips already materialized work at `crates/meld-execution/tests/planning_runtime.rs:489`.
- Current lowering creates capability tasks from operators, while the real docs writer is prepared through the task-package workflow route.

Spec gaps:

- Product assembly does not bind the planning actor.
- The execution-owned active-goal query is not exposed through the narrow planning boundary.
- The available docs action binding is not frozen between stewardship package semantics and execution lowering.
- Any replan after an unsatisfactory aggregate outcome needs a stable relationship among goal, revised planner frame, prior package-run identity, plan identity, and new task network mutation.
- Repeated planning must distinguish revised useful work from duplicate materialization without using runtime-global scheduling state.

Implementation guide:

1. Freeze the execution-owned available-action input derived from stewardship package semantics.
2. Expose the narrow active-goal query contract for execution planning.
3. Require the exact planner projection identity in planning input and deterministic plan identity.
4. Adapt method lowering to the existing docs task package and workflow route without adding a package compiler or a flywheel-specific writer.
5. Preserve the package-authored bottom-up traversal, sibling fan-out, child-finalization-before-parent-preparation dependencies, provider turns, frame publication, and workspace `README.md` writes.
6. Keep task network command passive and durable.
7. Ensure unchanged planning input replays or skips deterministically while revised input can create refined work.

Boundary rules:

- Execution owns operational commitment, task package selection, lowering, and task network commands.
- Execution must not interpret confidence or choose satisfaction policy.
- Root may select the stewardship package and inject its typed execution binding but may not manufacture a plan or task mutation.
- No composite docs orchestrator may sequence planning, dispatch, publication, and evidence.

Parallel shape:

Planning causality and package selection require high strength because they join formal world state, domain method semantics, and durable task identity. Active-goal query exposure is standard-strength after contracts freeze. A fresh reviewer must prove that the real package path is selected without root semantic logic and that any later replan consumes an unsatisfactory aggregate outcome rather than partial folder progress.

Verification:

- `cargo test -p meld-execution --test planning_runtime`
- Existing identical-input replay remains idempotent.
- Revised planner frame produces a causally distinct or refined accepted result.
- The lowered work resolves to the real docs writer package and workflow path.
- The lowered graph retains sibling fan-out and child-finalization-before-parent-preparation dependencies for a branching target tree.
- Planning never reads belief internals or raw world-model storage.

## Workstream Four Execution Dispatch And Publication

Owner: `meld-execution` for dispatch and publication, with separate adapter scopes owned by capability, provider, task, workflow, workspace, prompt context, and root runtime

Objective baseline:

- Claim and execute bounded ready work through the real workspace scan, existing docs writer package, provider, traversal fan-out, frame publication, workspace writer, artifact repository, task outcome, and publication paths.

Current anchors:

- Fenced claim and task outcome contracts exist at `crates/meld-execution/src/task_network/dispatch.rs:31-113`.
- Claim helpers intentionally leave capability invocation to the runtime host at `crates/meld-execution/src/task_network/dispatch.rs:115-134`.
- Publication already appends idempotent canonical events and persists identity-bearing receipts at `crates/meld-execution/src/task_network/publication.rs:252-330`.
- Real workspace scan and docs writer behavior have independent integration tests.

Spec gaps:

- No bounded actor discovers ready tasks, claims one bounded set, invokes the real execution route, persists artifacts, and records outcomes.
- The existing execute-to-completion helper releases all ready invocations and completes the in-memory package graph in one call, so it cannot be one actor tick for an unbounded tree.
- Expanded task executor, expansion, artifact, and completion progress cannot yet be reopened and resumed between bounded ticks.
- Provider assembly exposes availability rather than a real execution context.
- The runtime proof currently synthesizes a `docs_patch` artifact and success outcome.
- Real writer artifacts and publication payloads are not yet aligned with the world-model mapping input.

Implementation guide:

1. Freeze the canonical outcome and artifact identity consumed by the world-model mapping.
2. Add deterministic bounded ready-task selection and claim identity.
3. Add a resumable package-step contract that releases no more than the tick budget of ready capability invocations while preserving sibling fan-out and compiled dependency edges.
4. Persist expanded executor, expansion, artifact, readiness, and completion progress before the tick returns.
5. Build the dispatch actor inside `meld-execution` over public capability, provider, task, workflow, workspace, prompt, and artifact ports.
6. Persist real artifacts before accepting the terminal task-network outcome.
7. Record success or bounded invocation failure through the existing task-network command boundary before the tick returns.
8. Bind the existing publication actor to aggregate package completion.
9. Preserve canonical task outcome, per-folder publish results, and artifact records intact inside the aggregate publication payload.

Durability and idempotency:

- Execution changes remain under `crates/meld-execution` and depend only on explicit public ports.
- Each top-level adapter domain owns any required change under its own `src` domain path.
- Root runtime composes public ports but does not reach into capability, provider, task, workflow, workspace, or prompt internals.
- Claims remain fenced by existing task instance, claim revision, worker, and idempotency identity.
- Artifact records remain the task-owned canonical product.
- Task outcome becomes authoritative only through the task-network command boundary.
- Package progress is durable and resumable without reconstructing completed work from diagnostics.
- Failed bounded work does not count as progress, does not satisfy the goal, and does not hot-loop. Firm retry, recovery, interrupted-claim, and terminality mechanics remain deferred.
- Publication receipts remain identity-bearing and idempotent.
- No invocation journal or generalized fencing layer is added.

Parallel shape:

The resumable package-step contract, dispatch actor, and provider execution binding are high-strength tasks with separate write scopes. Publication actor binding is standard-strength after the aggregate outcome contract freezes. Capability, provider, task, workflow, workspace, prompt, and root adapter work may run as independent lanes only after their public contracts freeze. They may run parallel to world-model work because interaction occurs only through the frozen canonical publication contract. A fresh reviewer must execute the real route and reject synthesized artifacts, direct workspace writes outside task semantics, bypassed provider boundaries, or in-memory execute-to-completion calls inside one actor tick.

Verification:

- `cargo test -p meld-execution --test task_network_execution_bridge`
- `cargo test -p meld-execution --test task_network_publication_bridge`
- `cargo test --test integration_tests workspace_scan_capability`
- `cargo test --test integration_tests docs_writer_task`
- One actor tick never claims beyond its budget.
- Real artifact identity survives task outcome and publication unchanged.
- A branching fixture produces `README.md` in every actionable in-scope folder through the workspace publication capability.
- Durable dependencies prove every child final frame completed before preparation of its dependent parent.
- Existing reuse and idempotent expansion behavior survives flywheel dispatch.
- Reopen between bounded waves resumes the same package run without repeating completed provider calls or workspace writes.
- Duplicate dispatch and publication do not duplicate domain outputs.
- Failed bounded work cannot produce satisfaction evidence or immediate self-wake.

## Workstream Five Runtime Composition And Supervision

Owner: root runtime assembly and supervisor

Objective baseline:

- Supervise only concrete actors, report lifecycle truthfully, run one bounded tick per actor pass, reach active idle without semantic writes, and remain available for later evidence.

Current anchors:

- Twelve descriptors are declared at `src/runtime/assembly.rs:625-670`.
- Only graph replay and event append diagnostics have semantic handles at `src/runtime/assembly.rs:846-887`.
- Missing semantic tick reports currently become healthy at `src/runtime/supervisor/entrypoint.rs:1012-1022`.
- Each supervisor pass already calls at most one tick per active handle at `src/runtime/supervisor/entrypoint.rs:436-501`.
- The foreground loop already waits for cancellation or optional duration and uses event commitment for early wake at `src/runtime/tooling.rs:268-375`.

Implementation guide:

1. Split descriptor catalog entries from concrete active actor factories.
2. Remove passive services and no-op placeholders from lease acquisition.
3. Bind only real actor factories supplied by owner-scoped registrations.
4. Make unavailable active registrations truthful rather than healthy.
5. Require every started actor to return a bounded domain report.
6. Translate real zero-work reports into active idle.
7. Preserve one bounded invocation per active actor per maintenance pass.
8. Keep committed event notification as generic wake transport and heartbeat as fallback.
9. Preserve foreground continuity across active idle and satisfaction.
10. Keep domain cursors, confidence, goal state, and semantic order out of supervisor storage.

Boundary rules:

- Supervisor owns lifecycle, leases, bounded ticking, health, safe points, and shutdown.
- Domain stores own readiness, progress, quiescence eligibility, and resumption.
- Active goal presence alone is not actor readiness.
- Existing restart machinery is not an acceptance dependency and must not drive convergence design.

Parallel shape:

False-health correction and lifecycle projection can run at high strength beside domain actor work after registration contracts freeze. Per-domain actor adapters can be implemented in parallel by owning lanes. One root integration owner reserves the handle registry and startup package. A fresh reviewer must drive zero-work, event wake, no-self-wake, continued foreground, and truthful unavailable traces without using semantic test orchestration.

Verification:

- Passive services receive no lease or actor health.
- Unavailable actors are never healthy.
- Real zero-work reports are active idle.
- Repeated idle ticks do not change semantic checkpoints or flood activity.
- Events emitted during a tick do not immediately self-wake a hot loop.
- New committed evidence wakes eligible work.
- Satisfaction does not terminate the foreground runtime.
- `cargo test --test integration_tests runtime_cli`
- Focused supervisor and assembly unit tests

## Deferred Live Account And Presentation Work

The former event-follow and two-lane CLI workstreams are not prerequisites for operational parity. Runtime completion keeps only a minimal foreground availability proof that the process remains alive through bounded work, quiescence, satisfaction, and later wakeup.

Committed event following, terminal stream representation, broad domain activity instrumentation, log relocation, sink failure matrices, and ordered final-watermark draining are deferred follow-on work. They must not occupy implementation lanes until the real branching docs writer route converges through resumable bounded execution.

## Workstream Six Product Convergence Proof

Owner: root integration tests

Objective baseline:

- Prove the configured product assembly converges through multiple causally linked bounded turns, the existing branching docs writer route, and per-folder filesystem output without direct semantic handoff calls after startup.

Implementation guide:

1. Build a deterministic branching workspace and provider fixture through the same public binding used by real runs.
2. Seed only the centrally selected initial observation or bootstrap input.
3. Start product assembly and the foreground runtime.
4. Drive only lifecycle operations and bounded observation time.
5. Prove the real docs writer task expansion preserves sibling fan-out and child-finalization-before-parent-preparation dependencies.
6. Advance the package through multiple bounded actor ticks and reopen durable state between steps.
7. Prove provider-backed workflow turns and workspace publication create or update `README.md` in every actionable in-scope folder.
8. Compare the README path set and deterministic contents with the existing workflow route over an equivalent fixture.
9. Prove durable execution outcomes retain the selected tree and per-folder subject relationship without caller reattachment.
10. Prove no satisfaction evidence is eligible before aggregate package completion.
11. Prove aggregate completion evidence crosses the threshold and closes the goal.
12. Prove unchanged no-progress state remains quiescent across repeated supervisor ticks.
13. Append new evidence and prove wakeup.
14. Introduce later drift through the selected real observation contract while the original foreground runtime remains alive.
15. Prove the selected goal lifecycle rule makes bounded work eligible again without direct goal or satisfaction calls from the test.
16. Prove minimum foreground availability from the same run.

Forbidden proof behavior:

- Calling belief assessment, curation, planning, task mutation, publication, evidence ingestion, or satisfaction directly after startup
- Injecting synthetic artifacts or task outcomes
- Replacing the existing docs writer package or bypassing its traversal, provider, frame publication, or workspace write paths
- Treating context frames or top-level task success as proof of per-folder filesystem output
- Using direct CLI graph catch-up
- Advancing domain cursors from the test
- Claiming causality from task count alone

Parallel shape:

Branching fixture construction, deterministic provider behavior, workflow-baseline execution, and expected filesystem assertions may be prepared at standard strength after the coordinated domain contract gate. Final convergence proof implementation and review require high strength and wait for integrated product assembly. A fresh reviewer runs the proof from validated configuration and inspects external workspace results, durable package records, aggregate outcomes, and goal state.

Verification:

- `cargo fmt --all -- --check`
- `cargo test -p meld-events`
- `cargo test -p meld-world-model`
- `cargo test -p meld-execution`
- `cargo test --test integration_tests docs_freshness`
- `cargo test --test integration_tests runtime_cli`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Conditional Vertical Write Packets

These packets become executable only after the requirements decision gate closes. Exact new filenames may be selected within the listed domain path, but a worker must not write outside its packet without integration-owner approval.

| Packet | Owned paths | Frozen contract inputs | Reserved files and merge order |
| --- | --- | --- | --- |
| Config source and intent | `src/config.rs`, `src/config/` | Root-owned minimal selection and physical binding schema, source precedence, selected stewardship package and target | Root config owner merges before product assembly |
| Belief selection | `crates/meld-world-model/src/belief.rs`, `crates/meld-world-model/src/belief/`, `crates/meld-world-model/src/planner.rs`, `crates/meld-world-model/src/planner/`, focused tests | Subject binding, exact belief key, bounded request, domain report | World-model integration owner reserves public exports |
| Evidence ingestion | New behavior files under `crates/meld-world-model/src/belief/`, focused evidence tests | Intact `EventRecord`, mapping identity, promoted-evidence identity, event cursor mutation port | Merges after event contract and before world-model root adapter |
| Agent convergence | `crates/meld-world-model/src/agent.rs`, `crates/meld-world-model/src/agent/`, focused agent tests | Goal lifecycle rule, belief revision identity, bounded request, goal and mutation ports | World-model integration owner resolves any shared store or export edit |
| Execution planning | `crates/meld-execution/src/goals.rs`, `crates/meld-execution/src/goals/`, `crates/meld-execution/src/planning.rs`, `crates/meld-execution/src/planning/`, focused planning tests | Exact planner frame, package selection input, active-goal query | Execution integration owner reserves public exports |
| Resumable package execution | `crates/meld-execution/src/task.rs`, `crates/meld-execution/src/task/`, focused task executor tests | Bounded ready-invocation budget, durable executor and expansion state, existing compiled package graph | Execution integration owner reserves public exports |
| Execution dispatch | `crates/meld-execution/src/task_network.rs`, `crates/meld-execution/src/task_network/dispatch.rs`, new dispatch behavior files, focused task-network tests | Public execution ports, bounded request, package-step contract, canonical aggregate outcome | Merges after execution planning and resumable package contracts and before root actor binding |
| Execution publication | `crates/meld-execution/src/task_network/publication.rs`, focused publication tests | Canonical task outcome and event append sink | May run beside dispatch after outcome contract freezes |
| Domain adapters | Only the required files under `src/capability.rs` and `src/capability/`, `src/provider.rs` and `src/provider/`, `src/task.rs` and `src/task/`, `src/workflow.rs` and `src/workflow/`, `src/workspace.rs` and `src/workspace/`, `src/prompt_context.rs` and `src/prompt_context/` | Public ports frozen by the owning producer or consumer domain | One owner per top-level domain, no root-runtime internals |
| Supervisor truth | `src/runtime/supervisor.rs`, `src/runtime/supervisor/`, `src/runtime/contracts.rs` | Root registration classification and translated lifecycle report | Merges before actor binding and foreground account |
| Runtime actor binding | `src/runtime/assembly.rs`, `src/runtime/ports.rs`, `src/runtime/storage.rs`, `src/cli/runtime_assembly.rs` | Reviewed domain actor constructors, ports, reports, stewardship expression, and physical binding | Single high-strength root integration owner |
| Foreground availability | `src/runtime/tooling.rs`, required routing edits in `src/cli/route.rs`, focused runtime tests | Runtime actor binding, quiescence and wake contract | Single root owner, merges after supervisor truth |
| Product proof | Replacement convergence and workflow parity test files under `tests/integration/` | All reviewed public contracts and branching deterministic fixture | Merges last after product assembly and foreground availability |

Reserved parent exports such as `crates/meld-world-model/src/lib.rs`, `crates/meld-execution/src/lib.rs`, root `src/lib.rs`, and Cargo manifests are changed only by the named domain integration owner after sublane review. Workers send export and dependency requests to that owner instead of editing these files concurrently.

Root startup integration is not split across worktrees when changes require the same lifecycle ordering. Runtime actor binding lands before foreground-availability reconciliation. The product proof lands only after both compile together.

## Parallel Delivery Waves

The active environment supports three worker agents beside one orchestrator. Each build wave therefore uses up to three concurrent worker lanes. A lane is delayed only by a real contract or write-scope dependency, not by phase numbering alone.

### Gate Wave

No implementation begins until the user-selected requirements decisions are recorded. One high-strength integration owner then coordinates commits for the domain-owned public contract skeletons. Fresh architecture and boundary reviewers must approve this gate before domain implementation begins.

### Foundation Wave

Run these lanes concurrently:

| Lane | Scope | Strength | Exit gate |
| --- | --- | --- | --- |
| Stewardship selection and physical binding foundations | XDG source, explicit config selection, minimal selection schema, pure binding resolver | High for schema and binding, standard for localized path work | Policy and storage review passed |
| Resumable package execution foundation | Bounded package-step contract and durable expanded-executor state | High | Semantic-unit and reopen review passed |
| Belief selection and planner causality | Exact-key queries, bounded assessment selector, belief actor | High | World-model causality review passed |

Each domain integration owner reserves its parent exports and public contract files. The root integration owner reserves root lifecycle adapter files. Workers use disjoint worktrees when implementation begins.

### Domain Convergence Wave

Run these lanes concurrently after the relevant Foundation Wave contracts pass review:

| Lane | Scope | Strength | Exit gate |
| --- | --- | --- | --- |
| World-model evidence | Mapping port, bounded event ingestion, cursor ordering | High | Idempotency and policy-ownership review passed |
| World-model agent convergence | Goal delivery selection, satisfaction eligibility, future-drift rule implementation | High | No-hot-loop and durable-receipt review passed |
| Execution planning | Active-goal query, revised-state plan identity, docs package selection and lowering | High | Causal-plan and package-path review passed |

### Execution And Lifecycle Wave

Run these lanes concurrently:

| Lane | Scope | Strength | Exit gate |
| --- | --- | --- | --- |
| Execution dispatch | Bounded claim, real scan and writer route, artifacts, outcomes, publication binding | High | Real-route and artifact-truth review passed |
| Supervisor truthfulness | Concrete actor readiness, passive state, active idle, bounded tick projection | High | Lifecycle neutrality review passed |
| Aggregate package publication | Selected-tree identity, expected directory set, per-folder publish receipts, aggregate completion outcome | High | No-premature-satisfaction review passed |

### Product Integration Wave

Run these lanes concurrently after all owning actor contracts are reviewed:

| Lane | Scope | Strength | Exit gate |
| --- | --- | --- | --- |
| Runtime actor binding | Registrations, ports, report translation, assembly factories | High | Clean-boundary integration review passed |
| Stewardship outcome interpretation | Aggregate package outcome to selected-tree evidence mapping | High | Epistemic-boundary review passed |
| Parity fixture preparation | Branching depth-and-breadth workspace, deterministic provider, workflow baseline assertions | Standard, escalating to high for causal assertions | Workflow-parity review passed |

### Proof And Closeout Wave

Run flywheel parity execution, minimum foreground-availability proof, and repository-wide quality gates concurrently where commands do not contend for the same mutable test resources. The branching filesystem parity and bounded durable convergence proof remains the authoritative exit gate and requires high-strength implementation and review.

## Independent Review Protocol

Every workstream receives a fresh-context reviewer who did not implement that workstream. Review prompts contain the objective baseline, owned write scope, frozen contracts, applicable policies, and verification evidence. They do not contain the implementer conclusion.

Review findings use this shape:

- Severity
- Domain
- File or guide section
- Objective baseline item or policy basis
- Blocking or deferred disposition
- Concrete correction only when blocking

Each review maintains the workstream objective. Reviewers must reject scope expansion into comprehensive failure mechanics, generalized activation, broad observability, or unrelated cleanup unless the finding proves false progress, false satisfaction, lost durable state, a hot loop, or a direct policy violation.

Per-workstream reviewers focus on:

| Workstream | Independent review objective | Reviewer strength |
| --- | --- | --- |
| Stewardship selection and physical binding | Domain-language purity, XDG behavior, side-effect freedom, workspace purity | High |
| World model | Epistemic ownership, bounded eligibility, exact revision causality, quiescence | High |
| Execution planning | Revised-state consumption, package selection ownership, idempotent plan identity | High |
| Execution dispatch | Real execution route, bounded claims, artifact truth, publication identity | High |
| Supervisor | Lifecycle neutrality, truthful actor state, active idle, no self-wake | High |
| Resumable package execution | Bounded ready waves, durable reopen, preserved fan-out and dependency semantics | High |
| Foreground availability | Continued process life through work, quiescence, satisfaction, and wake | High |
| Product proof | No semantic test orchestration, real workflow parity, aggregate evidence, wake and drift availability | High |

After packet reviews pass, fresh program reviewers cover six lanes:

- Spec coverage
- Domain architecture validity
- Clean boundary check
- Durability and idempotency
- Test sufficiency
- Orchestration readiness

These six lanes may be paired across three high-strength reviewers to use all available concurrency without weakening independence. Any correction that changes a shared contract, domain owner, or dependency order triggers focused re-review by an agent that did not author the correction.

## Commit And Integration Gates

Future implementation follows these gates:

- Each completed workstream vertical ends in at least one reviewed atomic commit.
- Domain-owned public contract commits land before dependent worktrees branch.
- Each delivery wave integrates only reviewed committed verticals.
- Formatter evidence is recorded before test evidence for source or Markdown changes.
- Breaking contract changes are called out before commit and follow Compatibility Policy.
- Final closeout has a clean committed worktree or an explicit no-commit exception naming dirty files, reason, owner, and next action.
- Push always requires separate user confirmation.

The current design pass creates no implementation commit.

## Design Review Disposition

The operational-parity rebaseline invalidates the former completion review. Its live-account, two-complete-attempt, synthetic artifact, and in-memory dispatch assumptions are not implementation authority.

Fresh independent analysis established the new blocking requirements:

- Execute the existing docs writer package rather than an output-equivalent replacement.
- Preserve sibling fan-out and child-finalization-before-parent-preparation dependencies.
- Advance one package run through bounded durable steps rather than one in-memory execute-to-completion call.
- Prove physical per-directory README parity against the existing workflow route.
- Interpret only aggregate package completion as selected-tree satisfaction evidence.
- Defer comprehensive failure and live-account mechanics until operational convergence passes.

Final spec, architecture, durability, test, and orchestration reviews must run again after the remaining satisfaction and evidence decision is recorded and the revised packets are internally consistent.

## Buildout Handoff

Source plan:

- `design/plan/integration/runtime_completion_ground_map.md`

Synthesized guide:

- `design/plan/integration/runtime_completion_implementation_workstreams.md`

Current readiness:

- Operational parity and bounded package execution are now the objective baseline.
- Implementation handoff remains blocked until the remaining product decisions are selected and the rebaseline receives fresh review.
- Durable resumable execution of the expanded docs writer package is a real missing capability, not a configuration detail.

Parallel-safe domains after the gate:

- Product config and physical binding
- World-model belief, evidence, and agent sublanes under reserved exports
- Execution planning, resumable package execution, dispatch, and publication under reserved exports
- Top-level adapter domains with one owner per domain path
- Branching workflow parity fixture preparation

Sequential gates:

1. Record selected product decisions.
2. Freeze and review domain-owned public contracts.
3. Complete and independently review domain verticals.
4. Integrate supervisor truth and runtime actor binding.
5. Integrate minimum foreground availability.
6. Run the branching workflow parity and bounded convergence proof.
7. Run formatter, focused crates, integration tests, clippy, commit cleanliness, and final reconciliation.

Residual risks requiring decision:

- Later drift needs an explicit goal reactivation or episode identity rule.
- Initial observation needs a selected source and bootstrap owner.
- The stewardship-to-execution available-action binding needs a fixed typed seam.
- Aggregate selected-tree satisfaction and evidence granularity remains a product choice.
- CLI path targeting remains a public choice.
- Broader retry, recovery, interruption, live-account, and presentation policy remains deferred.

Implementation orchestration should use phased program delivery. Each phase vertical should use solo vertical delivery with isolated worktrees for disjoint write scopes, fresh objective reviewers, mandatory reviewed commits, and reconciliation through the reserved integration owners.
