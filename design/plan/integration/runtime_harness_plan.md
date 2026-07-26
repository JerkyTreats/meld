# Runtime Harness Implementation Plan

Date: 2026-07-25
Status: active
Scope: phased delivery of the interactive runtime harness over the three-customer information model, from the causal-record substrate to the t3code-adapted application, and the reintegration of the runtime completion endgame under it

## Overview

Objective: build the interactive runtime harness defined by the information model in [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md) — one causal record with provenance and eligibility walks, three customer projections at three altitudes, durable shared sessions, and a standalone application adapted to the t3code collaboration environment.

Outcome: the live anchor stall from the runtime survey is presented at all three customer altitudes from one session record; the flywheel-ignition corrections and the bounded convergence proof then run under the harness so their behavior is validated live rather than reported second-hand.

Anti-goal, recorded from the program owner: a generic analytics dashboard that looks complete and communicates nothing. Visualization is a projection for one customer and is designed last. Stalls observed through the harness are findings to diagnose at root cause, never conditions to work around with synthetic inputs or test orchestration.

## Development phases

### Phase 1 — Session record and causal thread walk

Goal: one shared evidence record with a navigable provenance walk, derived entirely from existing durable references.

Tasks:

- Define the session manifest: store roots, ledger identity, stimuli, step schedule, and closing watermarks for one run or isolate session. Harness-level; no runtime change.
- Build the causal thread walk as a reader-side index over existing references — event provenance, task lineage, belief hydration references, decision input references, epoch citations — resolving any record identity to its transitive thread across domain boundaries.
- Reproduce the survey scenario as a recorded session so the walk has a real specimen.

Exit criteria:

- The thread walk resolves the anchor-stall chain end to end from the session's stores: genesis fact to absent anchor, including the subject-key dead end.
- The walk creates no second source of truth: deleting the index and re-deriving it yields identical threads.

Key seams: the harness reads stores only after a run or through served surfaces during one, never concurrently by direct open; the walk is derivation, not storage.

### Phase 2 — Waiting-on declarations

Goal: the eligibility walk — DBG-016, the register's one chartered contract change.

Tasks:

- Add an additive, serde-defaulted waiting-on declaration to the domain bounded reports whose selectors compute eligibility: belief assessment, evidence ingestion, agent curation and satisfaction, planning, dispatch. Each declaration is domain-owned and names what would make work eligible.
- Translate the declaration through the shared worker tick report and preserve it in the durable report store.
- Emission neutrality holds: the declaration derives from selector state the tick already computed and never gates, reorders, or fails semantic work.

Exit criteria:

- A live run over the survey configuration produces per-tick waiting-on declarations naming the absent anchor and exact subject key on the stalled actor.
- Legacy stored reports without the field load unchanged; a JSON strip test proves it.
- Fresh review at pillar tier: the declaration shape is consumed by every projection and by the future debugger surface.

Key seams: the declaration must state absence truthfully without embedding domain policy in root; domains own the vocabulary.

### Phase 3 — Served projections

Goal: the three customer projections as machine-readable surfaces, identical live and in playback.

Tasks:

- Add the remote read surface for the per-tick report store beside the existing transport-neutral event authority contract; the running foreground process serves both, because store access is single-process.
- Subagent projection: scoped causal diff since a watermark — records appeared or changed within a named delegation scope, eligibility deltas, the owned actor's report diff — with a blocking watch built on the existing watermark wait.
- Parent projection: scope-complement exceptions plus trajectory signatures — attempted-without-committed runs, repeating failure signatures, budget exhaustion trends — silent when nothing crosses the boundary.
- User projection: coupling flow rates and queue depths at each flywheel coupling — dirty keys, undelivered revisions, unclaimed ready tasks, unreceipted decisions, pending publications — plus whole-thread retrieval for any record.
- Delegation scope as a named filter of subjects, actors, and couplings; a harness concept requiring no runtime change.

Exit criteria:

- All three projections served from one live run and byte-consistent when replayed from the session artifact.
- The register's exit criterion executes: one real stall presented at all three altitudes from one session record, with every projection citing shared record identities.

Key seams: projections present only what durable records confirm; the served surfaces are read-only over the running process.

### Phase 4 — t3code adaptation and the user visual projection

Goal: the standalone application, shared between agent and user through the collaboration environment.

Tasks:

- Package the served projections as a standalone application with launch and attach conventions for the t3code preview tooling: open, snapshot, and video recording against the same rendered surface the user sees.
- Design the user visual projection over the user projection surface only, after phases one through three have answered real questions in machine form. Every rendered element traces to a recorded customer question; anything else is rejected in review under the recorded anti-goal.
- Playback: a session artifact replays through the same application; preview recording captures shareable video.

Exit criteria:

- Agent and user inspect the same rendered evidence for one session; a subagent inspects the same session through the machine surface; all three cite identical record identities.
- A recorded playback of the anchor-stall session exists.

Key seams: the application consumes public contracts only; a session that behaves differently from the product is a defect of the harness.

### Phase 5 — Runtime program reintegration

Goal: the runtime completion endgame runs under the harness.

Tasks:

- Execute the flywheel-ignition lane recorded in [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md): anchor-optional cold start as family-declared theory, selection-subject canonicalization into the shared identity vocabulary, planning-theory production composition, assembly diagnostics surfacing. Each correction is validated live through the harness before its fresh review.
- Run the bounded convergence proof; the harness observes but never drives semantic work — the proof's forbidden behaviors remain binding.

Exit criteria:

- The ignition stalls visibly clear at all three altitudes in a live session.
- The convergence proof passes with the harness attached, closing the runtime completion program.

## Verification strategy and gates

- Formatter check gate: the repository formatter runs before test evidence for every phase that changes source or Markdown.
- Each phase lands as reviewed atomic commits under the durability-tiered review workflow: phase 2 is pillar-tier; phases 1, 3, and 4 are slice-tier with objective-scoped fresh review; phase 5 follows the runtime program's own gates.
- The authoritative harness gate is the three-altitude stall presentation from one session record; the authoritative program gate remains the convergence proof.
- Live validation follows the stall-reporting discipline: no synthetic events, no store pokes, no forced cursor advances anywhere in the harness path.

## Implementation order summary

Session record and thread walk, then waiting-on declarations, then served projections with the three-altitude validation, then the t3code application and visual projection, then ignition and proof under the harness. Phases one and two may run as parallel lanes with disjoint write scopes; phase three depends on both; phases four and five depend on three.

## Related documentation

- [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md) — the register and information model this plan implements
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) — the program this plan reintegrates in phase five
- [Runtime Initialization](runtime_initialization.md) — isolate boots share the staged pipeline
- [Strategy Ground Map](../world_model/strategy/ground_map.md) — the first development workflow the harness serves

## Exceptions

- The harness application may be launched resident during interactive sessions; the no-daemon posture of DBG-013 applies to the debugger CLI modes, not to a user-invoked application that exits with the session.
- New workspace members for the harness application require Cargo manifest changes, which stay with the integration owner as reserved files.
