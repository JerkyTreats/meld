# Meld 3: prove the flywheel through excellent commands

Date: 2026-09-10. Mode: active delivery following design commit `06fe52fc`. The native first-use journey is locally verified; the delivery loop remains active.
Source reference: `f2af45c8` on `feat/meld-3-candidate`, with working design changes.

## Outcome

Deliver excellent command-line ergonomics that prove the flywheel runtime. A user can initialize, run, observe, direct, interrupt and restart the native product, understand its outcome, and distinguish current work from retained evidence through ordinary commands.

Every requirement must serve that outcome or enable a concrete dependency. Requirements without that connection harm delivery and leave scope. The plan is our current hypothesis for reaching the outcome, not a specification to finish before learning from the runtime.

The control interface is raw but complete for operating and explaining working native capabilities. Commands expose those capabilities through their existing owners. They do not invent missing semantic behavior to satisfy an interface inventory. Startup readiness, control and observation use the same unauthenticated loopback interface. No private readiness channel, launch handshake or control credentials are required. Exact instance targeting and exclusive product ownership keep command effects correct.

## Working method

Use fast iterations through the external harness. Explore and improve the product through the product:

1. Choose the next concrete command journey or question about runtime behavior.
2. Exercise the compiled runtime through ordinary commands and capture what actually happens.
3. Identify the smallest gap in command ergonomics, visibility, harness behavior or implementation of an existing owner contract.
4. Repair that gap, improving commands and harness together when needed.
5. Rerun the same journey against the changed artifact, retain the result, and choose the next gap from the evidence.

Start with an available identifiable artifact when useful. If a missing command prevents a journey, expose the smallest existing capability needed to try it, then return to runtime feedback. Code inspection and focused tests explain or repair observed gaps; they do not replace product proof. Avoid accumulating speculative fixes or redesigning the complete command surface before exercising it.

Keep one current journey and a short record of the command, artifact, observation, correction and rerun result in the [first journey record](m3_01_installed_startup.md). Ordinary implementation choices, small harness changes and evidence formatting do not require a separate design cycle. Under implementation authority, routine iterations continue without repeated approval requests. Consequential changes to meaning or ownership invoke the circuit breaker below.

The user identifies Meld Wallpaper session `8ab30d18-9700-4f05-861a-3695221cf3f2` in `~/meld-wallpaper` as the working-method reference: fast feedback, runtime exploration and product remediation through the product. This session description is user-provided; its transcript was not retrieved here. The local [native harness guide](../../../../../meld-wallpaper/docs/native-harness.md) supports exposing existing behavior and honestly reporting absence. Transfer that feedback method, not simulation stepping, UI components or a prescribed harness architecture.

## Architectural circuit breaker

The premise is that the flywheel already has the semantic architecture needed to work. This program makes it usable and tests that premise. It is not authority to complete a fundamentally missing flywheel architecture under the heading of ergonomics.

Stop the workstream when runtime evidence reveals a major architectural gap, or a proposed repair would require one of the following:

- A new semantic owner, planner, scheduler, reconciliation loop or competing writer to make the advertised loop operate.
- A fundamental change to how intention, authorization, Execution, owner evidence or Goal judgment compose because the existing contracts cannot close the loop.
- Harness-authored semantic records, manual actor stepping or control-plane decisions substituting for missing native progress or success.
- Broad architectural repair whose scope cannot be bounded to implementing an existing owner contract.

A broken route, configuration error, missing public read, poor diagnostic or bounded bug in an existing owner is an ordinary iteration. A timeout alone is not proof that the flywheel is fundamentally broken. If the distinction is unclear, permit a bounded read-only investigation or already-authorized reproduction to classify it; do not keep patching while architectural expansion remains unresolved. Repeated unsuccessful repairs to the same native transition require this classification before another repair.

When the breaker trips, stop feature work and dependent qualification. Preserve the artifact identity, command transcript, expected native transition, observed failure and the ownership or contract gap. Mark dependent success claims unresolved. Report the finding and reassess the workstream with the user before resuming. Necessary native drain or cleanup may finish; no architectural workaround is authorized by the cleanup need. Resume only after the user agrees on the reassessed scope and authorizes the next work.

## First journey and later exploration

Start with native Startup because it exercises the loop without a provider or workspace. Through public commands: initialize, start, observe nonce and independent Goal evidence, inspect progress, stop, read retained evidence and start again. Exercise explicit requests only where the native contract supports them; Startup rejects them because lifecycle initiates admission-epoch work. Repeat initialization and supported requests, and check that old success cannot satisfy a new activation. The [M3-01 notes](m3_01_installed_startup.md) guide this journey without freezing its implementation.

The existing M3 labels remain navigation for backlog outcomes, not six fixed phases or six fully specified implementations:

| Label | Outcome to explore through the harness |
| --- | --- |
| M3-01 | Simple first-use Startup and lifecycle commands that prove the native loop. |
| M3-02 | Understand work, waits, failures, interruption and observation gaps during operation. |
| M3-03 | Make useful native commands consistent and complete; retire the paths they replace. |
| M3-04 | Exercise advertised Docs, Security and Code Change journeys with their actual dependencies and obtainable artifacts. |
| M3-05 | Support a concrete existing-user transition while preserving old data. |
| M3-06 | Qualify the resulting advertised journeys against an identifiable candidate. |

Choose work from observed product gaps. Observation and command improvements will overlap from the first iteration. Startup proof establishes that journey, not the whole advertised product set. Expand proof to real product behavior, including actual model contribution where the product requires it. Distribution and release work enable acquisition and reproducible proof; they do not block earlier local feedback merely because eventual publication is unfinished.

## Simple migration, harness and commands

Migration first preserves data and makes a current installation usable. Reuse demonstrably compatible state. Otherwise refuse incompatible data with an actionable separate-configuration path to fresh Startup, leaving old state intact. Do not make archive movement, cross-filesystem journals, a new installation metadata system or semantic history conversion prerequisites for the first journey. Add a bounded migration only when an exercised user transition needs it. Moving old data would require a specific preservation and recovery design then.

Use a small external harness to invoke an explicit compiled Meld binary, capture structured results and diagnostics, and repeat useful journeys. Interactive exploration can use Meld commands directly. Reuse `meld-eval` where useful; placement and internal code organization are routine choices. No mandatory client SDK, second operator command language, component framework, bundle layout or dedicated evidence reader is required. Preserve enough evidence to explain and rerun a result. The [harness notes](m3_runtime_harness.md) describe this loop.

Commands need useful defaults, clear help, explicit selection when ambiguous, readable progress and scriptable results. Optional overrides are appropriate when they help an actual operation; do not force configuration editing merely to keep a flag inventory small. Retain source and instance identities necessary to distinguish current work, history, acceptance and completion. Evolve output and recovery details with the journeys that exercise them.

The [command reference](m3_command_reference.md), [control matrix](m3_control_matrix.md) and [workspace assessment](m3_workspace_assessment.md) are working hypotheses and source evidence. They are not a checklist of components to invent or a requirement to implement every proposed spelling. Update them as runtime exploration establishes useful behavior. Future TUI, dashboard and direct harness HTTP support create no acceptance obligations now.

PDS expression/compiler work and compiled instrumentation/runtime-graph experiments remain deferred. Boot-time service installation, automatic process crash restart and hosted interfaces are not requirements for the first command journey.

## Existing architecture and evidence

The [domain assessment](meld_3_domain_assessment.md) maps the current foundations. Native preparation installs packages and creates identities through existing owners. Runtime owns activation, readiness and drain. Agent owns intention, authorization and Goal judgment. Execution admits and realizes authorized Tasks. Native owners return evidence. Public observation explains those records without taking over semantic decisions.

Historical Startup and reconciliation evidence supports beginning the exploration, not assuming success on a new artifact. The [flywheel remediation register](flywheel_remediation.md) retains R1–R5 evidence and acceptance history. Its soft-freeze objective remains superseded. Bounded core fixes through existing owners are allowed when implementation is authorized; major architectural gaps invoke the breaker.

[Runtime Invariants](../../../../governance/runtime_invariants.md) and [Contribution Policy](../../../../governance/contribution_policy.md) remain active. Route replacements through their canonical successor and retire superseded authorities within the same completed change. Keep runtime state outside source workspaces. Preserve existing state and do not fabricate semantic evidence. These obligations make product proof credible; they do not require a speculative redesign of unaffected code.

## Authority and qualification

The local CI workflow now gates its release job on an explicit manual `publish_release` opt-in on `master`, defaulting to false. This closes the previous automatic master-publication path in the proposed change; it is not active on GitHub until merged. No workflow was dispatched or publication attempted.

The user approved this design, requested its commit, and authorized proceeding with the program delivery loop. The design is committed. Continue bounded runtime/command implementation and external harness changes, committing at natural checkpoints until the command control plane is complete or the architectural circuit breaker trips. This does not authorize release publication, installation over the user's existing Meld, push or publication. Previous R5/Startup commits remain historical evidence; no acceptance judgment is changed here.

The user clarified that local Cargo builds and checks are routine and freely authorized for delivery. The earlier separate-build gate was a misinterpretation and is removed. Build, test, check, lint and local package verification may proceed without further approval. The guard concerns accidental release, especially through CI: crates.io publication, release tags, published candidate artifacts and release-capable workflow dispatch require explicit release authorization. Inspect workflow effects before dispatch; a local build or successful check never authorizes publication.

For each runtime iteration, retain an identifiable artifact, the exercised command and enough output to explain the observed result. For the final candidate, preserve the requested matching CI checks, Cargo/package status, build command/toolchain, exact source and artifact identities, relevant regressions and installed-product evidence. Older binaries or unrelated green checks cannot qualify a changed candidate. Publication, push and deployment remain separately controlled.

The user now supersedes the earlier design-first implementation gates, journaled-migration prerequisite, mandatory harness decomposition and requirement to finalize future contracts before starting. Design commit `06fe52fc` records that method and circuit breaker. The [iteration record](m3_01_installed_startup.md#iteration-record) retains help/version, native initialization and live Startup proof, plus the correction to misleading request-rejection diagnostics. Two native epochs have reached independent Goal satisfaction through public commands. Managed controls and wider product qualification remain open; no architectural breaker has tripped.
