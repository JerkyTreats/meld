# Initialization And Genesis Review

Date: 2026-08-20

Status: discovery evidence, implementation not authorized

## Question

This review asks whether current Meld can establish one exact World Model Reconciliation generation from installed PDS meaning through assignment, activation, world initialization, Agent registration, subscriptions, runtime assembly, participant readiness, and current-generation publication.

The success condition is stronger than successful construction of each local record. Every required producer must leave a durable product that its consumer can resolve, verify, and resume. A generation may become current only after its complete participant set has reconstructed the same exact semantic and physical position and each owner has published readiness in its own vocabulary.

In scope are package materialization, owner theory installation, assignment preparation, activation, world initialization, Agent genesis, subscription creation, runtime assembly, participant planning, readiness aggregation, current-generation publication, idempotency, restart, and pre-current failure.

Out of scope are steady-state reconciliation behavior, quiescence after publication, fencing, drain, retirement, exact future contract shapes, implementation sequencing, and changes to the semantics of product domains.

The review used the current repository, [Runtime Lifecycle And Quiescence](../../../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md), [Strategy Is Bigger Than A Task](../../strategy_plan_redesign.md), [PDS Design Framing After World Model Reconciliation](../../pds_boundary_assessment/pds_design_framing_after_world_model_reconciliation.md), [Agent Runtime Surface](../../../../../cognitive_architecture/world_model/agent/runtime_surface.md), and [Agent Genesis And Activation](../../../../../cognitive_architecture/world_model/agent/genesis_and_activation.md). The discovery budget was twelve batched inspections and was fully used. No history queries or runtime experiments were needed.

## Evidence Legend

Implemented fact means behavior present in current code or tests.

Canonical lifecycle intent means a boundary fixed by active architecture documentation.

World Model Reconciliation proposal means behavior introduced by the current redesign proposal and not yet implemented.

Inferred connective need means the smallest relationship required to make the implemented parts satisfy the canonical lifecycle. It is not an approved contract or design.

## Finding

The runtime primitives are individually credible, but initialization is not one lifecycle today.

Current Meld has four strong but disconnected constructions:

```text
PDS package installation

world initialization and Agent genesis

root runtime registration and supervision

activation generation publication
```

No production path proves that these constructions refer to the same package receipt, assignment, activation, participant plan, Agent topology, owner theory revisions, and runtime generation before work becomes live.

The present system can mark an Agent operational after one active belief subscription, start root runtime roles from configuration, and publish a startup generation from caller-supplied opaque readiness references. None of those milestones consumes the other two. This is the exact class of disconnected flywheel risk named in the charter.

The canonical lifecycle already supplies the correct framing. Initialization is a distributed preparation and readiness proof followed by one conditional live-authority publication. Root owns structural closure and aggregation. Native owners create and validate their own durable state. The missing work is connective rather than a replacement runtime.

Confidence is high for the current-code finding and medium-high for the exact affected set. Confidence is medium for future World Model Reconciliation participants because the Strategy Plan and Curation runtime products remain proposed.

## Implemented Ground Map

### Package And Theory Installation

The current [`TheoryRouter`](../../../../../../src/theory/router.rs) materializes a package, resolves exact imports, checks structural requirements, delegates validation to owner routes, installs owner revisions, verifies those revisions, validates cross-component links, writes an immutable exact receipt, and optionally advances a package head using an expected-prior fence.

Validation of every component precedes owner installation. If a later owner installation fails, earlier owner revisions remain durable but no complete package receipt or selectable head is created. Router tests prove that retry can reuse those owner revisions when the owner registries are content-idempotent.

The [`PdsPackageInstallationReceiptV1`](../../../../../../src/theory/receipt.rs) has a content-derived identity over the package, exact imports, and exact installed component revisions. Installation sequence is provenance but not receipt identity. This is a sound durable handoff from structural PDS linking to native theory owners.

### Assignment And Activation Preparation

[`StewardshipAssignmentV1`](../../../../../../src/config/stewardship/assignment.rs) has a content identity over package receipt, principal, Agent, subject, perspective, branch, requested authority, and principal grant. Physical bindings are correctly excluded.

[`StewardshipActivationV1`](../../../../../../src/config/stewardship/activation.rs) has a separate content identity over assignment, physical binding references, selected implementations, placement, isolation requirements, and operational limits. Secret values are correctly excluded.

[`PreparedActivationClosureV1`](../../../../../../src/theory/activation.rs) hashes the assignment, activation, exact package receipt and hash, owner preparation references, Capability closure reference, structural participant plan, binding revisions, and effective authority input references. Construction verifies assignment and activation identity, their mutual binding, owner receipt presence, Capability closure presence, participant uniqueness, dependency presence, and dependency acyclicity. The closure is explicitly inert.

This is a strong proposed preparation aggregate. Current production code does not construct it. Its construction sites are tests only.

### World Initialization And Agent Genesis

[`WorldInitPipeline`](../../../../../../src/init/world/pipeline.rs) is a separate three-stage command pipeline:

```text
install owner theory
-> create Agent identity and belief subscription
-> append epistemic genesis fact
```

The request may select any subset. Selected stages are normalized into fixed order. The pipeline stops at the first error and preserves completed owner writes. Repeating the same content is intended to be safe.

Owner theory installation is mostly idempotent by content identity. Routed installation produces an exact package receipt. The compatibility path writes an older aggregate theory receipt. Agent genesis resolves the current belief family, registers a seed Agent, binds Curation and maintained-condition revisions, creates one Agent belief subscription by natural key, and marks the Agent operational.

The operational transition checks only that at least one active subscription exists. It does not prove that the first belief view is readable, the planner projection succeeds, a Goal can be grounded, all World Model Reconciliation participants exist, or an activation generation is current. Those stronger checks are canonical in [Agent Genesis And Activation](../../../../../cognitive_architecture/world_model/agent/genesis_and_activation.md) but absent from the command.

The epistemic genesis Event is appended after the Agent is marked operational. A failure during the final stage therefore leaves an operational Agent without the genesis fact that is supposed to wake its first observation path. A later rerun can append the Event idempotently, but there is no durable aggregate initialization record that projects this state as incomplete.

Existing Agent identity handling also has an important asymmetry. Seed registration returns any existing record with the same Agent id. Genesis then migrates Curation and maintained-condition revisions, but it does not reconcile a changed subject, perspective, branch, observation scope, directive, provenance, or package and assignment lineage. The Agent can therefore retain a prior identity body while the command reports the current selected theory records.

### Runtime Assembly And Registration

[`ProductRuntimeAssembly`](../../../../../../src/runtime/assembly.rs) opens stores, builds direct handoff ports, derives or accepts a root registration set, and binds actor factories. It explicitly does not create semantic state. When genesis-dependent state is absent, factories remain truthfully unresolved.

[`RuntimeRegistration`](../../../../../../src/runtime/registration.rs) identifies root-local runtime roles and required resources. The supervisor uses these registrations to create operational handles and leases. Registration lifecycle distinguishes unresolved binding, starting, active work, active idle, unhealthy, and stopped.

These registration records are not generated from `ActivationParticipantPlanV1`. They do not carry activation generation, assignment, package receipt, owner readiness contract, owner wake contract, safe-point contract, or stop contract. Runtime assembly does not query a current activation generation before constructing or starting a semantic handle.

The activation lifecycle service is itself a registered runtime actor. Its current bounded step only records that an unprocessed intent is waiting for exact inputs. It does not prepare owners, realize participants, collect readiness, or publish a generation.

### Harness Ordering

[`HarnessRun::boot`](../../../../../../src/harness/boot.rs) resolves the Event authority and calls `ProductRuntimeAssembly::load_composed` before it invokes scoped world initialization. Assembly therefore resolves Stewardship theory and constructs semantic handle factories against stores as they existed before the requested init stages install theory or create the Agent.

The assembly contract handles absent semantic state honestly by producing unresolved or body-less factory bindings. The hazard is that the same `HarnessRun` retains that already-built assembly after world initialization succeeds. It does not recompose factories from the newly installed revisions. A later supervisor start may therefore exercise unresolved bindings even though the manifest reports successful theory installation and Agent genesis.

The inverse masking risk also exists on a reused root. Assembly may bind an older installed head before world initialization installs a newer revision. The later stage report proves the new revision exists, while the active factory still carries the earlier resolved body.

This does not make the harness primitives unsound. It proves that its stated claim of booting exactly like the product cannot establish end-to-end product readiness until assembly is downstream of exact initialization closure or is explicitly rehydrated from that closure. Foreground runtime assembly and world initialization are also exposed as separate commands, so the same ordering gap is not harness-only.

### Startup Generation Publication

[`StartupActivationStore`](../../../../../../src/runtime/activation.rs) can persist assignment, activation, prepared closure, startup generation stages, and an assignment head. It requires opaque readiness references for every required participant in the supplied plan and uses a compare-and-swap from an absent head before returning `CurrentAssignmentRuntime`.

This store has no production call site. Its use is confined to its own tests. Startup generation number is fixed at one, expected prior generation is always absent, and replacement is not supported.

Readiness evidence consists only of participant id and non-empty receipt reference. The store does not resolve the plan's readiness contract reference, verify receipt owner, bind the receipt to the generation or participant incarnation, reject undeclared extra readiness, or prove that a participant uses the selected runtime realization. A caller can therefore satisfy structural readiness with arbitrary strings.

Publication is not crash-atomic. The store writes the ready generation, changes the assignment head, then writes the current generation record. Process loss after the head change can leave the head pointing at a generation for which `current_generation` cannot resolve the expected current record.

The returned `CurrentAssignmentRuntime` mixes durable identity strings with process-local `Arc` values for the Capability catalog and executor registry. It does not prove that those process-local values realize the prepared Capability closure reference.

### General Activation Lifecycle

[`ActivationLifecycleStore`](../../../../../../src/runtime/lifecycle.rs) separately provides lifecycle intent identity, assignment heads, admission fences, participant incarnations, durable operations, attempts, waits, liveness projection, and fenced-quiescence receipts. Intent retries are idempotent by request key. Incarnation identity is separate from generation and operation attempt.

This store and `StartupActivationStore` both name the same `pds_assignment_heads_v1` tree when opened over the same theory database, but they do not share one generation record or publication implementation. Runtime assembly constructs only the general lifecycle service. Startup publication constructs only the startup store. The two paths are not integrated.

General head publication also has crash windows. It changes the head and then opens admission as a separate write. Replacement closes old admission before comparing and changing the head. A conflict or process loss can leave the old current generation closed without a new live generation. These are implemented facts, not judgments about a future persistence choice.

Owner lifecycle ports exist for preparation, readiness, wake resolution, safe point, and stop. No current owner implements them and no production coordinator invokes them.

## Identity And Handoff Ledger

| Identity | Implemented producer | Intended consumer | Current durable relation | Initialization finding |
| --- | --- | --- | --- | --- |
| package content hash | package materialization | router and receipt | exact and content-derived | sound |
| package receipt id | theory router | assignment and owner resolution | exact linked owner revisions | sound |
| owner theory revision | native owner registry | Agent, Belief, Strategy, policy, and other native runtime | exact registry, id, and content hash | sound locally |
| assignment id | config stewardship | activation and lifecycle | exact package and situated principal binding | not connected to Agent record |
| activation id | config stewardship | prepared closure and generation | exact physical realization inputs | not connected to assembly |
| prepared id | theory activation | generation publication | exact inert closure | test-only construction |
| lifecycle intent id | runtime lifecycle | lifecycle service | request-key idempotent | stops at exact-inputs-pending |
| generation id | startup activation | current runtime | fixed startup identity | test-only and disconnected |
| participant plan id | theory activation | realization and readiness aggregation | exact structural hash | no production producer or consumer |
| root registration id | runtime assembly | supervisor | root-local only | not mapped to participant id |
| supervisor instance and lease | supervisor | operational restart logic | durable operational ownership | not mapped to participant incarnation |
| participant incarnation id | activation lifecycle | owner readiness and attempts | exact generation, participant, realization, and incarnation number | no realization path |
| Agent id | world init and Agent owner | Agent runtime | durable owner record | package and assignment lineage absent |
| Agent subscription id | Agent owner | Agent work selection | deterministic Agent and belief-key natural key | not a participant readiness receipt |
| passive subscription id | runtime delivery | passive source delivery | activation and incarnation lineage | separate from Agent subscription |
| genesis Event record id | init through Events | graph and epistemic consumers | deterministic idempotent append | no aggregate readiness barrier |
| readiness receipt reference | caller or future owner | startup publisher | opaque string only | unverifiable today |

## Ordering And Visibility Findings

The implemented happy-path order is not one authoritative order. It is a set of conventions spread across commands and comments.

The strongest safe ordering facts are local. Package validation precedes owner installation. Complete owner installation precedes package receipt creation. World initialization orders selected stages. Agent subscription creation precedes the current operational marker. Startup activation checks supplied readiness before changing its assignment head.

The missing cross-domain visibility barriers are material:

| Producer milestone | Consumer that needs it | Implemented barrier | Evidence class |
| --- | --- | --- | --- |
| exact owner theory installed | Agent genesis and runtime actor binding | world init resolves selected revisions, assembly may also resolve current theory independently | implemented but duplicated selection |
| world initialization installs or advances theory | already-built harness or foreground assembly | no rebind or reassembly barrier | implemented ordering gap |
| package receipt current | assignment preparation | assignment type can cite receipt, no production materializer | inferred connective need |
| assignment durable | Agent genesis | none | inferred connective need |
| activation and Capability closure prepared | runtime assembly | none | inferred connective need |
| Agent record, subscriptions, and owner state reconstructed | participant readiness | none | inferred connective need |
| graph replay caught up through genesis Event | Agent readiness | none | World Model Reconciliation proposal |
| Belief view readable through required cut | Agent readiness | none | canonical Agent readiness |
| planner projection available | Agent readiness | none | canonical Agent readiness |
| Curation and Strategy runtime bindings exact | Agent readiness | none | World Model Reconciliation proposal |
| every required owner readiness receipt resolved | current generation publication | opaque strings accepted only in test-only publisher | canonical lifecycle intent, partial implementation |
| current generation published | root semantic actor admission | none | inferred connective need |

The crucial boundary is current-generation publication. It must be the point after which ordinary reconciliation work can be admitted. Current Agent operational status and root supervisor handle start can both precede that boundary.

## Idempotency, Failure, And Restart

### What Is Already Strong

Package receipts, assignments, activations, prepared closures, lifecycle intents, participant incarnations, Agent subscriptions, and genesis Events all have stable identities. Owner theory registries generally install by content revision. The world initializer stops on first failure and is safe to rerun across its existing owner commands. Agent subscription cursors and decision records are durable. Supervisor leases distinguish process ownership from durable domain state.

These primitives materially reduce the amount of new lifecycle behavior needed.

### What Remains Disconnected

There is no durable initialization attempt that records the exact phase and aggregate closure across package installation, assignment, activation, Agent genesis, owner preparation, participant realization, readiness, and publication.

There is no restart reconstructor that starts from one lifecycle intent and determines which local products already exist, which readiness receipts remain valid, which participant incarnations are stale, and whether publication occurred.

There is no durable negative or incomplete state for an Agent that is registered and locally operational but lacks genesis Event visibility, graph catch-up, Belief readability, Strategy projection, or current generation authority.

`FailedBeforeCurrent` exists in startup generation status but current publication never writes it. General lifecycle intents retain only an opaque phase transition saying exact inputs are pending. Neither path records a truthful pre-current failure account suitable for restart or operator inspection.

Package installation deliberately permits immutable partial owner revisions while withholding the complete receipt. That is recoverable because selectability is fenced by the receipt. World initialization and activation need the same style of aggregate visibility rule. Local products may exist, but they must remain inert until a complete exact readiness closure is current.

## Regenerated Domain Snapshot

The root domain universe was regenerated on 2026-08-20 with the governance command over top-level Rust modules. Current domains are:

```text
agent api branches capability cli compat concurrency config context control
dependency_security docs error events execution harness heads ignore init lib
logging merkle_traversal metadata prompt_context provider runtime serve session
store task telemetry theory tree types views workflow workspace world_state
```

The workspace crate universe was regenerated from `crates` and contains `meld-events`, `meld-execution`, `meld-lang`, and `meld-world-model`.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence or non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- |
| `agent` | none | separate root Agent command and profile surface | not needed | stewardship Agent genesis is owned by `meld-world-model` | none |
| `api` | none | no initialization or activation-generation API | not needed | CLI is the current adapter and no API surface is required by this concern | none |
| `branches` | publish | supplies durable branch identity and scope | complete | assignment and Agent records carry branch identity | preserve source ownership |
| `capability` | publish | supplies exact contract registry, active inventory, catalog, executor registry, and closure inputs | partial | prepared closure and returned runtime are not verified against one exact Capability realization | retain Capability ownership |
| `cli` | adapter | exposes world initialization and runtime commands | partial | command paths invoke separate world and machine lifecycle systems | keep adapter non-authoritative |
| `compat` | none | compatibility namespace only | not needed | no truthful initialization ownership | none |
| `concurrency` | none | generic concurrency support | not needed | activation operational limits do not require this domain to interpret lifecycle | none |
| `config` | publish | selects stewardship product and defines assignment and activation identities | partial | current selection drives world init but does not materialize one lifecycle intent | connect through structural input only |
| `context` | none | context frames are later cognitive products | not needed | current-generation readiness need not create prompt context | none |
| `control` | none | legacy orchestration state | not needed | lifecycle dependencies must not become Steward choreography | none |
| `dependency_security` | publish | publishes one current PDS theory package and owner theory | partial | product contribution exists but has no generation-ready materialization path | preserve domain semantics |
| `docs` | publish | publishes one current PDS theory package and owner theory | partial | routed installation is currently hard-wired through docs for explicit package source | remove product-specific root assumption when contracts are drawn |
| `error` | none | common error adaptation | not needed | domain errors already remain local and no new error owner is justified | none |
| `events` | publish | adapts canonical append, replay, cursor, and genesis Event transport | partial | genesis append is durable but not included in activation readiness | expose exact visibility milestone |
| `execution` | publish | owns executable Goal and Task Network runtime participants | partial | root actors exist but no participant-generation readiness is published | remain semantically independent |
| `harness` | observe | characterizes PDS install, world init, runtime assembly, and flywheel behavior | partial | assembly precedes world init and is not recomposed, so successful stage reports can coexist with stale or body-less bindings | observe aggregate lineage later |
| `heads` | none | legacy head compatibility | not needed | activation and package heads have independent ownership | none |
| `ignore` | none | workspace file selection policy | not needed | no initialization lifecycle behavior | none |
| `init` | adapter | owns the staged world initialization command pipeline | partial | durable owner writes are sound but not joined to assignment-local activation | adapt one declared genesis topology |
| `lib` | none | crate export surface | not needed | public export changes are later implementation scope | none |
| `logging` | none | operational log setup | not needed | logs cannot prove readiness | none |
| `merkle_traversal` | none | workspace scan traversal | not needed | not the World Model Traversal participant | none |
| `metadata` | none | frame metadata schemas | not needed | no initialization ownership | none |
| `prompt_context` | none | prompt artifact orchestration | not needed | prompt construction follows activation and situated planning | none |
| `provider` | publish | provides physical profiles and process-local execution binding | partial | activation references providers but readiness is not generation-bound | preserve provider ownership |
| `runtime` | own | opens stores, assembles registrations, supervises roles, stores lifecycle records, and has two generation stores | partial | no single preparation, realization, readiness, and publication path | structural lifecycle owner |
| `serve` | none | service adapter | not needed | no current lifecycle authority and no requirement for one | none |
| `session` | none | command session lifecycle | not needed | program and activation lifetime must survive sessions | none |
| `store` | none | persistence primitives | not needed | persistence does not interpret lifecycle closure | none |
| `task` | none | root legacy Task compatibility runtime | not needed | Execution crate owns the canonical Task Network participant | none |
| `telemetry` | observe | reports runtime and session observations | partial | operational health is visible but program readiness is not | project only owner receipts |
| `theory` | own | materializes packages, routes owner installation, records exact receipts, and defines inert activation closure | partial | prepared closure has no production construction or lifecycle consumer | retain structural linking only |
| `tree` | none | tree utilities | not needed | implementation detail | none |
| `types` | none | shared compatibility exports | not needed | no initialization ownership | none |
| `views` | none | presentation-shaped read models | not needed | future lifecycle projection does not require this current domain | none |
| `workflow` | none | explicitly legacy workflow system | not needed | World Model Reconciliation does not initialize through Workflow | none |
| `workspace` | publish | supplies physical target, subject binding, and first observation source | partial | binding is selected during world init but not readiness-linked | publish owner visibility only |
| `world_state` | none | root adapter over world-state capabilities | not needed | current runtime binds directly to `meld-world-model` graph owners | none |
| `meld-events` | publish | owns durable Event append, replay, cursor, watermark, and subscription contracts | complete locally | lifecycle does not yet consume an exact graph or consumer visibility receipt | reuse unchanged and connect structurally |
| `meld-execution` | publish | owns Goal Set, planning runtime, Task Network, dispatch, publication, and execution progress | complete locally | no activation-generation participant receipt exists | reuse semantics and add lifecycle participation only if required |
| `meld-lang` | none | permissive desired-state and planning vocabulary | not needed | lifecycle must not enforce grammar here | none |
| `meld-world-model` | own | owns Agent, Belief, graph, planner, Strategy, durable subscriptions, and reconciliation state | partial | current Agent genesis is executable-only and weaker than canonical readiness | major proposed reconciliation participant |

## Frozen Affected-Domain Set

The affected set is frozen as root `branches`, `capability`, `cli`, `config`, `dependency_security`, `docs`, `events`, `execution`, `harness`, `init`, `provider`, `runtime`, `telemetry`, `theory`, and `workspace`, plus `meld-events`, `meld-execution`, and `meld-world-model`.

Every other current domain is an explicit non-integration. Runtime traversal alone does not place a domain in implementation scope.

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| branch identity | branches | durable branch and branch-scope identities | resolve exact assignment and Agent scope | reuse unchanged | root invents branch meaning | assignment and Agent records |
| Capability contract installation | capability | exact owner registry revisions | expose exact activation input | reuse unchanged | semantic package becomes live catalog | prepared closure |
| Capability realization closure | capability | catalog, inventory, binding view, executor registry | prove one exact generation realization | extend existing | runtime trusts process-local values without durable match | startup publication return value |
| world-init command routing | CLI | selected stages and explicit package source | invoke structural initialization without owning readiness | adapter only | manual stage selection bypasses complete genesis | world init tooling |
| stewardship selection | config | product, target, subject, Agent, principal, and theory selection | materialize exact assignment-local lifecycle input | extend existing | compatibility selection freezes product contract | stewardship config |
| assignment identity | config and PDS | exact content-derived assignment | bind Agent genesis and runtime generation lineage | extend existing | Agent identity drifts from assignment | assignment contract |
| activation identity | config and runtime | exact bindings, implementation choices, placement, isolation, and limits | bind runtime assembly and participant realization | extend existing | declared isolation exceeds actual realization | activation contract |
| docs theory publication | docs | owner components and PDS package | remain one producer into generic materialization | reuse and extend data | root docs path becomes universal PDS compiler | routed install adapter |
| security theory publication | dependency security | owner components and PDS package | remain one producer into generic materialization | reuse and extend data | security runtime meaning moves into PDS | security package |
| Event genesis append | events and meld-events | deterministic durable append | publish exact append receipt into readiness closure | extend existing seam | append durability is confused with downstream visibility | world init pipeline |
| replay and consumer visibility | meld-events | durable sequence and cursor authority | identify exact wake and catch-up milestones | reuse unchanged | root interprets Event payload meaning | event authority |
| Execution runtime readiness | execution and meld-execution | bounded actors and durable stores | report owner-defined readiness for declared generation participant | extend existing seam | lifecycle adds epistemic guards to Execution | runtime assembly |
| harness proof | harness | isolated boot assembles before world init and retains the original factories | observe one end-to-end lineage and restart position | extend existing | test ordering masks stale or unresolved product bindings | harness boot |
| theory installation stage | init | ordered owner-command adapter | consume exact package and assignment genesis declaration | extend existing | init duplicates package resolution | world init pipeline |
| Agent genesis stage | init adapter and Agent owner | idempotent seed registration and one subscription | materialize exact declared Agent topology and preserve lineage | extend existing | root writes or interprets Agent state | genesis identities stage |
| epistemic seed stage | init adapter and Events owner | idempotent genesis Event append | make required downstream visibility explicit | extend existing | operational Agent precedes its first wake path | seed facts stage |
| provider binding | provider | profiles, clients, and execution adapters | publish exact realization readiness | extend existing seam | provider availability changes product semantics | activation and assembly |
| package materialization | theory | bounded source and content hash | retain exact inert input | reuse unchanged | package loader interprets owner bodies | package module |
| owner routing | theory and native owners | validate, install, verify, and link | retain federated semantic ownership | reuse unchanged | router becomes central ontology | theory router |
| package receipts and heads | theory | immutable exact receipts and expected-prior head | remain selectability boundary for semantic install | reuse unchanged | partial installs become visible | theory receipt and registry |
| prepared activation closure | theory and runtime composition | exact inert aggregate | become the single preparation input to realization | extend existing | closure absorbs owner state | theory activation |
| participant plan | runtime composition with owner declarations | structural identity and dependencies | derive exact required runtime closure | extend existing | dependencies become cognitive choreography | theory activation |
| lifecycle intent | runtime | durable idempotent request | retain phase and exact input lineage through restart | extend existing | lifecycle becomes semantic coordinator | runtime lifecycle |
| participant realization | runtime supervisor plus native owners | registrations, factories, leases, and separate incarnation records | map each required participant to one generation and incarnation | extend existing | role registration is mistaken for domain readiness | assembly, registration, lifecycle |
| readiness aggregation | runtime composition plus native owners | opaque references in test-only startup publisher | resolve exact owner receipts and complete participant closure | new local behavior | root interprets owner readiness | startup activation |
| current-generation publication | runtime | two separate assignment-head implementations | establish one conditional live-authority boundary | extend existing | crash leaves split head and admission state | activation and lifecycle stores |
| Agent registration | meld-world-model Agent | durable record and revision binding | retain exact package, assignment, activation, and genesis provenance | extend existing | PDS shadows Agent state | Agent registration |
| Agent subscription | meld-world-model Agent | durable belief-key subscription and cursor | contribute owner readiness only after source is readable | extend existing | active subscription is treated as ready cognition | Agent subscription |
| Belief readiness | meld-world-model Belief | durable family revisions, evidence ingestion, and current views | report exact family and readable-view milestone | extend existing | root interprets confidence or evidence | belief registry and runtime |
| graph readiness | meld-world-model graph | durable Event replay and Traversal store | report materialization through required sequence | extend existing | append receipt is treated as graph visibility | graph runtime |
| Strategy readiness | meld-world-model Strategy | current executable candidate theory | later report exact Plan-construction inputs are resolvable | new local behavior | readiness requires constructing a situated Plan | reconciliation proposal |
| Curation readiness | future meld-world-model Curation owner | no bounded EpiOp runtime yet | later report installed Curation operation theory and durable intake availability | new local behavior | Agent Goal curation is mistaken for epistemic Curation | reconciliation proposal |
| workspace binding | workspace | target root, filesystem subject, scan and watch | publish source readiness and first observable cut | extend existing seam | lifecycle interprets workspace truth | world init tooling and workspace source |
| telemetry projection | telemetry | runtime status and session summaries | correlate owner receipts without creating truth | adapter only | health is presented as program readiness | runtime reports |

## Ownership Synthesis

Theory owns exact package linking and inert closure identity. It should not supervise or decide readiness.

Native owners own installation, reconstruction, and the meaning of their readiness. For World Model Reconciliation that includes Agent, Belief, graph, Strategy, and future Curation. Events owns durable visibility positions. Capability and provider own exact realization evidence. Execution owns its own participant readiness without learning epistemic semantics.

Root init remains an adapter for first durable meaning and Agent genesis. Root runtime owns the structural participant closure, operational realization, readiness aggregation, and conditional current-generation boundary. The supervisor owns leases and process restart, not semantic readiness.

PDS assignment and activation explain why this exact generation exists. They do not become a second owner for Agent, Belief, Strategy, Curation, Events, or Execution state.

The smallest missing connective behavior is one durable assignment-local initialization account that carries exact package, assignment, activation, participant-plan, and Agent-genesis lineage while aggregating owner receipts without interpreting them. Current evidence does not settle its type, store, or API.

## Scope Separation

Domains on the runtime path are config, theory, init, runtime, events, capability, provider, workspace, branches, `meld-events`, `meld-execution`, and `meld-world-model`. Product contributors such as docs and dependency security are on the installation path. CLI, harness, and telemetry observe or adapt the path.

Behavior that must change is narrower. Root runtime needs one coherent generation protocol. Root init must bind Agent genesis to exact assignment lifecycle rather than an independent command convention. Agent readiness must become truthful for the World Model Reconciliation topology. Native participants need owner-defined readiness publication. The PDS router, Event authority, Execution semantics, and language can remain unchanged in meaning.

Likely write scope cannot be truthfully frozen at this discovery maturity. The current missing seams touch root runtime, root init, PDS activation preparation, and `meld-world-model`. Capability, Events, provider, workspace, and Execution may need only narrow readiness adapters. Docs and dependency security may require data changes when Agent topology and Curation theory are specified. CLI, harness, and telemetry remain adapter or evidence candidates.

## Explicit Rejections

Initialization must not become a manual list of Steward actions. Participant dependencies express physical readiness and safe lifecycle ordering only.

Root runtime must not decide whether a belief is epistemically sufficient, whether a Plan is coherent, whether Curation is correct, or whether an Execution Goal is justified.

An Agent subscription, clean actor tick, heartbeat, open store, package head, or genesis append is not by itself a readiness proof for the whole program.

Owner readiness must not be replaced by root probes into foreign stores. Producers publish exact receipts. Root verifies identity and closure.

World Model Reconciliation must not start under one configuration-derived runtime composition while assignment lifecycle reports another generation as current.

## Unresolved Questions

Current evidence does not determine whether first product installation and later activation replacement use one intent type or related types.

The exact Agent topology declared by PDS remains open. This prevents a final participant-plan cardinality.

It remains open whether the genesis Event must be graph-visible, belief-visible, or Agent-accepted before generation publication. The consumer milestone must be chosen from actual first-work requirements.

It remains open whether participant readiness receipts live in native owner stores and are referenced by root, or whether owners append neutral lifecycle Events carrying those references.

It remains open how a package upgrade binds existing Agent identity and subscriptions to a new semantic generation without rewriting historical cognition.

The relationship between root supervisor lease identity and activation participant incarnation needs an explicit mapping. Current evidence proves they are distinct but does not determine the carrier.

The current startup activation store and general lifecycle store overlap in assignment-head ownership. Evidence does not justify keeping both authoritative paths.

## Confidence

High confidence: current production has no end-to-end path from prepared activation closure to owner readiness and current-generation publication. The startup publisher and prepared closure are test-only. World initialization, assembly, supervision, and lifecycle intent handling are separately invoked.

High confidence: current Agent operational status is weaker than canonical readiness and is not activation-generation scoped.

High confidence: exact identities and idempotent local owner products provide a solid foundation. The refactor need is lifecycle composition, not wholesale replacement of runtime logic.

Medium-high confidence: the frozen affected-domain set is complete for initialization and genesis. Future Curation and heterogeneous Strategy Plan contracts can alter participant details but do not change the ownership frame.

Medium confidence: the exact visibility milestone for the first reconciliation wake remains unresolved and must follow the eventual consumer contract.
