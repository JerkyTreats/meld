# Runtime Completion Ground Map

Status: operational-parity rebaseline under collaborative requirements planning

Evidence date: 2026-07-16

Baseline branch: `event-foundation-closeout`

Baseline revision: `1f6dc1d`

Amendment date: 2026-07-24 — Strategy continuity constraints, harness enablement hooks, and observability emission requirements added. Fresh review runs against the Strategy corpus and [Strategy Ground Map](../world_model/strategy/ground_map.md) rather than the July baseline alone.

## Concern

This assessment maps the work required to complete one honest, supervised bounded convergence loop of the Meld cognitive flywheel with docs freshness as the first operational domain theory. It evaluates runtime assembly, lifecycle supervision, configuration, domain contracts, resumable package execution, durable aggregate evidence, physical README output, and the current proof surface. It also uses the Persistent Domain Stewardship proposal as non-authoritative boundary grounding. It does not prescribe the implementation before the remaining product requirements are chosen.

## Authority Boundary

The branch `production-cognitive-runtime-closure` and the production closure design artifacts are not implementation authority for this effort. They may be used only as negative evidence for accidental complexity, user configuration violations, or architecture pressure that this effort must avoid. No code or design is to be copied from that branch merely because it is technically complete.

GitHub PR 10 at head `b6a109520f41f0eeb218ba4df432e06dc8a774d8` is a non-authoritative Persistent Domain Stewardship proposal. It may clarify the distinction between product intent, physical binding, and cognitive runtime mechanics. Its package, profile, assignment, activation, facet, lineage, and control-plane designs are not accepted runtime requirements. Its references to `runtime-operator-visibility` do not grant that branch implementation authority.

The positive authority for this assessment is the current branch, active governance, canonical domain documentation, current code, and passing focused tests.

## Objective Baseline

The target is a maturity-appropriate vertical slice that can run Meld end to end. Completion means the supervisor starts from validated user intent, real domain actors repeatedly move one selected workspace tree through the canonical loop, and execution drives the existing docs writer task package to its real operational result. Every in-scope folder is processed through the existing fan-out strategy, each child final frame precedes preparation of its dependent parent, and the resulting `README.md` files are materialized through the existing workspace publication path. Durable records prove each semantic handoff and preserve progress between bounded turns, and observed aggregate output eventually changes goal satisfaction when useful work remains available.

In scope:

- One minimally expressed docs freshness stewardship package
- One selected workspace root or subtree
- One complete supervised convergence loop composed of bounded actor work
- Existing event, graph, belief, agent, planning, task network, capability, publication, evidence, and satisfaction contracts
- Honest runtime role state and health
- External runtime state under the governed data root
- Minimum foreground availability through work, quiescence, satisfaction, and later wake
- A deterministic integration proof that uses the same product assembly as a real run
- Exact execution through the existing `docs_writer` task package, traversal expansion, provider turns, and workspace publication behavior
- Filesystem proof of per-folder `README.md` outputs across a branching tree
- Foreground availability through quiescence, convergence, and later evidence
- Native domain emission and preserved per-tick runtime reports as the stable hook surface for later presentation tools

Out of scope:

- A generalized runtime configuration platform
- Daemon management and interprocess control
- Production restart matrices and crash hardening
- Observability platforms, status caches, invocation journals, and generalized action feeds
- Presentation surfaces such as dashboards and terminal visualizers that consume the runtime hooks
- Dynamic agents, generalized scheduling, and multi-network hosting
- Full sensory, causation, regime, multi-agent, and switching-cost behavior
- New reliability machinery that does not advance the semantic loop

## Executive Finding

Meld has enough durable domain capability to close the first flywheel slice without redesigning its architecture. The event ledger, graph replay, belief machinery, goal curation, satisfaction curation, planning actor, task network command boundary, workspace scan capability, docs writer path, and publication actor are real and independently tested.

The runtime itself is not semantically complete. Assembly and supervision are credible lifecycle foundations, but most registered roles have no actor. Missing actors are still started, leased, and reported healthy. The only cognitive actor currently wired by the product runtime is graph replay. The existing docs freshness integration test manually performs the semantic handoffs and synthesizes the work result, so it characterizes contracts rather than proving the product runtime.

The foreground command is also visually incomplete. By default it writes sparse tracing diagnostics to a file, emits no live ledger records, discards detailed worker activity after reducing it to health, and prints only a shutdown summary. Extracted crate instrumentation is especially sparse: `meld-events` reports exceptional warnings, `meld-execution` has one unrelated warning, and `meld-world-model` has no tracing dependency or emissions.

The architecture direction is appropriate. The closure work should connect existing domain contracts through thin root adapters, not move domain behavior into the supervisor or create a new platform layer.

## Canonical Flywheel Slice

Each traversal of the first product slice must preserve this semantic sequence:

```text
workspace observation
  -> durable event
  -> graph projection
  -> belief assessment
  -> goal curation
  -> active execution goal
  -> planning
  -> task network command
  -> task dispatch through real capability and workflow paths
  -> publication event
  -> outcome evidence ingestion
  -> satisfaction curation
  -> durable goal mutation
```

Every arrow is a domain contract or durable record. Root resolves configuration, assembles ports, and registers actors. The supervisor performs policy-neutral bounded ticks. Each actor discovers ready work through its own domain store, cursor, or selection contract. Root must not sequence semantic handoffs or become the source of truth for their meaning.

The sequence is not limited to one traversal. When satisfaction remains below threshold and useful work exists, planning must consume the revised durable world state and initiate another bounded attempt. Satisfaction closes the goal only after the configured threshold is crossed.

## Docs Writer Operational Parity Contract

Flywheel docs freshness must produce the same operational result as workflow docs freshness. It must select and execute the existing built-in `docs_writer` task package and `docs_writer_thread_v1` workflow route. A second flywheel-specific writer, simplified single-node task, or synthetic outcome substitute does not satisfy runtime completion.

The required behavior is:

```text
selected workspace root or subtree
  -> resolve its stable workspace node
  -> expand the existing bottom-up traversal
  -> fan out work across in-scope folder nodes
  -> complete each child final frame before preparing its dependent parent
  -> run the existing provider-backed docs writer turns
  -> publish each current generated frame through the existing workspace path
  -> create or update README.md in every actionable in-scope folder
  -> preserve unchanged reusable child outputs when existing policy selects reuse
  -> publish canonical execution outcomes for observation by the flywheel
```

The flywheel owns why work becomes useful, when the existing action is selected, how outcomes revise belief, and whether another bounded attempt is warranted. The existing task package, traversal expansion, workflow stages, provider route, frame publication policy, and workspace file writer own how documentation work is performed.

Bottom-up parity concerns generation dependencies rather than a total filesystem-write order. Sibling folders remain independent fan-out branches. Once a child final frame exists, child publication and parent preparation may both become eligible according to the existing graph.

Operational parity is proved from a branching fixture tree rather than a single folder. The proof must inspect the target filesystem and durable execution records, and compare the resulting README path set and deterministic contents with the existing workflow route over an equivalent fixture. Context frames, task artifacts, or a successful top-level command alone are insufficient evidence that every required `README.md` was materialized.

## Bounded Operational Units

Bounded convergence must preserve the semantic unit of one docs writer package run:

```text
actor tick
  -> bounded ready-invocation wave
  -> durable package progress
  -> later actor tick
  -> next bounded ready-invocation wave
  -> aggregate package completion
  -> outcome observation and satisfaction evaluation
```

One actor tick must not call the existing execute-to-completion helper for an unbounded tree. Runtime execution needs the bounded package-step contract, which limits ready capability invocations, preserves sibling fan-out and all compiled dependency edges, and persists executor, expansion, artifact, and completion progress between turns.

Multiple bounded turns do not imply multiple complete docs writer package runs. Satisfaction is evaluated from aggregate package completion. Replanning selects another complete package run only when the observed aggregate result remains unsatisfactory and distinct useful work exists.

## Bounded Convergence Contract

The runtime contract is:

```text
observe
  -> update world state
  -> evaluate goal
  -> select useful work
  -> execute bounded work
  -> observe outcome
  -> update world state again
  -> repeat
```

Cognitive work reaches a stable state when the goal is satisfied, no useful action is currently available, or an external failure prevents current progress. These are cognitive-loop states, not automatic foreground process termination.

Boundedness applies to each unit of work rather than to the number of flywheel turns:

- Each actor tick processes bounded input.
- Each event replay reads a bounded page.
- Each plan and dispatch attempt is bounded.
- Each provider invocation is bounded.
- Progress is persisted between turns.
- An active goal alone never permits an actor to hot-loop.

Convergence remains open-ended. Low confidence with useful available work causes another planning and action cycle. No useful action produces quiescence while the goal remains active. New evidence wakes eligible work from durable selection state. After satisfaction closes the goal, the foreground runtime remains available to observe future drift.

This workstream proves operational convergence before defining comprehensive failure policy. A failed unit of work must remain bounded, must not be represented as progress or satisfaction, must not close an active goal, and must not cause a hot loop. Firm retry schedules, backoff policy, failure classification, recovery matrices, and terminality rules are deferred.

## Foreground Availability

`meld runtime run` must remain alive while bounded package work is active, while the runtime is quiescent, and after satisfaction. New eligible evidence or later drift must wake the same foreground runtime without requiring process restart.

The foreground process must narrate the flywheel as it runs. A silent healthy runtime is an anti-pattern of the same class as false health. Each bounded tick emits a structured account of every domain report: actor, scope, input and output checkpoints, items attempted and committed, and issues. Domain crates emit native structured tracing at their semantic transition points, derived from the same domain records and reports that carry authoritative truth.

Emission primitives are runtime completion scope. The per-tick domain report is preserved rather than collapsed to health counts, and the emission surface is stable enough for later dashboards and terminal visualizers to consume without runtime redesign. A committed event follower, two-lane terminal presentation, log relocation, output failure matrix, and ordered final-watermark drain remain deferred presentation work.

Durable domain records and physical workspace outputs remain the authoritative completion evidence. Diagnostics do not become semantic truth.

## Stewardship Package And Runtime Boundary

PR 10 supports treating docs freshness as an operational domain theory without making PDS part of runtime completion.

The useful conceptual split is:

| Conceptual layer | Docs freshness first slice |
| --- | --- |
| Stewardship package | Declare what exists, what can be observed, what can be believed, what should be maintained, available actions, success measurement, authority, and governance |
| Minimal expression | Select docs freshness and the target scope that it stewards |
| Physical runtime binding | Resolve the concrete workspace, provider, capabilities, credentials, stores, and runtime placement needed to exercise the declared theory |
| Runtime composition | Derive owner-scoped registrations for the generic actors and passive services required to run the theory |
| Runtime state | Persist observations, beliefs, goals, plans, task attempts, evidence, cursors, leases, wake eligibility, and health outside the stewardship package |

The stewardship package contains no runtime state. It dictates what the domain means and what successful stewardship means. It does not name internal actors, prescribe tick order, allocate worker capacity, or enumerate runtime bindings.

These are conceptual seams, not accepted PDS records or public vocabulary. Runtime completion needs only a minimal docs freshness expression and generic internal seams that could later consume a stewardship package. It must not implement the proposed PDS control plane.

There is no product-slot count. The twelve current registry descriptors are an internal runtime inventory, not product capacity and not a declaration surface. Selecting docs freshness causes runtime composition to derive whatever actor and passive-service bindings the convergence loop requires. No user-facing or durable configuration represents unused slots.

Runtime completion should leave only inexpensive neutral hooks:

- A root-local registration identity separate from actor identity when existing session, subject, stream, and domain registration identities are insufficient
- Generic actors consuming owner-scoped registrations
- Domain-local registration identifiers
- Existing session, subject, stream, event, actor, and domain identities as the preferred durable correlation surface
- Optional root-local registration identity in diagnostic activity only
- Physical-binding-selected source and capability adapters
- Domain-owned outcome-to-evidence mapping
- Assembly composition from an explicit registration set, with the stewardship-derived set as one producer rather than the only entry point
- One public actor bounded-step contract implemented by every active actor handle
- Store and port opening scoped to the composed registration set
- Preserved per-tick domain reports as the runtime hook surface

The last four entries are the harness enablement hooks. They are what the gated agent-native debugger workstream composes. See [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md).

Runtime completion must not add a package compiler, profile language, persistent assignment or activation records, a facet lifecycle, standing stewardship objectives, stewardship episodes, package upgrades, full lineage, semantic diffs, or a stewardship projection.

## Domain Theory And Runtime Machinery Must Remain Separate

The current registry contains twelve internal role descriptors. A docs freshness stewardship expression declares domain intent. These are different concepts.

The descriptor count has no product meaning. The catalog may describe potential active actors and passive services, but runtime composition produces bindings only for required machinery. An absent descriptor receives no registration or lifecycle state. Unavailable status is reserved for a required binding that cannot be resolved. The existing registry must not become the stewardship package schema or its cardinality.

## Current Twelve-Role Registry

| Internal role | Current factory | Current default | Semantic status |
| --- | --- | --- | --- |
| `event.append` | diagnostics observer | enabled | Passive event capability is misrepresented as an active healthy actor |
| `event.replay` | none | enabled | No actor and falsely healthy |
| `world_model.graph_replay` | graph replay actor | enabled | Real cognitive actor with durable cursor behavior |
| `world_model.belief_assessment` | none | enabled | Domain capability exists but runtime actor binding is absent |
| `world_model.agent_goal_curation` | none | enabled | Domain runtime exists but runtime actor binding and selection are absent |
| `world_model.evidence_ingestion` | none | enabled | Mapping seam exists but durable consumer selection is absent |
| `world_model.satisfaction_curation` | none | enabled | Domain runtime exists but runtime actor binding and selection are absent |
| `execution.goal_set` | none | enabled | Durable command service exists and may not need its own actor |
| `execution.planning` | none | enabled | Planning actor exists but runtime binding is absent |
| `execution.task_network_command` | none | enabled | Durable command service exists and may not need its own actor |
| `execution.task_dispatch` | none | disabled | The bounded real dispatch actor is the largest missing capability |
| `execution.publication` | none | enabled | Publication actor exists but runtime binding is absent |

An empty enabled-role list currently expands to every role except task dispatch. The supervisor starts every enabled factory, immediately marks it healthy, and maps a missing tick report to healthy. This produces eleven enabled roles, two nonempty handles, one cognitive actor, and nine no-op handles reported as healthy.

This is the primary runtime truthfulness defect. An inert role must be explicitly disabled or visibly inert. It must not hold a lease or claim healthy execution.

## Domain Capability Map

| Flywheel responsibility | Existing capability | Product runtime state | Completion need |
| --- | --- | --- | --- |
| Event authority | Idempotent append, bounded replay, watermarks, consumer registry | Available through root ports | Use existing authority without a second ledger or cursor system |
| Graph projection | Bounded durable graph replay actor | Supervised | Keep as the proven actor pattern |
| Belief assessment | External family loading, evidence normalization, revision, planner view | Library complete enough | Bind a bounded actor and define the first subject source |
| Goal curation | Durable decision and command receipts | Library complete enough | Select pending belief deliveries across bounded turns |
| Goal query | Durable active goal store | Available in execution domain | Expose a thin query port to planning |
| Planning | Reads goals, projects state, lowers a plan, submits a task network command | Actor complete enough | Bind to the supervisor and to the available docs action declared by the stewardship package |
| Task network command | Durable passive command boundary | Complete enough | Keep passive unless evidence proves an actor is needed |
| Dispatch | Claim helpers and executor bridges | Partial | Build one bounded actor that claims, invokes real work, persists artifacts, and records outcome |
| Workspace scan | Real registered capability with typed artifacts and publication candidates | Independently tested | Route dispatch through it rather than synthesizing observation |
| Docs writing | Real workflow package with provider turns and README publication path | Independently tested | Route dispatch through it with a deterministic test provider |
| Publication | Bounded actor that emits durable outcome facts | Actor complete enough | Bind to supervisor |
| Evidence ingestion | World-model interpretation mapping over published aggregate outcomes | Complete | Aggregate payloads carry the package-declared semantic yield the installed mapping discriminates on |
| Satisfaction | Durable review, decision, and goal mutation | Library complete enough | Select changed beliefs across turns and mutate the goal only when satisfaction policy decides |

## Proof Truth

`tests/integration/docs_freshness_reopen_contract.rs` is valuable characterization evidence, but it is not an end-to-end runtime proof.

The test manually seeds graph state, inserts a low-confidence belief, calls goal curation, supplies a hand-built world state, calls the planner, ignores the plan result, injects a single-task mutation set, claims the task, writes a synthetic artifact containing fixed text, constructs a success outcome, invokes publication, drives evidence ingestion through a test-support replay helper preserved from the removed root port, and invokes satisfaction review.

It does not start product assembly and let supervised actors discover and perform those handoffs. It does not run workspace scan, the provider boundary, the docs writer workflow, or the real README write path.

The replacement proof must start validated configuration and product assembly, then permit only lifecycle driving such as bounded supervisor ticks. The test must not call semantic domain handoffs after startup. Durable records and the target workspace must show that one real docs writer package run advanced through multiple bounded turns and produced the same physical README result as the existing workflow route.

For docs freshness, the convergence proof must establish:

1. Planning selects the existing docs writer package for the selected workspace subtree.
2. The package expands the branching tree with sibling fan-out and child-finalization-before-parent-preparation dependencies.
3. Each actor tick advances only a bounded ready-invocation wave.
4. Package executor, expansion, artifact, and completion progress persist between turns and across reopen.
5. Every actionable directory receives its expected physical `README.md` through the existing workspace publication capability.
6. No satisfaction evidence becomes eligible before aggregate package completion.
7. Aggregate completion evidence closes the selected-tree goal when the configured success policy is met.
8. An unsatisfactory aggregate outcome permits replanning only when distinct useful work exists.
9. A no-progress case becomes quiescent without repeated work.
10. New evidence resumes a quiescent active goal.
11. The foreground runtime remains available after convergence to observe future drift.

The proof must demonstrate causal use of durable package state and aggregate outcome evidence. Repeating the package from scratch, duplicating a task, or accepting one child output as selected-tree completion does not prove convergence.

The CLI currently performs direct graph catch-up around command routing. A runtime proof must not rely on this hidden semantic work when claiming supervisor-owned execution.

## Architecture Assessment

### Appropriate foundations

- Root assembly opens stores, constructs ports, and binds actors without owning event, graph, belief, planning, task, or workspace meaning.
- The supervisor owns lifecycle, leases, heartbeats, bounded ticking, and shutdown.
- Durable event authority and consumer cursors already exist.
- Event authority exposes bounded replay and durable consumer foundations suitable for outcome evidence ingestion.
- Domain crates expose explicit runtime actors and command contracts.
- Runtime state is already rooted outside the target workspace through the product storage paths.

### Boundaries to preserve

- Runtime assembly may translate between explicit domain contracts, but it must not choose semantic policy.
- Supervisor health must describe actual actor execution, not registry presence.
- Passive command services must not be promoted into actors merely to fill a topology.
- Configuration must describe product intent, not internal worker sequencing.
- The target workspace may contain user content and resulting documentation, but locks, cursors, receipts, and other runtime state remain external.
- Diagnostics must not become satisfaction evidence or substitute for durable package progress.

### Boundary defect, corrected

`DocsTaskEvidenceReplayPort` — which replayed events, hardcoded docs evidence probabilities and source kind, constructed belief runtime state, and performed ingestion — was removed 2026-07-31; its mapper survives only as integration test support driving the reopen contract. The replacement direction is the implemented state: the world-model-owned mapping is installed theory selected by id under stage 2 of [Runtime Initialization](runtime_initialization.md), and docs outcome semantics originate in the stewardship package — including the package-declared semantic yield the completed interpretation discriminates on. Root adapts event transport and injects the selected typed mapping; it chooses no probabilities, source kinds, artifact meaning, or belief runtime policy.

## Configuration Assessment

The active runtime governance requires default user configuration through the XDG configuration home, explicit selection of workspace-local configuration, no working-directory-dependent discovery, external runtime state, pure loading and validation, and product-facing language.

The current base has three relevant gaps:

- Global config resolution uses `HOME` rather than honoring `XDG_CONFIG_HOME`.
- Workspace config is automatically merged from a supplied workspace root instead of requiring an explicit user choice.
- The validated config has no minimal docs freshness selection and target expression, so assembly silently derives an internal enabled-role set.

The first closure should add only the minimum docs freshness intent required for this slice. It must not generalize internal roles, delivery waves, test fixtures, or scheduling policy into public configuration.

The PDS proposal provides a useful conceptual check without defining the schema. The first physical config may combine the minimal docs freshness selection, target responsibility, and provider or workspace binding. Domain semantics remain in the stewardship package boundary, while physical resources and all runtime state remain runtime concerns. Future separation into package, profile, assignment, and activation must not require changing domain runtime contracts.

## Completion Blockers

The following are real semantic blockers rather than reliability enhancements:

1. Define the minimum configured docs freshness intent and its physical XDG boundary.
2. Define aggregate selected-tree satisfaction evidence and the content of the stage 4 genesis fact.
3. Make inert, disabled, passive, and active role state truthful.
4. Bind belief assessment and pending-delivery selection.
5. Bind goal curation and a thin active-goal query for planning.
6. Bind the existing planning actor.
7. Implement the bounded package-step contract, preserving the existing docs writer graph with progress persisted between ticks.
8. Implement bounded dispatch through real workspace scan, docs writer, provider, frame publication, and workspace write paths.
9. Align aggregate real package completion with the evidence contract without fabricating a synthetic result or accepting one child output as completion. Discharged 2026-07-31: the aggregate carries per-folder verified yield and a substantive-or-hollow summary computed from real artifact content, and the installed interpretation discriminates on it.
10. Bind publication, event-backed evidence selection, and satisfaction curation.
11. Replace manual orchestration proof with a supervisor-driven bounded package convergence proof over a branching fixture.
12. Prove quiescence, evidence-driven wakeup, and continued foreground availability without hot-looping.

## Explicit Deferrals

The following work does not block the first slice:

- Daemon mode and interprocess control
- Broad status and observability surfaces
- General runtime activation documents
- Production restart policy and crash matrices
- Firm retry schedules, backoff policy, failure classification, recovery matrices, and terminality rules
- Invocation journals and generalized fencing
- Dynamic agents and generalized scheduling
- Multiple concurrent stewardship expressions
- Full sensory, causation, regime, or multi-agent behavior
- Process-loss proof between every durable mutation and receipt
- PDS package compilation, profile authoring, assignment, activation, facet lifecycle, lineage, stewardship episodes, and unified projection
- A generalized diagnostic protocol, remote log transport, log rotation system, or observability platform
- Committed event following, two-lane terminal presentation, log relocation, sink failure matrices, and ordered final-watermark draining

## Assessment By Domain

Domain snapshot command:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

Snapshot:

```text
agent
api
branches
capability
cli
compat
concurrency
config
context
control
error
events
execution
heads
ignore
init
lib
logging
merkle_traversal
metadata
prompt_context
provider
runtime
session
store
task
telemetry
tree
types
views
workflow
workspace
world_state
```

| Domain | Needed Integration | Current Integration | Completeness | Evidence | Non Integration Rationale | Follow Up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `consume` | Supplies configured agent identity used by the docs writer path | `partial` | `src/agent.rs` and docs writer integration test |  | Keep identity input product-facing and avoid making agent config runtime topology |
| `api` | `none` | No runtime API is required for the first convergence proof | `not needed` | Current objective baseline | An API would add a second adapter surface without advancing the loop | `none` |
| `branches` | `none` | First proof uses the existing selected branch | `not needed` | Current workspace and storage contracts | Branch federation is unrelated to one local docs convergence loop | `none` |
| `capability` | `consume` | Real workspace scan capability is registered and returns typed artifacts | `complete` | `src/workflow/task_path.rs` and `tests/integration/workspace_scan_capability.rs` |  | Route dispatch through the registered capability contract |
| `cli` | `adapter` | `meld runtime run` exists but performs incomplete semantic work | `partial` | `src/cli/route.rs` and `src/runtime/tooling.rs` |  | Complete the public foreground command over the same product assembly used by proof and keep it available after convergence |
| `compat` | `none` | No compatibility migration is required | `not needed` | Current objective baseline | The slice adds no legacy replacement path | `none` |
| `concurrency` | `none` | Existing bounded supervisor coordination is sufficient | `not needed` | `src/runtime/supervisor/entrypoint.rs` | New shared concurrency policy would be reliability scope | `none` |
| `config` | `own` | Configuration loads existing product settings but has no minimal docs freshness selection and target expression and violates two active location rules | `partial` | `src/config.rs`, `src/config/sources/global_file.rs`, and `src/config/merge/service.rs` |  | Define and validate the minimum XDG docs freshness intent |
| `context` | `consume` | Dispatch resources expose context and workspace node storage | `partial` | `src/runtime/ports.rs` and `src/runtime/assembly.rs` |  | Supply only the context needed by the real workflow path |
| `control` | `none` | Control plans are not part of the docs freshness slice | `not needed` | Canonical cognitive architecture | A second orchestration authority would conflict with execution planning | `none` |
| `error` | `observe` | Runtime errors and health reports already have stable root contracts | `complete` | `src/runtime/contracts.rs` and supervisor tests |  | Preserve domain error context in actor reports |
| `events` | `own` | Durable append, replay, watermark, and consumer registry are complete | `complete` | `crates/meld-events` and focused event tests |  | Reuse the consumer registry for evidence selection |
| `execution` | `own` | Goals, planning, task network, dispatch helpers, and publication exist at different completion levels | `partial` | `crates/meld-execution` and focused actor tests |  | Bind planning and publication, then add one real bounded dispatcher |
| `heads` | `none` | Legacy head index behavior is not needed | `not needed` | Current objective baseline | Durable event and graph contracts already provide the required progress model | `none` |
| `ignore` | `consume` | Workspace scan already honors ignored-path policy | `complete` | Workspace scan capability tests |  | Preserve behavior through real dispatch |
| `init` | `adapter` | Existing initialization assets can support a referenced stewardship package if chosen | `partial` | `src/init.rs` |  | Implement stages 2 through 4 of [Runtime Initialization](runtime_initialization.md); explicit initialization owns bootstrap and activation never creates |
| `lib` | `adapter` | Public root exports expose assembly and runtime contracts | `partial` | `src/lib.rs` |  | Export only the selected product entrypoint |
| `logging` | `adapter` | Emission primitives and native domain tracing are in scope per Workstream Seven; diagnostics are never proof evidence | `partial` | Current objective baseline and Workstream Seven | Durable package state and physical README output remain authoritative | Presentation surfaces stay deferred |
| `merkle_traversal` | `consume` | Workspace scan and context paths use existing traversal behavior | `complete` | Current workspace scan capability |  | `none` |
| `metadata` | `consume` | Existing task and workspace paths carry metadata needed by real work | `complete` | Workspace scan and docs writer integration tests |  | Do not add runtime-owned metadata meaning |
| `prompt_context` | `consume` | Docs writer path builds prompt artifacts through existing contracts | `complete` | Docs writer task integration test |  | Route real dispatch through the existing path |
| `provider` | `consume` | Provider-backed docs writing works independently, but assembly exposes only availability flags | `partial` | `src/runtime/ports.rs` and docs writer task integration test |  | Bind a real provider execution context with deterministic test support |
| `runtime` | `own` | Assembly and supervision work, but semantic actor bindings and health truth are incomplete | `partial` | `src/runtime/assembly.rs` and `src/runtime/supervisor/entrypoint.rs` |  | Complete honest bindings without adding platform reliability scope |
| `session` | `observe` | Existing lifecycle records can identify a foreground run | `complete` | `src/session.rs` |  | Use only if needed by the selected public entrypoint |
| `store` | `consume` | Product storage paths keep durable runtime state outside the target workspace | `complete` | `src/config/workspace/storage_paths.rs` |  | Preserve external state root |
| `task` | `own` | Real docs writer package and artifact persistence are independently complete | `complete` | Docs writer task integration test |  | Invoke through dispatch without changing task semantics |
| `telemetry` | `none` | Semantic completion is proved by durable records; per-tick report preservation and emission are owned by runtime and domain crates under Workstream Seven, not by this domain | `not needed` | Current objective baseline | Durable domain records are the proof surface | `none` |
| `tree` | `none` | Tree behavior is internal to existing workspace and context capabilities | `not needed` | Current capability contracts | No new tree integration is required | `none` |
| `types` | `none` | No generic shared type layer is needed | `not needed` | Explicit domain contracts already exist | New shared runtime types would weaken domain ownership | `none` |
| `views` | `consume` | Existing planner and workspace views provide read models | `complete` | Planning actor and workspace scan tests |  | Keep views read-only and domain-owned |
| `workflow` | `adapter` | Registered task paths reach workspace scan and docs writer behavior outside product dispatch | `partial` | `src/workflow/task_path.rs` |  | Bind the stewardship-declared docs action to the existing workflow route |
| `workspace` | `own` | Real scan and README publication paths exist, but runtime does not invoke them | `partial` | Workspace scan and docs writer integration tests |  | Define the first target subject and run the real paths |
| `world_state` | `own` | Graph, belief, agent goal, and satisfaction capabilities exist, with only graph replay supervised | `partial` | `crates/meld-world-model` and `src/runtime/assembly.rs` |  | Bind bounded actors and explicit selection contracts |

## Evidence And Verification

The following focused suites passed on the assessed baseline:

- Runtime CLI integration tests
- Runtime assembly unit tests
- Supervisor entrypoint unit tests
- Event and world model crate tests
- Docs freshness contract tests
- Workspace scan capability tests
- Docs writer task integration test with four provider calls
- Planning actor test
- Publication actor test
- Task network execution bridge test
- Event subscription and follow tests
- Default file logging and verbose terminal logging tests

Passing tests show that the pieces are credible. They do not change the product-runtime conclusion because the current proof manually orchestrates the loop.

The completion proof must additionally establish:

- A branching selected workspace tree expands through the existing `docs_writer` package and workflow route.
- Every actionable in-scope folder receives its expected `README.md` through the existing workspace publication capability.
- Durable task dependencies prove every child final frame completed before preparation of its dependent parent.
- Existing fan-out, reusable-child, and idempotent expansion behavior remains intact.
- Execution outcomes identify the selected tree and per-folder work without the proof reattaching a fixture subject.
- One package run advances across multiple bounded runtime turns without losing compiled fan-out, dependency, expansion, artifact, or completion state.
- Actor progress persists before the next bounded package step.
- Aggregate package completion produces evidence only after all required directory generation and publication work completes.
- Replanning occurs only after an unsatisfactory aggregate result and only when distinct useful work remains.
- A satisfactory aggregate result crosses the satisfaction threshold and closes the goal.
- No useful action produces quiescence without repeated work.
- New evidence resumes eligible work for a quiescent active goal.
- The foreground runtime remains available after convergence for future drift.

## Requirements Decisions For The Next Session

The investigation has settled the initial runtime and stewardship boundaries. Completion includes the public `meld runtime run` path over product assembly, supported by a deterministic assembly-level proof. Durable correlation relies on existing session, stream, subject, event, actor, package-run, and domain registration identities. Run mode is a foreground convergence loop that remains active through quiescence and after satisfaction so later evidence or drift can make work eligible again.

Docs freshness supplies the operational domain theory with no runtime state. Meld consumes the minimal stewardship expression and derives the required internal runtime bindings. The current descriptor count has no product meaning, and neither configured nor durable unused slots exist.

Comprehensive failure policy is not a planning prerequisite for this workstream. The first slice requires bounded failed work, truthful non-progress, and freedom from hot-looping. It defers firm retry, backoff, classification, recovery, and terminality mechanics until the convergence loop is operational.

Recorded decisions:

1. Satisfaction and evidence granularity: per-folder evidence facts persist on the ledger, and one derived aggregate belief computed by belief policy over those facts carries the selected-tree question. Per-folder belief keys remain a later addition, not a migration.
2. Stage 4 genesis fact content: an idempotent unobserved-scope declaration for the selected subtree. The exact record identity freezes at Gate B.
3. CLI path targeting: `meld runtime run` takes the target path as its explicit default argument per the CLI targeting policy. Configuration resolves only through the XDG config home, and no runtime state is written under the target workspace per the storage policy. A config-forcing `meld workspace track` flow is noted as a possible later refinement outside this slice.
4. Per-tick report preservation is durable: the existing typed action-record schema is wired rather than a streamed hook, which may layer over the durable record later.
5. Future-drift recording is reopen-in-place: the same goal identity transitions from satisfied to active through an idempotent Agent-curated reopen mutation appended as a new goal revision with the lifecycle epoch advanced and provenance citing the triggering belief revision. Storage remains append-only; identity is stable over the revision stream. Satisfaction evidence binds the epoch it was produced under, and no consumer may treat the satisfied state as absorbing — the epoch fence and revision reads enforce this, and it is a named verification invariant.

Open product decisions: none. The requirements gate closed 2026-07-24.

Resolved by structure or prior selection:

- Provider requirement: deterministic local provider in proof with the same real docs writer route used by configured runs.
- Artifact semantics: resolved by [Runtime Initialization](runtime_initialization.md) stage 2 — the world-model-owned mapping is installed theory selected by id from the stewardship expression.
- Bootstrap ownership: resolved by [Runtime Initialization](runtime_initialization.md) structure — explicit initialization owns theory installation, identity genesis, and epistemic seeding; the runtime owns activation, which never creates semantic state.

These are product requirements, not missing platform mechanisms. With all decisions recorded, they are sufficient to produce a small implementation plan with clear domain ownership and acceptance tests.

## Strategy Continuity Constraints

The Strategy corpus under `design/cognitive_architecture/world_model/strategy` is authoritative for the downstream shape these decisions must not foreclose. The constraints below bound the decisions above without expanding this slice.

1. Evidence facts persist per folder on the ledger. Aggregation into selected-tree satisfaction is belief or satisfaction policy over those facts, never a substitution for them. Directive grounding later instantiates per-folder questions over the same evidence stream.
2. Later drift advances the goal lifecycle epoch. The reactivation rule aligns with the epoch semantics that Strategy admission later formalizes and that dispatch claim fencing can extend.
3. The available docs action binding freezes affordance-shaped: action identity, artifact meaning, outcome contract reference, and realization route. It may later publish as a semantic action affordance without rework.
4. The curation-to-goal-set binding passes through one named port. The later Goal draft gate and admission bundle insert at that port without rewiring curation or the goal set.
5. Committed task-network dependency edges record their origin so the later semantic-versus-scheduling separation requires no migration.
6. The bounded package-step contract is defined at the dispatch boundary and is domain-neutral: bounded ready-wave budget, durable readiness, expansion, artifact, and completion state, and reopen-resume semantics. The existing package executor is its first implementor and is compatibility-scoped. Task-network composition graphs are its second consumer.
