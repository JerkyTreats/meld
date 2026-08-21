# Evidentiary Review Of Goal Language And Heterogeneous Strategy Plans

Date: 2026-08-20

Status: independent discovery review, implementation not authorized

## Review Function

This review tests the proposed shared Goal language and heterogeneous Strategy plan against current Meld code, active design, and the docs freshness example. Its function is evidentiary rather than advocative. It does not select a runtime, define migration work, or authorize changes.

The evidence packet begins with the neutral [current code ground map](../current_code_groundmap.md), the [Traversal and Curation assessment](../curation_assessment.md), and the [bounded epistemic operation refinement](../bounded_epistemic_operations.md). It also checks the canonical [Strategy thesis](../../../../cognitive_architecture/world_model/strategy/README.md), canonical [Strategy Plan construction](../../../../cognitive_architecture/world_model/strategy/search.md), and the cited implementation contracts directly.

## Executive Finding

The proposal contains one strongly supported distinction and two claims that need narrower wording.

**Supported inference.** Task and bounded epistemic operation may share the language of desired outcomes, Agent ownership, causal contribution, and Goal reference. Both may be selected by Strategy as ways to discharge conditions required by one Agent Goal.

**Qualification.** Task and bounded epistemic operation should not be treated as Goals merely because both perform work. The current `Goal` is a desired proposition with Agent ownership, priority, provenance, and lifecycle. Task and epistemic operation are different discharge products with different executing authorities. The shared semantic center is the desired proposition or Strategy obligation, not necessarily the complete current `Goal` record or its runtime workflow.

**Supported inference.** A heterogeneous Strategy plan is better described as a causal graph of desired conditions, obligations, and closed discharge products than as one Task.

**Qualification.** A plan is not adequately modeled as a set of Goals. A set does not preserve causality, enabling knowledge, conditionality, or the distinction between a desired condition and the operation selected to establish it. The current Composition already demonstrates that semantic dependency edges are necessary even for executable work.

**Contradiction.** Strategy is not an implemented continuous reconciliation vehicle. It is currently a pure bounded construction call over one proposed Goal and one immutable planner snapshot. Agent satisfaction curation can reopen a satisfied Goal after observed drift, but no durable Strategy plan is progressed, reconciled, or reconstructed automatically when Events change the world model.

The strongest evidence-backed formulation is therefore:

> Strategy constructs and may later reconstruct a causal plan for an Agent Goal. The plan may contain executable and epistemic discharge products. Agent retains Goal and authorization authority. Execution receives only executable work. Curation receives only epistemic work. Results return through world-model observation and Event-backed knowledge flow.

That formulation is an inferred extension. It is not current runtime behavior.

## What Goal Means In Current Code

**Implemented fact.** The shared language `Goal` contains a stable identity, Agent identity, desired `Proposition`, operational priority, provenance, and lifecycle. It says what an Agent wants to become true. It carries no executor class and no operation body. See [Goal contracts](../../../../../crates/meld-lang/src/goal.rs).

**Implemented fact.** The lifecycle is not merely descriptive. Current Agent curation creates a proposed Goal, Execution admission makes the Goal active, and Agent satisfaction curation later requests satisfaction or reopening. The current runtime therefore gives the complete `Goal` record cross-domain lifecycle meaning even though `meld-lang` itself is a permissive value owner. See [Agent curation](../../../../../crates/meld-world-model/src/agent/curation.rs), [Agent Goal contracts](../../../../../crates/meld-world-model/src/agent/contracts.rs), and the [current Goal Set path](../current_code_groundmap.md#agent-goal-curation-and-goal-set-publication).

**Implemented fact.** Composition contains a distinct recursive goal form named `StepKind::Goal`, but that form stores only a `Proposition`. It does not embed the full Agent-owned `Goal` record. See [Composition contracts](../../../../../crates/meld-lang/src/composition.rs).

**Implemented fact.** Current Strategy candidate completion rejects every non-operator step. The recursive proposition form is present in the permissive language but cannot survive into a current Strategy candidate. See [candidate completion](../../../../../crates/meld-world-model/src/strategy/search.rs#L378).

**Supported inference.** Meld already contains the beginning of a useful language distinction. A top-level Agent Goal is a durable desired-state commitment. A recursive goal proposition is a condition that planning may need to discharge. The second concept is closer to a heterogeneous plan node than manufacturing a full lifecycle-bearing Goal for every intermediate condition.

**Qualification.** This does not prove that future Strategy should reuse `StepKind::Goal`. Current code neither expands it during Strategy search nor lowers it as executable work. It proves only that the language already distinguishes a desired proposition from an executable operator.

## Should Task And Epistemic Operation Share Goal Language

**Supported inference.** They should share the semantic language needed to state what condition each product is intended to establish, what parent Agent Goal it contributes to, and what evidence could later support that contribution. Without this common semantic frame, Strategy cannot compare or chain heterogeneous work against one outcome.

**Qualification.** Sharing Goal language does not mean sharing one executor contract. A Task closes executable Capability, binding, artifact, and authority concerns. A bounded epistemic operation closes perspective, source cut, traversal bounds, rule identity, admissible Curation vocabulary, and completion meaning. The [bounded operation refinement](../bounded_epistemic_operations.md) correctly keeps those closure requirements isolated.

**Qualification.** The assertion that both actions will always have an end result is valid only if result means a terminal outcome. Current Tasks can fail, and the proposed epistemic operation may return unchanged, abstained, or bounded incomplete. Neither contract can promise the desired state. Their common guarantee may be a typed terminal account of the attempt, not successful Goal discharge.

**Contradiction.** Treating Task and epistemic operation themselves as `Goal` values would invert the current language. A `Goal` is the desired proposition. A Task or epistemic operation is a candidate cause selected to discharge that proposition. Current code has no operation body, operation result, source cursor, traversal bound, or executor identity in `Goal`.

**Supported inference.** A future plan may use Goal-like propositions at multiple levels while preserving exactly one durable Agent Goal as the authority root. Intermediate desired conditions can remain Strategy-owned obligations unless they need an independently maintained lifecycle. This avoids turning every causal dependency into an Execution Goal Set member.

**Unresolved question.** The evidence does not settle when an intermediate desired condition deserves promotion from a Strategy obligation into a durable Agent Goal. Independent maintenance, authorization, reuse across plans, and satisfaction lifecycle are plausible reasons, but none is specified today.

## Is A Plan A Set Of Goals

**Qualification.** The claim is directionally useful but structurally incomplete. A plan explains how desired conditions may be established. It therefore needs both desired conditions and candidate causes, plus dependency meaning between them.

**Implemented fact.** Even the current executable-only Composition is a directed graph of steps and typed edges. Its edge vocabulary includes ordering, artifact data flow, and conditional activation. See [Composition contracts](../../../../../crates/meld-lang/src/composition.rs).

**Implemented fact.** Current Strategy search closes required artifacts backward through Capability producers and constructs data-flow edges. Candidate terminality depends on closure and evidence routing, not merely on collecting a set of Goals or operators. See [Strategy search](../../../../../crates/meld-world-model/src/strategy/search.rs).

**Supported inference.** A heterogeneous plan requires at least three semantic roles. It needs conditions to establish, discharge products that may establish them, and causal dependencies that explain eligibility and contribution. Task and epistemic operation are discharge products. They are not interchangeable with the conditions they serve.

**Supported inference.** HTN-style decomposition can use goal propositions as intermediate nodes without making the plan only a goal hierarchy. The leaves still need ground products owned by their executors. Epistemic leaves belong to Curation. Executable leaves belong to Execution after Agent authorization.

**Unresolved question.** The evidence does not decide whether the durable Strategy product preserves explicit condition nodes or compiles them into proof obligations attached to product edges. Both can express the causal model. This is a representation question, not an ownership question.

## Is Strategy Plan Construction

**Active design.** The active Strategy thesis calls Strategy Meld's only canonical semantic planning workflow. It begins from a Goal, exact planner state, Capability knowledge, Method knowledge, and construction policy. It owns semantic choice and leaves operational realization to Execution. See [Strategy thesis](../../../../cognitive_architecture/world_model/strategy/README.md).

**Active design.** The same thesis currently narrows successful construction to one authorizable Task. The active search design narrows the canonical product to one `StrategyCandidate` carrying an executable Capability dependency blueprint. See [Strategy search design](../../../../cognitive_architecture/world_model/strategy/search.md#strategy-candidate).

**Implemented fact.** Current `StrategyProblem` contains one ground Goal, one immutable `WorldState`, one planner snapshot identity, Strategy theory, executable Capability views, optional Methods, and evaluation policy. Current `StrategyCandidate` contains one `Composition` and one prospective evidence route. See [Strategy contracts](../../../../../crates/meld-world-model/src/strategy/contracts.rs).

**Contradiction.** Current code cannot construct the proposed heterogeneous plan. It has no epistemic-operation catalog, epistemic step, multi-product authorization, causal plan state, or plan progression contract. Candidate completion and independent verification require operator-only executable Composition steps.

**Supported inference.** Expanding Strategy from Task construction to causal Plan Construction is consistent with its existing semantic authority. Strategy already decides which declared causes contribute to Goal settlement and why. Adding a separately declared epistemic operation vocabulary would broaden the available cause classes without transferring their execution mechanics into Strategy.

**Qualification.** Strategy may author a ground epistemic operation request without executing epistemic semantics. The same principle already applies to executable Capability contracts. Strategy grounds and chains producer-declared products. Curation and Capability owners define what those products mean and how completion is judged.

## Is Strategy A Continuous Reconciliation Vehicle

**Implemented fact.** One Strategy search call is pure and bounded. It consumes an immutable request and returns a result. No current Strategy store, plan cursor, subscription, Event consumer, or progression runtime exists. See [Strategy search entry](../../../../../crates/meld-world-model/src/strategy/search.rs#L12).

**Implemented fact.** Agent curation is driven by revised Belief and Planner products. It may create a Goal when a maintained condition is breached, satisfy an active Goal when its target is observed, and reopen a satisfied Goal after drift. See [Agent curation](../../../../../crates/meld-world-model/src/agent/curation.rs).

**Qualification.** Events do flow into Traversal and Belief before Agent reassessment in the current architecture, but not every Event becomes a Belief update and not every graph fact reaches Strategy. Planner currently exposes propositions plus reduced graph scope rather than relation topology. See the [implemented end-to-end path](../current_code_groundmap.md#implemented-end-to-end-path) and [Belief integration](../current_code_groundmap.md#belief-integration).

**Supported inference.** Strategy can be understood as the pure reconstruction function used by future continuous reconciliation. New admitted knowledge could invalidate assumptions, enable a product, or reveal that a remaining product is unnecessary. Reinvoking Strategy over the same active Agent Goal and a new frozen snapshot is consistent with the current pure-function shape.

**Contradiction.** Calling Strategy itself a continuous reconciliation vehicle overstates current ownership. Continuous reconciliation also requires a trigger, durable plan identity, completed-product history, supersession rules, and authority for newly enabled work. None of those belongs to current Strategy code, and the active design does not assign them.

**Unresolved question.** The missing reconciliation owner could be Agent, a Strategy plan runtime, or another world-model orchestration concern. Evidence supports keeping semantic reconstruction in Strategy, but it does not establish who detects invalidation and requests reconstruction.

## Agent And Strategy Authority

**Implemented fact.** Agent owns the durable directive, perspective, subject, branch, maintained condition, and curation rule. Agent curation authors Goal commands and Goal lifecycle mutations. See [Agent record](../../../../../crates/meld-world-model/src/agent/contracts.rs#L103) and [Agent curation](../../../../../crates/meld-world-model/src/agent/curation.rs).

**Implemented fact.** Strategy recommends a candidate for the exact Goal and planner snapshot. It does not authorize that candidate. Agent runtime independently verifies the recommendation and attaches an Agent-owned `StrategyAuthorization` to its durable decision and Goal command. See [Strategy authorization](../../../../../crates/meld-world-model/src/agent/strategy.rs).

**Canonical design.** Agent judges What and Why as expressed by the exact Strategy product. Execution owns How and When for admitted executable work. This boundary is explicit in the [Strategy design](../../../../cognitive_architecture/world_model/strategy/README.md).

**Supported inference.** In a heterogeneous design, Strategy should express what causal products are required and why they contribute. Agent should retain authority over the root Goal and the exact plan it is willing to pursue. Curation and Execution should retain authority over accepting and executing their respective product shapes.

**Qualification.** Agent ownership of the Goal does not imply that Agent directly authors every epistemic edge. A Curation operation may execute under an Agent perspective and installed rule while Curation owns the operation semantics and emitted vocabulary. The result remains attributable to the Agent perspective without collapsing Curation into Agent.

**Unresolved question.** Current evidence does not decide whether Agent authorizes the whole causal plan once, authorizes each newly enabled product, or performs both at distinct levels. That decision affects reconciliation, supersession, and what exact content identity an authorization covers.

## The Execution Goal Set Seam

**Implemented fact.** The current seam does not receive a compiled Task. Agent submits a Goal command containing the Goal and an optional Strategy authorization. That authorization contains the ground executable Composition. Execution stores the Goal and authorization, then its planning domain lowers Composition operators into compiled Task nodes. See the [current Goal Set path](../current_code_groundmap.md#agent-goal-curation-and-goal-set-publication).

**Qualification.** The active design calls the authorized Strategy product a Task, while current code calls it a `StrategyCandidate` with a `Composition`. Any statement that only Tasks cross the seam must first preserve this ownership distinction. A world-model executable blueprint may cross. An Execution-owned compiled Task cannot cross before Execution creates it.

**Supported inference.** Only executable discharge products should cross the Execution Goal Set seam. An epistemic operation belongs at a Curation boundary. Sending the whole heterogeneous Strategy plan to Execution would expose epistemic dependencies and world-model reasoning to a consumer that currently has no need or authority to interpret them.

**Supported inference.** The heterogeneous Strategy plan therefore remains a world-model product. Agent may authorize the whole plan, but it routes only the eligible executable slice to Execution and only the eligible epistemic slice to Curation. Results can later make other products eligible without giving either consumer ownership of the whole plan.

**Qualification.** Current Goal Set storage associates one Goal with one optional executable Strategy authorization. It does not accept several independently enabled executable products over the life of one heterogeneous plan. The principle that only executable work crosses is supported. The exact package cardinality and lifecycle are unresolved and not implemented.

**Unresolved question.** It is not yet clear whether every executable slice carries the root Agent Goal, a Goal reference, or a separately durable execution-facing Goal derived from the root. Reusing the complete root Goal preserves current admission and lifecycle mechanics, but may obscure that one Task is only partial discharge of a larger heterogeneous plan.

## Docs Freshness Proof

**Implemented fact.** The installed docs Strategy theory turns the docs freshness Goal into an artifact settlement obligation and exposes only executable Capabilities. Closing inputs yields inspect, draft, validate, publish, and assess. See [docs Strategy theory](../../../../../theory/docs_freshness/strategy_theory.docs_freshness.json) and the [current docs branch assessment](../current_code_groundmap.md#docs-freshness-current-branch-behavior).

**Implemented fact.** Scope inspection excludes managed README files. Drafting then creates a README candidate for each meaningful directory. Claim validation evaluates claims attached to candidate README patches, and final assessment requires a publication receipt. See [docs capability code](../../../../../src/docs/capability.rs) and [claim validation](../../../../../src/docs/claim_validation.rs).

**Contradiction.** The current branch cannot represent the epistemic-only case where README F already exists and only the missing expected entity, claim mapping, or correctness relationship must be established. It must construct the executable publication chain before it can produce the configured assessment artifact.

**Supported inference.** The proposed example demonstrates why one Agent Goal may need heterogeneous discharge products. Establishing expected README F, connecting required claims, and assessing observed coverage are epistemic operations. Creating or revising physical README F is executable work. Their shared contribution target is docs freshness, but their executors and terminal result contracts differ.

**Supported inference.** The first epistemic result can eliminate executable work. If Curation establishes that an observed README realizes expected README F and covers the required claims under a bounded source cut, Strategy need not select a write Task. Conversely, a bounded non-realization result gives Strategy an explicit mismatch from which an executable product can be constructed.

**Qualification.** Epistemically asserting expected README F does not mean that a workspace file exists. Expected identity and observed realization must remain distinct predicates or entities. Otherwise Strategy would see the desired state as already satisfied and lose the mismatch it needs to plan.

## Consolidated Verdict

**Implemented fact.** Meld currently has a shared desired-state language, executable Composition language, Agent Goal lifecycle, one-shot Strategy search, and an Execution Goal Set seam carrying one executable authorization.

**Active design.** Strategy is the only semantic planning workflow, but its current canonical product is one authorizable executable Task or executable candidate.

**Supported inference.** Effective planning may require both epistemic and executable causal work. Strategy is the natural owner of their semantic composition because it already connects Agent Goals to declared causes. The products remain separately closed and separately executed.

**Qualification.** Sharing Goal language should mean sharing desired proposition, contribution, and root Goal reference. It should not mean making Tasks and epistemic operations into identical Goals or sending both through one runtime workflow.

**Contradiction.** Strategy is not currently a heterogeneous Plan Constructor or continuous reconciliation runtime. The docs freshness branch proves the missing epistemic product path, while current Strategy and Goal Set contracts prove the executable-only limitation.

**Unresolved question.** The decisive remaining design questions are the durable shape of a Strategy plan, the promotion rule for intermediate Goal propositions, the owner of continuous reconciliation progression, the granularity of Agent authorization, and the cardinality of executable submissions under one root Goal.

## Evidence Confidence

Confidence is high for the implemented Goal shape, Strategy candidate shape, Agent authorization path, executable-only Goal Set seam, one-shot search behavior, and docs freshness limitation because each is directly present in code or installed theory.

Confidence is high that a heterogeneous plan cannot be represented by current Strategy contracts and that sending epistemic operations to Execution would contradict the established domain boundary.

Confidence is moderate that proposition-level Goal language is the correct common semantic layer. Current contracts strongly support the distinction, but no accepted heterogeneous plan specification yet establishes its durable representation.

Confidence is moderate that Strategy should be described as reconstruction inside continuous reconciliation rather than as the loop owner. The current pure function is compatible with that role, but plan progression ownership remains unspecified.

## Reviewer Self-Assessment

This review found the proposal directionally coherent but narrowed three terms that could otherwise collapse domain boundaries. Goal is desired state rather than work. Plan is a causal graph rather than a set. Continuous reconciliation is a workflow around Strategy reconstruction rather than implemented Strategy behavior.

The strongest part of the evidence is the direct alignment among `Goal`, `StepKind`, `StrategyCandidate`, Agent authorization, and the docs freshness pipeline. The weakest part is future lifecycle placement. Current code proves absence there but cannot decide the correct owner.

No claim in this review treats the discovery documents as implemented behavior. No implementation plan is proposed.
