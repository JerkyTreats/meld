# Post-VC-04 Retirement Assessment

Date: 2026-09-02

Status: accepted assessment evidence; program amendment required before later source activation

Concern: determine which incumbent authorities must be retained, moved, or deleted as the WMR source program advances from VC-05 through Startup and the two primary product migrations.

Implementation, source activation, gate freezing, commit, optional Startup gating, and generalized Workflow deletion are outside this assessment.

## Evidence Basis

The domain universe comes from the current source delivery ledger, the accepted WMR design index, root module ownership, crate roots, active Runtime Invariants, and current code one dependency hop from each later product path.

Discovery used the canonical bounded two-pass assessment. Confidence is high for ownership and route shape. Confidence is medium for exact stored-data compatibility and the independently supported portion of legacy Workflow.

## Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Retirement follow-up |
| --- | --- | --- | --- | --- |
| PDS and theory | own | package routing and partial owner installation | partial | disposition current selection and installation authority in VC-05 |
| workspace and sensory owners | publish | canonical workspace publication | complete for workspace | reuse unchanged |
| Events | publish | canonical append and replay | complete | reuse unchanged |
| Graph and Traversal | publish | canonical owner projection and cut query | complete | reuse unchanged |
| Belief | publish | configured ingestion and revision storage | complete | reuse unchanged |
| Causation | none | no current source domain | not needed | do not create for these slices |
| Regime | none | no current source domain | not needed | do not create for these slices |
| Planner | consume | accepted complete-cut route | complete | reuse unchanged |
| Curation | consume | accepted standing and planned authority | complete | reuse unchanged |
| Strategy | consume | accepted heterogeneous Plan route | complete | reuse unchanged |
| Agent | own | canonical reconciliation with partial genesis | partial | extend genesis, subscriptions, and epoch relationships only |
| Execution | consume | accepted Task route and unified Task Network | complete after VC-04-R | reuse unchanged after retirement closes |
| Capability | own | contracts and invocation exist without complete preparation | partial | add preparation and product binding in VC-05 |
| Meld Language | none | pure values | not needed | no write scope |
| runtime lifecycle | own | disconnected activation and lifecycle records | partial | replace with one generation authority in VC-06 |
| supervisor | consume | leases, health, stop, and flush exist | partial | retain process ownership and subordinate it to lifecycle |
| root initialization and composition | adapter | world-init pipeline and inferred registrations | partial | move semantic coordination behind owner contracts |
| inspection | observe | runtime snapshots and harness diagnostics | partial | extend as read-only proof projection |
| Docs Freshness | own | executable correction chain | partial | add owner observation and returned evidence in VC-08 |
| Dependency Security | own | fixture-backed private route | partial | migrate to common runtime seams in VC-09 |
| legacy Workflow | own | live CLI, root, control, context, and Execution routes | partial | classify exact retained product and superseded WMR overlap |
| CLI, API, and serve | adapter | current command and presentation routes | partial | change only when a canonical entrypoint moves |

## Frozen Affected Set

The affected set is PDS and theory, workspace and sensory owners, Events, Graph and Traversal, Belief, Planner, Curation, Strategy, Agent, Execution, Capability, runtime lifecycle, supervisor, root initialization and composition, inspection, Docs Freshness, Dependency Security, legacy Workflow, and CLI, API, and serve adapters.

Causation, Regime, and Meld Language have explicit `none` integration.

This is a runtime assessment set, not write authority. Events, Graph, Traversal, Belief, Planner, Curation, Strategy, accepted Agent reconciliation, Task Network, provider dispatch, workspace publication, and Meld Language remain source-unchanged unless direct evidence exposes an owner-scoped defect.

## Affected-Domain Decomposition

| Domain concern | Current ground | Required relationship | Change posture | Boundary risk |
| --- | --- | --- | --- | --- |
| product selection | `StewardshipDeclaration` and `TheorySelection` select a partial image | feed one complete principal-facing product declaration or retire | replace or extend | two compilation authorities |
| package linking and installation | package manifest and theory router install owners | produce complete package and compilation receipts | extend existing | root pipeline duplicates completion |
| Agent genesis and subscriptions | world-init directly creates a seed Agent and belief subscription | root emits intent while owners issue native receipts | move authority | durable request mistaken for completed genesis |
| Capability preparation | registry proves contracts and invokers | create inert compatibility and binding receipt | new local behavior | preparation mistaken for availability |
| lifecycle and generation | startup activation and portable lifecycle stores are disconnected | one lifecycle decision, head, and admission epoch authority | replace and consolidate | two generation truths |
| runtime registration | assembly infers factory descriptors or accepts another set | project the accepted participant plan exactly | replace selector | fixed catalog remains authoritative |
| supervisor process ownership | leases, health, stop, and flush exist | cite lifecycle incarnation and fences | extend existing | health mistaken for readiness |
| wait and quiescence | clean tick and no-work accounts can imply quiescence | aggregate owner checkpoints, waits, and resolvable wakes | replace projection | active idle mistaken for semantic completion |
| lifecycle retirement | stop and safe-point mechanics exist | close admission, fence, collect safe points, reverse stop, and receipt | new local behavior | shutdown mistaken for retirement |
| Startup product | no canonical nonce product exists | ordinary one-Agent PDS over accepted seams | new local behavior | Startup becomes a coordinator |
| Startup inspection | startup snapshots already exist | correlate native positions without creating truth | extend existing | diagnostics become authority |
| Docs observation | capabilities focus on correction and post-publication assessment | observe current README existence and required coverage before Task need | extend or replace | every reconciliation requires executable work |
| Docs return | VC-04 ends at operational terminal return | re-observe through Docs and workspace owner publication | new owner behavior | Execution success becomes correctness |
| dependency observation | domain uses fixture-backed evidence | publish real inventory, advisory, assessment, and verification products | replace fixture route | fixtures become product truth |
| dependency execution | private admission and portable lifecycle path exist | use canonical Agent, Execution, and lifecycle seams | replace and delete | second admission authority |
| Workflow overlap | explicit Workflow routes remain live | retain distinct product behavior or delete WMR-equivalent entrypoints | disposition required | broad deletion breaks unrelated behavior |

## Candidate Retirement Requirements

### WMR-VC-05 Product Compilation And Agent Genesis

Disposition must cover `StewardshipDeclaration`, `TheorySelection`, `WorldInitPipeline`, current theory installation receipts, direct seed Agent registration, direct belief subscription, descriptor-derived participant preparation, and their exports and exclusive tests.

The successor is one product declaration, complete package and compilation receipts, assignment, Agent-owned genesis and subscription receipts, Capability preparation receipt, participant plan, and inert prepared closure.

VC-05 remains assessment-only until it identifies which current selection and installation contracts survive, which callers migrate, and which source and records retire.

### WMR-VC-06 Activation Generation And Lifecycle Closure

Disposition must cover `StartupActivationStore`, `ActivationLifecycleStore`, independent assignment heads, inferred registration sets, `enabled_runtime_ids`, fixed handle catalogs, tick-level quiescence, and supervisor shutdown as terminality.

The successor is one lifecycle acceptance and generation journal, exact participant-plan projection, readiness barrier, atomic head and epoch publication, complete wait and wake accounting, recovery, replacement, fencing, and retirement receipt.

VC-06 remains assessment-only until one surviving store authority and any record migration are selected explicitly.

### WMR-VC-07 Startup Nonce Proof And Inspection

Startup is new product behavior over accepted seams. Existing startup snapshots and tick accounts remain operational diagnostics only. They cannot satisfy nonce proof, quiescence, or Agent belief.

The slice must name the reusable nonce Capability owner, ordinary product data, exact registration, and native proof projection. It must not modify shared WMR owners merely because the proof traverses them.

### WMR-VC-08 Docs Freshness Migration

Disposition must cover executable-only freshness proof, post-publication assessment assumptions, direct or fixture package installation, test-only hand-composed Docs PDS data, and every overlapping Workflow entrypoint.

The successor observes current README existence and required claims before deciding that a Task is needed, executes only when reconciliation requires action, then re-observes through Docs and workspace ownership. Execution success cannot satisfy Docs correctness.

### WMR-VC-09 Dependency Security Migration

Disposition must cover fixture adapters, private dependency-security admission, fixture-truth execution, portable lifecycle use, direct package installation, exports, registrations, and exclusive tests.

The successor publishes native inventory, advisory, assessment, mitigation, successor observation, and verification through the common PDS, Agent, Execution, Event, and lifecycle seams. The private admission and portable execution authority must be deleted in the completed migration.

### Workflow Disposition

Before VC-08 or VC-09 becomes approval-ready, inventory root and `meld-execution` Workflow registries, CLI execution, context and control compatibility, task-package bridges, stores, exports, and tests.

Classify each surface as a separately supported explicit Workflow product, a read-only compatibility need, or a superseded WMR route. Do not create a generalized Workflow-retirement slice unless evidence shows independently retained behavior cannot be classified inside the two product migrations.

## Program Amendment

Replace the undifferentiated post-VC-04 backlog with these assessment-only candidates:

- `WMR-VC-05` product compilation and Agent genesis
- `WMR-VC-06` activation generation and lifecycle closure
- `WMR-VC-07` Startup nonce proof and native inspection
- `WMR-VC-08` Docs Freshness product migration
- `WMR-VC-09` Dependency Security product migration

Every candidate must add a responsibility disposition table before approval. Each table covers current entrypoint, incumbent authority, successor, callers, registrations, stores and record families, public exports, exclusive tests, deletion, stable reader retention, unchanged retention, and removal condition.

Every replacement gate must prove positively that real callers use the successor and negatively that superseded writers, planners, selectors, coordinators, registrations, stores, exports, and exclusive tests are unreachable or deleted.

Workflow classification is a prerequisite for the two product migrations. VC-04-R completion is a prerequisite for all later source activation.

## Authorization State

This assessment authorizes no later implementation. Separate program-owner authority committed and pushed the accepted `WMR-VC-04-R1` candidate. `WMR-VC-05` through `WMR-VC-09`, Workflow disposition source changes, and deployment remain unauthorized.
