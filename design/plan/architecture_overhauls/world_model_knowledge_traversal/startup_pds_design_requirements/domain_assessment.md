# Startup PDS Domain Assessment

Date: 2026-08-23

Status: independent requirements evidence

## Concern And Scope

The assessed behavior is one generation and admission-epoch scoped Startup PDS nonce that begins as an Agent-visible epistemic mismatch, causes an ordinary Goal and heterogeneous Strategy Plan, executes one Task through the unified Task Network, publishes one owner-issued Event kind `nonce`, confirms the Event through bounded Curation and configured Belief, and ends with exact Agent Goal satisfaction evidence.

The design target is the [expected post-reconciliation injection baseline](expected_injection_baseline.md). Current source defines the domain universe and likely implementation anchors. Accepted design defines the behavior expected to exist before injection.

In scope are product compilation, Agent genesis, current-generation eligibility, nonce identity, Curation, planning, Task realization, Event publication, epistemic return, Goal disposition, inspection, retry, recovery, replacement, and retirement lineage.

Out of scope are general runtime health, a new scheduler, a second Event system, a new graph store, global quiescence, arbitrary operator probes, universal activation gating, legacy Workflow compatibility, Rust type selection, storage selection, migration, and implementation sequencing.

The inspection budget was twelve batched repository calls and twenty additional directly linked source items. The assessment remained within that budget.

## Regenerated Domain Snapshot

The current universe is the exported top-level domain set of the root crate plus `meld-world-model`, `meld-execution`, `meld-events`, and `meld-lang`. The expected Curation and nonce owners are listed separately as absent target domains rather than represented as current code.

### Root Crate Sweep

| Domain | Needed integration | Current integration | Completeness | Non-integration rationale or required relationship |
| --- | --- | --- | --- | --- |
| `agent` | none | separate root Agent utilities | not needed | not the world-model Agent authority |
| `api` | adapter | generic external API | partial | may expose inspection later but cannot own nonce truth |
| `branches` | publish | branch identity and lookup | complete | reuse exact branch identity |
| `capability` | publish | root Capability composition | partial | register the exact `nonce.emit.v1` contract and binding |
| `cli` | adapter | command presentation | partial | optional inspection presentation only |
| `compat` | none | compatibility helpers | not needed | no legacy path is preserved |
| `concurrency` | none | generic utilities | not needed | no domain relationship |
| `config` | own | stewardship declaration, assignment, and activation inputs | partial | select the Startup PDS and bind its assignment and activation |
| `context` | none | contextual frame domain | not needed | no Startup PDS authority |
| `control` | none | legacy control behavior | not needed | no startup orchestration path |
| `dependency_security` | none | independent product domain | not needed | later product, not canary semantics |
| `docs` | none | independent product domain | not needed | later product, not canary semantics |
| `error` | none | shared errors | not needed | implementation detail only |
| `events` | adapter | root Event bindings | partial | wire owner publication and inspection without interpreting it |
| `execution` | adapter | root Execution ports | partial | carry the complete authorized Task through the existing seam |
| `harness` | observe | integration evidence and projections | partial | prove the exact round trip during implementation |
| `heads` | none | independent head utilities | not needed | lifecycle head remains in runtime authority |
| `ignore` | none | workspace filtering | not needed | no domain relationship |
| `init` | adapter | current world initialization | partial | materialize compiled product and bootstrap activation without semantic shortcuts |
| `logging` | observe | operational logs | complete | optional diagnostics only |
| `merkle_traversal` | none | workspace Capability | not needed | not part of nonce behavior |
| `metadata` | none | frame metadata contracts | not needed | no domain relationship |
| `prompt_context` | none | prompt assembly | not needed | no model prompt is required by the minimal canary |
| `provider` | consume | Capability implementation and binding support | partial | host or resolve the `nonce.emit.v1` implementation if selected |
| `runtime` | own | activation, assembly, lifecycle, supervision, self observation | partial | compose the expected lifecycle and native participants without interpreting nonce meaning |
| `serve` | adapter | Event service routes | partial | optional inspection transport only |
| `session` | none | interactive session behavior | not needed | nonce is not session scoped |
| `store` | none | generic persistence utilities | not needed | no new common store is required |
| `task` | none | root compatibility Task surface | not needed | canonical Task behavior belongs to `meld-execution` |
| `telemetry` | observe | operational observation | complete | optional correlated diagnostics only |
| `theory` | own | package router, owner routes, and receipts | partial | install the exact Startup PDS components |
| `tree` | none | Merkle tree mechanics | not needed | no domain relationship |
| `types` | none | shared root values | not needed | no new universal type is required |
| `views` | adapter | presentation views | partial | optional read-only nonce projection |
| `workflow` | none | explicitly legacy workflow | not needed | forbidden as an implementation target |
| `workspace` | none | workspace observation | not needed | the canary does not inspect files |
| `world_state` | adapter | root world-model facade | partial | expose owner-routed Graph and Traversal contracts only |

### Meld World Model Sweep

| Domain | Needed integration | Current integration | Completeness | Non-integration rationale or required relationship |
| --- | --- | --- | --- | --- |
| `agent` | own | current curation decisions and Goal commands | partial | own nonce instance, Goal, Plan judgment, authorizations, milestone absorption, and satisfaction |
| `belief` | consume | configured Event and evidence settlement | partial | admit exact nonce expectation and realization evidence under installed routes |
| `planner` | publish | current flattened Strategy projection | partial | assemble the complete nonce `PlannerCut` |
| `strategy` | own | current executable composition search | partial | construct one Task and one verification Epistemic Operation in a heterogeneous Plan |
| `waiting` | publish | native wait grammar | partial | express exact Event, projection, operation, and epoch wakes |
| `world_state` | consume | current Graph and Traversal with a narrow owner allowlist | partial | admit and return the nonce owner publication under an exact cut |

### Meld Execution Sweep

| Domain | Needed integration | Current integration | Completeness | Non-integration rationale or required relationship |
| --- | --- | --- | --- | --- |
| `authority` | consume | Execution-owned authorization checks | partial | validate only Task intake authority and fences |
| `capability` | own | contract, catalog, binding, and invocation | partial | admit `nonce.emit.v1` and its selected implementation |
| `error` | none | shared errors | not needed | implementation detail only |
| `execution` | consume | Execution intake and ports | partial | accept the complete Task without startup semantics |
| `generation` | consume | Execution generation fencing | partial | enforce exact admission epoch |
| `goals` | consume | current Goal Set | partial | preserve Goal attribution for the one authorized Task |
| `planning` | consume | lowering and realization | partial | lower the Task into the unified network |
| `publish` | publish | Execution outcome publication | partial | publish operational outcome independently from the custom nonce Event |
| `task` | consume | Task compilation and execution | partial | carry exact event and nonce inputs |
| `task_network` | own | durable network and dispatch | partial | realize the Task through the one shared Execution coherence domain |
| `traversal` | publish | Execution internal traversal | complete | reuse unchanged for network traversal |
| `waiting` | publish | Execution wait grammar | partial | retain exact binding, claim, attempt, and outcome wakes |
| `workflow` | none | legacy subsystem | not needed | explicitly excluded |

### Meld Events Sweep

| Domain | Needed integration | Current integration | Completeness | Non-integration rationale or required relationship |
| --- | --- | --- | --- | --- |
| `error` | none | shared Event errors | not needed | implementation detail only |
| `events` | publish | durable append, replay, cursors, provenance, and observability | complete | reuse neutral carriage and exact append receipt without Event grammar changes |

### Meld Lang Sweep

| Domain | Needed integration | Current integration | Completeness | Non-integration rationale or required relationship |
| --- | --- | --- | --- | --- |
| `authority` | publish | permissive authority values | complete | reuse unchanged in Task authorization |
| `composition` | publish | executable composition value | complete | reuse only inside the complete Task if required by Task compiler |
| `condition` | publish | condition values | complete | reuse for desired and satisfaction expressions |
| `cost` | none | cost values | not needed | no new relationship |
| `effect` | publish | effect values | complete | describe the nonce-emission effect without new grammar enforcement |
| `evaluate` | none | expression evaluation | not needed | Strategy and Agent own semantic decisions |
| `goal` | publish | Goal language | complete | reuse for the Agent-owned nonce Goal |
| `method` | none | legacy planning language | not needed | Strategy Plan construction does not require Method changes |
| `operator` | none | executable operator language | not needed | no new language enforcement is justified |
| `proposition` | publish | proposition values | complete | express expected and realized nonce propositions |
| `substitute` | none | term substitution | not needed | no direct domain relationship |
| `term` | publish | object and scalar terms | complete | carry exact object references |
| `unify` | none | unification | not needed | no direct domain relationship |
| `validate` | none | permissive value validation | not needed | product owners validate their own meaning |
| `world_state` | publish | language world-state values | complete | reuse only as a value boundary where current consumers require it |

## Absent Target Domains

| Target domain | Why it is not represented as current | Required ownership |
| --- | --- | --- |
| Curation | current Agent curation is not the accepted first-class domain | standing expectation authorship, planned confirmation, terminal results, and semantic publication |
| nonce | no current owner defines a reusable nonce Event and emitter Capability | smallest semantic owner for the generic nonce publication identity and global Capability contract |

The nonce owner is a genuine finding from the generalization pass. PDS cannot own the Event merely because it packages the Startup product. Events cannot own it because Events is neutral carriage. Execution cannot own it because Execution owns operational realization rather than the Event proposition. Curation cannot impersonate the Event producer. A very small reusable owner is therefore required unless an already accepted native owner explicitly adopts the same contract during later design review.

The nonce contract is global. It knows an issuer, subject, bounded ordered correlation references, a fence reference, and its own contract revision. It knows nothing about Startup, Agent, Goal, activation, or runtime health. Startup theory supplies those meanings when it constructs and later interprets one nonce use.

## Frozen Affected-Domain Set

The affected current-domain set is root `api`, `branches`, `capability`, `cli`, `config`, `events`, `execution`, `harness`, `init`, `logging`, `provider`, `runtime`, `serve`, `telemetry`, `theory`, `views`, and `world_state`; every current `meld-world-model` domain; `meld-execution` authority, capability, execution, generation, goals, planning, publish, task, task network, traversal, and waiting; `meld-events` events; and `meld-lang` authority, composition, condition, effect, goal, proposition, term, and world state.

Curation and nonce are added as absent target domains. Presence on this set means the domain lies on the operating or evidence path. It does not imply code changes.

## Affected-Domain Decomposition

| Domain concern | Owner | Expected injection ground | Required relationship | Change posture | Boundary risk |
| --- | --- | --- | --- | --- | --- |
| package compilation | root theory and PDS structure | complete product and package receipts | install exact Agent, Curation, Belief, Strategy, Capability, Graph publication, and presentation components | extend existing | router interprets component semantics |
| assignment and topology | root config stewardship | finite declared Agent topology | bind one stable Startup Agent to runtime subject, perspective, branch, and grants | extend existing | assignment identity depends on later generation |
| activation preparation | root config and runtime composition | inert prepared closure | include every participant and Capability needed by the nonce path | extend existing | prepared is reported as live |
| lifecycle admission | root runtime lifecycle | current generation plus open epoch | expose exact generation and epoch to every nonce product | reuse unchanged | canary becomes its own readiness prerequisite |
| product sequencing | PDS product control | no accepted universal gate | optionally withhold dependent activation requests on exact Agent satisfaction receipt | new local behavior only if selected | root interprets Agent meaning or bootstrap becomes circular |
| nonce semantics | nonce | absent | define one reusable deterministic nonce publication and owner-issued Event contract | new local behavior | product-specific meaning enters the global contract |
| nonce emission | nonce plus Capability implementation | absent | idempotently append one exact owner publication from generic authorized Task inputs | new local behavior | generic emission becomes arbitrary Event fabrication |
| standing mismatch | Curation | accepted standing operation grammar | author expected Event and bounded non-realization under an exact cut | extend accepted design | missing Event is mistaken for proved absence |
| Goal inception | Agent | accepted maintained-condition and Goal authority | create one deterministic Goal from the exact mismatch | extend accepted design | startup coordinator incepts Goal outside Agent |
| reasoning cut | Planner | accepted complete `PlannerCut` | bind nonce expectation, mismatch, catalogs, generation, epoch, and authority | reuse unchanged | live reads enter Strategy |
| Plan construction | Strategy | accepted heterogeneous Plan | construct one Event-emission Task plus one post-Event verification operation | reuse unchanged | Strategy targets Task Network or executes Curation |
| progression | Agent | accepted milestone-driven account | authorize Task, absorb Event visibility, authorize verification, absorb Belief, and judge Goal | reuse unchanged | raw append or Task success closes Goal |
| executable admission | Execution Goal Set | accepted complete Task envelope | admit the Task under exact epoch and preserve Goal attribution | reuse unchanged | Execution revalidates startup semantics |
| lowering and dispatch | Execution Planning and Task Network | one unified network | lower and execute through ordinary coherence and fencing | reuse unchanged | private startup executor bypasses shared network |
| operational outcome | Execution | exact attempt and outcome account | report Task outcome independently of nonce realization | reuse unchanged | outcome is treated as Event receipt |
| Event authority | Events | neutral durable append and replay | assign one exact ledger position to the deterministic nonce Event | reuse unchanged | Events validates product grammar |
| graph admission | Graph and Traversal | owner-routed publication contract | materialize the nonce publication and expose an immutable cut | extend existing owner registry | source allowlist becomes universal semantics |
| planned confirmation | Curation | accepted planned operation grammar | confirm the exact Event realization after required Graph visibility | extend accepted design | Curation asserts Event existence without owner evidence |
| belief settlement | Belief | configured route and comparator | produce exact nonce realization revision | extend configuration only | every nonce Event is admitted globally |
| final receipt | Agent | accepted milestone and Goal disposition | record exact Event receipt milestone and separate Goal satisfaction | extend accepted design | Belief revision is confused with Agent receipt |
| native waits | every participant owner | accepted work and wait closure | name exact missing successor and viable wake | reuse unchanged | polling or timeout becomes correctness |
| inspection | read-only proof projection | accepted `WMR-H18` | correlate one nonce account and first missing successor | extend projection | projection creates health truth or advances state |
| language values | `meld-lang` | permissive nouns and verbs | carry existing Goal, proposition, condition, effect, authority, and term shapes | reuse unchanged | generic language attempts to enforce Startup PDS grammar |

## Ownership Synthesis

PDS owns the product definition, selected package set, topology, assignment, activation inputs, and optional dependent-product policy. Runtime canary owns the minimal product semantics that no existing neutral domain may honestly own. Agent owns the nonce instance after the epoch is open, every authority decision, and final satisfaction. Curation owns expectation and realization assessment. Strategy owns the immutable Plan. Execution owns only executable realization. Events owns append and replay. Graph and Traversal own structural publication and reads. Belief owns configured admission. Root lifecycle owns structural generation coherence. Inspection owns no truth.

## Separated Impact Scopes

The runtime path includes PDS, configuration, initialization, lifecycle, nonce, Agent, Curation, Planner, Strategy, Execution, Capability, Events, Graph, Traversal, Belief, waiting, and inspection.

Behavior change is confined to the new reusable nonce owner, Startup PDS data, owner-route configuration, installed Curation and Belief theory, installed Strategy theory, product topology and preparation, and the Startup nonce inspection specialization. The accepted generic Agent, Strategy, Execution, Events, Traversal, Belief, and lifecycle meanings are reused.

Likely implementation writes are a new first-party Startup PDS package, a small nonce domain or an explicitly adopted equivalent owner, one global `nonce.emit.v1` Capability, package and owner-route registration, activation composition for the Startup product, and integrated proof fixtures. `meld-lang`, Events core, Execution planning, Task Network semantics, generic Graph storage, legacy Workflow, Docs, Dependency Security, and current runtime health observation need no semantic write.

## Confidence And Unresolved Ownership

Confidence is high that the canary composes the accepted reconciliation contracts without a new coordinator, scheduler, store, Event grammar, or execution path. Confidence is high that one nonce per open admission epoch is the correct recovery scope.

Confidence is high that the Capability and Event contract should be generic rather than Startup-specific. Confidence is medium-high that a tiny nonce owner is the cleanest semantic home. A later review may prove that an accepted Capability owner already has authority to issue this exact product. It may not assign the meaning to PDS, Events, Graph, Execution, or root lifecycle merely to avoid the small domain.

The only material unresolved policy is fail-closed activation of dependent products. The nonce itself does not require it. Selecting that policy adds a live PDS-to-Agent evidence dependency and changes the existing statement that PDS leaves the live path after preparation.
