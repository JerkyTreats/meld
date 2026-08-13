# Cognitive Architecture Implementation Plan

Date: 2026-08-12
Status: active
Scope: declarative implementation readiness and dependency order for the cognitive architecture

## Purpose

This directory defines implementation readiness, dependency order, scope cuts, and contract closure for the cognitive architecture.

Durable architecture intent lives under [design/cognitive_architecture](../cognitive_architecture/README.md). That directory is canonical for declarative design intent. This plan states what can be built, what must be built first, and what remains blocked.

Each assessment named as current authority states current truth. Historical and superseded artifacts preserve dated evidence but must not direct implementation.

When this plan conflicts with `design/cognitive_architecture`, treat the architecture document as intent and this plan as an implementation readiness snapshot that needs reconciliation.

## Current Authority

The active forward objective is the [Theory Elevation Program](integration/theory_elevation_program.md): elevate the hand-lowered docs stewardship image into durable installed theory, remove root expression dispatch, and prove runtime agnosticism through a second dissimilar expression.

The runtime completion program it succeeds is delivered. The [Runtime Completion Ground Map](integration/runtime_completion_ground_map.md) and [Runtime Completion Implementation Workstreams](integration/runtime_completion_implementation_workstreams.md) closed with every wave and the flywheel-ignition lane complete, and the Strategy first slice landed the dynamic loop: bounded candidate search, Agent authorization, authorized realization with no Method or workflow route, claim-validated publication, and evidence-driven settlement to quiescence.

The observation substrate delivered under the [Runtime Harness Plan](integration/runtime_harness_plan.md) serves the frozen [Agent-Native Debugger Requirements](integration/agent_native_debugger_requirements.md); its phase four consumer contract remains open. The standing storage posture behind that substrate is the [Storage Substrate Decision Record](integration/storage_substrate_decision_record.md).

## Current Delivery Order

The order below is the active program sequence as of 2026-08-12, grounded in the strategy landing. The prior sequence — instrument first, ignition through the instrument, Strategy inside the harness — completed: harness phases one through three closed with the three-altitude gate passed, the ignition lane closed, and the Strategy first slice landed the dynamic loop with a bounded convergence to quiescence and claim-validated true publication. What orders the work now is a single fact: the runtime kernel is domain-agnostic but the docs theory package is hand-lowered Rust composed from root, so widening to any second domain is blocked on theory elevation, not on machinery.

1. Theory elevation steps one and two under the [Theory Elevation Program](integration/theory_elevation_program.md): durable theory registries following the belief-family pattern, then declaration lowering that removes the root expression dispatch. These convert the docs stewardship image from compiled constants into installed theory.
2. Parity reassessment: the [Flywheel Parity Workstream](integration/flywheel_parity_workstream.md) resumes from its Strategy pause, re-measuring comparator calibration and the satisfaction threshold against the rewritten docs theory data, and recording the live provider runs its slice seven still lacks.
3. Standing maintained condition and authority contracts, elevation steps three and four, landing the runtime seams the stewardship corpus requires before any declaration layer may depend on them.
4. The second expression proof: CVE freshness as the designed inverse domain, rejected as a generality proof if it needs any expression-named root branch. This is the evidence gate before stewardship schemas freeze.
5. Settled replay and strategy promotion, closing the elevation program. The [Multiplier Harness Program](integration/multiplier_harness_program.md) proceeds in parallel once its Ollama validation joins; its other dependencies are delivered. Harness phase four, the published consumer contract, proceeds in parallel as capacity allows.
6. Observation widens: the sensory promotion contract replaces the genesis substitute at the first coupling, claim re-entry closes the outer knowledge loop, and the measurement-gated deferred programs — spine compaction, causation, regime — activate on metrics the harness user projection is the instrument for.

Why this order: registries precede declaration lowering because lowering needs somewhere durable to land; seams precede the second expression because the CVE semantics require maintained conditions and authority the docs slice never exercised; and the second expression precedes schema freezing because the stewardship corpus requires dissimilar-expression evidence before its package model hardens.

Flywheel docs freshness now composes its five-capability chain dynamically from the stewardship catalog with no Method or workflow route; the docs writer package path and its earlier one-turn, synthetic-patch, activation-heavy, and production-closure plans are retained only as historical characterization.

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
5. runtime completion ground map and implementation workstreams — delivered, all waves and the ignition lane closed
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
21. runtime completion with initialization and emission — delivered
22. composition-path parity — Workstream Eight closed 2026-07-25
23. world-model Strategy first slice — implemented, scope in [Strategy Minimal Slice Requirements](world_model/strategy/minimal_slice_requirements.md), cross-domain ownership in [Strategy Assessment By Domain](world_model/strategy/assessment_by_domain.md), and delivery evidence in [Strategy Ground Map](world_model/strategy/ground_map.md)
24. agent-native debugger — register frozen, exit conjuncts discharged with harness phase three and the Strategy landing, phase four consumer contract open
25. theory elevation — current forward authority in [Theory Elevation Program](integration/theory_elevation_program.md)

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
- [integration/theory_elevation_program.md](integration/theory_elevation_program.md) - current forward authority: theory registries, declaration lowering, maintained conditions, authority, second expression proof, settled replay
- [integration/theory_durability_symmetry_workstream.md](integration/theory_durability_symmetry_workstream.md) - Step 1 workstream ready for phased implementation with frozen theory inventory, acceptance criteria, rejection criteria, and verification gates
- [integration/theory_durability_symmetry_domain_assessment.md](integration/theory_durability_symmetry_domain_assessment.md) - initial Step 1 domain sweep retained as assessment history
- [integration/theory_durability_symmetry_contract_coherence_assessment.md](integration/theory_durability_symmetry_contract_coherence_assessment.md) - focused contract-reuse audit and refreshed cross-domain coherence assessment
- [integration/theory_durability_symmetry_implementation_design.md](integration/theory_durability_symmetry_implementation_design.md) - dependency-ordered Step 1 contracts, storage, installation, runtime resolution, lineage, compatibility, and verification design
- [integration/runtime_completion_ground_map.md](integration/runtime_completion_ground_map.md) - delivered runtime completion authority, retained as ground evidence
- [integration/runtime_completion_implementation_workstreams.md](integration/runtime_completion_implementation_workstreams.md) - delivered workstream decomposition, all waves closed
- [integration/flywheel_parity_workstream.md](integration/flywheel_parity_workstream.md) - everything-up-to-Strategy workstream, resuming from the Strategy pause for calibration re-measurement and live-run evidence
- [integration/silent_success_findings.md](integration/silent_success_findings.md) - silent-success bug-class findings register with open residuals F3, F7, F8, F9, and F11
- [integration/gate_signal_first_slice.md](integration/gate_signal_first_slice.md) - completed slice making gate outcomes recorded signals with bounded retry on the package route
- [world_model/strategy/ground_map.md](world_model/strategy/ground_map.md) - Strategy primitive inventory and construction delta, delivered
- [integration/agent_native_debugger_requirements.md](integration/agent_native_debugger_requirements.md) - frozen debugger register with exit conjuncts discharged
- [integration/runtime_harness_plan.md](integration/runtime_harness_plan.md) - phased delivery of the interactive runtime harness over the three-customer information model
- [integration/runtime_initialization.md](integration/runtime_initialization.md) - staged initialization contract owning theory installation, identity genesis, and epistemic seeding
- [integration/pds_theory_runtime_layer.md](integration/pds_theory_runtime_layer.md) - implementation-readiness map of the settled PDS layer: theory kinds, seams, and the layer-settlement rule, continued by the theory elevation program
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
- [integration/world_model_runtime_requirements.md](integration/world_model_runtime_requirements.md) - superseded world model runtime requirements
- [integration/execution_runtime_requirements.md](integration/execution_runtime_requirements.md) - superseded execution runtime requirements
- [integration/event_runtime_requirements.md](integration/event_runtime_requirements.md) - delivered event runtime foundation requirements
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
