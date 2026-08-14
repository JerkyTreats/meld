# CVE Freshness Step 5 Discovery Ground Map

Date: 2026-08-13
Status: discovery signal, not implementation authority
Scope: ground the second-expression proof against current design, current code, and the CVE tracker specification before requirements or sequencing are frozen

## Purpose

Step 5 is the first attempt to make the elevated stewardship image carry a materially different expression. It therefore tests more than whether CVE-shaped files can be loaded. It tests whether the architecture can preserve exact theory, standing responsibility, observation choice, adaptive strategy, authority, execution safety, and evidence-backed settlement without retaining hidden docs assumptions.

The central finding is that the current runtime is vocabulary-neutral but not yet shape-neutral. Root assembly no longer branches on an expression name, yet the selected package, complete receipt, source loader, physical binding, and capability activation path still require the fixed shape of the docs proof. CVE freshness also exposes two semantic limits that docs did not: one standing assignment may need to evaluate several derived subject instances, and exclusive effects are described by capability contracts but are not arbitrated across lowered tasks or concurrent Goals.

This artifact maps those contradictions and states the strongest design direction supported by current evidence. It deliberately stops before acceptance criteria, delivery slices, or an implementation plan.

## Authority And Evidence Basis

The evidence hierarchy for this pass is:

1. Current code and tests on `implementation/theory-durability-symmetry`
2. The active [Theory Elevation Program](theory_elevation_program.md)
3. The [CVE Freshness Use Case](../../use_cases/cve_freshness.md)
4. Evergreen Strategy and PDS architecture documents
5. Proposed package and facet documents as option evidence only

The CVE Strategy page is explanatory. The use-case falsification list and the active program define the required pressure test. Proposed PDS facet and package shapes do not override current owner contracts.

## Concern Definition

The concern is whether one CVE freshness expression can be installed and activated through the same root composition as docs freshness while preserving domain ownership and proving the following distinct semantics:

- settlement through acquisition of external advisory evidence
- a bounded negative claim that never turns missing assessment into clean state
- resolver-backed feasibility as typed external judgment
- coupled remediation scope over a shared manifest and lockfile
- durable reconsideration when advisory or resolver evidence changes
- non-substitution among advisory, resolver, and verification evidence

The concern is not whether a dependency bot can be wrapped as one opaque capability. Such a wrapper would execute work but would not prove the adaptive judgment claimed by the use case.

## In Scope

- the complete installed stewardship image and owner revision boundary
- declaration scope, concrete subjects, and Agent subscription shape
- belief, observation, maintained-condition, and curation semantics
- planner projection and Strategy feasibility inputs
- candidate scope and shared-artifact execution effects
- product capability publication and activation
- durable event-driven wake behavior
- authority and evidence lineage
- a maturity-appropriate deterministic CVE expression proof

## Out Of Scope

- a production internet advisory service
- credential rotation or connector marketplace design
- a periodic scheduler or general timer service
- pull request creation and hosted forge integration
- multiple package ecosystems in the first proof
- reachability analysis beyond a typed domain verdict
- risk acceptance workflow and organizational approval UI
- a universal PDS package schema
- a full facet compile, prepare, activate, migrate, and inspect protocol
- settled Strategy replay, which remains Step 6

## Product Maturity Boundary

Step 5 should prove semantic generality with deterministic local sources and adapters. A fixture advisory source may publish exact source revisions. A fixture resolver may issue typed satisfiability verdicts. The existing event ledger and consumer cursors can then prove durable reconsideration after a later revision.

This boundary is not a claim that the product can notice an external database change without a connector observation. The runtime can durably react after a canonical event exists. Polling cadence, credentials, network failure policy, and elapsed-time currency are separate operational concerns. If elapsed-time expiry is included in Step 5, it needs explicit semantic time input and must not be smuggled in through wall-clock reads.

## CVE Semantic Ground

### Domain objects and relations

The CVE expression needs stable identities for at least:

- project and dependency declaration
- resolved component version
- advisory and advisory-source revision
- resolver verdict
- remediation scope
- manifest and lockfile
- verification result

The dependency-security owner needs relations such as declares, resolves to, constrains, affects, fixed in, and belongs to remediation scope. These are domain vocabulary carried through generic object and relation identities. They do not belong as Rust variants in `meld-lang` or root runtime.

### Bounded negative meaning

The desired state is not global absence of vulnerabilities. It is a positive, evidence-backed assessment that no admitted advisory from a declared source set and source revision affects the resolved scope above the configured severity boundary.

The negative claim is therefore bounded by:

- subject or remediation scope
- advisory source set
- admitted source revisions
- severity and applicability policy
- dependency and resolution observations
- verification posture

`unassessed`, `clean`, `vulnerable`, and `conflicted` must remain distinguishable. Silence is never clean evidence.

The strongest ownership signal is that dependency-security should establish this bounded verdict. Generic proposition evaluation should consume the typed verdict or confidence projection. Generic planning should not implement a universal query over CVE records.

### Feasibility meaning

A resolver verdict is authoritative for whether one concrete dependency change set can resolve. It is not advisory evidence, proof of safety, or proof that the resulting lockfile is clean.

A candidate is feasible only when its exact remediation scope cites a current resolver verdict. A later manifest, lockfile, advisory, or resolver revision can invalidate that ground.

### Coupling meaning

Dependency coupling is domain truth. The dependency-security owner determines which vulnerable declarations must move together and publishes a ground remediation scope. Strategy chooses and verifies a course of action for that ground scope. It should not learn package-manager constraint semantics.

This allocation satisfies the use-case requirement that aggregation may happen before the Goal or inside candidate construction. It chooses pre-Goal domain aggregation because the current generic Strategy search has no domain graph enumeration and should not gain one for CVE vocabulary.

### Settlement meaning

Three outcomes remain distinct:

- advisory acquisition may settle what current sources report
- resolver evaluation may settle feasibility for one proposed scope
- build and test verification may settle whether the realized change passed verification

None substitutes for another. A final clean assessment may depend on all three, but each retains its own schema, source identity, and revision lineage.

## Current Causal Path

The current elevated path is:

```text
one declaration
→ one concrete subject and one provider
→ one fixed theory selection shape
→ one complete receipt with a docs claim policy
→ one Agent and one belief subscription
→ one threshold-shaped curation path
→ one ground Goal
→ one Strategy problem over one projected belief view
→ one authorized Composition
→ one task per operator
→ capability dispatch
→ outcome mapping
→ belief revision
→ Agent satisfaction
```

The generic middle is real. Strategy candidate types, Agent authorization, exact execution realization, capability contracts, outcome mappings, belief ingestion, and authority policy carry no docs vocabulary. The fixed ends still constrain which expression can reach that middle.

## Design To Code Trace

| Concern | Current ground | Design consequence |
| --- | --- | --- |
| Declaration | `StewardshipDeclaration` requires one expression, concrete subject, workspace root, provider, and fixed `TheorySelection` | The declaration is a docs-era activation record, not yet a general scope assignment |
| Theory selection | `TheorySelection` always requires `claim_policy_id` | Docs policy has become a universal slot even though it is domain-owned |
| Physical binding | `PhysicalBinding` always carries one workspace and one provider | Provider-free deterministic expressions need a dummy binding |
| Source loading | `init/world/source.rs` requires exactly one fixed body of every known kind and parses `DocsClaimPolicy` | A new owner cannot contribute exact theory without changing root loader code |
| Complete install | `WorldInitTheoryBundle` and `CompleteTheoryInstall` carry docs policy body and registry | Installation is exact but closed over the docs image shape |
| Receipt | `TheoryInstallationReceipt` and `ResolvedStewardshipTheory` include a mandatory docs claim-policy revision | Historical replay is strong but the root receipt knows a foreign domain type |
| Product inventory | `published_product_contracts` publishes only docs contracts | Product capability inventory is not yet compositional |
| Activation | `activate_exact_capabilities` directly calls docs registration and passes docs policy | Exact contract selection is generic, invoker construction is not |
| Maintained condition | `AgentMaintainedCondition` builds one `Holds` proposition for one dimension and any ground `Condition` | The owner contract already supports more than numeric thresholds |
| Curation rule | `AgentCurationRuleConfig` still requires a probability threshold and curation is named `curate_threshold_rule` | Exact maintained conditions bypass part of the threshold meaning, leaving a compatibility field in the active schema |
| Unassessed belief | Belief exposes `NeedsObservation` and `ObservationOpportunity`, while Agent evaluates projected confidence and emits no Goal only on an indeterminate proposition | Missing required evidence can still look like low confidence rather than a distinct observation choice |
| Planner projection | Projection emits confidence, stale, observation-needed, and accessible propositions for one belief view | Resolver fit, remediation-scope relations, and multiple belief dimensions are absent |
| Strategy search | Search chooses the first matching settlement rule, rejects indeterminate preconditions, closes artifacts by type, and ranks complete candidates | The evergreen observation-branch design is not implemented in the minimal engine |
| Candidate graph | `Composition` can carry conditional edges | Lowering and task readiness explicitly defer conditional execution |
| Capability contracts | Contracts already support owner identity, typed bindings, many inputs, effects, and exclusivity | Existing contracts should be extended before new CVE action machinery is invented |
| Effect ordering | `TaskCompiler` orders exclusive effects sharing scope and target inside one task definition | Composition lowering emits one task per operator, so this ordering does not cross tasks or Goals |
| Scope preservation | Lowering collapses `DomainObjectRef` to its object id for `scope_ref` | Cross-domain scope identity is already named as a TODO and becomes material for remediation groups |
| Freshness | Belief marks stale for newer evidence, superseded anchors, config changes, and evidence-policy changes | Source revision events wake the system, but elapsed-time currency is not represented |
| Evidence ingestion | Outcome mappings and evidence schemas are config-driven and revisioned | Advisory, resolver, and verification evidence can reuse the existing path with distinct schemas |
| Authority | Effective policy is exact and enforced at Agent judgment, admission, planning, and dispatch | CVE actions can reuse capability type identities as action classes unless the concrete proof falsifies that fit |

## Contradiction Ledger

### C1 Root is expression-neutral but image-shape-specific

The absence of an expression-name match arm is necessary but insufficient. A CVE package must currently invent a docs claim policy and pass through docs capability registration. That fails the Step 5 generality test even if the expression string is never inspected.

### C2 One declaration means one concrete subject

CVE remediation groups are discovered from current dependency and advisory state. They are not stable configuration entries. The current declaration and activation path bind one subject, one Agent subscription, and one belief family instance.

The architecture must distinguish a standing stewardship scope from the ground subject instances evaluated inside that scope. Without that distinction, Step 5 can prove only one hand-bound project case and cannot truthfully prove coupled and uncoupled remediation scopes.

### C3 Observation opportunity does not yet become observation action

Belief already names missing or stale evidence. Agent curation currently returns an indeterminate decision without a Goal when its target cannot be evaluated. Strategy search also rejects indeterminate preconditions instead of constructing an observation branch.

Acquisition settlement needs one causal bridge from an open observation opportunity to an Agent-authorized observation candidate. A separate CVE polling workflow would duplicate the existing belief, Agent, Strategy, and capability path.

### C4 Planner context is too narrow for feasibility

One projected belief confidence cannot carry a current resolver verdict, remediation-scope membership, advisory coverage, and artifact availability with exact lineage. Root validation also requires every requested Strategy dimension to equal the single selected belief-family dimension.

The missing capability is a richer authoritative planner snapshot, not a CVE branch in Strategy.

### C5 Effect exclusivity is descriptive beyond one compiled task

Capabilities can declare exclusive writes to a manifest. The current compiler can order conflicts only when multiple capability instances share one task definition. Authorized Composition lowering produces separate tasks, and separate Goals have no shared effect arbiter.

The shared-manifest requirement therefore exposes a real execution gap. Adding ordering inside the CVE capability would hide a generic safety property in the wrong owner.

### C6 Generic negation has unsafe indeterminate behavior

`meld-lang` maps an indeterminate child of `Not` to unsatisfied. This is not open-world negation. `Condition::Absent` also treats a missing `Holds` dimension as satisfied.

The CVE expression should not depend on either behavior to establish clean state. The dependency-security owner should publish a bounded assessment verdict only after coverage is established. If Step 5 uses generic negation directly, three-valued negation must preserve indeterminate and receive focused compatibility review.

### C7 Event-driven wake is present, source acquisition scheduling is not

New evidence advances durable ingestion and Agent subscription cursors. This supports replay-safe wake after a new source revision event. It does not create the event or know that an external database changed.

The first proof can establish durable reaction to an injected source revision. It cannot claim autonomous source currency without an explicit connector trigger contract.

## Reuse, Extension, And New Ownership

| Existing structure | Posture | Signal |
| --- | --- | --- |
| `BeliefFamilyConfig` | extend existing only where source currency needs generic meaning | Use required, non-substitutable advisory schemas and preserve observation status |
| `OutcomeMappingSetConfig` | reuse unchanged | Map advisory, resolver, and verification events through distinct rules and schemas |
| `ObservationOpportunity` | reuse unchanged | It already names the evidence gap and owning belief key |
| `AgentMaintainedCondition` | reuse core, remove threshold leakage around it | Its ground `Condition` is already broader than the threshold curation wrapper |
| `AgentCurationRuleConfig` | extend existing | Exact conditions need an explicit breach response for unsatisfied and indeterminate states |
| Agent subscriptions | extend existing | Add assignment-scoped subject discovery or family subscription rather than a CVE watcher |
| `PlannerProjectionOutput` | extend existing | Admit domain-owned typed propositions with source and revision lineage |
| `StrategyProblem` | reuse core | Keep one immutable ground problem per Goal |
| Strategy successor semantics | extend existing | Implement the already-designed observation branch only as far as Step 5 proves it |
| `CapabilityTypeContract` | extend existing activation use | Typed bindings, effects, and owner identity are the correct action boundary |
| Capability product inventory | extend existing | Aggregate domain contributors and select by exact installed contracts |
| Complete receipt | extend existing | Keep common exact refs and add sorted owner-issued extension refs without a root enum per application |
| Execution effect contracts | extend existing | Arbitrate exclusive bound targets across lowered tasks and concurrent Goals |
| Event ledger and consumer cursors | reuse unchanged | New canonical source revisions can wake current durable consumers |
| Authority policy | reuse unchanged unless concrete action classes fail | Capability availability remains distinct from exact authority |
| Docs claim policy | retain under docs owner | Move it out of the mandatory common image shape, do not generalize it into a universal claim policy |
| Dependency-security semantics | new local behavior | Own advisory coverage, applicability, dependency coupling, remediation scopes, and resolver verdict interpretation |

## Strong Design Signal

### 1 Core stewardship image plus exact owner extensions

The common image should retain only semantics demonstrated by both expressions:

- belief family
- evidence mapping
- curation rule
- maintained condition
- Strategy theory
- executable capability contracts
- authority policy

Docs claim policy is an exact docs-owned extension used by docs capability activation. CVE advisory-source and resolver policies are dependency-security extensions. The complete receipt should pin these owner revisions without learning their application vocabulary.

The smallest credible mechanism is a sorted set of owner-issued extension references plus product contributors that install, resolve, and activate their own bodies. Root coordinates exact closure and rejects unresolved requirements. It does not parse foreign payloads or branch on expression names.

This is deliberately smaller than the proposed full facet protocol. Step 5 does not yet justify compile, prepare, compensation, migration, or inspection lifecycle machinery.

### 2 Standing assignment scope plus derived ground subjects

A stewardship declaration should name a bounded assignment scope. It should not require every future concern subject to be known at configuration time.

The dependency-security owner should derive exact remediation-scope objects from dependency, advisory, and resolver facts. Agent evaluation should bind the installed maintained condition to each eligible ground subject discovered inside the assignment scope. Coupled dependencies share one remediation-scope subject. Uncoupled groups yield separate subjects and therefore separate Goals.

This preserves the existing one-ground-Goal Strategy contract and keeps dependency coupling out of Strategy.

The design must choose a general subscription mechanism, such as a family-within-scope subscription or an owner-published subject stream. Adding one declaration per derived scope is not viable because scopes change and current target resolution rejects multiple declarations for the same workspace.

### 3 Domain verdicts projected as typed planning ground

Dependency-security owns the bounded negative assessment and resolver fit verdict. It publishes ground propositions and their source revision lineage into the planner snapshot. Strategy consumes those propositions through ordinary preconditions.

The generic world model remains responsible for evidence admission, uncertainty, projection, and provenance. It does not learn CVE applicability or package-manager constraint logic.

This design treats `unassessed` as a real epistemic state. A clean proposition appears only when required advisory coverage exists. A resolver-fit proposition appears only after the declared resolver evaluated the exact remediation scope.

### 4 Observation and intervention remain two Agent choices

When required advisory or resolver evidence is missing, the standing condition should cause an observation response, not a remediation response and not silence.

The existing observation opportunity should be transformed into a ground observation Goal or observation settlement obligation through Agent and Strategy contracts. After acquisition produces evidence, normal belief revision and subscription delivery cause a new decision. No CVE-specific watcher or alternate Goal store is needed.

Conditional task execution is not required for the first proof if advisory acquisition, resolver evaluation, remediation, and verification occur as separate evidence-driven episodes. This fits current maturity better than activating deferred conditional edges merely to keep one long-running candidate alive.

### 5 Execution arbitrates declared effects by bound target

Independent remediation Goals may target the same manifest and lockfile. Execution should use existing exclusive effect declarations and fully preserved scope identity to serialize conflicting work across task boundaries.

The dependency-security domain declares the effects. Execution owns arbitration. Strategy may consider candidates independently and must not become a lock manager.

### 6 External wake is an event contract before it is a scheduler

The durable semantic wake condition is a newer admitted advisory-source or resolver revision relevant to the assignment scope. Once that revision enters the canonical event path, existing durable cursors should make reconsideration replay-safe.

The deterministic proof should separate this semantic contract from how a production connector learns of the revision. That avoids premature scheduler and network reliability machinery while still proving the architecture can wait truthfully and wake on new evidence.

## Candidate Causal Model

```text
standing dependency-security assignment
→ domain discovery of ground remediation scopes
→ belief key and maintained-condition evaluation per scope
→ missing advisory or resolver evidence
→ observation opportunity
→ Agent-authorized acquisition or resolver observation
→ canonical typed outcome
→ evidence admission and belief revision
→ current bounded assessment and fit propositions
→ Agent-authorized remediation Goal
→ Strategy candidate for one exact remediation scope
→ Execution arbitration over manifest and lockfile effects
→ resolver, build, test, and advisory verification evidence
→ reconciled assessment
→ Agent satisfaction

later advisory-source revision
→ canonical event
→ durable ingestion and subscription cursors
→ reevaluation of affected scopes
```

## Complete Top Level Domain Sweep

Evidence date: 2026-08-13

Snapshot command:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

Independent workspace crates were added to the same pass because the required behavior is owned there.

| Domain | Needed integration | Current integration | Completeness | Evidence | Non integration rationale | Follow up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | adapter | Root re-exports world-model Agent contracts | partial | `src/agent.rs` |  | Preserve facade if new generic Agent contracts are exported |
| `api` | none | No Step 5 semantic ownership | not needed | `src/api.rs` | No API surface is needed for the deterministic proof | none |
| `branches` | none | Branch identity already participates in belief and Goal meaning | not needed | `src/branches.rs` | CVE does not change branch semantics | none |
| `capability` | own | Product inventory currently publishes docs only | partial | `src/capability.rs` |  | Compose exact domain contributors without expression dispatch |
| `cli` | none | Existing init and run adapters can address the workspace | not needed | `src/cli.rs` | A new command would not prove semantic generality | none |
| `compat` | none | Compatibility remains docs selection lowering | not needed | `src/compat.rs` | No new legacy form exists | none |
| `concurrency` | none | Filesystem helper concurrency is not task effect arbitration | not needed | `src/concurrency.rs` | The required owner is Execution | none |
| `config` | own | One concrete subject, provider, and fixed theory slots | partial | `src/config/stewardship/selection.rs`, `binding.rs` |  | Separate assignment scope, common refs, owner extensions, and physical requirements |
| `context` | none | Context hydration is not required for the deterministic proof | not needed | `src/context.rs` | Resolver and advisory inputs are typed domain artifacts, not prompt context | none |
| `control` | none | No control-plan semantics are needed | not needed | `src/control.rs` | Strategy and Execution already own this path | none |
| `docs` | publish | Owns claim policy and docs capability contributor now called directly by root | partial | `src/docs/capability.rs`, `claim_validation.rs` |  | Preserve behavior behind the same contributor boundary used by dependency-security |
| `error` | none | Existing configuration and domain errors are sufficient at discovery stage | not needed | `src/error.rs` | Stable new errors follow contract design later | none |
| `events` | own | Canonical append, replay, and durable consumer progress exist | complete | `src/events.rs`, `src/events/binding.rs` |  | Reuse unchanged for source revision and outcome events |
| `execution` | adapter | Root binds execution runtime context and ports | partial | `src/execution.rs`, `src/runtime` |  | Route new generic execution contracts without domain semantics |
| `harness` | none | Product harness is not required to own CVE semantics | not needed | `src/harness.rs` | Focused deterministic tests can use existing fixture patterns | none |
| `heads` | none | Legacy head compatibility is unrelated | not needed | `src/heads.rs` | No head migration is involved | none |
| `ignore` | none | Ignore policy does not define manifest semantics | not needed | `src/ignore.rs` | Dependency-security selects its own artifacts | none |
| `init` | adapter | Complete image installation is fixed over docs-shaped bodies | partial | `src/init/world/source.rs`, `pipeline.rs`, `theory.rs` |  | Coordinate common installs and owner contributions |
| `lib` | adapter | Root crate exports product contracts | partial | `src/lib.rs` |  | Export only settled generic surfaces |
| `logging` | none | Logs do not own correctness | not needed | `src/logging.rs` | No new logging contract is needed | none |
| `merkle_traversal` | none | Filesystem traversal is not dependency resolution | not needed | `src/merkle_traversal.rs` | A CVE adapter should not repurpose docs traversal | none |
| `metadata` | none | Existing metadata schema is unrelated | not needed | `src/metadata.rs` | CVE evidence retains its own typed schema | none |
| `prompt_context` | none | No provider prompt is required for deterministic CVE semantics | not needed | `src/prompt_context.rs` | Provider-free proof is preferred | none |
| `provider` | consume | Docs needs a provider while the CVE proof may not | partial | `src/provider.rs`, stewardship binding |  | Make provider a capability requirement rather than a universal declaration field |
| `runtime` | own | Root resolves exact theory but activates docs capabilities directly | partial | `src/runtime/assembly.rs`, `theory.rs` |  | Aggregate owner contributors and activate exact selected contracts |
| `serve` | none | No HTTP surface is needed | not needed | `src/serve.rs` | Service exposure would be adapter pageantry for this proof | none |
| `session` | none | Session lifecycle is unchanged | not needed | `src/session.rs` | Existing execution context can be reused | none |
| `store` | none | Domain registries already use existing storage primitives | not needed | `src/store.rs` | No new generic store is justified | none |
| `task` | consume | Root task adapter consumes execution contracts | partial | `src/task.rs` |  | Preserve adapter while Execution adds cross-task effect arbitration |
| `telemetry` | none | Telemetry is not needed to establish semantics | not needed | `src/telemetry.rs` | Focused records and assertions are sufficient | none |
| `tree` | none | Tree semantics are unrelated | not needed | `src/tree.rs` | Dependency graphs belong to dependency-security and world-model facts | none |
| `types` | none | No universal CVE types should enter root shared types | not needed | `src/types.rs` | Use domain-owned types and generic identities | none |
| `views` | none | A user-facing read model is not required for the proof | not needed | `src/views.rs` | Inspection can follow after semantic closure | none |
| `workflow` | none | A CVE workflow would duplicate Agent, Strategy, and task behavior | not needed | `src/workflow.rs` | Keep workflow compatibility out of the new path | none |
| `workspace` | adapter | Owns physical workspace reads and publication concerns | partial | `src/workspace.rs` |  | Provide thin artifact access while dependency-security owns manifest meaning |
| `world_state` | adapter | Root exposes graph and planner world-state facilities | partial | `src/world_state.rs` |  | Carry domain-published propositions without interpreting them |
| `meld-events` | publish | Neutral object identity and event contracts already support new domain records | complete | `crates/meld-events/src` |  | Reuse unchanged |
| `meld-execution` | own | Exact authorization and effect declarations exist, cross-task effect arbitration does not | partial | `crates/meld-execution/src/capability`, `planning`, `task_network` |  | Preserve scope identity and arbitrate exclusive targets across tasks and Goals |
| `meld-lang` | own | Generic proposition language exists, open-world negation is unsafe | partial | `crates/meld-lang/src/evaluate.rs` |  | Correct negation only if selected design relies on it; keep domain vocabulary out |
| `meld-world-model` | own | Belief, Agent, planner, and Strategy are generic but single-subject and narrow-projection | partial | `crates/meld-world-model/src/belief`, `agent`, `planner`, `strategy` |  | Extend observation choice, scoped subject binding, and typed planner ground |

## Frozen Affected Domain Set

The affected set is frozen for this discovery pass as:

- `agent`
- `capability`
- `config`
- `docs`
- `events`
- `execution`
- `init`
- `lib`
- `provider`
- `runtime`
- `task`
- `workspace`
- `world_state`
- `meld-events`
- `meld-execution`
- `meld-lang`
- `meld-world-model`
- prospective `dependency-security`

Domains in this set include both behavior owners and traversed adapters. Only the ownership summary below identifies likely semantic change owners.

## Affected Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Agent facade | `agent` | Re-exports world-model contracts | Expose generic scope and observation contracts only | adapter only | Root could acquire Agent semantics | `src/agent.rs` |
| Product capability inventory | `capability` | Returns docs contracts | Aggregate built-in owner contributors | extend existing | Inventory aggregation could become expression dispatch | `src/capability.rs` |
| Capability activation bindings | `capability` | Binding kinds and bound values already exist | Carry exact policy, config, provider, and source refs required by selected contracts | extend existing | A second parallel binding model would duplicate capability contracts | `crates/meld-execution/src/capability/contracts.rs` |
| Stewardship declaration | `config` | Concrete subject and universal provider | Bind assignment scope and typed physical requirements | extend existing | Free-form payloads would make root an untyped linker | `src/config/stewardship/selection.rs` |
| Selected package | `config` | Fixed common refs plus docs policy | Retain common refs and owner extension selections | extend existing | Root enum per owner would recreate application branching | `src/config/stewardship/binding.rs` |
| Docs claim policy | `docs` | Exact durable owner registry | Remain docs-owned and selected through docs activation requirements | reuse unchanged | Generalizing it would erase domain meaning | `src/docs/claim_validation.rs` |
| Docs capability contributor | `docs` | Exact registration called directly by root | Implement the common product contributor boundary | extend existing | Docs behavior regression during extraction | `src/docs/capability.rs` |
| Canonical events | `events` | Durable append and replay | Carry new owner-published source and outcome events | reuse unchanged | Domain payload must not leak into event core enums | `src/events` |
| Execution facade | `execution` | Product context and ports | Route generic effect and capability contracts | adapter only | Adapter must not decide remediation semantics | `src/execution.rs` |
| Common theory install | `init` | Fixed body loader and fixed complete bundle | Install common exact refs and coordinate owner contributions | extend existing | Root could become a universal domain compiler | `src/init/world` |
| Crate surface | `lib` | Exports root modules | Export settled generic boundaries | adapter only | Premature public surface freeze | `src/lib.rs` |
| Provider execution binding | `provider` | Required by physical stewardship binding | Become an optional capability activation requirement | extend existing | Removing provider globally would break docs | `src/provider.rs` |
| Complete receipt | `runtime` | Exact but mandatory docs policy ref | Pin common revisions plus owner extension receipts | extend existing | Opaque refs without owner validation could weaken replay | `src/runtime/theory.rs` |
| Runtime composition | `runtime` | Resolves exact image and calls docs activation | Call product contributors by exact contract requirements | extend existing | Contributor selection must not inspect expression names | `src/runtime/assembly.rs` |
| Task adapter | `task` | Routes product task contracts | Consume execution effect arbitration unchanged | adapter only | Root task code could duplicate scheduler logic | `src/task.rs` |
| Workspace artifact access | `workspace` | Physical workspace source and publication | Expose thin reads and writes for manifest adapters | reuse unchanged | Workspace must not interpret dependency constraints | `src/workspace` |
| World-state facade | `world_state` | Root graph and projection adapter | Carry dependency-security propositions | adapter only | Root must not validate CVE meaning | `src/world_state.rs` |
| Object and event identity | `meld-events` | Generic strings plus provenance | Reuse for advisories, scopes, and verdicts | reuse unchanged | New enum vocabulary would close the ontology | `crates/meld-events/src` |
| Capability effects | `meld-execution` | Scope, target, and exclusive flag | Preserve full scope identity and derive one conflict key | extend existing | String target collisions or aliasing can serialize wrong work | `crates/meld-execution/src/capability/contracts.rs` |
| Task-network arbitration | `meld-execution` | Readiness follows semantic edges only | Serialize exclusive effects across lowered tasks and active Goals | extend existing | Scheduling policy must not alter authorized semantic graph | `crates/meld-execution/src/task_network` |
| Composition lowering | `meld-execution` | One task per operator and lossy scope ref | Carry exact domain object scope and effect claims | extend existing | Identity loss breaks cross-domain conflict detection | `crates/meld-execution/src/planning/lowering.rs` |
| Proposition evaluation | `meld-lang` | Three-valued `Holds`, closed behavior for some absence and negation | Preserve indeterminate for any selected negative semantics | extend existing | Compatibility changes can alter existing planner results | `crates/meld-lang/src/evaluate.rs` |
| Belief family and evidence | `meld-world-model` | Generic schemas, unanchored assessment, observation opportunities | Represent advisory coverage without substitution | extend existing | Bayesian confidence alone may hide categorical assessment posture | `crates/meld-world-model/src/belief` |
| Agent curation | `meld-world-model` | One belief delivery and threshold-era rule | Choose observation or intervention from exact condition state | extend existing | A CVE-specific curation mode would fork Agent semantics | `crates/meld-world-model/src/agent` |
| Subject subscriptions | `meld-world-model` | Explicit subscription per concrete belief key | Bind an Agent assignment to discovered subjects in scope | extend existing | Unbounded discovery or duplicate Goals | `crates/meld-world-model/src/agent/subscription.rs` |
| Planner projection | `meld-world-model` | One belief view plus graph accessibility | Include domain-owned fit and scope propositions with lineage | extend existing | Projection must not become an alternate domain evaluator | `crates/meld-world-model/src/planner` |
| Strategy search | `meld-world-model` | Ground direct construction and hard rejection of indeterminate preconditions | Construct the bounded observation alternative already named by Strategy design | extend existing | Premature conditional search and branching framework | `crates/meld-world-model/src/strategy` |
| Dependency model | prospective `dependency-security` | Not present | Own declarations, resolved versions, coupling, and remediation scopes | new local behavior | Leakage into workspace or Strategy | CVE use case |
| Advisory assessment | prospective `dependency-security` | Not present | Own source coverage, applicability, severity, and bounded negative verdict | new local behavior | Treating silence as clean | CVE use case |
| Resolver interpretation | prospective `dependency-security` | Not present | Own exact fit verdict and invalidation rules | new local behavior | Resolver exit status mistaken for domain truth | CVE use case |
| CVE capabilities and mappings | prospective `dependency-security` | Not present | Publish typed contracts, invokers, events, and owner extension refs | new local behavior | One opaque bot capability would erase adaptive choice | CVE use case |

## Ownership Summary

### Semantic behavior owners

`dependency-security` owns new CVE meaning. `meld-world-model` owns generic observation choice, subject-scoped evaluation, planner projection, and Strategy construction. `meld-execution` owns effect arbitration. `config`, `init`, `capability`, and `runtime` own general installed-image and activation closure.

### Adapters

Root `agent`, `execution`, `lib`, `task`, `workspace`, and `world_state` remain thin. They may route new contracts but must not interpret advisory, resolver, coupling, or clean-state meaning.

### Reuse unchanged

Canonical events, durable consumer cursors, object identity, evidence ingestion, outcome mapping, exact authority enforcement, Goal storage, and docs claim validation remain under their current owners.

## Duplicate Machinery Guardrails

Step 5 should be rejected if it introduces any of the following:

- a CVE-specific event loop
- a CVE belief store or assessment cursor
- a CVE Goal type or planner
- a second capability binding schema unrelated to existing `BindingSpec`
- a second effect lock system hidden inside a package-manager adapter
- a universal claim policy derived from docs claim validation
- one generic facet lifecycle protocol implemented only by forwarding current owner calls
- an opaque dependency-bot capability that owns observation, decision, remediation, and verification as one step

The legitimate new machinery is the dependency-security domain itself. Its source truth and interpretation do not exist elsewhere. The legitimate generic extensions are those directly falsified by the second expression: open owner extensions in the installed image, assignment scope over derived subjects, observation choice, richer typed planner ground, and cross-task effect arbitration.

## Proof Obligations Exposed By The Specification

These are discovery obligations, not frozen acceptance criteria.

- missing advisory coverage remains unassessed and causes observation work
- admitted advisory coverage can support a bounded clean assessment
- advisory evidence cannot satisfy resolver or verification requirements
- a no-fix resolver verdict prevents remediation without converting the standing condition into satisfied state
- a later advisory or resolver revision causes durable reconsideration
- coupled dependencies share one ground remediation scope
- uncoupled scopes remain separate and conflicting manifest writes serialize in Execution
- exact capability authority is enforced at all existing gates
- docs freshness still activates through the same root contributor mechanism
- neither root nor generic crates contain CVE vocabulary branches
- restart behavior uses existing durable events, beliefs, decisions, Goals, and task state without a new CVE store

## Open Design Questions

### Scope discovery contract

Should an Agent subscribe to a belief family within an assignment scope, or should dependency-security publish an explicit stream of eligible subject bindings. The choice must bound discovery, preserve cursor durability, and avoid duplicate Goal families.

### Owner extension envelope

What is the smallest exact owner reference that lets root prove complete installation while the owner retains body validation and historical resolution. Existing revision reference shapes should be reused where possible, but a root enum for every owner is not acceptable.

### Planner projection boundary

Should dependency-security publish verdicts as belief dimensions, graph propositions, or a domain projection contribution. The answer should preserve exact source lineage and avoid making one belief family carry unrelated advisory, resolver, and verification state.

### Observation Goal shape

Should Agent emit a distinct Goal target for the open observation opportunity, or should Strategy transform an indeterminate settlement obligation into an observation candidate. Evergreen Strategy design permits the latter, while current Agent causality may make the former easier to reason about. Only one authority path should emerge.

### Effect conflict key

The conflict key needs complete scope identity and a stable target identity for manifest and lockfile. The current `scope_ref` string loses domain and kind. The corrected key should be generic and deterministic without embedding filesystem paths as universal semantics.

### Source currency

Does the Step 5 proof require elapsed-time expiry, or only wake on an explicit newer source revision. If elapsed time is required, it needs an event or other explicit semantic time source and a declared currency policy.

## Non Commitments

This discovery pass does not choose:

- a package ecosystem
- a manifest parser library
- an advisory database
- a resolver process
- a concrete source file schema
- a connector polling model
- a full facet protocol
- a generic scheduler
- a final Agent scope subscription API
- implementation sequencing

## Discovery Verdict

Step 5 is viable, but it is not ready to begin as a simple new theory directory. The current generic cognitive path is reusable. The installed image boundary, subject binding model, observation bridge, planner projection, and execution effect arbitration need explicit design before a CVE expression can be an honest generality proof.

The strongest design signal is:

```text
common exact stewardship image
+ domain-owned exact extension references
+ standing assignment scope
+ domain-derived ground remediation subjects
+ evidence-driven observation and intervention choices
+ domain-published feasibility propositions
+ execution-owned effect arbitration
```

This direction challenges the docs implementation where it is truly overfit while resisting a premature PDS framework. It keeps the new CVE semantics local, extends existing generic contracts at the points the second expression falsifies, and leaves production connector reliability outside the semantic proof.

## Read With

- [External Domain Theory Attachment Proposition Brief](external_domain_theory_attachment_proposition_brief.md)
- [Theory Elevation Program](theory_elevation_program.md)
- [CVE Freshness Use Case](../../use_cases/cve_freshness.md)
