# Meld 3.0 candidate: lifecycle, ergonomics and release

Date: 2026-09-09. Mode: design. Lifecycle: active assessment. Readiness: assessment-only.
Baseline: `f41c4fea`, the user-authorized R5 and Startup commit, on `design/meld-3-candidate`. No exact releasable source candidate has been selected.

Current authorization update: the user requested committing R5 and Startup, creating the candidate branch, and recording CI, Cargo and build-candidate gates. R5 and Startup are committed at the baseline above. Candidate planning is committed separately on the new branch. Earlier pending-R5 and no-commit statements below describe the assessment-stage authority and are superseded for this closeout. Runtime implementation, crate builds, push and publication are not authorized. Every crate build requires explicit user instruction.

## Objective and authority

Deliver a named, reproducible Meld 3.0 release candidate that a new user can install, initialize, run, observe, interrupt, restart and use for the advertised native products without repository-only instructions or a custom local launcher.

The user requested a wide next-workstream analysis and explicitly withdrew soft freeze. This ledger owns the proposed next workstream and the removal of that dependency. The [flywheel remediation register](flywheel_remediation.md) continues to own R1–R5 issue and acceptance history. Its R6 soft-freeze objective and requirement to wait for freeze before lifecycle/product design are superseded. Its useful integrated proof obligations carry forward here.

Core systems may change wherever supported lifecycle, ergonomics, observation or distribution outcomes require it. The earlier command-only, no-new-functionality scope applies to the historical command inventory, not this wider workstream. Not needing further core repair is potential evidence of maturity after exercising real requirements; it is not a method constraint or release checkbox.

[Runtime Invariants](../../../../governance/runtime_invariants.md) and [Contribution Policy](../../../../governance/contribution_policy.md) remain active. Canonical ownership is not withdrawn. Accepted external-owner separation remains the architecture; a fixed-binary package-variation experiment can still demonstrate that separation without freezing the core throughout this workstream.

Authorized now: cross-domain analysis, workstream design, source inspection, delegated analysis and design-record updates, including explicit soft-freeze supersession. Not authorized by this request: implementation, acceptance of pending R5, commit, push, release tagging, package publication, installation over the user's existing Meld, or deployment. No implementation slice is active. Next boundary is authorization of a fully specified first product increment.

## Current ground and maturity

R4 was accepted by the user. R5 has recorded bounded correctness and actual local-model evidence and is ready for acceptance, not recorded here as newly accepted. Local `meld3` proves a useful native Startup path but relies on a hand-built launcher, configuration and copied package. Its package metadata still names 2.7.0. It is not an official candidate.

Maturity posture: first slice for fresh installation and lifecycle UX, with operational obligations from the user's existing 2.7 installation, persistent histories and real workspace effects. Confidence is moderate. Native reconciliation has stronger historical product evidence than first-use, upgrade, distribution and in-flight observation. That asymmetry prevents treating release engineering as a final version bump.

Direct proof must use the distributed native product. Harness tests remain useful for faults and repeatability, but neither a harness-created world nor a repository-only wrapper establishes installation or first-use success.

The user subsequently extended the workstream to improve the agent harness, with the explicit objective of observing and directing Meld as a black-box compiled runtime. This is a required consumer and acceptance path. The existing in-process test harness remains supporting infrastructure and does not satisfy that external-consumer objective.

The [domain assessment](meld_3_domain_assessment.md) owns the supporting two-pass sweep. Its affected set includes core world-model, Execution, Events, native owners, runtime, configuration, initialization, theory, observation, adapters, support storage and package/release. Pure language and utility exclusions are provisional non-integration findings, not protected files.

## Product path and ownership

Acquisition of host and required product artifacts → initialization-owned authored setup → config-owned selection and theory-owned preparation → runtime-owned activation/admission and participant readiness → Agent-owned judgment and authorized Execution/Curation → returned owner evidence → shared observation and user explanation → runtime-owned drain/recovery and retained history.

Commands parse intent and render results. Initialization may author configuration, but loading configuration must not silently start work. Runtime does not take over Agent judgment; observation does not take over scheduling. Durable Events, retained actor reports and live diagnostics remain distinguishable.

Admission has distinct meanings that must remain explicit:

| Transition | Authority | Identity and handoff to preserve |
| --- | --- | --- |
| Assignment admission | Root runtime lifecycle | Selected assignment, current activation generation and admission epoch determine whether that generation may operate. |
| Work authorization and semantic judgment | Native Agent | Agent identity, Plan and authorization references identify accepted work under the active scope. Agent separately judges returned evidence and Goal state. |
| Task admission and realization | Native Execution | The complete authorized Task enters its Task Network through Execution's admission contract. Task, network and returned operational record identities remain linked to the originating authorization and generation scope. |

Commands and observations carry references to those existing identities across the handoff. A command session or trace ID supplies correlation and cannot replace authorization, currentness or a Task identity. First-slice design must verify where the current contracts already carry each link and where an explicit contract extension is needed; this table does not claim a new shared admission record exists.

## Proposed workstream outcomes

These are dependent backlog outcomes, not six pre-authorized implementations or a required commit count.

| ID | Outcome | Core or supporting responsibility | Dependency and completion evidence |
| --- | --- | --- | --- |
| M3-01 | A real first-use Startup journey from an installable local candidate | Native bootstrap, package availability, explicit targeting, command lifecycle, minimal truthful progress | First resolve distribution and initialization decisions. Prove install into a clean user environment, initialize, run, inspect the nonce, interrupt, restart and inspect retained versus current evidence without repository paths or manual TOML. |
| M3-02 | Responsive component observation and lifecycle control | Runtime stepping and owner boundaries, Execution/Agent observation where required, sessions, live reads, shared projections, logging/export | Build on the selected lifecycle identity from M3-01. Prove visible in-flight work and waits, interruption acknowledgment, actual drain outcome, freshness and loss. Preserve native behavior when consumers disconnect or exporters fail. |
| M3-03 | Canonical public command and compatibility cutover | CLI/API adapters, native Agent/world reads as justified, profile/Context/Workflow/watch dispositions and assembly | Use the final lifecycle and observation contracts. Every advertised operation has an actual owner, old callers migrate, superseded authorities are removed, and retained historical readers have evidence. |
| M3-04 | All advertised native products are independently installable and usable | Host/package/owner distribution, provider configuration, Docs, Security and Code Change supported routes | Establish exact package and executable acquisition, identity and update relationships. Prove ordinary Docs, Security and Code Change outcomes with real dependencies; no source-workspace-only setup hidden behind installed-product claims. |
| M3-05 | Existing local users and histories have an explicit upgrade path | Versioned storage/config contracts, installation isolation, recovery, retained evidence and justified migration | Declare the supported 2.7 transition. Prove supported migration or explicit refusal with actionable separate-profile setup and preservation; do not silently reinterpret old histories or delete state. |
| M3-06 | One official candidate identity and acceptance record | Release manifests/workflows, documentation, integrated product proof and review | All prior advertised outcomes demonstrated on exact source/host/owner/package identities. Package validation, clean install and selected upgrade proof, final relevant workspace checks, actual local-model proof, and known limitations agree. Tag/publish only under explicit authority. |

M3-01 should be the first implementation increment once its contract is designed. It must include enough native observation to prove the stated journey; M3-02 is not a reason to ship an opaque first run. Minimal artifact distribution belongs in M3-01; M3-04 completes the advertised product set rather than deferring all packaging to the end. Any replaced behavior must retire within its owning increment, not wait for M3-03.

## Decisions proposed for the first increment

The external agent harness participates from M3-01: initialize and launch the compiled candidate through supported commands, discover its public interface, observe native Startup, request supported reconciliation, interrupt through supported process/control semantics, and inspect retained history after restart. M3-02 completes responsive observation, directed-operation receipts, reconnect behavior and diagnostic coverage. M3-06 includes independent black-box execution against the exact distributed artifacts.

- Use native Startup as the first-use proof because it exercises the actual loop without a provider or workspace. Product choice and target must be understandable; exact command spelling remains subordinate to the completed journey.
- Make `meld init` actual native first-use setup. This expands beyond redirecting `world init`: required package acquisition and configuration authoring must be explicit implemented behavior.
- Keep native execution foreground initially. Explicit remote stop or reattachment may be warranted by the lifecycle analysis, but background daemon/service installation is not assumed necessary.
- Reuse the existing runtime lifecycle, served read boundary and evidence-backed projections wherever sufficient. Core contract changes are allowed when their concrete limitations block the outcome.
- Prefer the existing separately selected owner model for Docs and Security. Resolve a user-facing artifact acquisition mechanism rather than folding owner semantics back into the host.
- Target a versioned prerelease such as `3.0.0-rc.1` only after the candidate artifact closure and compatibility policy are settled. Exact prerelease numbering and publication are not decided by this document.

Unresolved first-increment decisions: support platform/toolchain envelope, package-source/distribution mechanism, default initialization selection, 2.7 coexistence behavior, invocation/assignment/generation correlation, lifecycle interruption contract, and production ownership of shared context. No slice is build-ready while these remain unresolved.

## Responsibility dispositions to carry into slice design

| Responsibility | Mode | Incumbent and intended disposition | Required proof |
| --- | --- | --- | --- |
| First-use setup | replace | Replace default-profile-centric primary setup with native bootstrap through init/config/theory owners. Exact retained legacy setup path requires a consumer justification. | Clean installation, idempotent initialization, explicit failures, no unintended workspace-state writes. |
| Assignment admission and participant lifecycle | extend | Retain root lifecycle authority and participant ownership; change explicit core contracts if user-visible operation requires it. | Interrupted/restarted generations remain correctly attributed; UI never opens admission or manufactures release. |
| Execution operation observation | extend | Extend existing bounded reports/ports with necessary start/wait/return context. | Visible pending work before completion; truthful failure, timeout and discharge states. |
| Shared runtime context | move or extend, decision pending | Existing harness projections and serve consumers must have one supported production authority. A move requires caller migration and incumbent removal within the same slice. | CLI and machine consumer cite the same records; history/freshness/loss semantics agree. |
| Public commands | move/replace | Route proposed native command meanings into canonical owners; retire obsolete public execution paths and misleading meanings. | Actual entrypoint behavior and negative old-path evidence. |
| Legacy writers and readers | decision pending per surface | Retain only demonstrated required behavior. Read compatibility cannot authorize a second writer or coordinator. | Caller, export, registration, store, recovery and exclusive test disposition. |
| Distribution | extend | Existing host and owner architecture remains; release packaging must include the supported artifact closure. | Clean acquisition and package validation of exact artifacts used in acceptance. |

Modes pending decisions intentionally prevent implementation readiness. The active-slice record will identify exact current callers, successor, stored state, retirement and proof before a move or replacement starts.

## Candidate evidence and release contract

### External agent harness contract

The harness is an external operator of the compiled product. It must understand activity and direct supported work without importing runtime implementation, opening domain stores, constructing an in-process assembly, injecting supervisor time, stepping actors or manufacturing semantic records. Shared wire-schema or client code is permissible; embedded runtime behavior is not.

The runtime owns scheduling, admission, recovery and semantic decisions. The harness owns its requested operations, observation cursors, bounded waits and evidence transcript. A timeout awaiting a reply means the outcome may be unknown; it does not establish operation failure or authorize an uncorrelated retry.

| Objective | Required black-box behavior | Evidence |
| --- | --- | --- |
| Identify and connect | Discover compiled host identity, supported interface version, selected assignment and generation. Distinguish harness-launched processes from attached instances. | Connect to an exact candidate without source imports or store access; unsupported protocol versions produce an explicit result. |
| Observe | Read and follow semantic evidence, actor reports and operational diagnostics, including current work, waits, freshness, gaps and unavailable observations. | Distinguish idle, pending, disconnected and historical success using public responses. |
| Direct | Invoke supported initialization, reconciliation and lifecycle operations through public commands or protocol. | Correlated acknowledgment distinguishes acceptance, rejection and pending work. Completion follows evidence from the owning domain. |
| Reconnect | Resume bounded observation after client disconnect or runtime restart. | Cursor gaps and generation changes remain explicit; old success cannot satisfy a new request. |
| Interrupt and recover | Request supported shutdown or interrupt a harness-owned foreground process, then observe drain and exit. | Acceptance, draining, process exit and retained obligations are distinguishable. Attaching does not grant permission to kill an unrelated process. |
| Produce evidence | Record artifact/interface identity, request references, public responses, cursors, elapsed waits and final dispositions. | Another agent can inspect the transcript without hidden runtime access. Credentials and indiscriminate environment dumps are excluded. |
| Remain external | Exercise the ordinary compiled runtime throughout. | The principal scenario runs without runtime implementation imports or direct access to product stores. |

Black-box means no privileged access or manipulation, not deliberately opaque status. Public diagnostics can expose domain facts through their owning contracts. Structured evidence should establish outcomes rather than guesses from free-form log messages.

Existing native reconciliation intake is grounded functionality. Remote stop, request-status lookup, capability discovery or other missing controls must become explicit product contracts where required; they are not assumed available today. Raw Event append access does not authorize a harness to synthesize Belief, Goal satisfaction or owner results.

Ordinary coding agents should use stable commands and machine-readable replies. No particular agent vendor, private prompt, mandatory provider or new daemon is required. Harness packaging and client-library placement remain design choices.

### Reference assessment: Meld Wallpaper

Read-only inspection of the user-named sibling repository found two different reference surfaces. The native human lab in `meld-wallpaper/docs/native-harness.md` and `src/lab/runtime_observer.rs` observes shared production simulation plugins inside the lab process. Component explanations, bounded diagnostics and explicit absence are useful presentation patterns, but that lab is not itself a black-box runtime harness.

The production path in `meld-wallpaper/scripts/meld-wallpaper-runtime`, `src/runtime/control.rs` and `src/runtime/supervisor.rs` is the closer reference. The client sends a version-prefixed request through a local control interface with bounded reads and timeouts. Runtime source and documentation distinguish generation readiness and profile acknowledgment from process existence, and place recovery and candidate activation with the supervisor. The README documents fixed candidate, status and rollback operations.

Transfer the thin external client, explicit runtime identity, observed readiness, admitted control and bounded evidence patterns. Do not copy simulation controls, injected lab stepping, renderer topology, transport choice or timeout values by analogy alone. Renderer rollback does not establish that Meld can reverse workspace effects; executable lifecycle and semantic effect recovery remain distinct.

The reference was inspected, not executed or performance-validated in this turn. No files or running processes in Meld Wallpaper changed.

Meld already serves Events, reports, projections and live Agent reconciliation intake. Its current harness boot embeds the runtime assembly and injected stepping. Preserve that path for appropriate deterministic tests, but use the external harness for compiled-product acceptance. Evolve the production observation/control boundary as needed while retaining one canonical projection authority.

### Qualification evidence

An official candidate must state which platforms and product paths it supports. For each claimed path, record source revision and dirty-state disposition, host binary/version, owner executable identities, package receipts, configuration schema, and dependency/provider identity appropriate to the proof. Package behavior and documentation must match the same artifact set.

Required evidence categories:

- Fresh install and first use from the intended distribution route, with no hidden repository checkout, temporary harness setup or manually installed local launcher.
- Native nonce success, current versus retained generation distinction, repetition, interruption and reopen.
- Responsive operational context for meaningful waiting and failure paths, with component-observation obligations exercised rather than a count of logging calls.
- An external agent harness observes and directs the exact compiled candidate through supported interfaces. Disconnect, repeat request, interruption and restart preserve identity and truthful outcomes without direct store access, injected actor stepping or fabricated completion.
- Actual local-model contribution to the advertised Docs path, plus retained Security and Code Change proof appropriate to final changes. Provider availability alone is insufficient.
- Correct candidate packaging, version reporting, release change detection and required owner/package availability. Read-only registry verification and dry-run packaging precede any publish decision.
- Declared 2.7 coexistence/upgrade policy, preserved state, actionable refusal for unsupported migration, and no silent reset.
- Canonical retirement evidence, relevant domain/static checks and final workspace regression against the selected candidate.
- A README and help that lead through the proved journey and honestly distinguish unsupported features and known limitations.

These carry forward the substance of final R6 validation. There is no final requirement to declare the runtime soft-frozen. An official candidate may have stated limits; those cannot contradict its advertised first-use and product claims.

## Risks, tripwires and expansion decisions

### Required CI, Cargo and candidate-build gate

The user explicitly requires CI checks, Cargo/package status and build-candidate evidence as a qualification gate. The user also explicitly prohibits crate builds without a separate user instruction. This overrides any earlier suggestion that general implementation or validation permission is sufficient to start builds.

Read-only inspection of manifests, lockfile changes, existing artifacts and CI results may proceed. Do not run compilation-producing commands such as Cargo build, check, test, clippy, run, install, bench, package verification or publish dry-run without explicit user build authorization. Do not evade this through scripts, hooks, agents, containers or CI dispatch. Record the authorized target and scope before building. Publication, push and deployment remain separately controlled.

The gate records exact source revision and worktree disposition; relevant CI checks and their matching revision; Cargo versions, lockfile/dependency changes, publish eligibility and artifact closure; explicit build authorization; build command/toolchain/platform; candidate binary and owner/package identities; and the relevant installed-product validation results. Missing or stale evidence is pending, not passing. No candidate can qualify from a prior binary or unrelated green CI run.

Read-only baseline at commit preparation: the host and four core crates are version 2.7.0. Both external owner crates are version 0.1.0 and declare `publish = false`. A GitHub Actions query for `design/world-model-reconciliation` returned no runs. No CI run was dispatched and no Cargo compilation was performed. Owner distribution and matching candidate CI/build evidence remain unresolved.

Known risks are manual bootstrap hidden by local setup, blocked work hidden by completed-tick reporting, stale historical success displayed as current, diagnostics transport unavailable while work continues, retained legacy writers mistaken for native Agents, and host publication without required external products.

Core edits are permitted by scope. New stores, services, protocols or crates still require a concrete blocked behavior, current consumer, simpler alternative analysis and explicit disposition in the slice design. This is a design obligation, not a ban on changing core architecture. No fixed changed-file ceiling or no-core-rebuild rule is imposed.

Stop and repair design if presentation becomes a scheduler, a second writer is retained to reduce migration effort, installed product proof depends on harness preparation, or release claims exceed acquired artifacts. New evidence can reopen prior findings and acceptance where it actually contradicts them.

Full graphical TUI, hosted dashboard deployment, remote owners, arbitrary natural-language product creation, and new cognitive domains remain separate possible work. The workstream must supply runtime context suitable for consumers without assuming those interfaces must all ship to qualify the first candidate.

## Authorization, decisions and review

| Decision | Authority and disposition |
| --- | --- |
| Lift soft freeze and permit necessary core changes | Current explicit user instruction. Effective for this new analysis and workstream design. |
| Replace prior command-only constraint with lifecycle/product scope | Current user broadening of scope. Previous command inventory remains evidence, not a fixed implementation boundary. |
| Require an external compiled-runtime agent harness | Subsequent user instruction adds black-box observation and direction. Meld Wallpaper is a reference, not an implementation dependency or authority over Meld. |
| Preserve R4 acceptance and R5 evidence | Existing user acceptance and recorded proof. R5 acceptance remains pending; do not infer it from permission to assess the next workstream. |
| Supersede R6 freeze objective, retain integrated proof | Current user instruction changes the objective; this ledger carries forward applicable evidence rather than requiring the old gate first. |
| Implementation, commit, publication | Not authorized in this turn. No active implementation slice. |

Integrated design review: the breadth pass covers every current root module, crate, owner and release surface; direct source confirms the main product gaps. The proposal respects canonical ownership and preserves existing proof while withdrawing the obsolete freeze dependency. It remains assessment-only because distribution, upgrade, shared-context ownership and precise first-slice dispositions are not resolved. No implementation judgment or release acceptance is claimed.

A bounded Sol review found no material conflict with the user's direction and identified ambiguity in the repeated term admission. The transition and identity map above resolves that design-language ambiguity while leaving actual contract verification in first-slice design. No implementation readiness is inferred from this review.

Reassessment trigger: first-slice design resolves those decisions, new direct lifecycle evidence changes ownership scope, or candidate packaging exposes an incompatible distribution contract. Later outcomes remain backlog until their predecessor evidence and implementation authority are recorded.
