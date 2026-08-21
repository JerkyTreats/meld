# Root Meld Crate Entity-First Impact Assessment

Date: 2026-08-20

Status: evidence assessment, requirements and implementation not authorized

## Reviewer Function

This report assesses the root `meld` crate against the proposed heterogeneous Strategy Plan. It begins with public semantic entities and durable products, synthesizes those entities into their owning root domains, and then synthesizes the domains into one crate-level impact finding.

The review is evidentiary. Current code is treated as implemented fact. The architecture proposal is treated as the behavior being assessed. A required relationship describes the pressure the proposal places on a current boundary. It does not authorize a requirement, migration, sequence, or implementation.

## Concern And Scope

The concrete failure is docs freshness. The installed branch cannot establish that a current README already exists and is correct. Its inspection deliberately excludes managed README files, its claim records describe generated candidate README content, and its final assessment requires a publication receipt. The only configured path to freshness knowledge therefore passes through an executable write chain.

The proposed architecture separates three products. Strategy constructs and reconstructs a causal Plan during reconciliation. A complete Task is an executable discharge product sent through the existing Execution Goal Set seam. A bounded Epistemic Operation is an epistemic discharge product realized by Curation. Curation publishes shared semantic results through Events, Traversal materializes those results, and later admitted knowledge may cause Agent and Strategy to reconcile the Plan.

This root-crate assessment covers product-owned observations, theory package sources, physical selection, initialization, runtime composition, cross-crate adapters, current executable product supply, diagnostics, and compatibility surfaces.

The following caller limits remain fixed:

- Execution remains ignorant of Curation, Epistemic Operations, Traversal, Belief, and Strategy reasoning.
- `meld-lang` remains permissive shared vocabulary.
- producers own published meaning.
- consumers protect only their needed transport shape and owned state.
- no change outside world-model is in scope unless current evidence proves a specific boundary adaptation.
- old workflow behavior remains explicitly legacy and is not a design surface for the new Plan.

Primary evidence is the [assessment charter](../assessment_charter.md), [Strategy redesign proposal](../../strategy_plan_redesign.md), [current code ground map](../../current_code_groundmap.md), [Goal language review](../../reviews/goal_language_strategy_plan_review.md), [Event-backed operation review](../../reviews/event_backed_epistemic_operations_review.md), and [Traversal and Curation assessment](../../curation_assessment.md).

## Regenerated Root Domain Snapshot

The complete root domain set comes from the current [crate surface](../../../../../../src/lib.rs).

```text
agent
api
branches
capability
cli
compat
concurrency
config
context
control
dependency_security
docs
error
events
execution
harness
heads
ignore
init
logging
merkle_traversal
metadata
prompt_context
provider
runtime
serve
session
store
task
telemetry
theory
tree
types
views
workflow
workspace
world_state
```

Every exported root domain appears once in the pass-one sweep below.

## Pass One Domain Sweep

| Root domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Entity follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Root-local human-facing Agent profile and tooling domain | `not needed` | [root Agent surface](../../../../../../src/agent.rs) | World-model Agent owns Goal and Plan authority | none |
| `api` | `none` | Compatibility facade over context, provider, workspace, and execution ports | `not needed` | [Context API](../../../../../../src/api.rs) | No proposed Curation or Plan contract is owned here | none |
| `branches` | `none` | Owns workspace branch catalog, ledger, and query behavior | `not needed` | [branch surface](../../../../../../src/branches.rs) | Branch selection does not decide Strategy product kinds | none |
| `capability` | `publish` | Assembles exact executable Capability contracts and invokers for Strategy and Execution | `complete` for existing Tasks | [capability inventory](../../../../../../src/capability.rs), [contribution contracts](../../../../../../src/capability/contribution.rs) | not applicable | verify that epistemic operations remain outside this executable catalog |
| `cli` | `none` | Adapts commands and loads root runtime assembly | `not needed` | [CLI runtime assembly](../../../../../../src/cli/runtime_assembly.rs) | No new user command is proven by the architecture | none |
| `compat` | `none` | Re-exports legacy root world-state types | `not needed` | [compatibility surface](../../../../../../src/compat.rs) | Compatibility exports do not own new semantics | none |
| `concurrency` | `none` | Node locking for existing root operations | `not needed` | [concurrency domain](../../../../../../src/concurrency.rs) | Curation concurrency belongs to its owning runtime | none |
| `config` | `adapter` | Selects stewardship identity, theory identities, physical targets, provider, and principal | `partial` | [selection](../../../../../../src/config/stewardship/selection.rs), [physical binding](../../../../../../src/config/stewardship/binding.rs) | not applicable | assess fixed theory selection and activation identity |
| `context` | `none` | Publishes frames and heads and supplies Execution context adapters | `not needed` | [context surface](../../../../../../src/context.rs) | The proposed docs proof does not require new context meaning | none |
| `control` | `none` | Compatibility orchestration over generation and queue submission | `not needed` | [control surface](../../../../../../src/control.rs) | It is not part of canonical Strategy Plan reconciliation | none |
| `dependency_security` | `publish` | Owns security observations, assessments, verification, admission, and an executable-only theory package | `partial` | [security contracts](../../../../../../src/dependency_security/contracts.rs), [security capabilities](../../../../../../src/dependency_security/capability.rs), [security theory](../../../../../../theory/dependency_security/strategy_theory.security.json) | not applicable | assess epistemic products currently expressed as Execution Capabilities |
| `docs` | `own` | Owns docs evidence, generated patches, candidate README claims, policy, publication, and aggregate freshness assessment | `partial` | [docs capabilities](../../../../../../src/docs/capability.rs), [claim validation](../../../../../../src/docs/claim_validation.rs) | not applicable | assess missing observed README and independently addressable claim products |
| `error` | `none` | Root error vocabulary | `not needed` | [error domain](../../../../../../src/error.rs) | No architecture-specific error ownership is proven | none |
| `events` | `adapter` | Re-exports Events and binds the product Event authority | `complete` | [event surface](../../../../../../src/events.rs), [authority binding](../../../../../../src/events/binding.rs) | not applicable | confirm carrier reuse and absence of semantic ownership |
| `execution` | `adapter` | Maps world-model Goal commands and provider-facing ports into Execution contracts | `partial` | [execution surface](../../../../../../src/execution.rs), [root Goal adapter](../../../../../../src/runtime/ports.rs) | not applicable | assess the one-candidate authorization mapping |
| `harness` | `observe` | Projects and explains a hardcoded current actor topology | `partial` | [harness projections](../../../../../../src/harness/projections.rs), [walk](../../../../../../src/harness/walk.rs) | not applicable | assess evidence visibility for Plan and Curation |
| `heads` | `none` | Compatibility index for context heads | `not needed` | [head index](../../../../../../src/heads.rs) | No new head semantics are required | none |
| `ignore` | `none` | Workspace ignore-list policy and helpers | `not needed` | [ignore domain](../../../../../../src/ignore.rs) | Existing workspace observation policy remains owner-local | none |
| `init` | `adapter` | Installs theory, creates Agent genesis identities, and seeds the first epistemic Event | `partial` | [world initialization](../../../../../../src/init/world.rs), [theory routes](../../../../../../src/init/world/routes.rs), [pipeline](../../../../../../src/init/world/pipeline.rs) | not applicable | assess fixed theory bundle and route publication |
| `logging` | `none` | Logging configuration and initialization | `not needed` | [logging domain](../../../../../../src/logging.rs) | No semantic relationship | none |
| `merkle_traversal` | `none` | Executes filesystem Merkle traversal | `not needed` | [Merkle traversal](../../../../../../src/merkle_traversal.rs) | It is not knowledge-graph Traversal | none |
| `metadata` | `none` | Context frame metadata contracts | `not needed` | [metadata surface](../../../../../../src/metadata.rs) | No proposed product uses this ownership boundary | none |
| `prompt_context` | `none` | Stores and orchestrates provider prompt artifacts | `not needed` | [prompt context](../../../../../../src/prompt_context.rs) | Provider-backed Curation is not established by current evidence | none |
| `provider` | `none` | Owns model provider clients, profiles, and execution adapters | `not needed` | [provider surface](../../../../../../src/provider.rs) | A general Epistemic Operation does not imply a provider call | none |
| `runtime` | `adapter` | Composes stores, theory, actor registrations, ports, and bounded actor ticks | `partial` | [runtime assembly](../../../../../../src/runtime/assembly.rs), [runtime ports](../../../../../../src/runtime/ports.rs), [resolved theory](../../../../../../src/runtime/theory.rs) | not applicable | assess Curation realization, Plan routing, and fixed actor topology |
| `serve` | `none` | Serves existing status and projection APIs | `not needed` | [served surface](../../../../../../src/serve.rs) | No new public API is proven | none |
| `session` | `none` | Owns root session lifecycle and storage | `not needed` | [session surface](../../../../../../src/session.rs) | Event session identity already carries current products | none |
| `store` | `none` | Owns root workspace node records | `not needed` | [node store](../../../../../../src/store.rs) | Typed hydration pressure is assessed at the workspace publication boundary | none |
| `task` | `consume` | Root compatibility facade for compiled Tasks, artifacts, packages, and execution | `complete` for executable products | [Task surface](../../../../../../src/task.rs) | not applicable | verify unchanged executable realization |
| `telemetry` | `none` | Emits and projects generic operational Events | `not needed` | [telemetry surface](../../../../../../src/telemetry.rs) | Semantic Curation results must not become telemetry meaning | none |
| `theory` | `own` | Owns generic PDS package materialization, routing, receipts, and exact revision references | `complete` as generic transport | [theory contracts](../../../../../../src/theory/contracts.rs), [package](../../../../../../src/theory/package.rs), [router](../../../../../../src/theory/router.rs) | not applicable | confirm generic reuse and owner-specific route adaptation |
| `tree` | `none` | Owns Merkle node construction and hashing | `not needed` | [tree surface](../../../../../../src/tree.rs) | Physical observation remains workspace-owned | none |
| `types` | `none` | Shared hash aliases | `not needed` | [root types](../../../../../../src/types.rs) | No semantic relationship | none |
| `views` | `none` | Re-exports context view policy | `not needed` | [views surface](../../../../../../src/views.rs) | No semantic relationship | none |
| `workflow` | `none` | Exposes the legacy turn-based workflow system and current package-route compatibility | `not needed` | [workflow surface](../../../../../../src/workflow.rs), [workflow subsystem](../../../../../../src/workflow/README.md) | The canonical Plan must not be represented as a workflow | none |
| `workspace` | `publish` | Publishes source, snapshot, node, containment, and observation facts | `partial` for epistemic consumers | [workspace Events](../../../../../../src/workspace/events.rs), [scan contract](../../../../../../src/workspace/scan.rs) | not applicable | assess observed README identity and typed hydration |
| `world_state` | `adapter` | Re-exports `meld-world-model` as a root compatibility surface | `complete` as an adapter | [world-state surface](../../../../../../src/world_state.rs) | not applicable | confirm that root does not duplicate Plan or Curation meaning |

## Frozen Affected-Domain Set

The affected root domain set is frozen as:

```text
capability
config
dependency_security
docs
events
execution
harness
init
runtime
task
theory
workspace
world_state
```

This set contains every root domain with a truthful direct relationship to the assessed architecture. It is deliberately broader than the likely write set.

## Entity Assessment By Affected Domain

### Capability

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `ProductCapabilityContributor` | root capability | Lets product domains publish executable contracts and implementation offers | Remain the producer contract for Task Construction only | `reuse unchanged` | Treating every causal product as an Execution Capability would erase Curation | [contribution contracts](../../../../../../src/capability/contribution.rs) |
| `ProductCapabilityInventory` | root capability | Deterministically combines docs and dependency-security executable inventory | Continue supplying exact Capability views to Strategy Task Construction | `reuse unchanged` | Adding Epistemic Operations here would route them toward Execution | [inventory assembly](../../../../../../src/capability.rs) |
| `CapabilityTypeContract` and `CapabilityCatalog` | `meld-execution`, re-exported by root | Describe executable bindings, inputs, outputs, effects, and execution class | Remain executable producer vocabulary | `reuse unchanged` | A shared catalog could become an accidental universal operation grammar | [capability surface](../../../../../../src/capability.rs) |
| exact implementation activation | root capability | Closes selected executable contracts against physical bindings | Remain limited to executable implementation realization | `reuse unchanged` | Curation admission and execution would inherit inappropriate Capability assumptions | [activation request](../../../../../../src/capability/contribution.rs) |

Domain synthesis: capability supply is on the Strategy and Task runtime path but has no proven behavior change. Its value is precisely that it remains the complete executable cause catalog while world-model Curation owns a separate epistemic-operation catalog.

### Config

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `StewardshipDeclaration` | root config | Selects expression, target, Agent, principal, provider, and fixed theory identities | Continue selecting the Agent specification and physical scope that bound Curation | `extend existing` | Product selection could embed Curation semantics instead of selecting owner identities | [selection contracts](../../../../../../src/config/stewardship/selection.rs) |
| `TheorySelection` | root config | Names belief family, outcome mapping, Goal curation rule, maintained condition, Strategy theory, authority policy, and docs claim policy | Represent any additional exact Curation-owned theory identity needed at activation | `extend existing` | The current field named curation rule is Agent Goal curation, not graph authorship | [theory selection](../../../../../../src/config/stewardship/selection.rs) |
| `SelectedStewardshipPackage` | root config | Carries the same fixed identity set into installation and assembly | Carry only selected identities or a receipt reference, never theory bodies | `extend existing` | A permanently fixed list makes new owner component kinds root-owned | [selected package](../../../../../../src/config/stewardship/binding.rs) |
| `PhysicalBinding` | root config | Resolves workspace, subject, Agent, provider, package, and external storage root | Continue binding physical scope without constructing Plan or Curation products | `reuse unchanged` | Root binding could begin guessing semantic actor topology | [physical binding](../../../../../../src/config/stewardship/binding.rs) |
| `StewardshipActivationV1` | root config | Content-identifies physical bindings, implementation choices, placement, and limits | Continue identifying operational activation independently of semantic Plan state | `reuse unchanged` | Mixing active Plan identity into physical activation would destabilize both | [activation identity](../../../../../../src/config/stewardship/activation.rs) |

Domain synthesis: root configuration has a real adaptation pressure because its fixed theory selection names only the current executable-only image. Physical binding and activation identity remain reusable and should not acquire Strategy or Curation grammar.

### Dependency Security

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `DependencyInventorySnapshotV1` | dependency security | Represents bounded dependency inventory and explicit completeness | Be available as an independently addressable observed knowledge product | `extend existing` | Hiding it inside a Task artifact prevents graph discovery and reuse | [security contracts](../../../../../../src/dependency_security/contracts.rs) |
| `AdvisoryKnowledgeSnapshotV1` | dependency security | Represents source revision, coverage, advisories, conflicts, and completeness | Be available as an independently addressable observed knowledge product | `extend existing` | Source currency and scope can disappear in aggregate settlement | [security contracts](../../../../../../src/dependency_security/contracts.rs) |
| `DependencySecurityAssessmentV1` | dependency security | Owns policy-bound posture, findings, coverage, and reasons | Remain owner-authored semantic assessment consumable by Curation and Belief | `extend existing` | Generic Curation must not re-author dependency-security posture | [security assessment contract](../../../../../../src/dependency_security/contracts.rs) |
| `DependencySecurityVerificationV1` | dependency security | Owns independent verification of an assessment | Remain a distinct owner product rather than Task success | `extend existing` | Task termination could be mistaken for verified epistemic closure | [verification contract](../../../../../../src/dependency_security/contracts.rs) |
| `DependencySecurityAdmission` | dependency security | Validates adapter lineage and explicit completeness, then returns an in-memory canonical product reference | Continue owner admission before shared publication | `extend existing` | The current in-memory map is not a durable shared knowledge publication | [admission](../../../../../../src/dependency_security/admission.rs) |
| `DurableOperationV1` use | root runtime with dependency-security caller | Tracks a Capability operation, attempt, status, and admitted product reference | Serve as evidence of bounded-operation lifecycle needs without becoming Curation semantics by default | `reuse unchanged` | Reusing a Capability-specific operation record could make Curation an Execution concern | [portable operation](../../../../../../src/dependency_security/portable.rs), [lifecycle record](../../../../../../src/runtime/lifecycle.rs) |
| four read-and-emit Capabilities | dependency security | Models inventory, advisory acquisition, assessment, and verification as Execution Capabilities | Distinguish executable observation from epistemic assessment and verification in the Strategy cause catalog | `extend existing` | The current executable-only shape reproduces the docs design flaw | [capabilities](../../../../../../src/dependency_security/capability.rs) |
| installed security Strategy theory | dependency security package source | Exposes all four products as Capability-backed operators yielding one aggregate artifact type | Participate in a heterogeneous Plan without teaching Execution epistemic semantics | `extend existing` | Leaving the package unchanged would keep epistemic causes indistinguishable from Tasks | [security Strategy theory](../../../../../../theory/dependency_security/strategy_theory.security.json) |

Domain synthesis: dependency security independently validates the architectural pressure. It already has bounded owner records, completeness, admission, and verification, yet exposes all causal work as executable Capability operators. The impact is primarily product publication and installed theory shape, not a new Execution semantic.

### Docs

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `DirectoryEvidence` | docs | Stores direct file names, child directories, and bounded source text for one generated README target | Supply stable observed source identities from which addressable source claims can be authored | `extend existing` | A text blob cannot support durable claim identity or cross-run provenance | [docs evidence](../../../../../../src/docs/capability.rs) |
| `DocsEvidenceBundle` | docs | Aggregates meaningful directories under one source fingerprint and excludes managed README bytes | Stop being the only observation input for freshness assessment | `extend existing` | Exclusion makes an already-correct README epistemically invisible | [scope inspection](../../../../../../src/docs/capability.rs) |
| observed README revision | workspace and docs observation boundary | No docs-owned public product exists for current README bytes and extracted claims | Provide a stable observed README product distinct from an expected README | `new local behavior` | Reusing expected identity would assert physical existence from a norm | [README exclusion](../../../../../../src/docs/capability.rs), [claim types](../../../../../../src/docs/claim_validation.rs) |
| observed source claim | docs observation owner | No independently addressable source-claim product exists | Provide stable source claims with source revision and exact evidence provenance | `new local behavior` | Curation cannot author materiality edges to an aggregate evidence blob | [directory evidence](../../../../../../src/docs/capability.rs) |
| `ReadmeClaim` | docs | Deterministically identifies a claim inside candidate generated README content | Apply the same exact claim discipline to observed README revisions | `extend existing` | Candidate and observed claims may be conflated without source-kind identity | [claim contract](../../../../../../src/docs/claim_validation.rs) |
| `ClaimAssessment` and `ReadmeClaimReport` | docs | Attach semantic verdicts and citations to candidate patch claims | Remain docs-owned verification products that Curation can connect to expected coverage | `extend existing` | Curation must not guess docs correctness from generic edges | [claim assessment](../../../../../../src/docs/claim_validation.rs) |
| `ValidatedDocsPatchSet` | docs | Fences accepted reports to exact candidate bytes before publication | Remain the executable write-path validation product | `reuse unchanged` | Turning it into the only correctness product forces a write before knowledge | [validated patch set](../../../../../../src/docs/claim_validation.rs) |
| `DocsPublicationReceipt` | docs | Records files written and aggregate validation measures | Remain evidence of external mutation, not a prerequisite for every freshness assessment | `reuse unchanged` | Publication would remain falsely synonymous with epistemic settlement | [publication receipt](../../../../../../src/docs/capability.rs) |
| `DocsFreshnessAssessment` | docs | Computes aggregate stale probability only from the current inspection and publication receipt | Admit an epistemic-only assessment path over observed README and source claims | `extend existing` | Aggregate counts erase exact claim and snapshot causality | [published-scope assessment](../../../../../../src/docs/capability.rs) |
| docs Capability chain | docs | Executes inspect, draft, validate, publish, and assess as one artifact chain | Remain available when the reconciled Plan contains actual workspace change | `reuse unchanged` | Reusing the chain for pure curation preserves the current defect | [published contracts](../../../../../../src/docs/capability.rs) |
| docs claim policy revision | docs | Installs exact policy identity and thresholds | Remain docs-owned policy used by docs verification and selectable by Curation operation theory | `reuse unchanged` | Moving policy interpretation into Traversal or Strategy would violate producer ownership | [claim policy registry](../../../../../../src/docs/claim_validation/registry.rs) |
| docs PDS package | docs package source with routed owners | Installs belief, outcome mapping, Agent Goal curation, maintained condition, Strategy theory, authority, claim policy, and five executable contracts | Carry the owner components needed to select Curation and heterogeneous Strategy theory | `extend existing` | The current package uses the name curation only for Goal thresholding | [package manifest](../../../../../../theory/docs_freshness/pds-package.json) |
| installed docs Strategy theory | world-model-owned component sourced by docs | Closes freshness through the executable publication chain | Express epistemic establishment as a distinct cause class so writing can be absent | `extend existing` | A prospective evidence route cannot run, gate, or close an epistemic-only Plan | [docs Strategy theory](../../../../../../theory/docs_freshness/strategy_theory.docs_freshness.json) |
| installed outcome interpretation | world-model-owned component sourced by docs | Maps unobserved genesis and an Execution success artifact into freshness evidence | Admit Curation-owned semantic outcomes where they are evidence for belief | `extend existing` | Graph visibility alone does not revise Belief | [outcome interpretation](../../../../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json) |

Domain synthesis: docs is the only proven root semantic producer that must gain new behavior for the README proof. The smallest root-owned change is not another write Capability. It is independently addressable observed README, source claim, README claim, and verification products that Curation can connect under exact policy and snapshot provenance.

### Events

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| root Events re-export | root adapter | Exposes `meld-events` contracts unchanged | Continue carrying producer-owned semantic Events | `reuse unchanged` | Root could accidentally claim Event payload meaning | [events surface](../../../../../../src/events.rs) |
| `ProductEventBinding` | root events | Resolves one product Event authority and legacy cutover state | Continue supplying one canonical ledger | `reuse unchanged` | A second Curation ledger would split ordering and replay | [authority binding](../../../../../../src/events/binding.rs) |
| `ProductEventAppendPort` | root runtime adapter | Appends envelopes idempotently and returns ledger sequence without interpreting payload | Carry Curation lifecycle and semantic result Events | `reuse unchanged` | Adding Curation validation here would violate producer ownership | [append port](../../../../../../src/runtime/ports.rs) |
| `ProductEventReplayPort` | root runtime adapter | Supplies bounded replay without owning consumer cursors | Supply Curation change discovery and recovery | `reuse unchanged` | Replay must not be treated as fresh authority to rerun settled work | [replay port](../../../../../../src/runtime/ports.rs) |
| `ProductGraphCursorPort` | root runtime adapter | Reports Traversal reduction progress | Remain one visibility barrier available to Plan progression | `reuse unchanged` | An append receipt does not prove graph visibility | [graph cursor port](../../../../../../src/runtime/ports.rs) |

Domain synthesis: no Event carrier or authority behavior change is proven in the root crate. The runtime must bind existing append and replay capabilities to Curation, while semantic contracts and idempotent identities remain producer-owned.

### Execution Adapter

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `ExecutionGoalCommandPort` | root adapter | Maps one `AgentGoalCommand` and its one `StrategyCandidate` into `GoalAcceptanceRequest` and `ExecutionStrategyAuthorization` | Map only an eligible complete executable Plan product into the existing Execution seam | `extend existing` | Sending the heterogeneous Plan would make Execution interpret epistemic causality | [Goal command adapter](../../../../../../src/runtime/ports.rs) |
| `ExecutionGoalMutationPort` | root adapter | Maps world-model satisfy and reopen commands into Execution Goal lifecycle mutations | Continue adapting root Goal lifecycle only | `reuse unchanged` | Task completion must not directly settle the Agent Goal | [Goal mutation adapter](../../../../../../src/runtime/ports.rs) |
| `CurationGoalExecutionPort` | root adapter | Combines Goal command and mutation sinks for current Agent Goal curation | Remain a Goal Set adapter and not become the Epistemic Curation port | `extend existing` | Its current name can conceal a semantic collision between Goal curation and graph Curation | [current named port](../../../../../../src/runtime/ports.rs) |
| `ExecutionAgentGoalQueryPort` | root adapter | Adapts Execution Goal records into Agent active-goal summaries | Continue exposing root Goal lifecycle to Agent reconciliation | `reuse unchanged` | Intermediate Strategy obligations could leak into the Execution Goal Set | [Goal query adapter](../../../../../../src/runtime/ports.rs) |
| `ExecutionRuntimeContext` | root execution | Gives executable Capabilities context, provider, workspace, Event, and world-model query access | Remain the Task invocation context | `reuse unchanged` | Using it for Epistemic Operations would put Curation behind Execution | [execution ports](../../../../../../src/execution/ports.rs) |

Domain synthesis: the one proven root Execution write is the adapter that currently copies a single executable Strategy candidate. The Execution crate need not learn heterogeneous Plan semantics. Root must adapt the new world-model executable product into the same Execution-owned acceptance shape.

### Harness

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| harness operational topology | root harness | Hardcodes graph, Belief, Agent Goal curation, Execution Goals, planning, network, dispatch, publication, evidence, and satisfaction stations | Observe Curation and Strategy Plan reconciliation as separate world-model stations | `extend existing` | Diagnostics could preserve the obsolete one-way executable story | [topology projection](../../../../../../src/harness/projections.rs) |
| harness causal walk | root harness | Walks evidence, belief revisions, Agent decisions, Goals, and Task records | Expose Plan revision, Epistemic Operation, Curation result, and exact Event provenance when those records exist | `extend existing` | A walk that stops at Agent Goal decisions cannot explain epistemic closure | [causal walk](../../../../../../src/harness/walk.rs) |
| harness eligibility explanation | root harness | Deepens current actor waiting declarations and known absent records | Distinguish waiting for Curation completion, graph visibility, Belief revision, and Agent acceptance | `extend existing` | One generic completion state would hide cursor barriers | [eligibility analysis](../../../../../../src/harness/eligibility.rs) |
| boot and manifest fixtures | root harness | Compose selected actors and record initialization stages | Remain a proof adapter for selected runtime subsets | `extend existing` | Test-only topology can accidentally become architecture authority | [harness boot](../../../../../../src/harness/boot.rs), [manifest](../../../../../../src/harness/manifest.rs) |

Domain synthesis: harness changes are observability and evidence work, not runtime semantics. The domain should reflect owner-produced records after those contracts exist and must not define Plan or Curation meaning itself.

### Initialization

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `WorldInitStage` | root initialization | Orders theory installation, Agent genesis, and epistemic genesis Event publication | Preserve the staged owner-command model | `reuse unchanged` | Adding a separate Curation bootstrap path could bypass the canonical Event spine | [world-init stages](../../../../../../src/init/world.rs) |
| `WorldInitTheoryBundle` | root initialization | Carries a fixed set of belief, Goal curation, maintained condition, outcome mapping, Strategy, executable Capability, authority, and docs policy bodies | Include or route exact Curation-owned theory needed by the selected package | `extend existing` | A root-owned fixed theory image can absorb owner semantics | [theory bundle](../../../../../../src/init/world/pipeline.rs) |
| `CompleteTheoryInstall` | root initialization | Injects exact owner stores and selected package into the install stage | Route new owner components through their owning registries | `extend existing` | Direct root writes would violate the installed owner boundary | [complete install](../../../../../../src/init/world/pipeline.rs) |
| `current_product_route_catalog` | root initialization adapter | Publishes handlers for current world-model, Execution, docs, and security component kinds | Publish any new world-model Curation route through owner validation and installation ports | `extend existing` | Root handler code currently decodes owner bodies and can become a semantic bottleneck | [route catalog](../../../../../../src/init/world/routes.rs) |
| Agent genesis content | root initialization adapter with world-model owner | Registers one Agent, perspective, branch, directive, observation scope, and current Goal curation rule | Bind any standing epistemic Curation specification to the same exact Agent perspective and provenance | `extend existing` | Reusing the Goal curation rule identity would merge distinct domains | [world-init content](../../../../../../src/init/world/pipeline.rs) |
| unobserved-scope genesis Event | world-model producer through root pipeline | Seeds the initial epistemic mismatch through canonical append | Remain the first observed knowledge input | `reuse unchanged` | Genesis must not directly create an executable Task | [initialization contract](../../../../../../src/init/world.rs) |

Domain synthesis: initialization remains an adapter over owner commands and one Event authority. Its fixed theory image and Agent genesis binding are incomplete for a distinct Curation domain, while stage ordering and genesis Event publication remain sound.

### Runtime Composition

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `ProductRuntimeAssembly` | root runtime | Opens scoped stores, composes ports, resolves factories, and builds semantic handles | Compose the world-model Curation actor and heterogeneous Plan progression owner without implementing their semantics | `extend existing` | Root assembly could become the Plan orchestrator | [runtime assembly](../../../../../../src/runtime/assembly.rs) |
| `RegistrationSet` and `RuntimeRegistration` | root runtime | Declare exact active actors and passive services derived from stewardship | Include newly required world-model participants as explicit bindings | `extend existing` | Descriptor inventory must not imply product cardinality or semantic ownership | [runtime registration](../../../../../../src/runtime/registration.rs) |
| `StewardshipTheoryBindings` | root runtime | Carries resolved belief, outcome mapping, Strategy, executable Capability, authority, and planning views | Carry exact Curation theory and heterogeneous Strategy theory references into owner actors | `extend existing` | Root may flatten separate owner products into one universal theory body | [theory bindings](../../../../../../src/runtime/assembly.rs) |
| `ResolvedStewardshipTheory` | root runtime | Hydrates a fixed package receipt into current owner revisions | Resolve any new Curation and Plan component kinds without changing generic package semantics | `extend existing` | Current resolution requires exactly one fixed set and treats docs claim policy as universal | [resolved theory](../../../../../../src/runtime/theory.rs) |
| `ProductRuntimePorts` | root runtime | Supplies Event, Goal, planner projection, context, provider, workspace, artifact, and network ports | Supply existing Event capabilities and explicit owner ports to Curation and Agent Plan progression | `extend existing` | A generic root operation port could erase Task versus Epistemic Operation | [runtime ports](../../../../../../src/runtime/ports.rs) |
| `PlannerProjectionPort` | root runtime adapter | Builds the current flat world-model projection from Belief and Traversal stores | Adapt any relation-rich frozen knowledge cut exposed by world-model without interpreting it | `extend existing` | Root could become the graph hydration or relevance owner | [planner projection adapter](../../../../../../src/runtime/ports.rs) |
| `AgentActorHandle` | root runtime adapter | Ticks Goal or satisfaction curation and routes both through the Execution Goal Set port | Tick Agent Plan reconciliation and route enabled product kinds to their separate owners | `extend existing` | The current direct Goal-to-Execution path assumes the Strategy candidate is the whole plan | [Agent handle](../../../../../../src/runtime/assembly.rs) |
| `PlanningHandle` | root runtime adapter over Execution | Runs Execution Planning and lowers accepted executable compositions into the Task Network | Continue receiving complete executable products only | `reuse unchanged` | Adding EpiOps here would violate the Execution invariant | [Execution Planning handle](../../../../../../src/runtime/assembly.rs) |
| `DurableOperationV1` and attempts | root runtime lifecycle | Track Capability-backed adapter operations by activation and generation | Remain operational evidence unless world-model explicitly adopts an equivalent Curation-owned lifecycle | `reuse unchanged` | Reusing it implicitly would move epistemic operation ownership to root runtime | [durable operation](../../../../../../src/runtime/lifecycle.rs) |
| actor waiting and status records | root runtime | Project bounded actor progress, checkpoints, issues, and waiting declarations | Observe new owner-reported actors and visibility barriers generically | `extend existing` | Root status must not infer semantic completion | [runtime contracts](../../../../../../src/runtime/contracts.rs) |

Domain synthesis: root runtime composition is the largest root write surface, but it remains an adapter. It must be able to instantiate owner-provided Curation and Plan reconciliation behavior, route Tasks and Epistemic Operations to different owners, and preserve the existing Event and Execution seams. It must not construct the Plan or author graph knowledge.

### Task

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| compiled Task contracts | `meld-execution`, re-exported by root | Represent execution-owned compiled nodes, dependencies, inputs, and artifacts | Remain the product after Execution lowers an authorized executable Strategy product | `reuse unchanged` | Calling the heterogeneous Plan a Task would collapse ownership | [Task surface](../../../../../../src/task.rs) |
| `TaskArtifactRepo` | Execution with root facade | Persists Task artifacts by execution identity | Continue storing executable products only | `reuse unchanged` | Curation results hidden as Task artifacts cannot become shared knowledge | [artifact repository](../../../../../../src/task/artifact_repo.rs) |
| Task package compatibility | root Task and workflow | Loads and prepares legacy workflow-backed package routes | Remain isolated from canonical Strategy Plan meaning | `reuse unchanged` | The old workflow path could be mistaken for the new causal Plan | [package facade](../../../../../../src/task/package.rs) |
| Task lifecycle Events | Execution with root facade | Publish executable attempt and outcome facts | Continue as external-action observation inputs | `reuse unchanged` | Task success must not equal Goal or epistemic satisfaction | [Task Events](../../../../../../src/task/events.rs) |

Domain synthesis: Task is on the runtime path and is intentionally outside the likely write set. The proposed architecture clarifies its scope rather than expanding it.

### Theory

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `TheoryRouteId` | root theory | Names owner domain, component kind, and route version without interpreting content | Carry new Curation component kinds unchanged | `reuse unchanged` | Root could hardcode semantic relationships between kinds | [route identity](../../../../../../src/theory/contracts.rs) |
| `TheoryRouteHandler` | semantic owners through root adapter | Validates, installs, verifies, and link-checks one exact owner component | Continue enforcing producer-owned theory admission | `reuse unchanged` | Consumer-side duplicate semantic guards would violate ownership | [route handler](../../../../../../src/theory/contracts.rs) |
| `PdsPackageManifestV1` | root theory | Materializes exact relative or published component content and validates structural dependency closure | Carry heterogeneous Strategy and Curation components without schema redesign | `reuse unchanged` | Package structure must not become a universal domain grammar | [package manifest contract](../../../../../../src/theory/package.rs) |
| `TheoryRouter` | root theory | Installs validated components in dependency order and records exact receipts | Reuse for new owner components | `reuse unchanged` | Router must not interpret Plan or Curation semantics | [router](../../../../../../src/theory/router.rs) |
| `PdsPackageInstallationReceiptV1` | root theory | Records installed exact owner revisions | Remain the exact activation evidence for every selected component | `reuse unchanged` | Runtime shortcuts around the receipt would weaken reconciliation provenance | [receipt](../../../../../../src/theory/receipt.rs) |

Domain synthesis: the generic theory domain already has the right producer-owned routing architecture. The adaptation belongs in the installed package sources, owner route publications, and fixed runtime resolution view, not in the generic router or package grammar.

### Workspace

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `WorkspaceScanRequest` and `WorkspaceScanOutcome` | workspace | Produce a bounded snapshot, root reference, observed node refs, and publication candidates | Remain the source of complete physical observation under one snapshot | `reuse unchanged` | Curation must not infer absence without bounded scan completeness | [scan contract](../../../../../../src/workspace/scan.rs) |
| `WorkspaceSnapshotEventData` | workspace | Publishes source, snapshot, and selected snapshot identity | Anchor Curation realization assessments to the exact observed snapshot | `reuse unchanged` | A later snapshot must not silently rewrite an earlier assessment | [workspace Events](../../../../../../src/workspace/events.rs) |
| `WorkspaceNodeObservedEventData` | workspace | Publishes node identity, path, parent, and snapshot with graph attachments | Supply stable physical README and source-file identities for typed hydration | `extend existing` | Traversal facts omit payload and cannot recover path or node kind from the generic object alone | [node observation Event](../../../../../../src/workspace/events.rs) |
| workspace node and containment relations | workspace | Publishes `belongs_to`, `observed_in`, and `contains` edges | Remain structural observation authored by workspace | `reuse unchanged` | Curation must not author physical containment or observation |
| `NodeRecord` | root store under workspace access | Stores node identity, path, type, deletion state, parent, and metadata | Remain the typed physical hydration source when Event payload is insufficient | `reuse unchanged` | Direct world-model store access would breach the workspace contract | [node record](../../../../../../src/store.rs) |
| `WorkspaceRuntimePort` | root runtime adapter | Exposes the workspace node store to current runtime factories | Provide only an explicit typed hydration boundary if current Event products cannot satisfy Curation | `extend existing` | A broad store port would let Curation bypass promoted knowledge and snapshot provenance | [workspace runtime port](../../../../../../src/runtime/ports.rs) |

Domain synthesis: workspace already authors the physical identity and snapshot relationships required for expected-versus-observed reasoning. The unresolved root impact is typed hydration. Current Traversal preserves object coordinates but not workspace Event payload, so a Curation consumer cannot recover path and node details from a walk alone. The evidence does not yet select richer workspace publication, Event hydration, or a narrow workspace query port.

### World State Root Adapter

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| root `world_state` re-export | root adapter | Re-exports every public `meld-world-model` contract | Expose new owner contracts without root semantic wrappers | `reuse unchanged` | Root wrappers could create a second vocabulary | [world-state surface](../../../../../../src/world_state.rs) |
| `PlannerProjectionPort` | root runtime adapter | Instantiates world-model query objects over world-model stores | Adapt exact public world-model query contracts only | `extend existing` | Root must not choose graph relevance or Curation currentness | [planner projection port](../../../../../../src/runtime/ports.rs) |
| graph runtime assembly | root runtime adapter | Instantiates `GraphRuntime` over the Event authority and world-model storage | Continue composing Traversal materialization as an independent actor | `reuse unchanged` | Curation must not mutate Traversal storage directly | [graph assembly](../../../../../../src/runtime/assembly.rs) |

Domain synthesis: `world_state` itself remains a transparent adapter. Concrete root impact appears in runtime construction and ports only where the new world-model public contracts must be wired.

## Recursive Upward Synthesis

### Product Observation Domains

Workspace already publishes structural physical facts for source, snapshots, nodes, containment, and observation. Docs does not publish the semantic observations needed by the README proof. Its source evidence is aggregated text, its README claims belong only to generated candidates, and its correctness report is fenced to a write candidate. Dependency security has stronger owner-shaped observations and explicit completeness, but keeps them behind executable Capability and in-memory admission paths rather than durable graph-addressed publication.

The common root impact is producer publication. Product owners must expose independently addressable observed records before Curation can author relationships among them. This does not make docs or dependency security responsible for Strategy Plan construction.

### Selection And Theory Domains

The generic theory router, package schema, exact route identity, and installation receipt already support new owner component kinds without acquiring their semantics. The fixed root views do not. `TheorySelection`, `SelectedStewardshipPackage`, `WorldInitTheoryBundle`, `ResolvedStewardshipTheory`, and `StewardshipTheoryBindings` enumerate the current executable-only image.

The root adaptation pressure is therefore exact selection, installation, and hydration of Curation-owned operation theory and heterogeneous Strategy theory. Generic package transport can be reused unchanged.

### Runtime And Boundary Adapters

Root runtime currently wires Agent Goal curation directly to the Execution Goal Set through `CurationGoalExecutionPort`. The Goal command adapter copies one `StrategyCandidate` with one executable `Composition` into Execution authorization. This is the precise current assumption that the Strategy product and executable product are the same package.

The heterogeneous Plan remains world-model-owned. Root runtime must only compose its owner actors and route their public products. Eligible Tasks cross the existing Execution adapter. Eligible Epistemic Operations cross a distinct Curation boundary. Shared Curation results use the existing Event append and replay capabilities. Execution Planning, Task lowering, and Task runtime remain unchanged in meaning.

### Compatibility And Observability Domains

The old workflow subsystem remains an active compatibility route for current package-backed Task dispatch. It is not an architecture input and must not become the representation of a heterogeneous Strategy Plan. Root Task compatibility can continue to realize executable work without learning Curation.

Harness topology and causal walks are evidence consumers. They currently present one executable flywheel and will not explain Plan reconciliation or epistemic closure when those owner records appear. Their impact is projection and explanation only.

## Root Crate Synthesis

The root crate is not the owner of the architecture redesign. Its substantial semantic pressure is concentrated in two producer domains and a bounded adapter set.

Docs must expose the observations that make epistemic-only freshness possible. Dependency security must stop relying on an executable Capability shape as the only expression of its epistemic calculations and adapter results. World-model owns Strategy Plan, Curation, Agent progression, and Traversal behavior. The root crate selects exact theory, composes owner runtimes, carries Events, and maps an eligible executable product into the unchanged Execution Goal Set seam.

The sharp root boundary is:

```text
root producers publish observed semantic products
-> Events
-> world-model Traversal and Curation
-> world-model Strategy Plan reconciliation
-> root routes eligible executable product to Execution
-> root routes shared epistemic results through Events
```

No root domain should construct the heterogeneous Plan, determine epistemic currentness, validate Curation semantics, or lower an Epistemic Operation into the Task Network.

## Separated Impact Scopes

### Current Runtime And Operating Path

The current docs path includes config, theory, init, capability, docs, provider, root runtime, root Execution adapters, Events, world-model, Execution, Task, workspace, context, and harness or served projections when those diagnostics are used.

The current dependency-security proof path includes config and activation records, theory, capability supply, dependency-security adapters and admission, runtime lifecycle, Execution Capability invocation, and owner products.

These runtime sets are not implementation write sets.

### Behavior That Must Change If The Proposal Is Accepted

The evidence proves behavior pressure in docs observation and semantic publication, dependency-security epistemic product publication and theory classification, root fixed theory resolution, root runtime composition, and the Agent-to-Execution adaptation of one executable product.

Harness evidence surfaces would need to observe the new owner records if the harness remains the canonical proof tool. Workspace must support typed hydration of observed nodes, but current evidence does not prove that workspace code itself must change.

### Likely Root Implementation Writes

The smallest evidence-backed likely write surface is:

```text
src/docs
src/dependency_security
src/config/stewardship
src/init/world
src/runtime/assembly.rs
src/runtime/ports.rs
src/runtime/theory.rs
theory/docs_freshness
theory/dependency_security
src/harness
```

`src/workspace` is conditional on the unresolved typed-hydration boundary. `src/execution` is not proven to need more than root adapter reuse. No `meld-execution` behavior change is established by this root assessment.

### Adapters Only

Root Events binding, Event append and replay ports, root world-state re-export, generic theory routing, planner query binding, and Execution Goal acceptance mapping remain adapters. Some adapters need new input shapes, but they do not gain domain meaning.

### Reuse Unchanged

The evidence supports unchanged reuse of the Event authority, Event envelope carrier, Event replay and cursor mechanics, generic PDS package and router contracts, physical workspace snapshot identities, executable Capability catalog, compiled Task contracts, Task artifacts, Task lifecycle Events, Execution Planning, Task Network lowering, provider clients, context frames, and the old workflow path only as isolated compatibility for executable realization.

## Explicit Non-Integration Findings

- Root runtime does not own Strategy Plan semantics.
- Root capability does not become the Epistemic Operation catalog.
- Root Events does not validate Curation vocabulary.
- Root theory does not enforce Plan grammar.
- Root config selects identities and physical bindings but does not carry theory bodies.
- Docs owns observed claim and correctness products but does not decide the Agent Plan.
- Workspace owns physical observation but does not author expected README entities.
- Dependency security owns security posture and verification but does not own generic Curation.
- Execution receives only complete executable products and remains ignorant of Epistemic Operations.
- Task completion does not establish Goal satisfaction or epistemic closure.
- The legacy workflow subsystem is not a Plan representation and receives no new architecture role.
- Harness and served projections observe owner records and do not define their meaning.

## Newly Exposed Ownership Boundary

Dependency security contains a useful but easy-to-misread precedent. `DependencySecurityAdmission` separates adapter transport from in-memory owner admission for inventory and advisory values only. `DurableOperationV1` separately records a bounded Capability operation, attempts, activation lineage, terminal status, and admitted product references. Assessment and verification have no admission or durable product path, and the declared security Events are not appended. The implemented slice validates operation identity, completeness, and terminal-accounting pressure without proving a complete semantic publication path.

It does not establish root runtime as the owner of Epistemic Operations. The record is Capability-contract based and the admission map is dependency-security owned. Reusing it without an explicit world-model owner decision would move Curation toward Execution and root lifecycle. The evidence should inform later requirements while preserving the ownership question.

The second newly exposed boundary is typed workspace hydration. Workspace Events publish enough structure to locate an observed node under a snapshot, but current Traversal facts do not preserve the Event payload that contains the path. A generic graph walk therefore cannot by itself tell Curation that a reached node is `README.md`. This is a concrete connective gap between root workspace observation and world-model knowledge consumption.

## Evidence Confidence

Confidence is high for the complete root domain snapshot, docs inspection behavior, candidate-only claim records, installed docs theory chain, root Goal-command adaptation, runtime actor composition, generic theory routing, workspace Event shape, and legacy workflow presence. Each finding is directly visible in current code or installed theory.

Confidence is high that no root Event or generic theory semantic change is required. Both current contracts already preserve producer-owned payloads and exact owner routing.

Confidence is high that dependency security models epistemic read and assessment work as Execution Capabilities today. Confidence is moderate on which of those operations should become Curation-owned because the canonical operation vocabulary has not yet been specified.

Confidence is moderate on the workspace write scope. The missing typed hydration is proven. Current evidence does not choose whether the smallest owner-correct solution enriches workspace publication, hydrates source Events, or adds a narrow typed query boundary.

Confidence is moderate on exact root theory field changes because a later canonical specification may prefer package-receipt-driven component discovery over adding more fixed selection fields.

## Unresolved Questions

- Which docs observations are owner facts and which relations are Curation-authored judgments.
- Whether observed source claims and README claims share one docs claim type or use distinct typed products.
- Which dependency-security operations are observation Tasks, Curation Epistemic Operations, or standing owner work.
- Whether an executable Plan slice carries the root Agent Goal or a separately durable execution-facing Goal reference.
- Whether root runtime routes each enabled product directly or calls one world-model progression port that performs owner routing.
- Which Event, graph, Belief, or Agent visibility milestone satisfies each Plan dependency.
- Which exact Curation theory products are selected and how the generic package receipt exposes them to runtime assembly.
- Whether workspace typed hydration is fulfilled by richer promoted observation, exact Event replay, or a narrow owner query port.
- How legacy workflow-backed Task package realization remains isolated while current dispatch still uses it.

These are evidence questions for the later canonical requirements specification. This report does not resolve them or establish implementation sequence.

## Reviewer Performance Self-Assessment

The assessment regenerated all 37 root domains and preserved explicit `none` findings before decomposing the affected set. It began from public entities, installed theory bodies, durable operation records, actor inputs and outputs, and cross-crate adapters. It then synthesized those entities into owning domains and the root crate.

The strongest findings are the exact docs inability to observe a current README, the hardcoded one-candidate Agent-to-Execution mapping, the fixed root theory image, the Event carrier reuse, the workspace typed-hydration gap, and the dependency-security operation precedent. Confidence is lower where the future owner contract remains intentionally unspecified, especially workspace hydration and the classification of individual dependency-security operations.

The review stayed within the charter. It does not draft requirements, migration steps, workstreams, acceptance tests, or implementation sequencing.
