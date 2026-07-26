# Runtime Harness Implementation Plan

Date: 2026-07-25
Status: active
Amended: 2026-07-25 — substrate boundary and serving-transport decisions recorded, phase two mechanics corrected to the durable encoding, eligibility walk chartered, ignition lane scope restored; register freeze pass applied, safety-guard boot pulled into phase one; phase five widened to the assembly-survey lane extension, phase three shrunk by the existing lock-free description surface
Scope: phased delivery of the interactive runtime harness over the three-customer information model, from the causal-record substrate to the served consumer boundary and its reference consumer, and the reintegration of the runtime completion endgame under it

## Overview

Objective: build the interactive runtime harness defined by the information model in [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md) — one causal record with provenance and eligibility walks, three customer projections at three altitudes, durable shared sessions, and a served substrate that external consumers read across a clear product boundary.

Boundary, recorded from the program owner: Meld ships no visualizer. The product surface ends at served, machine-readable contracts over one substrate. A terminal client, a dashboard, and the t3code-preview application are all external consumers — none authoritative, none inside the product guarantee. The guarantee is the substrate itself: every consumer reads the same record identities, live and playback surfaces are byte-consistent, and a consumer that renders what the substrate does not serve has a consumer defect, never an argument for widening the substrate.

Outcome: the live anchor stall from the runtime survey is presented at all three customer altitudes from one session record; the flywheel-ignition corrections and the bounded convergence proof then run under the harness so their behavior is validated live rather than reported second-hand.

Anti-goal, recorded from the program owner: a generic analytics dashboard that looks complete and communicates nothing. Visualization is a projection for one customer and is designed last. Stalls observed through the harness are findings to diagnose at root cause, never conditions to work around with synthetic inputs or test orchestration.

## Development phases

### Phase 1 — Session record and causal thread walk

Status: closed 2026-07-26, merged at `a5046f5` on `runtime-completion`. The harness domain landed the manifest and staged boot (`d396e4c`), the causal thread walk (`cf9f24c`), and the anchor-stall specimen (`30bbe33`); every slice passed objective-scoped fresh review with blocking findings corrected. Exit evidence: `tests/integration/harness_run.rs` pins the temp-root staged boot and the unsafe-flag guard; `tests/integration/harness_stall_specimen.rs` records the survey stall over the shipped theory body and resolves genesis fact to absent anchor including the subject-key dead end; walk re-derivation identity is pinned in both. Residuals recorded for later phases: the stage-4 genesis append still stamps a wall-clock timestamp (pre-existing events-domain request), and the theory citation on revisions cuts as out-of-scope until a registry read surface exists.

Goal: one shared evidence record with a navigable provenance walk, derived entirely from existing durable references.

Tasks:

- Define the session manifest: store roots, ledger identity, stimuli, step schedule, and closing watermarks for one run or isolate session. Harness-level; no runtime change. The manifest is a new concept with no partial predecessor; its type names must stay clear of the `Session` family already occupied by the CLI command lifecycle records and the ledger session partition.
- Boot every session through the staged pipeline of [Runtime Initialization](runtime_initialization.md) scoped to the registration subset, defaulting into a temporary root; pointing a session at an existing product data root requires an explicit unsafe flag. A harness that initializes differently from the product debugs a system that does not exist.
- Build the causal thread walk as a reader-side index over existing references — event provenance, task lineage, belief hydration references, decision input references, epoch citations — resolving any record identity to its transitive thread across domain boundaries.
- Reproduce the survey scenario as a recorded session so the walk has a real specimen.

Exit criteria:

- The thread walk resolves the anchor-stall chain end to end from the session's stores: genesis fact to absent anchor, including the subject-key dead end.
- The walk creates no second source of truth: deleting the index and re-deriving it yields identical threads.
- The default boot lands in a temporary root through the staged pipeline; opening a real data root without the explicit unsafe flag fails.

Key seams: the harness reads stores only after a run or through served surfaces during one, never concurrently by direct open; the walk is derivation, not storage.

### Phase 2 — Waiting-on declarations

Goal: the eligibility walk — DBG-016, the register's one chartered contract change.

Tasks:

- Add an additive, serde-defaulted waiting-on declaration to the domain bounded reports whose selectors compute eligibility: belief assessment, evidence ingestion, agent curation and satisfaction, planning, dispatch. Each declaration is domain-owned and names what would make work eligible.
- Translate the declaration through the shared worker tick report as the in-memory carrier and land the durable field on the stored action record, which is the serialized shape; the tick report itself is not serialized, so the contract change has two halves with different compatibility rules.
- Emission neutrality holds: the declaration derives from selector state the tick already computed and never gates, reorders, or fails semantic work.
- Derive the eligibility walk over the recorded declarations: any absent record resolves to a nameable divergence — the declaration chain that explains why it does not exist. The walk is a substrate derivation like the thread walk, never consumer-side logic, so independent consumers cannot diverge on the answer.

Exit criteria:

- A live run over the survey configuration produces per-tick waiting-on declarations naming the absent anchor and exact subject key on the stalled actor.
- The eligibility walk resolves the survey stall to its declaration chain: the walk presents the absent anchor and the subject-vocabulary mismatch as the reason the assessed revision does not exist.
- Legacy stored reports without the field load unchanged under the report store's actual encoding, which is bincode; the compatibility test decodes pre-field records rather than stripping JSON.
- Fresh review at pillar tier: the declaration shape is consumed by every projection and by the future debugger surface.

Key seams: the declaration must state absence truthfully without embedding domain policy in root; domains own the vocabulary.

### Phase 3 — Served projections

Goal: the three customer projections as machine-readable surfaces, identical live and in playback.

Tasks:

- Build the serving layer itself: the transport-neutral event authority contract exists as a trait with serialized request shapes and a conformance harness, but nothing serves it — the listener and codec are new work for both the ledger surface and the per-tick report store surface. The running foreground process serves both, because store access is single-process.
- Transport decision, recorded: HTTP over loopback. JSON bodies are the existing contract types verbatim; endpoints map one to one from the event authority contract methods plus the report store reader; the blocking watch is a long-poll over the existing commit watermark wait; the surface carries a `/v1` version prefix; and the same handlers mount over a session artifact so playback is served identically to live. The CLI JSON path remains the floor for environments without loopback. An MCP adapter is a possible later consumer-side convenience, never the substrate.
- Fix concurrent observation: the runtime survey recorded that runtime status from a second process fails on the legacy compatibility store lock. That correction lands here, pulled forward from the flywheel-ignition lane, because a served surface that cannot answer while the run is live serves nothing. The lock-free `ProductRuntimeDescription::describe_for_workspace` surface already exists with zero callers, so this fix is wiring plus a route bypass, not new construction.
- Subagent projection: scoped causal diff since a watermark — records appeared or changed within a named delegation scope, eligibility deltas, the owned actor's report diff — with a blocking watch built on the existing watermark wait.
- Parent projection: scope-complement exceptions plus trajectory signatures — attempted-without-committed runs, repeating failure signatures, budget exhaustion trends — silent when nothing crosses the boundary.
- User projection: coupling flow rates and queue depths at each flywheel coupling — dirty keys, undelivered revisions, unclaimed ready tasks, unreceipted decisions, pending publications — plus whole-thread retrieval for any record.
- Delegation scope as a named filter of subjects, actors, and couplings; a harness concept requiring no runtime change.

Exit criteria:

- All three projections served from one live run and byte-consistent when replayed from the session artifact.
- The harness gate executes cross-process: one real stall presented at all three altitudes from one session record, with every projection citing shared record identities, and the presentation made by a separate process consuming the served surface — the observation that proves the boundary is real rather than layered code in one binary.

Key seams: projections present only what durable records confirm; the served surfaces are read-only over the running process.

### Phase 4 — Consumer contract and reference consumer

Goal: the substrate published as a consumable contract, proven by one external reference consumer.

Tasks:

- Publish the consumer contract: the versioned endpoint set, the JSON shapes, the watch semantics, and the two substrate guarantees — shared record identities everywhere, and byte-consistency between live and playback.
- Build exactly one reference consumer, marked non-authoritative, over the user projection surface only, after phases one through three have answered real questions in machine form. The t3code preview tooling drives it for open, snapshot, and video recording, so agent and user share the same rendered evidence. Every rendered element traces to a recorded customer question; anything else is rejected in review under the recorded anti-goal.
- Playback: a session artifact replays through the same served surface, so the reference consumer renders live and recorded sessions identically; preview recording captures shareable video.

Exit criteria:

- Agent and user inspect the same rendered evidence for one session through the reference consumer; a subagent inspects the same session through the machine surface; all three cite identical record identities.
- The reference consumer uses only the published contract — no private imports from the product crates — so the contract is proven sufficient for a consumer we did not write.
- A recorded playback of the anchor-stall session exists.

Key seams: the reference consumer lives outside the product guarantee and consumes public contracts only; a session that behaves differently between live and playback is a defect of the harness, and a consumer that renders what the substrate does not serve is a defect of the consumer.

### Phase 5 — Runtime program reintegration

Goal: the runtime completion endgame runs under the harness.

Tasks:

- Execute the flywheel-ignition lane recorded in [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md), including its 2026-07-25 assembly-survey extension: anchor-optional cold start as family-declared theory, selection-subject canonicalization into the shared identity vocabulary, planning-theory and evidence-mapping production composition, theory load-path and selection-identity alignment, anchor source-mapping and perspective-identity alignment, publisher-to-mapping event vocabulary alignment with the per-task publication path resolved, dispatch enablement surfaced to the operator, assembly diagnostics surfacing, and genesis-fact evidence mapping so belief motion from an unobserved-scope genesis fact occurs. The lane's store-lock finding lands in phase three, where the served surface requires it. Each correction is validated live through the harness before its fresh review.
- Run the bounded convergence proof; the harness observes but never drives semantic work — the proof's forbidden behaviors remain binding.

Exit criteria:

- The ignition stalls visibly clear at all three altitudes in a live session.
- The convergence proof passes with the harness attached, closing the runtime completion program.

## Verification strategy and gates

- Formatter check gate: the repository formatter runs before test evidence for every phase that changes source or Markdown.
- Each phase lands as reviewed atomic commits under the durability-tiered review workflow: phase 2 is pillar-tier; phases 1, 3, and 4 are slice-tier with objective-scoped fresh review; phase 5 follows the runtime program's own gates.
- The authoritative harness gate is the three-altitude stall presentation from one session record, consumed by a separate process over the served surface; the authoritative program gate remains the convergence proof.
- The register's exit criteria are a four-part conjunction: register freeze, interaction-mode selection, the worked Goal-and-Belief prototype session during Strategy first-slice development, and the three-altitude stall presentation. This plan discharges the fourth and records the serving-transport selection toward the second; the freeze pass and the Goal-and-Belief specimen remain open register work outside this plan's phases and are tracked in the register itself.
- Live validation follows the stall-reporting discipline: no synthetic events, no store pokes, no forced cursor advances anywhere in the harness path.

## Implementation order summary

Session record and thread walk, then waiting-on declarations with the eligibility walk, then the served substrate with the cross-process three-altitude validation, then the consumer contract and reference consumer, then ignition and proof under the harness. Phases one and two may run as parallel lanes with disjoint write scopes; phase three depends on both; phases four and five depend on three.

## Related documentation

- [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md) — the register and information model this plan implements
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) — the program this plan reintegrates in phase five
- [Runtime Initialization](runtime_initialization.md) — isolate boots share the staged pipeline
- [Strategy Ground Map](../world_model/strategy/ground_map.md) — Strategy construction, whose first-slice development the register names as the harness's first customer
- [Storage Substrate Decision Record](storage_substrate_decision_record.md) — the sled affirmation, migration triggers, and the seam inventory the served substrate extends

## Exceptions

- The reference consumer may be launched resident during an interactive session and exits with it; the no-daemon posture of DBG-013 is read as constraining the debugger CLI modes, not a user-invoked consumer outside the product boundary.
- New workspace members for the serving layer require Cargo manifest changes, which stay with the root integration owner as reserved files under the reserved-files convention recorded in [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md).

## Register freeze

The register froze on 2026-07-25: every entry in [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md) carries a disposition, the interaction-mode selection is recorded against its criteria, and the freeze principle — durable harness primitives, not enterprise-grade productization — is recorded from the program owner. The amendments this plan forced are dispositioned there: the no-daemon posture scoped to the debugger CLI modes, the standalone application retired in favor of the substrate boundary, the serving transport selected, the waiting-on field corrected to the durable encoding, and the watch primitive narrowed to the watermark long-poll with semantic conditions as consumer-side filters. The register's two specimen exit conjuncts remain open: the cross-process stall presentation lands at phase three, and the Goal-and-Belief prototype session lands with Strategy first-slice development.
