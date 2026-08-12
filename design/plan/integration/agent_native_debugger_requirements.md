# Agent-Native Debugger Requirements

Date: 2026-07-24
Status: register frozen 2026-07-25 — every entry dispositioned, interaction modes selected; one specimen exit conjunct remains open

Conjunct note 2026-08-12: the cross-process stall-presentation conjunct discharged at harness phase three on 2026-07-26. The Goal-and-Belief prototype conjunct's stated precondition, Strategy first-slice development, has landed on `implementation/docs-freshness-strategy-runtime`, but no prototype session artifact proving the drive side — boot, inject, step, watch a posterior move through the harness — is recorded. The conjunct stays open until that session exists.
Scope: interaction model for agentic tooling driving isolated and composed Meld domain runtimes as part of the development workflow

Amendment date: 2026-07-25 — the information model and customer taxonomy added from the live runtime survey discussion, register entries DBG-015 through DBG-019 added, and the presentation posture revised: the user-facing visual projection joins this workstream, anchored on the information model and designed last.

Freeze date: 2026-07-25 — every register entry dispositioned below, the substrate boundary and serving transport recorded, and the interaction-mode selection made; delivery is chartered by the [Runtime Harness Plan](runtime_harness_plan.md).

## Concern

Runtime completion delivers isolate primitives: assembly from an explicit registration set, a public actor bounded-step contract, canonical event injection, typed query facades, preserved per-tick reports, and native domain emission. This workstream determines what sits on top: how a coding agent — Claude, Codex, or a local model — natively interacts with live Meld runtimes while implementing experiments, features, and use cases.

The end state is an agent-native custom debugger over the cognitive flywheel. The path there is a requirements exercise, not a presumed tool: gather the interaction requirements from real development workflows, evaluate interaction modes against them, and only then design the surface.

This is distinct from integration testing. Integration tests pre-compose inputs and verify outputs against a fixed expectation. The debugger is a live-fire environment: the driver decides the next stimulus after observing the last response, and the value is the feedback loop itself.

## Relationship to runtime completion

The gate is per composition, not monolithic. An isolate over registration set S is implementable once the harness enablement hooks recorded in the [Runtime Completion Ground Map](runtime_completion_ground_map.md), the scoped initialization pipeline, and every actor in S have passed review. The Goal-and-Belief isolate below therefore becomes buildable at the end of the Domain Convergence Wave, before dispatch or the convergence proof. Full-composition sessions across all twelve roles wait for runtime completion, and the emission stream depends on Workstream Seven in [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md).

Requirements gathering itself is gated by nothing and is underway in this document.

Informs: the seam constraints already folded into runtime completion exist because this workstream needs them. Registration-set composition, the public step contract, scoped resource opening, and preserved tick reports are runtime completion deliverables whose acceptance criteria this document motivates.

The requirements below were candidates; the freeze pass of 2026-07-25 dispositioned each one against the observed evidence — the live runtime survey, the isolate exit test, and the harness plan validation. Each entry carries its disposition inline.

Freeze principle, recorded from the program owner: durable harness primitives, not enterprise-grade productization. Where a candidate implies a product — a condition registry, a resident service, a dashboard — the freeze keeps the primitive and rejects the product around it.

## The layer model

```text
Layer 1  Isolate
         one domain runtime over its own stores and ledger in a
         temporary root, stepped deterministically

Layer 2  Composition
         several isolates over one shared ledger; the event spine is
         the bus, so connecting isolates requires no new transport

Layer 3  Agent surface
         the interaction mode through which tooling boots, stimulates,
         steps, observes, and scripts layers one and two
```

Layer one and layer two are runtime completion primitives composed. Only layer three is genuinely new design, and it is where this requirements exercise concentrates.

## Worked requirement: Goal and Belief isolates for Strategy development

The motivating scenario. A developer building the Strategy first slice needs the epistemic upstream live before any Strategy code exists: belief revisions arriving at Goal curation and producing Goal commands.

```text
boot composition { belief_assessment, agent_goal_curation }
    over one temporary ledger and store root
author one belief family as JSON config
register the seed agent and its curation rule
append synthetic observation events through the canonical append port
step the belief actor          → inspect revisions and views
step the curation actor        → inspect the decision record,
                                 dedupe key, and emitted Goal command
append contradicting evidence  → step both → watch the posterior move
                                 and curation absorb or re-fire
```

The connection between the two isolates is nothing but shared durable state: belief revisions and subscription cursors on one side, delivery selection on the other. Stepping actor-by-actor makes the cross-domain handoff visible at exactly the granularity the on-paper worked examples describe. When the Strategy slice lands, the same session gains a third isolate and the draft gate becomes observable at the named curation port.

The use-case walks in [Use Case Catalog](../../use_cases/README.md) are the scripted form of this scenario: an on-paper derivation becomes an executable session.

## Information model

The harness is an information hierarchy before it is any surface. Three customers consume one shared evidence record at three altitudes, and every surface is a projection of that record. Starting from visualization and working backward is the recorded anti-goal: a generic analytics dashboard that looks complete and communicates nothing.

| Customer | Question | Cadence and shape |
|---|---|---|
| Subagent | Did my last action move the coupling I am working on? | High frequency, narrow, machine-shaped: a scoped causal diff since its watermark with a blocking watch |
| Parent agent | Did anything change outside the scope I delegated, or does in-scope progress look wrong? | Exception stream plus trajectory signatures: spinning, stuck, budget exhaustion; silent when nothing crosses the boundary |
| User | What is the flow and shape of the real runtime? | Low frequency, whole-loop: coupling flow rates, queue depths at each coupling, thread exploration of one piece of work end to end |

The shared primitive is the causal record the runtime already persists: events cite source records, evidence cites publications, revisions cite evidence and theory revisions, decisions cite revisions, receipts cite decisions, task nodes carry full lineage, and epochs cite triggering revisions. Two derived walks make it navigable. The provenance walk answers why a record exists. The eligibility walk answers why a record does not exist — what would have to exist for a quiet actor to commit work. The second walk is the substance of failure context: not that an error fired, but that given the recorded chain the error was determined earlier, at a nameable divergence. Selectors compute eligibility every tick and currently discard it; recording it is the one contract change this model charters, DBG-016.

Delegation scope is a first-class filter: a named set of subjects, actors, and couplings granted by a parent when it spawns a worker. The subagent projection filters inside the scope, the parent projection filters its complement plus in-scope trajectory anomalies, and the user projection spans the loop. All three projections cite the same record identities, so any party drops to the identical evidence when it chooses to inspect — shared evidence is a property of the record, not of any application.

Worked check against the observed anchor stall: to a subagent the stall reads as a per-tick waiting-on naming the absent anchor for its exact subject key; to a parent it reads as one repeating failure signature whose thread ends at a record that does not exist; to the user it reads as a loop in which exactly one coupling never flowed, with the thread from the genesis fact dead-ending at the same absence and the subject-vocabulary mismatch visible because the selection subject never appears in any anchor record. One record, three altitudes, inevitability legible at each.

One physical constraint binds every live mode: the stores are single-process, so live observation must be served by the running foreground process — the transport-neutral event authority contract already exists for the ledger, and the per-tick report store needs the same remote read surface. Playback reads the session artifact directly after the run.

## Candidate requirements register

Each entry carries a statement, a rationale, its ground, and its disposition from the 2026-07-25 freeze.

### DBG-001 Explicit composition

Boot a named actor subset from an explicit registration set with one command. Rationale: the isolate is the unit of the workflow. Ground: the ground map's neutral hooks require registration-set assembly; a supervisor test already boots a one-actor subset.

Disposition: accepted. Delivered with the Goal-and-Belief specimen; the assembly primitive already exists.

### DBG-002 Deterministic stepping

Every step takes injected time and a bounded work budget. No wall clock inside a session. Rationale: reproducibility is what separates a debugger from watching logs. Ground: `RuntimeSupervisor::tick` already takes `now_ms`; `WorkBudget` exists.

Disposition: accepted. The phase-one session runner of the [Runtime Harness Plan](runtime_harness_plan.md) honors injected time and bounded budgets.

### DBG-003 Canonical injection

Stimuli enter only through the event append capability and public domain command ports. The debugger never pokes stores. Rationale: a harness that bypasses the sensory channel debugs a system that does not exist. Ground: append and command ports are public today.

Disposition: accepted. Standing discipline in the harness plan: no synthetic events, no store pokes, no forced cursor advances.

### DBG-004 Typed inspection

Every observable state is read through the domain query facades. Rationale: the facades are the authoritative read contracts; store internals are not. Ground: belief, agent, planner, traversal, goal, task-network, event, and supervisor read surfaces all exist.

Disposition: amended. The served `/v1` surface joins the domain query facades as an authoritative read contract, and playback mounts the same handlers over the session artifact, so post-run reads stay inside the contract. Raw store internals remain non-authoritative everywhere.

### DBG-005 Step diffing

After each step, the session reports what moved: the tick report with checkpoint movement plus the domain records created or revised since the prior step. Rationale: "what changed" is the primary debugger question and must not require manual store diffing. Ground: reports exist but are collapsed today; preservation is a Workstream Five criterion.

Observed evidence from the isolate exit test `crates/meld-world-model/tests/goal_belief_isolate.rs`: belief assessment commits inline during evidence ingestion, so the revision diff appears on the ingestion step while the assessment step truthfully reports zero attempts. Step diffing must attribute records to whichever actor committed them, or the ingestion and assessment responsibilities must later separate if per-actor causality granularity matters.

Disposition: accepted. Delivered by the phase-three projections; the attribution rule above is binding — diffs attribute records to whichever actor committed them.

### DBG-006 Machine-readable everything

Every session command emits stable JSON whose schemas are the existing serialized contract types. Rationale: the primary operator is a model; prose output is lossy. Ground: all contract types already derive serialization.

Disposition: accepted, strengthened by the substrate boundary: the JSON contract types are the product surface itself.

### DBG-007 Composed isolates

Any actor subset runs over one shared ledger and store root, and cross-domain flow is steppable actor-by-actor. Rationale: the worked scenario above; feature development almost always spans two domains. Ground: the reopen-contract test already hand-drives three actors in sequence over shared stores.

Disposition: accepted. Delivered with the Goal-and-Belief specimen.

### DBG-008 Session as artifact

A session records its stimuli and steps so it can replay deterministically. Rationale: a reproduced bug is a session file; a use-case walk is a session file; agent handoff between sessions needs an artifact. Ground: stimuli are already durable in the ledger; the step schedule is the only missing record.

Disposition: accepted. The phase-one session manifest records the step schedule, the one missing piece.

### DBG-009 Time travel

Rebuild world state to any ledger sequence and branch a session by copying the storage root. Rationale: event sourcing makes rewind nearly free, and exploring two remediation paths from one state is a core experiment shape. Ground: replay-from-ledger exists for the graph; store roots are plain directories.

Disposition: accepted, unscheduled. No observed workflow demands branching yet; replay-from-ledger arrives with playback. The freeze principle applies: the primitive is nearly free, and no product is built around it until a workflow asks.

### DBG-010 Watch conditions

Block or notify on a declared condition: a belief key revising, a goal transitioning, a task completing. Rationale: agents poll badly; watermarks and subscriptions poll well. Ground: event watermark wait and subscription poll capabilities exist.

Disposition: amended under the freeze principle. The substrate serves one watch primitive — the long-poll over the commit watermark; semantic conditions are consumer-side filters over the diff-since-watermark response. No server-side condition registry.

### DBG-011 Low ceremony and safety

One command boots an isolate with safe defaults into a temporary root. Pointing a session at an existing product data root requires an explicit unsafe flag. Rationale: agent workflows abandon tools with setup friction, and a debugger must not corrupt a real steward's state by accident. Ground: temp-root layout exists; the guard does not. The boot path is the staged pipeline of [Runtime Initialization](runtime_initialization.md) scoped to the isolate's registration subset — a harness that initializes differently from the product debugs a system that does not exist.

Disposition: accepted and scheduled: pulled into phase one of the harness plan, because phase one boots sessions. Temp-root default through the staged pipeline; explicit unsafe flag before a real data root.

### DBG-012 Emission fidelity

The session surfaces the native domain emission stream alongside step results. Rationale: narration and stepping are complementary; spew shows liveness, steps show causality. Ground: Workstream Seven.

Disposition: accepted, delivered after the three-altitude gate as a served stream endpoint; the stall work does not need it.

### DBG-013 Headless friendliness

CLI-first, no daemon, functional in sandboxed and remote environments. A Rust API and an MCP server are evaluated as additional modes by this exercise, not presumed. Rationale: the lowest common denominator across Claude, Codex, and local tooling is spawning a process and reading structured output. Ground: all current tooling is process-per-command.

Disposition: amended. The no-daemon posture constrains the debugger CLI modes; a foreground process serving loopback reads and exiting with the session is compliant. The mode selection is recorded under the interaction-mode candidates below.

### DBG-014 On-paper walk execution

A use-case worked example is expressible as a scripted session and its falsification tests as session assertions. Rationale: this closes the loop this project has already committed to — design walks become executable evidence. Ground: the walks exist; sessions do not.

Disposition: accepted. The session manifest is the script form; assertions land with the Goal-and-Belief specimen.

### DBG-015 Causal thread walk

Any record resolves to its transitive provenance thread across domain boundaries. Rationale: failure context is a chain, not a point event. Ground: the references exist piecewise — event provenance, task lineage, belief hydration references, decision input references — and a reader-side index can derive the unified walk without creating a second source of truth.

Disposition: accepted. Phase one of the harness plan.

### DBG-016 Waiting-on declarations

An idle or failing bounded report may carry a domain-owned declaration of what would make work eligible. Rationale: absence explanations are the other half of inevitability; selectors already compute eligibility and discard it. Ground: none today — this is the register's one chartered contract change, an additive field on domain reports translated through the shared tick report.

Disposition: amended twice against the code. The durable field lands on the stored action record under the report store's real encoding, which is bincode, with the tick report as the in-memory carrier. And the original may hardens to per-tick emission whenever a selector finds nothing eligible, so the eligibility walk always has a chain to resolve. Phase two of the harness plan.

### DBG-017 Delegation scope projection

A named scope of subjects, actors, and couplings filters the record into the subagent projection — scoped causal diff with blocking watch — and the parent projection — complement exceptions plus trajectory signatures such as attempted-without-committed runs and repeating failure signatures. Rationale: the parent steers on boundary deviations, not on feeds. Ground: worker scopes and subject keys on records suffice for filtering; scope itself is a harness concept requiring no runtime change.

Disposition: accepted. Phase three; trajectory signatures stay scoped to in-scope work, answering both halves of the parent question.

### DBG-018 Multi-party shared session

Extends DBG-008: one session artifact inspectable by subagent, parent agent, and user, with every projection citing shared record identities so any party can inspect the identical evidence on demand. Rationale: steering disputes resolve by dropping one altitude, not by re-running. Ground: the durable stores plus a session manifest of stimuli and steps.

Disposition: accepted. Split across phases one, three, and four of the harness plan.

### DBG-019 Standalone harness application through t3code

The harness is a standalone application serving the machine-readable projections first, with the user's visual projection rendered over the same surface, launched, snapshotted, and video-recorded through the t3code preview tooling so agent and user share the same rendered evidence. Rationale: the collaboration environment already drives local applications; adapting to it makes shared inspection structural. Ground: preview tooling exists in the collaboration harness; the live-serve constraint in the information model governs its attach path.

Disposition: amended by the substrate boundary decision. Meld ships no visualizer: the product surface is the served machine-readable substrate, and the t3code-driven application becomes the one reference consumer — non-authoritative, consuming only the published contract, still launched, snapshotted, and recorded through the preview tooling. Phase four of the harness plan.

## Interaction-mode candidates

The exercise selects among, or layers, these modes against the register:

| Mode | Strengths | Costs |
|---|---|---|
| CLI session subcommands | universal, sandbox-safe, zero residency | per-command process start; session state must live on disk |
| Rust harness crate | full typing, fastest iteration for in-repo tests | not reachable by non-Rust agent tooling |
| MCP server | richest agent ergonomics, persistent session, watch pushes | residency, authentication, one more runtime to keep truthful |
| Session transcript replay | reproducibility and handoff for all modes | not interactive by itself |

Selection criteria: agent ergonomics under real workflows, determinism, sandbox compatibility, maintenance cost, and truthfulness — a mode must not present state the durable records cannot confirm.

Selection, recorded 2026-07-25 at the freeze: the served mode is HTTP over loopback with JSON bodies drawn verbatim from the contract types and a long-poll watch over the commit watermark; the CLI session subcommands remain the sandboxed floor; the Rust crate is the serving layer's in-repo form rather than a separate mode; the MCP server is dispositioned as a possible later consumer-side adapter, never the substrate; session transcript replay is accepted through the session artifact served by the same handlers. Against the criteria: consumer-driven cursors keep determinism, loopback plus the CLI floor covers sandbox compatibility, one listener over existing contracts bounds maintenance cost, and truthfulness holds because every response body is a durable contract type.

## Out of scope

- Generic dashboards and any presentation surface designed ahead of the information model. The user-projection visual surface is in scope for this workstream, anchored on the information model, designed last, and rejected when it degrades into charts that answer no recorded customer question.
- Production attach and live debugging of a running steward. This workstream is development-time only.
- A second runtime. The debugger composes product primitives; a session that behaves differently from the product assembly is a defect of the debugger.

## Exit criteria

The workstream is ready to hand off to implementation when the register above is frozen with each entry accepted, amended, or rejected against observed agent workflows; the interaction mode set is selected against the criteria; one prototype session has reproduced the worked Goal-and-Belief scenario end to end during Strategy first-slice development; and one real stall has been presented at all three customer altitudes from one session record.

Status against the conjunction: the freeze and the mode selection were recorded 2026-07-25. The two specimen conjuncts remain open and both are owed — they prove different halves of the harness. The stall presentation proves the observation side and lands at phase three of the harness plan, cross-process over the served surface. The Goal-and-Belief prototype proves the drive side — boot, inject, step, watch a posterior move — and lands with Strategy first-slice development alongside DBG-001, DBG-007, and DBG-014.

## Read with

- [Runtime Harness Plan](runtime_harness_plan.md) — the delivery vehicle for this frozen register
- [Storage Substrate Decision Record](storage_substrate_decision_record.md) — the single-process constraint and the migration seam the served substrate extends
- [Runtime Completion Ground Map](runtime_completion_ground_map.md)
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md)
- [Runtime Initialization](runtime_initialization.md)
- [Strategy Ground Map](../world_model/strategy/ground_map.md)
- [Use Case Catalog](../../use_cases/README.md)
