# Expected Injection Baseline

Date: 2026-08-23

Status: independent requirements evidence

## Injection Point

The Startup PDS is designed for the first integrated runtime proof after the implementation equivalents of `WMR-DD-01` through `WMR-DD-06` exist and before Docs Freshness or Dependency Security is relied upon as a live product proof.

This is not the current codebase. It is the codebase expected at the point of injection. Current source files remain useful implementation anchors, but no Startup PDS requirement may preserve a current shortcut that the accepted reconciliation design replaces.

## Required Baseline Products

| Accepted design source | Required implemented baseline at injection | Startup PDS use |
| --- | --- | --- |
| `WMR-DD-01` | owner-qualified publication, neutral Event append, owner-routed Graph admission, occurrence-preserving projection, and immutable `TraversalCut` | publish and discover the exact nonce Event without inventing Event semantics in Graph |
| `WMR-DD-02` | standing and planned Curation over one bounded operation grammar, durable terminality, semantic publication, Graph visibility, and configured Belief settlement | establish the expected nonce, assess non-realization, and later confirm realization |
| `WMR-DD-03` | complete `PlannerCut`, immutable heterogeneous `StrategyPlan`, Agent Plan judgment, independent Task and Epistemic Operation authorization, milestone absorption, and Goal disposition | construct one Task plus one verification operation and let Agent own progression |
| `WMR-DD-04` | complete Task intake, Goal-attributed admission, one unified Task Network, fenced attempt, outcome, owner return, and independent semantic milestones | execute the nonce emitter without teaching Execution startup meaning |
| `WMR-DD-05` | exact product compilation, owner installation receipts, finite Agent topology, Agent genesis, Capability preparation, participant plan, and inert prepared closure | install and prepare one Startup Agent and its exact theory |
| `WMR-DD-06` | authoritative generation and admission epoch, participant parity, owner readiness, work and wait closure, recovery, replacement, and retirement | run one nonce for each open admission epoch after structural readiness |
| `WMR-DD-07` | read-only inspection over exact native owner positions | expose the deepest nonce position and first missing successor |

## Baseline Invariants

The activation generation is structurally current and its admission epoch is open before ordinary Startup Agent work becomes eligible. The canary cannot be part of the structural readiness barrier that permits its own execution.

Agent owns Goal inception, Plan judgment, product authorization, milestone absorption, and Goal disposition. Strategy is pure construction. Curation owns expected entities and epistemic relations. Execution sees only one complete authorized Task at a time. Events remains neutral carriage. Graph and Traversal remain structural knowledge access. Belief admits only configured evidence. Root lifecycle aggregates structural receipts without interpreting the nonce.

The Task Network is unified. The Startup PDS does not create a private executor, private event loop, private graph path, direct Agent callback, or startup-only scheduling lane.

## Current Anchors And Expected Replacements

| Current anchor | Current limitation | Expected injection form |
| --- | --- | --- |
| [`PdsPackageManifestV1`](../../../../../src/theory/package.rs) and [`TheoryRouter`](../../../../../src/theory/router.rs) | package mechanics are substantial, but the complete post-reconciliation owner route set is absent | exact product compilation and native-owner receipts from `WMR-DD-05` |
| [`StewardshipDeclaration`](../../../../../src/config/stewardship/selection.rs), [`StewardshipAssignmentV1`](../../../../../src/config/stewardship/assignment.rs), and [`StewardshipActivationV1`](../../../../../src/config/stewardship/activation.rs) | current declaration and activation shapes are narrower than the accepted product, topology, and prepared-closure design | one product revision, situated assignment, one-Agent topology, and activation inputs |
| [world initialization pipeline](../../../../../src/init/world/pipeline.rs) | Agent operational status currently precedes complete structural readiness | Agent genesis followed by accepted lifecycle realization and current publication |
| [startup activation store](../../../../../src/runtime/activation.rs) | current substrate is disconnected from production preparation and supervision | one authoritative generation and admission epoch account |
| [runtime self observation](../../../../../src/runtime/self_observation.rs) | negative operational threshold Events do not prove a positive cognitive round trip | independent operational evidence retained beside the nonce proof |
| [current Graph reducer](../../../../../crates/meld-world-model/src/world_state/graph/reducer.rs) | current admission allowlist rejects the proposed nonce owner publication | owner-routed admission under `WMR-DD-01` |
| [current Agent curation path](../../../../../crates/meld-world-model/src/agent/curation.rs) | current behavior emits Goal commands rather than realizing first-class bounded Curation | standing and planned Curation from `WMR-DD-02` and `WMR-DD-03` |
| [current Strategy](../../../../../crates/meld-world-model/src/strategy.rs) | current candidate shape is not a heterogeneous persistent Plan | immutable `StrategyPlan` with Task and Epistemic Operation products |
| [current Execution Goal Set](../../../../../crates/meld-execution/src/goals.rs) | current authorization and Goal cardinality predate corrected Task intake | complete Goal-attributed Task admission from `WMR-DD-04` |

## Prohibited Compatibility Assumptions

Legacy Workflow is not a Startup PDS implementation surface. Current Agent-to-Execution curation adapters are not preserved as the target path. Process-local actor order is not a correctness mechanism. A clean supervisor tick is not nonce completion. Event append is not Agent receipt. Execution success is not Event realization. Graph presence is not Belief settlement. Belief settlement is not Agent acceptance. Agent acceptance is not whole-runtime quiescence.

If the implementation baseline lacks one required accepted contract, Startup PDS work waits on that baseline. It must not create a private substitute.
