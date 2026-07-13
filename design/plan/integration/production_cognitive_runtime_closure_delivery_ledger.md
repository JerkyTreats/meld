# Production Cognitive Runtime Closure Delivery Ledger

Date: 2026-07-12
Status: implementation in progress
Program: [Production Cognitive Runtime Closure Program](production_cognitive_runtime_closure_program.md)
Base branch: `event-foundation-closeout`
Integration branch: `production-cognitive-runtime-closure`
Thread ceiling: eight

## Purpose

This ledger converts the program waves into schedulable delivery packets.
It is the implementation control plane for `phased-program-delivery`.
The parent program remains authoritative for architecture, wave outcomes, completion gates, and deferred scope.

Only the root integrator changes packet state, accepted commit ids, wave completion, and next-ready state.

## State Vocabulary

| State | Meaning |
| --- | --- |
| ready | dependencies are satisfied and the packet may be assigned |
| active | one named worker owns the packet in one worktree |
| review | focused gates passed and a fresh reviewer owns the packet |
| accepted | reviewed commit integrated and serial gates passed |
| blocked | a named dependency or finding prevents work |
| superseded | a later packet replaces the scope before implementation |

At program start only W0A is ready.
All other packets are blocked by their listed dependency.

## Thread And Worktree Contract

Thread zero is always the root integrator.
Threads one through five are normal implementation lanes.
Thread six is the test or documentation lane.
Thread seven is held for dependency repair and fresh review.

Worktrees use `../meld-w<packet>-<lane>` and branches use `delivery/<packet>-<lane>`.
A builder retires its thread after focused evidence, a reviewed provisional commit, and a clean handoff.
The released thread may then host a fresh reviewer that did not author the packet.

## Central Ownership

Only thread zero edits:

- root manifests
- `src/runtime/contracts.rs`
- `src/runtime/assembly.rs`
- `src/runtime/ports.rs`
- `src/runtime/storage.rs`
- `src/runtime.rs`
- `src/cli/parse.rs`
- `src/bin/meld.rs`
- crate public export files
- integration test registries
- shared activation, process, IPC, and wire schemas
- compatibility wrappers and migration entrypoints
- this ledger and parent program state

Builders may add exclusive new child modules and focused test files named in their packet.
They send exact central-file amendments to thread zero and do not edit central files in their worktree.
Thread zero registers every new integration test module.

The scope phrases in packet tables are planning boundaries, not directory-wide write grants.
Before a blocked packet becomes ready, thread zero appends an `Allowed Files` record with every exact existing path and every exclusive new-file namespace.
No builder starts from a directory-only scope.
Any newly discovered file outside that record requires a central scope amendment before it changes.

## Packet Report

Every handoff reports:

- packet id, worker, branch, and worktree
- exact files changed and unexpected dirty files
- dependency and contract commit ids
- focused commands and results
- provisional commit id
- compatibility or migration evidence
- unresolved risks and requested central amendments
- reviewer findings and dispositions
- next packets that become ready after acceptance

## Wave 0 Delivery

### W0A Truth Gate

Initial state: ready
Current state: complete
Thread: zero with thread six as documentation analyst
Strength: highest available for reconciliation, strong for mechanical evidence
Write scope: parent program, this ledger, `design/plan/README.md`, runtime visibility ledger, affected current assessments
Expected commit: `design(plan): reconcile production runtime source truth`

Required output:

- classify every cited plan as current, partially applicable, superseded, or historical
- name the controlling requirement for every Wave 0 packet
- reconcile the plan index before source fanout
- identify canonical architecture text that contains implementation status
- leave canonical intent changes for a separately reviewed design amendment

Gate: plan links, Markdown policy, diff check, and fresh source-coverage review.

Completion evidence: source classification, canonical hygiene inventory, plan index reconciliation, Markdown checks, boundary scan, and fresh review passed.

### W0A Active Assignment

Program branch: `production-cognitive-runtime-closure`
Root owner: thread zero
Analyst lanes: source classification and runtime contract baseline
Strength selector: unavailable in the active orchestration surface
Strength handling: packet scopes are narrowed by reasoning risk and record the intended relative strength
Started from checkpoint: `1f6dc1d`

### W0A Source Classification

| Source | Classification | Controlling Use |
| --- | --- | --- |
| parent closure program | current | architecture, outcomes, gates, precedence |
| delivery ledger | current | packet state and execution evidence |
| cognitive architecture index and domain roots | partially applicable | declarative ownership and invariants only, hygiene candidates tracked below |
| runtime requirements index | partially applicable | shared ownership rules, parent controls order |
| supervisor runtime requirements | partially applicable | lifecycle baseline, parent controls ids, roles, status, and order |
| world model runtime requirements | partially applicable | actor ownership, parent controls ids and activation order |
| execution runtime requirements | partially applicable | actor ownership and effects, parent controls activation order |
| event runtime requirements | partially applicable | foundation truth plus downstream hosting, older order superseded |
| physical configuration requirements | partially applicable | Wave 2 configuration and bootstrap ownership |
| runtime visibility ledger | partially applicable | current cache and process scope, parent controls sequencing |
| runtime assembly ledger | historical | closed evidence, not current acceptance |
| plan index and dated assessments | partially applicable | discovery only until reconciled against code and parent |

### W0A Controlling Requirements

| Packet | Controlling Contract |
| --- | --- |
| W0B | parent runtime taxonomy, canonical id registry, alias migration, cache bounds, contender truth, and health eligibility |
| W0C1 | parent bounded cache persistence, tolerant reads, lifecycle publication, and truncation rules |
| W0C2 | parent passive early status route with no `RunContext` or product database open |
| W0C3 | parent separate-process lock-isolation proof |
| W0C4 | parent evidence, source freshness, canonical hygiene, and plan reconciliation gates |
| W0D | parent full ladder, fresh reviews, commit gate, and truthful closeout |

### W0A Canonical Hygiene Inventory

Immediate corrections remove current implementation assessment from the world model and sensory domain roots.
W0C4 rewrote the following confirmed historical or implementation-shaped documents as declarative contracts:

- `design/cognitive_architecture/meld-lang/PLAN.md`
- `design/cognitive_architecture/execution/GAPS.md`
- implementation-status preface in `design/cognitive_architecture/world_model/planner/spec.md`
- landed-slice wording in `design/cognitive_architecture/world_model/belief/README.md`
- implementation location wording in `design/cognitive_architecture/execution/task_network.md`
- implementation comparison wording in `design/cognitive_architecture/meld-lang/README.md`
- current implementation wording in `design/cognitive_architecture/world_model/graph/README.md`
- current capability wording in `design/cognitive_architecture/world_model/agent/README.md`
- current public-surface wording in `design/cognitive_architecture/world_model/public_interface.md`
- implementation sequencing in `design/cognitive_architecture/world_model/belief/curation.md`
- current implementation wording in `design/cognitive_architecture/execution/planning/planning_pipeline.md`

W0C4 also adjudicated every static candidate returned by this scan:

```sh
rg -l "implemented|implementation status|landed|current implementation|not yet implemented|already exists|exists today|current code|currently implemented|first slice" design/cognitive_architecture -g '*.md'
```

The adjudicated candidate set included:

- `design/cognitive_architecture/core/CRATE.md`
- `design/cognitive_architecture/execution/examples/bayesian_evaluation.md`
- `design/cognitive_architecture/execution/goals/README.md`
- `design/cognitive_architecture/execution/planning/htn/README.md`
- `design/cognitive_architecture/execution/planning/htn/lineage_model.md`
- `design/cognitive_architecture/execution/research/htn_turing.md`
- `design/cognitive_architecture/meld-lang/primitives.md`
- `design/cognitive_architecture/meld-lang/requirements.md`
- `design/cognitive_architecture/meld-lang/world_state.md`
- `design/cognitive_architecture/world_model/agent/genesis_and_activation.md`
- `design/cognitive_architecture/world_model/agent/goal_curation.md`
- `design/cognitive_architecture/world_model/agent/runtime_surface.md`
- `design/cognitive_architecture/world_model/belief/belief_families.md`
- `design/cognitive_architecture/world_model/belief/comparator_model.md`
- `design/cognitive_architecture/world_model/belief/fact_to_belief.md`
- `design/cognitive_architecture/world_model/belief/microarchitecture.md`
- `design/cognitive_architecture/world_model/belief/requirements.md`
- `design/cognitive_architecture/world_model/belief/substrate.md`
- `design/cognitive_architecture/world_model/causation/README.md`
- `design/cognitive_architecture/world_model/causation/requirements.md`
- `design/cognitive_architecture/world_model/planner/README.md`
- `design/cognitive_architecture/world_model/regime/README.md`

### W0B Contract Inputs Frozen By W0A

- current registry has 12 ids, 11 default-enabled roles, nine enabled inert handles, one diagnostics-only passive observer, one concrete graph worker, and one disabled inert role
- assembly and supervisor validators disagree on uppercase acceptance
- supervisor persistence embeds runtime ids across desired state, leases, heartbeats, health, restarts, and lifecycle records
- alias migration must characterize every tree, reject collisions, accept legacy ids only at ingress, and emit canonical ids
- contender health keyed only by runtime id can overwrite active-owner truth
- cache schema traits exist without a concrete filesystem host, lifecycle invocation, bounds enforcement, or passive route
- W0B must freeze identity and migration before cache and route fanout

## Wave Execution Log

### 2026-07-12 Program Start

Ready items: W0A only
Parallelization: two read-only analysts beside root reconciliation
Workers: `w0a_source_truth` and `w0a_runtime_baseline`
Commits accepted: `642af1c`
Gate state: W0A checks and fresh review passed
Next ready set: W0B after the W0A commit gate

### 2026-07-12 W0B Contract Freeze

Ready items: W0B only
Parallelization: root-owned contract implementation with one fresh cache and compatibility reviewer
Implementation state: review
Contract evidence:

- twelve canonical persisted runtime ids with ingress-only requirement aliases
- one lowercase runtime id validator shared by assembly and supervisor
- actor, passive service, and port-only role classes
- concrete, inert, unavailable, and older-data implementation states with explicit desired-disabled presentation
- all roles visible in desired state with only graph replay enabled by default
- frozen cache paths, schema compatibility, byte and count limits, action envelopes, truncation metadata, staleness, and atomic replacement steps
- active-owner lease identity exposed in status
- heartbeat and health accepted for presentation only when they match the active lease
- a losing supervisor contender fails startup without overwriting owner health or heartbeat

Focused evidence:

- `cargo test --locked runtime::contracts --lib`
- `cargo test --locked runtime::assembly --lib`
- `cargo test --locked runtime::supervisor --lib`
- `cargo test --locked --test integration_tests runtime_cli`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`

Commit state: accepted as `0819dbf`
Next ready set: W0C1 through W0C4

### 2026-07-12 W0C Runtime Visibility Core

Ready items: W0C1 through W0C4
Parallelization: isolated cache-host worktree beside root-owned passive route and lifecycle integration
Cache commits: `4823303`, `b6dd724`
Integrated state: accepted as `fcac040`
Implementation evidence:

- exclusive bounded cache writer with atomic full-file replacement
- tolerant bounded reader for missing, partial, old, malformed, oversized, and future data
- normalized action bounds and visible retention truncation
- passive status description and truthful desired-state fallback
- early binary route before `RunContext`
- startup, tick action, and shutdown cache publication
- monotonic per-instance action identities
- real separate-process status read while the foreground host owns product databases

Focused evidence:

- `cargo test --locked runtime::status_cache --lib`
- `cargo test --locked runtime::passive_status --lib`
- `cargo test --locked runtime::supervisor --lib`
- `cargo test --locked --test integration_tests runtime_cli`
- `cargo test --locked --test integration_tests runtime_status_process_isolation`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`

Review state: cache host and root integration approved after fix loops with no remaining high or critical findings
Next ready set: W0C4 documentation evidence, then W0D closeout

### 2026-07-12 W0D Integrated Closeout

Ready items: W0C4 and W0D
Accepted commits: `355f244`, `7d5e209`, `00e0b23`
Implementation evidence:

- canonical architecture status leaks replaced with declarative contracts and an explicit adjudication register
- restart policy transitions routed through a first-class cache publication seam
- requirement-era runtime ids canonicalized at ingress
- resumable supervisor-store migration across desired state, leases, active lease indexes, heartbeats, health, restarts, and lifecycle values
- divergent alias and canonical records fail closed during migration
- assembly reopen checks tolerate the documented delayed sled lock release

Final gate evidence:

- `cargo fmt --all -- --check`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --all-targets`
- `git diff --check`
- no `mod.rs` paths
- exhaustive canonical hygiene scan with every residual match adjudicated

Fresh integrated review found two high issues: restart publication was not explicit and persisted alias migration lacked complete characterization.
Commit `00e0b23` closed both findings, and the reviewer approved the corrected integrated state with no remaining high or critical blockers.

Closeout state: accepted
Next ready set: W1A authority contract freeze

### 2026-07-12 W1A Authority Contract Freeze

Ready items: W1A
Parallelization: root-owned cross-domain contract integration beside an isolated world-model contract lane
Contract scope:

- belief authority migration identity, marker state, reconciliation, and legacy posture
- identity-bearing evidence consumer cursor and durable receipt disposition
- subscription and assessment lease compare-and-swap intent
- conflict-aware goal request identity and flushed commit receipt
- deterministic projection frame and planning request identity inputs
- semantic outcome lineage preserving the canonical task outcome
- typed event ingress validation, shared fence state, and final identity-bearing barrier
- enforced restart eligibility and durable replacement ordering

Implementation state: accepted
Accepted commits: `7d01ba2`, `23d0c92`, `80ae62e`, `a9033eb`, `d9f8a43`
Gate evidence: focused contract suites, locked workspace check, locked workspace clippy with warnings denied, formatting, and diff validation passed
Review evidence: three independent correction loops closed every critical and high finding
Next ready set: W1B1 through W1B4

## Gate Evidence

| Wave | Gate | Result | Commit | Notes |
| --- | --- | --- | --- | --- |
| baseline | full locked workspace ladder | passed | `1f6dc1d` | implementation-ready checkpoint before program branch |
| W0A | source truth and documentation checks | passed | `642af1c` | fresh reviewer withdrew all findings |
| W0B | runtime contract focused ladder and workspace clippy | passed | `0819dbf` | fresh review passed after two fix loops |
| W0C1 | bounded cache host | passed | `4823303`, `b6dd724` | fresh review passed after ingress-bound fix loop |
| W0C2 | passive route and lifecycle publication | passed | `fcac040` | fresh review passed after action identity fix |
| W0C3 | separate-process lock isolation | passed | `fcac040` | foreground host remained active while status succeeded |
| W0C4 | canonical document truth | passed | `355f244` | all candidates and retained matches adjudicated |
| W0D | integrated Wave 0 ladder | passed | `00e0b23` | full locked workspace gate and fresh integrated review passed |
| W1A | authority recovery contract freeze | passed | `7d01ba2`, `23d0c92`, `80ae62e`, `a9033eb`, `d9f8a43` | focused suites and workspace static gates passed after three review loops |

## Review Findings

W0A findings for incomplete hygiene inventory and missing plan-index ledgers were fixed and withdrawn by the fresh reviewer.

W0B fresh review found false worker eligibility for passive and port-only roles, contender mutation before lease conflict, hidden terminal health, incomplete tolerant-reader outcomes, ambiguous availability presentation, bypassable registry invariants, duplicate instance overwrite, and a startup rollback gap.
Both fix loops closed every high finding.
The final reviewer approved W0B for commit and W0C fanout with no remaining high or critical findings.

W0C1 fresh review found arbitrary action ingress could bypass bounds, custom test limits were public, and action retention was silent.
The cache fix commit normalized every ingress, made custom limits test-only, and carried retention warnings into the next atomic snapshot.
Final cache review passed.

W0C2 fresh review found same-millisecond action identity collisions.
A supervisor-owned monotonic action sequence and focused regression closed the finding.
Final route and lifecycle review passed.

W0D fresh review found no explicit restart publication seam and incomplete persisted-id migration evidence.
The closeout fix added distinct restart snapshot publication and a resumable collision-detecting migration for every supervisor record family that stores a runtime id.
The final review approved the integrated Wave 0 state with no high or critical findings.

W1A review first found missing parity proof, belief commit intent, complete goal and planning identities, event identifier grammar, final barrier coherence, and replacement ordering.
The second loop found freely mismatched parity, incomplete commit invariants, arbitrary projection frame identity, unchecked lease and subscription transitions, and replacement acquisition before restart eligibility.
The final loop found malformed migration states, projection identity detached from the canonical request, incomplete assessment lease validity, and inaccessible commit recovery products.
Commits `80ae62e`, `a9033eb`, and `d9f8a43` closed the findings.
The final reviewer approved W1A with no remaining critical or high blockers.

## Phase Completion Matrix

| Packet | Status | Implementation Evidence | Test Evidence | Review |
| --- | --- | --- | --- | --- |
| W0A | complete | source classification and reconciled docs | Markdown, links, diff, boundaries | passed after fix loop |
| W0B | accepted | canonical identity, truthful roles, cache freeze, contender safety | focused runtime suites and workspace clippy | passed after two fix loops |
| W0C1 | accepted | cache host | 15 focused tests | passed after fix loop |
| W0C2 | accepted | passive status route and lifecycle publisher | focused passive and runtime CLI suites | passed after fix loop |
| W0C3 | accepted | process isolation proof | separate-process test | passed |
| W0C4 | accepted | canonical hygiene register and current visibility truth | documentation scope, links, prose, scans, and diff | passed |
| W0D | accepted | restart publication, alias migration, and integrated closeout | full locked workspace ladder | passed after fix loop |
| W1A | accepted | authority, replay, identity, validation, restart, and shutdown contracts | focused contract suites and full workspace static ladder | passed after three fix loops |
| W1B1 | accepted | canonical belief authority, fenced migration, atomic recovery, and gap-free evidence cursor | 180 world-model tests and strict clippy | passed after durability and concurrency review loops |
| W1B2 | accepted | payload-derived goal identity, durable receipt visibility, monotonic updates, and lifecycle guards | 53 goal tests, 17 doc tests, full execution suite, and strict clippy | passed after two review loops |
| W1B3 | accepted | deterministic planning identity, typed subject lowering, and attributed outcome publication | full execution all-target tests and clippy | passed after urgency fix loop |
| W1B4 | accepted | typed append validation, shared ingress fence, retryable drain, and final durable barrier | all-feature event matrix and workspace static ladder | passed after drain-retry fix loop |
| W1B5 | active | integrated Wave 1 fault and parity gate | pending | pending |
| W1C | ready | supervisor and root cutover dependencies are accepted | pending | pending |
| W2A through W6G | blocked | none | none | none |

## Risks And Exceptions

- The orchestration surface does not expose model selection, so relative strength is enforced through packet scope and review depth.
- A separate dirty T3 worktree remains outside this program and must not be reused or modified.

### W0B Runtime Contract Freeze

Initial state: blocked by W0A
Current state: accepted
Thread: zero
Strength: highest available
Write scope: central runtime contracts, assembly registry, supervisor contract adapter, compatibility tests
Expected commit: `fix(runtime): classify canonical runtime identities and roles`

Required output:

- canonical runtime id registry and ingress-only aliases
- role class and implementation state
- cache schema, bounds, path, atomic write protocol, and staleness
- contender truth and health eligibility
- characterization and parity for persisted ids

Gate: runtime contract tests, supervisor characterization, compatibility review, and boundary scan.

Accepted commit: `0819dbf`

### W0C Parallel Implementation

Initial state: blocked by W0B

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W0C1 cache host | 1 | strong | new `src/runtime/status_cache.rs`, new focused unit tests | `feat(runtime): persist bounded status cache snapshots` |
| W0C2 passive route helper | 2 | strong | new `src/runtime/passive_status.rs`, focused helper tests | `feat(runtime): read status through passive configuration` |
| W0C3 process isolation proof | 3 | strong | new `tests/integration/runtime_status_process_isolation.rs` | `test(runtime): prove status process isolation` |
| W0C4 documentation evidence | 6 | standard | assessments named by W0A only | `design(plan): record runtime truth gate evidence` |

Thread zero integrates route changes in `src/bin/meld.rs`, CLI parsing, runtime assembly, and test registration after builder commits are reviewed.

Gate: focused tests, root integration, full wave ladder, two fresh review lenses, and accepted documentation evidence.

### W0C Allowed Files

W0C1 cache host:

- new `src/runtime/status_cache.rs`
- focused unit tests inside the new module

W0C2 passive route helper:

- new `src/runtime/passive_status.rs`
- focused unit tests inside the new module

Thread zero exclusively owns registration in `src/runtime.rs`, CLI routing, shared contracts, and any integration test registration.
W0C3 and W0C4 receive exact allowed files only after W0C1 and W0C2 expose their reviewed seams.

### W0D Wave Closeout

Initial state: accepted
Thread: zero, then thread seven reviewer
Strength: highest available
Output: accepted commits, exact gate evidence, updated risks, and W1A ready.

Accepted commits: `355f244`, `7d5e209`, `00e0b23`
Gate result: passed

## Wave 1 Delivery

### W1A Authority Contract Freeze

Initial state: ready
Current state: accepted
Thread: zero
Strength: highest available
Write scope: central compatibility contracts and public domain amendment integration
Expected commit: `fix(runtime): freeze cognitive authority recovery contracts`

Freeze belief migration, evidence cursor identity, goal request hashes, projection frame identity, planning inputs, object lineage, event fence, contender state, and shutdown ordering.

Accepted commits: `7d01ba2`, `23d0c92`, `80ae62e`, `a9033eb`, `d9f8a43`
Gate result: passed
Next ready set: W1B1 through W1B4

### W1B Domain Correctness Batch

Initial state: ready after accepted W1A

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W1B1 belief authority | 1 | highest available | `crates/meld-world-model/src/belief/` and focused belief tests | `fix(belief): reconcile product belief authority` |
| W1B2 goal durability | 2 | highest available | `crates/meld-execution/src/goals/` and goal tests | `fix(execution): reject divergent goal command replay` |
| W1B3 planning and lineage | 3 | highest available | planning lowering, outcome lineage, focused execution tests | `fix(planning): preserve projection and object lineage` |
| W1B4 event validation | 4 | highest available | event authority validation and writer fence modules | `fix(events): validate fenced append ingress` |
| W1B5 fault harness | 5 | strong | new Wave 1 crash, parity, and concurrency tests | `test(runtime): prove authority recovery boundaries` |

Thread six prepares documentation evidence without changing program state.
Threads retire after focused review.

W1B3 accepted commit: `8f464cd`

W1B3 gate evidence:

- complete planning identity binds goal update sequence, exact projection request, derived frame, method library, capability catalog, and planning version
- bounded planning selects ascending numeric urgency and then stable goal id
- full typed object coordinates survive lowering in task lineage
- canonical lowered tasks reject legacy unattributed outcomes
- attributed outcomes survive task-network reopen and retain semantic publication lineage
- full `meld-execution` all-target tests passed
- focused planning, lowering, task-network store, execution bridge, and publication bridge suites passed
- all-target clippy with warnings denied passed
- fresh review approved after one urgency-order fix loop

W1B4 accepted commit: `9a425f4`

W1B4 gate evidence:

- idempotent append rejects missing record identity before admission
- malformed envelope identifiers, duplicate objects, and undeclared relation endpoints return typed validation errors
- every append capability clone shares one generation-bearing ingress fence
- final drain rejects new admission, flushes accepted work, and binds the closed fence to the durable watermark
- transient flush failure retains Draining and a later close retries writer-owned pending durability
- raw store activity cannot advance the writer watermark or satisfy the final barrier
- all-feature event library, authority, recovery, migration, remote, observability, and benchmark targets passed
- locked workspace check and clippy with warnings denied passed
- full workspace testing exposed and corrected one malformed workspace snapshot relation and one stale idempotent test
- fresh review approved after one drain failure-state fix loop

W1B2 accepted commit: `fffdb9a`

W1B2 gate evidence:

- complete canonical command payload derives request identity and divergent command reuse rejects
- low-level mutation methods are sealed behind the durable goal API
- every public persistent receipt observation flushes before return
- unique mutations require a strictly newer update sequence
- modify commands preserve lifecycle and lifecycle commands enforce legal transitions
- active goal ordering uses ascending numeric urgency and stable goal id
- in-memory and persistent stores pass collision, stale sequence, lifecycle, concurrency, and reopen parity
- conservative legacy reconstruction rejects intent that old durable data cannot prove
- 53 focused goal tests and 17 documentation tests passed
- full `meld-execution` all-target tests and strict clippy passed
- fresh review approved after boundary, lifecycle, parity-order, and receipt-visibility fixes

W1B1 accepted commit: `8475ffb`

W1B1 gate evidence:

- product belief storage is the canonical authority after fenced resumable migration
- legal marker successor transitions and exact retries preserve durable cutover truth
- independent handles share one durable store identity and one drain admission gate
- source fence, parity snapshot, empty-source cutover, and assignment-generation metadata are durable and migration-visible
- lease, active index, dirty state, assignment, and mutation generation update transactionally
- current view and subject index repair transactionally and reopen detects missing indexes
- evidence cursor advances one canonical sequence per disposition and exact receipt replay remains idempotent
- post-acquire evidence is excluded from the current lease and included exactly once by the rescheduled generation
- recovery excludes evidence committed across the complete revision history
- indeterminate fence, marker, receipt, and prepared-stage flushes are retried and survive reopen
- all 180 world-model tests, strict clippy, formatting, diff, and no-mod-rs checks passed
- final fresh review reported no critical, high, or medium findings

### W1C Supervisor And Root Cutover

Initial state: blocked by W1B1 through W1B4

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W1C1 supervisor safety | 1 | highest available | `src/runtime/supervisor/` excluding central entry integration | `fix(runtime): enforce restart and contender safety` |
| W1C2 compatibility proof | 2 | strong | belief cutover, legacy goal, and runtime id parity tests | `test(runtime): prove compatibility cutover parity` |

Thread zero integrates root belief cutover, projection adapters, event fence wiring, supervisor entrypoint changes, and public exports.

Gate: crash matrix, repeated serial concurrency, full ladder, three fresh review lenses, and W2A ready.

## Wave 2 Delivery

### W2A Activation Contract Freeze

Initial state: blocked by Wave 1 closeout
Thread: zero
Strength: highest available
Expected commit: `feat(runtime): define typed product activation packages`

Freeze TOML schema, explicit path policy, normalized hash, size and field validation, owner packages, bootstrap identity, execution validation receipt, and legacy directive migration.

### W2B Domain Activation Batch

Initial state: blocked by W2A

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W2B1 world model bootstrap | 1 | highest available | agent and belief activation child modules plus focused tests | `feat(agent): bootstrap configured seed directive` |
| W2B2 execution validation | 2 | highest available | method, package, artifact, network, and publication validation modules | `feat(execution): validate docs freshness activation` |
| W2B3 activation proof | 3 | strong | checked-in activation fixture and new activation integration test | `test(runtime): prove activation replay and conflict handling` |

Thread zero lands the loader, central schemas, CLI adapter, assembly factory, exports, and test registration.

Gate: reopen after every bootstrap stage, divergent-config proof, compatibility parity, two fresh reviews, and W3A ready.

## Wave 3 Delivery

### W3A Semantic Contract And Authority Freeze

Initial state: blocked by Wave 2 closeout
Threads: zero and one
Strength: highest available
Expected commits: shared semantic lifecycle contracts, then execution task-network authority

Thread one owns new execution authority child modules and focused authority tests.
Thread zero owns public exports, root lifecycle contracts, and assembly registry integration.
The authority commit must pass serialization, poison, bounded admission, durable acknowledgement, and reopen tests before actor fanout.

### W3B Semantic Actor Batch

Initial state: blocked by W3A

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W3B1 belief and evidence | 1 | highest available | belief assessment and event-ingestion child modules | `feat(belief): run bounded evidence and assessment actors` |
| W3B2 agent curation | 2 | strong | agent selection, curation, and satisfaction child modules | `feat(agent): run bounded curation actors` |
| W3B3 projection and planning | 3 | highest available | world-model projection and execution planning child modules | `feat(planning): submit plans through execution authority` |
| W3B4 publication | 4 | strong | execution publication child modules and tests | `feat(execution): publish outcomes through authority ports` |
| W3B5 actor fault proof | 5 | highest available | stale-writer, sink receipt, cursor, reopen, and order tests | `test(runtime): prove semantic actor recovery` |

Graph replay compatibility is part of W3B1.
Thread zero integrates root wrappers, native report adapters, central factories, exports, and test registration.

Gate: one concrete actor per enabled role, domain-owned stale-writer fences, full actor transition proof, six fresh review lenses, and W4A ready.

## Wave 4 Delivery

### W4A Dispatch Contract Freeze

Initial state: blocked by Wave 3 closeout
Thread: zero
Strength: highest available
Expected commit: `feat(execution): define restart-safe invocation recovery`

Freeze claim leases, legacy resolution, task journal, provider result capture, canonical request hashes, capability recovery policy, artifact replay, cross-store reconciliation, cancellation, and safe-point contracts.

### W4B Persistence Batch

Initial state: blocked by W4A

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W4B1 claim recovery | 1 | highest available | task-network dispatch, command, reducer, and fixture tests | `feat(execution): recover fenced task claims` |
| W4B2 invocation journal | 2 | highest available | new task journal and run-store child modules | `feat(execution): persist capability invocation progress` |
| W4B3 artifact replay | 3 | strong | artifact repository child modules and tests | `fix(execution): accept exact artifact replay` |
| W4B4 recovery policy | 4 | highest available | capability contracts and provider adapter child modules | `feat(provider): bind idempotency keys to request hashes` |
| W4B5 fault harness | 5 | strong | subprocess interruption and property-test modules | `test(execution): exercise dispatch crash boundaries` |

### W4C Dispatch Integration Batch

Initial state: blocked by W4B1 through W4B4

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W4C1 dispatch actor | 1 | highest available | new task dispatch runtime and focused tests | `feat(execution): dispatch tasks through durable recovery` |
| W4C2 legacy resolution proof | 2 | strong | migration command and operator-resolution integration tests | `test(execution): prove legacy claim reconciliation` |
| W4C3 cross-store proof | 3 | highest available | reconciliation and provider-loss integration tests | `test(execution): prove cross-store dispatch recovery` |

Thread zero integrates provider ports, root handle, central contracts, exports, and test registration.

Gate: every crash boundary, unknown-call behavior, subprocess loss, property tests, full ladder, six fresh review lenses, and W5A ready.

## Wave 5 Delivery

### W5A Product Harness Freeze

Initial state: blocked by Wave 4 closeout
Threads: zero and one
Strength: highest available
Expected commits: supervisor harness, then deterministic external provider observer

Thread zero owns harness contracts, integration registry, fixture identity, and shortcut scan.
Thread one owns the isolated provider observer test module.

### W5B Product Proof Batch

Initial state: blocked by W5A

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W5B1 success proof | 1 | highest available | new product success proof file | `test(integration): prove supervised docs freshness success` |
| W5B2 failure proof | 2 | strong | new product failure proof file | `test(integration): prove docs freshness failure semantics` |
| W5B3 recovery proof | 3 | highest available | process-loss and dispatch-recovery proof files | `test(integration): prove runtime recovery checkpoints` |
| W5B4 honesty audit | 4 | strong | executable static scan and negative fixture tests | `test(integration): reject semantic proof shortcuts` |
| W5B5 documentation | 6 | standard | proof evidence and fixture retirement records | `design(plan): record honest product proof evidence` |

Wave 5 asserts the minimal typed cache classifications supplied by Waves 0 and 3.
Full console frames and broad action detail remain Wave 6.

Gate: authoritative reopen matrix, competing-supervisor proof, no fixture semantic writes, full ladder, six fresh reviews, and W6A ready.

## Wave 6 Delivery

Wave 6 uses the W6A through W6G slices defined in the parent program.

| Packet | Active Threads | Strength | Ready When | Retire When |
| --- | --- | --- | --- | --- |
| W6A process and wire freeze | 0 and 7 reviewer | highest available | Wave 5 closes | contracts and security review pass |
| W6B native action publishers | 1 through 5 | strong | W6A report placement freezes | native reports and boundary scans pass |
| W6C console and lifecycle bridge | 1 and 2 | strong and highest available | W6B shapes stabilize | bridge crash proof and presentation tests pass |
| W6D shutdown barrier | 0 with 7 reviewer | highest available | W6A and actor safe points exist | external and internal drain gates pass |
| W6E foreground control and control IPC | 1 through 3 | highest available | W6D passes | contention, authenticated status, and stop pass |
| W6F detached launch | 1 and 2 | highest available | W6E passes | readiness and launcher-failure tests pass |
| W6G event authority IPC hosting | 1 through 4 | highest available | W6F passes | event conformance, security, reconnect, and stress pass |

Thread zero integrates every central process, CLI, wire, assembly, and test-registry change.
Fresh reviewers use retired builder threads after each slice.

## Review Transition

Every packet moves from active to review only after focused gates and a provisional commit.
Critical and high findings return the packet to active.
A shared contract finding blocks all dependent packets and invalidates worktrees based on the old contract.
Medium findings require a fix or an accepted-risk record with owner, evidence, and target packet.

Wave closeout requires architecture, durability, test sufficiency, documentation, and next-wave readiness reviews.
Waves 1, 3, 4, 5, and 6 add authority-specific or security-specific review lenses from the parent program.

## Rollback And Roll-Forward

| Wave | Safe Rollback Boundary | Persisted-State Rule | Roll-Forward Trigger |
| --- | --- | --- | --- |
| 0 | before canonical id and cache writers enable | compatibility aliases and old cache reader remain | canonical records exist without reversible alias parity |
| 1 | before belief cutover and new goal writes enable | legacy belief remains read-only and migration marker records source | new authority has accepted writes that legacy cannot represent |
| 2 | disable bootstrap actor before first durable receipt | typed activation remains config, legacy directive reader remains | durable directive or bootstrap receipt exists in new shape |
| 3 | disable new actors before domain cursors advance | passive services and compatibility readers remain | new domain cursor or authority revision becomes authoritative |
| 4 | disable dispatch before first new claim or invocation | old claims remain classified and new journals remain readable | external call starts or new claim epoch is accepted |
| 5 | test-only commits may revert independently | no production state migration belongs to this wave | a discovered production fix has already landed in its owner wave |
| 6 | keep foreground mode and disable detached or IPC entry | process records, cache, and protocol readers remain compatible | a detached host owns stores or a new wire version serves clients |

Every wave begins with new behavior disabled or passive until its migration and compatibility gates pass.
Rollback uses a normal revert of the smallest reviewed commit and never deletes persisted state.
When rollback would strand authoritative new state or repeat an external effect, the wave is forward-repair only.
The ledger records that transition, disables further actor admission, and names the repair packet and owner.

## Final Reconciliation

Program completion requires:

- every packet accepted or explicitly superseded
- every provisional commit integrated or rejected
- full serial gates from a clean integration worktree
- current plan, child ledgers, assessments, Rustdoc, and indexes reconciled
- compatibility shims named with removal conditions
- no implementation status under canonical cognitive architecture
- no unexplained dirty file beyond user-owned out-of-scope files
- independent final verification with exact command evidence
- final residual risks and later-program ready set recorded
