# Cognitive Architecture Implementation Plan

Date: 2026-06-02
Status: active
Scope: declarative implementation readiness and dependency order for the cognitive architecture

## Purpose

This directory defines implementation readiness, dependency order, scope cuts, and contract closure for the cognitive architecture.

Durable architecture intent lives under [design/cognitive_architecture](../cognitive_architecture/README.md). That directory is canonical for declarative design intent. This plan states what can be built, what must be built first, and what remains blocked.

Each assessment named as current authority states current truth. Historical and superseded artifacts preserve dated evidence but must not direct implementation.

When this plan conflicts with `design/cognitive_architecture`, treat the architecture document as intent and this plan as an implementation readiness snapshot that needs reconciliation.

## Current Runtime Completion Authority

The active runtime-completion objective is [Runtime Completion Ground Map](integration/runtime_completion_ground_map.md), with build-facing decomposition in [Runtime Completion Implementation Workstreams](integration/runtime_completion_implementation_workstreams.md).

The runtime completion endgame — the flywheel-ignition lane and the bounded convergence proof — executes under the [Runtime Harness Plan](integration/runtime_harness_plan.md), which delivers the served observation substrate defined by the frozen [Agent-Native Debugger Requirements](integration/agent_native_debugger_requirements.md). The substrate boundary is recorded there: served machine-readable contracts as the product surface, all visualization external. The standing storage posture behind that substrate is the [Storage Substrate Decision Record](integration/storage_substrate_decision_record.md).

## Current Delivery Order

The order below is the active program sequence as of 2026-07-25, grounded in an assembly code survey of the same date. Two facts order everything. First, the domain contracts are substantially built and tested — goal acceptance, publication, evidence mapping, satisfaction ownership are all implemented, and the [Non Assembly Gap Fix Requirements](integration/non_assembly_gap_requirements.md) are closed as contracts — but the live product composition does not turn: theory is never injected, event vocabularies do not intersect, dispatch is disabled, and every such defect is recorded in the flywheel-ignition lane of [Runtime Completion Implementation Workstreams](integration/runtime_completion_implementation_workstreams.md). Second, the one live stall was found by manual survey because no shipped surface can observe a running runtime; the operator-visibility program that would have built one is superseded. The order therefore builds the observation instrument first and drives every semantic correction through it.

1. Harness phases one and two, parallel lanes: the session record with the guarded temp-root boot and causal thread walk, and the waiting-on declarations with the eligibility walk. These make stalls diagnosable as per-tick declarations instead of archaeology.
2. Harness phase three: the served HTTP substrate over the running foreground process, including the store-lock concurrent-observation fix. Gate: one real stall presented at all three customer altitudes by a separate process consuming the served surface. From this point Meld has a stall detector.
3. Harness phase five: the flywheel-ignition lane — the five survey findings plus the assembly-survey extension — each validated live through the harness, then the bounded convergence proof with the harness attached. This closes the runtime completion program and is the first time the full loop turns semantically. Harness phase four, the published consumer contract and single non-authoritative reference consumer, proceeds in parallel, as do the Strategy delta's pure-language items from the [Strategy Ground Map](world_model/strategy/ground_map.md).
4. The merged train reaches master as one unit at the proof, per branch discipline.
5. Strategy first-slice construction proceeds inside the harness as its first development customer, landing the Goal-and-Belief prototype session that closes the debugger register's final exit conjunct. The [Multiplier Harness Program](integration/multiplier_harness_program.md) unlocks in parallel once its Ollama validation joins — its other dependencies, the turning flywheel and belief injection, are delivered by step three and the already-landed [Generation Read Path First Slice](integration/generation_read_path_first_slice.md).
6. Observation widens: the sensory promotion contract replaces the genesis substitute at the first coupling, claim re-entry closes the outer knowledge loop, and the measurement-gated deferred programs — spine compaction, causation, regime — activate on metrics the harness user projection is the instrument for.

Why this order: the instrument precedes the ignition because the ignition corrections must be validated live rather than reported second-hand; ignition precedes Strategy because Strategy develops inside the harness against a turning loop; and everything measurement-gated waits because its own activation criteria are observations only the harness can produce.

Flywheel docs freshness must run the existing docs writer package over a selected branching workspace tree, preserve sibling fan-out and child-finalization-before-parent-preparation dependencies, materialize one `README.md` per actionable folder, and advance package progress through bounded durable runtime turns. Earlier one-turn, synthetic-patch, activation-heavy, and production-closure plans are retained only as historical characterization.

## Historical Strategy

The original vertical slice threaded the thinnest possible path through every layer of the cognitive flywheel using the `docs_freshness` scenario. It targeted one complete flywheel turn before deepening any individual layer. That completion target is superseded by the current operational-parity authority above.

The typed loop proved the full contract chain through `meld-lang` pure types and operations. The historical vertical slice made each layer real with runtime code that materialized state, projected belief, curated goals, and dispatched work.

The former focused runtime wiring template is [Minimal Runtime Flywheel](integration/minimal_runtime_flywheel.md). It is historical characterization and not current implementation guidance.

## Historical Vertical Slice Implementation Inventory

This inventory preserves the sequence used by the earlier one-turn slice. It is evidence only and is not the current runtime-completion sequence.

### Phase 1: Shared Language — `complete`

Pure types and operations shared between world model and execution. `Term`, `Proposition`, `Condition`, `Effect`, `WorldState`, `Goal`, `Method`, `Composition`, `Operator`. Pure operations: `evaluate`, `unify`, `substitute`, `validate`, `apply_effects`. Full evaluation loop proven end-to-end in integration tests.

Owner: `meld-lang`
Plan: [meld-lang/PLAN.md](meld-lang/PLAN.md)

### Phase 2: World Model Graph — `complete`

Materialize current anchors from event spine facts. Subject identity lookup for a docs node, current anchor reads, provenance. Enough for belief to consume.

Owner: `meld-world-model`

### Phase 3: Belief Layer — `complete`

One belief family: `docs_freshness`. Graph anchor and promoted evidence normalization. One configured comparator that produces a confidence value. One compact planner-facing belief view. First layer that exercises epistemic judgment.

Owner: `meld-world-model`
Blocked by: Phase 2
Plan: [world_model/belief/PLAN.md](world_model/belief/PLAN.md)

### Phase 4: Planner Projection — `complete`

Convert public graph and belief views into ground `meld-lang::WorldState`. First projection emits `Proposition::Holds` for confidence, stale state, observation-needed state, and `Proposition::Accessible` for graph scope. Bridge that makes world model state consumable by execution.

Owner: `meld-world-model`
Blocked by: Phase 3

### Phase 5: Agent Goal Curation — `complete`

One seed agent, one perspective, one curation rule: if `docs_freshness` confidence is below 0.7, emit a proposed `AgentGoalCommand` carrying a ground `Goal` requiring confidence above 0.7. First point where the system generates operational intent from belief.

This phase makes seed agent authority explicit. Dynamic spawned agents through curated `CreateAgent` goals and process restart activation remain deferred.

It lands the minimal runtime surface: durable seed record, subscription cursor, curation decision record, goal command dedupe key, and read query facade.

Owner: `meld-world-model`
Blocked by: Phase 4

### Phase 6: Execution Planning Runtime — `first slice implemented`

Method library loading. Goal evaluation against `WorldState`, method matching via `unify`, composition preparation via `substitute` and `validate`. Bridges from typed planning substrate to runtime orchestration. Task network execution is deferred. First slice stops at an execution composition artifact.

Progress landed in `meld-execution`: execution goal contracts, in memory goal store, durable `PersistentGoalSetStore`, method library loading, planning result contracts, and one goal planning runtime.

Owner: `meld-execution`
Depends on: Phase 5

### Phase 7: Task Dispatch and Outcome — `first slice implemented`

This phase historically bridged execution composition to task network commands and the existing task and capability engine. It dispatched one task, published outcome events to the spine, and proved one flywheel turn. That completion claim is superseded by the current operational-parity authority.

Owner: `meld-execution`
Depends on: Phase 6 first slice
Plan: [execution/task_network/PLAN.md](execution/task_network/PLAN.md)

### Phase 8: Expanded Execution Slice — `implemented`

Mature task network execution before sensory work. Lower one execution composition into a multi node task graph, commit it atomically, materialize task init payloads from static seeds and upstream artifacts, dispatch real task runs, and replay the accepted graph state across reopen.

Owner: `meld-execution`
Depends on: Phase 7 first slice and hardening gates
Plan: [execution/task_network/PHASE8.md](execution/task_network/PHASE8.md)

### Phase 9: Sensory — `deferred`

Diff-native observation for the docs node. Publishes to the event spine. Closes the loop after execution can run and replay a real multi node graph.

Owner: sensory domain
Depends on: Phase 8 implemented expanded execution slice and event spine contract

## Foundation

These components predate the vertical slice and support all phases.

| Component | Status | Owner |
|---|---|---|
| `events` ledger mechanics | complete | `meld-events` |
| event foundation closeout | closed 2026-07-12 | `meld-events` with root `meld` |
| product event authority cutover | complete, E5 closed 2026-07-12 | `events` with root `meld` |
| production cognitive runtime closure | superseded for current runtime completion | root `meld` with domain crates |
| `integration/typed_loop` | complete | `meld-lang` integration tests |

Foundation work follows this order:

```text
event authority and observability hardening
-> product CLI authority cutover
-> events foundation closed
-> operational parity through the current runtime completion ground map and workstreams
```

## Dependency Order

1. `events` ledger mechanics — complete
2. event authority and observability hardening — complete, E1 through E4
3. product event authority cutover — complete, E5 closed 2026-07-12
4. event foundation closure — closed 2026-07-12, E6
5. runtime completion ground map and implementation workstreams — current operational-parity authority
6. `meld-lang` — complete, Phase 1
7. `world_model/graph` — complete, Phase 2
8. `world_model/belief` — complete, Phase 3
9. `world_model/planner` — complete, Phase 4
10. `world_model/agent` — complete, Phase 5
11. `execution/goals` — first slice implemented, Phase 6
12. `integration/typed_loop` — complete
13. `execution/planning` — first slice implemented, Phase 6
14. `execution/task_network` — first slice implemented, Phase 7
15. `execution/task_network/expanded` — implemented, Phase 8
16. `sensory` — deferred, Phase 9
17. `world_model/causation` — deferred past vertical slice
18. `world_model/regime` — deferred past vertical slice
19. `world_model` — full integration deferred
20. `execution` — full integration deferred
21. runtime completion with initialization and emission — current operational-parity authority
22. composition-path parity — chartered as Workstream Eight under runtime completion, gating Strategy end-to-end execution
23. world-model Strategy first slice — pure-language track unblocked now, integration track per-workstream gated per the Strategy ground map
24. agent-native debugger — requirements now, isolate prototyping per registration set, full sessions after runtime completion

## Implementation Plans

Implementation plans decompose assessed areas into phased, dependency-ordered work with tasks, exit criteria, and verification commands.

- [meld-lang/PLAN.md](meld-lang/PLAN.md) — complete
- [world_model/belief/PLAN.md](world_model/belief/PLAN.md) — complete
- [world_model/planner/PLAN.md](world_model/planner/PLAN.md) — complete
- [world_model/agent/PLAN.md](world_model/agent/PLAN.md) — complete
- [execution/planning/PLAN.md](execution/planning/PLAN.md) — first slice implemented
- [execution/task_network/PLAN.md](execution/task_network/PLAN.md) — first slice implemented
- [execution/task_network/PHASE8.md](execution/task_network/PHASE8.md) — implemented
- [events/event_foundation_closeout_program.md](events/event_foundation_closeout_program.md) - closed event foundation closeout
- [integration/runtime_completion_ground_map.md](integration/runtime_completion_ground_map.md) - current operational-parity runtime completion authority
- [integration/runtime_completion_implementation_workstreams.md](integration/runtime_completion_implementation_workstreams.md) - current build-facing workstream decomposition
- [world_model/strategy/ground_map.md](world_model/strategy/ground_map.md) - current Strategy primitive inventory, concept-to-ground map, and construction delta
- [integration/agent_native_debugger_requirements.md](integration/agent_native_debugger_requirements.md) - requirements gathering for the agent-native runtime debugger, gated by runtime completion
- [integration/runtime_harness_plan.md](integration/runtime_harness_plan.md) - phased delivery of the interactive runtime harness over the three-customer information model
- [integration/runtime_initialization.md](integration/runtime_initialization.md) - staged initialization contract owning theory installation, identity genesis, and epistemic seeding
- [integration/pds_theory_runtime_layer.md](integration/pds_theory_runtime_layer.md) - implementation-readiness map of the settled PDS layer: theory kinds, seams, and the layer-settlement rule
- [integration/production_cognitive_runtime_closure_program.md](integration/production_cognitive_runtime_closure_program.md) - superseded production runtime closure proposal
- [integration/production_cognitive_runtime_closure_delivery_ledger.md](integration/production_cognitive_runtime_closure_delivery_ledger.md) - superseded production-closure delivery ledger
- [integration/minimal_runtime_flywheel.md](integration/minimal_runtime_flywheel.md) — historical one-turn template
- [integration/durable_runtime_first_slice.md](integration/durable_runtime_first_slice.md) - historical first-slice design evidence
- [integration/durable_runtime_pre_implementation_gaps.md](integration/durable_runtime_pre_implementation_gaps.md) - historical pre-assembly gap evidence
- [integration/durable_flywheel_runtime_phase_design.md](integration/durable_flywheel_runtime_phase_design.md) - historical phase design
- [integration/flywheel_runtime_code_assessment.md](integration/flywheel_runtime_code_assessment.md) - historical code assessment
- [integration/runtime_supervisor_domain_plan.md](integration/runtime_supervisor_domain_plan.md) - historical supervisor design evidence
- [integration/runtime_requirements.md](integration/runtime_requirements.md) - current domain requirement index with superseded vertical links
- [integration/runtime_phase_design_detail_audit.md](integration/runtime_phase_design_detail_audit.md) - historical phase detail audit
- [integration/product_runtime_assembly_requirements.md](integration/product_runtime_assembly_requirements.md) - completed E5 assembly evidence with continuation authority moved
- [integration/product_event_authority_cutover.md](integration/product_event_authority_cutover.md) - complete E5 CLI and product event authority cutover
- [integration/docs_freshness_physical_configuration_requirements.md](integration/docs_freshness_physical_configuration_requirements.md) - superseded activation-heavy configuration proposal
- [integration/docs_freshness_flywheel_next_iteration_report.md](integration/docs_freshness_flywheel_next_iteration_report.md) - superseded one-turn iteration report
- [integration/docs_freshness_flywheel_domain_spec_skeleton.md](integration/docs_freshness_flywheel_domain_spec_skeleton.md) - superseded one-turn domain skeleton
- [integration/docs_freshness_flywheel_implementation_guide.md](integration/docs_freshness_flywheel_implementation_guide.md) - superseded one-turn implementation guide
- [integration/world_model_runtime_requirements.md](integration/world_model_runtime_requirements.md) - proposed world model runtime requirements
- [integration/execution_runtime_requirements.md](integration/execution_runtime_requirements.md) - proposed execution runtime requirements
- [integration/event_runtime_requirements.md](integration/event_runtime_requirements.md) - proposed event runtime requirements
- [integration/supervisor_runtime_requirements.md](integration/supervisor_runtime_requirements.md) - historical supervisor contract evidence
- [integration/durable_flywheel_vertical_proof_requirements.md](integration/durable_flywheel_vertical_proof_requirements.md) - historical synthetic one-turn proof requirements

## Assessment Inventory

- [integration/runtime_completion_ground_map.md](integration/runtime_completion_ground_map.md) - assessed runtime ground map for collaborative completion planning
- `design/plan/events/assessment.md`
- `design/plan/meld-lang/assessment.md`
- `design/plan/world_model/graph/assessment.md`
- `design/plan/world_model/belief/assessment.md`
- `design/plan/world_model/planner/assessment.md`
- `design/plan/world_model/agent/assessment.md`
- `design/plan/execution/goals/assessment.md`
- `design/plan/integration/typed_loop.md`
- `design/plan/integration/product_event_authority_domain_assessment.md`
- `design/plan/execution/planning/assessment.md`
- `design/plan/sensory/assessment.md`
- `design/plan/world_model/causation/assessment.md`
- `design/plan/world_model/regime/assessment.md`
- `design/plan/world_model/assessment.md`
- `design/plan/execution/assessment.md`

## Scope Cuts

The vertical slice excludes full causal effect summaries, regime sensitivity summaries, broad risk envelopes, multi-agent divergence, learned normative policy, multi-agent coordination, goal conflict resolution, broad utility estimation, recursive sub-goal lowering, plan diffing, sensory runtime, and switching cost model.

This historical phase inventory implemented the minimum needed for one `docs_freshness` flywheel turn. Current completion instead requires operational parity with the branching docs writer workflow through bounded durable execution.

## Blocked Areas

- recursive sub-goal lowering
- plan diffing
- sensory runtime
- switching cost model
- workflow integration strategy
- causal effect estimation
- regime inference

## Assessment Shape

Each assessment includes:

- verdict summary
- conceptual correctness
- completeness
- boundary clarity
- dependency readiness
- first-slice feasibility
- current implementation evidence
- gaps
- open questions
- recommendation

## Validation Rules

Current plan files must avoid hidden chronology and stale state language. Historical files must carry an explicit supersession notice and link to current authority.

Plan files must avoid literal parenthesis characters in Markdown prose.

Every readiness verdict must name its exact scope.

Every deferred runtime concern must be explicit.
