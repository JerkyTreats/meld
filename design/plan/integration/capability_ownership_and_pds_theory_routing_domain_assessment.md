# Capability Ownership And PDS Theory Routing Domain Assessment

Date: 2026-08-15
Status: discovery assessment, not implementation authority
Scope: domain-owned capabilities, PDS package attachment through typed theory routes, a self-contained CVE runtime, and event-mediated cooperation among independent stewards

> This discovery assessment predates the accepted cognition boundary. Its exact capability routing is pressure evidence, not the canonical package shape. Use [PDS Cognition Boundary Assessment](pds_cognition_boundary_domain_assessment.md) and [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) for current ownership.

## Concern

Assess how Meld can preserve domain ownership when externally authored PDS packages contribute domain theory and executable capabilities. The concrete pressure test treats a CVE package as a complete dependency-security runtime and asks how it attaches to Meld, publishes evidence, invokes domain-owned capabilities, and participates in an event-to-belief network with independent developer and documentation stewards. The assessment tests a prospective theory-routing concern without granting it authority over the semantics carried through its routes.

## Direct Behavior

One package enters through one typed package attachment surface. Its components name routes published by owning domains. The attachment surface resolves those routes, verifies complete exact installation, and records lineage. Each owning domain validates and installs its own component. Domain-owned capability publishers contribute exact contracts and invokers to a shared catalog. After activation, canonical events and beliefs allow independent stewards to react without direct steward-to-steward commands.

## In Scope

- domain ownership of capability contracts and implementations
- product-wide capability catalog assembly
- a typed theory-routing analogue to capability publication
- package installation closure and exact revision receipts
- CVE scanning, assessment, notification, remediation, and verification boundaries
- external CVE runtime placement behind domain adapters
- event, evidence, belief, Agent, Goal, and outcome lineage across stewards
- coexistence of several independently activated stewards on one event and belief substrate
- placement-independent activation isolation and assignment-local runtime views
- external result admission across request-response and passive source-delivery adapters

## Out Of Scope

- conflict policy among independently authored stewardship packages
- automatic authority delegation from one steward to another
- a package marketplace
- one mandatory CVE scanner or advisory provider
- production connector credentials and polling reliability
- one mandatory process, sidecar, or remote-service topology
- a universal package schema for all domain bodies
- a workflow that manually sequences CVE, developer, and docs work
- implementation sequencing and acceptance tests

## Evidence Basis

Evidence date: 2026-08-15

The current domain snapshot was regenerated with:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

Independent workspace domains were added from the current Cargo workspace. The proposed `theory` and `dependency-security` domains are marked prospective because neither exists as an independent current domain.

Primary repository evidence:

- [Capability domain architecture](../../completed/capabilities/domain_architecture.md)
- [Current shared capability facade](../../../src/capability.rs)
- [Execution capability contracts](../../../crates/meld-execution/src/capability/contracts.rs)
- [Execution capability catalog](../../../crates/meld-execution/src/capability/catalog.rs)
- [Docs capability publication](../../../src/docs/capability.rs)
- [Current exact capability activation](../../../src/runtime/assembly.rs)
- [Current complete theory receipt](../../../src/runtime/theory.rs)
- [Current stewardship declaration](../../../src/config/stewardship/selection.rs)
- [Current theory initialization](../../../src/init/world/pipeline.rs)
- [Agent subscriptions](../../../crates/meld-world-model/src/agent/subscription.rs)
- [Belief outcome interpretation](../../../crates/meld-world-model/src/belief/outcome/interpretation.rs)
- [CVE Step Five discovery ground](cve_freshness_step_5_discovery_ground_map.md)
- [External domain theory attachment proposition](external_domain_theory_attachment_proposition_brief.md)
- [PDS isolation and runtime portability](../../persistent_domain_stewardship/isolation_and_runtime_portability.md)

Applicable policy:

- [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md)
- repository domain architecture rules in `AGENTS.md`

## External CVE Runtime Research

The current ecosystem contains several useful program shapes. None alone defines the right Meld package contract.

| Program | Runtime shape | Relevant behavior | Signal for Meld |
| --- | --- | --- | --- |
| [CVE Program](https://www.cve.org/about/overview) | public vulnerability identifier and record program | identifies, defines, and catalogs publicly disclosed vulnerabilities | CVE is one source identity within dependency security, not the name of the whole runtime concern |
| [OSV-Scanner](https://google.github.io/osv-scanner/usage/) | CLI and Go library | scans source, images, lockfiles, and SBOMs with structured output | credible package-local observation adapter |
| [OSV guided remediation](https://google.github.io/osv-scanner/experimental/guided-remediation/) | interactive or scripted mutation tool | evaluates transitive graphs and can update manifests and lockfiles | scanning and remediation are distinct capabilities even when shipped together |
| [Dependency-Track](https://docs.dependencytrack.org/usage/cicd/) | persistent portfolio service | ingests SBOMs and continually reanalyzes components against vulnerability sources | closest example of a complete external CVE runtime |
| [Dependency-Track notifications](https://docs.dependencytrack.org/integrations/notifications/) | event and scheduled publisher | emits typed vulnerability, policy, audit, and BOM notifications through webhooks | strong evidence for event-mediated attachment |
| [Snyk Monitor](https://docs.snyk.io/developer-tools/snyk-cli/commands/monitor) | hosted persistent monitor seeded by a project snapshot | reruns vulnerability analysis as knowledge changes | external runtime can retain domain source truth while Meld retains admitted evidence and provenance |
| [Dependabot alerts](https://docs.github.com/en/code-security/concepts/supply-chain-security/dependabot-alerts) | repository-integrated monitor | reacts to advisory and dependency graph changes | validates revision-driven wake rather than one-shot scanning |
| [Dependabot security updates](https://docs.github.com/en/code-security/concepts/supply-chain-security/dependabot-security-updates) | remediation bot | raises dependency update pull requests linked to alerts | detection and code mutation can be separated or combined |
| [Renovate](https://docs.renovatebot.com/configuration-options/) | scheduled dependency update bot | can consume vulnerability alerts and create security update pull requests | an independent developer steward can consume security findings |
| [Syft and Grype](https://oss.anchore.com/docs/architecture/grype/) | composable inventory and scanner tools | separates SBOM production from vulnerability matching | supports capability decomposition inside one package distribution |
| [Trivy server](https://trivy.dev/docs/v0.72/guide/references/modes/client-server/) | scanner service with centralized vulnerability database | serves remote scans for repositories, filesystems, images, and roots | executable placement can vary without changing package meaning |
| [GUAC](https://docs.guac.sh/guac/graphql/) | supply-chain knowledge graph | records package, vulnerability, source, scan time, and database revision relationships | validates provenance-rich graph facts and explicit clean certifications |
| [Codex Security](https://help.openai.com/en/articles/20001107-codex-security) | agentic security runtime | identifies, validates, proposes a patch, and revalidates after remediation | validates the independent remediation and verification steward shape |
| [GitHub agent assignment](https://docs.github.com/en/code-security/concepts/supply-chain-security/dependabot-alerts) | alert-to-agent handoff | assigning an alert to an AI agent starts a session and draft pull request | the proposed frontier-agent developer steward already has a close production analogue |

### Research Synthesis

A complete CVE package may legitimately wrap a local CLI, embed a library, run a sidecar service, or bind a hosted monitor. Package identity must therefore remain independent of executable placement.

Placement alone does not settle isolation. A linked library, subprocess, shared sidecar, and hosted monitor must each preserve assignment, grant, binding, source, runtime-generation, and owner-admission lineage. They may offer different hard containment and shared-failure properties.

Request-response capabilities keep a stable operation key separate from attempt id and runtime generation. Persistent monitor callbacks use authenticated subscription, delivery, cursor, and source-revision lineage instead of fabricating an execution claim.

The package should preserve separate domain results for inventory, advisory assessment, remediation feasibility, code mutation, and verification. Existing programs often combine several of these operations, but Meld should not collapse their evidence merely because one executable produced it.

Dependency-Track is the closest match to the complete CVE runtime thought experiment. GUAC is the closest match to provenance-rich security knowledge. Dependabot, Renovate, and current coding-agent integrations validate the independent developer-steward handoff. OSV-Scanner and Grype are strong deterministic first-proof adapters.

## Architectural Analogy

Current capability ownership follows this pattern:

```text
domain-owned contract and invoker
        ↓ publish
shared capability catalog
        ↓ exact selection
execution-owned invocation
```

One canonical exact capability contract may have several physical implementation offers. Product or administrator composition publishes those offers. Activation selects one into an assignment-local executor view. A PDS package declares compatible action-class semantics but does not select an exact capability contract, native library, executable, endpoint, or owner identity.

The prospective theory attachment should follow the same pattern:

```text
domain-owned theory route and handler
        ↓ publish
shared theory route catalog
        ↓ package component dispatch
domain-owned validation and installation
        ↓ exact revision reference
shared installation closure and receipt
```

The public API router analogy is valid at the routing boundary. It becomes invalid if the router translates every body into runtime semantics. A router owns addresses, route matching, closure, errors, and receipts. The destination domain owns request meaning and behavior.

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `adapter` | root facade re-exports world-model Agent contracts | `partial` | `src/agent.rs` |  | expose settled generic multi-steward contracts only |
| `api` | `none` | no package API is required for the semantic proof | `not needed` | `src/api.rs` | a public HTTP facade would not establish ownership | none |
| `branches` | `none` | branch scope already participates in belief identity | `not needed` | `src/branches.rs` | no branch behavior changes | none |
| `capability` | `own` | shared contract facade exists, but product inventory publishes docs only | `partial` | `src/capability.rs` |  | aggregate domain-owned publishers without expression dispatch |
| `cli` | `none` | current init and run commands can host focused proofs | `not needed` | `src/cli.rs` | a new command would be presentation only | none |
| `compat` | `none` | only existing docs compatibility lowering applies | `not needed` | `src/compat.rs` | no CVE legacy form exists | none |
| `concurrency` | `none` | helper concurrency does not own task effect arbitration | `not needed` | `src/concurrency.rs` | execution owns conflicting effect coordination | none |
| `config` | `own` | declarations select a fixed theory shape and one physical binding | `partial` | `src/config/stewardship` |  | represent package entry, assignment scope, and activation requirements |
| `context` | `publish` | context defines domain-owned capability invokers | `partial` | `src/context/capability.rs` |  | publish through the same product contributor surface |
| `control` | `none` | control plans do not own steward cooperation | `not needed` | `src/control.rs` | event and belief causality replaces a manual control chain | none |
| `docs` | `publish` | docs owns exact capability contracts and claim policy | `partial` | `src/docs/capability.rs` |  | expose capability and theory contributions through common routes |
| `error` | `none` | no stable new error surface is frozen | `not needed` | `src/error.rs` | route errors follow contract design later | none |
| `events` | `own` | product event authority provides append, replay, and durable cursors | `complete` | `src/events.rs` |  | reuse canonical spine without steward-specific channels |
| `execution` | `adapter` | root facade maps capability, Goal, planning, and task-network contracts | `partial` | `src/execution.rs` |  | route settled generic contracts only |
| `harness` | `none` | harness is not semantic authority | `not needed` | `src/harness.rs` | proof fixtures may use it later without changing ownership | none |
| `heads` | `none` | legacy frame heads are unrelated | `not needed` | `src/heads.rs` | no head behavior is required | none |
| `ignore` | `none` | ignore policy does not define dependency scope | `not needed` | `src/ignore.rs` | dependency-security owns package selection | none |
| `init` | `adapter` | fixed root pipeline installs known owner revisions | `partial` | `src/init/world` |  | route package components to owner installers and collect receipts |
| `lib` | `adapter` | root crate exports current product contracts | `partial` | `src/lib.rs` |  | export only settled generic surfaces |
| `logging` | `none` | logs are not correctness state | `not needed` | `src/logging.rs` | provenance belongs in durable records | none |
| `merkle_traversal` | `publish` | traversal defines a domain-owned capability invoker | `partial` | `src/merkle_traversal/capability.rs` |  | publish through the common contributor surface |
| `metadata` | `none` | frame metadata does not own package theory | `not needed` | `src/metadata.rs` | CVE evidence retains its own schema | none |
| `prompt_context` | `none` | prompt artifacts are not required by a CVE package | `not needed` | `src/prompt_context.rs` | developer-agent context is a provider implementation detail | none |
| `provider` | `publish` | provider defines a domain-owned execution capability | `partial` | `src/provider/capability.rs` |  | publish normally and bind only when a package requires it |
| `runtime` | `own` | root resolves one fixed receipt and activates docs invokers directly | `partial` | `src/runtime/theory.rs`, `src/runtime/assembly.rs` |  | compose route catalogs, exact images, and several active stewards |
| `serve` | `none` | no service API is needed for the proof | `not needed` | `src/serve.rs` | external runtimes attach through domain adapters | none |
| `session` | `none` | command sessions are not durable domain facts | `not needed` | `src/session.rs` | external execution sessions remain capability implementation detail | none |
| `store` | `none` | generic storage primitives do not own theory meaning | `not needed` | `src/store.rs` | owners may reuse storage without new shared semantics | none |
| `task` | `consume` | task compilation and runtime consume capability catalogs | `complete` | `src/task.rs`, `src/task/runtime.rs` |  | reuse exact capability consumption |
| `telemetry` | `none` | observability does not establish causal truth | `not needed` | `src/telemetry.rs` | durable events and evidence carry provenance | none |
| `tree` | `none` | tree behavior is unrelated | `not needed` | `src/tree.rs` | dependency graphs are domain facts, not filesystem tree semantics | none |
| `types` | `none` | no universal CVE types belong in shared root types | `not needed` | `src/types.rs` | use generic identifiers and owner contracts | none |
| `views` | `none` | no presentation contract is required | `not needed` | `src/views.rs` | stewardship projections are later product work | none |
| `workflow` | `none` | compatibility task paths assemble several current capabilities | `not needed` | `src/workflow/task_path.rs` | workflow must not become the steward chaining mechanism | none |
| `workspace` | `publish` | workspace owns scan, resolution, and write capabilities | `partial` | `src/workspace/capability.rs` |  | publish through common catalog and emit exact change evidence |
| `world_state` | `adapter` | root facade exposes world-model queries and projections | `partial` | `src/world_state.rs` |  | carry owner-published facts without interpreting them |
| `meld-events` | `own` | neutral event, subject, provenance, append, and replay contracts exist | `complete` | `crates/meld-events/src` |  | reuse unchanged |
| `meld-execution` | `own` | owns capability contracts, catalog, invocation, Goals, planning, and tasks | `partial` | `crates/meld-execution/src` |  | preserve capability ownership and support exact contributor closure |
| `meld-lang` | `consume` | generic propositions, Goals, effects, and evaluation carry no package vocabulary | `complete` | `crates/meld-lang/src` |  | reuse unchanged for domain-published meaning |
| `meld-world-model` | `own` | owns evidence, belief, subscriptions, maintained conditions, Agent decisions, and Strategy | `partial` | `crates/meld-world-model/src` |  | support several activated stewards reacting to shared facts |
| prospective `theory` | `own` | no independent router exists; root currently coordinates a fixed image | `not started` | `src/runtime/theory.rs`, `src/init/world` |  | settle whether routing and receipts justify an independent domain |
| prospective `dependency-security` | `own` | no current domain implementation exists | `not started` | CVE use case and research evidence |  | own CVE program adapters, verdicts, policies, and capabilities |

## Frozen Affected-Domain Set

The frozen affected set is:

```text
agent
capability
config
context
docs
events
execution
init
lib
merkle_traversal
provider
runtime
task
workspace
world_state
meld-events
meld-execution
meld-lang
meld-world-model
prospective theory
prospective dependency-security
```

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Agent facade | `agent` | thin re-export surface | expose generic subscription and decision contracts | `adapter only` | root acquires Agent semantics | `src/agent.rs` |
| Capability vocabulary | `capability` | re-exports exact execution contracts | remain shared neutral vocabulary | `reuse unchanged` | shared vocabulary gains domain behavior | `src/capability.rs` |
| Product capability inventory | `capability` | publishes docs contracts only | aggregate deterministic domain publishers | `extend existing` | aggregation becomes expression dispatch | `src/capability.rs` |
| Package selection | `config` | fixed `TheorySelection` and concrete subject | select one package entry and exact route requirements | `extend existing` | root schema enumerates every owner body | `src/config/stewardship/selection.rs` |
| Physical activation | `config` | workspace, subject, agent, principal, and provider are universal | bind only declared package requirements | `extend existing` | package identity becomes deployment topology | `src/config/stewardship/binding.rs` |
| Context capability publication | `context` | invokers own context contracts | contribute contracts and factories through common publisher route | `extend existing` | context internals leak into catalog assembly | `src/context/capability.rs` |
| Docs capability publication | `docs` | complete domain-owned contract set and exact registration | implement common publisher and activation route | `extend existing` | docs remains privileged in root | `src/docs/capability.rs` |
| Docs theory contribution | `docs` | claim policy has exact owner registry but mandatory root slot | publish docs-owned theory route and revision reference | `extend existing` | claim policy is generalized into common theory | `src/docs/claim_validation.rs` |
| Canonical append and replay | `events` | one product event authority | carry every steward fact through the same spine | `reuse unchanged` | steward-specific side channels | `src/events.rs` |
| Event binding adapter | `events` | root binds physical authority | keep physical assembly independent of package meaning | `adapter only` | event binding branches on package | `src/events/binding.rs` |
| Execution facade | `execution` | maps root runtime ports to execution contracts | route generic capability and Goal contracts | `adapter only` | facade interprets security or developer meaning | `src/execution.rs` |
| Theory initialization coordinator | `init` | loads a fixed list of known files and owner stores | dispatch components by published theory route and gather exact receipts | `extend existing` | root becomes universal compiler | `src/init/world` |
| Crate exports | `lib` | exposes product modules | export settled route and contributor contracts only | `adapter only` | premature public API freeze | `src/lib.rs` |
| Traversal capability publication | `merkle_traversal` | domain invoker owns traversal behavior | contribute through common publisher route | `extend existing` | traversal becomes package-specific | `src/merkle_traversal/capability.rs` |
| Provider capability publication | `provider` | domain invoker owns provider transport | contribute through common publisher route | `extend existing` | provider becomes mandatory for every package | `src/provider/capability.rs` |
| Provider activation | `provider` | one binding is universal in stewardship declaration | bind only packages whose selected contracts require it | `extend existing` | semantic package identity depends on provider choice | `src/config/stewardship/binding.rs` |
| Exact image resolution | `runtime` | resolves fixed common refs plus docs policy | resolve common refs and owner-issued extension receipts | `extend existing` | root deserializes foreign bodies | `src/runtime/theory.rs` |
| Runtime capability activation | `runtime` | calls docs exact registration directly | invoke domain publisher factories selected by exact contracts | `extend existing` | factory selection branches on expression name | `src/runtime/assembly.rs` |
| Steward composition | `runtime` | one optional stewardship composition | host several installed Agent and theory bindings over one runtime | `extend existing` | shared facts lose package and assignment lineage | `src/runtime/assembly.rs` |
| Task compilation | `task` | consumes exact capability catalog | compile domain-owned contracts without knowing package origin | `reuse unchanged` | task acquires package semantics | `src/task.rs` |
| Task invocation | `task` | resolves exact invoker and artifacts | preserve producer, assignment, and package lineage | `extend existing` | runtime artifacts erase steward provenance | `src/task/runtime.rs` |
| Workspace observation capabilities | `workspace` | domain owns scan and object resolution | publish through common contributor route | `extend existing` | dependency parsing leaks into workspace | `src/workspace/capability.rs` |
| Workspace mutation evidence | `workspace` | writes and publications emit domain outcomes | publish exact changed artifact and commit evidence for downstream beliefs | `extend existing` | commit is treated as proof of security resolution | `src/workspace/publish.rs` |
| World-state facade | `world_state` | adapts world-model query surfaces | carry dependency-security and workspace facts neutrally | `adapter only` | facade validates CVE meaning | `src/world_state.rs` |
| Event identities and provenance | `meld-events` | generic domain, stream, subject, sequence, and causation contracts | retain producer and evidence lineage across steward reactions | `reuse unchanged` | adding steward-specific event variants | `crates/meld-events/src` |
| Capability contracts | `meld-execution` | exact owner, scope, bindings, inputs, outputs, effects, and execution class | remain the sole executable contract grammar | `reuse unchanged` | theory router invents a second capability schema | `crates/meld-execution/src/capability/contracts.rs` |
| Capability catalog | `meld-execution` | deterministic exact lookup | accept assembled domain publisher snapshot | `extend existing` | mutable plugin registry weakens deterministic compile | `crates/meld-execution/src/capability/catalog.rs` |
| Capability invocation | `meld-execution` | registry binds contract and invoker | preserve exact domain implementation binding | `reuse unchanged` | package bypasses execution authority | `crates/meld-execution/src/capability/invocation.rs` |
| Goal and task lineage | `meld-execution` | exact authorization and theory refs travel through execution | retain source belief, assignment, package, and triggering event refs | `extend existing` | apparent chain cannot be reconstructed | `crates/meld-execution/src/goals`, `task_network` |
| Shared semantic language | `meld-lang` | generic terms, propositions, Goals, operators, and effects | carry domain-published statements only | `reuse unchanged` | CVE and developer vocabulary becomes core enum variants | `crates/meld-lang/src` |
| Evidence admission | `meld-world-model` | mapping revisions interpret canonical outcomes | admit security, commit, scan, and docs evidence without substitution | `extend existing` | one event satisfies unrelated belief families | `crates/meld-world-model/src/belief/outcome` |
| Belief revision | `meld-world-model` | beliefs retain evidence and exact theory lineage | support several belief families over shared causal events | `reuse unchanged` | global steward state replaces domain beliefs | `crates/meld-world-model/src/belief` |
| Agent subscriptions | `meld-world-model` | one Agent subscribes to exact belief keys | allow independent package assignments to subscribe to shared facts in scope | `extend existing` | direct steward addressing replaces belief causality | `crates/meld-world-model/src/agent/subscription.rs` |
| Maintained-condition curation | `meld-world-model` | belief delivery can cause transient Goals | each steward evaluates its own condition and authority | `reuse unchanged` | producer implicitly authorizes consumer action | `crates/meld-world-model/src/agent/curation.rs` |
| Strategy construction | `meld-world-model` | constructs candidates from one authorized Goal and theory | remain local to the reacting steward | `reuse unchanged` | a global cross-steward planner emerges | `crates/meld-world-model/src/strategy` |
| Theory route catalog | prospective `theory` | no current contract | index route id, owner, schema versions, requirements, and handler identity | `new local behavior` | catalog becomes semantic owner | current fixed theory path |
| Package attachment | prospective `theory` | root source loader knows every body kind | route opaque typed components to domain handlers | `new local behavior` | opaque payloads evade owner validation | `src/init/world/source.rs` |
| Installation closure | prospective `theory` | complete receipt pins fixed owner revisions | verify every requested route installed and record exact owner receipts | `new local behavior` | router duplicates owner stores | `src/runtime/theory.rs` |
| CVE program adapter | prospective `dependency-security` | absent | bind OSV, Grype, Dependency-Track, Snyk, or another runtime behind stable domain contracts | `new local behavior` | external program becomes canonical Meld state | external research |
| Security assessment | prospective `dependency-security` | absent | own inventory meaning, advisory coverage, applicability, and bounded verdicts | `new local behavior` | silence becomes clean state | CVE ground map |
| Security capabilities | prospective `dependency-security` | absent | publish observation, feasibility, remediation-request, and verification contracts | `new local behavior` | one opaque bot capability collapses evidence stages | external research |
| Security events | prospective `dependency-security` | absent | publish typed assessment and remediation facts with source revisions | `new local behavior` | events become commands to another steward | CVE thought experiment |

## Ownership And Boundary Synthesis

### Capability Ownership

The user claim is validated with one current limitation.

The durable pattern is already explicit: a domain owns its capability contract and invoker, while `capability` owns the shared vocabulary and catalog. Context, provider, workspace, traversal, and docs already express capabilities under their domain modules.

The incomplete part is product assembly. The elevated stewardship path publishes only docs contracts and activates docs invokers directly. The missing behavior is domain publisher aggregation, not a new capability ownership model.

### Theory Routing

The public API router analogy is directionally correct when constrained to control-plane behavior.

The prospective theory concern may truthfully own:

- route identity and owner identity
- accepted component schema versions
- deterministic route lookup
- package import and requirement closure
- routing diagnostics
- exact owner receipt collection
- complete installation identity

It must not own:

- advisory meaning
- docs claim meaning
- belief comparison
- Agent curation
- Strategy construction
- capability implementation
- live steward state

The closest architectural analogue is not a web controller that adapts every request. It is the capability catalog plus domain-owned invoker pattern applied to theory installation.

### CVE As A Self-Contained Runtime

A PDS distribution can carry a complete external CVE program. Complete means it can inventory dependencies, acquire advisories, assess findings, and perhaps propose or apply remediation. It does not mean it becomes the owner of Meld beliefs, Goals, authority, or canonical history.

The domain should be named `dependency-security`, not `CVE`. CVE is one public vulnerability identity and record system. The domain concern is broader: dependency inventory, advisory correlation, policy, remediation, verification, and evidence across sources. Naming the domain after CVE would bind domain identity to one upstream taxonomy and understate its behavior.

The dependency-security domain decides what external program output means. Its adapter promotes selected results into typed canonical observations. Meld then supplies durable event history, evidence lineage, belief revision, Agent judgment, execution authority, and cross-domain provenance.

External program persistence is allowed as domain source truth. Meld should record the exact source revision and admitted result, not copy every internal scanner database row.

### Independent Steward Cooperation

The proposed chain should be understood as shared facts causing independent evaluations:

```text
dependency-security observation
→ admitted violation evidence
→ security belief revision
→ CVE steward records actionable violation

the same actionable violation fact
→ developer steward belief revision
→ developer maintained condition breaches
→ developer Goal
→ code and dependency change
→ commit evidence

changed dependency graph
→ new security observation
→ subsequent scan evidence
→ security belief becomes resolved

changed workspace and public behavior facts
→ docs steward belief revision
→ docs maintained condition independently evaluates
```

There is no direct call from the CVE Agent to the developer Agent. There is no manually authored workflow connecting the three packages. Each Agent reacts because an admitted fact is relevant to its own installed belief family and maintained condition.

The producer does not grant the consumer authority. The developer steward must independently hold authority to modify code and create a pull request.

### Evidence Refinement

The proposed belief narrative needs one evidence correction.

Commit `abc123` is evidence that a dependency change was applied. It is not by itself evidence that the vulnerability was resolved. The subsequent scan against an identified advisory-source revision is the resolution evidence. A truthful sequence is:

```text
violation assessed
→ remediation required
→ dependency change committed at abc123
→ dependency graph re-observed
→ scan performed against advisory revision R
→ violation absent within declared coverage
→ resolution assessed
```

This preserves evidence non-substitution and makes the full chain reconstructable.

## Three Scope Views

### Runtime Path Domains

```text
config
theory routing
domain theory owners
capability catalog
events
meld-world-model
meld-lang
meld-execution
runtime
domain adapters
```

### Domains With Likely Changed Behavior

```text
capability
config
docs
init
runtime
meld-execution
meld-world-model
prospective theory
prospective dependency-security
```

Context, provider, traversal, workspace, root Agent, root execution, root world-state, root task, and root library surfaces are traversed or adapted. They are not automatically major implementation scopes.

### Likely Code Regions

This is evidence of probable touch points, not an implementation plan.

```text
src/capability.rs
src/config/stewardship
src/docs/capability.rs
src/init/world
src/runtime/theory.rs
src/runtime/assembly.rs
crates/meld-execution/src/capability
crates/meld-world-model/src/agent
crates/meld-world-model/src/belief
new domain-owned theory routing location if approved
new dependency-security domain location if approved
```

## Explicit Non-Integration Decisions

- `workflow` does not coordinate steward chaining.
- `control` does not own a global multi-steward plan.
- `capability` does not own domain implementations.
- prospective `theory` does not parse or lower foreign domain meaning.
- `meld-lang` does not gain CVE, developer, or docs enum variants.
- `provider` is not mandatory for packages that use deterministic local programs.
- `context` and `prompt_context` are not required for CVE observation.
- external CVE runtime storage is not automatically copied into Meld storage.
- a commit does not prove security resolution.
- a producer steward does not authorize a consumer steward.

## Smallest Missing Connective Behavior

The smallest missing connective behavior is a pair of symmetric publication boundaries:

1. Domain capability contributors publish exact contracts and factories into product catalog assembly.
2. Domain theory route handlers accept typed package components, validate and install owner bodies, and return exact owner receipts into package installation closure.

The existing canonical event, evidence, belief, Agent, Goal, Strategy, and execution path can carry the cooperation model once several package assignments can be activated over the same runtime.

## Unresolved Ownership Questions

1. Is theory routing substantial enough to be an independent `theory` domain, or should root composition own the small route catalog while domains own every handler.
2. Does one route address one owner theory kind, one domain package facet, or one complete domain contribution.
3. Which package manifest fields are genuinely common across docs and dependency-security.
4. How does a complete external CVE runtime expose source revision and result identity without making its internal database part of Meld replay.
5. What typed domain fact represents an actionable security violation for downstream stewards without becoming an imperative command.
6. How are several package assignments composed into one runtime while conflict policy remains deferred.
7. Which belief mappings may consume the same event, and how is evidence non-substitution enforced across those mappings.
8. Does the current exact belief-key subscription model need assignment-scope discovery before independent stewards can react to shared domain facts.
9. Is capability type identity sufficient as the authority action class for external scanner and developer-agent capabilities.
10. Which isolation requirements are common enough to encode above domain adapters without becoming a universal deployment schema.
11. Which late passive observations remain admissible after activation retirement.
12. Where aggregate activation receipts, catalog generations, and current runtime-generation heads are owned.

## Assessment Verdict

Capability ownership is a stable core Meld pattern and should be reused directly. The current defect is centralized assembly of a docs-only inventory, not misplaced capability contracts.

A theory-routing concern is supported by the same pattern, with a strict boundary: shared routing and installation closure, domain-owned handlers and semantics. Whether that concern warrants a full independent domain remains open.

The CVE package can be treated as a complete external runtime without becoming a second Meld runtime. Dependency-security owns the external integration and verdicts. Meld owns durable cognition, authority, provenance, and execution coordination.

Independent stewards can cooperate through event-to-belief causality. The architecture should publish facts, not steward commands. Each downstream Agent makes its own decision under its own theory and authority.
