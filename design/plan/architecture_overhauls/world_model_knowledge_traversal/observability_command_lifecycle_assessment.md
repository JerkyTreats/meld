# Observability, command lifecycle, and runtime context

Status: architecture assessment, 2026-09-09. Proposals are not implemented or accepted delivery scope.

## Problem and architectural thesis

A user can start the native runtime without being able to tell whether an owner is executing, waiting on a dependency, blocked inside a call, or no longer observable. The command surface also gives initialization and Agent different meanings from the native product lifecycle. Improving terminal wording alone cannot resolve either problem.

Every participating component should expose a defined observation contract. Shared runtime context should compose those observations with authoritative lifecycle state and domain evidence. Commands request work through existing owners and present the resulting context; CLI, agent harness, TUI, and HTTP consumers should not independently reconstruct what happened.

The user's command expectations are `meld agent`, `meld init` for first-time initialization, `meld event`, and `meld world` for the world model. This assessment treats those as product intent. It does not assume those four commands are an exhaustive command list or that their subcommands have been selected.

Scope covers operational diagnostics, component observation coverage, runtime context, command lifecycle, and presentation boundaries. It does not implement commands, an OTel exporter, a TUI, or a dashboard. This is a targeted source assessment rather than a complete instrumentation census or a fresh runtime experiment.

## Existing foundation and concrete gaps

| Concern | Current source evidence | Architectural implication |
| --- | --- | --- |
| Logging | [Logging](../../../../src/logging.rs) configures tracing filters, structured output, files, and terminal destinations. | Working transport and formatting exist. They do not establish that all components are observable. |
| OTel and TUI sinks | [OTel sink](../../../../src/telemetry/sinks/otel.rs) and [TUI sink](../../../../src/telemetry/sinks/tui.rs) contain stub comments only. | Previous statements about surviving hooks must distinguish real contracts from unimplemented sinks. |
| Actor observation | [Runtime contracts](../../../../src/runtime/contracts.rs) expose scope, checkpoints, attempts, commits, errors, budgets, and domain-owned waiting conditions. | Reuse these reports. Non-event visibility is already partially represented. |
| Retained diagnostics | [Supervisor reports](../../../../src/runtime/supervisor/reports.rs) preserve bounded tick reports outside the Events ledger. | Operational observation is not synonymous with either ephemeral logs or semantic Events. Reports already have a canonical writer and retention policy. |
| In-flight visibility | [Runtime tooling](../../../../src/runtime/tooling.rs) emits tick accounts after `supervisor.tick` returns. [Owner connection](../../../../src/runtime/owners/connection.rs) performs synchronous request and callback exchanges with deadlines. | The completed-tick account cannot itself explain a currently blocked invocation. Request deadlines exist, but their existence does not provide a live observation of elapsed work. |
| Shared context | [Harness projections](../../../../src/harness/projections.rs) derive user, parent, and subagent views with shared record identities. [Eligibility](../../../../src/harness/eligibility.rs) explains absent work using waiting declarations. | Extend and clarify ownership of the existing shared read products before introducing another context builder. These projections currently ground their claims in retained records. |
| Live access | [Served routes](../../../../src/serve/routes.rs) expose replay, subscription polling, health, reports, projections, and reconciliation intake. [Sources](../../../../src/serve/sources.rs) share the live process's stores and fence report reads to its session. | Reuse this access boundary for concurrent consumers. An observer should not need another process to acquire the database writer's lock. |
| Semantic promotion | [Runtime self-observation](../../../../src/runtime/self_observation.rs) promotes selected threshold crossings into durable facts while excluding routine gauges and per-tick samples. | Preserve explicit domain ownership of promotion. Adding a log or metric must not automatically author semantic evidence. |
| Terminal progress | [CLI progress](../../../../src/cli/progress.rs) renders replay-derived context generation progress only. | The existing renderer does not yet cover native runtime lifecycle. |
| Command semantics | [CLI definitions](../../../../src/cli/parse.rs) expose configuration-oriented Agent operations, scaffold-oriented `init`, and product preparation under `world init`. | Names and responsibilities need reconciliation with the user's expected model, beyond aliases or help edits. |

## Component observance and coverage

Define component observance as the ability to answer what a component is doing, why it is waiting, what happened to its last operation, and whether that answer is still trustworthy.

The coverage unit should be an observable behavior or boundary, not a source line or logging call. Derive the active inventory from the actual runtime composition and its declared participants, supplemented by the dependencies those participants invoke. Providers, external owner processes, transport, storage access, startup, and shutdown matter even when they are not separate registered actors.

For each applicable component behavior, the proposed coverage record contains:

- Owning component and supported lifecycle phase.
- Required observation, such as operation start, current wait, return, timeout, failure, cancellation, or recovery.
- Correlation scope, such as invocation, assignment, generation, actor, operation, and attempt, using native identities where available.
- Source contract, emission location, and intended consumers.
- Availability during execution, after completion, and after process loss.
- Last observation, freshness rule, retention boundary, and known sampling or loss.
- Verification evidence and the exact missing obligation when coverage is incomplete.

Distinguish declared, instrumented, observed, and verified coverage. A declared field is not a signal that reached a consumer. A happy-path trace does not prove timeout or recovery coverage. An unexercised behavior remains unverified; unsupported observation remains a visible gap. Any percentage needs an explicit inventory and applicability denominator. This assessment does not claim a coverage percentage.

Example: an external owner call can have durable work intent and a timeout while still lacking a visible in-flight start, elapsed time, current callback, or last response. The coverage report should identify those gaps individually. Logs arriving only after the call returns cannot satisfy in-flight coverage.

## Shared runtime context

Compose three distinguishable sources in the existing read architecture:

1. Domain evidence and authoritative lifecycle state establish what is accepted, current, committed, confirmed, or satisfied.
2. Runtime reports and operational observations explain execution, waits, attempts, transport behavior, and failures, including activity before a semantic record exists.
3. Observation availability describes freshness, missing reports, disconnected producers, truncation, and loss.

The combined result should preserve source identities and observation times. It should allow consumers to distinguish a reported idle state from silence, a pending result from a failed result, and a historical success from a current generation. It should not imply one atomic cross-domain snapshot unless the source contracts provide that guarantee. Existing Startup inspection fences are useful precedent.

Keep transient observations explicitly transient. Playback can reproduce only retained evidence and diagnostics; it must identify missing live context. A trace span ending successfully does not establish Goal satisfaction, and a satisfied Goal does not discharge an outstanding operational return.

Logging, OTel export, and presentation are consumers of these contracts. Native component signals should not require an external collector to make the CLI usable. Correlation must cross external owner and provider boundaries explicitly rather than relying on process-local span inheritance. Field selection should avoid exporting prompts, credentials, or file contents merely because an operation references them.

The current harness is documented as a development-time observation domain. Production context ownership therefore needs an explicit decision: retain its reusable projections under a clearly supported contract, or move that responsibility and route existing callers through the successor. Do not create a second authoritative projection path for each frontend.

## Command and lifecycle responsibilities

| Entry point | Intended responsibility | Current mismatch |
| --- | --- | --- |
| `meld init` | First-time setup with an explicit target and understandable readiness result, repeatable without destructive reset. | Current command initializes default agent and prompt configuration. Native preparation instead requires a declaration and package source through another command. |
| `meld agent` | Expose the native Agent's identity, intention, state, outstanding work, and supported requests. | Current family chiefly manages Reader and Writer configuration and prompt files. Native reconciliation intake is under `runtime request`. |
| `meld event` | Inspect durable Event history, causality, and ledger condition. | Useful existing operations should remain, but this surface cannot promise all operational diagnostics. |
| `meld world` | Expose supported world-model questions and operations in domain terms. | Current family contains initialization stages rather than a world-model inspection surface. Exact query vocabulary requires further design. |

Initialization, product preparation, activation, reconciliation requests, observation, and shutdown are distinct transitions even if a friendly command composes some of them. A first-time command must report which transitions it performed and what remains. Initialization must not silently imply that a running process or a satisfied Goal now exists.

Command adapters submit intent to the existing initialization, Agent, and runtime lifecycle owners. Runtime retains admission, generation, recovery, drain, and restart decisions. Observers cannot manufacture completion, retry work by inspecting it, or independently control recovery based on missing logs.

The public placement of run, status, attach, and stop remains unresolved. Evaluate it against a complete first-use and reattachment journey, including targeting when several assignments exist. The user's four command names do not resolve this choice by themselves. Observation and control should share target identity and context while retaining separate read and mutation contracts.

## Responsive presentation

The normal foreground view should describe meaningful changes and current waits. Verbosity can reveal component reports, operation attempts, evidence references, and trace details without changing execution semantics. Quiet behavior, machine-readable output, stderr diagnostics, and flag placement need one consistent contract.

The observation path must remain responsive while the work path waits. Publishing an operation start before entering a blocking call, with elapsed time and a known deadline available to a separate reader, is materially different from adding a log after completion. Losing a signal must produce unknown or stale context rather than a false healthy display. Slow observers and export failures need bounded handling and visible loss without becoming another scheduler.

Lifecycle responsiveness also includes command acceptance, interruption, and drain. Displaying an in-flight call does not make it cancellable; the architecture must distinguish an accepted stop request, draining work, and completed shutdown. It must preserve the existing owner and admission semantics.

For Startup, a useful presentation can say that the nonce is published, confirmation is pending, the Goal is satisfied, and an operational return is still outstanding. CLI, agent harness, TUI, and HTTP consumers should derive those statements from the same contracts at different levels of detail.

## Assessment outcome and remaining decisions

The smallest missing connective behavior is live operational observation joined to the existing evidence-backed projections, with explicit component coverage and a command lifecycle that exposes the same target and state. The source already contains substantial portions of this foundation; OTel export and broad native terminal presentation remain unimplemented.

Runtime participants and their dependencies are the observation inventory. Likely behavior changes concern runtime observation, component boundary instrumentation, shared projections, and first-use orchestration. CLI and HTTP remain adapters. Events, Agent, and world-model domains retain their current truth ownership; their presence on the runtime path does not imply rewriting them.

Before implementation scope is fixed, resolve production ownership of shared context, operational retention and coverage obligations, the public lifecycle command placement, the first-time initialization result, and the precise world-model command vocabulary. No command migration, runtime implementation, or remediation acceptance is performed by this assessment.

A bounded Sol review independently traced command dispatch and lifecycle ownership. It confirmed that the existing initialization, lifecycle, and supervisor authorities can support a revised public command surface. Its proposal to place foreground lifecycle under `meld agent` is retained as an option rather than a user requirement. This review did not measure instrumentation coverage or verify every command through execution.
