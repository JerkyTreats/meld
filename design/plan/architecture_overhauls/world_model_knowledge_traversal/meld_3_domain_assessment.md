# Meld 3.0 lifecycle and ergonomics domain assessment

Date: 2026-09-09. Scope: wide architecture assessment and next-workstream design; no implementation or candidate acceptance.

## Concern and authority

A new user must be able to acquire the supported product, initialize it, run native work, understand activity and waits, request work, interrupt safely, and return to retained evidence. Today that journey depends on manually assembled local setup and exposes incompatible command meanings. The proposed workstream may change core systems where this journey or its evidence requires it.

User direction is preserved verbatim:

> Ok, lets do a wide analysis of next required workstream. I want to drop soft freeze. Not having to touch core systems is evidence of soft freeze. I think now the workstream is lifecycle, ergonomics, canonize road to meld 3.0 official candidate

Soft freeze is withdrawn as a prerequisite and acceptance objective. Avoiding core edits is not a constraint. Actual repeated use that no longer needs core repair can later demonstrate stability. Single canonical authority, external runtime storage, complete replacement, and evidence requirements remain in [Runtime Invariants](../../../../governance/runtime_invariants.md).

Included: command and process lifecycle, participant observation, shared runtime context, first use, distribution, compatibility disposition, documentation and candidate proof. Full graphical TUI, hosted dashboard, new cognitive domains, and unrelated product expansion are not presumed necessary. Existing CLI, harness and HTTP contracts are current consumers; future interfaces inform contract suitability but do not become invented existing domains.

Discovery was bounded to a catalog sweep and targeted direct-path reads, with two independent Sol batches covering four crate manifests and packaging/release evidence. Existing source-backed reports supply earlier deeper evidence. This is not a full call-graph or instrumentation census; catalog-only rows identify relationships, not proved correctness. No new executable tests or registry queries were run.

## Domain universe and pass one

The universe is the 35 public root modules declared by `src/lib.rs`, four workspace library crates, two external owner packages, and the existing package/release surface. Technical modules are included explicitly so none disappear through grouping. Root exports are not assumed to be independent semantic domains.

In the table, partial means coverage for this requested user journey is incomplete or unverified, not that every listed implementation is defective. Current integration is catalog/source evidence. Needed integration selects the affected set before implementation scope.

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale or follow-up |
| --- | --- | --- | --- | --- | --- |
| `agent` | observe | Profile registry and tooling | partial | [module](../../../../src/agent.rs) | Separate profile retention from native Agent setup |
| `api` | adapter | Context adapter shared by CLI and provider | partial | [module](../../../../src/api.rs) | Preserve current consumers through composition changes |
| `branches` | consume | Registry and federated queries | partial | [module](../../../../src/branches.rs) | Preserve target identity and migration diagnostics |
| `capability` | publish | Catalog and invocation contracts | partial | [module](../../../../src/capability.rs) | Expose capability operation identity and outcomes |
| `cli` | adapter | Parser, routes, sessions, progress | partial | [module](../../../../src/cli.rs) | Replace misleading public lifecycle and help |
| `code_change` | publish | Materialization and publication contracts | partial | [module](../../../../src/code_change.rs) | Observe application attempts separately from semantic confirmation |
| `concurrency` | observe | Context locking support | partial | [module](../../../../src/concurrency.rs) | Identify relevant waits without making locks domain lifecycle owners |
| `config` | own | Selection, paths and assignment binding | partial | [module](../../../../src/config.rs) | Make explicit native first-use configuration and upgrade targeting |
| `context` | publish | Frame retrieval and generation mechanisms | partial | [module](../../../../src/context.rs) | Disposition compatibility writers and preserve required reads |
| `error` | none | Shared error types | not needed | [module](../../../../src/error.rs) | No independent component or product lifecycle |
| `events` | adapter | Canonical Events reexports and tooling | partial | [module](../../../../src/events.rs) | Route live consumers through existing owner |
| `execution` | adapter | Execution ports and projections | partial | [module](../../../../src/execution.rs) | Preserve operational identity across host and Execution |
| `harness` | own | Shared projections, walks and Startup account | partial | [module](../../../../src/harness.rs) | Resolve production ownership of reusable runtime context |
| `heads` | none | Context-owned head import | not needed | [module](../../../../src/heads.rs) | No separate authority to instrument or redesign |
| `ignore` | consume | Filesystem scan/watch selection | partial | [module](../../../../src/ignore.rs) | Preserve declared selection through retained workspace behavior |
| `init` | own | Profile setup and native preparation pipeline | partial | [module](../../../../src/init.rs) | Make native first-use setup real and repeatable |
| `logging` | own | Tracing configuration and sinks | partial | [module](../../../../src/logging.rs) | Structured operational diagnostics and filter consistency |
| `merkle_traversal` | none | Tree traversal utility | not needed | [module](../../../../src/merkle_traversal.rs) | No independently owned lifecycle established by inventory |
| `metadata` | none | Frame metadata contracts | not needed | [module](../../../../src/metadata.rs) | No new lifecycle or telemetry authority needed |
| `nonce` | publish | Deterministic nonce publication | partial | [module](../../../../src/nonce.rs) | Keep exact epoch proof through production startup and restart |
| `prompt_context` | observe | Artifact storage and lineage | partial | [module](../../../../src/prompt_context.rs) | Retain only current consumer needs and expose storage failures |
| `provider` | publish | Configured completion and diagnostics | partial | [module](../../../../src/provider.rs) | Observe in-flight requests, timeouts and returns with native correlation |
| `runtime` | own | Assignment lifecycle, supervisor, owner transport | partial | [module](../../../../src/runtime.rs) | Compose responsive lifecycle without duplicating participant authority |
| `serve` | adapter | Discovery, listener, shared read contracts | partial | [module](../../../../src/serve.rs) | Support responsive reads/control and explicit disconnected state |
| `session` | own | Command session records | partial | [module](../../../../src/session.rs) | Correlate invocation to long-lived runtime and retained history |
| `store` | consume | Node persistence | partial | [module](../../../../src/store.rs) | Protect 2.7 history and external storage through setup changes |
| `task` | publish | Host task compilation, runtime and artifacts | partial | [module](../../../../src/task.rs) | Expose actual admitted operation and artifact boundaries |
| `telemetry` | own | Sessions and emission, unimplemented OTel/TUI sinks | partial | [module](../../../../src/telemetry.rs) | Define component observations, loss and export contracts |
| `theory` | own | Package resolution, receipts and activation | partial | [module](../../../../src/theory.rs) | Provide installable native product package closure |
| `tree` | publish | Filesystem Merkle construction | partial | [module](../../../../src/tree.rs) | Retain product observations and diagnose bounded scan work |
| `types` | none | Shared primitive values | not needed | [module](../../../../src/types.rs) | No independent lifecycle behavior |
| `views` | none | Stored-context view definitions | not needed | [module](../../../../src/views.rs) | No independent lifecycle behavior |
| `workflow` | observe | Historical profile inspection | partial | [module](../../../../src/workflow.rs) | Retire unsupported execution surfaces and justify readers |
| `workspace` | own | Indexed state, watch, cleanup and CLI tools | partial | [module](../../../../src/workspace.rs) | Separate native support from defective compatibility behavior |
| `world_state` | adapter | World-model public access surface | partial | [module](../../../../src/world_state.rs) | Preserve Graph reads and evidence provenance |
| meld-world-model | own | Agent, Belief, Strategy, Planner, Curation, lifecycle and waiting contracts | partial | [source](../../../../crates/meld-world-model/src/lib.rs) | Trace semantic currentness, waits and observation without merging Agent and runtime authority |
| meld-execution | own | Task admission, Task Network, capability invocation, lifecycle and waiting | partial | [source](../../../../crates/meld-execution/src/lib.rs) | Trace progress during work, cancellation, retry and discharge; core changes allowed |
| meld-events | own | Canonical append, replay, subscriptions and observability | partial | [source](../../../../crates/meld-events/src/lib.rs) | Preserve causal identities and consumer behavior; do not store every diagnostic as a semantic fact |
| meld-lang | none | Pure shared expressions, validation and composition | not needed | [source](../../../../crates/meld-lang/src/lib.rs) | No current evidence requires changing pure language semantics; reopen if a concrete lifecycle contract needs it |
| Docs owner | publish | Selected external executable and provider-backed native product | partial | [source](../../../../owners/README.md) | Observe owner calls and include exact executable/package in distribution proof |
| Security owner | publish | Selected external executable with Cargo/advisory grants | partial | [source](../../../../owners/README.md) | Preserve independent diagnosis and mitigation evidence; acquire the executable officially |
| package/release | own | Cargo workspace and release automation | partial | [source](../../../../Cargo.toml) | Resolve release closure, version identity, installation and upgrade proof |

## Fixed affected set and pass two

The affected set is exactly the pass-one rows with needed integration other than none. The explicit exclusions are `error`, `heads`, `merkle_traversal`, `metadata`, `types`, and `views` plus the separate pure-language crate `meld-lang`; that is six root exclusions and one crate exclusion. This is an assessment boundary, not a new soft freeze. New contradictory evidence reopens the specific boundary.

Each row below decomposes one level only. Modes are change postures for investigation, not permission to edit every listed module. The root module links above ground each row.

| Domain | Major concern and owner | Required relationship | Change posture | Boundary risk |
| --- | --- | --- | --- | --- |
| `agent` | registry, prompt access, owned by agent | Separate profile retention from native Agent setup | extend existing | Preserve existing domain decisions and source identities |
| `api` | delegation, owned by api | Preserve current consumers through composition changes | adapter only | Preserve existing domain decisions and source identities |
| `branches` | selection, migration, query, owned by branches | Preserve target identity and migration diagnostics | extend existing | Preserve existing domain decisions and source identities |
| `capability` | registration, invocation, owned by capability | Expose capability operation identity and outcomes | extend existing | Preserve existing domain decisions and source identities |
| `cli` | targeting, routing, rendering, owned by cli | Replace misleading public lifecycle and help | adapter only | Preserve existing domain decisions and source identities |
| `code_change` | acquisition, materialization, publication, owned by code_change | Observe application attempts separately from semantic confirmation | extend existing | Preserve existing domain decisions and source identities |
| `concurrency` | acquisition, release, owned by concurrency | Identify relevant waits without making locks domain lifecycle owners | extend existing | Preserve existing domain decisions and source identities |
| `config` | selection, validation, persistence paths, owned by config | Make explicit native first-use configuration and upgrade targeting | extend existing | Preserve existing domain decisions and source identities |
| `context` | query, generation, publication, owned by context | Disposition compatibility writers and preserve required reads | extend existing | Preserve existing domain decisions and source identities |
| `events` | binding, CLI observation, owned by events | Route live consumers through existing owner | adapter only | Preserve existing domain decisions and source identities |
| `execution` | dispatch ports, projection, owned by execution | Preserve operational identity across host and Execution | extend existing | Preserve existing domain decisions and source identities |
| `harness` | projection, causal walk, eligibility, owned by harness | Resolve production ownership of reusable runtime context | extend existing | Preserve existing domain decisions and source identities |
| `ignore` | selection, owned by ignore | Preserve declared selection through retained workspace behavior | reuse unchanged | Preserve existing domain decisions and source identities |
| `init` | bootstrap, package preparation, owned by init | Make native first-use setup real and repeatable | replace | Preserve existing domain decisions and source identities |
| `logging` | filtering, output, correlation, owned by logging | Structured operational diagnostics and filter consistency | extend existing | Preserve existing domain decisions and source identities |
| `nonce` | capability, publication, owned by nonce | Keep exact epoch proof through production startup and restart | reuse unchanged | Preserve existing domain decisions and source identities |
| `prompt_context` | artifact persistence, lineage, owned by prompt_context | Retain only current consumer needs and expose storage failures | extend existing | Preserve existing domain decisions and source identities |
| `provider` | resolution, completion, diagnostics, owned by provider | Observe in-flight requests, timeouts and returns with native correlation | extend existing | Preserve existing domain decisions and source identities |
| `runtime` | admission, stepping, recovery, drain, observation, owned by runtime | Compose responsive lifecycle without duplicating participant authority | extend existing | Preserve existing domain decisions and source identities |
| `serve` | discovery, serving, routing, owned by serve | Support responsive reads/control and explicit disconnected state | extend existing | Preserve existing domain decisions and source identities |
| `session` | start, finish, persistence, owned by session | Correlate invocation to long-lived runtime and retained history | extend existing | Preserve existing domain decisions and source identities |
| `store` | open, migration boundary, persistence, owned by store | Protect 2.7 history and external storage through setup changes | extend existing | Preserve existing domain decisions and source identities |
| `task` | compilation, readiness, invocation, owned by task | Expose actual admitted operation and artifact boundaries | extend existing | Preserve existing domain decisions and source identities |
| `telemetry` | emission, correlation, sinks, owned by telemetry | Define component observations, loss and export contracts | extend existing | Preserve existing domain decisions and source identities |
| `theory` | resolution, installation, receipt, topology, owned by theory | Provide installable native product package closure | extend existing | Preserve existing domain decisions and source identities |
| `tree` | scan, hashing, publication inputs, owned by tree | Retain product observations and diagnose bounded scan work | extend existing | Preserve existing domain decisions and source identities |
| `workflow` | registry, historical decoding, owned by workflow | Retire unsupported execution surfaces and justify readers | extend existing | Preserve existing domain decisions and source identities |
| `workspace` | scan, watch, state administration, owned by workspace | Separate native support from defective compatibility behavior | extend existing | Preserve existing domain decisions and source identities |
| `world_state` | query forwarding, owned by world_state | Preserve Graph reads and evidence provenance | adapter only | Preserve existing domain decisions and source identities |
| meld-world-model | Agent judgment, epistemic progression, lifecycle and waits | Report native state and absence conditions through explicit contracts | extend existing | UI must not author Goal success or duplicate planning |
| meld-execution | Admission, invocation, return, recovery and discharge | Make operational work observable and interruption semantics explicit | extend existing | Stopping an observer does not cancel or discharge Tasks |
| meld-events | Append, replay, subscription and retention | Preserve authoritative evidence and expose consumer loss or lag | extend existing | Diagnostics must not become another Event spine |
| Docs owner | Observation, provider calls, publication | Attribute calls and results through selected owner identity | extend existing | Provider success is not document correctness |
| Security owner | Observation, mitigation proposal, independent verification | Attribute operation and independently verified result | extend existing | Applied source edits are not proof of resolved security posture |
| package/release | Versioning, artifact closure, installation and release evidence | Ship everything required by advertised supported paths | extend existing | Publishing the host alone is not complete product distribution |

## Synthesis and concrete findings

1. First-use setup is a real core composition problem. Native preparation expects a declaration and package source. Default initialization creates legacy profiles. A fresh-install journey needs deliberate configuration and package behavior, not only a renamed command. Configuration remains a pure resolver where that is its current contract; initialization owns any new authored setup.
2. Lifecycle has multiple legitimate owners. Assignment admission/currentness belongs to root runtime lifecycle. Readiness and safe points belong to participants. Agent owns intention and Goal judgment; Execution owns operational realization. A common user account must compose these truths rather than introduce a universal state machine that takes them over.
3. Current observation is strongest after work returns. Tick reports, waiting declarations and durable projections exist. The tick account follows the completed tick, while owner callbacks can block synchronously. In-flight activity, elapsed waits, interruption acknowledgment and observation freshness require explicit contracts at the actual boundaries.
4. Live observation is operationally significant. The served substrate already shares live stores, but listener failure currently allows execution to continue. The listener has four workers shared with blocking polls. Establish the supported failure state and reader/control responsiveness under concurrent use; do not infer that the current arrangement already meets the future UX contract.
5. Core instrumentation is in scope. Assess required observations by component and lifecycle behavior, not by log-call count. Coverage must distinguish declared, instrumented, observed and verified obligations, including waits, failures and process loss. Existing OTel/TUI sink files are stubs. Native CLI context must remain useful without an external collector.
6. Shared runtime context already exists partly in a development harness. Production ownership and supported contracts need an explicit disposition. Move or extend the canonical projections and migrate consumers in the same change; do not add an independent CLI diagnosis engine.
7. Legacy retention cannot remain speculative. Profile writers, compatibility watch, historical Workflow contracts, stored frames and assembly dependencies need current-consumer dispositions. Known watch background/debounce problems prevent describing it as a proven native service. Preserve evidence-backed reads; retire superseded writers rather than hiding them under a new label.
8. Candidate distribution is incomplete relative to the advertised products. The package review found root version 2.7.0 and a five-crate release flow that omits external owner artifacts and owner-only release triggers. Docs and Security require selected external executables. This is source evidence, not a registry audit or proof that package upload currently succeeds.
9. R5 evidence remains useful but not final-candidate acceptance. Prior native nonce and actual local-model results must be correlated to the eventual source, host, owners, packages and provider identities. Removing R6 soft freeze does not remove integrated product proof, restart proof or source-retirement obligations.

## Separate scopes

Runtime path: native owners; Events; Graph, Belief, Agent, Planner, Strategy and Curation; Execution; language contracts; runtime lifecycle; configuration, theory and provider support; existing projections and command/HTTP adapters.

Expected behavior changes: native bootstrap, runtime operation/observation boundaries, shared context ownership, command targeting/presentation, release closure, and justified legacy retirement. Core Agent, Execution, Events and owner contracts may change wherever that behavior requires it.

Likely first write scope: init/config/theory/package setup; root runtime and command/serve composition; shared projections; selected participant observation contracts; packaging and documentation. Exact files are not fixed by this breadth pass. No write to every runtime participant is required or forbidden.

Explicit non-integration: pure value/import modules do not need their own lifecycle or telemetry subsystem. OTel does not own recovery. Neither CLI nor dashboard owns semantic judgment. The current nonce and real local model are product-proof instruments, not replacement schedulers.

## Confidence and unresolved decisions

Subsequent scope addition: the user requires an agent harness that observes and directs the compiled Meld runtime as a black box. This makes the external harness a required consumer of the existing affected runtime, serve, CLI, session and observation boundaries; it adds no independently owned semantic domain. The earlier future-interface exclusion does not exclude this harness. The program ledger records objectives and a bounded reference assessment of the sibling Meld Wallpaper project. In-process injected stepping remains test support, not black-box acceptance evidence.

High confidence: current command mismatch, manual package prerequisites, completed-tick reporting, stub sinks, live read substrate, profile/native Agent distinction, and recorded native product paths. Moderate confidence: proposed integration and retirement scope. Lower confidence: uninspected core transition internals, registry publishability, supported platform/upgrade envelope, and fault behavior not freshly exercised.

Sol's core batch established public boundaries only. Its observation that a crate has stronger module documentation does not establish that it should own the whole lifecycle workstream. The package batch identified concrete source-level release omissions; owner manifest policy and registry state remain unverified.

Resolve the supported platform and distribution closure, default first-use product, 2.7 upgrade disposition, production context owner, command control contract, and observability retention/export envelope before implementation readiness. A next-workstream proposal is recorded in [the program ledger](meld_3_candidate_program.md).
