# World Model Reconciliation Design And Documentation Alignment Assessment

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-08-23

Status: historical architecture alignment evidence, superseded for source sequencing

Concern: determine whether canonical cognitive architecture, accepted World Model Reconciliation design, Startup requirements, source delivery controls, and implementation-facing contracts describe one realizable architecture before source work begins.

## Scope

This pass checks ownership, product identity, terminology, runtime connectivity, source authority, delivery state, gate evidence, and document links across the accepted architecture.

It does not inspect source implementation correctness, redesign accepted owner boundaries, select fail-closed Startup policy, activate Docs Freshness or Dependency Security, or implement runtime behavior.

Primary evidence is the [canonical cognitive architecture](../../../cognitive_architecture/README.md), [accepted Startup manifest](reviews/world_model_reconciliation_startup_integration_candidate.sha256), [WMR-DG-04 revision 2 receipt](delivery_gates/wmr_dg_04_revision_2_acceptance_receipt.md), [WMR-DG-07 revision 3 receipt](delivery_gates/wmr_dg_07_revision_3_acceptance_receipt.md), [Startup design](startup_pds_design_requirements/startup_pds_design_specification.md), [handoff ledger](world_model_reconciliation_handoff_ledger.md), and [source Style Assurance overlay](delivery_gates/wmr_source_style_assurance_overlay.md).

## Recovery Supersession

The semantic alignment findings remain evidence for the approved architecture. The former conclusion that the source sequence was implementation-ready is withdrawn.

The additive implementation attempt demonstrated that locally aligned domain contracts can still create parallel runtime authorities when the real entrypoints and superseded paths are deferred. The [single-runtime recovery amendment](world_model_reconciliation_single_runtime_recovery.md) and [Runtime Invariants](../../../../governance/runtime_invariants.md) now control delivery reconstruction. No source slice is active.

## Authority Order

The evergreen cognitive architecture defines what Meld shall be. Accepted WMR revisions supply implementation-level detail where they preserve those seams. Gate Receipts establish accepted candidate state. Later activation and policy overlays may change delivery authority without rewriting the accepted manifest. Historical discovery and review artifacts remain evidence, not current normative contracts.

This distinction matters because the accepted manifest intentionally preserves pre-acceptance status prose. Current program authority lives in the [source delivery program ledger](world_model_reconciliation_source_delivery_program_ledger.md). The former [WMR-SI-01 activation record](delivery_gates/wmr_si_01_source_activation_record.md) is withdrawn historical evidence.

## Pass One Domain Sweep

| Domain | Needed integration | Current design integration | Completeness | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- |
| PDS | publish | supplies product theory, assignment, compilation, genesis, and activation lineage | complete | canonical PDS and Startup design | reuse unchanged |
| Sensory and product owners | publish | publish owner observations and semantic verdicts through Events | complete | cognitive architecture and product proofs | reuse unchanged |
| Events | own | neutral durable append, replay, authority, and cursor spine | complete | canonical Events and WMR handoffs | reuse unchanged |
| Graph and Traversal | own | admit owner products and provide bounded occurrence-rich reads | complete | canonical Graph and accepted `TraversalCut` design | reuse and extend accepted contracts |
| Belief | publish | settle configured Agent-scoped revisions from admitted evidence | complete after terminology correction | canonical Belief and Startup route | preserve semantic ownership |
| Causation | publish | provide exact mechanisms, effects, and assumptions to Planner | complete | canonical Causation and `PlannerCut` inventory | reuse unchanged |
| Regime | publish | provide exact condition and stress revisions to Planner and Agent | complete after Goal-boundary correction | canonical Regime and `PlannerCut` inventory | preserve Agent authority |
| Planner | own | assemble one complete immutable reasoning cut | complete after specification replacement | Planner overview, corrected specification, accepted `WMR-DD-03` | implement `PlannerCut`, not retired Execution handoff |
| Curation | own | perform bounded Agent-authorized epistemic authorship | complete | canonical Curation and accepted `WMR-DD-02` | new local runtime behavior |
| Strategy | own | construct immutable heterogeneous Plans from frozen context | complete | canonical Strategy and accepted `WMR-DD-03` | new Plan behavior |
| Agent | own | own Goals, Plan judgment, product authorization, progression, and reconciliation | complete after naming and routing correction | canonical Agent and accepted `WMR-DD-03` | new durable progression behavior |
| Execution | consume | accept complete Goal-attributed Tasks and cohere them in one Task Network | complete after terminology correction | canonical Execution and accepted `WMR-DG-04` revision 2 | narrow intake and lowering changes only |
| Meld Language | publish | provide permissive Goal, Task, Capability, Method, and Composition values | complete | canonical language design | reuse unchanged |
| Runtime lifecycle | own | compose readiness, epochs, waits, wakes, fencing, recovery, and quiescence | complete | canonical lifecycle and accepted `WMR-DD-06` | implement structural composition only |
| Root composition and inspection | adapter | wire native owners and project their evidence without semantic absorption | complete | canonical core, Startup account, and `WMR-DG-07` | thin adapters only |

Frozen affected-domain set: PDS, Sensory and product owners, Events, Graph and Traversal, Belief, Causation, Regime, Planner, Curation, Strategy, Agent, Execution, Meld Language, runtime lifecycle, root composition, and inspection.

Every domain lies on the accepted product path. That does not make every domain a source write target.

## Pass Two Ownership Decomposition

| Domain concern | Owner | Current aligned contract | Change posture | Boundary risk |
| --- | --- | --- | --- | --- |
| theory, assignment, and genesis | PDS | inert owner-issued packages become one situated Agent generation | extend existing | PDS must not become runtime cognition |
| owner observation | Sensory and product domains | owner meaning enters through typed Event publication | reuse unchanged | root or Graph must not invent product truth |
| append and replay | Events | neutral envelope and durable cursors | reuse unchanged | Event append must not imply downstream acceptance |
| publication admission | Graph | owner-qualified objects and relation occurrences | extend existing | Graph must not settle belief |
| bounded discovery | Traversal | exact roots, relations, scope, frontier, and resource bounds | extend existing | Traversal must not author graph meaning |
| evidence settlement | Belief | immutable configured revisions and explicit unresolved states | extend existing | settlement must not become Goal authority |
| causal and regime sources | Causation and Regime | exact native revisions inside `PlannerCut` | reuse unchanged | Planner must not infer missing owner semantics |
| cut assembly | Planner | complete `PlannerCut` or explicit refusal | new local behavior | no `WorldModelView -> ExecutionPlanningInput` path |
| epistemic authorship | Curation | bounded accepted operation through terminal result and publication | new local behavior | Curation must not become Traversal mutation or Execution work |
| Plan construction | Strategy | immutable heterogeneous Plan with closed products | extend existing | Strategy must not know Task Network state |
| Goal and Plan progression | Agent | separate judgment, authorization, publication, milestones, and successors | new local behavior | consumer receipt must not imply semantic completion |
| executable intake | Execution Goal Set | producer-neutral Goal-attributed complete Task | extend existing | Execution must not reconstruct Strategy proof |
| coherence and lowering | Execution Planning | one unified Task Network with exact compatibility and attribution | extend existing | shared work must not merge Goal authority |
| nonce publication | nonce owner | fixed `nonce.emit.v1` grammar and deterministic Event | new local behavior | no arbitrary Event Capability |
| lifecycle composition | root and native owners | structural readiness, epoch, fencing, waits, wakes, and safe points | extend existing | nonce satisfaction must not become bootstrap readiness |
| inspection | read-only projection | correlate exact native positions and uncertainty | adapter only | projection must not synthesize truth |

## Findings And Corrections

### Planner Contract Conflict

The linked Planner specification still defined `WorldModelView` as the canonical root, exported `ExecutionPlanningInput`, and projected Active Goals directly toward mutation. That contradicted the Planner README, accepted `WMR-DD-03`, the sacred Strategy and Execution seam, and the immutable `PlannerCut` lineage required by Startup.

The specification now defines `PlannerCut` as the canonical reasoning root, retains `WorldModelView` only as a derived projection, makes refusal explicit, and forbids Planner products from entering Execution.

### Agent Curation Name Collision

The canonical Agent package still used Goal Curation as a document and link name even though Curation is now the separate epistemic authorship domain. Directive grounding also implied that a Goal becomes real only when Strategy produces an executable candidate for Execution.

The document is now Agent Plan Progression. Directive grounding now permits epistemic-only and already-satisfied Plans and routes Tasks and Epistemic Operations through their separate consumers.

### Belief And Regime Boundary Drift

Belief examples still used the flywheel metaphor and Goal Curation label. The comparator document also suggested that Execution could synthesize or register a Capability for missing evidence.

The canonical vocabulary now uses reconciliation loop and Agent Goal judgment. Belief emits an observation opportunity. Strategy may select an admitted Capability into a complete Task. Capability provisioning remains owner-controlled, and Execution does not invent semantic work.

### Execution Terminology Drift

Execution docs used Curation as a generic intake verb, named legacy Workflow package triggers as Task initialization sources, and referred to a planning agent committing Task Network subgraphs.

The corrected language names Task admission lifecycle, producer-bound Task inputs, Execution-owned lowering values, and Execution Planning. Conditional execution remains legal only inside a complete admitted Task.

### Historical Program State Drift

The frozen revision 3 candidate correctly records its own pre-acceptance state, but later receipts and user authorization made that prose stale as a current program index. Rewriting the frozen candidate would destroy exact acceptance identity.

The first source activation record incorrectly made Startup the active source slice. That confused first integrated product proof with first substrate implementation. A later correction made owner publication through immutable `TraversalCut` the first horizontal slice and deferred the remaining domains.

That correction was still insufficient. It allowed successor runtime surfaces to accumulate while incumbent planners, actors, Goal adapters, Execution routes, lifecycle assembly, and Workflow control remained live. The complete horizontal source sequence is now withdrawn. Startup, Docs Freshness, and Dependency Security remain required proofs, but none currently has a source-slice identifier.

## Separated Scopes

The runtime path includes every domain in the frozen affected set.

Behavior expected to change across the complete source program spans PDS realization, world-model Graph and Traversal, Belief routes, Planner, Curation, Strategy, Agent, narrow Execution intake and lowering, nonce publication, root lifecycle composition, and inspection.

There is no active write scope. The next assessment must inventory the complete live route for each affected responsibility and state whether that route remains canonical or will be replaced or moved. A replacement cut must migrate the real entrypoint and delete its superseded authority in the completed change.

## Current Readiness Judgment

The architecture describes a coherent intended product path and retains approval. The prior claim that only implementation remained was too strong because it did not bind intended ownership to one live runtime route.

Source readiness is `assessment-only`. Confidence remains high for semantic owner boundaries, product cardinality, Planner and Strategy separation, Agent authority, Execution ignorance, Startup epoch identity, and lifecycle intent. Confidence is not yet sufficient for source slicing, canonical entrypoints, retirement scope, storage compatibility, or the exact cutover order.

The next correct task is the read-only canonical runtime groundmap named by the source delivery ledger. A reconstructed source program becomes build-ready only after it identifies complete vertical cutovers and receives explicit user approval.
