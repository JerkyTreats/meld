# Production Cognitive Runtime Closure Delivery Ledger

Date: 2026-07-13
Status: superseded after accepted W3A work
Program: [Production Cognitive Runtime Closure Program](production_cognitive_runtime_closure_program.md)
Base branch: `event-foundation-closeout`
Integration branch: `production-cognitive-runtime-closure`
Thread ceiling: eight

## Purpose

The [Docs Freshness Flywheel Validation Program](docs_freshness_flywheel_validation_program.md) has completed the first working vertical. This ledger remains authoritative for accepted historical commits through W3A. Its unfinished W3B through W6 packets are a deferred reliability backlog rather than current acceptance gates or an automatic next program.

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

### 2026-07-12 W1B Domain Correctness And Fault Proof

Ready items: W1B1 through W1B5
Parallelization: four isolated domain lanes followed by one cross-domain fault lane
Accepted commits: `8f464cd`, `9a425f4`, `5688842`, `fffdb9a`, `8475ffb`, `db3249c`, `1f5ecc0`, `6ce4a8a`
Implementation evidence:

- belief authority migration, commit reconciliation, evidence cursor ownership, and concurrent lease behavior
- conflict-aware durable goal commands with legal lifecycle transitions and flushed receipt visibility
- deterministic projection and planning identity with complete typed object and outcome lineage
- typed event append validation, shared ingress fencing, retryable drain, and a durable final barrier
- subprocess interruption, reopen, parity, divergent replay, concurrent append and drain, and abrupt barrier recovery tests across events, execution, and world model
- durable compatibility mutation internals narrowed behind crate-owned high-level operations

Focused gate state: passed for every accepted domain lane and the committed cross-domain fault suites
Integrated Wave 1 gate state: passed with W1C closure `eec0181`
Next ready set: W1C integration only

### 2026-07-12 W1C Root And Supervisor Integration

Ready items: W1C1 and W1C2
Parallelization: isolated supervisor-store and compatibility lanes beside root-owned entrypoint integration
Committed component evidence: `f02253a`, `c131ca8`, `84e2e17`, `1506325`, `6ce4a8a`, `8fa37b9`, `eec0181`
Implementation evidence through `6ce4a8a`:

- runtime assembly cuts legacy belief storage over to the canonical product authority
- characterization proves CLI context and generation reopen on one migrated belief authority
- error semantics reject invalid legacy paths and incompatible migration state without fallback
- durable restart schedules, ordered replacement checkpoints, and checked shutdown completions survive reopen and reject divergent replay

Integrated closure in `eec0181`:

- supervisor entrypoint consumes durable restart schedules and resumes ordered replacement checkpoints after reopen
- replacement waits for eligibility after the old handle stops, reaches a safe point, flushes, and releases its lease
- stale handles stop and flush before their local process state is discarded
- shutdown closes and drains append ingress before actor stop, safe-point, flush, product-store flush, and lease release
- the final event barrier is identity bearing, survives abrupt loss, and is persisted in the checked shutdown completion
- interrupted shutdown reopens fenced and resumes without starting actors
- completed shutdown replays exactly and rejects a divergent fence at the same watermark

Current state: accepted
Commit gate: accepted as `eec0181`
Review state: final durability rereview found no critical, high, or medium findings
Next ready set: W2A activation contract freeze

Final gate evidence:

- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --all-targets -- --test-threads=1`
- ten serial repetitions of `runtime::supervisor::store::tests::concurrent_replacement_successors_allow_one_winner`
- five serial repetitions of `concurrent_append_and_drain_admission_is_linearizable`

The serial workspace run includes 443 root library tests, 419 root integration tests, the dedicated CLI belief cutover target, every extracted crate target, and the subprocess restart and shutdown crash matrices.

### 2026-07-12 W2A Activation Contract Freeze

Ready items: W2A only
Parallelization: root-owned source loading and package split with world model and execution contract review
Accepted commit: `0bb34ee`
Contract evidence:

- versioned strict TOML source with unknown-field rejection
- explicit `--activation` path and early dry-run routing before product stores open
- one MiB source limit enforced while reading at most one extra byte
- relative source resolution against the workspace root and canonicalization before store access
- deployment-bound canonical BLAKE3 identity derived from normalized typed content and resolved deployment coordinates
- source-neutral runtime, world model, and execution owner packages with source metadata excluded
- versioned legacy embedded-directive migration identity, receipt, and conflict contracts
- deterministic world model identity and bootstrap receipt contracts without bootstrap persistence
- execution-owned scan-flow validation and deterministic method, package, network, artifact, and publication receipt contracts
- schema one restricts enabled runtimes to graph replay and the one-shot docs freshness bootstrap identity

Gate evidence:

- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --all-targets -- --test-threads=1`

The serial workspace run includes 459 root library tests, 419 root integration tests, and every workspace test target.
The final fresh rereview found no critical, high, or medium findings.
No world-model bootstrap writes, real execution asset binding, non-dry activation, recurring semantic actor activation, or Wave 2 completion are claimed.
Next ready set: W2B1 world model bootstrap and W2B2 execution validation
Blocked set: W2B3 activation proof until W2B1 and W2B2 are accepted

### 2026-07-12 Wave 2 Domain Activation And Supervised Closeout

Ready items: W2B1, W2B2, and W2B3 in dependency order
Parallelization: isolated world-model and execution owner lanes followed by root preflight and supervised integration
Integrated commits: `49fdce3`, `82dc94a`, `f549c50`, `9b82984`, `82f0e5a`, `c40e2ec`, `7fbee49`, `37cf617`, `09f9747`, `de1d54c`

Implementation evidence:

- execution binds one sealed authored method and the real docs writer package, requires the workspace snapshot DataFlow edge, validates configured repository provider identities, and resolves the typed `docs_patch` output mapping without opening execution stores
- root preflight loads repository configuration, binds execution assets, validates owner packages, and rejects invalid source or provider input before product stores open
- world-model bootstrap durably confirms belief configuration, directive, registered seed agent, curation rule, subscription, staged progress, compatibility migration, and final receipt
- bootstrap stages are `Started`, `BeliefConfigured`, `AgentRegistered`, `RuleRegistered`, `SubscriptionBound`, `ProductsConfirmed`, and `Completed`
- exact replay reconfirms immutable genesis products while preserving mutable agent lifecycle and subscription cursor state
- bootstrap leaves the agent registered, writes no `AgentActivationRecord`, and defers process hydration and operational readiness to Wave 3
- the concrete runtime `world_model.agent.bootstrap.docs_freshness` runs under supervisor lifecycle, recovers from world-model state, reaches durable no-work after completion, and shuts down cleanly
- world-model bootstrap failures carry typed fatal or retryable classification while retaining stable issue codes and messages through CLI apply verification
- supervised non-dry activation permits at most three total attempts for retryable storage failures, requires replacement before another attempt, preserves the exhaustion diagnostic, and always completes clean shutdown
- application readiness is false during passive preflight and becomes true only after supervised durable bootstrap and clean shutdown

Focused evidence:

- 19 world-model bootstrap tests
- full execution activation and package validation suites
- 6 product activation integration tests
- 32 runtime assembly tests
- 4 tooling activation tests
- 3 binary activation tests
- 9 activated branch runtime tests
- 1 world-model bootstrap classification correction test
- 5 focused bootstrap correction tests
- retry exhaustion proof with exactly two restart schedules, a stopped instance, and no active leases

Integrated gate evidence:

- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --lib --bins --tests --examples -- --test-threads=1`
- domain boundary check
- no `mod.rs` check

Criterion benchmark targets compile and lint under the all-target check and clippy gates. The combined all-target test command is not the canonical serial proof because Criterion rejects the unit-test-only `--test-threads` argument.
The canonical locked serial suite includes 470 root library tests, 425 root integration tests, and every selected workspace target.

Fresh W2B3 review and targeted rereview found no critical, high, or medium findings after the activated branch registration correction. The later integrated Wave 2 review found two medium runtime issues: retryable bootstrap failures lacked typed bounded retry classification, and CLI apply verification did not preserve the bootstrap issue code and message.

Commit `de1d54c` closes both findings with typed domain classification, stable diagnostics, bounded supervised retry, required replacement, diagnostic-preserving exhaustion, and unconditional clean shutdown. Focused correction suites, the locked serial workspace suite, strict all-target clippy, formatting, diff, domain boundary, and module layout gates pass. The final integrated rereview found no critical, high, or medium findings and validated typed classification, same-tick required replacement, two restart schedules for three total attempts, preservation of the first domain diagnostic, unconditional shutdown, receipt and readiness truth, and UTF-8 bounds.

Wave 2 does not claim recurring semantic actors, task-network mutation, task work, provider calls, artifact persistence, publication, belief revision, goal curation, satisfaction, agent operational readiness, full agent initialization, or a complete flywheel turn.
Closeout state: accepted
Next ready set: W3A semantic contract and authority freeze

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
| W1B1 | belief authority and evidence cursor | passed | `8475ffb` | full world model suite, strict clippy, formatting, and durability review passed |
| W1B2 | durable goal commands | passed | `fffdb9a` | focused and full execution suites plus strict clippy passed |
| W1B3 | planning and outcome lineage | passed | `8f464cd` | full execution all-target tests and strict clippy passed after urgency correction |
| W1B4 | fenced event append | passed | `9a425f4`, `5688842` | all-feature event matrix and workspace static ladder passed after drain correction |
| W1B5 | integrated authority fault harness | passed | `db3249c`, `1f5ecc0` | subprocess interruption, reopen, divergent replay, concurrency, and barrier recovery coverage landed |
| W1C belief cutover | passed | `f02253a`, `84e2e17`, `1506325`, `6ce4a8a` | CLI compatibility characterization and narrowed public mutation surface landed |
| W1C supervisor store | passed | `c131ca8` | checked restart, replacement, shutdown, reopen, replay, and concurrency products landed |
| W1C integrated entrypoint | passed | `eec0181` | restart ordering, abrupt recovery, final barrier, checked shutdown completion, formatting, and durability rereview passed |
| W1 integrated closeout | passed | `6ce4a8a`, `8fa37b9`, `eec0181` | full locked workspace serial ladder, repeated concurrency suites, architecture hygiene, and final durability rereview passed |
| W2A | passed | `0bb34ee` | strict source, owner contracts, early dry-run route, full locked workspace serial ladder, and final rereview passed |
| W2B1 | passed | `82f0e5a`, `c40e2ec`, `7fbee49`, `37cf617` | staged genesis bootstrap, compatibility parity, exact replay, sequence safety, and fixture cutover passed |
| W2B2 | passed | `49fdce3`, `82dc94a` | sealed authored assets, configured provider validation, required DataFlow path, and typed output mapping passed |
| W2B3 preflight | passed | `f549c50`, `9b82984` | store-free provider and asset preflight with truthful not-ready presentation passed |
| W2B3 supervised proof | passed | `09f9747`, `de1d54c` | focused proof, correction gates, and final integrated rereview passed |
| W2 integrated closeout | passed | `09f9747`, `de1d54c` | all gates passed with no critical, high, or medium review findings |
| W3A semantic authority freeze | passed | `e9d89f8` through `0217117` | locked check, strict clippy, serial workspace, full all-target and benchmark, formatting, diff, module layout, boundary, architecture review, and durability review gates passed |

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

W1B domain reviews closed durability, concurrency, lifecycle, urgency, validation, drain retry, and public-boundary findings before their accepted commits.
The W1B5 harness then added abrupt process loss and concurrent admission coverage at the cross-domain authority boundaries.

W1C integration first exposed restart and shutdown recovery work that could not be accepted from durable store products alone.
Commit `eec0181` wired checked restart schedules, ordered replacement checkpoints, append close and drain, final barrier reporting, and checked shutdown completion into the entrypoint.
The final durability rereview found no critical, high, or medium findings.

W2A review verified that commit `0bb34ee` binds canonical BLAKE3 identity to normalized typed content and resolved deployment coordinates, validates a real scan dependency path, limits the network receipt to configured identity and derived storage key, freezes versioned migration products, and routes explicit activation validation before product stores open.
The final rereview found no critical, high, or medium findings.

W2B2 review found that caller-supplied assets could drift from the built-in method and package, provider identity was not tied to repository configuration, dependency validation admitted the wrong edge kind, and output mapping was declarative only.
Commit `82dc94a` sealed complete semantic assets, required configured provider identities and the exact DataFlow artifact path, and made output mapping an execution-owned resolver.

Root preflight review found that application readiness was reported before a bootstrap factory existed.
Commit `9b82984` kept passive preflight not ready until supervised bootstrap was installed.

W2B1 review found that bootstrap crossed the canonical genesis boundary by creating activation state and marking the seed operational.
Commits `c40e2ec` and `7fbee49` left the seed registered, removed activation from the receipt, reconfirmed immutable products without rewriting mutable lifecycle state, and allocated completion after current durable product sequences.

W2B3 fresh review found an activated branch registration gap.
Commit `09f9747` includes the correction, and targeted rereview found no remaining critical, high, or medium findings.
The later integrated review found missing typed bounded retry treatment for retryable bootstrap failures and loss of bootstrap issue code and message during CLI apply verification.
Commit `de1d54c` closes both findings through typed fatal and retryable classification, stable issue codes and messages, three total supervised attempts, required replacement, diagnostic-preserving exhaustion, and unconditional clean shutdown.
Focused correction suites and the complete locked gate ladder pass. The final integrated rereview found no critical, high, or medium findings. It validated typed classification, same-tick required replacement, two restart schedules for three total attempts, preservation of the first domain diagnostic, unconditional shutdown, receipt and readiness truth, and UTF-8 bounds.

The first W3A architecture review found three high findings. Operational readiness accepted unverified belief and planner identity strings, hydration lacked durable epoch and lease fencing with a complete failure transition, and execution derived planner identity independently from world-model frames. It also found two medium boundary findings. Canonical agent field disposition was absent, and root evidence adaptation owned docs task semantic mapping.

Commits `5021817`, `0e742ab`, `cc23a68`, and `0217117` bind readiness to owner-verified durable belief attestations and planner products, fence hydration by epoch and lease, seal raw mutation surfaces, preserve restart completion, stabilize reopen recovery, and keep operational status indexes unique. Commits `a610857`, `df850a5`, `bde4a8d`, and `5f9f065` preserve exact world-model frame and world-state identity through execution and prove cross-domain reopen parity. Commit `a288040` moves docs task evidence mapping and belief mutation into the world-model belief domain. The W3A closeout records the canonical field disposition under the world-model agent owner.

The first W3A durability review found two medium issues. An operational retry could acknowledge an in-memory transaction after an indeterminate flush, and a poisoned passive task-network authority was reported unhealthy without guaranteed replacement. Commits `5021817`, `d7eb806`, and `39577b7` require durable replay acknowledgement, trigger health-based replacement, treat bounded saturation as backpressure, preserve poison behavior, and reflush indeterminate shutdown state before replacement.

The final architecture rereview at `0217117` found no critical, high, or medium findings. The final durability rereview at `0217117` also found no critical, high, or medium findings. It verified durable acknowledgement, epoch fencing, poison stickiness, five repeated saturation shutdown and reopen runs, health-triggered replacement, shutdown reflush, hydration fences, sealed mutation surfaces, owner proof verification, repeated operational hydration, indeterminate flush replay, unique status indexing, and cross-domain frame identity.

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
| W1B5 | accepted | integrated subprocess fault, parity, concurrency, and reopen harness | focused cross-domain fault suites | passed with Wave 1 closeout |
| W1C | accepted | canonical belief cutover, checked restart ordering, durable shutdown recovery, and final event barrier | focused cutover and store suites plus entrypoint recovery gates | final durability rereview passed |
| W2A | accepted | strict TOML loader, deployment-bound identity, owner packages, migration contracts, and execution validation contracts | 459 root library tests, 419 root integration tests, all workspace targets, full check, and strict clippy | final rereview passed |
| W2B1 | accepted | staged registered-agent genesis bootstrap and compatibility migration | 19 bootstrap tests, full world-model suites, and integrated serial gate | passed after genesis and sequence fix loops |
| W2B2 | accepted | sealed method and package binding with store-free deterministic validation | full execution activation and package suites plus strict clippy | passed after asset-sealing fix loop |
| W2B3 | accepted | repository preflight, supervised durable bootstrap, and bounded retry correction | focused product activation, assembly, tooling, binary, branch runtime, correction, and locked serial gates | final integrated rereview passed |
| W2 closeout | accepted | owner packages, pure execution binding, registered genesis, supervised one-shot runtime, truthful readiness, and bounded retry | locked all-target check and clippy plus locked serial workspace tests | no critical, high, or medium findings |
| W3A | accepted | process hydration, operational readiness, planner frame identity, semantic selection, evidence ownership, and task-network passive authority | 481 root library tests, 427 root integration tests, 46 world-model agent tests, 67 world-model library tests, all selected workspace targets, full all-target and benchmark suite | final architecture and durability rereviews found no critical, high, or medium findings |
| W3B | ready | accepted W3A authority and lifecycle contracts | W3A gates complete | ready for bounded semantic actor implementation |
| W4A through W6G | blocked | none | none | blocked by prior packet closeout |

## Risks And Exceptions

- The orchestration surface does not expose model selection, so relative strength is enforced through packet scope and review depth.
- A separate dirty T3 worktree remains outside this program and must not be reused or modified.
- W3A closes authority and lifecycle contracts only. Recurring semantic actors remain disabled until their W3B packets are accepted.
- Trust policy, responsibility summary, and activation policy remain deferred to the later multi-agent genesis and spawned-agent program under the world-model agent owner.
- The private embedded-directive compatibility decoder remains until supported stores carry migration receipts and no embedded directive records remain.

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

W1B5 accepted commits: `db3249c`, `1f5ecc0`

W1B5 gate evidence:

- event append recovery distinguishes work before and after durable acknowledgement under abrupt process loss
- append admission and drain are linearizable under concurrency
- final barrier recovery reopens at the durable tip without admitting late writes through the closed authority
- goal command recovery preserves the canonical commit and rejects divergent replay across reopen
- attributed outcome publication remains canonical and does not duplicate after reopen
- belief migration preserves a gap-free evidence receipt chain and fences the legacy source
- every harness uses public domain operations and durable reopen behavior rather than internal mutation shortcuts

### W1C Supervisor And Root Cutover

Initial state: blocked by W1B1 through W1B4
Current state: accepted

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W1C1 supervisor safety | 1 | highest available | `src/runtime/supervisor/` excluding central entry integration | `fix(runtime): enforce restart and contender safety` |
| W1C2 compatibility proof | 2 | strong | belief cutover, legacy goal, and runtime id parity tests | `test(runtime): prove compatibility cutover parity` |

Thread zero integrates root belief cutover, projection adapters, event fence wiring, supervisor entrypoint changes, and public exports.

Committed component evidence:

- `f02253a` cuts root assembly over to canonical belief authority
- `c131ca8` persists checked restart schedules, ordered replacement checkpoints, and shutdown completions
- `84e2e17` characterizes CLI belief authority migration and reopen
- `1506325` preserves fail-closed belief cutover errors
- `6ce4a8a` narrows durable compatibility mutations behind domain-owned public operations
- `8fa37b9` reconciles canonical language and execution contracts with the accepted authority boundaries
- `eec0181` enforces restart ordering and recoverable final-barrier shutdown in the supervisor entrypoint

Accepted root integration:

- checked restart schedules and replacement checkpoints drive entrypoint recovery
- replacement eligibility is enforced before lease acquisition and actor start
- the old actor stops, reaches a safe point, flushes, and releases its lease before replacement
- event ingress closes and drains before actor shutdown and lease release
- shutdown completion persists the final identity-bearing event barrier and survives abrupt reopen
- final durability rereview reports no critical, high, or medium findings

Gate: crash matrix, repeated serial concurrency, full ladder, three fresh review lenses, and W2A ready.
Accepted closure commit: `eec0181`
Gate result: passed

## Wave 2 Delivery

### W2A Activation Contract Freeze

Initial state: blocked by Wave 1 closeout
Current state: accepted
Thread: zero
Strength: highest available
Expected commit: `feat(runtime): define typed product activation packages`

Freeze TOML schema, explicit path policy, normalized hash, size and field validation, owner packages, bootstrap identity, execution validation receipt, and legacy directive migration.

Accepted commit: `0bb34ee`

Accepted contract scope:

- strict versioned TOML source and explicit activation path
- passive early dry-run loader before product stores open
- one MiB source gate and strict unknown-field rejection
- deployment-bound canonical BLAKE3 identity over normalized typed content
- independent source-neutral runtime, world model, and execution packages
- durable directive and seed-agent reference contracts
- versioned legacy embedded-directive migration identity, receipt, and conflict products
- deterministic belief, bootstrap, and execution receipt identities
- real workspace scan dependency-flow validation contract
- task network identity and storage-key receipt without claiming authored topology
- artifact and publication mapping receipt contracts

Acceptance does not include world-model bootstrap writes, real execution asset binding, non-dry activation, recurring semantic work, or Wave 2 completion.
Gate result: passed
Next ready set: W2B1 and W2B2

### W2B Domain Activation Batch

Initial state: blocked by W2A
Current state: accepted through `de1d54c`

| Packet | Thread | Strength | Exclusive Builder Scope | Expected Commit |
| --- | ---: | --- | --- | --- |
| W2B1 world model bootstrap | 1 | highest available | agent and belief activation child modules plus focused tests | `feat(agent): bootstrap configured seed directive` |
| W2B2 execution validation | 2 | highest available | method, package, artifact, network, and publication validation modules | `feat(execution): validate docs freshness activation` |
| W2B3 activation proof | 3 | strong | checked-in activation fixture and new activation integration test | `test(runtime): prove activation replay and conflict handling` |

The root-owned loader, central schemas, early CLI dry-run adapter, and exports landed in W2A.
Thread zero integrates reviewed owner implementations with the assembly factory and test registration after W2B1 and W2B2.

Gate: reopen after every bootstrap stage, divergent-config proof, compatibility parity, two fresh reviews, and W3A ready.

Accepted W2B1 commits: `82f0e5a`, `c40e2ec`, `7fbee49`, `37cf617`
Accepted W2B2 commits: `49fdce3`, `82dc94a`
Accepted W2B3 preflight commits: `f549c50`, `9b82984`
Integrated W2B3 supervised proof: `09f9747`
Integrated Wave 2 correction: `de1d54c`

Closeout boundary:

- bootstrap confirms registered genesis products and does not perform process activation
- no recurring semantic role is enabled
- no task, provider, artifact, publication, belief revision, curation, satisfaction, or full flywheel completion is claimed

## Wave 3 Delivery

### W3A Semantic Contract And Authority Freeze

Initial state: blocked by Wave 2 closeout
Current state: accepted through `0217117`
Threads: zero and one
Strength: highest available
Accepted commits: `e9d89f8`, `4cfabc7`, `7920db7`, `d050d7c`, `1377127`, `2bc8f9f`, `574e861`, `6b01c33`, `a610857`, `d7eb806`, `df850a5`, `a288040`, `5021817`, `bde4a8d`, `5f9f065`, `0e742ab`, `39577b7`, `cc23a68`, `0217117`

W3A starts from a registered seed and completed genesis receipt. The accepted freeze gives the world-model agent domain proof-bearing process hydration, readiness, activation record, and operational transition authority. Belief attestations and planner request and frame products are durable owner-verified readiness inputs. Hydration start, failure, retry, and operational completion are fenced by epoch, lease, sequence, and exact durable identity.

Execution owns one serialized task-network authority per configured network with cloneable bounded command and query ports, durable acknowledgement, lifecycle epoch fencing, poison behavior, safe shutdown, and reopen recovery. Root hosts that authority as a passive service and replaces an unhealthy authority through supervisor lifecycle without presenting it as a ticking actor. Semantic runtime selection binds accepted Wave 2 receipt identity, exact actor and service sets, and the configured task network. World-model projection frame identity is preserved through execution planning without parallel derivation. Root event adaptation exposes generic replay only, while docs task evidence mapping and belief mutation are world-model belief responsibilities.

Canonical field disposition:

| Field | Disposition |
| --- | --- |
| subscription references on `AgentRecord` | superseded by normalized `AgentSubscriptionRecord` values and agent indexes |
| agent-level evidence policy | superseded for this slice by `BeliefKey` and belief family configuration |
| trust policy | deferred to the later multi-agent genesis and spawned-agent program |
| responsibility summary | deferred to the later multi-agent genesis and spawned-agent program |
| activation policy | deferred to the later multi-agent genesis and spawned-agent program |

Disposition owner: world-model agent domain

Disposition reason: avoid duplicate authority and unauthored policy in the single-seed slice.

Disposition target: the later multi-agent genesis and spawned-agent program beyond this single-seed production closure.

Gate evidence:

- locked workspace all-target check passed
- locked workspace strict clippy passed
- locked serial workspace passed with 481 root library tests, 427 root integration tests, 46 world-model agent tests, 67 world-model library tests, and every selected workspace target
- full all-target suite including benchmarks passed
- formatting, diff hygiene, no `mod.rs`, and domain boundary scans passed
- final architecture and durability rereviews at `0217117` found no critical, high, or medium findings

Acceptance boundary: W3A enables no recurring semantic actor and claims no curation, projection tick, planning tick, publication tick, satisfaction tick, task dispatch, provider call, artifact, belief revision, or complete flywheel turn.

Next ready set: W3B semantic actor batch

### W3B Semantic Actor Batch

Initial state: blocked by W3A
Current state: implementation in progress after accepted W3A

Accepted shared prework:

| Commit | Evidence | Unblocks |
| --- | --- | --- |
| `376714a` | operational readiness preserves the first pending belief delivery while fencing equal, ahead, and divergent cursors | recurring agent delivery selection |
| `8d5e391` | semantic handles receive the exact supervisor lease context | actor-owned lease fencing |
| `4532c55` | root opens and flushes the world-model planner authority, with durable pending-request reopen proof | planner projection actor and durable execution bridge |

The active dependency is the execution planning request and runtime asset freeze. It adds explicit subject and durable source sequence identity to execution projection requests and exposes the exact execution-owned capability catalog used during activation verification. After that freeze, durable planner projection, authority-backed execution planning, and authority-backed publication are parallel-safe.

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
