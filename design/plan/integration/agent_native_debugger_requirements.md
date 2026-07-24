# Agent-Native Debugger Requirements

Date: 2026-07-24
Status: requirements gathering now; isolate prototyping gated per registration set; full-composition sessions gated by runtime completion
Scope: interaction model for agentic tooling driving isolated and composed Meld domain runtimes as part of the development workflow

## Concern

Runtime completion delivers isolate primitives: assembly from an explicit registration set, a public actor bounded-step contract, canonical event injection, typed query facades, preserved per-tick reports, and native domain emission. This workstream determines what sits on top: how a coding agent — Claude, Codex, or a local model — natively interacts with live Meld runtimes while implementing experiments, features, and use cases.

The end state is an agent-native custom debugger over the cognitive flywheel. The path there is a requirements exercise, not a presumed tool: gather the interaction requirements from real development workflows, evaluate interaction modes against them, and only then design the surface.

This is distinct from integration testing. Integration tests pre-compose inputs and verify outputs against a fixed expectation. The debugger is a live-fire environment: the driver decides the next stimulus after observing the last response, and the value is the feedback loop itself.

## Relationship to runtime completion

The gate is per composition, not monolithic. An isolate over registration set S is implementable once the harness enablement hooks recorded in the [Runtime Completion Ground Map](runtime_completion_ground_map.md), the scoped initialization pipeline, and every actor in S have passed review. The Goal-and-Belief isolate below therefore becomes buildable at the end of the Domain Convergence Wave, before dispatch or the convergence proof. Full-composition sessions across all twelve roles wait for runtime completion, and the emission stream depends on Workstream Seven in [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md).

Requirements gathering itself is gated by nothing and is underway in this document.

Informs: the seam constraints already folded into runtime completion exist because this workstream needs them. Registration-set composition, the public step contract, scoped resource opening, and preserved tick reports are runtime completion deliverables whose acceptance criteria this document motivates.

The requirements below are candidates. The exercise freezes, amends, or rejects each one against observed agent workflows once isolates are bootable.

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

## Candidate requirements register

Each entry carries a statement, a rationale, and its current ground. The exercise resolves each to accepted, amended, or rejected.

### DBG-001 Explicit composition

Boot a named actor subset from an explicit registration set with one command. Rationale: the isolate is the unit of the workflow. Ground: the ground map's neutral hooks require registration-set assembly; a supervisor test already boots a one-actor subset.

### DBG-002 Deterministic stepping

Every step takes injected time and a bounded work budget. No wall clock inside a session. Rationale: reproducibility is what separates a debugger from watching logs. Ground: `RuntimeSupervisor::tick` already takes `now_ms`; `WorkBudget` exists.

### DBG-003 Canonical injection

Stimuli enter only through the event append capability and public domain command ports. The debugger never pokes stores. Rationale: a harness that bypasses the sensory channel debugs a system that does not exist. Ground: append and command ports are public today.

### DBG-004 Typed inspection

Every observable state is read through the domain query facades. Rationale: the facades are the authoritative read contracts; store internals are not. Ground: belief, agent, planner, traversal, goal, task-network, event, and supervisor read surfaces all exist.

### DBG-005 Step diffing

After each step, the session reports what moved: the tick report with checkpoint movement plus the domain records created or revised since the prior step. Rationale: "what changed" is the primary debugger question and must not require manual store diffing. Ground: reports exist but are collapsed today; preservation is a Workstream Five criterion.

### DBG-006 Machine-readable everything

Every session command emits stable JSON whose schemas are the existing serialized contract types. Rationale: the primary operator is a model; prose output is lossy. Ground: all contract types already derive serialization.

### DBG-007 Composed isolates

Any actor subset runs over one shared ledger and store root, and cross-domain flow is steppable actor-by-actor. Rationale: the worked scenario above; feature development almost always spans two domains. Ground: the reopen-contract test already hand-drives three actors in sequence over shared stores.

### DBG-008 Session as artifact

A session records its stimuli and steps so it can replay deterministically. Rationale: a reproduced bug is a session file; a use-case walk is a session file; agent handoff between sessions needs an artifact. Ground: stimuli are already durable in the ledger; the step schedule is the only missing record.

### DBG-009 Time travel

Rebuild world state to any ledger sequence and branch a session by copying the storage root. Rationale: event sourcing makes rewind nearly free, and exploring two remediation paths from one state is a core experiment shape. Ground: replay-from-ledger exists for the graph; store roots are plain directories.

### DBG-010 Watch conditions

Block or notify on a declared condition: a belief key revising, a goal transitioning, a task completing. Rationale: agents poll badly; watermarks and subscriptions poll well. Ground: event watermark wait and subscription poll capabilities exist.

### DBG-011 Low ceremony and safety

One command boots an isolate with safe defaults into a temporary root. Pointing a session at an existing product data root requires an explicit unsafe flag. Rationale: agent workflows abandon tools with setup friction, and a debugger must not corrupt a real steward's state by accident. Ground: temp-root layout exists; the guard does not. The boot path is the staged pipeline of [Runtime Initialization](runtime_initialization.md) scoped to the isolate's registration subset — a harness that initializes differently from the product debugs a system that does not exist.

### DBG-012 Emission fidelity

The session surfaces the native domain emission stream alongside step results. Rationale: narration and stepping are complementary; spew shows liveness, steps show causality. Ground: Workstream Seven.

### DBG-013 Headless friendliness

CLI-first, no daemon, functional in sandboxed and remote environments. A Rust API and an MCP server are evaluated as additional modes by this exercise, not presumed. Rationale: the lowest common denominator across Claude, Codex, and local tooling is spawning a process and reading structured output. Ground: all current tooling is process-per-command.

### DBG-014 On-paper walk execution

A use-case worked example is expressible as a scripted session and its falsification tests as session assertions. Rationale: this closes the loop this project has already committed to — design walks become executable evidence. Ground: the walks exist; sessions do not.

## Interaction-mode candidates

The exercise selects among, or layers, these modes against the register:

| Mode | Strengths | Costs |
|---|---|---|
| CLI session subcommands | universal, sandbox-safe, zero residency | per-command process start; session state must live on disk |
| Rust harness crate | full typing, fastest iteration for in-repo tests | not reachable by non-Rust agent tooling |
| MCP server | richest agent ergonomics, persistent session, watch pushes | residency, authentication, one more runtime to keep truthful |
| Session transcript replay | reproducibility and handoff for all modes | not interactive by itself |

Selection criteria: agent ergonomics under real workflows, determinism, sandbox compatibility, maintenance cost, and truthfulness — a mode must not present state the durable records cannot confirm.

## Out of scope

- Dashboards, terminal visualizers, and any presentation surface. They consume the same hooks and come later.
- Production attach and live debugging of a running steward. This workstream is development-time only.
- A second runtime. The debugger composes product primitives; a session that behaves differently from the product assembly is a defect of the debugger.

## Exit criteria

The workstream is ready to hand off to implementation when the register above is frozen with each entry accepted, amended, or rejected against observed agent workflows; the interaction mode set is selected against the criteria; and one prototype session has reproduced the worked Goal-and-Belief scenario end to end during Strategy first-slice development.

## Read with

- [Runtime Completion Ground Map](runtime_completion_ground_map.md)
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md)
- [Runtime Initialization](runtime_initialization.md)
- [Strategy Ground Map](../world_model/strategy/ground_map.md)
- [Use Case Catalog](../../use_cases/README.md)
