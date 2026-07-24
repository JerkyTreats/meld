# Strategy Contracts

Date: 2026-07-23
Status: active
Scope: durable records and cross-domain handoffs for Agent-authorized Strategy

## Contract thesis

Strategy records an Agent judgment that a concrete theory of action is viable under exact world-model inputs. It does not record that the theory will succeed, that Execution accepted it, or that the desired world state was restored.

The durable chain is:

```text
Goal draft
→ Strategy construction attempt
→ candidate proposal or NoMethodAvailable
→ Agent authorizes candidates and Goal admission
→ accepted Execution Goal
→ Execution planning attempt
→ accepted task-network commitment
→ task and artifact outcomes
→ evidence and belief revision
→ Agent satisfaction curation
```

Each arrow crosses an explicit authority boundary. No downstream record may rewrite an upstream record.

## Contract map

| Record | Purpose |
|---|---|
| `GoalDraft` | desired state being considered before Execution |
| `KnownStrategyCatalogEntry` | one persisted reusable Strategy |
| `AvailableStrategySet` | reused and novel candidates for one Goal draft |
| `StrategyAlternative` | one concrete Composition with its justification |
| `NoMethodAvailable` | initial construction found no eligible theory of action |
| `StrategyDecision` | Agent authorization of a nonempty candidate inventory |
| `GoalAdmissionBundle` | Goal and authorized candidates sent to Execution together |
| `StrategyPlanningRequest` | admitted Goal and candidates presented to Execution Planning |
| `PlanningCommitment` | accepted operational realization |
| `StrategyOutcomeAssociation` | link from selected Strategy to observed outcomes |
| `StrategyAbstention` | active Goal has no useful action in the current state |

## Shared identity posture

Cross-domain references use `DomainObjectRef` as their coordinate shape. Strategy-owned records add immutable revision, content identity, and exact source lineage because a domain reference alone does not identify the semantic state that was used.

Every durable Strategy record must carry:

- stable record identity
- immutable revision
- content hash
- creation transaction time
- authorizing Agent identity
- perspective identity
- Directive and Goal draft revision
- admitted Goal revision when admission has occurred
- Strategy policy revision
- PDS package or facet-set revision
- profile, assignment, and activation lineage
- effective authority revision
- exact world-model frame
- capability semantic catalog revision
- verified Method inventory revision

Supersession creates a new revision. It never edits the prior judgment in place.

## Goal draft

`GoalDraft` is Agent-owned world-model curation state. It contains the exact ground `meld-lang::Goal` value proposed for Execution, but it is not present in the Execution Goal Set.

```text
GoalDraft
  draft_ref
  draft_revision
  agent_ref
  directive_ref
  proposed_goal
  source_belief_refs
  world_frame_ref
  value_posture_ref
  created_at
```

The proposed Goal identity may be allocated before admission so Strategy lineage remains stable. Its Execution lifecycle does not begin until Execution accepts a `GoalAdmissionBundle`.

## Known Strategy catalog

`KnownStrategyCatalog` is the canonical world-model index of persisted reusable Strategy knowledge.

```text
KnownStrategyCatalogEntry
  strategy_entry_ref
  strategy_entry_revision
  goal_pattern
  pds_lineage
  reusable_method_refs
  reusable_composition_template_refs
  prior_selection_refs
  prior_outcome_refs
  availability_posture
```

Execution retains custody of verified Method bodies. Catalog entries cite exact Method revisions. Novel episode candidates may be selected without entering the catalog.

`AvailableStrategySet` is the bounded result for one Goal draft and input frame.

```text
AvailableStrategySet
  set_ref
  goal_draft_ref
  input_frame
  known_strategy_catalog_revision
  reused_alternative_refs
  novel_alternative_refs
```

It is derived from applicable known entries plus newly constructed episode candidates. It is not another global catalog.

## Strategy construction attempt

`StrategyConstructionAttempt` records one bounded search episode.

```text
StrategyConstructionAttempt
  attempt_ref
  strategy_policy_ref
  agent_ref
  delegation_grant_ref
  directive_ref
  goal_draft_ref
  goal_ref
  input_frame
  known_strategy_catalog_revision
  search_bounds
  started_at
  completed_at
  candidate_refs
  result
```

`input_frame` references exact graph, belief, causal, regime, planner-projection, domain-theory, authority, activation, capability-catalog, verified Method inventory, and any consumed Execution operational-projection revisions.

`goal_ref` is absent during initial admission construction and present for later convergence attempts.

`search_bounds` identifies candidate count, expansion depth, compute budget, elapsed budget, and any model-use budget.

`result` is one of:

```text
candidate proposal
no method available
explicit abstention
superseded before proposal
```

An attempt is not a task and must not enter the task network.

`no method available` is valid only before initial Goal admission. `explicit abstention` is valid only for a later attempt associated with an already admitted Goal.

## Candidate alternative

`StrategyAlternative` wraps one concrete `meld-lang::Composition` with the semantic proof needed to inspect and compare it.

```text
StrategyAlternative
  alternative_ref
  origin
  known_strategy_entry_ref
  composition
  composition_hash
  obligation_refs
  subgoal_realizations
  method_derivations
  action_justifications
  edge_justifications
  authoritative_verdict_refs
  projection_ref
  assumptions
  authorized_bindings
  artifact_contracts
  reuse_contracts
  outcome_contract_refs
  validity_dependencies
  eligibility
```

The embedded Composition remains a pure language value. The Strategy wrapper owns persistence, provenance, lifecycle, and explanation.

`origin` distinguishes reused known Strategy, verified Method derivation, and novel episode construction. `known_strategy_entry_ref` is present only when the candidate came from the persisted catalog.

Each `subgoal_realizations` entry is keyed by Composition step identity and carries the exact child Composition hash or verified Method revision, bindings, and expanded Composition hash. The mapping must close every subgoal before proposal validation.

Each `method_derivations` entry is keyed by stable Method-instance identity and preserves parent step, verified inventory revision, Method identity, Method revision, Method content hash, template hash, bindings, and expanded Composition hash. The set recursively closes every nested Method-backed subgoal. Candidates with no Method source use an empty set.

`eligibility` distinguishes:

```text
eligible
ineligible with typed grounds
indeterminate with missing verdicts
```

An alternative may be retained for explanation even when it is not authorized.

## Candidate proposal

`StrategyProposal` records the bounded construction result presented to the Directive Agent or explicit delegate.

```text
StrategyProposal
  proposal_ref
  proposal_hash
  closure_hash
  attempt_ref
  goal_draft_ref
  input_frame
  ranked_alternative_refs
  recommendation
  unresolved_assumptions
  created_at
```

A proposal is immutable and inspectable. Its content-addressed closure includes every referenced alternative, Composition, projection, obligation graph, justification, subgoal realization, artifact contract, and outcome contract. The full closure must persist atomically, or every object must exist and hash-verify before the proposal becomes judgeable and the construction attempt becomes complete.

A proposal has no authority to enter Execution Planning.

## Agent judgment

`AgentStrategyJudgment` records acceptance or rejection of one immutable proposal.

```text
AgentStrategyJudgment
  judgment_ref
  agent_ref
  delegation_grant_ref
  proposal_ref
  proposal_hash
  goal_draft_ref
  proposed_goal_ref
  outcome
  selected_alternative_refs
  selection_policy
  command_preconditions
  rationale
  judged_at
```

`outcome` is accepted or rejected. Acceptance must authorize an exact nonempty subset of alternatives present in the referenced proposal. Rejection authorizes none and preserves typed rationale. Proposal reference, hash, Goal draft, proposed Goal, and alternative payload mismatch fails closed.

A configured deterministic judgment policy may compute the outcome under the Agent's authority, on the same pattern as the configured Goal curation rule. The policy identity and revision are part of the judgment record. Delegating computation to a policy does not move the authority, which remains the Agent's.

Initial acceptance preconditions require the Goal draft to remain current, proposal closure to hash-verify, delegation and effective authority to remain valid, assignment and activation revisions to remain current, and every selected alternative to remain eligible under its validity dependencies. Later replacement judgments also validate the admitted Goal lifecycle epoch. Judgment fails closed when any precondition is stale.

An accepted judgment creates the corresponding `StrategyDecision`. The judgment, decision, and durable publication obligation must be written atomically or derived by a deterministic idempotent reducer keyed by judgment identity. A crash must not strand accepted Agent authority without a queryable decision and publication obligation.

A rejected judgment remains durable but cannot enter Execution Planning.

## Edge justification

Every Strategy-created ordering or dataflow edge has its own inspectable justification.

```text
StrategyEdgeJustification
  edge_ref
  upstream_step_ref
  downstream_step_ref
  source_obligation_ref
  target_obligation_ref
  ground_relation_refs
  producing_effect_ref
  required_input_or_precondition_ref
  relevance_verdict_ref
  admission_verdict_ref
  prospective_artifact_contract_ref
  admission_gate_ref
  projected_sufficiency_ref
  capacity_fact_ref
  authority_fact_ref
```

Only fields material to the edge need values. Existing evidence cites its admission verdict. Future evidence cites a prospective artifact contract and an owning-domain admission gate. Ordinary operational dataflow cites capability and artifact contracts and does not require epistemic admission. Missing required support makes the edge indeterminate or invalid.

Artifact type compatibility is not a semantic justification. Graph adjacency is not evidence admission. A cost advantage is not causal proof.

## Prospective artifact contract

`ProspectiveArtifactContract` identifies future epistemic evidence or obligation discharge before its instance exists.

```text
ProspectiveArtifactContract
  contract_ref
  producer_step_ref
  producing_effect_ref
  subject_ref
  scope_ref
  semantic_type
  schema_revision
  content_identity_rule
  outcome_contract_ref
  admission_authority_ref
  admission_gate_ref
  downstream_obligation_refs
```

The eventual artifact instance and admission verdict must cite this contract and bind its exact content identity. A mismatched producer, subject, scope, schema, content identity, outcome contract, or admission authority leaves the gate closed.

## Settlement transform

`settlement` is a pure transform over a ground Goal target. The shared language owns the transform and the settlement proposition shape. The operational domain theory owns the observationality declaration the transform reads: each belief dimension declares whether its value is established only by admitted evidence that arrives after action, never by an action's declared effects. Evaluation over produced artifacts and observation of an authoritative external source are both settlement routes. Observationality is not inferred from proposition syntax.

The transform maps each observational proposition in the target to the proposition that its owning question is settled with admitted evidence bound to the subject revision of the referenced frame. The settling evidence arrives later in time; the binding is to the frame's subject revision, so later subject change invalidates the settlement. Non-observational propositions pass through unchanged.

The planner projection contract must emit the settlement vocabulary for every grounded question in scope, or regression over the transform has nothing to evaluate against.

Proposal validation regresses every candidate against `settlement(goal.target)`. Prospective effects gated on a future admission verdict are assumed discharged during regression, each assumption is recorded on the candidate, and a failed admission invalidates exactly the citing edges. The Goal threshold is never assumed. The untransformed target remains the satisfaction condition evaluated by Agent satisfaction curation over reconciled outcome evidence, and it is also the target that known-catalog `goal_pattern` lookup matches. A candidate whose effect model asserts an unobserved outcome value fails validation.

## Strategy projection

`StrategyProjection` is an authoritative world-model read over one bound candidate Composition.

```text
StrategyProjection
  projection_ref
  alternative_ref
  input_frame
  settlement_support
  action_effect_support
  outcome_threshold_projection
  evidence_assumptions
  causal_assumptions
  regime_sensitivity
  uncertainty
  risk
  capacity_feasibility
  expected_critical_path_time
  expected_resource_cost
  expected_provider_cost
  expected_convergence_turns
  expected_information_gain
  abstention_grounds
  validity_horizon
```

The owning world-model domains remain authoritative for epistemic values. Execution may supply current operational facts through a typed projection, but Strategy must preserve their source authority.

## Strategy decision

`StrategyDecision` is the durable Agent authorization of one bounded candidate inventory. It is distinct from construction and records the Agent judgment over a candidate proposal.

```text
StrategyDecision
  decision_ref
  decision_revision
  decision_hash
  attempt_ref
  proposal_ref
  proposal_hash
  agent_judgment_ref
  authorizing_agent_ref
  delegation_grant_ref
  directive_ref
  goal_draft_ref
  goal_ref
  input_frame
  alternative_refs
  selected_alternative_ref
  allowed_selection_policy
  value_posture_ref
  authorization_scope
  authority_epoch_fence
  strategy_eligibility_epoch_fence
  assumptions
  validity_horizon
  invalidation_dependencies
  supersedes
  reusable_method_proposal_ref
```

Selection may remain unset when the Agent authorizes Execution to choose among a bounded set using exact operational criteria.

The decision means:

```text
The Agent authorizes these candidates as viable theories of action
for this Goal under this frame and these assumptions.
```

It does not mean:

```text
The external world changed.
The Goal is satisfied.
Execution accepted the work.
```

## No method available

`NoMethodAvailable` records that bounded initial Strategy construction could not produce an eligible reusable or novel candidate for a Goal draft.

```text
NoMethodAvailable
  error_ref
  attempt_ref
  goal_draft_ref
  input_frame
  known_candidate_refs
  novel_candidate_refs
  rejection_grounds
  missing_semantics
  created_at
```

This is a visible runtime error rather than quiescence. The Goal draft remains outside Execution. An empty known Strategy catalog is not sufficient grounds because novel construction must also have been attempted under the available PDS affordances.

## Abstention

`StrategyAbstention` is a first-class result of a bounded later attempt for an already admitted Goal.

```text
StrategyAbstention
  abstention_ref
  attempt_ref
  goal_ref
  input_frame
  grounds
  missing_verdict_refs
  rejected_alternative_refs
  wake_dependencies
  validity_horizon
```

Grounds are typed and may include missing observation, indeterminate scope, insufficient evidence, unsupported causal chain, absent authority, unavailable capability class, capacity infeasibility, exhausted search bound, or no useful action.

An active Goal with a valid no-useful-action abstention keeps its `Active` lifecycle state. Its Strategy association is quiescent. New evidence or another declared wake dependency may authorize a new attempt.

`StrategyWakeRegistration` is written atomically with the abstention and quiescent association transition.

```text
StrategyWakeRegistration
  registration_ref
  abstention_ref
  goal_ref
  dependency_refs
  source_cursor_set
  dependency_epochs
  attempt_generation
```

Wake delivery is at least once from the durable cursor set. Restart replays every relevant dependency change after those cursors before declaring the association quiescent. Atomic registration and cursor replay prevent evidence arriving during commit, downtime, or restart from being missed. Duplicate wake delivery is idempotent by registration, source revision, and attempt generation.

## Goal association

`GoalStrategyAssociation` connects Goal lifecycle to Strategy lifecycle without embedding Strategy state in the Goal.

```text
GoalStrategyAssociation
  association_ref
  goal_draft_ref
  goal_ref
  active_decision_ref
  latest_attempt_ref
  current_status
  updated_at
```

`current_status` distinguishes draft awaiting construction, no method available, proposal pending Agent judgment, proposal rejected by Agent, admission pending, accepted by Execution, rejected by Execution, invalidated, convergence quiescent, and superseded.

The association is a projection over owned records. It must not become an alternate Goal state machine.

## Goal admission

`GoalAdmissionBundle` is the initial cross-domain handoff. It prevents an authored Goal from entering Execution without a viable theory of action.

```text
GoalAdmissionBundle
  admission_ref
  goal_draft_ref
  proposed_goal
  strategy_decision_ref
  authorized_candidate_refs
  authorizing_agent_ref
  effective_authority_ref
  world_frame_ref
```

`authorized_candidate_refs` must be nonempty and must match the Strategy decision. Execution rejects an empty or mismatched bundle. Acceptance creates the initial Execution Goal revision and makes the Strategy decision eligible for Planning.

This is intentionally one small admission contract. It does not require Strategy persistence and the Execution Goal store to share an implementation or storage engine.

```text
GoalAdmissionAccepted
  admission_ref
  goal_ref
  goal_revision
  goal_epoch_fence
  strategy_decision_ref
```

The accepted record supplies the first Goal lifecycle epoch. Later planning requests combine it with the Strategy decision authority and eligibility fences.

## Invalidation

`StrategyInvalidation` records why a prior decision is no longer eligible.

```text
StrategyInvalidation
  invalidation_ref
  decision_ref
  triggering_ref
  dependency_kind
  prior_revision
  current_revision
  reason
  recorded_at
```

Invalidation may follow graph change, belief revision, evidence-admission change, causal or regime change, Goal revision, authority change, activation change, capability-catalog change, verified Method inventory change, capacity change, expiry, or an Execution rejection.

Execution exposes the verified Method inventory to Strategy through a revisioned read contract. The inventory returns exact Method identity, revision, content hash, visibility, verification status, and source lineage without transferring registry authority.

Invalidation is produced either by a deterministic reducer over declared dependency changes or by an authorized Agent judgment. Persistence access alone cannot invalidate a decision.

Invalidation advances the Strategy eligibility epoch and blocks new task-network commitment and dispatch for that decision. It does not perform cancellation mechanics for work already in flight. Agent authority decides whether the broader intent remains authorized. Execution decides how committed work stops, transitions, or cleans up.

Every Strategy decision carries Agent authority and Strategy eligibility epochs. Accepted Goal admission supplies the initial Goal lifecycle epoch. Task-network commit and dispatch claims validate all three through a linearizable authority-preserving fence. Authority revocation and Strategy invalidation advance their owning epochs. The Goal lifecycle epoch advances on every lifecycle, replacement, or content transition that changes work eligibility, including activation, suspension, resume, satisfaction, reopening, abandonment, removal, supersession, and replacement. Each transition serializes against commit and dispatch claims. A stale fence rejects commitment and blocks new dispatch.

## Execution handoff

`StrategyPlanningRequest` is the cross-domain command from Agent-authorized Strategy into Execution Planning.

```text
StrategyPlanningRequest
  request_ref
  decision_ref
  goal_ref
  candidate_refs
  world_frame_requirement
  effective_authority_ref
  activation_ref
  capability_catalog_ref
  method_inventory_ref
  execution_operational_projection_ref
  capability_availability_revision
  capacity_revision
  target_task_network_ref
  target_task_network_revision
  target_task_network_state_hash
  planning_attempt_generation
  authority_epoch_fence
  goal_epoch_fence
  strategy_eligibility_epoch_fence
  planning_commitment_intent
  idempotency_key
```

Execution must validate every referenced revision before attempting realization. The allowed selection policy is read from the immutable Strategy decision and must not be supplied or changed after Agent authorization.

Execution may select only among authorized alternatives and bind only declared variables. Any semantic replacement requires a new Strategy decision.

The request may be issued only after Execution accepts the corresponding Goal admission. Execution `NoApplicableMethod` or an equivalent typed rejection remains a mechanical guard when the authorized inventory is stale or cannot be realized. It does not replace the world-model `NoMethodAvailable` admission error.

## Execution response

`StrategyPlanningResponse` reports the operational result without claiming domain success.

```text
StrategyPlanningResponse
  response_ref
  request_ref
  decision_ref
  planning_attempt_ref
  status
  selected_alternative_ref
  applied_bindings
  rejection_grounds
  accepted_commit_ref
  no_change_reason
  existing_commit_ref
```

`status` is one of:

```text
accepted
rejected
superseded
no operational change required
```

A rejection is a typed operational fact. Strategy may use it as an invalidation or wake input but may not rewrite it.

Temporary operational rejection does not invalidate semantic viability unless a declared dependency changed. `no operational change required` must cite a typed reason and the existing accepted commitment whose work already realizes the decision.

Exactly one durable terminal response exists per planning request idempotency key. Acceptance, rejection, supersession, and no-change results are recoverable after restart. Rejection and no-change responses create deterministic publication obligations when they drive Strategy invalidation, wake, or audit.

At the task-network boundary, `network_id` plus `planning_request_idempotency_key` is the reducer uniqueness key for a Strategy-originated mutation. Command identity is derived from that key. The reducer atomically persists the uniqueness mapping, accepted command, graph commit, planning commitment, terminal response, and publication outbox. A retry cannot bypass deduplication by supplying a different command identity.

## Planning commitment

`PlanningCommitmentIntent` is immutable input to the task-network mutation command. It carries planning request identity and idempotency key, Strategy decision, alternative revision, Goal revision, Composition hash, the complete Method-derivation inventory reference and hash, world frame, PDS lineage, all three epoch fences, activation, capability catalog, and stable event identity seed.

`PlanningCommitment` is derived deterministically from the accepted task-network mutation and its `CommitRecord`. Its publication outbox entry is created atomically with that accepted commit so a crash cannot leave durable work without a durable publication obligation.

```text
PlanningCommitment
  commitment_ref
  planning_request_ref
  planning_request_idempotency_key
  strategy_decision_ref
  strategy_decision_revision
  strategy_decision_hash
  strategy_alternative_ref
  strategy_alternative_revision
  strategy_alternative_hash
  goal_ref
  goal_revision
  goal_content_hash
  composition_hash
  method_inventory_revision
  method_derivation_inventory_ref
  method_derivation_inventory_hash
  planning_attempt_ref
  world_frame_ref
  task_network_ref
  base_revision
  pre_state_hash
  accepted_commit_ref
  accepted_revision
  post_state_hash
  mutation_set_ref
  applied_bindings
  reuse_decisions
  pds_package_ref
  pds_package_revision
  pds_package_hash
  profile_ref
  profile_revision
  assignment_ref
  assignment_revision
  activation_ref
  activation_revision
  effective_authority_ref
  capability_catalog_ref
  authority_epoch_fence
  goal_epoch_fence
  strategy_eligibility_epoch_fence
```

The reducer persists the accepted `StrategyPlanningResponse` atomically with the commit or derives it deterministically from the commitment using the request identity and idempotency key. Crash recovery must return the same accepted response.

The task network remains the operational plan and execution state. The Strategy decision remains its upstream semantic lineage.

Every committed dependency edge preserves its origin. A semantic edge cites its Strategy edge-justification record and may change only through a new Strategy decision. An operational scheduling constraint cites its Execution policy source and may be relaxed by Execution alone. The two origins must remain distinguishable in the committed network and in replay.

## Strategy outcome association

`StrategyOutcomeAssociation` preserves the join needed for later efficacy curation without defining that curation model now.

```text
StrategyOutcomeAssociation
  association_ref
  strategy_decision_ref
  strategy_entry_ref
  selected_alternative_ref
  planning_commitment_ref
  outcome_event_refs
  input_world_frame_ref
  observed_world_frame_ref
  evaluation_status
```

`strategy_entry_ref` is optional for a novel episode candidate. `evaluation_status` may remain unassessed. The record preserves which theory was selected, what Execution committed, and which observed outcomes followed. A later curation step may compare the projection with reconciled reality and update reusable Strategy preference through a new record.

## Evidence-admission ingestion

An owning epistemic domain submits a completed evidence-admission judgment through an authority-preserving task-network command.

```text
EvidenceAdmissionVerdict
  verdict_ref
  owning_domain_ref
  owning_domain_revision
  prospective_contract_ref
  artifact_content_identity
  subject_ref
  scope_ref
  schema_ref
  admission_authority_ref
  decision
```

The task-network reducer validates the verdict against the pending `EvidenceAdmission` edge and persists an immutable accepted-verdict record in its own revision stream. Dependency state advances only after that commit. Verdict identity is idempotent and conflicting reuse is rejected. Replay reconstructs readiness from the accepted record without reading mutable external state.

Execution lineage classifies work by origin. Method identity is present only for Method-derived steps. Strategy decision identity and Composition hash are mandatory for Strategy-generated lineage.

## Event posture

Canonical semantic events must be published for Goal admission, `NoMethodAvailable`, Agent authorization, Strategy invalidation, accepted planning commitment, task and artifact outcomes, accepted evaluation, and observed domain outcomes. `NoMethodAvailable` is surfaced as a runtime error event because it means a desired state could not be connected to any reusable or novel theory of action. Strategy decision and invalidation records create durable outbox obligations in the same transaction as their owning transition or through an equivalent deterministic reducer. Stable event identities derive from the owning Strategy transition, task-network commit, outcome record, or evaluator record.

Other canonical semantic events may be published for:

- Execution acceptance or rejection of a Strategy decision
- Strategy replacement
- material task-network expansion, replacement, or abandonment

Candidate enumeration, score iterations, ready-set recalculation, leases, heartbeats, and retry bookkeeping remain local unless an operational audit contract explicitly promotes them.

A Strategy event records semantic judgment. A planning event records operational intent. An outcome event records an observation. None may impersonate another.

## Idempotency and replay

The initial Strategy attempt idempotency key must include Goal draft revision, exact input frame, Strategy policy revision, known Strategy catalog revision, construction-grant revision, and invalidation state.

The planning request idempotency key must include Strategy decision revision, selected candidate inventory, Goal revision, execution operational-projection revision, capability availability, capacity revision, target task-network revision and state hash, and planning-attempt generation.

The task-network reducer must enforce the planning request key as part of its own uniqueness boundary. A new command identity cannot create a second commit for an already recorded `network_id` and planning request key.

A temporary operational rejection closes only that exact request. A declared wake condition increments the attempt generation or changes another keyed operational revision, allowing the still-valid Strategy decision to be attempted again without replaying the prior rejection.

Replay may reconstruct deterministic construction and compare content hashes. Nondeterministic construction reuses the exact recorded generator artifact and reruns deterministic validation. Replay treats Agent judgment as authoritative recorded input and must never regenerate or infer it.

Model-backed construction must retain model identity, policy or prompt asset revision, sampled output artifact, deterministic validation result, and replay posture.

## Boundary invariants

```text
Strategy decision does not imply planning acceptance.

Planning acceptance does not imply task success.

Task success does not imply claimed domain effect.

Artifact existence does not imply evidence admission.

Evidence admission does not imply correctness.

Correctness evaluation does not imply Goal satisfaction.

Only Agent satisfaction curation closes the Goal.

An initial Goal does not enter Execution without a nonempty authorized Strategy inventory.

NoMethodAvailable is not convergence quiescence.

An empty known Strategy catalog does not prove that no Strategy can be constructed.

A candidate settles questions. It does not assert outcomes.

A scheduling constraint is not a semantic dependency.
```

## Read with

- [World Model Strategy](README.md)
- [Strategy Requirements](requirements.md)
- [Docs Freshness Strategy](docs_freshness.md)
- [Strategy Ground Map](../../../plan/world_model/strategy/ground_map.md)
- [World Model Agent](../agent/README.md)
- [Directive Grounding](../agent/directive_grounding.md)
- [World Model Planner](../planner/README.md)
- [Execution Planning](../../execution/planning/README.md)
- [Task Network](../../execution/task_network.md)
- [Meld Lang](../../meld-lang/README.md)
