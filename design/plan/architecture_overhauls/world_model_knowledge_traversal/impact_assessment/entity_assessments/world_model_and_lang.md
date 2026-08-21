# Entity First Impact Assessment For World Model And Language

Date: 2026-08-20

Status: evidence for later canonical requirements

## Problem First

The current docs freshness path cannot discover that an existing README is already correct. Its installed Strategy theory can settle the Goal only through an executable artifact chain, and the current Agent can authorize only that one executable candidate. The same structural limitation appears in dependency security: installed Strategy theory contains Capability products only, while its outcome mapping does not yet map a completed dependency assessment into the configured belief families.

The architecture proposal changes the semantic product before it changes either executor. Strategy becomes construction and reconstruction of a heterogeneous causal Plan. Agent retains root Goal authority and progresses the Plan during reconciliation. Curation realizes bounded Epistemic Operations. Execution continues to receive executable products only. Traversal materializes Curation results after Events carries them.

The entity-first finding is sharp:

> The substantial code impact belongs in `meld-world-model`. Current evidence does not prove a required Rust change in `meld-lang`.

The language crate already supplies permissive Goal, Proposition, term, condition, world-state, and executable Composition vocabulary. A Strategy-owned Plan can reference those values while owning its heterogeneous node grammar locally. Extending `StepKind` to include Epistemic Operations would instead make the shared language and every Composition consumer interpret Curation-specific work. No current boundary requires that coupling.

## Concern And Scope

This assessment traces the impact of the frozen [recursive assessment charter](../assessment_charter.md) through every current top-level domain in `meld-world-model` and `meld-lang`. It begins at public semantic contracts, installed theory products, durable records, actor inputs and outputs, and owner-visible state transitions. It then synthesizes those findings into each owning domain and finally into one finding per crate.

The evidence basis is the current repository plus the complete packet named by the charter. The primary proposal is [Strategy Is Bigger Than A Task](../../strategy_plan_redesign.md). Current behavior comes from the [neutral code ground map](../../current_code_groundmap.md). The specialist evidence is the [Goal language review](../../reviews/goal_language_strategy_plan_review.md), the [Event-backed operation review](../../reviews/event_backed_epistemic_operations_review.md), and the [Traversal and Curation assessment](../../curation_assessment.md).

This report does not write requirements, select concrete type names, define migration phases, or sequence implementation. Proposed products are clearly separated from implemented entities.

## Evidence Classification

`implemented` means the entity or transition exists in current code. `installed theory` means the product is current package data consumed through an implemented registry or loader. `proposed` means the entity comes from the frozen assessment concern and is absent from runtime code. `inferred impact` means current contracts establish the affected boundary without selecting one future representation.

## Regenerated Crate And Domain Snapshot

The current `meld-world-model` crate surface declares these top-level domains:

```text
agent
belief
planner
strategy
waiting
world_state
```

There is no current top-level `curation` domain. The snapshot is grounded in the [world-model crate surface](../../../../../../crates/meld-world-model/src/lib.rs) and its [current source tree](../../../../../../crates/meld-world-model/src).

The current `meld-lang` crate surface declares these top-level domains:

```text
authority
composition
condition
cost
effect
evaluate
goal
method
operator
proposition
substitute
term
unify
validate
world_state
```

The snapshot is grounded in the [language crate surface](../../../../../../crates/meld-lang/src/lib.rs) and its [current source tree](../../../../../../crates/meld-lang/src).

## Pass One World Model Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Current evidence | Non-integration rationale | Entity follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `own` | Owns root Agent identity, maintained intent, Goal drafting, Strategy authorization, Goal submission, satisfaction review, and belief-revision delivery cursors | `partial` | [Agent contracts](../../../../../../crates/meld-world-model/src/agent/contracts.rs), [Agent runtime](../../../../../../crates/meld-world-model/src/agent/runtime.rs), [Agent selection](../../../../../../crates/meld-world-model/src/agent/selection.rs) | not applicable | Trace Plan authority, progression, reconciliation, and separate Curation submission against current decision records and actors |
| `belief` | `consume` | Owns configurable Event-to-evidence mapping, evidence ingestion, revisions, views, and exact-key signals consumed by Agent | `partial` | [outcome mapping contract](../../../../../../crates/meld-world-model/src/belief/outcome/mapping.rs), [evidence ingestion actor](../../../../../../crates/meld-world-model/src/belief/evidence_ingestion.rs), [belief contracts](../../../../../../crates/meld-world-model/src/belief/contracts.rs) | Belief need not interpret every Curation result and does not execute Epistemic Operations | Trace reusable ingestion entities and installed mapping gaps |
| `planner` | `publish` | Publishes one flattened `WorldState` plus source and hydration handles from Belief and current anchors | `partial` | [Planner contracts](../../../../../../crates/meld-world-model/src/planner/contracts.rs), [Planner query](../../../../../../crates/meld-world-model/src/planner/query.rs) | Planner remains read-only and does not own Plan choice or Curation authorship | Trace the loss of relation topology and frozen-cut identity |
| `strategy` | `own` | Owns pure bounded construction, verification, candidate ranking, exact theory revisions, and one executable `StrategyCandidate` | `partial` | [Strategy contracts](../../../../../../crates/meld-world-model/src/strategy/contracts.rs), [Strategy search](../../../../../../crates/meld-world-model/src/strategy/search.rs), [Strategy verification](../../../../../../crates/meld-world-model/src/strategy/verification.rs) | Strategy does not execute either Tasks or Epistemic Operations | Trace the executable-only product and missing persistent Plan semantics |
| `waiting` | `none` | Publishes diagnostic declarations for current bounded actors | `not needed` | [waiting declarations](../../../../../../crates/meld-world-model/src/waiting.rs) | Diagnostics do not own Plan, Curation, Traversal, or reconciliation semantics | none |
| `world_state` | `own` | Owns Event-backed Traversal materialization, bounded graph reads, anchors, provenance, and a legacy claim compatibility surface | `partial` | [Traversal contracts](../../../../../../crates/meld-world-model/src/world_state/graph/contracts.rs), [Traversal reducer](../../../../../../crates/meld-world-model/src/world_state/graph/reducer.rs), [Traversal query](../../../../../../crates/meld-world-model/src/world_state/graph/query.rs) | Legacy claims do not need to become the new Curation domain | Trace event admission, relation occurrences, currentness, and query catch-up |

The missing `curation` boundary is not inserted into this regenerated list as if it already existed. It is assessed separately as an absent target domain because the frozen concern assigns Epistemic Operation realization and epistemic authorship to it.

## Frozen World Model Affected Set

The frozen current-domain set is `agent`, `belief`, `planner`, `strategy`, and `world_state`.

The missing target boundary is `curation`. The crate surface is an adapter impact. `waiting` is explicitly excluded.

## World Model Entity Assessment

### Agent Entities

| Entity or transition | Current owner | Current behavior | Required relationship to assessed architecture | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `AgentRecord` | Agent | Persists Agent identity, perspective, subject, branch, directive, installed Goal curation rule, maintained condition, and lifecycle | Remain the root perspective and directive authority from which Plan and Curation work derive | `extend existing` | Adding Plan or Curation state directly to the identity record could collapse identity, runtime progression, and semantic products | [record contract](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L105) |
| `AgentMaintainedCondition` and installed binding | Agent | Define durable desired state and Goal priority independently of transient Goal lifecycle | Continue to explain why a root Goal exists while Curation may establish expected epistemic entities before Goal creation | `reuse unchanged` | Treating standing Curation output as a Goal would erase the bootstrap distinction | [maintained condition](../../../../../../crates/meld-world-model/src/agent/maintained_condition.rs) |
| `AgentCurationRuleConfig` and registry revision | Agent | Define threshold-based Goal drafting from Belief divergence | Remain Goal-drafting theory rather than becoming the grammar for Epistemic Operations | `reuse unchanged` | The existing name `curation` can falsely imply graph authorship that current code does not perform | [rule contract](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L362), [registry](../../../../../../crates/meld-world-model/src/agent/curation_registry.rs) |
| `AgentSubscriptionRecord` and `AgentDelivery` | Agent | Bind one Agent to exact Belief revisions and advance only after durable decision handling | Continue to provide one reconciliation trigger, while not pretending that arbitrary Events or graph edges wake an Agent | `extend existing` | A mixed Plan can stall if relevant Curation results do not become a subscribed Belief revision or another explicit Agent signal | [subscription record](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L211), [delivery selection](../../../../../../crates/meld-world-model/src/agent/selection.rs#L72) |
| `AgentCurationInput` and `AgentGoalSatisfactionInput` | Agent | Assemble Belief, flattened Planner projection, active Goals, installed rule, and durable input references for pure decisions | Consume current Plan state and exact epistemic milestones during reconciliation without executing either product class | `extend existing` | Reusing these Goal-specific inputs as a universal Plan input could hide different progression semantics | [decision inputs](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L953) |
| `AgentCurationDecision` and `AgentDecisionKind` | Agent | Persist one Goal command, Goal mutation, absorbed result, or indeterminate result with optional one-candidate Strategy authorization | Represent Agent judgment over a Plan revision and later product eligibility while preserving current Goal decisions as distinct outcomes | `extend existing` | One record may become overloaded if Plan authorization, product authorization, Goal lifecycle, and Curation authorship are not distinct | [decision record](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L505), [decision kinds](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L83) |
| `StrategyAuthorization` on Agent decisions | Strategy body authorized by Agent | Binds one exact executable candidate, Goal, planner snapshot, policy, and optional effective authority decision | Bind Agent judgment to the heterogeneous Plan or its eligible products without sending the whole Plan to either executor | `extend existing` | Current one-candidate identity cannot express Plan revision lineage or several independently eligible products | [authorization contract](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L162), [Agent authorization assembly](../../../../../../crates/meld-world-model/src/agent/strategy.rs) |
| `AgentGoalCommand` | Agent | Carries one proposed `Goal` and optional executable Strategy authorization to the Goal Set seam | Continue to carry executable Goal and Task-shaped authorization only | `reuse unchanged` | Expanding this command to carry Epistemic Operations would violate the Execution seam | [Goal command](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L763) |
| `AgentGoalMutationCommand` | Agent | Requests satisfaction or reopening of an Execution-owned Goal lifecycle after Belief review | Remain the execution-facing Goal lifecycle path | `reuse unchanged` | Plan reconciliation and Goal lifecycle may be conflated if every Plan revision becomes a Goal mutation | [mutation command](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L839) |
| `CurationGoalSetPort` | Agent boundary to Execution | Names the combined Goal command and mutation sink used by current Agent runtime | Remain an executable boundary and stay separate from a future Curation submission boundary | `reuse unchanged` | Its current name can be mistaken for epistemic Curation even though it targets Execution Goal storage | [port contract](../../../../../../crates/meld-world-model/src/agent/goal_port.rs) |
| `AgentSinkReceipt` and Agent store dedupe records | Agent | Persist submission identity before advancing durable subscription state and support replay recovery | Preserve durable handoff accounting for each independently routed product owner | `extend existing` | One receipt vocabulary currently assumes Execution Goal identity and cannot account for Curation operation identity | [receipt contract](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L596), [store](../../../../../../crates/meld-world-model/src/agent/store.rs) |
| `AgentGoalCurationRuntime` | Agent | Curates a Goal, invokes Strategy once, persists one decision, submits one Goal command, records a receipt, then advances delivery | Become part of a broader reconciliation path that can reconstruct and progress a Plan without making Strategy an actor | `extend existing` | Retrofitting mixed progression into this Goal-only actor may bind unrelated state transitions to one delivery cursor | [runtime](../../../../../../crates/meld-world-model/src/agent/runtime.rs#L190) |
| `AgentSatisfactionCurationRuntime` | Agent | Reviews active Goals against current Planner projection and submits satisfy or reopen mutations | Continue to own final Goal lifecycle judgment after epistemic results reconcile | `reuse unchanged` | Task or Epistemic Operation termination must not be treated as Goal satisfaction | [satisfaction runtime](../../../../../../crates/meld-world-model/src/agent/runtime.rs#L572) |
| active Plan state and progression record | no current owner | No durable Strategy Plan identity, revision lineage, product state, or progression cursor exists | Supply the state that lets Agent reconcile a root Goal with new knowledge and authorize newly eligible products | `new local behavior` | Putting this state in Strategy would turn a pure constructor into the runtime owner | [absence established by current Strategy contracts](../../../../../../crates/meld-world-model/src/strategy/contracts.rs), [current Agent store surface](../../../../../../crates/meld-world-model/src/agent/store.rs) |

Agent synthesis: current Agent already owns the authority root, exact perspective, durable decision, replay-safe sink handoff, and final satisfaction judgment. The missing entity family is not another Goal command. It is durable Plan authority and progression that can route executable products to the existing Goal Set seam and epistemic products to a separate Curation seam. The current runtime shape supports reconciliation triggers only through subscribed Belief revisions, so graph visibility alone is not an Agent trigger.

### Belief Entities

| Entity or transition | Current owner | Current behavior | Required relationship to assessed architecture | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `OutcomeEvidenceMapping` and `OutcomeMappingDisposition` | Belief | Interpret intact Event records through installed mapping theory as applicable evidence, non-applicable records, or durable rejections | Admit only configured Curation result Events whose meaning contributes to a Belief family | `reuse unchanged` | Treating every Curation Event as evidence would bypass family-owned mapping and evidence policy | [mapping contract](../../../../../../crates/meld-world-model/src/belief/outcome/mapping.rs) |
| `ConfiguredOutcomeMappingSet` | Belief | Supplies data-driven event matching, subject binding, and evidence-field extraction | Carry new Curation event mappings as installed theory rather than Rust branching | `reuse unchanged` | Generic JSON mapping can transport fields but does not define Curation semantics | [configured mapping](../../../../../../crates/meld-world-model/src/belief/outcome/interpretation.rs) |
| `PromotedEvidenceRecord` | Belief | Carries generic subject, object, relation, cursor, content, and mapped field material into normalization | Accept Curation-owned result material when a mapping explicitly promotes it | `reuse unchanged` | Graph reachability cannot substitute for evidence admission | [promoted evidence](../../../../../../crates/meld-world-model/src/belief/contracts.rs#L260) |
| `EvidenceIngestionActor` and `EVIDENCE_CONSUMER_ID` | Belief | Replay Events from a durable cursor, map bounded records, persist evidence or rejection, then advance | Reuse the existing Event ingestion and recovery path for configured Curation results | `reuse unchanged` | A Curation result can be durable in Events while Belief still trails its independent cursor | [ingestion actor](../../../../../../crates/meld-world-model/src/belief/evidence_ingestion.rs), [consumer identity](../../../../../../crates/meld-world-model/src/belief/outcome/mapping.rs#L22) |
| `BeliefRevision` and `BeliefView` | Belief | Commit append-only evidence settlements and planner-safe current views with exact theory lineage | Continue to provide the high-signal epistemic state that Agent subscribes to and Strategy consumes through Planner | `reuse unchanged` | Curation authorship and Belief settlement are distinct even when both concern the same README entity | [belief contracts](../../../../../../crates/meld-world-model/src/belief/contracts.rs#L345) |
| `ObservationOpportunity` | Belief | Names useful missing evidence without issuing an execution command | Remain a belief-owned epistemic gap signal that Strategy may later reason about | `reuse unchanged` | It is not itself a bounded Epistemic Operation or Plan product | [observation opportunity](../../../../../../crates/meld-world-model/src/belief/contracts.rs#L428) |
| docs freshness belief and outcome mapping theory | Belief installed theory | Accepts `world_model.unobserved_scope` and one `execution.task.succeeded` docs assessment artifact | Add Curation result interpretation only if those results are intended to revise docs freshness Belief | `extend existing` | Mapping aggregate results alone would still not make individual claims traversable | [belief family](../../../../../../theory/docs_freshness/belief_family.docs_freshness.json), [outcome mapping](../../../../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json) |
| dependency security belief and outcome mapping theory | Belief installed theory | Defines coverage and posture families, while its installed mapping currently handles only unobserved scope | Interpret a future Curation assessment only through explicit family mapping | `extend existing` | Current theory does not connect completed dependency assessment outcomes to either configured family | [coverage family](../../../../../../theory/dependency_security/belief_family.security_coverage.json), [posture family](../../../../../../theory/dependency_security/belief_family.security_posture.json), [outcome mapping](../../../../../../theory/dependency_security/outcome_mapping.security.json) |

Belief synthesis: the core Belief crate already owns the correct consumer-side seam. It consumes intact Events through installed mappings and protects only Belief-owned state. No current evidence proves a required Rust change in Belief. The impact is installed theory when a Curation result should become evidence, plus the visibility barrier between Event durability, evidence ingestion, revision commit, and Agent delivery.

### Planner Entities

| Entity or transition | Current owner | Current behavior | Required relationship to assessed architecture | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `PlannerProjectionContext` | Planner | Binds subject, perspective, branch, and static projection version | Bind a frozen context suitable for Strategy reconstruction under one Agent perspective | `extend existing` | A static projection version is not a source cut or Plan-reconciliation identity | [projection context](../../../../../../crates/meld-world-model/src/planner/contracts.rs#L18) |
| `PlannerGraphScope` | Planner | Reduces Traversal state to accessibility, anchor ids, and source fact ids | Preserve the bounded relation topology and occurrence provenance needed to distinguish expected from observed state | `extend existing` | Flattening Curation products to accessible state destroys the mismatch Strategy needs | [graph scope](../../../../../../crates/meld-world-model/src/planner/contracts.rs#L100) |
| `PlannerProjectionOutput` | Planner | Publishes ground propositions, projection version, source references, hydration handles, warnings, and Belief theory lineage | Become an exact frozen Strategy knowledge cut or carry a reference to one | `extend existing` | Hydration handles without walk bounds and occurrence identity cannot replay context selection | [projection output](../../../../../../crates/meld-world-model/src/planner/contracts.rs#L108) |
| `PlannerSourceRef` and `PlannerHydrationRefs` | Planner | Preserve Belief revision, Evidence, source fact, graph anchor, and projection-rule identities | Preserve reached relation occurrences and typed owner products without flattening their semantics | `extend existing` | A universal planner DTO could become an undeclared ontology | [source and hydration refs](../../../../../../crates/meld-world-model/src/planner/contracts.rs#L124) |
| `project_world_state` | Planner | Projects Belief confidence, stale state, observation need, and graph accessibility into ground language propositions | Continue to produce Strategy-readable propositions while retaining richer graph context separately | `extend existing` | Encoding every graph edge as a generic proposition may discard currentness and authority | [projection](../../../../../../crates/meld-world-model/src/planner/projection.rs) |
| `PlannerQuery` | Planner | Reads one exact Belief key and current anchors for the same subject | Assemble relation-rich, perspective-correct, bounded context for Strategy without executing Curation | `extend existing` | Current subject-anchor lookup cannot discover unknown intermediate claim and README identities | [query facade](../../../../../../crates/meld-world-model/src/planner/query.rs) |

Planner synthesis: Planner is the current Strategy-facing context assembler, but its graph input is deliberately thin. The proposed Plan and README mismatch need relation-rich frozen context. The open ownership edge remains whether Planner owns that richer Strategy cut or consumes a neutral frozen-cut product also usable by Curation. Current evidence proves the missing information but does not select the owner.

### Strategy Entities

| Entity or transition | Current owner | Current behavior | Required relationship to assessed architecture | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `StrategyTheorySnapshot` and `StrategySettlementRule` | Strategy | Map one Goal pattern to one settlement proposition and one prospective evidence route | Express causal decomposition across desired conditions, executable products, and bounded epistemic products | `extend existing` | Keeping one settlement artifact as the only terminal shape preserves the current executable bias | [theory contracts](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L7) |
| `ProspectiveEvidenceRoute` | Strategy | Describes later evidence expected from successful Capability realization | Remain predictive metadata or be complemented by an actual Plan node when epistemic work is causally required | `extend existing` | Metadata cannot run, gate, abstain, or make a Task unnecessary | [evidence route](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L24) |
| `StrategyCapability` | Strategy | Supplies an exact executable operator and outcome contract for construction | Remain the executable construction vocabulary | `reuse unchanged` | Folding Epistemic Operation declarations into Capability would send epistemic work toward Execution | [Capability view](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L37) |
| Epistemic Operation declaration catalog | Curation semantics consumed by Strategy | No current entity exists | Supply Strategy with owner-declared constructible epistemic products without transferring their semantics to Strategy | `new local behavior` | Strategy could invent Curation meaning if no exact declared contract exists | [absence in Strategy problem](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L84) |
| `StrategyProblem` | Strategy | Carries one proposed Goal, one flat `WorldState`, one planner snapshot, one Strategy theory, Capabilities, Methods, and policy | Carry the exact Plan revision basis and both declared discharge-product catalogs | `extend existing` | Live graph reads or mutable state during construction would break deterministic reconstruction | [problem contract](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L84) |
| `Method` input | Shared language consumed by Strategy | Optionally seeds one executable Composition template | Remain an executable solution template unless a separate Strategy-owned heterogeneous template is established | `reuse unchanged` | Broadening shared Method grammar would make `meld-lang` guess Strategy and Curation intent | [Method contract](../../../../../../crates/meld-lang/src/method.rs), [method search](../../../../../../crates/meld-world-model/src/strategy/search.rs#L119) |
| `StrategyCandidate` | Strategy | Contains one executable Composition, one settlement obligation, one evidence route, Capability identities, bindings, and evaluation | Give way to or sit beneath a heterogeneous Strategy Plan product with causal dependencies and revision lineage | `extend existing` | Treating the current candidate as the whole Plan makes epistemic work descriptive rather than causal | [candidate contract](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L136) |
| Strategy Plan entity family | Strategy | No current entity exists | Represent desired conditions, closed Task-shaped products, closed Epistemic Operations, causal dependencies, exact context, and revision lineage | `new local behavior` | Modeling this as `meld_lang::Composition` would expose Curation grammar to every Composition consumer | [current candidate shape](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L136), [proposal](../../strategy_plan_redesign.md) |
| `StrategySearchRequest`, result, completion, statistics, and rejection grounds | Strategy | Report one bounded pure search over executable candidates | Report bounded Plan construction or reconstruction without claiming that the runtime progressed the Plan | `extend existing` | Search completion can be confused with Plan completion or Goal achievement | [search contracts](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L105) |
| `search` | Strategy | Unifies one settlement rule, grounds executable operators, closes artifacts, and rejects non-operator steps | Construct and reconstruct heterogeneous causal products from a frozen problem | `extend existing` | Current artifact closure is not sufficient for epistemic bounds, perspective, or terminal meaning | [search](../../../../../../crates/meld-world-model/src/strategy/search.rs) |
| `verify_candidate` | Strategy | Rechecks executable Composition structure, preconditions, artifact closure, contribution, evidence route, identity, and Capability use | Independently verify the Strategy-owned Plan and each producer-owned product reference without re-proving producer semantics | `extend existing` | Verification may become a boundary-enforcement sink if it attempts to validate Curation internals | [verification](../../../../../../crates/meld-world-model/src/strategy/verification.rs) |
| `StrategyAuthorization` | Strategy product authorized by Agent | Binds one Agent decision to one verified executable candidate and optional executable authority | Preserve exact Plan or product judgment, revision lineage, and producer-specific authority without becoming executor admission | `extend existing` | One authorization scope may be too broad for knowledge changes between product eligibility points | [authorization](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L162) |
| `StrategyTheoryPackage` and registry revision | Strategy | Install exact settlement rules, executable Capabilities, evaluation policy, search bounds, dimensions, and requested executable authority | Carry exact heterogeneous construction theory or reference a separate Curation theory revision | `extend existing` | Requiring every package to contain Capabilities currently excludes epistemic-only Strategy Plans | [package and registry](../../../../../../crates/meld-world-model/src/strategy/contracts.rs#L59), [registry validation](../../../../../../crates/meld-world-model/src/strategy/registry.rs) |
| docs freshness Strategy theory | Strategy installed theory | Requires a `docs_freshness_assessment` artifact and closes it through inspect, draft, validate, publish, and assess Capabilities | Represent the epistemic-only branch and mixed branch without requiring publication before assessment | `extend existing` | Current theory makes writing a prerequisite for knowing | [installed docs theory](../../../../../../theory/docs_freshness/strategy_theory.docs_freshness.json) |
| dependency security Strategy theory | Strategy installed theory | Declares only executable inventory, advisory, assessment, and verification Capabilities | Distinguish knowledge acquisition and epistemic settlement from external executable intervention when the use case requires it | `extend existing` | Current flat Capability set does not express causal dependency or epistemic terminal milestones | [installed security theory](../../../../../../theory/dependency_security/strategy_theory.security.json) |

Strategy synthesis: this is the largest semantic impact. Current Strategy is a pure and useful constructor, but it constructs exactly one executable candidate. The proposal preserves purity and moves durable progression to Agent. Strategy gains a heterogeneous Plan product, an epistemic product catalog, causal dependency semantics, Plan reconstruction, and independent verification at the owner boundary. No current evidence makes Strategy the Curation executor or reconciliation actor.

### Missing Curation Domain Entities

| Proposed entity family | Current owner | Current behavior | Required relationship to assessed architecture | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- | --- |
| bounded Epistemic Operation contract | no current owner | Absent | Close perspective, source cut, roots, traversal bounds, exact rule, admissible result vocabulary, authority, and terminal meaning before execution | `new local behavior` | A generic Event payload would hide the semantic contract rather than own it | [Event-backed review](../../reviews/event_backed_epistemic_operations_review.md#eventenvelope-is-carrier-not-epistemic-grammar) |
| standing Curation input | no current owner | Absent | Apply installed Agent specification before a Goal exists and author expected entities such as README F | `new local behavior` | Requiring a Goal for every expected entity creates a bootstrap cycle | [standing and planned distinction](../../strategy_plan_redesign.md#curation-has-two-entrances) |
| planned Curation input | no current owner | Absent | Accept an Agent-authorized Epistemic Operation selected by a Strategy Plan | `new local behavior` | Routing it through the Execution Goal Set would violate the frozen seam | [whole Plan publication review](../../reviews/event_backed_epistemic_operations_review.md#whole-plan-publication-to-execution) |
| durable operation identity and lifecycle | no current owner | Absent | Make replay discover accepted, active, and terminal work without rerunning settled work | `new local behavior` | Event replay alone does not provide exactly-once semantic execution | [replay assessment](../../reviews/event_backed_epistemic_operations_review.md#replay-and-idempotency) |
| typed terminal operation result | no current owner | Absent | Record completed, unchanged, abstained, bounded incomplete, rejected, or failed outcomes when later Plan work depends on termination | `new local behavior` | Silence cannot distinguish completion from work that never ran | [bounded outcomes review](../../reviews/event_backed_epistemic_operations_review.md#bounded-operations-have-outcomes-not-guaranteed-success) |
| Curation-owned epistemic products | no current owner | Absent | Author expected entities, requirement, materiality, realization, coverage, and bounded assessment connections under exact Agent perspective and rule lineage | `new local behavior` | Curation must not assert physical observation or another owner's vocabulary as its own truth | [Curation assessment](../../curation_assessment.md#the-missing-readme-question) |
| Curation Event publication | no current owner | Absent | Append terminal and shared semantic results through the existing Event spine for replay and fanout | `new local behavior` | An append receipt is not Traversal, Belief, or Agent visibility | [Event result assessment](../../reviews/event_backed_epistemic_operations_review.md#should-epistemic-operation-results-append-events) |
| Curation actor selection and cursor state | no current owner | Absent | Discover eligible standing and planned operations from durable inputs and resume bounded work | `new local behavior` | Consuming and producing the same graph can create feedback, oscillation, and unchanged-result loops | [feedback assessment](../../reviews/event_backed_epistemic_operations_review.md#feedback-loops) |
| Curation rule and result theory revisions | no current owner | Absent | Pin exact semantic rule, perspective, bounds, result vocabulary, and currentness policy | `new local behavior` | Reusing Agent threshold rules would conflate Goal drafting with epistemic graph authorship | [current Agent rule](../../../../../../crates/meld-world-model/src/agent/contracts.rs#L362), [Curation ownership finding](../../curation_assessment.md#traversal-and-curation-boundary) |

Curation synthesis: the proposed domain has no current entity to extend. It is a new world-model ownership boundary. Its public surface must be semantic and typed even though Events carries its durable facts. Its operation lifecycle, result currentness, perspective, and replay identity cannot be delegated to Traversal or `EventEnvelope`.

### World State And Traversal Entities

| Entity or transition | Current owner | Current behavior | Required relationship to assessed architecture | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `TraversalFactRecord` | Traversal | Copies admitted Event sequence, event type, objects, and relations into durable graph facts | Materialize graph-attached Curation Events as provenance-bearing facts | `extend existing` | Materializing every Event regardless of deliberate graph attachments could pollute the substrate | [fact contract](../../../../../../crates/meld-world-model/src/world_state/graph/contracts.rs#L129) |
| reducer source admission | Traversal | Accepts only producer domains `workspace_fs`, `context`, and `execution` | Admit promoted Curation products without hardcoding Curation semantic meaning | `extend existing` | A blanket producer-neutral rule changes current feedback-loop exclusion behavior | [source filter](../../../../../../crates/meld-world-model/src/world_state/graph/reducer.rs#L239) |
| `TraversalIntent` and source-specific anchor extraction | Traversal | Derive current anchors from four exact source event types | Remain separate from generic object and relation indexing | `extend existing` | Generic admission must not guess currentness or anchor semantics for Curation edges | [intent contract](../../../../../../crates/meld-world-model/src/world_state/graph/contracts.rs#L120), [intent extraction](../../../../../../crates/meld-world-model/src/world_state/graph/source_intent.rs) |
| `RelationRecord` | Traversal storage | Persists each relation with producing fact id and Event sequence | Supply occurrence provenance to public traversal results | `extend existing` | The store already knows occurrence identity but the public walk discards it | [relation record](../../../../../../crates/meld-world-model/src/world_state/graph/store.rs#L52) |
| `GraphWalkSpec` | Traversal | Bounds direction, relation filters, depth, current-only behavior, and fact inclusion | Remain the structural bounded-read contract used by Curation and Planner | `extend existing` | Its generic `current_only` flag has meaningful lifecycle behavior only for `selected` relations | [walk spec](../../../../../../crates/meld-world-model/src/world_state/graph/contracts.rs#L159), [visibility logic](../../../../../../crates/meld-world-model/src/world_state/graph/store.rs#L646) |
| `GraphWalkResult` | Traversal | Returns visited objects, optional facts, and bare relations | Preserve each traversed edge occurrence and source identity for replayable epistemic reasoning | `extend existing` | Bare equal-valued relations can have different producer, sequence, perspective, and currentness meaning | [walk result](../../../../../../crates/meld-world-model/src/world_state/graph/contracts.rs#L181), [walk implementation](../../../../../../crates/meld-world-model/src/world_state/graph/store.rs#L589) |
| `TraversalQuery` | Traversal | Exposes anchors, neighbors, bounded walks, facts, and provenance over storage | Serve bounded Curation and Planner reads without exposing raw storage | `extend existing` | Adding semantic filters here would transfer owner meaning into Traversal | [query facade](../../../../../../crates/meld-world-model/src/world_state/graph/query.rs) |
| `WorldModelQueries` | World state adapter | Catches Graph runtime up before each high-level read | Remain a catch-up-aware adapter for traversal reads | `extend existing` | Catch-up to Events does not establish downstream Belief or Agent visibility | [runtime query facade](../../../../../../crates/meld-world-model/src/world_state/query_runtime.rs) |
| `GraphRuntime` and graph ports | Traversal | Replay bounded Event pages, persist projection state, publish derived anchor Events through an outbox, and advance an identity-bearing cursor | Reuse the durable materialization mechanism for admitted Curation facts | `reuse unchanged` | Generic Curation admission may create new read-write feedback that current anchor exclusions avoid | [runtime](../../../../../../crates/meld-world-model/src/world_state/graph/runtime.rs), [ports](../../../../../../crates/meld-world-model/src/world_state/graph/ports.rs) |
| anchor records and provenance | Traversal | Own current anchor selection and supersession for source-specific perspectives | Continue to serve source anchors without becoming universal Curation currentness | `reuse unchanged` | README requirement and coverage currentness cannot inherit `selected` semantics automatically | [anchor contracts](../../../../../../crates/meld-world-model/src/world_state/graph/contracts.rs#L74) |
| legacy `ClaimRecord`, `EvidenceRecord`, and `WorldStateQuery` | legacy world state compatibility | Preserve the older claim and evidence projection beside Traversal | No direct relationship to the proposed Curation domain is established | `not needed` | Reusing legacy claims would create a second graph authorship path | [legacy contracts](../../../../../../crates/meld-world-model/src/world_state/contracts.rs), [legacy query](../../../../../../crates/meld-world-model/src/world_state/query.rs) |

World-state synthesis: Traversal is already the knowledge graph read substrate in the loose architectural sense. It owns materialization, structural queries, occurrence storage, replay, and provenance. Its changes are bounded to admission and public read fidelity. It does not gain Curation grammar, arbitrary mutation, or universal currentness.

### World Model Crate Surface

| Entity | Current owner | Current behavior | Required relationship | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `meld-world-model::lib` exports | crate adapter | Re-exports Agent, Belief, Planner, Strategy, waiting, and world-state contracts | Export explicit Plan, Curation, and occurrence-preserving Traversal contracts while keeping internals private | `adapter only` | Re-exporting raw stores would bypass domain contracts | [crate surface](../../../../../../crates/meld-world-model/src/lib.rs) |

## World Model Upward Synthesis

### Runtime Path

Current runtime state flows from Events through Traversal and configured Belief ingestion into Planner and Agent. Agent invokes Strategy for a Goal and sends the executable authorization to Execution through its Goal command sink. Under the assessed architecture, Curation joins that path as both a Traversal consumer and an Event producer. Agent remains the reconciliation and routing owner around the pure Strategy constructor.

### Behavior That Changes

Strategy changes from one executable candidate into heterogeneous Plan construction and reconstruction. Agent gains Plan authority, progression, and a separate Curation handoff. Curation is new. Planner must preserve enough bounded graph structure for Strategy. Traversal must admit deliberate Curation graph products and return occurrence provenance.

Belief behavior remains generic. Installed mappings change only where a Curation result is intended to update a configured belief family. Waiting behavior does not change.

### Likely World Model Writes

Likely Rust writes are concentrated in the Agent, Strategy, Planner, and Traversal domains, the new Curation domain, and the crate export adapter. Likely installed theory writes include Strategy theory packages, new Curation rule products, and selected Belief outcome mappings. This is impact evidence rather than implementation authorization.

### Reused Unchanged

The existing Agent root Goal and maintained-condition language, Execution-facing Goal commands, Goal lifecycle mutation path, Belief evidence admission core, Belief revision and view model, Graph runtime replay and cursor discipline, source-specific anchor semantics, and legacy claim compatibility surface can remain unchanged in meaning.

### World Model Ownership Boundary Finding

The newly exposed boundary is between Plan progression and Plan construction. Strategy owns the immutable semantic Plan body and deterministic reconstruction. Agent owns the durable active Plan state, authority, eligibility progression, and request for reconciliation. Curation owns epistemic realization and graph authorship. Putting persistent progression into Strategy would collapse a pure constructor into a runtime actor. Putting epistemic realization into Agent would collapse authority and semantic execution.

## Pass One Language Domain Sweep

Every language domain participates in the current executable Strategy path or its shared desired-state vocabulary. Inclusion in this table does not imply a write. The caller limit that `meld-lang` remains permissive is preserved throughout.

| Domain | Needed integration | Current integration | Completeness | Current evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `authority` | `publish` | Supplies executable Composition authority values and pure evaluation | `complete` | [authority](../../../../../../crates/meld-lang/src/authority.rs) | Does not authorize Curation operations | Confirm executable-only reuse |
| `composition` | `publish` | Supplies executable operator and subordinate Goal step graphs | `complete` for Task-shaped products | [composition](../../../../../../crates/meld-lang/src/composition.rs) | Does not need to become the heterogeneous Strategy Plan | Confirm local Plan ownership |
| `condition` | `publish` | Supplies proposition comparison vocabulary | `complete` | [condition](../../../../../../crates/meld-lang/src/condition.rs) | Does not decide epistemic proof | Reuse |
| `cost` | `publish` | Aggregates operator costs across Composition | `complete` for executable products | [cost](../../../../../../crates/meld-lang/src/cost.rs) | Does not price Curation work unless an owner supplies a separate model | Reuse |
| `effect` | `publish` | Supplies predicted proposition state changes for Operators and Methods | `complete` for executable products | [effect](../../../../../../crates/meld-lang/src/effect.rs) | Predicted effects are not Curation results or observed truth | Reuse |
| `evaluate` | `consume` | Evaluates propositions against ground world state | `complete` | [evaluation](../../../../../../crates/meld-lang/src/evaluate.rs) | Does not settle Belief or Goal lifecycle | Reuse |
| `goal` | `own` | Owns neutral desired proposition, Agent attribution, priority, provenance, and lifecycle values | `complete` | [Goal](../../../../../../crates/meld-lang/src/goal.rs) | Task and Epistemic Operation remain causes rather than Goals | Reuse |
| `method` | `publish` | Supplies reusable Composition templates triggered by propositions | `complete` for current executable Strategy | [Method](../../../../../../crates/meld-lang/src/method.rs) | No evidence proves shared Methods must contain Curation operations | Reuse |
| `operator` | `publish` | Supplies executable Capability-backed action contracts | `complete` | [Operator](../../../../../../crates/meld-lang/src/operator.rs) | Epistemic Operations are not executable Operators | Reuse |
| `proposition` | `own` | Owns desired and believed statement forms including relationships | `complete` | [Proposition](../../../../../../crates/meld-lang/src/proposition.rs) | Does not assign authority or truth | Reuse |
| `substitute` | `consume` | Grounds Composition variables from bindings | `complete` for current executable construction | [substitution](../../../../../../crates/meld-lang/src/substitute.rs) | Does not ground Curation-specific fields unless Strategy owns separate logic | Reuse |
| `term` | `own` | Owns object, dimension, artifact, literal, variable, and derived reference atoms | `complete` | [Term](../../../../../../crates/meld-lang/src/term.rs) | Does not encode perspective or Curation lifecycle | Reuse |
| `unify` | `consume` | Unifies proposition patterns and concrete propositions | `complete` | [unification](../../../../../../crates/meld-lang/src/unify.rs) | Does not select causal products | Reuse |
| `validate` | `consume` | Validates executable Composition structure and grounding | `complete` for Composition | [validation](../../../../../../crates/meld-lang/src/validate.rs) | Must not validate Strategy Plan or Curation grammar by guessing owner intent | Reuse |
| `world_state` | `own` | Owns immutable ground proposition sets, query, gap, and predicted effect application | `complete` | [language world state](../../../../../../crates/meld-lang/src/world_state.rs) | It is a reasoning value, not Traversal storage or accepted graph truth | Reuse |

## Frozen Language Affected Set

All fifteen current language domains are on the semantic or executable construction path and are frozen as runtime participants. No current language domain is frozen into likely Rust write scope.

## Language Entity Assessment

### Goal And Desired State Entities

| Entity | Current behavior | Required relationship | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- |
| `Goal` | Names one Agent-owned desired proposition with priority, source, and lifecycle | Remain the durable root desired-state language used by Agent and Strategy | `reuse unchanged` | Making Task or Epistemic Operation a Goal would invert ends and causes | [Goal contract](../../../../../../crates/meld-lang/src/goal.rs#L14) |
| `GoalPriority` | Adds urgency and optional cost ceiling to a Goal | Continue to qualify root Goal authority | `reuse unchanged` | Applying it automatically to every Plan obligation would manufacture lifecycle semantics | [priority](../../../../../../crates/meld-lang/src/goal.rs#L31) |
| `GoalSource` | Records maintained-condition, Belief, user, maintenance, or decomposition provenance | Continue to explain root Goal origin | `reuse unchanged` | `Decomposed` does not prove every intermediate condition should become a durable Goal | [source](../../../../../../crates/meld-lang/src/goal.rs#L40) |
| `GoalLifecycle` | Represents proposed, active, suspended, satisfied, or abandoned Goal state | Remain distinct from Task and Epistemic Operation lifecycles | `reuse unchanged` | Reusing Goal lifecycle for operation termination would conflate satisfaction with completion | [lifecycle](../../../../../../crates/meld-lang/src/goal.rs#L80) |
| `Proposition` | Represents holds, exists, accessible, related, conjunction, alternative, and negation | Provide the neutral language for root Goal targets and Strategy obligations | `reuse unchanged` | Proposition truth still depends on owner-admitted world state | [proposition](../../../../../../crates/meld-lang/src/proposition.rs) |
| `Term`, `Literal`, and `Condition` | Supply typed atoms and comparisons for propositions | Continue to express desired and projected state without Curation grammar | `reuse unchanged` | Encoding operation bounds or perspective as loose terms would erase typed owner contracts | [terms](../../../../../../crates/meld-lang/src/term.rs), [conditions](../../../../../../crates/meld-lang/src/condition.rs) |

Goal synthesis: Goal is already the shared language for the desired outcome. It should not become the name of causal work. Intermediate desired conditions can use Proposition without receiving a complete durable Goal lifecycle. Current code already proves the semantic distinction even though the future Plan representation remains unresolved.

### Executable Construction Entities

| Entity | Current behavior | Required relationship | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- |
| `Composition` | Represents a directed graph of steps and typed dependencies | Remain the Task-shaped executable graph referenced by Strategy products | `reuse unchanged` | Treating it as the mixed Plan would force all consumers to understand Curation | [Composition](../../../../../../crates/meld-lang/src/composition.rs#L13) |
| `Step` and `StepKind` | Represent an executable Operator or a subordinate Goal proposition | Continue to describe executable planning vocabulary | `reuse unchanged` | Adding an Epistemic Operation variant would widen Execution-facing grammar without proven need | [step contracts](../../../../../../crates/meld-lang/src/composition.rs#L22) |
| `Edge` and `EdgeKind` | Represent ordering, artifact data flow, and conditional activation | Continue to express executable dependencies inside a Task-shaped Composition | `reuse unchanged` | These edge kinds do not express epistemic visibility milestones or Plan contribution semantics | [edge contracts](../../../../../../crates/meld-lang/src/composition.rs#L40) |
| `Operator`, `Resolution`, `SlotConstraint`, and `CapabilityRef` | Represent one Capability-backed executable action with inputs, outputs, scope, tags, and exact identity | Remain the executable leaf language | `reuse unchanged` | Recasting Epistemic Operations as Operators would imply Execution realization | [operator contracts](../../../../../../crates/meld-lang/src/operator.rs) |
| `Effect` | Represents predicted assertion, retraction, or update in proposition state | Remain predicted executable contribution | `reuse unchanged` | A predicted effect is not an admitted Curation fact or observation | [effect](../../../../../../crates/meld-lang/src/effect.rs) |
| `Method` | Represents a reusable proposition-triggered Composition template | Remain available to current executable Strategy construction | `reuse unchanged` | No implemented consumer can instantiate a heterogeneous Method | [Method](../../../../../../crates/meld-lang/src/method.rs) |
| `CostEstimate` | Aggregates time, money, and provider calls over Operator steps | Remain the executable candidate cost vocabulary | `reuse unchanged` | A Curation cost model cannot be inferred from operator costs | [cost](../../../../../../crates/meld-lang/src/cost.rs) |

Executable-construction synthesis: these entities form a coherent Task-shaped sublanguage. They are not a neutral heterogeneous Plan grammar. Preserving that distinction avoids modifying Execution and prevents shared language validation from guessing Curation intent.

### Pure Reasoning And Validation Entities

| Entity or function | Current behavior | Required relationship | Change posture | Boundary risk | Exact evidence |
| --- | --- | --- | --- | --- | --- |
| `WorldState` and `GroundingError` | Hold ground propositions, answer query and gap, and apply predicted effects immutably | Remain Strategy's pure proposition snapshot while richer graph context stays owner-shaped | `reuse unchanged` | A flat proposition set alone cannot preserve every traversal occurrence or source cut | [world state](../../../../../../crates/meld-lang/src/world_state.rs) |
| `evaluate` and `EvalResult` | Perform three-valued proposition evaluation | Continue to support Agent and Strategy desired-state checks | `reuse unchanged` | Evaluation does not admit evidence or satisfy Goals | [evaluation](../../../../../../crates/meld-lang/src/evaluate.rs) |
| `Bindings` and `unify` | Bind proposition variables deterministically | Continue to support rule and Goal matching | `reuse unchanged` | Successful unification does not establish product eligibility or authority | [unification](../../../../../../crates/meld-lang/src/unify.rs) |
| `substitute`, `SubstitutionError`, and `UnboundVariable` | Ground Composition templates from bindings | Continue to support executable product construction | `reuse unchanged` | Curation-specific grounding belongs with its typed producer contract | [substitution](../../../../../../crates/meld-lang/src/substitute.rs) |
| `validate`, `ValidationResult`, `ValidationError`, and `ValidationWarning` | Validate Composition edges, cycles, artifact production, grounding, disconnection, and cost warnings | Continue to protect the structural shape it owns | `reuse unchanged` | Extending it to mixed Plans would violate permissive language ownership and duplicate producer checks | [validation](../../../../../../crates/meld-lang/src/validate.rs) |
| `AuthorityPolicy`, binding, decision, and evaluation | Evaluate requested Capability ids against executable Composition steps and subject scope | Remain executable authority language used for Task authorization | `reuse unchanged` | It cannot authorize an Epistemic Operation because it derives actions only from `StepKind::Op` | [authority](../../../../../../crates/meld-lang/src/authority.rs) |

Reasoning synthesis: all current pure helpers remain useful under the redesigned world-model boundary. None proves that `meld-lang` should own a Strategy Plan validator, Curation rule validator, operation lifecycle, or cross-domain grammar.

## Language Crate Synthesis

### Runtime Path

`meld-lang` remains present in Goal creation, Planner projection, Strategy unification and evaluation, executable candidate construction, Agent authority judgment, and later Execution consumption. Runtime participation is broad because these are shared values and pure functions.

### Behavior That Changes

No `meld-lang` behavior change is proven by current evidence. The architecture needs heterogeneous Plan semantics, but the ownership evidence places that aggregate in world-model Strategy. The current permissive language remains an input vocabulary rather than the enforcer of the new grammar.

### Likely Language Writes

No Rust write in `meld-lang` is established by this assessment. A later canonical requirement could move the boundary, but that would be a new ownership decision rather than an implication of current code.

### Reused Unchanged

All current language domains remain reusable in their existing meanings. Goal and Proposition express ends. Composition, Operator, Effect, Method, Cost, authority, substitution, and validation express or inspect executable means. WorldState, evaluation, and unification remain pure reasoning tools.

### Explicit Language Non-Integration

`meld-lang` does not own Strategy Plan lifecycle, Plan reconciliation, Curation execution, Curation authority, Event publication, Traversal currentness, Belief admission, Agent authorization policy, or Execution admission. It does not need an Epistemic Operation step variant merely because Strategy relates Epistemic Operations to Tasks.

## Cross Crate Synthesis

The two crates meet at a clean seam. `meld-lang` supplies permissive nouns and pure operations. `meld-world-model` assigns those nouns meaning inside Agent, Strategy, Belief, Planner, Traversal, and future Curation ownership.

The heterogeneous Plan is therefore a world-model aggregate over shared language values, not a replacement for `Composition`. A Task-shaped executable product may continue to contain or reference a `Composition`. A bounded Epistemic Operation uses a Curation-owned contract. Strategy relates both to Goal obligations through a Strategy-owned Plan. Agent authorizes and progresses that Plan during reconciliation.

This separation also resolves the user-raised terminology concern. Reconciliation names the continuing process in which admitted knowledge causes reassessment and Strategy reconstruction. No HTN-specific term is needed to define Meld's runtime ownership.

## Separated Impact Scopes

| Scope | Included domains and entities |
| --- | --- |
| runtime path | all `meld-lang` domains, world-model Agent, Belief, Planner, Strategy, Traversal, and proposed Curation |
| behavior change | world-model Agent, Planner, Strategy, Traversal, proposed Curation, and selected installed theory products |
| likely Rust writes | world-model Agent, Planner, Strategy, Traversal, new Curation domain, and crate exports |
| likely theory writes | Strategy theory, Curation rule and result theory, selected Belief outcome mappings, and PDS package routing owned outside these two crate internals |
| adapters | world-model crate exports and root composition ports assessed by the parent synthesis |
| reused unchanged | `meld-lang`, world-model Belief core, waiting, current Execution-facing Goal commands, Graph runtime replay discipline, source anchor semantics, and legacy world-state compatibility |

## Explicit Non-Integration Findings

Execution does not receive Epistemic Operations or the heterogeneous Strategy Plan.

Traversal does not author epistemic meaning. It materializes and reads producer-authored graph products.

Curation does not become a Traversal mutation API. It publishes through Events.

Belief does not interpret every Curation result. Installed mappings choose which results become evidence.

Planner does not execute Curation or select a Plan.

Strategy does not own durable Plan progression and does not execute its products.

Agent does not become the semantic executor of Curation rules merely because it supplies perspective and authority.

`meld-lang` does not enforce Strategy or Curation grammar.

The legacy world-state claim surface does not become the new epistemic authorship path.

## Smallest Missing Connective Behavior

At entity level, the smallest missing connective behavior is a world-model-owned chain that can persist one Agent-authorized Plan revision, route one eligible bounded Epistemic Operation to Curation, receive a typed terminal Curation result through Events and Traversal or Belief visibility, and ask pure Strategy to reconstruct against the new frozen cut. That chain has no current durable Plan entity, Curation operation contract, Curation actor, or Agent progression record.

This finding does not establish one concrete implementation shape. It establishes why extending only `StrategyCandidate`, only `GraphWalkResult`, or only `EventEnvelope` cannot close the architecture gap.

## Evidence Confidence

Confidence is high for the complete domain snapshots, current public entity shapes, Strategy's executable-only candidate, Agent's one-command authorization path, Planner's flattened graph scope, Traversal's producer allowlist, stored relation provenance, Belief's configured Event ingestion, and the absence of a Curation domain. Each is directly present in current code.

Confidence is high that no current `meld-lang` entity needs to understand Curation for the assessed architecture to be representable. A world-model-local aggregate can reference existing Goal, Proposition, WorldState, and Composition values without changing their owners.

Confidence is moderate on the exact boundary between Planner and a neutral frozen-cut assembler. Current code proves that Strategy lacks relation-rich context, but it does not decide whether Planner should own the new cut or consume another world-model query product.

Confidence is moderate on Agent authorization granularity and exact Plan persistence shape. Current ownership strongly places progression with Agent, but current code cannot decide whether authorization binds the whole Plan, each enabled product, or both at distinct moments.

## Unresolved Questions

The evidence does not select the durable representation of Plan nodes and dependencies, the promotion rule from a Strategy obligation to a durable Goal, the exact Curation operation vocabulary, the identity and supersession model for perspective-scoped expected entities, or the visibility milestone that enables a dependent Plan product.

The evidence also does not select whether Curation reads Traversal directly or receives a neutral frozen cut shared with Planner. It does establish that live unbounded graph reads are insufficient for deterministic Epistemic Operations and Strategy reconstruction.

No requirements, migration sequence, or implementation authorization is created by this report.
